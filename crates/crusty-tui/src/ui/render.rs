//! Main render function for the TUI layout.

use crate::app::{App, FocusedPane, ResponseTab};
use crusty_core::response::{HttpResponse, StatusCategory};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap};
use ratatui::Frame;

// Design system colors from PRD
const BG_PRIMARY: Color = Color::Rgb(13, 17, 23); // #0D1117
const BG_SURFACE: Color = Color::Rgb(22, 27, 34); // #161B22
const BG_ELEVATED: Color = Color::Rgb(33, 38, 45); // #21262D
const BORDER: Color = Color::Rgb(48, 54, 61); // #30363D
const TEXT_PRIMARY: Color = Color::Rgb(230, 237, 243); // #E6EDF3
const TEXT_SECONDARY: Color = Color::Rgb(139, 148, 158); // #8B949E
const ACCENT_BLUE: Color = Color::Rgb(88, 166, 255); // #58A6FF
const STATUS_SUCCESS: Color = Color::Rgb(63, 185, 80); // #3FB950
const STATUS_WARNING: Color = Color::Rgb(210, 153, 34); // #D29922
const STATUS_ERROR: Color = Color::Rgb(248, 81, 73); // #F85149

/// Render the entire TUI.
pub fn render(frame: &mut Frame, app: &App) {
    // Background
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BG_PRIMARY)),
        area,
    );

    if app.show_help {
        render_help_overlay(frame, area);
        return;
    }

    // Main layout: optional sidebar | main content
    let main_chunks = if app.sidebar_visible {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(40)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(0), Constraint::Min(40)])
            .split(area)
    };

    if app.sidebar_visible {
        render_sidebar(frame, app, main_chunks[0]);
    }

    // Right side: URL bar + request/response split
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // URL bar
            Constraint::Min(10),  // Request + Response
            Constraint::Length(1), // Status bar
        ])
        .split(main_chunks[1]);

    render_url_bar(frame, app, right_chunks[0]);

    // Request and response split vertically
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(right_chunks[1]);

    render_request_pane(frame, app, content_chunks[0]);
    render_response_pane(frame, app, content_chunks[1]);
    render_status_bar(frame, app, right_chunks[2]);
}

fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == FocusedPane::Sidebar;
    let border_color = if is_focused { ACCENT_BLUE } else { BORDER };

    let block = Block::default()
        .title(" Collections ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG_SURFACE));

    if app.collections.is_empty() {
        let empty = Paragraph::new("  No collections yet")
            .style(Style::default().fg(TEXT_SECONDARY))
            .block(block);
        frame.render_widget(empty, area);
    } else {
        let items: Vec<ListItem> = app
            .collections
            .iter()
            .map(|c| {
                ListItem::new(format!("  📁 {}", c.name))
                    .style(Style::default().fg(TEXT_PRIMARY))
            })
            .collect();
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

fn render_url_bar(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(10), // Method
            Constraint::Min(20),   // URL
            Constraint::Length(10), // Send button
        ])
        .split(area);

    // Method selector
    let method_style = method_color(app.method);
    let method_text = format!(" {} ", app.method.as_str());
    let method_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.focus == FocusedPane::MethodSelector {
            ACCENT_BLUE
        } else {
            BORDER
        }))
        .style(Style::default().bg(BG_SURFACE));
    let method_widget = Paragraph::new(method_text)
        .style(method_style)
        .block(method_block);
    frame.render_widget(method_widget, chunks[0]);

    // Method dropdown overlay
    if app.method_selector_open {
        let methods = crusty_core::request::HttpMethod::all();
        let dropdown_area = Rect {
            x: chunks[0].x,
            y: chunks[0].y + 3,
            width: chunks[0].width.max(12),
            height: (methods.len() as u16 + 2).min(area.height),
        };

        let items: Vec<ListItem> = methods
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let style = if i == app.method_selector_index {
                    method_color(*m).add_modifier(Modifier::REVERSED)
                } else {
                    method_color(*m)
                };
                ListItem::new(format!(" {} ", m.as_str())).style(style)
            })
            .collect();

        let dropdown = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT_BLUE))
                .style(Style::default().bg(BG_ELEVATED)),
        );
        frame.render_widget(dropdown, dropdown_area);
    }

    // URL input
    let url_focused = app.focus == FocusedPane::UrlBar;
    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if url_focused { ACCENT_BLUE } else { BORDER }))
        .style(Style::default().bg(BG_SURFACE));

    let url_display = if app.url_input.is_empty() {
        Span::styled("Enter URL...", Style::default().fg(TEXT_SECONDARY))
    } else {
        Span::styled(&app.url_input, Style::default().fg(TEXT_PRIMARY))
    };

    let url_widget = Paragraph::new(Line::from(vec![url_display])).block(url_block);
    frame.render_widget(url_widget, chunks[1]);

    // Show cursor when URL bar is focused
    if url_focused {
        let cursor_x = chunks[1].x + 1 + app.url_cursor as u16;
        let cursor_y = chunks[1].y + 1;
        frame.set_cursor_position((cursor_x.min(chunks[1].x + chunks[1].width - 2), cursor_y));
    }

    // Send button
    let send_style = if app.loading {
        Style::default().fg(BG_PRIMARY).bg(STATUS_WARNING)
    } else {
        Style::default().fg(BG_PRIMARY).bg(ACCENT_BLUE).bold()
    };

    let send_text = if app.loading { " Sending " } else { "  Send  " };
    let send_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_SURFACE));
    let send_widget = Paragraph::new(send_text).style(send_style).block(send_block);
    frame.render_widget(send_widget, chunks[2]);
}

fn render_request_pane(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Request ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_SURFACE));

    let tabs = Tabs::new(vec!["Params", "Headers", "Body", "Auth"])
        .select(app.request_tab as usize)
        .style(Style::default().fg(TEXT_SECONDARY))
        .highlight_style(Style::default().fg(ACCENT_BLUE).add_modifier(Modifier::BOLD))
        .divider("│");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tab_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    frame.render_widget(tabs, tab_chunks[0]);

    // Tab content
    match app.request_tab {
        crate::app::RequestTab::Params => {
            render_key_value_list(frame, &app.params, "Query Parameters", tab_chunks[1]);
        }
        crate::app::RequestTab::Headers => {
            render_key_value_list(frame, &app.headers, "Headers", tab_chunks[1]);
        }
        crate::app::RequestTab::Body => {
            let body_text = if app.body_input.is_empty() {
                "No body content. Press 'e' to edit.".to_string()
            } else {
                app.body_input.clone()
            };
            let body_widget = Paragraph::new(body_text)
                .style(Style::default().fg(TEXT_SECONDARY))
                .wrap(Wrap { trim: false });
            frame.render_widget(body_widget, tab_chunks[1]);
        }
        crate::app::RequestTab::Auth => {
            let auth_text = "No authentication configured.";
            let auth_widget = Paragraph::new(auth_text)
                .style(Style::default().fg(TEXT_SECONDARY));
            frame.render_widget(auth_widget, tab_chunks[1]);
        }
    }
}

fn render_key_value_list(frame: &mut Frame, items: &[crusty_core::request::KeyValue], title: &str, area: Rect) {
    if items.is_empty() {
        let text = format!("No {title} configured.");
        let widget = Paragraph::new(text).style(Style::default().fg(TEXT_SECONDARY));
        frame.render_widget(widget, area);
    } else {
        let list_items: Vec<ListItem> = items
            .iter()
            .map(|kv| {
                let style = if kv.enabled {
                    Style::default().fg(TEXT_PRIMARY)
                } else {
                    Style::default().fg(TEXT_SECONDARY)
                };
                let prefix = if kv.enabled { "✓" } else { "✗" };
                ListItem::new(format!(" {prefix} {}: {}", kv.key, kv.value)).style(style)
            })
            .collect();
        let list = List::new(list_items);
        frame.render_widget(list, area);
    }
}

fn render_response_pane(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(response_title(app))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_SURFACE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(ref error) = app.error {
        let error_widget = Paragraph::new(format!("Error: {error}"))
            .style(Style::default().fg(STATUS_ERROR))
            .wrap(Wrap { trim: false });
        frame.render_widget(error_widget, inner);
        return;
    }

    if app.loading {
        let loading_widget = Paragraph::new("  Sending request...")
            .style(Style::default().fg(STATUS_WARNING));
        frame.render_widget(loading_widget, inner);
        return;
    }

    let Some(ref response) = app.response else {
        let placeholder = Paragraph::new("  Send a request to see the response here.")
            .style(Style::default().fg(TEXT_SECONDARY));
        frame.render_widget(placeholder, inner);
        return;
    };

    // Response tabs
    let tabs = Tabs::new(vec!["Body", "Headers", "Timing"])
        .select(app.response_tab as usize)
        .style(Style::default().fg(TEXT_SECONDARY))
        .highlight_style(Style::default().fg(ACCENT_BLUE).add_modifier(Modifier::BOLD))
        .divider("│");

    let tab_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    frame.render_widget(tabs, tab_chunks[0]);

    match app.response_tab {
        ResponseTab::Body => render_response_body(frame, response, app.response_scroll, tab_chunks[1]),
        ResponseTab::Headers => render_response_headers(frame, response, tab_chunks[1]),
        ResponseTab::Timing => render_response_timing(frame, response, tab_chunks[1]),
    }
}

fn response_title(app: &App) -> Line<'static> {
    let Some(ref response) = app.response else {
        return Line::from(" Response ");
    };

    let status_color = match response.status_category() {
        StatusCategory::Informational => ACCENT_BLUE,
        StatusCategory::Success => STATUS_SUCCESS,
        StatusCategory::Redirection => STATUS_WARNING,
        StatusCategory::ClientError => STATUS_ERROR,
        StatusCategory::ServerError => Color::Rgb(200, 40, 40),
        StatusCategory::Unknown => TEXT_SECONDARY,
    };

    let time_ms = response.timing.total.as_millis();
    let size = format_size(response.size.body_size);

    Line::from(vec![
        Span::raw(" Response "),
        Span::styled(
            format!("{} {} ", response.status, response.status_text),
            Style::default().fg(status_color).bold(),
        ),
        Span::styled(
            format!("{}ms ", time_ms),
            Style::default().fg(TEXT_SECONDARY),
        ),
        Span::styled(size, Style::default().fg(TEXT_SECONDARY)),
    ])
}

fn render_response_body(frame: &mut Frame, response: &HttpResponse, scroll: u16, area: Rect) {
    let body_text = response
        .body_json_pretty()
        .or_else(|| response.body_text().map(String::from))
        .unwrap_or_else(|| format!("<binary data, {} bytes>", response.body.len()));

    let widget = Paragraph::new(body_text)
        .style(Style::default().fg(TEXT_PRIMARY))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(widget, area);
}

fn render_response_headers(frame: &mut Frame, response: &HttpResponse, area: Rect) {
    let mut headers: Vec<(&String, &String)> = response.headers.iter().collect();
    headers.sort_by(|a, b| a.0.cmp(b.0));

    let items: Vec<ListItem> = headers
        .iter()
        .map(|&(key, value)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{key}: "),
                    Style::default().fg(ACCENT_BLUE),
                ),
                Span::styled(value.to_string(), Style::default().fg(TEXT_PRIMARY)),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
}

fn render_response_timing(frame: &mut Frame, response: &HttpResponse, area: Rect) {
    let timing = &response.timing;
    let total_ms = timing.total.as_millis();
    let mut lines: Vec<Line<'_>> = vec![
        Line::from(vec![
            Span::styled("Total:             ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(
                format!("{total_ms}ms"),
                Style::default().fg(TEXT_PRIMARY).bold(),
            ),
        ]),
    ];

    if let Some(ttfb) = timing.ttfb {
        let ms = ttfb.as_millis();
        lines.push(Line::from(vec![
            Span::styled("Time to First Byte:", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(
                format!(" {ms}ms"),
                Style::default().fg(TEXT_PRIMARY),
            ),
        ]));
    }

    if let Some(ct) = timing.content_transfer {
        let ms = ct.as_millis();
        lines.push(Line::from(vec![
            Span::styled("Content Transfer:  ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(
                format!(" {ms}ms"),
                Style::default().fg(TEXT_PRIMARY),
            ),
        ]));
    }

    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled("Headers Size:      ", Style::default().fg(TEXT_SECONDARY)),
        Span::styled(
            format!(" {}", format_size(response.size.headers_size)),
            Style::default().fg(TEXT_PRIMARY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Body Size:         ", Style::default().fg(TEXT_SECONDARY)),
        Span::styled(
            format!(" {}", format_size(response.size.body_size)),
            Style::default().fg(TEXT_PRIMARY),
        ),
    ]));

    let widget = Paragraph::new(lines).style(Style::default().fg(TEXT_PRIMARY));
    frame.render_widget(widget, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let hints = if app.method_selector_open {
        "↑↓: Select method │ Enter: Confirm │ Esc: Cancel"
    } else {
        "Tab: Switch pane │ Ctrl+Enter: Send │ ?: Help │ Ctrl+B: Toggle sidebar │ q: Quit"
    };

    let status = Paragraph::new(hints)
        .style(Style::default().fg(TEXT_SECONDARY).bg(BG_ELEVATED));
    frame.render_widget(status, area);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let help_area = centered_rect(60, 70, area);

    let block = Block::default()
        .title(" Keyboard Shortcuts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_ELEVATED));

    let shortcuts = vec![
        ("Ctrl+Enter / Ctrl+R", "Send request"),
        ("Tab / Shift+Tab", "Cycle panes"),
        ("Ctrl+B", "Toggle sidebar"),
        ("m", "Open method selector"),
        ("1-4", "Switch request tabs (Params/Headers/Body/Auth)"),
        ("F1-F3", "Switch response tabs (Body/Headers/Timing)"),
        ("j/k or ↑/↓", "Scroll response"),
        ("y", "Copy response body"),
        ("?", "Toggle this help"),
        ("q / Ctrl+C", "Quit"),
    ];

    let lines: Vec<Line> = shortcuts
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("  {key:<24}"),
                    Style::default().fg(ACCENT_BLUE).bold(),
                ),
                Span::styled(*desc, Style::default().fg(TEXT_PRIMARY)),
            ])
        })
        .collect();

    let widget = Paragraph::new(lines).block(block);
    frame.render_widget(widget, help_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn method_color(method: crusty_core::request::HttpMethod) -> Style {
    use crusty_core::request::HttpMethod;
    let color = match method {
        HttpMethod::Get => STATUS_SUCCESS,
        HttpMethod::Post => STATUS_WARNING,
        HttpMethod::Put => ACCENT_BLUE,
        HttpMethod::Patch => Color::Rgb(163, 113, 247), // Purple
        HttpMethod::Delete => STATUS_ERROR,
        HttpMethod::Head => TEXT_SECONDARY,
        HttpMethod::Options => TEXT_SECONDARY,
        HttpMethod::Trace => TEXT_SECONDARY,
        HttpMethod::Connect => TEXT_SECONDARY,
    };
    Style::default().fg(color).bold()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
