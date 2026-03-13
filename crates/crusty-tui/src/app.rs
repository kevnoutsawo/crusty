//! Application state for the Crusty TUI.

use crusty_core::collection::Collection;
use crusty_core::request::{HttpMethod, KeyValue};
use crusty_core::response::HttpResponse;
use crusty_http::HttpClient;
use std::collections::HashMap;

/// Which pane is currently focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Sidebar,
    UrlBar,
    MethodSelector,
    RequestBody,
    ResponseBody,
}

/// Which tab is active in the request section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestTab {
    Params,
    Headers,
    Body,
    Auth,
}

/// Which tab is active in the response section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
    Timing,
}

/// The main application state.
pub struct App {
    /// Whether the app is running.
    pub running: bool,
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
    /// Request headers.
    pub headers: Vec<KeyValue>,
    /// Request query params.
    pub params: Vec<KeyValue>,
    /// Request body text (for JSON/raw).
    pub body_input: String,
    /// The last HTTP response received.
    pub response: Option<HttpResponse>,
    /// Whether a request is currently in flight.
    pub loading: bool,
    /// Error message to display.
    pub error: Option<String>,
    /// HTTP client.
    pub http_client: HttpClient,
    /// Collections for sidebar.
    pub collections: Vec<Collection>,
    /// Sidebar scroll offset.
    pub sidebar_scroll: usize,
    /// Whether the sidebar is visible.
    pub sidebar_visible: bool,
    /// Response body scroll offset.
    pub response_scroll: u16,
    /// Whether to show help overlay.
    pub show_help: bool,
}

impl App {
    /// Create a new App with default state.
    pub fn new() -> Self {
        Self {
            running: true,
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
            body_input: String::new(),
            response: None,
            loading: false,
            error: None,
            http_client: HttpClient::default(),
            collections: Vec::new(),
            sidebar_scroll: 0,
            sidebar_visible: true,
            response_scroll: 0,
            show_help: false,
        }
    }

    /// Build a resolved request from the current app state.
    pub fn build_request(
        &self,
    ) -> Result<crusty_core::request::ResolvedRequest, crusty_core::error::CoreError> {
        let mut url_str = self.url_input.trim().to_string();

        // Add https:// if no scheme is present
        if !url_str.is_empty() && !url_str.contains("://") {
            url_str = format!("https://{url_str}");
        }

        let mut url = url::Url::parse(&url_str).map_err(|e| {
            crusty_core::error::CoreError::InvalidUrl {
                url: url_str.clone(),
                reason: e.to_string(),
            }
        })?;

        // Append enabled query params
        {
            let mut query_pairs = url.query_pairs_mut();
            for param in &self.params {
                if param.enabled {
                    query_pairs.append_pair(&param.key, &param.value);
                }
            }
        }

        // Build headers map
        let mut headers = HashMap::new();
        for h in &self.headers {
            if h.enabled {
                headers.insert(h.key.clone(), h.value.clone());
            }
        }

        // Build body
        let body = if self.body_input.trim().is_empty() {
            crusty_core::request::ResolvedBody::None
        } else {
            crusty_core::request::ResolvedBody::Bytes {
                data: self.body_input.as_bytes().to_vec(),
                content_type: "application/json".to_string(),
            }
        };

        Ok(crusty_core::request::ResolvedRequest {
            method: self.method,
            url,
            headers,
            body,
            settings: crusty_core::request::RequestSettings::default(),
        })
    }
}
