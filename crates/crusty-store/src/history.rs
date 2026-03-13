//! Request history entry model.

use serde::{Deserialize, Serialize};

/// A record of a sent request and its response, stored in history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique ID for this history entry.
    pub id: String,
    /// HTTP method used.
    pub method: String,
    /// URL that was requested.
    pub url: String,
    /// Response status code (None if the request failed).
    pub status: Option<u16>,
    /// Total duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Serialized request definition.
    pub request_data: String,
    /// Serialized response (None if the request failed).
    pub response_data: Option<String>,
    /// When this request was sent (ISO 8601).
    pub timestamp: String,
}
