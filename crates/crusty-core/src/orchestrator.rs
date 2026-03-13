//! Request orchestration.
//!
//! Takes a `RequestDefinition`, resolves environment variables,
//! applies authentication, and produces a `ResolvedRequest` ready
//! to be sent by the HTTP engine.

use crate::environment::Environment;
use crate::error::{CoreError, Result};
use crate::interpolation;
use crate::request::{ResolvedBody, ResolvedRequest, RequestBody, RequestDefinition};
use std::collections::HashMap;

/// Build a `ResolvedRequest` from a definition, environment layers, and optional auth.
///
/// This is the main entry point for turning user-defined requests into
/// something the HTTP engine can execute.
pub fn resolve_request(
    definition: &RequestDefinition,
    environments: &[&Environment],
    auth_headers: &HashMap<String, String>,
) -> Result<ResolvedRequest> {
    // Merge all environment variables
    let variables = crate::environment::resolve_layers(environments);

    // Interpolate URL
    let url_str = interpolation::interpolate(&definition.url, &variables)?;
    let url_str = if !url_str.contains("://") {
        format!("https://{url_str}")
    } else {
        url_str
    };

    let mut url = url::Url::parse(&url_str).map_err(|e| CoreError::InvalidUrl {
        url: url_str.clone(),
        reason: e.to_string(),
    })?;

    // Append enabled query params
    let enabled_params: Vec<_> = definition
        .params
        .iter()
        .filter(|p| p.enabled)
        .collect();

    if !enabled_params.is_empty() {
        let mut query_pairs = url.query_pairs_mut();
        for param in enabled_params {
            let key = interpolation::interpolate(&param.key, &variables)?;
            let value = interpolation::interpolate(&param.value, &variables)?;
            query_pairs.append_pair(&key, &value);
        }
    }

    // Resolve headers
    let mut headers = HashMap::new();

    // Auth headers first (can be overridden by request headers)
    for (k, v) in auth_headers {
        headers.insert(k.clone(), v.clone());
    }

    for h in &definition.headers {
        if h.enabled {
            let key = interpolation::interpolate(&h.key, &variables)?;
            let value = interpolation::interpolate(&h.value, &variables)?;
            headers.insert(key, value);
        }
    }

    // Resolve body
    let body = resolve_body(&definition.body, &variables)?;

    Ok(ResolvedRequest {
        method: definition.method,
        url,
        headers,
        body,
        settings: definition.settings.clone(),
    })
}

fn resolve_body(body: &RequestBody, variables: &HashMap<String, String>) -> Result<ResolvedBody> {
    match body {
        RequestBody::None => Ok(ResolvedBody::None),
        RequestBody::Json(json) => {
            let interpolated = interpolation::interpolate(json, variables)?;
            Ok(ResolvedBody::Bytes {
                data: interpolated.into_bytes(),
                content_type: "application/json".to_string(),
            })
        }
        RequestBody::Raw {
            content,
            content_type,
        } => {
            let interpolated = interpolation::interpolate(content, variables)?;
            Ok(ResolvedBody::Bytes {
                data: interpolated.into_bytes(),
                content_type: content_type.clone(),
            })
        }
        RequestBody::FormUrlEncoded(fields) => {
            let mut pairs = Vec::new();
            for field in fields {
                if field.enabled {
                    let key = interpolation::interpolate(&field.key, variables)?;
                    let value = interpolation::interpolate(&field.value, variables)?;
                    pairs.push(format!(
                        "{}={}",
                        urlencoded_encode(&key),
                        urlencoded_encode(&value)
                    ));
                }
            }
            let body_str = pairs.join("&");
            Ok(ResolvedBody::Bytes {
                data: body_str.into_bytes(),
                content_type: "application/x-www-form-urlencoded".to_string(),
            })
        }
        RequestBody::FormData(_entries) => {
            // Multipart form data requires special handling (boundary generation).
            // For now, signal that we have form data that the HTTP engine
            // will need to handle with reqwest's multipart API.
            Ok(ResolvedBody::None) // TODO: implement multipart
        }
        RequestBody::Binary(path) => {
            let interpolated_path = interpolation::interpolate(path, variables)?;
            let data = std::fs::read(&interpolated_path).map_err(|e| {
                CoreError::RequestBuild(format!(
                    "failed to read binary file `{interpolated_path}`: {e}"
                ))
            })?;
            Ok(ResolvedBody::Bytes {
                data,
                content_type: "application/octet-stream".to_string(),
            })
        }
    }
}

fn urlencoded_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Environment;
    use crate::request::HttpMethod;
    use crate::request::{KeyValue, RequestDefinition};

    #[test]
    fn test_resolve_simple_get() {
        let def = RequestDefinition::new("Test", "https://api.example.com/users");
        let result = resolve_request(&def, &[], &HashMap::new()).unwrap();
        assert_eq!(result.url.as_str(), "https://api.example.com/users");
        assert_eq!(result.method, HttpMethod::Get);
    }

    #[test]
    fn test_resolve_with_env_vars() {
        let mut env = Environment::new("Test");
        env.add_variable("host", "api.example.com");
        env.add_variable("version", "v2");

        let def = RequestDefinition::new("Test", "https://{{host}}/{{version}}/users");
        let result = resolve_request(&def, &[&env], &HashMap::new()).unwrap();
        assert_eq!(result.url.as_str(), "https://api.example.com/v2/users");
    }

    #[test]
    fn test_resolve_with_params() {
        let mut def = RequestDefinition::new("Test", "https://api.example.com/search");
        def.params.push(KeyValue::new("q", "rust"));
        def.params.push(KeyValue::new("page", "1"));

        let result = resolve_request(&def, &[], &HashMap::new()).unwrap();
        assert!(result.url.as_str().contains("q=rust"));
        assert!(result.url.as_str().contains("page=1"));
    }

    #[test]
    fn test_resolve_with_headers() {
        let mut def = RequestDefinition::new("Test", "https://api.example.com");
        def.headers
            .push(KeyValue::new("Accept", "application/json"));

        let result = resolve_request(&def, &[], &HashMap::new()).unwrap();
        assert_eq!(
            result.headers.get("Accept").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_resolve_with_auth_headers() {
        let def = RequestDefinition::new("Test", "https://api.example.com");
        let mut auth = HashMap::new();
        auth.insert("Authorization".to_string(), "Bearer token123".to_string());

        let result = resolve_request(&def, &[], &auth).unwrap();
        assert_eq!(
            result.headers.get("Authorization").unwrap(),
            "Bearer token123"
        );
    }

    #[test]
    fn test_resolve_json_body_with_interpolation() {
        let mut env = Environment::new("Test");
        env.add_variable("user_name", "Alice");

        let mut def = RequestDefinition::new("Test", "https://api.example.com/users");
        def.method = HttpMethod::Post;
        def.body = RequestBody::Json(r#"{"name": "{{user_name}}"}"#.to_string());

        let result = resolve_request(&def, &[&env], &HashMap::new()).unwrap();
        match &result.body {
            ResolvedBody::Bytes { data, content_type } => {
                assert_eq!(content_type, "application/json");
                let body_str = String::from_utf8(data.clone()).unwrap();
                assert_eq!(body_str, r#"{"name": "Alice"}"#);
            }
            ResolvedBody::None => panic!("Expected body"),
        }
    }

    #[test]
    fn test_auto_prepend_https() {
        let def = RequestDefinition::new("Test", "example.com/api");
        let result = resolve_request(&def, &[], &HashMap::new()).unwrap();
        assert!(result.url.as_str().starts_with("https://"));
    }
}
