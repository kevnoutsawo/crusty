//! The JSON shapes the web client sends and receives, and their conversion to
//! and from the core types. Plain Rust, so it is tested natively.

use crusty_core::request::{HttpMethod, KeyValue, RequestBody, RequestDefinition};
use crusty_core::response::HttpResponse;
use crusty_export::codegen::Language;
use serde::{Deserialize, Serialize};

/// A request as the web client edits it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<WebHeader>,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebHeader {
    pub key: String,
    pub value: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebResponse {
    pub status: u16,
    pub status_text: String,
    /// Sorted by name.
    pub headers: Vec<(String, String)>,
    /// Pretty-printed when the body is JSON.
    pub body: String,
    pub body_is_json: bool,
    pub body_size: u64,
    pub total_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WebLanguage {
    pub id: &'static str,
    pub label: &'static str,
}

pub fn to_definition(req: &WebRequest) -> Result<RequestDefinition, String> {
    let url = req.url.trim();
    if url.is_empty() {
        return Err("Enter a URL first.".to_string());
    }

    let mut def = RequestDefinition::new("Web request", url);
    def.method = parse_method(&req.method)?;

    def.headers = req
        .headers
        .iter()
        .filter(|h| !h.key.trim().is_empty())
        .map(|h| KeyValue {
            key: h.key.trim().to_string(),
            value: h.value.clone(),
            enabled: h.enabled,
        })
        .collect();

    if !req.body.trim().is_empty() {
        let content_type = def
            .headers
            .iter()
            .find(|h| h.enabled && is_content_type(&h.key))
            .map(|h| h.value.trim().to_string());
        let is_json = serde_json::from_str::<serde_json::Value>(&req.body).is_ok();
        let declared_json = content_type.as_deref().is_none_or(|ct| ct.contains("json"));
        def.body = if is_json && declared_json {
            RequestBody::Json(req.body.clone())
        } else {
            RequestBody::Raw {
                content: req.body.clone(),
                content_type: content_type.unwrap_or_else(|| "text/plain".to_string()),
            }
        };
    }

    Ok(def)
}

/// The resolver sets Content-Type from the body type, so a Content-Type header
/// would be sent twice. Code generation needs the header, so this runs only
/// before sending.
pub fn for_sending(mut def: RequestDefinition) -> RequestDefinition {
    if !matches!(def.body, RequestBody::None) {
        def.headers.retain(|h| !is_content_type(&h.key));
    }
    def
}

fn is_content_type(key: &str) -> bool {
    key.trim().eq_ignore_ascii_case("content-type")
}

/// `fetch` throws on these before any request is made, with a message that
/// doesn't say how to fix it. Code generation still accepts them.
pub fn check_browser_can_send(def: &RequestDefinition) -> Result<(), String> {
    let has_body = !matches!(def.body, RequestBody::None);
    if has_body && matches!(def.method, HttpMethod::Get | HttpMethod::Head) {
        return Err(format!(
            "Browsers can't send a body with a {} request. Clear the body or choose another method.",
            def.method
        ));
    }
    if matches!(def.method, HttpMethod::Trace | HttpMethod::Connect) {
        return Err(format!("Browsers don't allow {} requests.", def.method));
    }
    Ok(())
}

pub fn from_definition(def: &RequestDefinition) -> WebRequest {
    let mut url = def.url.clone();
    let params: Vec<&KeyValue> = def.params.iter().filter(|p| p.enabled).collect();
    if !params.is_empty() {
        if let Ok(mut parsed) = url::Url::parse(&url) {
            parsed
                .query_pairs_mut()
                .extend_pairs(params.iter().map(|p| (&p.key, &p.value)));
            url = parsed.to_string();
        }
    }

    let body = match &def.body {
        RequestBody::Json(s) => s.clone(),
        RequestBody::Raw { content, .. } => content.clone(),
        RequestBody::FormUrlEncoded(fields) => url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(
                fields
                    .iter()
                    .filter(|f| f.enabled)
                    .map(|f| (&f.key, &f.value)),
            )
            .finish(),
        RequestBody::None | RequestBody::FormData(_) | RequestBody::Binary(_) => String::new(),
    };

    WebRequest {
        method: def.method.as_str().to_string(),
        url,
        headers: def
            .headers
            .iter()
            .map(|h| WebHeader {
                key: h.key.clone(),
                value: h.value.clone(),
                enabled: h.enabled,
            })
            .collect(),
        body,
    }
}

pub fn from_response(response: &HttpResponse) -> WebResponse {
    let pretty = response.body_json_pretty();
    let body_is_json = pretty.is_some();
    let body = pretty
        .or_else(|| response.body_text().map(String::from))
        .unwrap_or_else(|| format!("<binary data, {} bytes>", response.body.len()));

    let mut headers: Vec<(String, String)> = response
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    headers.sort();

    WebResponse {
        status: response.status,
        status_text: response.status_text.clone(),
        headers,
        body,
        body_is_json,
        body_size: response.size.body_size,
        total_ms: response.timing.total.as_secs_f64() * 1000.0,
    }
}

pub fn languages() -> Vec<WebLanguage> {
    Language::all()
        .iter()
        .map(|&l| WebLanguage {
            id: language_id(l),
            label: l.label(),
        })
        .collect()
}

pub fn language_from_id(id: &str) -> Result<Language, String> {
    Language::all()
        .iter()
        .copied()
        .find(|&l| language_id(l) == id)
        .ok_or_else(|| format!("Unknown language `{id}`"))
}

fn language_id(lang: Language) -> &'static str {
    match lang {
        Language::Curl => "curl",
        Language::Rust => "rust",
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::Go => "go",
    }
}

fn parse_method(method: &str) -> Result<HttpMethod, String> {
    HttpMethod::all()
        .iter()
        .copied()
        .find(|m| m.as_str().eq_ignore_ascii_case(method.trim()))
        .ok_or_else(|| format!("Unknown HTTP method `{method}`"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crusty_core::response::{ResponseSize, ResponseTiming};
    use std::collections::HashMap;
    use std::time::Duration;

    fn header(key: &str, value: &str) -> WebHeader {
        WebHeader {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }

    fn request(method: &str, url: &str) -> WebRequest {
        WebRequest {
            method: method.into(),
            url: url.into(),
            ..Default::default()
        }
    }

    #[test]
    fn method_is_case_insensitive() {
        let def = to_definition(&request("post", "https://example.com")).unwrap();
        assert_eq!(def.method, HttpMethod::Post);
    }

    #[test]
    fn unknown_method_is_an_error() {
        assert!(to_definition(&request("FETCH", "https://example.com")).is_err());
    }

    #[test]
    fn empty_url_is_an_error() {
        assert!(to_definition(&request("GET", "  ")).is_err());
    }

    #[test]
    fn blank_header_rows_are_dropped() {
        let mut req = request("GET", "https://example.com");
        req.headers = vec![header("  ", "x"), header("Accept", "text/html")];
        let def = to_definition(&req).unwrap();
        assert_eq!(def.headers.len(), 1);
        assert_eq!(def.headers[0].key, "Accept");
    }

    #[test]
    fn json_body_becomes_json() {
        let mut req = request("POST", "https://example.com");
        req.body = r#"{"a": 1}"#.into();
        assert!(matches!(
            to_definition(&req).unwrap().body,
            RequestBody::Json(_)
        ));
    }

    #[test]
    fn non_json_body_is_plain_text() {
        let mut req = request("POST", "https://example.com");
        req.body = "hello".into();
        match to_definition(&req).unwrap().body {
            RequestBody::Raw { content_type, .. } => assert_eq!(content_type, "text/plain"),
            other => panic!("expected raw body, got {other:?}"),
        }
    }

    #[test]
    fn content_type_header_sets_the_body_type_and_is_kept() {
        let mut req = request("POST", "https://example.com");
        req.headers = vec![header("content-type", "application/xml")];
        req.body = "<a/>".into();
        let def = to_definition(&req).unwrap();
        assert_eq!(def.headers.len(), 1, "code generation needs the header");
        match def.body {
            RequestBody::Raw { content_type, .. } => assert_eq!(content_type, "application/xml"),
            other => panic!("expected raw body, got {other:?}"),
        }
    }

    #[test]
    fn content_type_is_dropped_for_sending_when_the_body_sets_it() {
        let mut req = request("POST", "https://example.com");
        req.headers = vec![
            header("Content-Type", "application/json"),
            header("Accept", "*/*"),
        ];
        req.body = "{}".into();
        let def = for_sending(to_definition(&req).unwrap());
        let keys: Vec<_> = def.headers.iter().map(|h| h.key.as_str()).collect();
        assert_eq!(keys, ["Accept"]);
    }

    #[test]
    fn generated_code_keeps_the_content_type() {
        let mut req = request("POST", "https://example.com");
        req.headers = vec![header("Content-Type", "application/json")];
        req.body = r#"{"a":1}"#.into();
        let code =
            crusty_export::codegen::generate(&to_definition(&req).unwrap(), Language::Python);
        assert!(code.contains("Content-Type"), "{code}");
    }

    #[test]
    fn json_text_with_non_json_content_type_stays_raw() {
        let mut req = request("POST", "https://example.com");
        req.headers = vec![header("Content-Type", "text/plain")];
        req.body = "[1, 2]".into();
        assert!(matches!(
            to_definition(&req).unwrap().body,
            RequestBody::Raw { .. }
        ));
    }

    #[test]
    fn content_type_without_body_is_still_sent() {
        let mut req = request("GET", "https://example.com");
        req.headers = vec![header("Content-Type", "application/json")];
        let def = for_sending(to_definition(&req).unwrap());
        assert_eq!(def.headers.len(), 1);
    }

    #[test]
    fn browser_rejects_get_or_head_with_a_body() {
        for method in ["GET", "HEAD"] {
            let mut req = request(method, "https://example.com");
            req.body = "{}".into();
            let err = check_browser_can_send(&to_definition(&req).unwrap()).unwrap_err();
            assert!(err.contains(method), "{err}");
        }
    }

    #[test]
    fn browser_accepts_get_without_body_and_post_with_one() {
        let get = request("GET", "https://example.com");
        assert!(check_browser_can_send(&to_definition(&get).unwrap()).is_ok());
        let mut post = request("POST", "https://example.com");
        post.body = "{}".into();
        assert!(check_browser_can_send(&to_definition(&post).unwrap()).is_ok());
    }

    #[test]
    fn browser_rejects_trace() {
        let req = request("TRACE", "https://example.com");
        assert!(check_browser_can_send(&to_definition(&req).unwrap()).is_err());
    }

    #[test]
    fn round_trips_through_definition() {
        let mut req = request("PUT", "https://example.com/users/1");
        req.headers = vec![header("Authorization", "Bearer x")];
        req.body = r#"{"name":"Bob"}"#.into();
        let back = from_definition(&to_definition(&req).unwrap());
        assert_eq!(back, req);
    }

    #[test]
    fn enabled_params_are_folded_into_the_url() {
        let mut def = RequestDefinition::new("r", "https://example.com/search");
        def.params = vec![KeyValue::new("q", "rust gpui"), {
            let mut off = KeyValue::new("page", "2");
            off.enabled = false;
            off
        }];
        assert_eq!(
            from_definition(&def).url,
            "https://example.com/search?q=rust+gpui"
        );
    }

    #[test]
    fn form_body_is_encoded() {
        let mut def = RequestDefinition::new("r", "https://example.com");
        def.body =
            RequestBody::FormUrlEncoded(vec![KeyValue::new("a", "1 2"), KeyValue::new("b", "&")]);
        assert_eq!(from_definition(&def).body, "a=1+2&b=%26");
    }

    #[test]
    fn response_json_is_pretty_and_headers_sorted() {
        let response = HttpResponse {
            status: 201,
            status_text: "Created".into(),
            headers: HashMap::from([
                ("x-b".to_string(), "2".to_string()),
                ("x-a".to_string(), "1".to_string()),
            ]),
            body: br#"{"ok":true}"#.to_vec(),
            timing: ResponseTiming {
                total: Duration::from_millis(12),
                dns_lookup: None,
                tcp_connect: None,
                tls_handshake: None,
                ttfb: None,
                content_transfer: None,
            },
            size: ResponseSize {
                headers_size: 0,
                body_size: 11,
            },
        };
        let web = from_response(&response);
        assert!(web.body_is_json);
        assert!(web.body.contains("\n"), "expected pretty-printed JSON");
        assert_eq!(web.headers[0].0, "x-a");
        assert_eq!(web.total_ms, 12.0);
    }

    #[test]
    fn every_language_has_a_unique_id_that_round_trips() {
        let langs = languages();
        for l in &langs {
            assert_eq!(language_id(language_from_id(l.id).unwrap()), l.id);
        }
        let mut ids: Vec<_> = langs.iter().map(|l| l.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), langs.len());
    }
}
