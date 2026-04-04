//! Postman Collection v2.1 import/export.
//!
//! Converts between Crusty collections and Postman Collection format.

use crate::ExportError;
use crusty_core::collection::{Collection, CollectionItem, Folder};
use crusty_core::request::{HttpMethod, KeyValue, RequestBody, RequestDefinition};
use serde::{Deserialize, Serialize};

// --- Postman v2.1 JSON types ---

#[derive(Debug, Serialize, Deserialize)]
struct PostmanCollection {
    info: PostmanInfo,
    #[serde(default)]
    item: Vec<PostmanItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostmanInfo {
    name: String,
    schema: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostmanItem {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request: Option<PostmanRequest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    item: Vec<PostmanItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostmanRequest {
    method: String,
    #[serde(default)]
    header: Vec<PostmanHeader>,
    url: PostmanUrl,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<PostmanBody>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum PostmanUrl {
    Simple(String),
    Detailed(PostmanUrlDetailed),
}

#[derive(Debug, Serialize, Deserialize)]
struct PostmanUrlDetailed {
    raw: String,
    #[serde(default)]
    query: Vec<PostmanQueryParam>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostmanHeader {
    key: String,
    value: String,
    #[serde(default = "default_true")]
    disabled: bool,
}

fn default_true() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize)]
struct PostmanQueryParam {
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostmanBody {
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    urlencoded: Option<Vec<PostmanQueryParam>>,
}

const POSTMAN_SCHEMA: &str = "https://schema.getpostman.com/json/collection/v2.1.0/collection.json";

// --- Import ---

/// Import a Postman Collection v2.1 JSON string into a Crusty Collection.
pub fn import(json: &str) -> Result<Collection, ExportError> {
    let pc: PostmanCollection = serde_json::from_str(json)?;

    if !pc.info.schema.contains("v2.1") && !pc.info.schema.contains("v2.0") {
        return Err(ExportError::InvalidFormat(format!(
            "Unsupported Postman schema: {}",
            pc.info.schema
        )));
    }

    let mut collection = Collection::new(&pc.info.name);
    for item in &pc.item {
        collection.items.push(convert_postman_item(item));
    }
    Ok(collection)
}

fn convert_postman_item(item: &PostmanItem) -> CollectionItem {
    if !item.item.is_empty() {
        // It's a folder
        let mut folder = Folder::new(&item.name);
        for child in &item.item {
            folder.items.push(convert_postman_item(child));
        }
        CollectionItem::Folder(folder)
    } else if let Some(ref req) = item.request {
        let mut def = RequestDefinition::new(&item.name, postman_url_raw(req));
        def.method = parse_method(&req.method);
        def.headers = req
            .header
            .iter()
            .map(|h| {
                let mut kv = KeyValue::new(&h.key, &h.value);
                kv.enabled = !h.disabled;
                kv
            })
            .collect();

        // Query params from URL
        if let PostmanUrl::Detailed(ref detail) = req.url {
            def.params = detail
                .query
                .iter()
                .map(|q| {
                    let mut kv = KeyValue::new(&q.key, &q.value);
                    kv.enabled = !q.disabled;
                    kv
                })
                .collect();
        }

        // Body
        if let Some(ref body) = req.body {
            match body.mode.as_str() {
                "raw" => {
                    if let Some(ref raw) = body.raw {
                        def.body = RequestBody::Json(raw.clone());
                    }
                }
                "urlencoded" => {
                    if let Some(ref params) = body.urlencoded {
                        def.body = RequestBody::FormUrlEncoded(
                            params
                                .iter()
                                .map(|p| KeyValue::new(&p.key, &p.value))
                                .collect(),
                        );
                    }
                }
                _ => {}
            }
        }

        CollectionItem::Request(def)
    } else {
        // Empty item — treat as empty folder
        CollectionItem::Folder(Folder::new(&item.name))
    }
}

fn postman_url_raw(req: &PostmanRequest) -> String {
    match &req.url {
        PostmanUrl::Simple(s) => s.clone(),
        PostmanUrl::Detailed(d) => d.raw.clone(),
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

// --- Export ---

/// Export a Crusty Collection to Postman Collection v2.1 JSON.
pub fn export(collection: &Collection) -> Result<String, ExportError> {
    let pc = PostmanCollection {
        info: PostmanInfo {
            name: collection.name.clone(),
            schema: POSTMAN_SCHEMA.to_string(),
        },
        item: collection.items.iter().map(export_item).collect(),
    };
    serde_json::to_string_pretty(&pc).map_err(|e| ExportError::Serialization(e.to_string()))
}

fn export_item(item: &CollectionItem) -> PostmanItem {
    match item {
        CollectionItem::Folder(folder) => PostmanItem {
            name: folder.name.clone(),
            request: None,
            item: folder.items.iter().map(export_item).collect(),
        },
        CollectionItem::Request(req) => PostmanItem {
            name: req.name.clone(),
            request: Some(export_request(req)),
            item: Vec::new(),
        },
    }
}

fn export_request(req: &RequestDefinition) -> PostmanRequest {
    PostmanRequest {
        method: req.method.as_str().to_string(),
        header: req
            .headers
            .iter()
            .map(|h| PostmanHeader {
                key: h.key.clone(),
                value: h.value.clone(),
                disabled: !h.enabled,
            })
            .collect(),
        url: PostmanUrl::Detailed(PostmanUrlDetailed {
            raw: req.url.clone(),
            query: req
                .params
                .iter()
                .map(|p| PostmanQueryParam {
                    key: p.key.clone(),
                    value: p.value.clone(),
                    disabled: !p.enabled,
                })
                .collect(),
        }),
        body: export_body(&req.body),
    }
}

fn export_body(body: &RequestBody) -> Option<PostmanBody> {
    match body {
        RequestBody::None => None,
        RequestBody::Json(raw) => Some(PostmanBody {
            mode: "raw".to_string(),
            raw: Some(raw.clone()),
            urlencoded: None,
        }),
        RequestBody::Raw { content, .. } => Some(PostmanBody {
            mode: "raw".to_string(),
            raw: Some(content.clone()),
            urlencoded: None,
        }),
        RequestBody::FormUrlEncoded(params) => Some(PostmanBody {
            mode: "urlencoded".to_string(),
            raw: None,
            urlencoded: Some(
                params
                    .iter()
                    .map(|p| PostmanQueryParam {
                        key: p.key.clone(),
                        value: p.value.clone(),
                        disabled: !p.enabled,
                    })
                    .collect(),
            ),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_POSTMAN: &str = r#"{
        "info": {
            "name": "My API",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": [
            {
                "name": "Get Users",
                "request": {
                    "method": "GET",
                    "header": [
                        {"key": "Accept", "value": "application/json"}
                    ],
                    "url": {
                        "raw": "https://api.example.com/users",
                        "query": [
                            {"key": "page", "value": "1"}
                        ]
                    }
                }
            },
            {
                "name": "Auth",
                "item": [
                    {
                        "name": "Login",
                        "request": {
                            "method": "POST",
                            "header": [],
                            "url": "https://api.example.com/login",
                            "body": {
                                "mode": "raw",
                                "raw": "{\"username\":\"admin\",\"password\":\"secret\"}"
                            }
                        }
                    }
                ]
            }
        ]
    }"#;

    #[test]
    fn test_import_basic() {
        let col = import(SAMPLE_POSTMAN).unwrap();
        assert_eq!(col.name, "My API");
        assert_eq!(col.items.len(), 2);
    }

    #[test]
    fn test_import_request() {
        let col = import(SAMPLE_POSTMAN).unwrap();
        let CollectionItem::Request(ref req) = col.items[0] else {
            panic!("Expected a request");
        };
        assert_eq!(req.name, "Get Users");
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.url, "https://api.example.com/users");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].key, "Accept");
        assert_eq!(req.params.len(), 1);
        assert_eq!(req.params[0].key, "page");
    }

    #[test]
    fn test_import_folder() {
        let col = import(SAMPLE_POSTMAN).unwrap();
        let CollectionItem::Folder(ref folder) = col.items[1] else {
            panic!("Expected a folder");
        };
        assert_eq!(folder.name, "Auth");
        assert_eq!(folder.items.len(), 1);
        let CollectionItem::Request(ref req) = folder.items[0] else {
            panic!("Expected a request");
        };
        assert_eq!(req.name, "Login");
        assert_eq!(req.method, HttpMethod::Post);
    }

    #[test]
    fn test_import_body() {
        let col = import(SAMPLE_POSTMAN).unwrap();
        let CollectionItem::Folder(ref folder) = col.items[1] else {
            panic!("Expected folder");
        };
        let CollectionItem::Request(ref req) = folder.items[0] else {
            panic!("Expected request");
        };
        match &req.body {
            RequestBody::Json(raw) => assert!(raw.contains("admin")),
            _ => panic!("Expected JSON body"),
        }
    }

    #[test]
    fn test_export_roundtrip() {
        let col = import(SAMPLE_POSTMAN).unwrap();
        let exported = export(&col).unwrap();
        let reimported = import(&exported).unwrap();
        assert_eq!(reimported.name, "My API");
        assert_eq!(reimported.items.len(), 2);
    }

    #[test]
    fn test_export_structure() {
        let col = import(SAMPLE_POSTMAN).unwrap();
        let exported = export(&col).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&exported).unwrap();
        assert!(parsed["info"]["schema"].as_str().unwrap().contains("v2.1"));
        assert_eq!(parsed["item"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_invalid_schema() {
        let json = r#"{"info": {"name": "test", "schema": "v1.0"}, "item": []}"#;
        assert!(import(json).is_err());
    }
}
