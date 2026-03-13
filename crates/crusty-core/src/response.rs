//! Response models for representing HTTP responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// An HTTP response with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code (e.g., 200, 404, 500).
    pub status: u16,
    /// Status text (e.g., "OK", "Not Found").
    pub status_text: String,
    /// Response headers.
    pub headers: HashMap<String, String>,
    /// Response body as bytes.
    #[serde(with = "serde_bytes_vec")]
    pub body: Vec<u8>,
    /// Response timing information.
    pub timing: ResponseTiming,
    /// Size information.
    pub size: ResponseSize,
}

impl HttpResponse {
    /// Try to interpret the response body as UTF-8 text.
    pub fn body_text(&self) -> Option<&str> {
        std::str::from_utf8(&self.body).ok()
    }

    /// Try to parse the response body as JSON.
    pub fn body_json(&self) -> Option<serde_json::Value> {
        serde_json::from_slice(&self.body).ok()
    }

    /// Get a formatted/pretty-printed JSON body, if the body is valid JSON.
    pub fn body_json_pretty(&self) -> Option<String> {
        let value: serde_json::Value = serde_json::from_slice(&self.body).ok()?;
        serde_json::to_string_pretty(&value).ok()
    }

    /// Returns the content type from headers, if present.
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.as_str())
    }

    /// Returns true if the status code indicates success (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Returns the status category color hint for UI rendering.
    pub fn status_category(&self) -> StatusCategory {
        match self.status {
            100..=199 => StatusCategory::Informational,
            200..=299 => StatusCategory::Success,
            300..=399 => StatusCategory::Redirection,
            400..=499 => StatusCategory::ClientError,
            500..=599 => StatusCategory::ServerError,
            _ => StatusCategory::Unknown,
        }
    }
}

/// HTTP status code categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCategory {
    /// 1xx — Informational.
    Informational,
    /// 2xx — Success.
    Success,
    /// 3xx — Redirection.
    Redirection,
    /// 4xx — Client error.
    ClientError,
    /// 5xx — Server error.
    ServerError,
    /// Unknown status code.
    Unknown,
}

/// Timing breakdown for a request/response cycle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseTiming {
    /// Total time for the entire request.
    pub total: Duration,
    /// DNS lookup time (if available).
    pub dns_lookup: Option<Duration>,
    /// TCP connection time (if available).
    pub tcp_connect: Option<Duration>,
    /// TLS handshake time (if available).
    pub tls_handshake: Option<Duration>,
    /// Time to first byte (if available).
    pub ttfb: Option<Duration>,
    /// Content transfer time (if available).
    pub content_transfer: Option<Duration>,
}

/// Size information for a response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseSize {
    /// Size of the response headers in bytes.
    pub headers_size: u64,
    /// Size of the response body in bytes.
    pub body_size: u64,
}

/// Custom serialization for Vec<u8> as base64.
mod serde_bytes_vec {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)
    }
}
