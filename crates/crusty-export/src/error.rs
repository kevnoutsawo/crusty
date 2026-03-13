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
}
