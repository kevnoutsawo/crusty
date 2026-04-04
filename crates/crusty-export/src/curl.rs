//! cURL import and export.
//!
//! Parse a cURL command string into a `RequestDefinition`,
//! and export a `RequestDefinition` as a cURL command string.

use crate::error::ExportError;
use crusty_core::request::{HttpMethod, KeyValue, RequestBody, RequestDefinition};

/// Parse a cURL command string into a `RequestDefinition`.
///
/// Supports common cURL flags:
/// - `-X, --request METHOD`
/// - `-H, --header "Key: Value"`
/// - `-d, --data "body"`
/// - `--data-raw "body"`
/// - `-u, --user "user:pass"` (adds Authorization header)
/// - `-k, --insecure` (disables SSL verification)
pub fn import(curl_cmd: &str) -> Result<RequestDefinition, ExportError> {
    let tokens = tokenize(curl_cmd)?;
    let mut def = RequestDefinition::new("Imported cURL", "");
    let mut i = 0;

    // Skip "curl" if present
    if tokens.first().map(|s| s.as_str()) == Some("curl") {
        i = 1;
    }

    while i < tokens.len() {
        let token = &tokens[i];
        match token.as_str() {
            "-X" | "--request" => {
                i += 1;
                if i < tokens.len() {
                    def.method = parse_method(&tokens[i]);
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((key, value)) = parse_header(&tokens[i]) {
                        def.headers.push(KeyValue::new(key, value));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if i < tokens.len() {
                    def.body = RequestBody::Json(tokens[i].clone());
                    // Default to POST if method wasn't explicitly set
                    if def.method == HttpMethod::Get {
                        def.method = HttpMethod::Post;
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < tokens.len() {
                    let creds = &tokens[i];
                    let encoded = base64_encode(creds.as_bytes());
                    def.headers
                        .push(KeyValue::new("Authorization", format!("Basic {encoded}")));
                }
            }
            "-k" | "--insecure" => {
                def.settings.verify_ssl = false;
            }
            "--compressed" | "-s" | "--silent" | "-S" | "--show-error" | "-L" | "--location"
            | "-v" | "--verbose" => {
                // Ignored flags — common but don't affect the request definition
            }
            arg if arg.starts_with('-') => {
                // Unknown flag — skip its argument if it looks like it takes one
                if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                    i += 1; // Skip the argument
                }
            }
            _ => {
                // Positional argument — treat as URL
                if def.url.is_empty() {
                    def.url = token.clone();
                }
            }
        }
        i += 1;
    }

    if def.url.is_empty() {
        return Err(ExportError::CurlParse(
            "no URL found in cURL command".into(),
        ));
    }

    Ok(def)
}

/// Export a `RequestDefinition` as a cURL command string.
pub fn export(def: &RequestDefinition) -> String {
    let mut parts = vec!["curl".to_string()];

    // Method (omit for GET since it's the default)
    if def.method != HttpMethod::Get {
        parts.push("-X".to_string());
        parts.push(def.method.as_str().to_string());
    }

    // URL
    parts.push(shell_quote(&def.url));

    // Headers
    for h in &def.headers {
        if h.enabled {
            parts.push("-H".to_string());
            parts.push(shell_quote(&format!("{}: {}", h.key, h.value)));
        }
    }

    // Body
    match &def.body {
        RequestBody::Json(json) => {
            parts.push("-d".to_string());
            parts.push(shell_quote(json));
        }
        RequestBody::Raw { content, .. } => {
            parts.push("-d".to_string());
            parts.push(shell_quote(content));
        }
        RequestBody::FormUrlEncoded(fields) => {
            for field in fields {
                if field.enabled {
                    parts.push("-d".to_string());
                    parts.push(shell_quote(&format!("{}={}", field.key, field.value)));
                }
            }
        }
        _ => {}
    }

    // Query params
    if !def.params.is_empty() {
        let enabled: Vec<_> = def.params.iter().filter(|p| p.enabled).collect();
        if !enabled.is_empty() {
            // Rebuild URL with params
            let url_with_params = if def.url.contains('?') {
                let param_str: String = enabled
                    .iter()
                    .map(|p| format!("{}={}", p.key, p.value))
                    .collect::<Vec<_>>()
                    .join("&");
                format!("{}&{}", def.url, param_str)
            } else {
                let param_str: String = enabled
                    .iter()
                    .map(|p| format!("{}={}", p.key, p.value))
                    .collect::<Vec<_>>()
                    .join("&");
                format!("{}?{}", def.url, param_str)
            };
            // Replace the URL we already added
            if let Some(pos) = parts
                .iter()
                .position(|p| p.contains(&def.url) || p.contains("://"))
            {
                parts[pos] = shell_quote(&url_with_params);
            }
        }
    }

    if !def.settings.verify_ssl {
        parts.push("-k".to_string());
    }

    parts.join(" ")
}

// --- Helpers ---

fn tokenize(input: &str) -> Result<Vec<String>, ExportError> {
    let mut tokens = Vec::new();
    let cleaned: String = input
        .lines()
        .map(|line| {
            let l = line.trim();
            if let Some(stripped) = l.strip_suffix('\\') {
                stripped
            } else {
                l
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut chars = cleaned.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '\'' => {
                chars.next();
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '\'' {
                        chars.next();
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(s);
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        chars.next();
                        break;
                    }
                    if c == '\\' {
                        chars.next();
                        if let Some(&next) = chars.peek() {
                            s.push(next);
                            chars.next();
                        }
                        continue;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(s);
            }
            _ => {
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(s);
            }
        }
    }

    Ok(tokens)
}

fn parse_method(s: &str) -> HttpMethod {
    match s.to_uppercase().as_str() {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "PATCH" => HttpMethod::Patch,
        "DELETE" => HttpMethod::Delete,
        "HEAD" => HttpMethod::Head,
        "OPTIONS" => HttpMethod::Options,
        "TRACE" => HttpMethod::Trace,
        "CONNECT" => HttpMethod::Connect,
        _ => HttpMethod::Get,
    }
}

fn parse_header(s: &str) -> Option<(String, String)> {
    let colon_pos = s.find(':')?;
    let key = s[..colon_pos].trim().to_string();
    let value = s[colon_pos + 1..].trim().to_string();
    Some((key, value))
}

fn shell_quote(s: &str) -> String {
    if s.contains('\'') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else if s.contains('"') || s.contains(' ') || s.contains('&') || s.contains('?') {
        format!("'{s}'")
    } else {
        s.to_string()
    }
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_simple_get() {
        let def = import("curl https://api.example.com/users").unwrap();
        assert_eq!(def.method, HttpMethod::Get);
        assert_eq!(def.url, "https://api.example.com/users");
    }

    #[test]
    fn test_import_post_with_data() {
        let def = import(
            r#"curl -X POST https://api.example.com/users -H 'Content-Type: application/json' -d '{"name":"Alice"}'"#,
        )
        .unwrap();
        assert_eq!(def.method, HttpMethod::Post);
        assert_eq!(def.url, "https://api.example.com/users");
        assert_eq!(def.headers.len(), 1);
        assert_eq!(def.headers[0].key, "Content-Type");
        match &def.body {
            RequestBody::Json(s) => assert_eq!(s, r#"{"name":"Alice"}"#),
            _ => panic!("Expected JSON body"),
        }
    }

    #[test]
    fn test_import_with_multiple_headers() {
        let def = import(
            r#"curl -H "Authorization: Bearer token123" -H "Accept: application/json" https://api.example.com"#,
        )
        .unwrap();
        assert_eq!(def.headers.len(), 2);
        assert_eq!(def.headers[0].key, "Authorization");
        assert_eq!(def.headers[0].value, "Bearer token123");
    }

    #[test]
    fn test_import_multiline() {
        let def = import(
            "curl \\\n  -X POST \\\n  https://api.example.com \\\n  -H 'Content-Type: application/json' \\\n  -d '{\"key\":\"value\"}'",
        )
        .unwrap();
        assert_eq!(def.method, HttpMethod::Post);
        assert_eq!(def.url, "https://api.example.com");
    }

    #[test]
    fn test_import_implicit_post() {
        let def = import(r#"curl https://api.example.com -d '{"x":1}'"#).unwrap();
        assert_eq!(def.method, HttpMethod::Post);
    }

    #[test]
    fn test_import_insecure() {
        let def = import("curl -k https://self-signed.example.com").unwrap();
        assert!(!def.settings.verify_ssl);
    }

    #[test]
    fn test_export_simple_get() {
        let def = RequestDefinition::new("Test", "https://api.example.com/users");
        let curl = export(&def);
        assert!(curl.contains("curl"));
        assert!(curl.contains("https://api.example.com/users"));
        assert!(!curl.contains("-X")); // GET is default
    }

    #[test]
    fn test_export_post_with_body() {
        let mut def = RequestDefinition::new("Test", "https://api.example.com/users");
        def.method = HttpMethod::Post;
        def.headers
            .push(KeyValue::new("Content-Type", "application/json"));
        def.body = RequestBody::Json(r#"{"name":"Alice"}"#.to_string());

        let curl = export(&def);
        assert!(curl.contains("-X POST"));
        assert!(curl.contains("Content-Type: application/json"));
        assert!(curl.contains(r#"{"name":"Alice"}"#));
    }

    #[test]
    fn test_roundtrip() {
        let original = r#"curl -X PUT https://api.example.com/users/1 -H 'Content-Type: application/json' -d '{"name":"Bob"}'"#;
        let def = import(original).unwrap();
        assert_eq!(def.method, HttpMethod::Put);

        let exported = export(&def);
        assert!(exported.contains("-X PUT"));
        assert!(exported.contains("https://api.example.com/users/1"));
    }

    #[test]
    fn test_no_url_error() {
        let result = import("curl -H 'Accept: */*'");
        assert!(result.is_err());
    }
}
