//! Application state for the Crusty TUI.

use crusty_auth::{ApiKeyLocation, AuthConfig, AuthProvider};
use crusty_core::collection::{Collection, CollectionItem};
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
    /// Send button.
    SendButton,
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
    /// Pre/post-request script.
    Script,
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
    /// Test results.
    Tests,
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

/// An item in the sidebar's flat list.
#[derive(Debug, Clone)]
pub enum SidebarItem {
    /// A collection header.
    Collection {
        /// Index in the collections list.
        index: usize,
        /// Collection name.
        name: String,
    },
    /// A request within a collection.
    Request {
        /// Parent collection index.
        collection_index: usize,
        /// Request ID.
        request_id: uuid::Uuid,
        /// HTTP method string.
        method: String,
        /// Request name.
        name: String,
    },
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
    /// Cursor position in body input.
    pub body_cursor: usize,
    /// Whether we're editing the body.
    pub body_editing: bool,

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
    /// Selected index in the sidebar (flat list of collections + requests).
    pub sidebar_selected: usize,
    /// Expanded collection indices (which collections are expanded).
    pub sidebar_expanded: Vec<bool>,
    /// Which section of sidebar is focused: 0 = collections, 1 = history.
    pub sidebar_section: u8,
    /// Selected index in the history list.
    pub history_selected: usize,
    /// Whether history search is active.
    pub history_search_active: bool,
    /// History search buffer.
    pub history_search_buf: String,
    /// Whether to show help overlay.
    pub show_help: bool,

    // --- Dialogs ---
    /// Whether the cURL import dialog is open.
    pub curl_import_open: bool,
    /// Text buffer for cURL import input.
    pub curl_import_buf: String,
    /// Cursor position in the cURL import buffer.
    pub curl_import_cursor: usize,
    /// Error from cURL import parsing.
    pub curl_import_error: Option<String>,

    /// Whether the code generation dialog is open.
    pub codegen_open: bool,
    /// Selected language index for code generation.
    pub codegen_lang_index: usize,

    /// Whether the save-to-collection dialog is open.
    pub save_dialog_open: bool,
    /// Buffer for request name when saving.
    pub save_name_buf: String,
    /// Cursor position in save name buffer.
    pub save_name_cursor: usize,
    /// Which collection to save to (index).
    pub save_collection_index: usize,
    /// Whether we're editing the name (true) or selecting collection (false).
    pub save_editing_name: bool,

    /// Whether the environment editor dialog is open.
    pub env_dialog_open: bool,
    /// Index of the environment being edited in the dialog.
    pub env_dialog_index: usize,
    /// Selected variable row in env editor.
    pub env_var_selected: usize,
    /// Editing mode in env editor: 0=navigate, 1=edit key, 2=edit value.
    pub env_var_edit_mode: u8,
    /// Edit buffer for env variable editing.
    pub env_var_edit_buf: String,
    /// Cursor in env variable edit buffer.
    pub env_var_edit_cursor: usize,
    /// Buffer for new environment name.
    pub env_name_buf: String,
    /// Whether we're editing the env name.
    pub env_editing_name: bool,

    // --- Scripting & Testing ---
    /// Post-request test script.
    pub script_input: String,
    /// Cursor position in script input.
    pub script_cursor: usize,
    /// Whether we're editing the script.
    pub script_editing: bool,
    /// Test run results from collection runner.
    pub test_results: Option<crusty_testing::runner::CollectionRunResult>,
    /// Whether a test run is in progress.
    pub test_running: bool,
    /// Which collection to run tests on (index).
    pub test_collection_index: usize,
    /// Selected test result entry for scrolling.
    pub test_result_scroll: usize,
    /// Script logs from the last run.
    pub script_logs: Vec<String>,
    /// Whether a send was requested (set by Enter in URL bar or Send button).
    pub send_requested: bool,
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
            body_cursor: 0,
            body_editing: false,
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
            sidebar_selected: 0,
            sidebar_expanded: Vec::new(),
            sidebar_section: 0,
            history_selected: 0,
            history_search_active: false,
            history_search_buf: String::new(),
            show_help: false,
            curl_import_open: false,
            curl_import_buf: String::new(),
            curl_import_cursor: 0,
            curl_import_error: None,
            codegen_open: false,
            codegen_lang_index: 0,
            save_dialog_open: false,
            save_name_buf: String::new(),
            save_name_cursor: 0,
            save_collection_index: 0,
            save_editing_name: true,
            env_dialog_open: false,
            env_dialog_index: 0,
            env_var_selected: 0,
            env_var_edit_mode: 0,
            env_var_edit_buf: String::new(),
            env_var_edit_cursor: 0,
            env_name_buf: String::new(),
            env_editing_name: false,
            script_input: String::new(),
            script_cursor: 0,
            script_editing: false,
            test_results: None,
            test_running: false,
            test_collection_index: 0,
            test_result_scroll: 0,
            script_logs: Vec::new(),
            send_requested: false,
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

    /// Load a history entry into the editor.
    pub fn load_history_entry(&mut self, index: usize) {
        // Clone the entry data to avoid borrow conflict
        let entry = {
            let filtered = self.filtered_history();
            match filtered.get(index) {
                Some(e) => (e.url.clone(), e.method.clone(), e.request_data.clone()),
                None => return,
            }
        };
        let (url, method, request_data) = entry;

        self.url_input = url;
        self.url_cursor = self.url_input.len();
        self.method = match method.as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head,
            _ => HttpMethod::Get,
        };
        // Try to restore headers/params/body from request_data JSON
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&request_data) {
            if let Some(headers) = data.get("headers") {
                if let Ok(h) = serde_json::from_value::<Vec<KeyValue>>(headers.clone()) {
                    self.headers = h;
                }
            }
            if let Some(params) = data.get("params") {
                if let Ok(p) = serde_json::from_value::<Vec<KeyValue>>(params.clone()) {
                    self.params = p;
                }
            }
            if let Some(body) = data.get("body").and_then(|v| v.as_str()) {
                self.body_input = body.to_string();
            }
        }
    }

    /// Get filtered history based on search.
    pub fn filtered_history(&self) -> Vec<&crusty_store::HistoryEntry> {
        if self.history_search_buf.is_empty() {
            self.history.iter().collect()
        } else {
            let search = self.history_search_buf.to_lowercase();
            self.history
                .iter()
                .filter(|h| {
                    h.url.to_lowercase().contains(&search)
                        || h.method.to_lowercase().contains(&search)
                })
                .collect()
        }
    }

    /// Load environments from the store.
    pub fn load_environments(&mut self) {
        if let Some(ref store) = self.store {
            if let Ok(env_list) = store.list_environments() {
                let mut envs = Vec::new();
                for (id, _name) in &env_list {
                    if let Ok(env) = store.get_environment(id) {
                        envs.push(env);
                    }
                }
                self.environments = envs;
            }
        }
    }

    /// Save the current environment to the store.
    pub fn save_current_environment(&self) {
        if let Some(ref store) = self.store {
            if let Some(env) = self.active_environment() {
                let _ = store.save_environment(env);
            }
        }
    }

    /// Load collections from the store.
    pub fn load_collections(&mut self) {
        if let Some(ref store) = self.store {
            if let Ok(col_list) = store.list_collections() {
                let mut cols = Vec::new();
                for (id, _name) in &col_list {
                    if let Ok(col) = store.get_collection(id) {
                        cols.push(col);
                    }
                }
                self.collections = cols;
                self.sidebar_expanded = vec![false; self.collections.len()];
            }
        }
    }

    /// Save the current request to a collection.
    pub fn save_request_to_collection(&mut self, name: &str, collection_index: usize) {
        let def = self.build_request_definition();
        let mut save_def = def;
        save_def.name = name.to_string();

        if self.collections.is_empty() {
            // Create a default collection
            let mut col = Collection::new("Default");
            col.add_request(save_def);
            if let Some(ref store) = self.store {
                let _ = store.save_collection(&col);
            }
            self.collections.push(col);
            self.sidebar_expanded.push(true);
        } else if let Some(col) = self.collections.get_mut(collection_index) {
            col.add_request(save_def);
            if let Some(ref store) = self.store {
                let _ = store.save_collection(col);
            }
        }
    }

    /// Get a flat list of sidebar items for rendering: (indent_level, label, is_collection, collection_idx, option request).
    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items = Vec::new();
        for (ci, col) in self.collections.iter().enumerate() {
            items.push(SidebarItem::Collection { index: ci, name: col.name.clone() });
            if ci < self.sidebar_expanded.len() && self.sidebar_expanded[ci] {
                for item in &col.items {
                    if let CollectionItem::Request(req) = item {
                        items.push(SidebarItem::Request {
                            collection_index: ci,
                            request_id: req.id,
                            method: req.method.as_str().to_string(),
                            name: req.name.clone(),
                        });
                    }
                }
            }
        }
        items
    }

    /// Load a request from a collection into the editor.
    pub fn load_request(&mut self, collection_index: usize, request_id: uuid::Uuid) {
        if let Some(col) = self.collections.get(collection_index) {
            if let Some(req) = col.find_request(&request_id) {
                self.url_input = req.url.clone();
                self.url_cursor = self.url_input.len();
                self.method = req.method;
                self.headers = req.headers.clone();
                self.params = req.params.clone();
                match &req.body {
                    RequestBody::Json(json) => self.body_input = json.clone(),
                    RequestBody::Raw { content, .. } => self.body_input = content.clone(),
                    _ => self.body_input.clear(),
                }
            }
        }
    }

    /// Build a RequestDefinition from current state (for export/codegen).
    pub fn build_request_definition(&self) -> RequestDefinition {
        let mut def = RequestDefinition::new("TUI Request", &self.url_input);
        def.method = self.method;
        def.headers = self.headers.clone();
        def.params = self.params.clone();
        if !self.body_input.trim().is_empty() {
            def.body = RequestBody::Json(self.body_input.clone());
        }
        def
    }

    /// Apply a parsed cURL import to the current app state.
    pub fn apply_curl_import(&mut self, def: &RequestDefinition) {
        self.url_input = def.url.clone();
        self.url_cursor = self.url_input.len();
        self.method = def.method;
        self.headers = def.headers.clone();
        self.params = def.params.clone();
        match &def.body {
            RequestBody::Json(json) => self.body_input = json.clone(),
            RequestBody::Raw { content, .. } => self.body_input = content.clone(),
            _ => {}
        }
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
