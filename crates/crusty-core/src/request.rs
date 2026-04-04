//! Request models for building and representing HTTP requests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// HTTP methods supported by Crusty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// GET method.
    #[default]
    Get,
    /// POST method.
    Post,
    /// PUT method.
    Put,
    /// PATCH method.
    Patch,
    /// DELETE method.
    Delete,
    /// HEAD method.
    Head,
    /// OPTIONS method.
    Options,
    /// TRACE method.
    Trace,
    /// CONNECT method.
    Connect,
}

impl HttpMethod {
    /// Returns the method as a string slice.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Connect => "CONNECT",
        }
    }

    /// All available methods, in display order.
    pub fn all() -> &'static [HttpMethod] {
        &[
            Self::Get,
            Self::Post,
            Self::Put,
            Self::Patch,
            Self::Delete,
            Self::Head,
            Self::Options,
            Self::Trace,
            Self::Connect,
        ]
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A key-value pair that can be enabled or disabled without deletion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    /// The key.
    pub key: String,
    /// The value (may contain `{{variable}}` references).
    pub value: String,
    /// Whether this entry is active.
    pub enabled: bool,
}

impl KeyValue {
    /// Create a new enabled key-value pair.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }
}

/// Request body types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[derive(Default)]
pub enum RequestBody {
    /// No body.
    #[default]
    None,
    /// JSON body (raw string, validated as JSON).
    Json(String),
    /// Raw text body with a content type hint.
    Raw {
        /// The raw text content.
        content: String,
        /// Content type (e.g., "text/plain", "application/xml").
        content_type: String,
    },
    /// Form URL-encoded body.
    FormUrlEncoded(Vec<KeyValue>),
    /// Multipart form data.
    FormData(Vec<FormDataEntry>),
    /// Binary body (file path).
    Binary(String),
}

/// A single entry in a multipart form data body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormDataEntry {
    /// Field name.
    pub key: String,
    /// Field value.
    pub value: FormDataValue,
    /// Whether this entry is active.
    pub enabled: bool,
}

/// The value of a form data field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FormDataValue {
    /// Text value.
    Text(String),
    /// File path.
    File(String),
}

/// A complete HTTP request definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestDefinition {
    /// Unique identifier.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// HTTP method.
    pub method: HttpMethod,
    /// URL (may contain `{{variable}}` references).
    pub url: String,
    /// Query parameters.
    pub params: Vec<KeyValue>,
    /// Request headers.
    pub headers: Vec<KeyValue>,
    /// Request body.
    pub body: RequestBody,
    /// Request-level settings.
    pub settings: RequestSettings,
}

impl RequestDefinition {
    /// Create a new request with the given name and URL.
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            method: HttpMethod::default(),
            url: url.into(),
            params: Vec::new(),
            headers: Vec::new(),
            body: RequestBody::default(),
            settings: RequestSettings::default(),
        }
    }
}

/// Per-request settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSettings {
    /// Whether to follow HTTP redirects.
    pub follow_redirects: bool,
    /// Maximum number of redirects to follow.
    pub max_redirects: u32,
    /// Connection timeout in milliseconds.
    pub connect_timeout_ms: u64,
    /// Read timeout in milliseconds.
    pub read_timeout_ms: u64,
    /// Whether to verify TLS certificates.
    pub verify_ssl: bool,
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            follow_redirects: true,
            max_redirects: 10,
            connect_timeout_ms: 30_000,
            read_timeout_ms: 30_000,
            verify_ssl: true,
        }
    }
}

/// A fully resolved request ready to be sent by the HTTP engine.
/// All variables have been interpolated, all params merged into the URL.
#[derive(Debug, Clone)]
pub struct ResolvedRequest {
    /// HTTP method.
    pub method: HttpMethod,
    /// Fully resolved URL (with query params appended).
    pub url: url::Url,
    /// Resolved headers.
    pub headers: HashMap<String, String>,
    /// Resolved body.
    pub body: ResolvedBody,
    /// Settings.
    pub settings: RequestSettings,
}

/// A body ready to be sent.
#[derive(Debug, Clone)]
pub enum ResolvedBody {
    /// No body.
    None,
    /// Bytes with a content type.
    Bytes {
        /// The body content.
        data: Vec<u8>,
        /// The content type.
        content_type: String,
    },
}
