//! Main render function for the TUI layout.

use crate::app::{App, AuthType, FocusedPane, KvEditMode, RequestTab, ResponseTab, SidebarItem};
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
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BG_PRIMARY)),
        area,
    );

    if app.show_help {
        render_help_overlay(frame, area);
        return;
    }

    if app.curl_import_open {
        render_curl_import_overlay(frame, app, area);
        return;
    }

    if app.codegen_open {
        render_codegen_overlay(frame, app, area);
        return;
    }

    if app.save_dialog_open {
        render_save_dialog(frame, app, area);
        return;
    }

    if app.env_dialog_open {
        render_env_dialog(frame, app, area);
        return;
    }

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

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // URL bar
            Constraint::Min(10),  // Request + Response
            Constraint::Length(1), // Status bar
        ])
        .split(main_chunks[1]);

    render_url_bar(frame, app, right_chunks[0]);

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
    let border_color = if is_focused && app.sidebar_section == 0 { ACCENT_BLUE } else { BORDER };

    // Split sidebar: collections top, history bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Collections
    let col_block = Block::default()
        .title(" Collections ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG_SURFACE));

    let sidebar_items = app.sidebar_items();
    if sidebar_items.is_empty() {
        let hint = if is_focused {
            "  No collections\n  Ctrl+S to save a request"
        } else {
            "  No collections"
        };
        let empty = Paragraph::new(hint)
            .style(Style::default().fg(TEXT_SECONDARY))
            .block(col_block);
        frame.render_widget(empty, chunks[0]);
    } else {
        let items: Vec<ListItem> = sidebar_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = is_focused && i == app.sidebar_selected;
                match item {
                    SidebarItem::Collection { index, name } => {
                        let expanded = *index < app.sidebar_expanded.len()
                            && app.sidebar_expanded[*index];
                        let arrow = if expanded { "▾" } else { "▸" };
                        let style = if is_selected {
                            Style::default().fg(ACCENT_BLUE).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD)
                        };
                        ListItem::new(format!(" {arrow} {name}")).style(style)
                    }
                    SidebarItem::Request { method, name, .. } => {
                        let style = if is_selected {
                            Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::REVERSED)
                        } else {
                            Style::default().fg(TEXT_SECONDARY)
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled("   ", Style::default()),
                            Span::styled(
                                format!("{:>4} ", method),
                                method_color_str(method),
                            ),
                            Span::styled(
                                truncate_url(name, 16),
                                style,
                            ),
                        ]))
                    }
                }
            })
            .collect();
        let list = List::new(items).block(col_block);
        frame.render_widget(list, chunks[0]);
    }

    // History
    let hist_focused = is_focused && app.sidebar_section == 1;
    let hist_border = if hist_focused { ACCENT_BLUE } else { BORDER };

    let hist_title = if app.history_search_active {
        format!(" History [/{}] ", app.history_search_buf)
    } else if !app.history_search_buf.is_empty() {
        format!(" History (filter: {}) ", app.history_search_buf)
    } else {
        " History ".to_string()
    };

    let hist_block = Block::default()
        .title(hist_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(hist_border))
        .style(Style::default().bg(BG_SURFACE));

    let filtered = app.filtered_history();
    if filtered.is_empty() {
        let msg = if app.history_search_buf.is_empty() {
            "  No history"
        } else {
            "  No matching history"
        };
        let empty = Paragraph::new(msg)
            .style(Style::default().fg(TEXT_SECONDARY))
            .block(hist_block);
        frame.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = filtered
            .iter()
            .take(20)
            .enumerate()
            .map(|(i, h)| {
                let is_selected = hist_focused && i == app.history_selected;
                let method_style = method_color_str(&h.method);
                let status_str = h
                    .status
                    .map(|s| format!("{s}"))
                    .unwrap_or_else(|| "ERR".to_string());
                let url_style = if is_selected {
                    Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(TEXT_PRIMARY)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:>4} ", h.method), method_style),
                    Span::styled(
                        truncate_url(&h.url, 18),
                        url_style,
                    ),
                    Span::styled(format!(" {status_str}"), Style::default().fg(TEXT_SECONDARY)),
                ]))
            })
            .collect();
        let list = List::new(items).block(hist_block);
        frame.render_widget(list, chunks[1]);
    }
}

fn render_url_bar(frame: &mut Frame, app: &App, area: Rect) {
    let has_env = app.active_environment().is_some();
    let env_width = if has_env { 14 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(10),        // Method
            Constraint::Min(20),           // URL
            Constraint::Length(env_width), // Env indicator
            Constraint::Length(10),        // Send button
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
    frame.render_widget(
        Paragraph::new(method_text).style(method_style).block(method_block),
        chunks[0],
    );

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

        frame.render_widget(
            List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ACCENT_BLUE))
                    .style(Style::default().bg(BG_ELEVATED)),
            ),
            dropdown_area,
        );
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

    frame.render_widget(
        Paragraph::new(Line::from(vec![url_display])).block(url_block),
        chunks[1],
    );

    if url_focused {
        let cursor_x = chunks[1].x + 1 + app.url_cursor as u16;
        let cursor_y = chunks[1].y + 1;
        frame.set_cursor_position((cursor_x.min(chunks[1].x + chunks[1].width - 2), cursor_y));
    }

    // Environment indicator
    if let Some(env) = app.active_environment() {
        let env_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_SURFACE));
        let env_name = truncate_url(&env.name, 10);
        frame.render_widget(
            Paragraph::new(format!(" {env_name}"))
                .style(Style::default().fg(STATUS_SUCCESS))
                .block(env_block),
            chunks[2],
        );
    }

    // Send button
    let send_style = if app.loading {
        Style::default().fg(BG_PRIMARY).bg(STATUS_WARNING)
    } else {
        Style::default().fg(BG_PRIMARY).bg(ACCENT_BLUE).bold()
    };
    let send_text = if app.loading { " Sending " } else { "  Send  " };
    let send_idx = if has_env { 3 } else { 3 };
    let send_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_SURFACE));
    frame.render_widget(
        Paragraph::new(send_text).style(send_style).block(send_block),
        chunks[send_idx],
    );
}

fn render_request_pane(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Request ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if matches!(app.focus, FocusedPane::KeyValueEditor) {
                ACCENT_BLUE
            } else {
                BORDER
            },
        ))
        .style(Style::default().bg(BG_SURFACE));

    let tabs = Tabs::new(vec!["Params", "Headers", "Body", "Auth", "Script"])
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

    match app.request_tab {
        RequestTab::Params => {
            render_kv_editor(frame, app, &app.params, "Query Parameters", tab_chunks[1]);
        }
        RequestTab::Headers => {
            render_kv_editor(frame, app, &app.headers, "Headers", tab_chunks[1]);
        }
        RequestTab::Body => {
            render_body_editor(frame, app, tab_chunks[1]);
        }
        RequestTab::Auth => {
            render_auth_form(frame, app, tab_chunks[1]);
        }
        RequestTab::Script => {
            render_script_editor(frame, app, tab_chunks[1]);
        }
    }
}

fn render_kv_editor(
    frame: &mut Frame,
    app: &App,
    items: &[crusty_core::request::KeyValue],
    title: &str,
    area: Rect,
) {
    let is_focused = app.focus == FocusedPane::KeyValueEditor;

    if items.is_empty() {
        let hint = if is_focused {
            format!("No {title}. Press 'a' to add one.")
        } else {
            format!("No {title}.")
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(TEXT_SECONDARY)),
            area,
        );
        return;
    }

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, kv)| {
            let is_selected = is_focused && i == app.kv_selected;
            let prefix = if kv.enabled { "●" } else { "○" };

            // If editing this row, show the edit buffer
            if is_selected && app.kv_mode != KvEditMode::Navigate {
                let (key_str, val_str) = match app.kv_mode {
                    KvEditMode::EditKey => (
                        format!("▸{}◂", app.kv_edit_buf),
                        kv.value.clone(),
                    ),
                    KvEditMode::EditValue => (
                        kv.key.clone(),
                        format!("▸{}◂", app.kv_edit_buf),
                    ),
                    _ => (kv.key.clone(), kv.value.clone()),
                };
                return ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {prefix} "),
                        Style::default().fg(ACCENT_BLUE),
                    ),
                    Span::styled(key_str, Style::default().fg(ACCENT_BLUE)),
                    Span::styled(" = ", Style::default().fg(TEXT_SECONDARY)),
                    Span::styled(val_str, Style::default().fg(ACCENT_BLUE)),
                ]));
            }

            let base_style = if is_selected {
                Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::REVERSED)
            } else if kv.enabled {
                Style::default().fg(TEXT_PRIMARY)
            } else {
                Style::default().fg(TEXT_SECONDARY)
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {prefix} "),
                    if kv.enabled {
                        Style::default().fg(STATUS_SUCCESS)
                    } else {
                        Style::default().fg(TEXT_SECONDARY)
                    },
                ),
                Span::styled(&kv.key, base_style),
                Span::styled(" = ", Style::default().fg(TEXT_SECONDARY)),
                Span::styled(&kv.value, base_style),
            ]))
        })
        .collect();

    frame.render_widget(List::new(list_items), area);
}

fn render_body_editor(frame: &mut Frame, app: &App, area: Rect) {
    if !app.body_editing && app.body_input.is_empty() {
        let hint = "No body content. Press 'b' or Tab (from URL) to edit.";
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(TEXT_SECONDARY)),
            area,
        );
        return;
    }

    let border_color = if app.body_editing { ACCENT_BLUE } else { BORDER };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG_SURFACE));

    let display_text = if app.body_editing && app.body_input.is_empty() {
        "".to_string()
    } else {
        app.body_input.clone()
    };

    let inner = block.inner(area);
    frame.render_widget(
        Paragraph::new(display_text)
            .style(Style::default().fg(TEXT_PRIMARY))
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );

    if app.body_editing {
        // Calculate cursor position from body_cursor
        let before_cursor = &app.body_input[..app.body_cursor.min(app.body_input.len())];
        let lines: Vec<&str> = before_cursor.split('\n').collect();
        let cursor_row = lines.len().saturating_sub(1) as u16;
        let cursor_col = lines.last().map(|l| l.len()).unwrap_or(0) as u16;

        let cx = inner.x + cursor_col;
        let cy = inner.y + cursor_row;
        frame.set_cursor_position((
            cx.min(inner.x + inner.width.saturating_sub(1)),
            cy.min(inner.y + inner.height.saturating_sub(1)),
        ));
    }
}

fn render_auth_form(frame: &mut Frame, app: &App, area: Rect) {
    let auth_types = AuthType::all();
    let type_names: Vec<&str> = auth_types.iter().map(|t| t.label()).collect();
    let selected_idx = auth_types
        .iter()
        .position(|t| *t == app.auth_type)
        .unwrap_or(0);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    // Auth type tabs
    let auth_tabs = Tabs::new(type_names)
        .select(selected_idx)
        .style(Style::default().fg(TEXT_SECONDARY))
        .highlight_style(Style::default().fg(STATUS_WARNING).bold())
        .divider("│");
    frame.render_widget(auth_tabs, chunks[0]);

    // Auth fields
    let mut lines: Vec<Line<'_>> = Vec::new();

    match app.auth_type {
        AuthType::None => {
            lines.push(Line::styled(
                "  No authentication",
                Style::default().fg(TEXT_SECONDARY),
            ));
        }
        AuthType::Bearer => {
            let editing = app.auth_editing && app.auth_field_index == 0;
            lines.push(auth_field_line("Token", &app.auth_bearer_token, editing));
        }
        AuthType::Basic => {
            let editing_user = app.auth_editing && app.auth_field_index == 0;
            let editing_pass = app.auth_editing && app.auth_field_index == 1;
            lines.push(auth_field_line("Username", &app.auth_basic_user, editing_user));
            lines.push(auth_field_line("Password", &app.auth_basic_pass, editing_pass));
        }
        AuthType::ApiKey => {
            let editing_key = app.auth_editing && app.auth_field_index == 0;
            let editing_val = app.auth_editing && app.auth_field_index == 1;
            lines.push(auth_field_line("Key", &app.auth_apikey_key, editing_key));
            lines.push(auth_field_line("Value", &app.auth_apikey_value, editing_val));
            let location = if app.auth_apikey_in_header {
                "Header"
            } else {
                "Query Param"
            };
            lines.push(Line::from(vec![
                Span::styled("  Send in: ", Style::default().fg(TEXT_SECONDARY)),
                Span::styled(location, Style::default().fg(TEXT_PRIMARY)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), chunks[1]);
}

fn auth_field_line<'a>(label: &'a str, value: &'a str, editing: bool) -> Line<'a> {
    let val_display = if value.is_empty() && !editing {
        "(empty)"
    } else {
        value
    };
    let val_style = if editing {
        Style::default().fg(ACCENT_BLUE).add_modifier(Modifier::UNDERLINED)
    } else if value.is_empty() {
        Style::default().fg(TEXT_SECONDARY)
    } else {
        Style::default().fg(TEXT_PRIMARY)
    };

    Line::from(vec![
        Span::styled(format!("  {label}: "), Style::default().fg(TEXT_SECONDARY)),
        Span::styled(val_display, val_style),
        if editing {
            Span::styled("▎", Style::default().fg(ACCENT_BLUE))
        } else {
            Span::raw("")
        },
    ])
}

fn render_response_pane(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(response_title(app))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if app.focus == FocusedPane::ResponseBody {
                ACCENT_BLUE
            } else {
                BORDER
            },
        ))
        .style(Style::default().bg(BG_SURFACE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(ref error) = app.error {
        frame.render_widget(
            Paragraph::new(format!("Error: {error}"))
                .style(Style::default().fg(STATUS_ERROR))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    if app.loading {
        frame.render_widget(
            Paragraph::new("  Sending request...").style(Style::default().fg(STATUS_WARNING)),
            inner,
        );
        return;
    }

    let Some(ref response) = app.response else {
        frame.render_widget(
            Paragraph::new("  Send a request to see the response here.")
                .style(Style::default().fg(TEXT_SECONDARY)),
            inner,
        );
        return;
    };

    let tabs = Tabs::new(vec!["Body", "Headers", "Timing", "Tests"])
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
        ResponseTab::Tests => render_test_results(frame, app, tab_chunks[1]),
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
        Span::styled(format!("{time_ms}ms "), Style::default().fg(TEXT_SECONDARY)),
        Span::styled(size, Style::default().fg(TEXT_SECONDARY)),
    ])
}

fn render_response_body(frame: &mut Frame, response: &HttpResponse, scroll: u16, area: Rect) {
    let body_text = response
        .body_json_pretty()
        .or_else(|| response.body_text().map(String::from))
        .unwrap_or_else(|| format!("<binary data, {} bytes>", response.body.len()));

    frame.render_widget(
        Paragraph::new(body_text)
            .style(Style::default().fg(TEXT_PRIMARY))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        area,
    );
}

fn render_response_headers(frame: &mut Frame, response: &HttpResponse, area: Rect) {
    let mut headers: Vec<(&String, &String)> = response.headers.iter().collect();
    headers.sort_by(|a, b| a.0.cmp(b.0));

    let items: Vec<ListItem> = headers
        .iter()
        .map(|&(key, value)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{key}: "), Style::default().fg(ACCENT_BLUE)),
                Span::styled(value.to_string(), Style::default().fg(TEXT_PRIMARY)),
            ]))
        })
        .collect();

    frame.render_widget(List::new(items), area);
}

fn render_response_timing(frame: &mut Frame, response: &HttpResponse, area: Rect) {
    let timing = &response.timing;
    let total_ms = timing.total.as_millis();
    let mut lines: Vec<Line<'_>> = vec![Line::from(vec![
        Span::styled("Total:             ", Style::default().fg(TEXT_SECONDARY)),
        Span::styled(format!("{total_ms}ms"), Style::default().fg(TEXT_PRIMARY).bold()),
    ])];

    if let Some(ttfb) = timing.ttfb {
        let ms = ttfb.as_millis();
        lines.push(Line::from(vec![
            Span::styled("Time to First Byte:", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(format!(" {ms}ms"), Style::default().fg(TEXT_PRIMARY)),
        ]));
    }

    if let Some(ct) = timing.content_transfer {
        let ms = ct.as_millis();
        lines.push(Line::from(vec![
            Span::styled("Content Transfer:  ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(format!(" {ms}ms"), Style::default().fg(TEXT_PRIMARY)),
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

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let hints = if app.script_editing {
        "Type to edit script │ Ctrl+T: Run │ Esc: Stop editing".to_string()
    } else if app.body_editing {
        "Type to edit body │ Esc: Stop editing".to_string()
    } else if app.method_selector_open {
        "↑↓: Select method │ Enter: Confirm │ Esc: Cancel".to_string()
    } else if app.kv_mode != KvEditMode::Navigate {
        "Tab: Next field │ Enter: Save & next │ Esc: Cancel".to_string()
    } else if app.auth_editing {
        "Tab/Enter: Next field │ Esc: Stop editing".to_string()
    } else if app.focus == FocusedPane::KeyValueEditor {
        "a: Add │ d: Delete │ Space: Toggle │ e/Enter: Edit │ Tab: Next pane".to_string()
    } else {
        let env_hint = if !app.environments.is_empty() {
            " │ Ctrl+E: Switch env"
        } else {
            ""
        };
        format!(
            "Tab: Switch pane │ Ctrl+Enter: Send │ Ctrl+I: Import cURL │ Ctrl+G: Code gen │ ?: Help{env_hint} │ q: Quit"
        )
    };

    frame.render_widget(
        Paragraph::new(hints).style(Style::default().fg(TEXT_SECONDARY).bg(BG_ELEVATED)),
        area,
    );
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let help_area = centered_rect(60, 80, area);

    let block = Block::default()
        .title(" Keyboard Shortcuts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_ELEVATED));

    let shortcuts = vec![
        ("Ctrl+Enter / Ctrl+R", "Send request"),
        ("Tab / Shift+Tab", "Cycle panes"),
        ("Ctrl+B", "Toggle sidebar"),
        ("Ctrl+E", "Switch environment"),
        ("m", "Open method selector"),
        ("1-5", "Switch request tabs (5=Script)"),
        ("F1-F4", "Switch response tabs (F4=Tests)"),
        ("Ctrl+T", "Run test script"),
        ("j/k or ↑/↓", "Scroll / navigate"),
        ("a", "Add key-value row (in editor)"),
        ("d", "Delete key-value row"),
        ("Space", "Toggle row enabled/disabled"),
        ("e / Enter", "Edit selected row"),
        ("Ctrl+I", "Import cURL command"),
        ("Ctrl+G", "Generate code snippet"),
        ("Ctrl+S", "Save request to collection"),
        ("Ctrl+N", "Environment editor"),
        ("g / G", "Scroll to top / bottom"),
        ("?", "Toggle this help"),
        ("q / Ctrl+C", "Quit"),
    ];

    let lines: Vec<Line> = shortcuts
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(format!("  {key:<24}"), Style::default().fg(ACCENT_BLUE).bold()),
                Span::styled(*desc, Style::default().fg(TEXT_PRIMARY)),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), help_area);
}

fn render_curl_import_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 30, area);

    frame.render_widget(
        Block::default().style(Style::default().bg(BG_PRIMARY)),
        area,
    );

    let block = Block::default()
        .title(" Import cURL Command ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_ELEVATED));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Instruction
            Constraint::Length(3), // Input
            Constraint::Min(1),   // Error / status
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new("  Paste a cURL command and press Enter to import:")
            .style(Style::default().fg(TEXT_SECONDARY)),
        chunks[0],
    );

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_SURFACE));

    let input_display = if app.curl_import_buf.is_empty() {
        Span::styled("curl https://...", Style::default().fg(TEXT_SECONDARY))
    } else {
        Span::styled(&app.curl_import_buf, Style::default().fg(TEXT_PRIMARY))
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![input_display])).block(input_block),
        chunks[1],
    );

    // Cursor
    let cursor_x = chunks[1].x + 1 + app.curl_import_cursor as u16;
    let cursor_y = chunks[1].y + 1;
    frame.set_cursor_position((cursor_x.min(chunks[1].x + chunks[1].width - 2), cursor_y));

    // Error message
    if let Some(ref err) = app.curl_import_error {
        frame.render_widget(
            Paragraph::new(format!("  Error: {err}"))
                .style(Style::default().fg(STATUS_ERROR)),
            chunks[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new("  Enter: Import │ Esc: Cancel")
                .style(Style::default().fg(TEXT_SECONDARY)),
            chunks[2],
        );
    }
}

fn render_env_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 70, area);

    frame.render_widget(
        Block::default().style(Style::default().bg(BG_PRIMARY)),
        area,
    );

    let env_name = app
        .environments
        .get(app.env_dialog_index)
        .map(|e| e.name.as_str())
        .unwrap_or("(none)");

    let title = format!(
        " Environment: {} ({}/{}) ",
        if app.env_editing_name {
            &app.env_name_buf
        } else {
            env_name
        },
        app.env_dialog_index + 1,
        app.environments.len()
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_ELEVATED));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),   // Variables
            Constraint::Length(1), // Hints
        ])
        .split(inner);

    // Variables list
    if let Some(env) = app.environments.get(app.env_dialog_index) {
        if env.variables.is_empty() {
            frame.render_widget(
                Paragraph::new("  No variables. Press 'a' to add one.")
                    .style(Style::default().fg(TEXT_SECONDARY)),
                chunks[0],
            );
        } else {
            let items: Vec<ListItem> = env
                .variables
                .iter()
                .enumerate()
                .map(|(i, var)| {
                    let is_selected = i == app.env_var_selected;
                    let prefix = if var.enabled { "●" } else { "○" };

                    // If editing this row
                    if is_selected && app.env_var_edit_mode > 0 {
                        let (key_str, val_str) = match app.env_var_edit_mode {
                            1 => (
                                format!("▸{}◂", app.env_var_edit_buf),
                                var.value.reveal().to_string(),
                            ),
                            2 => (
                                var.key.clone(),
                                format!("▸{}◂", app.env_var_edit_buf),
                            ),
                            _ => (var.key.clone(), var.value.reveal().to_string()),
                        };
                        return ListItem::new(Line::from(vec![
                            Span::styled(format!(" {prefix} "), Style::default().fg(ACCENT_BLUE)),
                            Span::styled(key_str, Style::default().fg(ACCENT_BLUE)),
                            Span::styled(" = ", Style::default().fg(TEXT_SECONDARY)),
                            Span::styled(val_str, Style::default().fg(ACCENT_BLUE)),
                        ]));
                    }

                    let base_style = if is_selected {
                        Style::default()
                            .fg(TEXT_PRIMARY)
                            .add_modifier(Modifier::REVERSED)
                    } else if var.enabled {
                        Style::default().fg(TEXT_PRIMARY)
                    } else {
                        Style::default().fg(TEXT_SECONDARY)
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {prefix} "),
                            if var.enabled {
                                Style::default().fg(STATUS_SUCCESS)
                            } else {
                                Style::default().fg(TEXT_SECONDARY)
                            },
                        ),
                        Span::styled(&var.key, base_style),
                        Span::styled(" = ", Style::default().fg(TEXT_SECONDARY)),
                        Span::styled(var.value.reveal(), base_style),
                    ]))
                })
                .collect();

            let var_block = Block::default()
                .title(" Variables ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(BG_SURFACE));

            frame.render_widget(List::new(items).block(var_block), chunks[0]);
        }
    }

    // Hints
    let hints = if app.env_editing_name {
        "Enter: Save name │ Esc: Cancel"
    } else if app.env_var_edit_mode > 0 {
        "Tab: Next field │ Enter: Save │ Esc: Cancel"
    } else {
        "a: Add var │ d: Delete │ e: Edit │ Space: Toggle │ n: Rename │ N: New env │ h/l: Switch env │ Esc: Save & close"
    };
    frame.render_widget(
        Paragraph::new(format!("  {hints}")).style(Style::default().fg(TEXT_SECONDARY)),
        chunks[1],
    );
}

fn render_save_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(50, 40, area);

    frame.render_widget(
        Block::default().style(Style::default().bg(BG_PRIMARY)),
        area,
    );

    let block = Block::default()
        .title(" Save Request to Collection ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_ELEVATED));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Label
            Constraint::Length(3), // Name input
            Constraint::Length(1), // Label
            Constraint::Min(3),   // Collection list
            Constraint::Length(1), // Hints
        ])
        .split(inner);

    // Name label
    frame.render_widget(
        Paragraph::new("  Request name:")
            .style(Style::default().fg(TEXT_SECONDARY)),
        chunks[0],
    );

    // Name input
    let name_border = if app.save_editing_name { ACCENT_BLUE } else { BORDER };
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(name_border))
        .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(
        Paragraph::new(app.save_name_buf.as_str())
            .style(Style::default().fg(TEXT_PRIMARY))
            .block(name_block),
        chunks[1],
    );

    if app.save_editing_name {
        let cx = chunks[1].x + 1 + app.save_name_cursor as u16;
        let cy = chunks[1].y + 1;
        frame.set_cursor_position((cx.min(chunks[1].x + chunks[1].width - 2), cy));
    }

    // Collection label
    let col_label = if app.collections.is_empty() {
        "  Will create 'Default' collection"
    } else {
        "  Save to collection:"
    };
    frame.render_widget(
        Paragraph::new(col_label).style(Style::default().fg(TEXT_SECONDARY)),
        chunks[2],
    );

    // Collection list
    if !app.collections.is_empty() {
        let items: Vec<ListItem> = app
            .collections
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let is_selected = !app.save_editing_name && i == app.save_collection_index;
                let style = if is_selected {
                    Style::default().fg(ACCENT_BLUE).add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(TEXT_PRIMARY)
                };
                ListItem::new(format!("  {} ({})", col.name, col.request_count())).style(style)
            })
            .collect();

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if app.save_editing_name { BORDER } else { ACCENT_BLUE }))
            .style(Style::default().bg(BG_SURFACE));

        frame.render_widget(List::new(items).block(list_block), chunks[3]);
    }

    // Hints
    frame.render_widget(
        Paragraph::new("  Enter: Save │ Tab: Switch field │ Esc: Cancel")
            .style(Style::default().fg(TEXT_SECONDARY)),
        chunks[4],
    );
}

fn render_codegen_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(80, 80, area);

    frame.render_widget(
        Block::default().style(Style::default().bg(BG_PRIMARY)),
        area,
    );

    let block = Block::default()
        .title(" Generate Code ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .style(Style::default().bg(BG_ELEVATED));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(30)])
        .split(inner);

    // Language selector
    let langs = crusty_export::codegen::Language::all();
    let lang_items: Vec<ListItem> = langs
        .iter()
        .enumerate()
        .map(|(i, lang)| {
            let style = if i == app.codegen_lang_index {
                Style::default()
                    .fg(ACCENT_BLUE)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(TEXT_PRIMARY)
            };
            ListItem::new(format!("  {} ", lang.label())).style(style)
        })
        .collect();

    let lang_block = Block::default()
        .title(" Language ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(List::new(lang_items).block(lang_block), chunks[0]);

    // Generated code
    let selected_lang = langs[app.codegen_lang_index];
    let def = app.build_request_definition();
    let code = crusty_export::codegen::generate(&def, selected_lang);

    let code_block = Block::default()
        .title(format!(" {} ", selected_lang.label()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(
        Paragraph::new(code)
            .style(Style::default().fg(TEXT_PRIMARY))
            .wrap(Wrap { trim: false })
            .block(code_block),
        chunks[1],
    );
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
        HttpMethod::Patch => Color::Rgb(163, 113, 247),
        HttpMethod::Delete => STATUS_ERROR,
        _ => TEXT_SECONDARY,
    };
    Style::default().fg(color).bold()
}

fn render_script_editor(frame: &mut Frame, app: &App, area: Rect) {
    if !app.script_editing && app.script_input.is_empty() {
        let hint = "No test script. Press Tab (from URL) or 5 to switch here.\nCtrl+T runs the script against the current response.\n\nExample:\n  test(\"Status is 200\", status == 200);\n  let body = json_parse(response_body);\n  assert_eq(\"Has data\", body[\"count\"], 10);";
        frame.render_widget(
            Paragraph::new(hint)
                .style(Style::default().fg(TEXT_SECONDARY))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    let border_color = if app.script_editing { ACCENT_BLUE } else { BORDER };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Rhai Script (Ctrl+T to run) ")
        .title_style(Style::default().fg(TEXT_SECONDARY))
        .style(Style::default().bg(BG_SURFACE));

    let inner = block.inner(area);
    frame.render_widget(
        Paragraph::new(app.script_input.clone())
            .style(Style::default().fg(TEXT_PRIMARY))
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );

    if app.script_editing {
        let before_cursor = &app.script_input[..app.script_cursor.min(app.script_input.len())];
        let lines: Vec<&str> = before_cursor.split('\n').collect();
        let cursor_row = lines.len().saturating_sub(1) as u16;
        let cursor_col = lines.last().map(|l| l.len()).unwrap_or(0) as u16;

        let cx = inner.x + cursor_col;
        let cy = inner.y + cursor_row;
        frame.set_cursor_position((
            cx.min(inner.x + inner.width.saturating_sub(1)),
            cy.min(inner.y + inner.height.saturating_sub(1)),
        ));
    }
}

fn render_test_results(frame: &mut Frame, app: &App, area: Rect) {
    let Some(ref results) = app.test_results else {
        frame.render_widget(
            Paragraph::new("  No test results. Write a script (tab 5) and press Ctrl+T to run.")
                .style(Style::default().fg(TEXT_SECONDARY)),
            area,
        );
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    // Summary line
    let summary_color = if results.failed_tests == 0 {
        STATUS_SUCCESS
    } else {
        STATUS_ERROR
    };
    lines.push(Line::from(vec![
        Span::styled(
            format!(
                " {} passed, {} failed ",
                results.passed_tests, results.failed_tests
            ),
            Style::default().fg(summary_color).bold(),
        ),
        Span::styled(
            format!("({}ms)", results.total_duration_ms),
            Style::default().fg(TEXT_SECONDARY),
        ),
    ]));
    lines.push(Line::from(""));

    // Per-request results
    for req_result in &results.request_results {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} {} ", req_result.method, req_result.name),
                Style::default().fg(TEXT_PRIMARY).bold(),
            ),
            Span::styled(
                format!(
                    "[{}] {}ms",
                    req_result.status.map(|s| s.to_string()).unwrap_or_else(|| "ERR".to_string()),
                    req_result.duration_ms
                ),
                Style::default().fg(TEXT_SECONDARY),
            ),
        ]));

        for test in &req_result.tests {
            let (icon, color) = if test.passed {
                ("  PASS", STATUS_SUCCESS)
            } else {
                ("  FAIL", STATUS_ERROR)
            };
            let mut spans = vec![
                Span::styled(format!("{icon} "), Style::default().fg(color).bold()),
                Span::styled(&test.name, Style::default().fg(TEXT_PRIMARY)),
            ];
            if let Some(ref err) = test.error {
                spans.push(Span::styled(
                    format!(" - {err}"),
                    Style::default().fg(STATUS_ERROR),
                ));
            }
            lines.push(Line::from(spans));
        }

        if let Some(ref err) = req_result.error {
            lines.push(Line::from(Span::styled(
                format!("  ERROR: {err}"),
                Style::default().fg(STATUS_ERROR),
            )));
        }

        // Show logs if any
        if !req_result.logs.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Logs:",
                Style::default().fg(TEXT_SECONDARY).add_modifier(Modifier::ITALIC),
            )));
            for log in &req_result.logs {
                lines.push(Line::from(Span::styled(
                    format!("    {log}"),
                    Style::default().fg(TEXT_SECONDARY),
                )));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(TEXT_PRIMARY))
            .scroll((app.test_result_scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn method_color_str(method: &str) -> Style {
    let color = match method {
        "GET" => STATUS_SUCCESS,
        "POST" => STATUS_WARNING,
        "PUT" => ACCENT_BLUE,
        "PATCH" => Color::Rgb(163, 113, 247),
        "DELETE" => STATUS_ERROR,
        _ => TEXT_SECONDARY,
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

fn truncate_url(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
