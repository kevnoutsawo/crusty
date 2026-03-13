//! Server-Sent Events (SSE) client for Crusty.
//!
//! Connect to an SSE endpoint and receive a stream of events.

use crate::error::ProtoError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
#[allow(unused_imports)]
use tokio::sync::{mpsc, Mutex};

/// A single SSE event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    /// Event type (from `event:` field, defaults to "message").
    pub event_type: String,
    /// Event data (from `data:` field(s), joined with newlines).
    pub data: String,
    /// Event ID (from `id:` field), if present.
    pub id: Option<String>,
    /// Retry interval in ms (from `retry:` field), if present.
    pub retry: Option<u64>,
    /// Timestamp when the event was received.
    pub timestamp: String,
}

/// SSE connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SseConnectionState {
    /// Not connected.
    Disconnected,
    /// Connected and streaming.
    Connected,
    /// Reconnecting after a disconnect.
    Reconnecting,
}

/// Events emitted by the SSE client.
#[derive(Debug)]
pub enum SseClientEvent {
    /// Connection established.
    Connected,
    /// An SSE event was received.
    Event(SseEvent),
    /// Connection lost (with optional reason).
    Disconnected(Option<String>),
    /// An error occurred.
    Error(String),
}

/// An SSE connection handle.
pub struct SseConnection {
    /// Channel to receive events.
    pub events: mpsc::Receiver<SseClientEvent>,
    /// Current connection state.
    pub state: Arc<Mutex<SseConnectionState>>,
    /// Event log.
    pub log: Arc<Mutex<Vec<SseEvent>>>,
    /// Cancel handle — drop to disconnect.
    _cancel: tokio::sync::watch::Sender<bool>,
}

/// Connect to an SSE endpoint.
///
/// Returns an `SseConnection` with an event stream.
/// The connection runs in a background task and can be cancelled
/// by dropping the `SseConnection`.
pub async fn connect(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<SseConnection, ProtoError> {
    let client = reqwest::Client::new();
    let mut request = client.get(url);
    for (key, value) in headers {
        request = request.header(key.as_str(), value.as_str());
    }
    request = request.header("Accept", "text/event-stream");

    let response = request
        .send()
        .await
        .map_err(|e| ProtoError::WebSocket(format!("SSE connection failed: {e}")))?;

    if !response.status().is_success() {
        return Err(ProtoError::WebSocket(format!(
            "SSE server returned status {}",
            response.status()
        )));
    }

    let (evt_tx, evt_rx) = mpsc::channel::<SseClientEvent>(256);
    let state = Arc::new(Mutex::new(SseConnectionState::Connected));
    let log: Arc<Mutex<Vec<SseEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);

    let _ = evt_tx.send(SseClientEvent::Connected).await;

    let state_bg = Arc::clone(&state);
    let log_bg = Arc::clone(&log);

    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut response = response;

        loop {
            tokio::select! {
                _ = cancel_rx.changed() => {
                    break;
                }
                chunk = response.chunk() => {
                    match chunk {
                        Ok(Some(bytes)) => {
                            let text = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&text);

                            // Parse complete events (separated by double newlines)
                            while let Some(pos) = buffer.find("\n\n") {
                                let event_text = buffer[..pos].to_string();
                                buffer = buffer[pos + 2..].to_string();

                                if let Some(event) = parse_sse_event(&event_text) {
                                    log_bg.lock().await.push(event.clone());
                                    let _ = evt_tx.send(SseClientEvent::Event(event)).await;
                                }
                            }
                        }
                        Ok(None) => {
                            // Stream ended
                            break;
                        }
                        Err(e) => {
                            let _ = evt_tx.send(SseClientEvent::Error(e.to_string())).await;
                            break;
                        }
                    }
                }
            }
        }

        *state_bg.lock().await = SseConnectionState::Disconnected;
        let _ = evt_tx.send(SseClientEvent::Disconnected(None)).await;
    });

    Ok(SseConnection {
        events: evt_rx,
        state,
        log,
        _cancel: cancel_tx,
    })
}

fn parse_sse_event(text: &str) -> Option<SseEvent> {
    let mut event_type = String::from("message");
    let mut data_lines: Vec<String> = Vec::new();
    let mut id = None;
    let mut retry = None;

    for line in text.lines() {
        if line.starts_with(':') {
            // Comment, skip
            continue;
        }

        let (field, value) = if let Some(colon_pos) = line.find(':') {
            let f = &line[..colon_pos];
            let v = line[colon_pos + 1..].trim_start();
            (f, v)
        } else {
            (line, "")
        };

        match field {
            "event" => event_type = value.to_string(),
            "data" => data_lines.push(value.to_string()),
            "id" => id = Some(value.to_string()),
            "retry" => retry = value.parse().ok(),
            _ => {} // Unknown fields are ignored per spec
        }
    }

    if data_lines.is_empty() && event_type == "message" {
        return None; // No data, not a real event
    }

    Some(SseEvent {
        event_type,
        data: data_lines.join("\n"),
        id,
        retry,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_event() {
        let event = parse_sse_event("data: hello world").unwrap();
        assert_eq!(event.event_type, "message");
        assert_eq!(event.data, "hello world");
    }

    #[test]
    fn test_parse_typed_event() {
        let event = parse_sse_event("event: update\ndata: {\"count\": 42}").unwrap();
        assert_eq!(event.event_type, "update");
        assert_eq!(event.data, "{\"count\": 42}");
    }

    #[test]
    fn test_parse_multiline_data() {
        let event = parse_sse_event("data: line1\ndata: line2\ndata: line3").unwrap();
        assert_eq!(event.data, "line1\nline2\nline3");
    }

    #[test]
    fn test_parse_event_with_id() {
        let event = parse_sse_event("id: 42\ndata: test").unwrap();
        assert_eq!(event.id, Some("42".to_string()));
    }

    #[test]
    fn test_parse_event_with_retry() {
        let event = parse_sse_event("retry: 3000\ndata: reconnect").unwrap();
        assert_eq!(event.retry, Some(3000));
    }

    #[test]
    fn test_parse_comment_ignored() {
        let event = parse_sse_event(": this is a comment\ndata: actual data").unwrap();
        assert_eq!(event.data, "actual data");
    }

    #[test]
    fn test_parse_empty_returns_none() {
        assert!(parse_sse_event(": just a comment").is_none());
    }
}
