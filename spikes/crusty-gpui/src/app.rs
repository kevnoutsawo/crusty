//! The Crusty window: method + URL bar, send button, status line, and a
//! virtualized response viewer (Body / Headers).

use crate::input::{InputEvent, TextInput};
use crate::theme::Theme;
use crusty_core::request::{HttpMethod, RequestDefinition};
use crusty_core::response::{HttpResponse, StatusCategory};
use crusty_http::HttpClient;
use gpui::{
    Context, Entity, FocusHandle, Focusable, FontWeight, Hsla, SharedString, Subscription, Task,
    UniformListScrollHandle, Window, div, prelude::*, px, uniform_list,
};
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(target_os = "macos")]
const MONO: &str = "Menlo";
#[cfg(not(target_os = "macos"))]
const MONO: &str = "DejaVu Sans Mono";

const ROW_HEIGHT: f32 = 20.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Body,
    Headers,
}

enum Status {
    Idle,
    Sending,
    Done {
        code: u16,
        text: SharedString,
        ms: u128,
        size: u64,
        category: StatusCategory,
    },
    Failed(SharedString),
}

pub struct CrustyApp {
    method: HttpMethod,
    url: Entity<TextInput>,
    status: Status,
    tab: Tab,
    body: Arc<Vec<SharedString>>,
    headers: Arc<Vec<(SharedString, SharedString)>>,
    scroll: UniformListScrollHandle,
    client: Arc<HttpClient>,
    runtime: Arc<tokio::runtime::Runtime>,
    in_flight: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl CrustyApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let url = cx.new(|cx| TextInput::new("Enter URL, then press Enter", cx));
        let on_submit = cx.subscribe(&url, |this, _, event: &InputEvent, cx| {
            if let InputEvent::Submit = event {
                this.send(cx);
            }
        });
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to start the request runtime");

        Self {
            method: HttpMethod::Get,
            url,
            status: Status::Idle,
            tab: Tab::Body,
            body: Arc::default(),
            headers: Arc::default(),
            scroll: UniformListScrollHandle::new(),
            client: Arc::new(HttpClient::default()),
            runtime: Arc::new(runtime),
            in_flight: None,
            _subscriptions: vec![on_submit],
        }
    }

    pub fn focus_url(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.url.focus_handle(cx));
    }

    fn cycle_method(&mut self) {
        let all = HttpMethod::all();
        let i = all.iter().position(|m| *m == self.method).unwrap_or(0);
        self.method = all[(i + 1) % all.len()];
    }

    fn send(&mut self, cx: &mut Context<Self>) {
        let url = self.url.read(cx).text().trim().to_string();
        if url.is_empty() || matches!(self.status, Status::Sending) {
            return;
        }

        let mut def = RequestDefinition::new("GPUI request", url);
        def.method = self.method;
        let resolved = match crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new())
        {
            Ok(r) => r,
            Err(e) => {
                self.status = Status::Failed(e.to_string().into());
                cx.notify();
                return;
            }
        };

        self.status = Status::Sending;
        cx.notify();

        let client = self.client.clone();
        let job = self
            .runtime
            .spawn(async move { client.execute(&resolved).await });
        self.in_flight = Some(cx.spawn(async move |this, cx| {
            let result = job.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(response)) => this.show(response),
                    Ok(Err(e)) => this.status = Status::Failed(e.user_message().into()),
                    Err(e) => {
                        this.status = Status::Failed(format!("request task failed: {e}").into())
                    }
                }
                this.in_flight = None;
                cx.notify();
            })
            .ok();
        }));
    }

    fn show(&mut self, response: HttpResponse) {
        let body = response
            .body_json_pretty()
            .or_else(|| response.body_text().map(String::from))
            .unwrap_or_else(|| format!("<binary data, {} bytes>", response.body.len()));
        self.body = Arc::new(
            body.lines()
                .map(|l| SharedString::from(l.to_string()))
                .collect(),
        );

        let mut headers: Vec<(SharedString, SharedString)> = response
            .headers
            .iter()
            .map(|(k, v)| (k.clone().into(), v.clone().into()))
            .collect();
        headers.sort_by(|a, b| a.0.cmp(&b.0));
        self.headers = Arc::new(headers);

        self.status = Status::Done {
            code: response.status,
            text: response.status_text.clone().into(),
            ms: response.timing.total.as_millis(),
            size: response.size.body_size,
            category: response.status_category(),
        };
        self.scroll = UniformListScrollHandle::new();
    }

    fn status_line(&self, t: &Theme) -> impl IntoElement {
        let row = div()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_1p5()
            .min_h(px(32.));
        match &self.status {
            Status::Idle => row
                .text_color(t.text_muted)
                .child("Type a URL and press Enter or click Send."),
            Status::Sending => row.text_color(t.warning).child("Sending request…"),
            Status::Failed(msg) => row
                .text_color(t.danger)
                .child(div().flex_1().min_w_0().child(msg.clone())),
            Status::Done {
                code,
                text,
                ms,
                size,
                category,
            } => row
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(status_color(*category, t))
                        .child(format!("{code} {text}")),
                )
                .child(div().text_color(t.text_muted).child(format!("{ms} ms")))
                .child(div().text_color(t.text_muted).child(format_size(*size))),
        }
    }

    fn tab_button(
        &self,
        tab: Tab,
        label: &'static str,
        t: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.tab == tab;
        div()
            .id(label)
            .px_3()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_color(if active { t.accent } else { t.text_muted })
            .when(active, |d| d.bg(t.elevated))
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.tab = tab;
                cx.notify();
            }))
    }

    fn viewer(&self, t: &Theme) -> impl IntoElement {
        let (text, muted, accent) = (t.text, t.text_muted, t.accent);
        let list = match self.tab {
            Tab::Body => {
                let lines = self.body.clone();
                uniform_list("body", lines.len(), move |range, _, _| {
                    range
                        .map(|i| {
                            div()
                                .h(px(ROW_HEIGHT))
                                .px_4()
                                .flex()
                                .gap_4()
                                .whitespace_nowrap()
                                .child(
                                    div()
                                        .w(px(44.))
                                        .text_color(muted)
                                        .child(format!("{:>5}", i + 1)),
                                )
                                .child(div().text_color(text).child(lines[i].clone()))
                        })
                        .collect::<Vec<_>>()
                })
            }
            Tab::Headers => {
                let rows = self.headers.clone();
                uniform_list("headers", rows.len(), move |range, _, _| {
                    range
                        .map(|i| {
                            let (k, v) = &rows[i];
                            div()
                                .h(px(ROW_HEIGHT))
                                .px_4()
                                .flex()
                                .whitespace_nowrap()
                                .child(div().text_color(accent).child(format!("{k}: ")))
                                .child(div().text_color(text).child(v.clone()))
                        })
                        .collect::<Vec<_>>()
                })
            }
        };
        list.track_scroll(self.scroll.clone())
            .size_full()
            .font_family(MONO)
            .text_sm()
    }
}

impl Render for CrustyApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.global::<Theme>();
        let sending = matches!(self.status, Status::Sending);
        let has_response = matches!(self.status, Status::Done { .. });

        let request_bar = div()
            .flex()
            .items_center()
            .gap_2()
            .p_3()
            .bg(t.surface)
            .border_b_1()
            .border_color(t.border)
            .child(
                div()
                    .id("method")
                    .w(px(92.))
                    .py_1p5()
                    .flex()
                    .justify_center()
                    .rounded_md()
                    .bg(t.elevated)
                    .font_weight(FontWeight::BOLD)
                    .text_color(method_color(self.method, &t))
                    .cursor_pointer()
                    .child(self.method.as_str())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cycle_method();
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(t.bg)
                    .border_1()
                    .border_color(t.border)
                    .child(self.url.clone()),
            )
            .child(
                div()
                    .id("send")
                    .px_5()
                    .py_1p5()
                    .rounded_md()
                    .bg(if sending { t.elevated } else { t.accent })
                    .text_color(if sending { t.text_muted } else { t.on_accent })
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .child(if sending { "Sending…" } else { "Send" })
                    .on_click(cx.listener(|this, _, _, cx| this.send(cx))),
            );

        let tabs = div()
            .flex()
            .gap_1()
            .px_3()
            .py_1()
            .border_b_1()
            .border_color(t.border)
            .child(self.tab_button(Tab::Body, "Body", &t, cx))
            .child(self.tab_button(Tab::Headers, "Headers", &t, cx));

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t.bg)
            .text_color(t.text)
            .text_sm()
            .child(request_bar)
            .child(self.status_line(&t))
            .when(has_response, |d| {
                d.child(tabs)
                    .child(div().flex_1().min_h_0().child(self.viewer(&t)))
            })
    }
}

impl Focusable for CrustyApp {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.url.focus_handle(cx)
    }
}

fn status_color(category: StatusCategory, t: &Theme) -> Hsla {
    match category {
        StatusCategory::Success => t.success,
        StatusCategory::Redirection => t.warning,
        StatusCategory::ClientError | StatusCategory::ServerError => t.danger,
        StatusCategory::Informational => t.accent,
        StatusCategory::Unknown => t.text_muted,
    }
}

fn method_color(method: HttpMethod, t: &Theme) -> Hsla {
    match method {
        HttpMethod::Get => t.success,
        HttpMethod::Post => t.warning,
        HttpMethod::Delete => t.danger,
        _ => t.accent,
    }
}

fn format_size(bytes: u64) -> String {
    match bytes {
        b if b < 1024 => format!("{b} B"),
        b if b < 1024 * 1024 => format!("{:.1} KB", b as f64 / 1024.),
        b => format!("{:.1} MB", b as f64 / (1024. * 1024.)),
    }
}
