//! HTTP engine error types.

use thiserror::Error;

/// Errors that can occur during HTTP operations.
#[derive(Debug, Error)]
pub enum HttpError {
    /// The request failed to build.
    #[error("failed to build request: {0}")]
    RequestBuild(String),

    /// A network or transport error occurred.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// The URL is invalid.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Timeout exceeded.
    #[error("request timed out after {0}ms")]
    Timeout(u64),
}

impl HttpError {
    /// Produce a human-readable message suitable for display in the UI.
    ///
    /// Unlike [`Display`](std::fmt::Display), this surfaces actionable hints
    /// for the common transport failures: DNS lookup failure, connection
    /// refused, TLS handshake errors, and timeouts.
    pub fn user_message(&self) -> String {
        match self {
            HttpError::InvalidUrl(e) => format!("Invalid URL — {e}"),
            HttpError::Timeout(ms) => {
                format!("Request timed out after {ms}ms. Check the URL or your network.")
            }
            HttpError::RequestBuild(msg) => format!("Couldn't build the request: {msg}"),
            HttpError::Network(e) => network_message(e),
        }
    }
}

fn network_message(err: &reqwest::Error) -> String {
    let chain = error_chain(err);

    if err.is_timeout() {
        return "Connection timed out. Check the URL and your network.".to_string();
    }
    // In the browser, fetch reports every network-level failure the same way and
    // hides the cause, so the likely reasons are spelled out instead.
    #[cfg(target_family = "wasm")]
    if err.is_request() {
        return format!(
            "The browser couldn't complete the request. The server may be unreachable, \
             or it doesn't allow requests from this page (CORS). ({chain})"
        );
    }
    #[cfg(not(target_family = "wasm"))]
    if err.is_connect() {
        return match classify_connect_error(&chain) {
            Some(ConnectKind::Dns) => {
                format!("DNS lookup failed. Check the hostname. ({chain})")
            }
            Some(ConnectKind::Refused) => {
                format!("Connection refused. Is the server running? ({chain})")
            }
            Some(ConnectKind::Unreachable) => format!("Network unreachable. ({chain})"),
            Some(ConnectKind::Tls) => {
                format!("TLS error — certificate or handshake failed. ({chain})")
            }
            None => format!("Couldn't connect: {chain}"),
        };
    }
    if err.is_decode() {
        return format!("Failed to decode the response body. ({chain})");
    }
    if err.is_redirect() {
        return format!("Too many redirects. ({chain})");
    }
    format!("Network error: {chain}")
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
#[derive(Debug, PartialEq, Eq)]
enum ConnectKind {
    Dns,
    Refused,
    Unreachable,
    Tls,
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
fn classify_connect_error(chain: &str) -> Option<ConnectKind> {
    let lower = chain.to_lowercase();
    // DNS variants across platforms
    if lower.contains("name resolution")
        || lower.contains("dns")
        || lower.contains("no such host")
        || lower.contains("nodename nor servname")
        || lower.contains("name or service not known")
        || lower.contains("temporary failure")
    {
        return Some(ConnectKind::Dns);
    }
    if lower.contains("connection refused") || lower.contains("refused") {
        return Some(ConnectKind::Refused);
    }
    if lower.contains("network is unreachable")
        || lower.contains("host is unreachable")
        || lower.contains("unreachable")
    {
        return Some(ConnectKind::Unreachable);
    }
    if lower.contains("certificate")
        || lower.contains("tls")
        || lower.contains("ssl")
        || lower.contains("handshake")
    {
        return Some(ConnectKind::Tls);
    }
    None
}

fn error_chain(err: &(dyn std::error::Error + 'static)) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = current {
        // Errors from the browser arrive as `JsValue(TypeError: …` followed by a
        // JS stack trace; keep just the browser's message.
        let s = browser_message(&e.to_string());
        if !s.is_empty() && !parts.iter().any(|p| p == &s) {
            parts.push(s);
        }
        current = e.source();
    }
    parts.join(" → ")
}

fn browser_message(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default().trim();
    match first.strip_prefix("JsValue(") {
        // With no stack trace, the wrapper's closing parenthesis ends the line.
        Some(inner) if !text.trim().contains('\n') => {
            inner.strip_suffix(')').unwrap_or(inner).to_string()
        }
        Some(inner) => inner.to_string(),
        None => first.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_url_user_message_is_friendly() {
        let err = HttpError::InvalidUrl(url::ParseError::EmptyHost);
        let msg = err.user_message();
        assert!(msg.starts_with("Invalid URL"), "got: {msg}");
    }

    #[test]
    fn timeout_user_message_mentions_duration() {
        let err = HttpError::Timeout(5000);
        let msg = err.user_message();
        assert!(msg.contains("5000ms"));
        assert!(msg.contains("Check the URL"));
    }

    #[test]
    fn request_build_user_message() {
        let err = HttpError::RequestBuild("bad header".to_string());
        let msg = err.user_message();
        assert!(msg.contains("bad header"));
    }

    #[test]
    fn classify_dns_failures() {
        assert_eq!(
            classify_connect_error("hyper / name resolution failed"),
            Some(ConnectKind::Dns)
        );
        assert_eq!(
            classify_connect_error("name or service not known"),
            Some(ConnectKind::Dns)
        );
        assert_eq!(
            classify_connect_error("DNS error: lookup"),
            Some(ConnectKind::Dns)
        );
    }

    #[test]
    fn classify_connection_refused() {
        assert_eq!(
            classify_connect_error("error connecting: Connection refused (os error 111)"),
            Some(ConnectKind::Refused)
        );
    }

    #[test]
    fn classify_unreachable() {
        assert_eq!(
            classify_connect_error("Network is unreachable"),
            Some(ConnectKind::Unreachable)
        );
    }

    #[test]
    fn classify_tls() {
        assert_eq!(
            classify_connect_error("invalid peer certificate"),
            Some(ConnectKind::Tls)
        );
        assert_eq!(
            classify_connect_error("TLS handshake failed"),
            Some(ConnectKind::Tls)
        );
    }

    #[test]
    fn classify_unknown_returns_none() {
        assert_eq!(classify_connect_error("something exotic"), None);
    }

    #[test]
    fn error_chain_dedupes_and_joins() {
        use std::fmt;

        #[derive(Debug)]
        struct Outer(Inner);
        #[derive(Debug)]
        struct Inner;
        impl fmt::Display for Outer {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "outer failure")
            }
        }
        impl fmt::Display for Inner {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "inner failure")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        impl std::error::Error for Inner {}

        let e = Outer(Inner);
        assert_eq!(error_chain(&e), "outer failure → inner failure");
    }

    #[test]
    fn error_chain_keeps_only_the_first_line_of_each_message() {
        #[derive(Debug)]
        struct WithStack;
        impl std::fmt::Display for WithStack {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "TypeError: Failed to fetch\n    at foo (app.js:1:2)")
            }
        }
        impl std::error::Error for WithStack {}

        assert_eq!(error_chain(&WithStack), "TypeError: Failed to fetch");
    }

    #[test]
    fn browser_message_unwraps_jsvalue() {
        assert_eq!(
            browser_message("JsValue(TypeError: Failed to fetch)"),
            "TypeError: Failed to fetch"
        );
        assert_eq!(
            browser_message(
                "JsValue(TypeError: Failed to fetch\nTypeError: x\n    at f (a.js:1:2))"
            ),
            "TypeError: Failed to fetch"
        );
        assert_eq!(
            browser_message("tcp connect error (os error 111)"),
            "tcp connect error (os error 111)"
        );
    }
}
