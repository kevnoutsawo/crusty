//! Crusty TUI — Terminal UI frontend for the Crusty HTTP client.

mod app;
mod ui;

use app::App;
use crossterm::event::{self as ct_event, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use std::io::{self, stdout};
use std::time::Duration;

#[tokio::main]
async fn main() -> miette::Result<()> {
    enable_raw_mode().map_err(|e| miette::miette!("Failed to enable raw mode: {e}"))?;
    stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| miette::miette!("Failed to enter alternate screen: {e}"))?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))
        .map_err(|e| miette::miette!("Failed to create terminal: {e}"))?;

    let result = run(&mut terminal).await;

    disable_raw_mode().ok();
    stdout().execute(LeaveAlternateScreen).ok();

    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> miette::Result<()> {
    let mut app = App::new();

    loop {
        terminal
            .draw(|frame| ui::render(frame, &app))
            .map_err(|e| miette::miette!("Render error: {e}"))?;

        if ct_event::poll(Duration::from_millis(50))
            .map_err(|e| miette::miette!("Event poll error: {e}"))?
        {
            if let Event::Key(key) = ct_event::read()
                .map_err(|e| miette::miette!("Event read error: {e}"))?
            {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Check for send request before general handling
                let should_send = matches!(
                    (key.modifiers, key.code),
                    (KeyModifiers::CONTROL, KeyCode::Enter)
                        | (KeyModifiers::CONTROL, KeyCode::Char('r'))
                );

                if should_send && !app.loading && !app.url_input.trim().is_empty() {
                    send_request(&mut app).await;
                    continue;
                }

                if !handle_key_event(&mut app, key) {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn handle_key_event(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    use crate::app::{FocusedPane, RequestTab, ResponseTab};

    // Global: Ctrl+C quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return false;
    }

    // Global: Ctrl+B toggles sidebar
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        app.sidebar_visible = !app.sidebar_visible;
        return true;
    }

    // Help toggle
    if key.code == KeyCode::Char('?') && app.focus != FocusedPane::UrlBar {
        app.show_help = !app.show_help;
        return true;
    }

    if app.show_help {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') {
            app.show_help = false;
        }
        return true;
    }

    // Method selector dropdown
    if app.method_selector_open {
        let methods = crusty_core::request::HttpMethod::all();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.method_selector_index = app.method_selector_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.method_selector_index =
                    (app.method_selector_index + 1).min(methods.len() - 1);
            }
            KeyCode::Enter => {
                app.method = methods[app.method_selector_index];
                app.method_selector_open = false;
            }
            KeyCode::Esc => {
                app.method_selector_open = false;
            }
            _ => {}
        }
        return true;
    }

    // Focus-specific handling
    match app.focus {
        FocusedPane::UrlBar => {
            match (key.modifiers, key.code) {
                (mods, KeyCode::Char(c)) if mods.is_empty() || mods == KeyModifiers::SHIFT => {
                    app.url_input.insert(app.url_cursor, c);
                    app.url_cursor += 1;
                }
                (_, KeyCode::Backspace) => {
                    if app.url_cursor > 0 {
                        app.url_cursor -= 1;
                        app.url_input.remove(app.url_cursor);
                    }
                }
                (_, KeyCode::Delete) => {
                    if app.url_cursor < app.url_input.len() {
                        app.url_input.remove(app.url_cursor);
                    }
                }
                (_, KeyCode::Left) => {
                    app.url_cursor = app.url_cursor.saturating_sub(1);
                }
                (_, KeyCode::Right) => {
                    app.url_cursor = (app.url_cursor + 1).min(app.url_input.len());
                }
                (_, KeyCode::Home) => app.url_cursor = 0,
                (_, KeyCode::End) => app.url_cursor = app.url_input.len(),
                (_, KeyCode::Tab) => app.focus = FocusedPane::ResponseBody,
                (_, KeyCode::Esc) => app.focus = FocusedPane::ResponseBody,
                _ => {}
            }
        }

        FocusedPane::ResponseBody => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                app.response_scroll = app.response_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.response_scroll = app.response_scroll.saturating_sub(1);
            }
            KeyCode::Char('q') => return false,
            KeyCode::Char('m') => {
                app.method_selector_open = true;
                let methods = crusty_core::request::HttpMethod::all();
                app.method_selector_index =
                    methods.iter().position(|m| *m == app.method).unwrap_or(0);
            }
            KeyCode::Tab => {
                app.focus = if app.sidebar_visible {
                    FocusedPane::Sidebar
                } else {
                    FocusedPane::UrlBar
                };
            }
            KeyCode::BackTab => app.focus = FocusedPane::UrlBar,
            KeyCode::Char('1') => app.request_tab = RequestTab::Params,
            KeyCode::Char('2') => app.request_tab = RequestTab::Headers,
            KeyCode::Char('3') => app.request_tab = RequestTab::Body,
            KeyCode::Char('4') => app.request_tab = RequestTab::Auth,
            KeyCode::F(1) => app.response_tab = ResponseTab::Body,
            KeyCode::F(2) => app.response_tab = ResponseTab::Headers,
            KeyCode::F(3) => app.response_tab = ResponseTab::Timing,
            KeyCode::Char('i') | KeyCode::Enter => app.focus = FocusedPane::UrlBar,
            _ => {}
        },

        FocusedPane::Sidebar => match key.code {
            KeyCode::Char('q') => return false,
            KeyCode::Tab => app.focus = FocusedPane::UrlBar,
            _ => {}
        },

        _ => match key.code {
            KeyCode::Char('q') => return false,
            KeyCode::Tab => app.focus = FocusedPane::UrlBar,
            _ => {}
        },
    }

    true
}

async fn send_request(app: &mut App) {
    app.loading = true;
    app.error = None;
    app.response = None;
    app.response_scroll = 0;

    match app.build_request() {
        Ok(resolved) => match app.http_client.execute(&resolved).await {
            Ok(response) => {
                app.response = Some(response);
                app.loading = false;
            }
            Err(e) => {
                app.error = Some(e.to_string());
                app.loading = false;
            }
        },
        Err(e) => {
            app.error = Some(e.to_string());
            app.loading = false;
        }
    }
}
