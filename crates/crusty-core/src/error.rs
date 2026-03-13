//! Error types for the Crusty core crate.

use thiserror::Error;

/// Core error type for Crusty operations.
#[derive(Debug, Error)]
pub enum CoreError {
    /// A variable referenced in interpolation was not found.
    #[error("undefined variable: `{name}` — check your environment or collection variables")]
    UndefinedVariable {
        /// The name of the missing variable.
        name: String,
    },

    /// Invalid URL after interpolation.
    #[error("invalid URL `{url}`: {reason}")]
    InvalidUrl {
        /// The URL that failed to parse.
        url: String,
        /// Why it failed.
        reason: String,
    },

    /// A request could not be constructed.
    #[error("request build error: {0}")]
    RequestBuild(String),

    /// Collection operation failed.
    #[error("collection error: {0}")]
    Collection(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP-related error.
    #[error("HTTP error: {0}")]
    Http(String),
}

/// Alias for `Result<T, CoreError>`.
pub type Result<T> = std::result::Result<T, CoreError>;
