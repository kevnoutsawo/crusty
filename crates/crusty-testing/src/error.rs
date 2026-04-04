//! Error types for the test runner.

/// Errors from test execution.
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    /// HTTP request failed.
    #[error("Request failed: {0}")]
    RequestFailed(String),

    /// Script execution failed.
    #[error("Script error: {0}")]
    ScriptError(String),

    /// Collection error.
    #[error("Collection error: {0}")]
    CollectionError(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<crusty_scripting::ScriptError> for TestError {
    fn from(e: crusty_scripting::ScriptError) -> Self {
        Self::ScriptError(e.to_string())
    }
}
