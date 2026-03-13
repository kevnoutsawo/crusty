//! Script execution context.
//!
//! Provides the data available to scripts during pre/post-request execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context passed to pre-request scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreRequestContext {
    /// Current request URL.
    pub url: String,
    /// HTTP method.
    pub method: String,
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Environment variables (mutable — scripts can set new ones).
    pub variables: HashMap<String, String>,
}

/// Context passed to post-request scripts (test scripts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostRequestContext {
    /// The request URL that was sent.
    pub url: String,
    /// HTTP method that was used.
    pub method: String,
    /// Response status code.
    pub status: u16,
    /// Response status text.
    pub status_text: String,
    /// Response headers.
    pub response_headers: HashMap<String, String>,
    /// Response body as text.
    pub response_body: String,
    /// Response time in milliseconds.
    pub response_time_ms: u64,
    /// Environment variables (mutable).
    pub variables: HashMap<String, String>,
}

/// Results from running a script.
#[derive(Debug, Clone, Default)]
pub struct ScriptResult {
    /// Updated variables (scripts may have set/changed variables).
    pub variables: HashMap<String, String>,
    /// Log messages produced by the script.
    pub logs: Vec<String>,
    /// Test results (name → passed).
    pub tests: Vec<TestResult>,
    /// Whether all assertions passed.
    pub all_passed: bool,
}

/// A single test result from a script.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name/description.
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Error message if failed.
    pub error: Option<String>,
}
