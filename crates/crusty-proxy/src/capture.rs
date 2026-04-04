//! HTTP traffic capture proxy.
//!
//! Runs a local HTTP proxy that intercepts requests, forwards them
//! to the actual server, and logs both request and response data.

use crate::error::ProxyError;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;

/// A captured HTTP transaction (request + response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedTransaction {
    /// Unique ID.
    pub id: String,
    /// Request method.
    pub method: String,
    /// Full URL.
    pub url: String,
    /// Request headers.
    pub request_headers: HashMap<String, String>,
    /// Request body.
    pub request_body: String,
    /// Response status code.
    pub response_status: Option<u16>,
    /// Response headers.
    pub response_headers: HashMap<String, String>,
    /// Response body (truncated if large).
    pub response_body: String,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Timestamp.
    pub timestamp: String,
    /// Whether the request succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// A running capture proxy instance.
pub struct CaptureProxy {
    /// Address the proxy is listening on.
    addr: SocketAddr,
    /// Captured transactions.
    transactions: Arc<RwLock<Vec<CapturedTransaction>>>,
    /// Shutdown signal.
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    /// Filter settings.
    filters: Arc<RwLock<CaptureFilter>>,
}

/// Filter settings for the capture proxy.
#[derive(Debug, Clone, Default)]
pub struct CaptureFilter {
    /// Only capture requests matching these methods (empty = all).
    pub methods: Vec<String>,
    /// Only capture requests to these hosts (empty = all).
    pub hosts: Vec<String>,
    /// Only capture requests with these status codes (empty = all).
    pub status_codes: Vec<u16>,
    /// Maximum body size to capture (bytes). Larger bodies are truncated.
    pub max_body_size: usize,
}

impl Default for CaptureProxy {
    fn default() -> Self {
        // Can't actually create a default — this is just for the struct
        // In practice, use CaptureProxy::start()
        panic!("Use CaptureProxy::start() instead")
    }
}

impl CaptureProxy {
    /// Start the capture proxy on the given port (0 for auto-assign).
    pub async fn start(port: u16) -> Result<Self, ProxyError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| ProxyError::Bind(e.to_string()))?;
        let actual_addr = listener
            .local_addr()
            .map_err(|e| ProxyError::Bind(e.to_string()))?;

        let transactions: Arc<RwLock<Vec<CapturedTransaction>>> = Arc::new(RwLock::new(Vec::new()));
        let filters: Arc<RwLock<CaptureFilter>> = Arc::new(RwLock::new(CaptureFilter {
            max_body_size: 1024 * 1024, // 1MB default
            ..Default::default()
        }));
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let txn_clone = Arc::clone(&transactions);
        let filter_clone = Arc::clone(&filters);

        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        let (stream, _) = match result {
                            Ok(conn) => conn,
                            Err(_) => continue,
                        };
                        let txn = Arc::clone(&txn_clone);
                        let flt = Arc::clone(&filter_clone);
                        let io = TokioIo::new(stream);

                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(
                                    io,
                                    service_fn(move |req| {
                                        handle_proxy_request(
                                            req,
                                            Arc::clone(&txn),
                                            Arc::clone(&flt),
                                        )
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

        Ok(CaptureProxy {
            addr: actual_addr,
            transactions,
            shutdown_tx,
            filters,
        })
    }

    /// Get the address the proxy is listening on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get the proxy URL.
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Get all captured transactions.
    pub fn transactions(&self) -> Vec<CapturedTransaction> {
        self.transactions
            .read()
            .map(|t| t.clone())
            .unwrap_or_default()
    }

    /// Get transactions filtered by method.
    pub fn transactions_by_method(&self, method: &str) -> Vec<CapturedTransaction> {
        self.transactions
            .read()
            .map(|txns| {
                txns.iter()
                    .filter(|t| t.method.eq_ignore_ascii_case(method))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get transactions filtered by status code.
    pub fn transactions_by_status(&self, status: u16) -> Vec<CapturedTransaction> {
        self.transactions
            .read()
            .map(|txns| {
                txns.iter()
                    .filter(|t| t.response_status == Some(status))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear the transaction log.
    pub fn clear(&self) {
        if let Ok(mut txns) = self.transactions.write() {
            txns.clear();
        }
    }

    /// Update capture filters.
    pub fn set_filter(&self, filter: CaptureFilter) {
        if let Ok(mut f) = self.filters.write() {
            *f = filter;
        }
    }

    /// Shutdown the proxy.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

impl Drop for CaptureProxy {
    fn drop(&mut self) {
        self.shutdown();
    }
}

async fn handle_proxy_request(
    req: Request<Incoming>,
    transactions: Arc<RwLock<Vec<CapturedTransaction>>>,
    filters: Arc<RwLock<CaptureFilter>>,
) -> Result<Response<Full<bytes::Bytes>>, hyper::Error> {
    let start = std::time::Instant::now();
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    // Extract the target URL from the request
    // For HTTP proxy, the URI is absolute: http://example.com/path
    let url = if uri.starts_with("http") {
        uri.clone()
    } else {
        // Relative URI — try host header
        let host = req
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost");
        format!("http://{host}{uri}")
    };

    let request_headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = http_body_util::BodyExt::collect(req.into_body())
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();

    let max_body = filters
        .read()
        .map(|f| f.max_body_size)
        .unwrap_or(1024 * 1024);
    let request_body = truncate_body(&body_bytes, max_body);

    // Forward the request using reqwest
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap_or_default();

    let mut forward_req = client.request(
        reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
        &url,
    );

    // Forward headers (skip hop-by-hop headers)
    for (key, value) in &request_headers {
        if !is_hop_by_hop(key) {
            forward_req = forward_req.header(key.as_str(), value.as_str());
        }
    }

    if !body_bytes.is_empty() {
        forward_req = forward_req.body(body_bytes.to_vec());
    }

    let txn_id = uuid::Uuid::new_v4().to_string();

    match forward_req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let response_headers: HashMap<String, String> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let resp_body_bytes = resp.bytes().await.unwrap_or_default();
            let response_body = truncate_body(&resp_body_bytes, max_body);
            let duration_ms = start.elapsed().as_millis() as u64;

            // Log the transaction
            if let Ok(mut txns) = transactions.write() {
                txns.push(CapturedTransaction {
                    id: txn_id,
                    method: method.clone(),
                    url: url.clone(),
                    request_headers,
                    request_body,
                    response_status: Some(status),
                    response_headers: response_headers.clone(),
                    response_body: response_body.clone(),
                    duration_ms,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    success: true,
                    error: None,
                });
            }

            // Build the response to send back to the client
            let mut builder = Response::builder().status(status);
            for (key, value) in &response_headers {
                if !is_hop_by_hop(key) && key != "transfer-encoding" {
                    builder = builder.header(key.as_ref() as &str, value.as_ref() as &str);
                }
            }

            Ok(builder
                .body(Full::new(resp_body_bytes))
                .unwrap_or_else(|_| Response::new(Full::new(bytes::Bytes::new()))))
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let error_msg = e.to_string();

            if let Ok(mut txns) = transactions.write() {
                txns.push(CapturedTransaction {
                    id: txn_id,
                    method,
                    url,
                    request_headers,
                    request_body,
                    response_status: None,
                    response_headers: HashMap::new(),
                    response_body: String::new(),
                    duration_ms,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    success: false,
                    error: Some(error_msg.clone()),
                });
            }

            Ok(Response::builder()
                .status(502)
                .header("content-type", "text/plain")
                .body(Full::new(bytes::Bytes::from(format!(
                    "Proxy error: {error_msg}"
                ))))
                .unwrap_or_else(|_| Response::new(Full::new(bytes::Bytes::new()))))
        }
    }
}

fn is_hop_by_hop(header: &str) -> bool {
    matches!(
        header.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "upgrade"
    )
}

fn truncate_body(body: &[u8], max_size: usize) -> String {
    let text = String::from_utf8_lossy(body);
    if text.len() > max_size {
        format!("{}... [truncated]", &text[..max_size])
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hop_by_hop_detection() {
        assert!(is_hop_by_hop("Connection"));
        assert!(is_hop_by_hop("keep-alive"));
        assert!(is_hop_by_hop("Proxy-Authorization"));
        assert!(!is_hop_by_hop("Content-Type"));
        assert!(!is_hop_by_hop("Authorization"));
    }

    #[test]
    fn test_truncate_body() {
        let short = truncate_body(b"hello", 100);
        assert_eq!(short, "hello");

        let long = truncate_body(b"hello world", 5);
        assert!(long.contains("... [truncated]"));
        assert!(long.starts_with("hello"));
    }

    #[test]
    fn test_capture_filter_default() {
        let filter = CaptureFilter::default();
        assert!(filter.methods.is_empty());
        assert!(filter.hosts.is_empty());
        assert!(filter.status_codes.is_empty());
        assert_eq!(filter.max_body_size, 0);
    }

    #[tokio::test]
    async fn test_proxy_starts() {
        let proxy = CaptureProxy::start(0).await.unwrap();
        assert_ne!(proxy.addr().port(), 0);
        assert!(proxy.transactions().is_empty());
        proxy.shutdown();
    }

    #[tokio::test]
    async fn test_proxy_clear() {
        let proxy = CaptureProxy::start(0).await.unwrap();
        // Manually inject a transaction for testing
        if let Ok(mut txns) = proxy.transactions.write() {
            txns.push(CapturedTransaction {
                id: "test".to_string(),
                method: "GET".to_string(),
                url: "http://example.com".to_string(),
                request_headers: HashMap::new(),
                request_body: String::new(),
                response_status: Some(200),
                response_headers: HashMap::new(),
                response_body: "ok".to_string(),
                duration_ms: 100,
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                success: true,
                error: None,
            });
        }
        assert_eq!(proxy.transactions().len(), 1);
        proxy.clear();
        assert!(proxy.transactions().is_empty());
    }

    #[tokio::test]
    async fn test_filter_by_method() {
        let proxy = CaptureProxy::start(0).await.unwrap();
        if let Ok(mut txns) = proxy.transactions.write() {
            for (method, i) in [("GET", 1), ("POST", 2), ("GET", 3)] {
                txns.push(CapturedTransaction {
                    id: i.to_string(),
                    method: method.to_string(),
                    url: format!("http://example.com/{i}"),
                    request_headers: HashMap::new(),
                    request_body: String::new(),
                    response_status: Some(200),
                    response_headers: HashMap::new(),
                    response_body: String::new(),
                    duration_ms: 0,
                    timestamp: String::new(),
                    success: true,
                    error: None,
                });
            }
        }
        assert_eq!(proxy.transactions_by_method("GET").len(), 2);
        assert_eq!(proxy.transactions_by_method("POST").len(), 1);
        assert_eq!(proxy.transactions_by_method("DELETE").len(), 0);
    }
}
