//! HAR (HTTP Archive) v1.2 import/export.
//!
//! Converts between HAR format and Crusty request/response data.

use crate::ExportError;
use crusty_core::request::{HttpMethod, KeyValue, RequestBody, RequestDefinition};
use crusty_core::response::HttpResponse;
use serde::{Deserialize, Serialize};

// --- HAR v1.2 JSON types ---

#[derive(Debug, Serialize, Deserialize)]
struct Har {
    log: HarLog,
}

#[derive(Debug, Serialize, Deserialize)]
struct HarLog {
    version: String,
    creator: HarCreator,
    #[serde(default)]
    entries: Vec<HarEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HarCreator {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HarEntry {
    request: HarRequest,
    #[serde(default)]
    response: Option<HarResponse>,
    #[serde(default, rename = "startedDateTime")]
    started_date_time: String,
    #[serde(default)]
    time: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct HarRequest {
    method: String,
    url: String,
    #[serde(default, rename = "httpVersion")]
    http_version: String,
    #[serde(default)]
    headers: Vec<HarNameValue>,
    #[serde(default, rename = "queryString")]
    query_string: Vec<HarNameValue>,
    #[serde(default, rename = "postData")]
    post_data: Option<HarPostData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HarResponse {
    status: u16,
    #[serde(default, rename = "statusText")]
    status_text: String,
    #[serde(default)]
    headers: Vec<HarNameValue>,
    content: HarContent,
}

#[derive(Debug, Serialize, Deserialize)]
struct HarNameValue {
    name: String,
    value: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HarPostData {
    #[serde(default, rename = "mimeType")]
    mime_type: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    params: Vec<HarNameValue>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HarContent {
    #[serde(default)]
    size: i64,
    #[serde(default, rename = "mimeType")]
    mime_type: String,
    #[serde(default)]
    text: Option<String>,
}

// --- Import ---

/// Import requests from a HAR JSON string.
pub fn import(json: &str) -> Result<Vec<RequestDefinition>, ExportError> {
    let har: Har = serde_json::from_str(json)?;
    let mut requests = Vec::new();

    for entry in &har.log.entries {
        let req = &entry.request;

        // Strip query string from URL (we'll store params separately)
        let base_url = req.url.split('?').next().unwrap_or(&req.url).to_string();
        let name = format!("{} {}", req.method, truncate(&base_url, 60));

        let mut def = RequestDefinition::new(&name, &req.url);
        def.method = parse_method(&req.method);

        def.headers = req
            .headers
            .iter()
            .filter(|h| !is_pseudo_header(&h.name))
            .map(|h| KeyValue::new(&h.name, &h.value))
            .collect();

        def.params = req
            .query_string
            .iter()
            .map(|q| KeyValue::new(&q.name, &q.value))
            .collect();

        if let Some(ref post_data) = req.post_data {
            if !post_data.params.is_empty() {
                def.body = RequestBody::FormUrlEncoded(
                    post_data
                        .params
                        .iter()
                        .map(|p| KeyValue::new(&p.name, &p.value))
                        .collect(),
                );
            } else if !post_data.text.is_empty() {
                if post_data.mime_type.contains("json") {
                    def.body = RequestBody::Json(post_data.text.clone());
                } else {
                    def.body = RequestBody::Raw {
                        content: post_data.text.clone(),
                        content_type: post_data.mime_type.clone(),
                    };
                }
            }
        }

        requests.push(def);
    }

    Ok(requests)
}

// --- Export ---

/// Export requests (with optional responses) to HAR v1.2 JSON.
pub fn export(
    entries: &[(RequestDefinition, Option<&HttpResponse>)],
) -> Result<String, ExportError> {
    let har = Har {
        log: HarLog {
            version: "1.2".to_string(),
            creator: HarCreator {
                name: "Crusty".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            entries: entries
                .iter()
                .map(|(req, resp)| export_entry(req, *resp))
                .collect(),
        },
    };
    serde_json::to_string_pretty(&har).map_err(|e| ExportError::Serialization(e.to_string()))
}

fn export_entry(req: &RequestDefinition, resp: Option<&HttpResponse>) -> HarEntry {
    HarEntry {
        request: export_request(req),
        response: resp.map(export_response),
        started_date_time: chrono::Utc::now().to_rfc3339(),
        time: resp
            .map(|r| r.timing.total.as_millis() as f64)
            .unwrap_or(0.0),
    }
}

fn export_request(req: &RequestDefinition) -> HarRequest {
    HarRequest {
        method: req.method.as_str().to_string(),
        url: req.url.clone(),
        http_version: "HTTP/1.1".to_string(),
        headers: req
            .headers
            .iter()
            .filter(|h| h.enabled)
            .map(|h| HarNameValue {
                name: h.key.clone(),
                value: h.value.clone(),
            })
            .collect(),
        query_string: req
            .params
            .iter()
            .filter(|p| p.enabled)
            .map(|p| HarNameValue {
                name: p.key.clone(),
                value: p.value.clone(),
            })
            .collect(),
        post_data: export_post_data(&req.body),
    }
}

fn export_post_data(body: &RequestBody) -> Option<HarPostData> {
    match body {
        RequestBody::None => None,
        RequestBody::Json(raw) => Some(HarPostData {
            mime_type: "application/json".to_string(),
            text: raw.clone(),
            params: Vec::new(),
        }),
        RequestBody::Raw {
            content,
            content_type,
        } => Some(HarPostData {
            mime_type: content_type.clone(),
            text: content.clone(),
            params: Vec::new(),
        }),
        RequestBody::FormUrlEncoded(params) => Some(HarPostData {
            mime_type: "application/x-www-form-urlencoded".to_string(),
            text: String::new(),
            params: params
                .iter()
                .map(|p| HarNameValue {
                    name: p.key.clone(),
                    value: p.value.clone(),
                })
                .collect(),
        }),
        _ => None,
    }
}

fn export_response(resp: &HttpResponse) -> HarResponse {
    HarResponse {
        status: resp.status,
        status_text: resp.status_text.clone(),
        headers: resp
            .headers
            .iter()
            .map(|(k, v)| HarNameValue {
                name: k.clone(),
                value: v.clone(),
            })
            .collect(),
        content: HarContent {
            size: resp.body.len() as i64,
            mime_type: resp
                .headers
                .get("content-type")
                .cloned()
                .unwrap_or_default(),
            text: resp.body_text().map(|s| s.to_string()),
        },
    }
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
        _ => HttpMethod::Get,
    }
}

fn is_pseudo_header(name: &str) -> bool {
    name.starts_with(':')
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HAR: &str = r#"{
        "log": {
            "version": "1.2",
            "creator": {"name": "test", "version": "1.0"},
            "entries": [
                {
                    "request": {
                        "method": "GET",
                        "url": "https://api.example.com/users?page=1",
                        "httpVersion": "HTTP/1.1",
                        "headers": [
                            {"name": "Accept", "value": "application/json"},
                            {"name": "Authorization", "value": "Bearer token123"}
                        ],
                        "queryString": [
                            {"name": "page", "value": "1"}
                        ]
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "headers": [
                            {"name": "Content-Type", "value": "application/json"}
                        ],
                        "content": {
                            "size": 42,
                            "mimeType": "application/json",
                            "text": "[{\"id\":1,\"name\":\"Alice\"}]"
                        }
                    },
                    "startedDateTime": "2026-03-13T00:00:00Z",
                    "time": 150
                },
                {
                    "request": {
                        "method": "POST",
                        "url": "https://api.example.com/users",
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "queryString": [],
                        "postData": {
                            "mimeType": "application/json",
                            "text": "{\"name\":\"Bob\"}"
                        }
                    },
                    "startedDateTime": "2026-03-13T00:00:01Z",
                    "time": 80
                }
            ]
        }
    }"#;

    #[test]
    fn test_import_entries() {
        let requests = import(SAMPLE_HAR).unwrap();
        assert_eq!(requests.len(), 2);
    }

    #[test]
    fn test_import_get_request() {
        let requests = import(SAMPLE_HAR).unwrap();
        let req = &requests[0];
        assert_eq!(req.method, HttpMethod::Get);
        assert!(req.url.contains("api.example.com/users"));
        assert_eq!(req.headers.len(), 2);
        assert_eq!(req.params.len(), 1);
        assert_eq!(req.params[0].key, "page");
    }

    #[test]
    fn test_import_post_body() {
        let requests = import(SAMPLE_HAR).unwrap();
        let req = &requests[1];
        assert_eq!(req.method, HttpMethod::Post);
        match &req.body {
            RequestBody::Json(raw) => assert!(raw.contains("Bob")),
            _ => panic!("Expected JSON body"),
        }
    }

    #[test]
    fn test_export_requests_only() {
        let requests = import(SAMPLE_HAR).unwrap();
        let entries: Vec<(RequestDefinition, Option<&HttpResponse>)> =
            requests.into_iter().map(|r| (r, None)).collect();
        let har_json = export(&entries).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&har_json).unwrap();
        assert_eq!(parsed["log"]["version"], "1.2");
        assert_eq!(parsed["log"]["entries"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_export_roundtrip() {
        let requests = import(SAMPLE_HAR).unwrap();
        let entries: Vec<(RequestDefinition, Option<&HttpResponse>)> =
            requests.into_iter().map(|r| (r, None)).collect();
        let har_json = export(&entries).unwrap();
        let reimported = import(&har_json).unwrap();
        assert_eq!(reimported.len(), 2);
        assert_eq!(reimported[0].method, HttpMethod::Get);
        assert_eq!(reimported[1].method, HttpMethod::Post);
    }

    #[test]
    fn test_form_urlencoded_body() {
        let har = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "test", "version": "1.0"},
                "entries": [{
                    "request": {
                        "method": "POST",
                        "url": "https://example.com/form",
                        "headers": [],
                        "queryString": [],
                        "postData": {
                            "mimeType": "application/x-www-form-urlencoded",
                            "text": "",
                            "params": [
                                {"name": "username", "value": "admin"},
                                {"name": "password", "value": "secret"}
                            ]
                        }
                    },
                    "time": 0
                }]
            }
        }"#;
        let requests = import(har).unwrap();
        match &requests[0].body {
            RequestBody::FormUrlEncoded(params) => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].key, "username");
            }
            _ => panic!("Expected form-urlencoded body"),
        }
    }
}
