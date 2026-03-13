//! Error types for the scripting engine.

/// Errors from script execution.
#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    /// Script compilation error.
    #[error("Script compilation error: {0}")]
    Compile(String),

    /// Script runtime error.
    #[error("Script runtime error: {0}")]
    Runtime(String),

    /// Assertion failure from a script.
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
}
