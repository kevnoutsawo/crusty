//! Proxy error types.

/// Errors that can occur in the proxy server.
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    /// Failed to bind the proxy server.
    #[error("Failed to bind proxy server: {0}")]
    Bind(String),
    /// Error during proxying.
    #[error("Proxy error: {0}")]
    Proxy(String),
    /// Connection error to upstream server.
    #[error("Upstream connection error: {0}")]
    Upstream(String),
}
