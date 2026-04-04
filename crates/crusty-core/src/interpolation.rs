//! Variable interpolation engine.
//!
//! Replaces `{{variable}}` placeholders in strings with their resolved values
//! from the environment. Also supports built-in dynamic variables like
//! `{{$timestamp}}`, `{{$randomUUID}}`, etc.

use crate::error::{CoreError, Result};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use uuid::Uuid;

static VAR_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{(\$?\w+)\}\}").expect("valid regex"));

/// Interpolate all `{{variable}}` references in the given string.
///
/// - Regular variables (e.g., `{{host}}`) are looked up in `variables`.
/// - Dynamic variables (e.g., `{{$timestamp}}`) are generated on the fly.
///
/// Returns an error if a referenced variable is not found and is not a
/// recognized dynamic variable.
pub fn interpolate(template: &str, variables: &HashMap<String, String>) -> Result<String> {
    let mut result = String::with_capacity(template.len());
    let mut last_end = 0;

    for cap in VAR_PATTERN.captures_iter(template) {
        let full_match = cap.get(0).expect("capture group 0 always exists");
        let var_name = &cap[1];

        result.push_str(&template[last_end..full_match.start()]);

        let value = resolve_variable(var_name, variables)?;
        result.push_str(&value);

        last_end = full_match.end();
    }

    result.push_str(&template[last_end..]);
    Ok(result)
}

/// Find all variable references in a template string.
pub fn find_variables(template: &str) -> Vec<String> {
    VAR_PATTERN
        .captures_iter(template)
        .map(|cap| cap[1].to_string())
        .collect()
}

fn resolve_variable(name: &str, variables: &HashMap<String, String>) -> Result<String> {
    // Check for dynamic variables first
    if let Some(value) = resolve_dynamic(name) {
        return Ok(value);
    }

    // Look up in the provided variables map
    variables
        .get(name)
        .cloned()
        .ok_or_else(|| CoreError::UndefinedVariable {
            name: name.to_string(),
        })
}

fn resolve_dynamic(name: &str) -> Option<String> {
    match name {
        "$timestamp" => Some(chrono::Utc::now().timestamp().to_string()),
        "$isoTimestamp" => Some(chrono::Utc::now().to_rfc3339()),
        "$randomUUID" => Some(Uuid::new_v4().to_string()),
        "$randomInt" => {
            let val: u32 = rand_u32() % 1000;
            Some(val.to_string())
        }
        _ => None,
    }
}

/// Simple pseudo-random u32 using timestamp as seed.
fn rand_u32() -> u32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Use nanoseconds for some variation
    (now.subsec_nanos() ^ 0xDEAD_BEEF).wrapping_mul(2654435761)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_interpolation() {
        let mut vars = HashMap::new();
        vars.insert("host".into(), "localhost".into());
        vars.insert("port".into(), "8080".into());

        let result = interpolate("http://{{host}}:{{port}}/api", &vars).unwrap();
        assert_eq!(result, "http://localhost:8080/api");
    }

    #[test]
    fn test_no_variables() {
        let vars = HashMap::new();
        let result = interpolate("http://example.com/api", &vars).unwrap();
        assert_eq!(result, "http://example.com/api");
    }

    #[test]
    fn test_undefined_variable_error() {
        let vars = HashMap::new();
        let result = interpolate("{{missing}}", &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_dynamic_timestamp() {
        let vars = HashMap::new();
        let result = interpolate("ts={{$timestamp}}", &vars).unwrap();
        assert!(result.starts_with("ts="));
        assert!(result.len() > 3);
    }

    #[test]
    fn test_dynamic_uuid() {
        let vars = HashMap::new();
        let result = interpolate("id={{$randomUUID}}", &vars).unwrap();
        // UUID format: 8-4-4-4-12
        let uuid_part = &result[3..];
        assert!(Uuid::parse_str(uuid_part).is_ok());
    }

    #[test]
    fn test_find_variables() {
        let vars = find_variables("{{host}}:{{port}}/{{$timestamp}}");
        assert_eq!(vars, vec!["host", "port", "$timestamp"]);
    }

    #[test]
    fn test_mixed_static_and_dynamic() {
        let mut vars = HashMap::new();
        vars.insert("host".into(), "example.com".into());

        let result = interpolate("https://{{host}}/{{$randomUUID}}", &vars).unwrap();
        assert!(result.starts_with("https://example.com/"));
    }
}
