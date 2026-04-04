//! Mock endpoint definitions.
//!
//! Defines how mock endpoints match requests and generate responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A mock endpoint that matches requests and returns configured responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEndpoint {
    /// Unique identifier for this endpoint.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Request matcher.
    pub matcher: RequestMatcher,
    /// Response to return when matched.
    pub response: MockResponse,
    /// Whether this endpoint is active.
    pub enabled: bool,
    /// Priority (higher = matched first when multiple endpoints match).
    pub priority: i32,
}

/// Criteria for matching incoming requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMatcher {
    /// HTTP method to match (None = any method).
    pub method: Option<String>,
    /// Path pattern to match (supports exact and regex).
    pub path: PathMatcher,
    /// Headers that must be present with matching values.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Query parameters that must be present.
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// Body content match (substring or regex).
    pub body_contains: Option<String>,
}

/// How to match the request path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathMatcher {
    /// Exact path match.
    Exact(String),
    /// Regex pattern match.
    Regex(String),
    /// Prefix match.
    Prefix(String),
}

/// The mock response to return.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body.
    #[serde(default)]
    pub body: String,
    /// Simulated delay in milliseconds.
    #[serde(default)]
    pub delay_ms: u64,
}

impl MockEndpoint {
    /// Create a new mock endpoint with basic settings.
    pub fn new(name: &str, method: &str, path: &str, status: u16, body: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            matcher: RequestMatcher {
                method: Some(method.to_uppercase()),
                path: PathMatcher::Exact(path.to_string()),
                headers: HashMap::new(),
                query_params: HashMap::new(),
                body_contains: None,
            },
            response: MockResponse {
                status,
                headers: HashMap::from([(
                    "content-type".to_string(),
                    "application/json".to_string(),
                )]),
                body: body.to_string(),
                delay_ms: 0,
            },
            enabled: true,
            priority: 0,
        }
    }

    /// Check if this endpoint matches the given request.
    pub fn matches(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        // Check method
        if let Some(ref expected_method) = self.matcher.method {
            if !method.eq_ignore_ascii_case(expected_method) {
                return false;
            }
        }

        // Check path
        match &self.matcher.path {
            PathMatcher::Exact(expected) => {
                if path != expected {
                    return false;
                }
            }
            PathMatcher::Prefix(prefix) => {
                if !path.starts_with(prefix.as_str()) {
                    return false;
                }
            }
            PathMatcher::Regex(pattern) => {
                let Ok(re) = regex::Regex::new(pattern) else {
                    return false;
                };
                if !re.is_match(path) {
                    return false;
                }
            }
        }

        // Check required headers
        for (key, expected_value) in &self.matcher.headers {
            match headers.get(key) {
                Some(actual) if actual == expected_value => {}
                _ => return false,
            }
        }

        // Check body contains
        if let Some(ref expected) = self.matcher.body_contains {
            if !body.contains(expected.as_str()) {
                return false;
            }
        }

        true
    }
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: String::new(),
            delay_ms: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let ep = MockEndpoint::new("test", "GET", "/api/users", 200, "[]");
        let headers = HashMap::new();
        assert!(ep.matches("GET", "/api/users", &headers, ""));
        assert!(!ep.matches("POST", "/api/users", &headers, ""));
        assert!(!ep.matches("GET", "/api/posts", &headers, ""));
    }

    #[test]
    fn test_prefix_match() {
        let mut ep = MockEndpoint::new("test", "GET", "/api", 200, "{}");
        ep.matcher.path = PathMatcher::Prefix("/api".to_string());
        let headers = HashMap::new();
        assert!(ep.matches("GET", "/api/users", &headers, ""));
        assert!(ep.matches("GET", "/api/posts", &headers, ""));
        assert!(!ep.matches("GET", "/other", &headers, ""));
    }

    #[test]
    fn test_regex_match() {
        let mut ep = MockEndpoint::new("test", "GET", "/api", 200, "{}");
        ep.matcher.path = PathMatcher::Regex(r"^/api/users/\d+$".to_string());
        let headers = HashMap::new();
        assert!(ep.matches("GET", "/api/users/123", &headers, ""));
        assert!(!ep.matches("GET", "/api/users/abc", &headers, ""));
    }

    #[test]
    fn test_header_match() {
        let mut ep = MockEndpoint::new("test", "POST", "/api/data", 200, "ok");
        ep.matcher
            .headers
            .insert("x-api-key".to_string(), "secret".to_string());
        let mut headers = HashMap::new();
        assert!(!ep.matches("POST", "/api/data", &headers, ""));
        headers.insert("x-api-key".to_string(), "secret".to_string());
        assert!(ep.matches("POST", "/api/data", &headers, ""));
    }

    #[test]
    fn test_body_contains() {
        let mut ep = MockEndpoint::new("test", "POST", "/api/data", 200, "ok");
        ep.matcher.body_contains = Some("search_term".to_string());
        let headers = HashMap::new();
        assert!(!ep.matches("POST", "/api/data", &headers, "no match here"));
        assert!(ep.matches(
            "POST",
            "/api/data",
            &headers,
            "contains search_term in body"
        ));
    }

    #[test]
    fn test_disabled_endpoint() {
        let mut ep = MockEndpoint::new("test", "GET", "/api", 200, "{}");
        ep.enabled = false;
        let headers = HashMap::new();
        assert!(!ep.matches("GET", "/api", &headers, ""));
    }

    #[test]
    fn test_any_method() {
        let mut ep = MockEndpoint::new("test", "GET", "/api", 200, "{}");
        ep.matcher.method = None;
        let headers = HashMap::new();
        assert!(ep.matches("GET", "/api", &headers, ""));
        assert!(ep.matches("POST", "/api", &headers, ""));
        assert!(ep.matches("DELETE", "/api", &headers, ""));
    }
}
