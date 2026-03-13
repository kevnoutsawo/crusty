//! Crusty TUI — Terminal UI frontend for the Crusty HTTP client.

mod app;
mod ui;

use app::{App, AuthType, FocusedPane, KvEditMode, RequestTab, ResponseTab, SidebarItem};
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
    app.load_collections();
    app.load_environments();

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

    // cURL import dialog
    if app.curl_import_open {
        return handle_curl_import(app, key);
    }

    // Code generation dialog
    if app.codegen_open {
        return handle_codegen(app, key);
    }

    // Save dialog
    if app.save_dialog_open {
        return handle_save_dialog(app, key);
    }

    // Environment dialog
    if app.env_dialog_open {
        return handle_env_dialog(app, key);
    }

    // If editing a key-value field, handle that first
    if app.kv_mode != KvEditMode::Navigate {
        return handle_kv_edit(app, key);
    }

    // If editing an auth field, handle that
    if app.auth_editing {
        return handle_auth_edit(app, key);
    }

    // If editing body, handle that
    if app.body_editing {
        return handle_body_edit(app, key);
    }

    // Global: Ctrl+I opens cURL import
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('i') {
        app.curl_import_open = true;
        app.curl_import_buf.clear();
        app.curl_import_cursor = 0;
        app.curl_import_error = None;
        return true;
    }

    // Global: Ctrl+G opens code generation
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('g') {
        app.codegen_open = true;
        app.codegen_lang_index = 0;
        return true;
    }

    // Global: Ctrl+N opens environment editor
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('n') {
        app.env_dialog_open = true;
        if app.environments.is_empty() {
            // Create a new environment
            let env = crusty_core::environment::Environment::new("New Environment");
            app.environments.push(env);
        }
        app.env_dialog_index = app.active_env_index.unwrap_or(0);
        app.env_var_selected = 0;
        app.env_var_edit_mode = 0;
        app.env_editing_name = false;
        app.env_name_buf = app.environments[app.env_dialog_index].name.clone();
        return true;
    }

    // Global: Ctrl+S saves request to collection
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.save_dialog_open = true;
        app.save_name_buf = if app.url_input.is_empty() {
            "New Request".to_string()
        } else {
            app.url_input.clone()
        };
        app.save_name_cursor = app.save_name_buf.len();
        app.save_collection_index = 0;
        app.save_editing_name = true;
        return true;
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
            // Tab into the request pane's key-value editor or body editor
            if matches!(app.request_tab, RequestTab::Params | RequestTab::Headers) {
                app.focus = FocusedPane::KeyValueEditor;
                app.kv_selected = 0;
            } else if app.request_tab == RequestTab::Body {
                app.body_editing = true;
                app.body_cursor = app.body_input.len();
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
        KeyCode::Char('b') => {
            // Quick-enter body editing from response pane
            if app.request_tab == RequestTab::Body {
                app.body_editing = true;
                app.body_cursor = app.body_input.len();
            }
        }
        _ => {}
    }
    true
}

fn handle_sidebar(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    let items = app.sidebar_items();
    let item_count = items.len();

    match key.code {
        KeyCode::Char('q') => return false,
        KeyCode::Tab => app.focus = FocusedPane::UrlBar,
        KeyCode::Char('j') | KeyCode::Down => {
            if item_count > 0 {
                app.sidebar_selected = (app.sidebar_selected + 1).min(item_count - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.sidebar_selected = app.sidebar_selected.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            if let Some(item) = items.get(app.sidebar_selected) {
                match item {
                    SidebarItem::Collection { index, .. } => {
                        let idx = *index;
                        if idx < app.sidebar_expanded.len() {
                            app.sidebar_expanded[idx] = !app.sidebar_expanded[idx];
                        }
                    }
                    SidebarItem::Request { collection_index, request_id, .. } => {
                        let ci = *collection_index;
                        let rid = *request_id;
                        app.load_request(ci, rid);
                        app.focus = FocusedPane::UrlBar;
                    }
                }
            }
        }
        KeyCode::Char('h') => {
            // Collapse current collection
            if let Some(item) = items.get(app.sidebar_selected) {
                match item {
                    SidebarItem::Collection { index, .. } => {
                        let idx = *index;
                        if idx < app.sidebar_expanded.len() {
                            app.sidebar_expanded[idx] = false;
                        }
                    }
                    SidebarItem::Request { collection_index, .. } => {
                        let ci = *collection_index;
                        if ci < app.sidebar_expanded.len() {
                            app.sidebar_expanded[ci] = false;
                        }
                    }
                }
            }
        }
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

// --- Body Editor ---

fn handle_body_edit(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(c) => {
            app.body_input.insert(app.body_cursor, c);
            app.body_cursor += 1;
        }
        KeyCode::Enter => {
            app.body_input.insert(app.body_cursor, '\n');
            app.body_cursor += 1;
        }
        KeyCode::Backspace => {
            if app.body_cursor > 0 {
                app.body_cursor -= 1;
                app.body_input.remove(app.body_cursor);
            }
        }
        KeyCode::Delete => {
            if app.body_cursor < app.body_input.len() {
                app.body_input.remove(app.body_cursor);
            }
        }
        KeyCode::Left => app.body_cursor = app.body_cursor.saturating_sub(1),
        KeyCode::Right => app.body_cursor = (app.body_cursor + 1).min(app.body_input.len()),
        KeyCode::Home => {
            // Move to start of current line
            let before = &app.body_input[..app.body_cursor];
            app.body_cursor = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        }
        KeyCode::End => {
            // Move to end of current line
            let after = &app.body_input[app.body_cursor..];
            app.body_cursor += after.find('\n').unwrap_or(after.len());
        }
        KeyCode::Esc => {
            app.body_editing = false;
        }
        _ => {}
    }
    true
}

// --- cURL Import Dialog ---

fn handle_curl_import(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(c) => {
            app.curl_import_buf.insert(app.curl_import_cursor, c);
            app.curl_import_cursor += 1;
            app.curl_import_error = None;
        }
        KeyCode::Backspace => {
            if app.curl_import_cursor > 0 {
                app.curl_import_cursor -= 1;
                app.curl_import_buf.remove(app.curl_import_cursor);
                app.curl_import_error = None;
            }
        }
        KeyCode::Left => app.curl_import_cursor = app.curl_import_cursor.saturating_sub(1),
        KeyCode::Right => {
            app.curl_import_cursor = (app.curl_import_cursor + 1).min(app.curl_import_buf.len())
        }
        KeyCode::Home => app.curl_import_cursor = 0,
        KeyCode::End => app.curl_import_cursor = app.curl_import_buf.len(),
        KeyCode::Enter => {
            match crusty_export::curl::import(&app.curl_import_buf) {
                Ok(def) => {
                    app.apply_curl_import(&def);
                    app.curl_import_open = false;
                }
                Err(e) => {
                    app.curl_import_error = Some(e.to_string());
                }
            }
        }
        KeyCode::Esc => app.curl_import_open = false,
        _ => {}
    }
    true
}

// --- Code Generation Dialog ---

fn handle_codegen(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    let langs = crusty_export::codegen::Language::all();
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.codegen_lang_index = (app.codegen_lang_index + 1).min(langs.len() - 1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.codegen_lang_index = app.codegen_lang_index.saturating_sub(1);
        }
        KeyCode::Esc | KeyCode::Char('q') => app.codegen_open = false,
        _ => {}
    }
    true
}

// --- Save Dialog ---

fn handle_save_dialog(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    if app.save_editing_name {
        match key.code {
            KeyCode::Char(c) => {
                app.save_name_buf.insert(app.save_name_cursor, c);
                app.save_name_cursor += 1;
            }
            KeyCode::Backspace => {
                if app.save_name_cursor > 0 {
                    app.save_name_cursor -= 1;
                    app.save_name_buf.remove(app.save_name_cursor);
                }
            }
            KeyCode::Left => app.save_name_cursor = app.save_name_cursor.saturating_sub(1),
            KeyCode::Right => {
                app.save_name_cursor =
                    (app.save_name_cursor + 1).min(app.save_name_buf.len())
            }
            KeyCode::Tab => {
                // Switch to collection selection if there are collections
                if !app.collections.is_empty() {
                    app.save_editing_name = false;
                }
            }
            KeyCode::Enter => {
                // Save
                let name = app.save_name_buf.clone();
                let idx = app.save_collection_index;
                app.save_request_to_collection(&name, idx);
                app.save_dialog_open = false;
            }
            KeyCode::Esc => app.save_dialog_open = false,
            _ => {}
        }
    } else {
        // Selecting collection
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !app.collections.is_empty() {
                    app.save_collection_index =
                        (app.save_collection_index + 1).min(app.collections.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.save_collection_index = app.save_collection_index.saturating_sub(1);
            }
            KeyCode::Tab | KeyCode::Enter => {
                app.save_editing_name = true;
            }
            KeyCode::Esc => app.save_dialog_open = false,
            _ => {}
        }
    }
    true
}

// --- Environment Dialog ---

fn handle_env_dialog(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    // Editing env name
    if app.env_editing_name {
        match key.code {
            KeyCode::Char(c) => app.env_name_buf.push(c),
            KeyCode::Backspace => { app.env_name_buf.pop(); }
            KeyCode::Enter | KeyCode::Esc => {
                // Apply name
                if let Some(env) = app.environments.get_mut(app.env_dialog_index) {
                    env.name = app.env_name_buf.clone();
                }
                app.env_editing_name = false;
            }
            _ => {}
        }
        return true;
    }

    // Editing a variable key/value
    if app.env_var_edit_mode > 0 {
        match key.code {
            KeyCode::Char(c) => {
                app.env_var_edit_buf.insert(app.env_var_edit_cursor, c);
                app.env_var_edit_cursor += 1;
            }
            KeyCode::Backspace => {
                if app.env_var_edit_cursor > 0 {
                    app.env_var_edit_cursor -= 1;
                    app.env_var_edit_buf.remove(app.env_var_edit_cursor);
                }
            }
            KeyCode::Left => app.env_var_edit_cursor = app.env_var_edit_cursor.saturating_sub(1),
            KeyCode::Right => {
                app.env_var_edit_cursor =
                    (app.env_var_edit_cursor + 1).min(app.env_var_edit_buf.len())
            }
            KeyCode::Tab | KeyCode::Enter => {
                // Save field and move to next
                save_env_var_field(app);
                if app.env_var_edit_mode == 1 {
                    // Move to value
                    app.env_var_edit_mode = 2;
                    if let Some(env) = app.environments.get(app.env_dialog_index) {
                        if let Some(var) = env.variables.get(app.env_var_selected) {
                            app.env_var_edit_buf = var.value.reveal().to_string();
                            app.env_var_edit_cursor = app.env_var_edit_buf.len();
                        }
                    }
                } else {
                    app.env_var_edit_mode = 0;
                }
            }
            KeyCode::Esc => {
                app.env_var_edit_mode = 0;
            }
            _ => {}
        }
        return true;
    }

    // Navigation mode
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(env) = app.environments.get(app.env_dialog_index) {
                if !env.variables.is_empty() {
                    app.env_var_selected =
                        (app.env_var_selected + 1).min(env.variables.len() - 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.env_var_selected = app.env_var_selected.saturating_sub(1);
        }
        KeyCode::Char('a') => {
            // Add variable
            if let Some(env) = app.environments.get_mut(app.env_dialog_index) {
                env.add_variable("", "");
                app.env_var_selected = env.variables.len() - 1;
                app.env_var_edit_mode = 1;
                app.env_var_edit_buf.clear();
                app.env_var_edit_cursor = 0;
            }
        }
        KeyCode::Char('d') => {
            // Delete variable
            if let Some(env) = app.environments.get_mut(app.env_dialog_index) {
                if !env.variables.is_empty() {
                    let idx = app.env_var_selected;
                    env.variables.remove(idx);
                    if app.env_var_selected > 0 && app.env_var_selected >= env.variables.len() {
                        app.env_var_selected -= 1;
                    }
                }
            }
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            // Edit selected variable key
            if let Some(env) = app.environments.get(app.env_dialog_index) {
                if let Some(var) = env.variables.get(app.env_var_selected) {
                    app.env_var_edit_mode = 1;
                    app.env_var_edit_buf = var.key.clone();
                    app.env_var_edit_cursor = app.env_var_edit_buf.len();
                }
            }
        }
        KeyCode::Char(' ') => {
            // Toggle variable enabled
            let idx = app.env_var_selected;
            if let Some(env) = app.environments.get_mut(app.env_dialog_index) {
                if let Some(var) = env.variables.get_mut(idx) {
                    var.enabled = !var.enabled;
                }
            }
        }
        KeyCode::Char('n') => {
            // Edit env name
            app.env_editing_name = true;
        }
        KeyCode::Char('N') => {
            // Create new environment
            let env = crusty_core::environment::Environment::new("New Environment");
            app.environments.push(env);
            app.env_dialog_index = app.environments.len() - 1;
            app.env_var_selected = 0;
            app.env_name_buf = "New Environment".to_string();
            app.env_editing_name = true;
        }
        KeyCode::Char('l') | KeyCode::Right => {
            // Next environment
            if !app.environments.is_empty() {
                app.env_dialog_index =
                    (app.env_dialog_index + 1) % app.environments.len();
                app.env_var_selected = 0;
                app.env_name_buf = app.environments[app.env_dialog_index].name.clone();
            }
        }
        KeyCode::Char('h') | KeyCode::Left => {
            // Previous environment
            if !app.environments.is_empty() {
                app.env_dialog_index = if app.env_dialog_index == 0 {
                    app.environments.len() - 1
                } else {
                    app.env_dialog_index - 1
                };
                app.env_var_selected = 0;
                app.env_name_buf = app.environments[app.env_dialog_index].name.clone();
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            // Save and close
            if let Some(ref store) = app.store {
                for env in &app.environments {
                    let _ = store.save_environment(env);
                }
            }
            // Set active env to the one we were editing
            app.active_env_index = Some(app.env_dialog_index);
            app.env_dialog_open = false;
        }
        _ => {}
    }
    true
}

fn save_env_var_field(app: &mut App) {
    let idx = app.env_var_selected;
    let buf = app.env_var_edit_buf.clone();
    let mode = app.env_var_edit_mode;
    if let Some(env) = app.environments.get_mut(app.env_dialog_index) {
        if let Some(var) = env.variables.get_mut(idx) {
            match mode {
                1 => var.key = buf,
                2 => var.value = crusty_core::environment::VariableValue::Plain(buf),
                _ => {}
            }
        }
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
