//! Collection test runner.
//!
//! Executes all requests in a collection sequentially,
//! running test scripts and collecting results.

use crate::error::TestError;
use crusty_core::collection::{Collection, CollectionItem};
use crusty_core::request::RequestDefinition;
use crusty_http::HttpClient;
use crusty_scripting::context::PostRequestContext;
use crusty_scripting::engine::ScriptEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Results from running an entire collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionRunResult {
    /// Collection name.
    pub collection_name: String,
    /// Results for each request.
    pub request_results: Vec<RequestRunResult>,
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
    /// Total tests run.
    pub total_tests: usize,
    /// Total tests passed.
    pub passed_tests: usize,
    /// Total tests failed.
    pub failed_tests: usize,
    /// Timestamp.
    pub timestamp: String,
}

/// Result of running a single request with its test script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRunResult {
    /// Request name.
    pub name: String,
    /// Request URL.
    pub url: String,
    /// HTTP method.
    pub method: String,
    /// Response status code (None if request failed).
    pub status: Option<u16>,
    /// Response time in milliseconds.
    pub duration_ms: u64,
    /// Test results from the script.
    pub tests: Vec<TestResultEntry>,
    /// Error if the request failed.
    pub error: Option<String>,
    /// Log messages from the script.
    pub logs: Vec<String>,
}

/// A test result entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultEntry {
    /// Test name.
    pub name: String,
    /// Whether it passed.
    pub passed: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Run all requests in a collection with their test scripts.
pub async fn run_collection(
    collection: &Collection,
    http_client: &HttpClient,
    variables: &HashMap<String, String>,
    test_scripts: &HashMap<String, String>,
) -> Result<CollectionRunResult, TestError> {
    let start = std::time::Instant::now();
    let mut results = Vec::new();
    let mut current_vars = variables.clone();

    // Flatten requests from collection
    let requests = flatten_requests(&collection.items);

    let engine = ScriptEngine::new();

    for def in &requests {
        let result =
            run_single_request(def, http_client, &engine, &current_vars, test_scripts).await;

        // Update variables from script output
        if let Ok(ref r) = result {
            for (k, v) in &r.updated_vars {
                current_vars.insert(k.clone(), v.clone());
            }
        }

        match result {
            Ok(r) => results.push(r.result),
            Err(e) => {
                results.push(RequestRunResult {
                    name: def.name.clone(),
                    url: def.url.clone(),
                    method: def.method.as_str().to_string(),
                    status: None,
                    duration_ms: 0,
                    tests: Vec::new(),
                    error: Some(e.to_string()),
                    logs: Vec::new(),
                });
            }
        }
    }

    let total_tests: usize = results.iter().map(|r| r.tests.len()).sum();
    let passed_tests: usize = results
        .iter()
        .flat_map(|r| &r.tests)
        .filter(|t| t.passed)
        .count();

    Ok(CollectionRunResult {
        collection_name: collection.name.clone(),
        request_results: results,
        total_duration_ms: start.elapsed().as_millis() as u64,
        total_tests,
        passed_tests,
        failed_tests: total_tests - passed_tests,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

struct SingleRunOutput {
    result: RequestRunResult,
    updated_vars: HashMap<String, String>,
}

async fn run_single_request(
    def: &RequestDefinition,
    http_client: &HttpClient,
    engine: &ScriptEngine,
    variables: &HashMap<String, String>,
    test_scripts: &HashMap<String, String>,
) -> Result<SingleRunOutput, TestError> {
    // Resolve the request
    let resolved = crusty_core::orchestrator::resolve_request(def, &[], &HashMap::new())
        .map_err(|e| TestError::RequestFailed(e.to_string()))?;

    // Execute
    let response = http_client
        .execute(&resolved)
        .await
        .map_err(|e| TestError::RequestFailed(e.to_string()))?;

    let mut updated_vars = variables.clone();
    let mut test_entries = Vec::new();
    let mut logs = Vec::new();

    // Run test script if one exists for this request
    let request_id = def.id.to_string();
    if let Some(script) = test_scripts
        .get(&request_id)
        .or_else(|| test_scripts.get(&def.name))
    {
        let body_text = response
            .body_text()
            .map(|s| s.to_string())
            .unwrap_or_default();

        let ctx = PostRequestContext {
            url: def.url.clone(),
            method: def.method.as_str().to_string(),
            status: response.status,
            status_text: response.status_text.clone(),
            response_headers: response.headers.clone(),
            response_body: body_text,
            response_time_ms: response.timing.total.as_millis() as u64,
            variables: variables.clone(),
        };

        let script_result = engine.run_post_request(script, &ctx)?;

        updated_vars = script_result.variables;
        logs = script_result.logs;
        test_entries = script_result
            .tests
            .into_iter()
            .map(|t| TestResultEntry {
                name: t.name,
                passed: t.passed,
                error: t.error,
            })
            .collect();
    }

    Ok(SingleRunOutput {
        result: RequestRunResult {
            name: def.name.clone(),
            url: def.url.clone(),
            method: def.method.as_str().to_string(),
            status: Some(response.status),
            duration_ms: response.timing.total.as_millis() as u64,
            tests: test_entries,
            error: None,
            logs,
        },
        updated_vars,
    })
}

fn flatten_requests(items: &[CollectionItem]) -> Vec<&RequestDefinition> {
    let mut requests = Vec::new();
    for item in items {
        match item {
            CollectionItem::Request(req) => requests.push(req),
            CollectionItem::Folder(folder) => {
                requests.extend(flatten_requests(&folder.items));
            }
        }
    }
    requests
}
