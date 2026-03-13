//! Export/import error types.

use thiserror::Error;

/// Errors that can occur during import/export.
#[derive(Debug, Error)]
pub enum ExportError {
    /// Failed to parse a cURL command.
    #[error("cURL parse error: {0}")]
    CurlParse(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// JSON parsing error.
    #[error("JSON parse error: {0}")]
    JsonParse(String),

    /// Invalid format or structure.
    #[error("invalid format: {0}")]
    InvalidFormat(String),
}

impl From<serde_json::Error> for ExportError {
    fn from(e: serde_json::Error) -> Self {
        Self::JsonParse(e.to_string())
    }
}
