//! HTTP engine error types.

use thiserror::Error;

/// Errors that can occur during HTTP operations.
#[derive(Debug, Error)]
pub enum HttpError {
    /// The request failed to build.
    #[error("failed to build request: {0}")]
    RequestBuild(String),

    /// A network or transport error occurred.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// The URL is invalid.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Timeout exceeded.
    #[error("request timed out after {0}ms")]
    Timeout(u64),
}
