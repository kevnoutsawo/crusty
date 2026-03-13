//! Mock HTTP server.
//!
//! Runs a local HTTP server that matches incoming requests
//! against configured endpoints and returns mock responses.

use crate::endpoint::MockEndpoint;
use crate::error::MockError;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use http_body_util::Full;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;

/// A running mock server instance.
pub struct MockServer {
    /// The address the server is listening on.
    addr: SocketAddr,
    /// Configured endpoints.
    endpoints: Arc<RwLock<Vec<MockEndpoint>>>,
    /// Request log.
    request_log: Arc<RwLock<Vec<LoggedRequest>>>,
    /// Shutdown signal sender.
    shutdown_tx: tokio::sync::watch::Sender<bool>,
}

/// A logged request received by the mock server.
#[derive(Debug, Clone)]
pub struct LoggedRequest {
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Request body.
    pub body: String,
    /// Timestamp.
    pub timestamp: String,
    /// Whether it was matched to an endpoint.
    pub matched: bool,
}

impl MockServer {
    /// Start a new mock server on the given port.
    /// Pass 0 for an auto-assigned port.
    pub async fn start(port: u16) -> Result<Self, MockError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| MockError::Bind(e.to_string()))?;
        let actual_addr = listener
            .local_addr()
            .map_err(|e| MockError::Bind(e.to_string()))?;

        let endpoints: Arc<RwLock<Vec<MockEndpoint>>> = Arc::new(RwLock::new(Vec::new()));
        let request_log: Arc<RwLock<Vec<LoggedRequest>>> = Arc::new(RwLock::new(Vec::new()));
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let ep_clone = Arc::clone(&endpoints);
        let log_clone = Arc::clone(&request_log);

        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        let (stream, _) = match result {
                            Ok(conn) => conn,
                            Err(_) => continue,
                        };
                        let ep = Arc::clone(&ep_clone);
                        let log = Arc::clone(&log_clone);
                        let io = TokioIo::new(stream);

                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(
                                    io,
                                    service_fn(move |req| {
                                        handle_request(req, Arc::clone(&ep), Arc::clone(&log))
                                    }),
                                )
                                .await;
                        });
                    }
                    _ = shutdown_rx.changed() => {
                        break;
                    }
                }
            }
        });

        Ok(MockServer {
            addr: actual_addr,
            endpoints,
            request_log,
            shutdown_tx,
        })
    }

    /// Get the address the server is listening on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get the URL of the mock server.
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Add a mock endpoint.
    pub fn add_endpoint(&self, endpoint: MockEndpoint) {
        if let Ok(mut eps) = self.endpoints.write() {
            eps.push(endpoint);
            eps.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
    }

    /// Remove an endpoint by ID.
    pub fn remove_endpoint(&self, id: &str) {
        if let Ok(mut eps) = self.endpoints.write() {
            eps.retain(|e| e.id != id);
        }
    }

    /// Get all configured endpoints.
    pub fn endpoints(&self) -> Vec<MockEndpoint> {
        self.endpoints.read().map(|e| e.clone()).unwrap_or_default()
    }

    /// Get all logged requests.
    pub fn request_log(&self) -> Vec<LoggedRequest> {
        self.request_log.read().map(|l| l.clone()).unwrap_or_default()
    }

    /// Clear the request log.
    pub fn clear_log(&self) {
        if let Ok(mut log) = self.request_log.write() {
            log.clear();
        }
    }

    /// Shutdown the mock server.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

async fn handle_request(
    req: Request<Incoming>,
    endpoints: Arc<RwLock<Vec<MockEndpoint>>>,
    request_log: Arc<RwLock<Vec<LoggedRequest>>>,
) -> Result<Response<Full<bytes::Bytes>>, hyper::Error> {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = http_body_util::BodyExt::collect(req.into_body())
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    // Find matching endpoint
    let matched_endpoint = endpoints
        .read()
        .ok()
        .and_then(|eps| {
            eps.iter()
                .find(|ep| ep.matches(&method, &path, &headers, &body))
                .cloned()
        });

    // Log the request
    if let Ok(mut log) = request_log.write() {
        log.push(LoggedRequest {
            method: method.clone(),
            path: path.clone(),
            headers: headers.clone(),
            body: body.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            matched: matched_endpoint.is_some(),
        });
    }

    match matched_endpoint {
        Some(endpoint) => {
            // Apply delay
            if endpoint.response.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(endpoint.response.delay_ms))
                    .await;
            }

            let mut response = Response::builder().status(endpoint.response.status);
            for (key, value) in &endpoint.response.headers {
                response = response.header(key.as_str(), value.as_str());
            }
            Ok(response
                .body(Full::new(bytes::Bytes::from(endpoint.response.body.clone())))
                .unwrap_or_else(|_| {
                    Response::new(Full::new(bytes::Bytes::from("Internal error")))
                }))
        }
        None => {
            // No matching endpoint — return 404
            Ok(Response::builder()
                .status(404)
                .header("content-type", "application/json")
                .body(Full::new(bytes::Bytes::from(
                    serde_json::json!({
                        "error": "No matching mock endpoint",
                        "method": method,
                        "path": path
                    })
                    .to_string(),
                )))
                .unwrap_or_else(|_| Response::new(Full::new(bytes::Bytes::from("Not found")))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::MockEndpoint;

    #[tokio::test]
    async fn test_server_starts_and_responds() {
        let server = MockServer::start(0).await.unwrap();
        server.add_endpoint(MockEndpoint::new(
            "hello",
            "GET",
            "/hello",
            200,
            r#"{"message":"Hello, World!"}"#,
        ));

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/hello", server.url()))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["message"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_unmatched_returns_404() {
        let server = MockServer::start(0).await.unwrap();

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/nonexistent", server.url()))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_request_logging() {
        let server = MockServer::start(0).await.unwrap();
        server.add_endpoint(MockEndpoint::new("test", "GET", "/api", 200, "ok"));

        let client = reqwest::Client::new();
        let _ = client
            .get(format!("{}/api", server.url()))
            .send()
            .await
            .unwrap();
        let _ = client
            .get(format!("{}/missing", server.url()))
            .send()
            .await
            .unwrap();

        // Give the server a moment to log
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let log = server.request_log();
        assert_eq!(log.len(), 2);
        assert!(log[0].matched);
        assert!(!log[1].matched);
    }

    #[tokio::test]
    async fn test_multiple_endpoints_priority() {
        let server = MockServer::start(0).await.unwrap();

        let mut low = MockEndpoint::new("low", "GET", "/api", 200, "low priority");
        low.priority = 0;
        let mut high = MockEndpoint::new("high", "GET", "/api", 200, "high priority");
        high.priority = 10;

        server.add_endpoint(low);
        server.add_endpoint(high);

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/api", server.url()))
            .send()
            .await
            .unwrap();
        let body = resp.text().await.unwrap();
        assert_eq!(body, "high priority");
    }

    #[tokio::test]
    async fn test_remove_endpoint() {
        let server = MockServer::start(0).await.unwrap();
        let ep = MockEndpoint::new("removable", "GET", "/api", 200, "exists");
        let id = ep.id.clone();
        server.add_endpoint(ep);

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/api", server.url()))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        server.remove_endpoint(&id);
        let resp = client
            .get(format!("{}/api", server.url()))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);
    }
}
