//! End-to-end integration tests.
//!
//! These tests spin up a mock server and exercise the full
//! request pipeline: definition -> resolution -> HTTP execution -> assertions.

use crusty_core::collection::{Collection, Folder};
use crusty_core::request::{HttpMethod, KeyValue, RequestBody, RequestDefinition};
use crusty_http::HttpClient;
use crusty_mock::endpoint::MockEndpoint;
use crusty_mock::server::MockServer;
use crusty_testing::assertion::{Assertion, AssertionOp, AssertionTarget};
use std::collections::HashMap;

/// Helper: start a mock server with common endpoints.
async fn setup_mock() -> MockServer {
    let server = MockServer::start(0).await.unwrap();

    server.add_endpoint(MockEndpoint::new(
        "get_users",
        "GET",
        "/api/users",
        200,
        r#"[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]"#,
    ));

    server.add_endpoint(MockEndpoint::new(
        "get_user",
        "GET",
        "/api/users/1",
        200,
        r#"{"id":1,"name":"Alice","email":"alice@example.com"}"#,
    ));

    server.add_endpoint(MockEndpoint::new(
        "create_user",
        "POST",
        "/api/users",
        201,
        r#"{"id":3,"name":"Charlie"}"#,
    ));

    server.add_endpoint(MockEndpoint::new(
        "delete_user",
        "DELETE",
        "/api/users/1",
        204,
        "",
    ));

    server.add_endpoint(MockEndpoint::new(
        "health",
        "GET",
        "/health",
        200,
        r#"{"status":"ok"}"#,
    ));

    server
}

// ---------------------------------------------------------------------------
// Basic HTTP client tests against mock server
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_request_through_full_pipeline() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let def = RequestDefinition::new("Get Users", format!("{}/api/users", server.url()));
    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new()).unwrap();
    let response = client.execute(&resolved).await.unwrap();

    assert_eq!(response.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 2);
    assert_eq!(body[0]["name"], "Alice");
}

#[tokio::test]
async fn test_post_request_with_json_body() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let mut def = RequestDefinition::new("Create User", format!("{}/api/users", server.url()));
    def.method = HttpMethod::Post;
    def.body = RequestBody::Json(r#"{"name":"Charlie"}"#.to_string());

    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new()).unwrap();
    let response = client.execute(&resolved).await.unwrap();

    assert_eq!(response.status, 201);
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["name"], "Charlie");
}

#[tokio::test]
async fn test_delete_request() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let mut def = RequestDefinition::new("Delete User", format!("{}/api/users/1", server.url()));
    def.method = HttpMethod::Delete;

    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new()).unwrap();
    let response = client.execute(&resolved).await.unwrap();

    assert_eq!(response.status, 204);
}

#[tokio::test]
async fn test_request_with_headers() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let mut def = RequestDefinition::new("Get Users", format!("{}/api/users", server.url()));
    def.headers
        .push(KeyValue::new("Accept", "application/json"));
    def.headers.push(KeyValue::new("X-Request-Id", "test-123"));

    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new()).unwrap();
    let response = client.execute(&resolved).await.unwrap();

    assert_eq!(response.status, 200);

    // Verify the mock server logged the headers
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let log = server.request_log();
    assert!(!log.is_empty());
    assert_eq!(log[0].headers.get("x-request-id").unwrap(), "test-123");
}

#[tokio::test]
async fn test_request_with_query_params() {
    let server = setup_mock().await;

    server.add_endpoint(MockEndpoint::new(
        "search",
        "GET",
        "/api/search",
        200,
        r#"{"results":[]}"#,
    ));

    let mut def = RequestDefinition::new("Search", format!("{}/api/search", server.url()));
    def.params.push(KeyValue::new("q", "rust"));
    def.params.push(KeyValue::new("page", "1"));

    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new()).unwrap();

    assert!(resolved.url.as_str().contains("q=rust"));
    assert!(resolved.url.as_str().contains("page=1"));
}

#[tokio::test]
async fn test_unmatched_endpoint_returns_404() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let def = RequestDefinition::new("Missing", format!("{}/api/nonexistent", server.url()));
    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new()).unwrap();
    let response = client.execute(&resolved).await.unwrap();

    assert_eq!(response.status, 404);
}

// ---------------------------------------------------------------------------
// Variable interpolation through the pipeline
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_environment_variable_interpolation() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let mut env = crusty_core::environment::Environment::new("test");
    env.add_variable("base_url", &server.url());

    let def = RequestDefinition::new("Health", "{{base_url}}/health");
    let resolved =
        crusty_core::orchestrator::resolve_request(&def, &[&env], &HashMap::new()).unwrap();
    let response = client.execute(&resolved).await.unwrap();

    assert_eq!(response.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["status"], "ok");
}

// ---------------------------------------------------------------------------
// Assertion engine against real responses
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_assertions_against_live_response() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let def = RequestDefinition::new("Get User", format!("{}/api/users/1", server.url()));
    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new()).unwrap();
    let response = client.execute(&resolved).await.unwrap();

    let body_text = std::str::from_utf8(&response.body).unwrap();

    // Status assertion
    let status_assert = Assertion {
        target: AssertionTarget::Status,
        operator: AssertionOp::Equals,
        expected: "200".to_string(),
    };
    let result = crusty_testing::assertion::evaluate(
        &status_assert,
        response.status,
        &response.headers,
        body_text,
        response.timing.total.as_millis() as u64,
    );
    assert!(result.passed);

    // Body contains assertion
    let body_assert = Assertion {
        target: AssertionTarget::Body,
        operator: AssertionOp::Contains,
        expected: "Alice".to_string(),
    };
    let result = crusty_testing::assertion::evaluate(
        &body_assert,
        response.status,
        &response.headers,
        body_text,
        response.timing.total.as_millis() as u64,
    );
    assert!(result.passed);

    // JSON path assertion
    let json_assert = Assertion {
        target: AssertionTarget::JsonPath("email".to_string()),
        operator: AssertionOp::Equals,
        expected: "alice@example.com".to_string(),
    };
    let result = crusty_testing::assertion::evaluate(
        &json_assert,
        response.status,
        &response.headers,
        body_text,
        response.timing.total.as_millis() as u64,
    );
    assert!(result.passed);
}

// ---------------------------------------------------------------------------
// Collection runner
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_collection_runner_executes_all_requests() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let mut collection = Collection::new("API Tests");
    collection.add_request(RequestDefinition::new(
        "Health Check",
        format!("{}/health", server.url()),
    ));
    collection.add_request(RequestDefinition::new(
        "Get Users",
        format!("{}/api/users", server.url()),
    ));

    let result = crusty_testing::runner::run_collection(
        &collection,
        &client,
        &HashMap::new(),
        &HashMap::new(),
    )
    .await
    .unwrap();

    assert_eq!(result.collection_name, "API Tests");
    assert_eq!(result.request_results.len(), 2);
    assert_eq!(result.request_results[0].status, Some(200));
    assert_eq!(result.request_results[1].status, Some(200));
    assert!(result.total_duration_ms < 5000);
}

#[tokio::test]
async fn test_collection_runner_with_nested_folders() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let mut collection = Collection::new("Nested API Tests");

    let mut auth_folder = Folder::new("Auth");
    auth_folder.add_request(RequestDefinition::new(
        "Health",
        format!("{}/health", server.url()),
    ));

    let mut users_folder = Folder::new("Users");
    users_folder.add_request(RequestDefinition::new(
        "List Users",
        format!("{}/api/users", server.url()),
    ));
    users_folder.add_request(RequestDefinition::new(
        "Get User",
        format!("{}/api/users/1", server.url()),
    ));

    collection.add_folder(auth_folder);
    collection.add_folder(users_folder);

    let result = crusty_testing::runner::run_collection(
        &collection,
        &client,
        &HashMap::new(),
        &HashMap::new(),
    )
    .await
    .unwrap();

    assert_eq!(result.request_results.len(), 3);
    assert!(result.request_results.iter().all(|r| r.error.is_none()));
}

// ---------------------------------------------------------------------------
// Report generation from real run results
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_junit_report_from_collection_run() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let mut collection = Collection::new("Report Test");
    collection.add_request(RequestDefinition::new(
        "Health",
        format!("{}/health", server.url()),
    ));

    let result = crusty_testing::runner::run_collection(
        &collection,
        &client,
        &HashMap::new(),
        &HashMap::new(),
    )
    .await
    .unwrap();

    let xml = crusty_testing::report::to_junit_xml(&result);
    assert!(xml.starts_with("<?xml version=\"1.0\""));
    assert!(xml.contains("Report Test"));

    let json = crusty_testing::report::to_json(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["collection_name"], "Report Test");
}

// ---------------------------------------------------------------------------
// Timing data is populated
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_response_timing_is_populated() {
    let server = setup_mock().await;
    let client = HttpClient::new().unwrap();

    let def = RequestDefinition::new("Timed", format!("{}/health", server.url()));
    let resolved = crusty_core::orchestrator::resolve_request(&def, &[], &HashMap::new()).unwrap();
    let response = client.execute(&resolved).await.unwrap();

    assert!(!response.timing.total.is_zero());
    assert!(response.timing.ttfb.is_some());
    assert!(response.size.body_size > 0);
}
