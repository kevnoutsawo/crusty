//! Auth error types.

use thiserror::Error;

/// Errors that can occur during authentication.
#[derive(Debug, Error)]
pub enum AuthError {
    /// Missing required field.
    #[error("missing required auth field: {0}")]
    MissingField(String),

    /// Token encoding error.
    #[error("token encoding error: {0}")]
    Encoding(String),
}
