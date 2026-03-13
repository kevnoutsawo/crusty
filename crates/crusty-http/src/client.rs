//! HTTP client implementation wrapping `reqwest`.

use crate::error::HttpError;
use crusty_core::request::{ResolvedBody, ResolvedRequest};
use crusty_core::response::{HttpResponse, ResponseSize, ResponseTiming};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The HTTP client that executes resolved requests.
pub struct HttpClient {
    inner: reqwest::Client,
}

impl HttpClient {
    /// Create a new HTTP client with default settings.
    pub fn new() -> Result<Self, HttpError> {
        let inner = reqwest::Client::builder()
            .build()
            .map_err(|e| HttpError::RequestBuild(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Execute a resolved request and return the response with timing data.
    pub async fn execute(&self, request: &ResolvedRequest) -> Result<HttpResponse, HttpError> {
        let method = to_reqwest_method(request.method);
        let mut builder = self.inner.request(method, request.url.as_str());

        // Apply settings
        if !request.settings.verify_ssl {
            // Note: reqwest Client-level setting; per-request requires a new client.
            // For MVP, we use the shared client. Per-request SSL config comes later.
        }

        builder = builder.timeout(Duration::from_millis(request.settings.read_timeout_ms));

        // Set headers
        for (key, value) in &request.headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        // Set body
        match &request.body {
            ResolvedBody::None => {}
            ResolvedBody::Bytes { data, content_type } => {
                builder = builder
                    .header("Content-Type", content_type.as_str())
                    .body(data.clone());
            }
        }

        // Execute with timing
        let start = Instant::now();
        let response = builder.send().await?;
        let ttfb = start.elapsed();

        let status = response.status().as_u16();
        let status_text = response
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();

        // Collect headers
        let mut headers = HashMap::new();
        let mut headers_size: u64 = 0;
        for (key, value) in response.headers() {
            let key_str = key.as_str().to_string();
            let value_str = value.to_str().unwrap_or("<binary>").to_string();
            headers_size += (key_str.len() + value_str.len() + 4) as u64; // ": " + "\r\n"
            headers.insert(key_str, value_str);
        }

        // Read body
        let body = response.bytes().await?.to_vec();
        let total = start.elapsed();
        let content_transfer = total.saturating_sub(ttfb);

        Ok(HttpResponse {
            status,
            status_text,
            headers,
            body: body.clone(),
            timing: ResponseTiming {
                total,
                dns_lookup: None,
                tcp_connect: None,
                tls_handshake: None,
                ttfb: Some(ttfb),
                content_transfer: Some(content_transfer),
            },
            size: ResponseSize {
                headers_size,
                body_size: body.len() as u64,
            },
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("failed to create default HTTP client")
    }
}

fn to_reqwest_method(method: crusty_core::request::HttpMethod) -> reqwest::Method {
    use crusty_core::request::HttpMethod;
    match method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
        HttpMethod::Trace => reqwest::Method::TRACE,
        HttpMethod::Connect => reqwest::Method::CONNECT,
    }
}
