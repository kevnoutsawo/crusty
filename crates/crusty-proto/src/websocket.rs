//! WebSocket client for Crusty.
//!
//! Connect to WebSocket servers, send and receive messages,
//! with a message log for debugging.

use crate::error::ProtoError;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// A WebSocket message in the conversation log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsLogEntry {
    /// Direction: sent or received.
    pub direction: WsDirection,
    /// Message content.
    pub content: String,
    /// Message type.
    pub msg_type: WsMessageType,
    /// Timestamp (ISO 8601).
    pub timestamp: String,
    /// Size in bytes.
    pub size: usize,
}

/// Direction of a WebSocket message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WsDirection {
    /// Message sent by the client.
    Sent,
    /// Message received from the server.
    Received,
}

/// Type of WebSocket message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WsMessageType {
    /// Text message.
    Text,
    /// Binary message.
    Binary,
    /// Ping frame.
    Ping,
    /// Pong frame.
    Pong,
    /// Close frame.
    Close,
}

/// Connection state of the WebSocket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsConnectionState {
    /// Not connected.
    Disconnected,
    /// Connection in progress.
    Connecting,
    /// Connected and ready.
    Connected,
    /// Connection closing.
    Closing,
}

/// Command sent to the WebSocket task.
#[derive(Debug)]
pub enum WsCommand {
    /// Send a text message.
    SendText(String),
    /// Send a binary message.
    SendBinary(Vec<u8>),
    /// Send a ping.
    Ping,
    /// Close the connection.
    Close,
}

/// Event emitted by the WebSocket task.
#[derive(Debug)]
pub enum WsEvent {
    /// Connection established.
    Connected,
    /// A message was received.
    Message(WsLogEntry),
    /// A message was sent (echoed back for logging).
    Sent(WsLogEntry),
    /// Connection closed (optional close reason).
    Disconnected(Option<String>),
    /// An error occurred.
    Error(String),
}

/// A WebSocket connection handle.
///
/// Provides channels to send commands and receive events from
/// a background WebSocket task.
pub struct WsConnection {
    /// Channel to send commands to the WebSocket task.
    pub commands: mpsc::Sender<WsCommand>,
    /// Channel to receive events from the WebSocket task.
    pub events: mpsc::Receiver<WsEvent>,
    /// Current connection state.
    pub state: Arc<Mutex<WsConnectionState>>,
    /// Message log.
    pub log: Arc<Mutex<Vec<WsLogEntry>>>,
}

/// Connect to a WebSocket server.
///
/// Returns a `WsConnection` handle with command/event channels.
/// The actual I/O runs in a background tokio task.
pub async fn connect(url: &str, headers: &[(String, String)]) -> Result<WsConnection, ProtoError> {
    let parsed_url = url::Url::parse(url)?;

    // Build the request with custom headers
    let mut request =
        tokio_tungstenite::tungstenite::http::Request::builder().uri(parsed_url.as_str());

    for (key, value) in headers {
        request = request.header(key.as_str(), value.as_str());
    }

    let request = request
        .body(())
        .map_err(|e| ProtoError::WebSocket(e.to_string()))?;

    let (ws_stream, _response) = tokio_tungstenite::connect_async(request).await?;
    let (write, read) = ws_stream.split();
    let write = Arc::new(Mutex::new(write));

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<WsCommand>(64);
    let (evt_tx, evt_rx) = mpsc::channel::<WsEvent>(256);
    let state = Arc::new(Mutex::new(WsConnectionState::Connected));
    let log: Arc<Mutex<Vec<WsLogEntry>>> = Arc::new(Mutex::new(Vec::new()));

    // Notify connected
    let _ = evt_tx.send(WsEvent::Connected).await;

    // Spawn reader task
    let evt_tx_read = evt_tx.clone();
    let log_read = Arc::clone(&log);
    let state_read = Arc::clone(&state);
    let mut read = read;
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(ws_msg) => {
                    let entry = ws_message_to_log_entry(&ws_msg, WsDirection::Received);
                    if let Some(entry) = entry {
                        log_read.lock().await.push(entry.clone());
                        let _ = evt_tx_read.send(WsEvent::Message(entry)).await;
                    }
                    if matches!(ws_msg, WsMessage::Close(_)) {
                        break;
                    }
                }
                Err(e) => {
                    let _ = evt_tx_read.send(WsEvent::Error(e.to_string())).await;
                    break;
                }
            }
        }
        *state_read.lock().await = WsConnectionState::Disconnected;
        let _ = evt_tx_read.send(WsEvent::Disconnected(None)).await;
    });

    // Spawn writer task
    let write_handle = Arc::clone(&write);
    let log_write = Arc::clone(&log);
    let state_write = Arc::clone(&state);
    let evt_tx_write = evt_tx;
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let ws_msg = match &cmd {
                WsCommand::SendText(text) => WsMessage::Text(text.clone().into()),
                WsCommand::SendBinary(data) => WsMessage::Binary(data.clone().into()),
                WsCommand::Ping => WsMessage::Ping(vec![].into()),
                WsCommand::Close => {
                    *state_write.lock().await = WsConnectionState::Closing;
                    WsMessage::Close(None)
                }
            };

            let entry = ws_message_to_log_entry(&ws_msg, WsDirection::Sent);

            let mut writer = write_handle.lock().await;
            if let Err(e) = writer.send(ws_msg).await {
                let _ = evt_tx_write.send(WsEvent::Error(e.to_string())).await;
                break;
            }

            if let Some(entry) = entry {
                log_write.lock().await.push(entry.clone());
                let _ = evt_tx_write.send(WsEvent::Sent(entry)).await;
            }

            if matches!(cmd, WsCommand::Close) {
                break;
            }
        }
    });

    Ok(WsConnection {
        commands: cmd_tx,
        events: evt_rx,
        state,
        log,
    })
}

fn ws_message_to_log_entry(msg: &WsMessage, direction: WsDirection) -> Option<WsLogEntry> {
    let now = chrono::Utc::now().to_rfc3339();
    match msg {
        WsMessage::Text(text) => Some(WsLogEntry {
            direction,
            content: text.to_string(),
            msg_type: WsMessageType::Text,
            timestamp: now,
            size: text.len(),
        }),
        WsMessage::Binary(data) => Some(WsLogEntry {
            direction,
            content: format!("<binary, {} bytes>", data.len()),
            msg_type: WsMessageType::Binary,
            timestamp: now,
            size: data.len(),
        }),
        WsMessage::Ping(data) => Some(WsLogEntry {
            direction,
            content: format!("<ping, {} bytes>", data.len()),
            msg_type: WsMessageType::Ping,
            timestamp: now,
            size: data.len(),
        }),
        WsMessage::Pong(data) => Some(WsLogEntry {
            direction,
            content: format!("<pong, {} bytes>", data.len()),
            msg_type: WsMessageType::Pong,
            timestamp: now,
            size: data.len(),
        }),
        WsMessage::Close(frame) => Some(WsLogEntry {
            direction,
            content: frame
                .as_ref()
                .map(|f| format!("Close: {} {}", f.code, f.reason))
                .unwrap_or_else(|| "Close".to_string()),
            msg_type: WsMessageType::Close,
            timestamp: now,
            size: 0,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_from_text_message() {
        let msg = WsMessage::Text("hello".to_string().into());
        let entry = ws_message_to_log_entry(&msg, WsDirection::Sent).unwrap();
        assert_eq!(entry.content, "hello");
        assert_eq!(entry.msg_type, WsMessageType::Text);
        assert_eq!(entry.direction, WsDirection::Sent);
        assert_eq!(entry.size, 5);
    }

    #[test]
    fn test_log_entry_from_binary_message() {
        let msg = WsMessage::Binary(vec![1, 2, 3].into());
        let entry = ws_message_to_log_entry(&msg, WsDirection::Received).unwrap();
        assert_eq!(entry.msg_type, WsMessageType::Binary);
        assert!(entry.content.contains("3 bytes"));
    }

    #[test]
    fn test_log_entry_from_close() {
        let msg = WsMessage::Close(None);
        let entry = ws_message_to_log_entry(&msg, WsDirection::Received).unwrap();
        assert_eq!(entry.msg_type, WsMessageType::Close);
    }

    #[test]
    fn test_ws_direction_serde() {
        let json = serde_json::to_string(&WsDirection::Sent).unwrap();
        let back: WsDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(back, WsDirection::Sent);
    }
}
