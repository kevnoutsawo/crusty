//! WebAssembly bindings that expose Crusty's core to the web client.
//!
//! Every function takes and returns plain JS values. Errors are thrown as
//! strings that are ready to show to the user.

mod dto;

use crusty_http::HttpClient;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Send a request with the core HTTP client (the browser's `fetch` underneath).
#[wasm_bindgen(js_name = sendRequest)]
pub async fn send_request(request: JsValue) -> Result<JsValue, JsValue> {
    let request: dto::WebRequest = from_js(request)?;
    let def = dto::for_sending(dto::to_definition(&request).map_err(fail)?);
    dto::check_browser_can_send(&def).map_err(fail)?;
    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new())
        .map_err(|e| fail(e.to_string()))?;
    let client = HttpClient::new().map_err(|e| fail(e.user_message()))?;
    let response = client
        .execute(&resolved)
        .await
        .map_err(|e| fail(e.user_message()))?;
    to_js(&dto::from_response(&response))
}

/// Parse a cURL command into a request.
#[wasm_bindgen(js_name = importCurl)]
pub fn import_curl(command: &str) -> Result<JsValue, JsValue> {
    let def = crusty_export::curl::import(command).map_err(|e| fail(e.to_string()))?;
    to_js(&dto::from_definition(&def))
}

/// Generate a code snippet for a request in one of [`languages`].
#[wasm_bindgen(js_name = generateCode)]
pub fn generate_code(request: JsValue, language: &str) -> Result<String, JsValue> {
    let request: dto::WebRequest = from_js(request)?;
    let def = dto::to_definition(&request).map_err(fail)?;
    let lang = dto::language_from_id(language).map_err(fail)?;
    Ok(crusty_export::codegen::generate(&def, lang))
}

/// The code generation languages, as `{ id, label }`.
#[wasm_bindgen]
pub fn languages() -> Result<JsValue, JsValue> {
    to_js(&dto::languages())
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| fail(e.to_string()))
}

fn from_js<T: DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(|e| fail(format!("Invalid request: {e}")))
}

fn fail(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}
