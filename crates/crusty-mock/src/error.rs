//! Mock server error types.

/// Errors that can occur in the mock server.
#[derive(Debug, thiserror::Error)]
pub enum MockError {
    /// Failed to bind the server to a port.
    #[error("Failed to bind mock server: {0}")]
    Bind(String),
    /// Server error during request handling.
    #[error("Server error: {0}")]
    Server(String),
    /// Invalid endpoint configuration.
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),
}
