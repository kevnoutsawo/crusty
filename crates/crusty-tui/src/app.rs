//! Application state for the Crusty TUI.

use crusty_auth::{ApiKeyLocation, AuthConfig, AuthProvider};
use crusty_core::collection::Collection;
use crusty_core::environment::Environment;
use crusty_core::request::{HttpMethod, KeyValue, RequestBody, RequestDefinition};
use crusty_core::response::HttpResponse;
use crusty_http::HttpClient;
use crusty_store::Store;
use std::collections::HashMap;

/// Which pane is currently focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    /// Collection sidebar.
    Sidebar,
    /// URL input bar.
    UrlBar,
    /// Method selector.
    MethodSelector,
    /// Request body editor.
    RequestBody,
    /// Response body viewer.
    ResponseBody,
    /// Key-value editor (headers/params).
    KeyValueEditor,
}

/// Which tab is active in the request section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestTab {
    /// Query parameters.
    Params,
    /// Request headers.
    Headers,
    /// Request body.
    Body,
    /// Authentication.
    Auth,
}

/// Which tab is active in the response section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    /// Response body.
    Body,
    /// Response headers.
    Headers,
    /// Timing information.
    Timing,
}

/// Input mode for key-value editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvEditMode {
    /// Navigating the list.
    Navigate,
    /// Editing the key of the selected row.
    EditKey,
    /// Editing the value of the selected row.
    EditValue,
}

/// Auth type selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    /// No auth.
    None,
    /// Bearer token.
    Bearer,
    /// Basic auth.
    Basic,
    /// API Key.
    ApiKey,
}

impl AuthType {
    /// All available auth types.
    pub fn all() -> &'static [AuthType] {
        &[Self::None, Self::Bearer, Self::Basic, Self::ApiKey]
    }

    /// Display name.
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Bearer => "Bearer Token",
            Self::Basic => "Basic Auth",
            Self::ApiKey => "API Key",
        }
    }
}

/// The main application state.
pub struct App {
    /// The currently focused pane.
    pub focus: FocusedPane,
    /// Current URL input.
    pub url_input: String,
    /// Cursor position within the URL input.
    pub url_cursor: usize,
    /// Currently selected HTTP method.
    pub method: HttpMethod,
    /// Whether the method selector dropdown is open.
    pub method_selector_open: bool,
    /// Index in the method list for the dropdown.
    pub method_selector_index: usize,
    /// Active request tab.
    pub request_tab: RequestTab,
    /// Active response tab.
    pub response_tab: ResponseTab,

    // --- Key-Value editors ---
    /// Request headers.
    pub headers: Vec<KeyValue>,
    /// Request query params.
    pub params: Vec<KeyValue>,
    /// Selected row in current key-value editor.
    pub kv_selected: usize,
    /// Current edit mode for key-value editor.
    pub kv_mode: KvEditMode,
    /// Edit buffer for key-value input.
    pub kv_edit_buf: String,
    /// Cursor position in edit buffer.
    pub kv_edit_cursor: usize,

    // --- Body ---
    /// Request body text (for JSON/raw).
    pub body_input: String,

    // --- Auth ---
    /// Selected auth type.
    pub auth_type: AuthType,
    /// Bearer token input.
    pub auth_bearer_token: String,
    /// Basic auth username.
    pub auth_basic_user: String,
    /// Basic auth password.
    pub auth_basic_pass: String,
    /// API Key name.
    pub auth_apikey_key: String,
    /// API Key value.
    pub auth_apikey_value: String,
    /// API Key location (0 = header, 1 = query).
    pub auth_apikey_in_header: bool,
    /// Which auth field is focused (0-based index within the auth form).
    pub auth_field_index: usize,
    /// Whether we're editing an auth field.
    pub auth_editing: bool,

    // --- Environments ---
    /// Available environments.
    pub environments: Vec<Environment>,
    /// Index of the active environment (None = no env selected).
    pub active_env_index: Option<usize>,

    // --- History ---
    /// Recent request history for display.
    pub history: Vec<crusty_store::HistoryEntry>,

    // --- Response ---
    /// The last HTTP response received.
    pub response: Option<HttpResponse>,
    /// Whether a request is currently in flight.
    pub loading: bool,
    /// Error message to display.
    pub error: Option<String>,
    /// Response body scroll offset.
    pub response_scroll: u16,

    // --- Infrastructure ---
    /// HTTP client.
    pub http_client: HttpClient,
    /// Persistent store.
    pub store: Option<Store>,
    /// Collections for sidebar.
    pub collections: Vec<Collection>,
    /// Whether the sidebar is visible.
    pub sidebar_visible: bool,
    /// Whether to show help overlay.
    pub show_help: bool,
}

impl App {
    /// Create a new App with default state.
    pub fn new() -> Self {
        // Try to open store in a standard location
        let store = dirs_data_path()
            .and_then(|p| Store::open(&p).ok());

        Self {
            focus: FocusedPane::UrlBar,
            url_input: String::new(),
            url_cursor: 0,
            method: HttpMethod::Get,
            method_selector_open: false,
            method_selector_index: 0,
            request_tab: RequestTab::Params,
            response_tab: ResponseTab::Body,
            headers: Vec::new(),
            params: Vec::new(),
            kv_selected: 0,
            kv_mode: KvEditMode::Navigate,
            kv_edit_buf: String::new(),
            kv_edit_cursor: 0,
            body_input: String::new(),
            auth_type: AuthType::None,
            auth_bearer_token: String::new(),
            auth_basic_user: String::new(),
            auth_basic_pass: String::new(),
            auth_apikey_key: String::new(),
            auth_apikey_value: String::new(),
            auth_apikey_in_header: true,
            auth_field_index: 0,
            auth_editing: false,
            environments: Vec::new(),
            active_env_index: None,
            history: Vec::new(),
            response: None,
            loading: false,
            error: None,
            response_scroll: 0,
            http_client: HttpClient::default(),
            store,
            collections: Vec::new(),
            sidebar_visible: true,
            show_help: false,
        }
    }

    /// Load history from the store.
    pub fn load_history(&mut self) {
        if let Some(ref store) = self.store {
            if let Ok(entries) = store.list_history(50) {
                self.history = entries;
            }
        }
    }

    /// Record a request/response in history.
    pub fn record_history(&mut self) {
        let Some(ref store) = self.store else { return };
        let status = self.response.as_ref().map(|r| r.status);
        let duration = self.response.as_ref().map(|r| r.timing.total.as_millis() as u64);
        let response_data = self.response.as_ref().and_then(|r| {
            r.body_text().map(|s| s.to_string())
        });

        let entry = crusty_store::HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            method: self.method.as_str().to_string(),
            url: self.url_input.clone(),
            status,
            duration_ms: duration,
            request_data: serde_json::json!({
                "headers": self.headers,
                "params": self.params,
                "body": self.body_input,
            }).to_string(),
            response_data,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let _ = store.add_history(&entry);
        self.load_history();
    }

    /// Build the auth config from current state.
    pub fn build_auth_config(&self) -> AuthConfig {
        match self.auth_type {
            AuthType::None => AuthConfig::None,
            AuthType::Bearer => AuthConfig::Bearer {
                token: self.auth_bearer_token.clone(),
            },
            AuthType::Basic => AuthConfig::Basic {
                username: self.auth_basic_user.clone(),
                password: self.auth_basic_pass.clone(),
            },
            AuthType::ApiKey => AuthConfig::ApiKey {
                key: self.auth_apikey_key.clone(),
                value: self.auth_apikey_value.clone(),
                location: if self.auth_apikey_in_header {
                    ApiKeyLocation::Header
                } else {
                    ApiKeyLocation::Query
                },
            },
        }
    }

    /// Get the active environment, if any.
    pub fn active_environment(&self) -> Option<&Environment> {
        self.active_env_index
            .and_then(|i| self.environments.get(i))
    }

    /// Build a resolved request from the current app state.
    pub fn build_request(
        &self,
    ) -> Result<crusty_core::request::ResolvedRequest, crusty_core::error::CoreError> {
        // Build request definition
        let mut def = RequestDefinition::new("TUI Request", &self.url_input);
        def.method = self.method;
        def.headers = self.headers.clone();
        def.params = self.params.clone();

        if !self.body_input.trim().is_empty() {
            def.body = RequestBody::Json(self.body_input.clone());
        }

        // Resolve auth
        let auth_config = self.build_auth_config();
        let mut auth_headers = HashMap::new();
        let mut auth_params = Vec::new();
        let _ = auth_config.apply(&mut auth_headers, &mut auth_params);

        // Build environment list
        let envs: Vec<&Environment> = self
            .active_environment()
            .into_iter()
            .collect();

        // Use the orchestrator
        let mut resolved = crusty_core::orchestrator::resolve_request(
            &def,
            &envs,
            &auth_headers,
        )?;

        // Apply auth query params
        for (key, value) in auth_params {
            resolved.url.query_pairs_mut().append_pair(&key, &value);
        }

        Ok(resolved)
    }

    /// Get the current key-value list being edited (params or headers).
    pub fn current_kv_list(&self) -> &Vec<KeyValue> {
        match self.request_tab {
            RequestTab::Params => &self.params,
            RequestTab::Headers => &self.headers,
            _ => &self.params,
        }
    }

    /// Get mutable reference to the current key-value list.
    pub fn current_kv_list_mut(&mut self) -> &mut Vec<KeyValue> {
        match self.request_tab {
            RequestTab::Params => &mut self.params,
            RequestTab::Headers => &mut self.headers,
            _ => &mut self.params,
        }
    }
}

fn dirs_data_path() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let dir = format!("{home}/.local/share/crusty");
    std::fs::create_dir_all(&dir).ok()?;
    Some(format!("{dir}/crusty.db"))
}
