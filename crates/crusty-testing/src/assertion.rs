//! Built-in assertions for response validation.
//!
//! Provides a declarative assertion system that doesn't require scripting.

use serde::{Deserialize, Serialize};

/// A declarative assertion on a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    /// What to assert on.
    pub target: AssertionTarget,
    /// The comparison operator.
    pub operator: AssertionOp,
    /// Expected value (as string, coerced as needed).
    pub expected: String,
}

/// What part of the response to check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssertionTarget {
    /// HTTP status code.
    Status,
    /// A specific response header.
    Header(String),
    /// Response body (full text).
    Body,
    /// JSONPath expression on the response body.
    JsonPath(String),
    /// Response time in milliseconds.
    ResponseTime,
}

/// Comparison operator for assertions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssertionOp {
    /// Equals.
    Equals,
    /// Not equals.
    NotEquals,
    /// Contains substring.
    Contains,
    /// Does not contain.
    NotContains,
    /// Less than (numeric).
    LessThan,
    /// Greater than (numeric).
    GreaterThan,
    /// Matches regex pattern.
    Matches,
}

/// Result of evaluating an assertion.
#[derive(Debug, Clone)]
pub struct AssertionResult {
    /// The assertion that was evaluated.
    pub assertion: Assertion,
    /// Whether it passed.
    pub passed: bool,
    /// Actual value found.
    pub actual: String,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Evaluate an assertion against response data.
pub fn evaluate(
    assertion: &Assertion,
    status: u16,
    headers: &std::collections::HashMap<String, String>,
    body: &str,
    response_time_ms: u64,
) -> AssertionResult {
    let actual = match &assertion.target {
        AssertionTarget::Status => status.to_string(),
        AssertionTarget::Header(name) => headers.get(name).cloned().unwrap_or_default(),
        AssertionTarget::Body => body.to_string(),
        AssertionTarget::JsonPath(path) => extract_json_path(body, path),
        AssertionTarget::ResponseTime => response_time_ms.to_string(),
    };

    let passed = match &assertion.operator {
        AssertionOp::Equals => actual == assertion.expected,
        AssertionOp::NotEquals => actual != assertion.expected,
        AssertionOp::Contains => actual.contains(&assertion.expected),
        AssertionOp::NotContains => !actual.contains(&assertion.expected),
        AssertionOp::LessThan => actual
            .parse::<f64>()
            .ok()
            .zip(assertion.expected.parse::<f64>().ok())
            .map(|(a, b)| a < b)
            .unwrap_or(false),
        AssertionOp::GreaterThan => actual
            .parse::<f64>()
            .ok()
            .zip(assertion.expected.parse::<f64>().ok())
            .map(|(a, b)| a > b)
            .unwrap_or(false),
        AssertionOp::Matches => regex::Regex::new(&assertion.expected)
            .map(|re| re.is_match(&actual))
            .unwrap_or(false),
    };

    let error = if !passed {
        Some(format!(
            "Expected {:?} {:?} '{}', got '{}'",
            assertion.target, assertion.operator, assertion.expected, actual
        ))
    } else {
        None
    };

    AssertionResult {
        assertion: assertion.clone(),
        passed,
        actual,
        error,
    }
}

/// Simple JSON path extraction (supports dot notation like "users.0.name").
fn extract_json_path(body: &str, path: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(body) else {
        return String::new();
    };

    for segment in path.split('.') {
        value = if let Ok(idx) = segment.parse::<usize>() {
            value.get(idx).cloned().unwrap_or(serde_json::Value::Null)
        } else {
            value
                .get(segment)
                .cloned()
                .unwrap_or(serde_json::Value::Null)
        };
    }

    match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_status_equals() {
        let assertion = Assertion {
            target: AssertionTarget::Status,
            operator: AssertionOp::Equals,
            expected: "200".to_string(),
        };
        let result = evaluate(&assertion, 200, &HashMap::new(), "", 0);
        assert!(result.passed);
    }

    #[test]
    fn test_status_not_equals() {
        let assertion = Assertion {
            target: AssertionTarget::Status,
            operator: AssertionOp::Equals,
            expected: "200".to_string(),
        };
        let result = evaluate(&assertion, 404, &HashMap::new(), "", 0);
        assert!(!result.passed);
    }

    #[test]
    fn test_body_contains() {
        let assertion = Assertion {
            target: AssertionTarget::Body,
            operator: AssertionOp::Contains,
            expected: "success".to_string(),
        };
        let result = evaluate(
            &assertion,
            200,
            &HashMap::new(),
            r#"{"status":"success"}"#,
            0,
        );
        assert!(result.passed);
    }

    #[test]
    fn test_header_check() {
        let assertion = Assertion {
            target: AssertionTarget::Header("content-type".to_string()),
            operator: AssertionOp::Contains,
            expected: "json".to_string(),
        };
        let headers = HashMap::from([("content-type".to_string(), "application/json".to_string())]);
        let result = evaluate(&assertion, 200, &headers, "", 0);
        assert!(result.passed);
    }

    #[test]
    fn test_response_time_less_than() {
        let assertion = Assertion {
            target: AssertionTarget::ResponseTime,
            operator: AssertionOp::LessThan,
            expected: "500".to_string(),
        };
        let result = evaluate(&assertion, 200, &HashMap::new(), "", 150);
        assert!(result.passed);
    }

    #[test]
    fn test_json_path() {
        let assertion = Assertion {
            target: AssertionTarget::JsonPath("users.0.name".to_string()),
            operator: AssertionOp::Equals,
            expected: "Alice".to_string(),
        };
        let body = r#"{"users":[{"name":"Alice"},{"name":"Bob"}]}"#;
        let result = evaluate(&assertion, 200, &HashMap::new(), body, 0);
        assert!(result.passed);
    }

    #[test]
    fn test_json_path_extraction() {
        assert_eq!(extract_json_path(r#"{"a":{"b":[1,2,3]}}"#, "a.b.1"), "2");
    }
}
