//! Error types for protocol adapters.

/// Errors from protocol operations.
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    /// WebSocket connection error.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Invalid URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Connection closed.
    #[error("Connection closed")]
    ConnectionClosed,

    /// Timeout.
    #[error("Operation timed out")]
    Timeout,

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<tokio_tungstenite::tungstenite::Error> for ProtoError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(e.to_string())
    }
}

impl From<url::ParseError> for ProtoError {
    fn from(e: url::ParseError) -> Self {
        Self::InvalidUrl(e.to_string())
    }
}
