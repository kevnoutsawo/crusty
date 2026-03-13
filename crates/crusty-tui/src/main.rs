//! Crusty TUI — Terminal UI frontend for the Crusty HTTP client.

mod app;
mod ui;

use app::{App, AuthType, FocusedPane, KvEditMode, RequestTab, ResponseTab};
use crossterm::event::{self as ct_event, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use crusty_core::request::KeyValue;
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
    app.load_history();

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

                // Send request: Ctrl+Enter or Ctrl+R (not while editing)
                let is_editing = app.kv_mode != KvEditMode::Navigate || app.auth_editing;
                let should_send = !is_editing
                    && matches!(
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
    // Global: Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return false;
    }

    // If editing a key-value field, handle that first
    if app.kv_mode != KvEditMode::Navigate {
        return handle_kv_edit(app, key);
    }

    // If editing an auth field, handle that
    if app.auth_editing {
        return handle_auth_edit(app, key);
    }

    // Global: Ctrl+B toggles sidebar
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        app.sidebar_visible = !app.sidebar_visible;
        return true;
    }

    // Global: Ctrl+E cycles environment
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
        cycle_environment(app);
        return true;
    }

    // Help toggle (not while typing in URL)
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
        return handle_method_selector(app, key);
    }

    // Focus-specific handling
    match app.focus {
        FocusedPane::UrlBar => handle_url_input(app, key),
        FocusedPane::ResponseBody => handle_response(app, key),
        FocusedPane::Sidebar => handle_sidebar(app, key),
        FocusedPane::KeyValueEditor => handle_kv_navigate(app, key),
        _ => handle_fallback(app, key),
    }
}

fn handle_url_input(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
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
        (_, KeyCode::Left) => app.url_cursor = app.url_cursor.saturating_sub(1),
        (_, KeyCode::Right) => app.url_cursor = (app.url_cursor + 1).min(app.url_input.len()),
        (_, KeyCode::Home) => app.url_cursor = 0,
        (_, KeyCode::End) => app.url_cursor = app.url_input.len(),
        (_, KeyCode::Tab) => {
            // Tab into the request pane's key-value editor
            if matches!(app.request_tab, RequestTab::Params | RequestTab::Headers) {
                app.focus = FocusedPane::KeyValueEditor;
                app.kv_selected = 0;
            } else {
                app.focus = FocusedPane::ResponseBody;
            }
        }
        (_, KeyCode::Esc) => app.focus = FocusedPane::ResponseBody,
        _ => {}
    }
    true
}

fn handle_response(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.response_scroll = app.response_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.response_scroll = app.response_scroll.saturating_sub(1);
        }
        KeyCode::Char('G') => app.response_scroll = u16::MAX,
        KeyCode::Char('g') => app.response_scroll = 0,
        KeyCode::Char('q') => return false,
        KeyCode::Char('m') => open_method_selector(app),
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
    }
    true
}

fn handle_sidebar(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => return false,
        KeyCode::Tab => app.focus = FocusedPane::UrlBar,
        _ => {}
    }
    true
}

fn handle_method_selector(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    let methods = crusty_core::request::HttpMethod::all();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.method_selector_index = app.method_selector_index.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.method_selector_index = (app.method_selector_index + 1).min(methods.len() - 1);
        }
        KeyCode::Enter => {
            app.method = methods[app.method_selector_index];
            app.method_selector_open = false;
        }
        KeyCode::Esc => app.method_selector_open = false,
        _ => {}
    }
    true
}

// --- Key-Value Editor ---

fn handle_kv_navigate(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    let list_len = app.current_kv_list().len();

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if list_len > 0 {
                app.kv_selected = (app.kv_selected + 1).min(list_len - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.kv_selected = app.kv_selected.saturating_sub(1);
        }
        KeyCode::Char('a') => {
            // Add new row
            app.current_kv_list_mut().push(KeyValue::new("", ""));
            app.kv_selected = app.current_kv_list().len() - 1;
            app.kv_mode = KvEditMode::EditKey;
            app.kv_edit_buf.clear();
            app.kv_edit_cursor = 0;
        }
        KeyCode::Char('d') => {
            // Delete selected row
            if list_len > 0 {
                let idx = app.kv_selected;
                app.current_kv_list_mut().remove(idx);
                let new_len = app.current_kv_list().len();
                if app.kv_selected >= new_len && app.kv_selected > 0 {
                    app.kv_selected -= 1;
                }
            }
        }
        KeyCode::Char(' ') => {
            // Toggle enabled/disabled
            let idx = app.kv_selected;
            if let Some(kv) = app.current_kv_list_mut().get_mut(idx) {
                kv.enabled = !kv.enabled;
            }
        }
        KeyCode::Enter | KeyCode::Char('e') => {
            // Edit key of selected
            if list_len > 0 {
                app.kv_mode = KvEditMode::EditKey;
                app.kv_edit_buf = app.current_kv_list()[app.kv_selected].key.clone();
                app.kv_edit_cursor = app.kv_edit_buf.len();
            }
        }
        KeyCode::Tab => app.focus = FocusedPane::ResponseBody,
        KeyCode::BackTab => app.focus = FocusedPane::UrlBar,
        KeyCode::Esc => app.focus = FocusedPane::ResponseBody,
        KeyCode::Char('q') => return false,
        KeyCode::Char('1') => app.request_tab = RequestTab::Params,
        KeyCode::Char('2') => app.request_tab = RequestTab::Headers,
        KeyCode::Char('3') => app.request_tab = RequestTab::Body,
        KeyCode::Char('4') => app.request_tab = RequestTab::Auth,
        _ => {}
    }
    true
}

fn handle_kv_edit(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(c) => {
            app.kv_edit_buf.insert(app.kv_edit_cursor, c);
            app.kv_edit_cursor += 1;
        }
        KeyCode::Backspace => {
            if app.kv_edit_cursor > 0 {
                app.kv_edit_cursor -= 1;
                app.kv_edit_buf.remove(app.kv_edit_cursor);
            }
        }
        KeyCode::Left => app.kv_edit_cursor = app.kv_edit_cursor.saturating_sub(1),
        KeyCode::Right => app.kv_edit_cursor = (app.kv_edit_cursor + 1).min(app.kv_edit_buf.len()),
        KeyCode::Tab => {
            // Save current field, move to next
            save_kv_edit(app);
            match app.kv_mode {
                KvEditMode::EditKey => {
                    app.kv_mode = KvEditMode::EditValue;
                    app.kv_edit_buf = app.current_kv_list()[app.kv_selected].value.clone();
                    app.kv_edit_cursor = app.kv_edit_buf.len();
                }
                KvEditMode::EditValue => {
                    app.kv_mode = KvEditMode::Navigate;
                }
                _ => {}
            }
        }
        KeyCode::Enter => {
            save_kv_edit(app);
            if app.kv_mode == KvEditMode::EditKey {
                app.kv_mode = KvEditMode::EditValue;
                app.kv_edit_buf = app.current_kv_list()[app.kv_selected].value.clone();
                app.kv_edit_cursor = app.kv_edit_buf.len();
            } else {
                app.kv_mode = KvEditMode::Navigate;
            }
        }
        KeyCode::Esc => {
            // Cancel edit
            app.kv_mode = KvEditMode::Navigate;
        }
        _ => {}
    }
    true
}

fn save_kv_edit(app: &mut App) {
    let idx = app.kv_selected;
    let mode = app.kv_mode;
    let buf = app.kv_edit_buf.clone();
    if let Some(kv) = app.current_kv_list_mut().get_mut(idx) {
        match mode {
            KvEditMode::EditKey => kv.key = buf,
            KvEditMode::EditValue => kv.value = buf,
            _ => {}
        }
    }
}

// --- Auth field editing ---

fn handle_auth_edit(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    let field = get_auth_field_mut(app);

    match key.code {
        KeyCode::Char(c) => field.push(c),
        KeyCode::Backspace => { field.pop(); }
        KeyCode::Tab | KeyCode::Enter => {
            app.auth_editing = false;
            app.auth_field_index = (app.auth_field_index + 1) % auth_field_count(app.auth_type);
        }
        KeyCode::Esc => app.auth_editing = false,
        _ => {}
    }
    true
}

fn get_auth_field_mut(app: &mut App) -> &mut String {
    match app.auth_type {
        AuthType::Bearer => &mut app.auth_bearer_token,
        AuthType::Basic => {
            if app.auth_field_index == 0 {
                &mut app.auth_basic_user
            } else {
                &mut app.auth_basic_pass
            }
        }
        AuthType::ApiKey => {
            if app.auth_field_index == 0 {
                &mut app.auth_apikey_key
            } else {
                &mut app.auth_apikey_value
            }
        }
        AuthType::None => &mut app.auth_bearer_token, // unused
    }
}

fn auth_field_count(auth_type: AuthType) -> usize {
    match auth_type {
        AuthType::None => 0,
        AuthType::Bearer => 1,
        AuthType::Basic => 2,
        AuthType::ApiKey => 2,
    }
}

// --- Helpers ---

fn open_method_selector(app: &mut App) {
    app.method_selector_open = true;
    let methods = crusty_core::request::HttpMethod::all();
    app.method_selector_index = methods.iter().position(|m| *m == app.method).unwrap_or(0);
}

fn cycle_environment(app: &mut App) {
    if app.environments.is_empty() {
        return;
    }
    app.active_env_index = Some(match app.active_env_index {
        None => 0,
        Some(i) => {
            if i + 1 >= app.environments.len() {
                return; // Wrap to None handled by separate logic if desired
            } else {
                i + 1
            }
        }
    });
}

fn handle_fallback(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => return false,
        KeyCode::Tab => app.focus = FocusedPane::UrlBar,
        _ => {}
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
                app.record_history();
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
