// ABOUTME: This module implements Server-Sent Events (SSE) transport for MCP protocol communication
// ABOUTME: It provides HTTP endpoints for bidirectional JSON-RPC message exchange using axum

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::Stream;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SseTransport {
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
}

#[derive(Debug)]
struct SessionState {
    tx: mpsc::Sender<Result<Event, Infallible>>,
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcMessage {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl Default for SseTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl SseTransport {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/sse", get(handle_sse))
            .route("/messages", post(handle_messages))
            .with_state(self)
    }

    async fn create_session(&self) -> (String, mpsc::Receiver<Result<Event, Infallible>>) {
        let session_id = Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel(100);

        let session = SessionState { tx };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);

        info!("Created new SSE session: {}", session_id);
        (session_id, rx)
    }

    async fn send_to_session(&self, session_id: &str, message: Value) -> Result<(), StatusCode> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let event = Event::default().data(message.to_string()).event("message");
            session
                .tx
                .send(Ok(event))
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(())
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }

    async fn remove_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
        info!("Removed SSE session: {}", session_id);
    }
}

async fn handle_sse(
    Query(query): Query<SessionQuery>,
    State(transport): State<SseTransport>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let (session_id, rx) = if let Some(id) = query.session_id {
        // Check if session exists
        if transport.sessions.read().await.contains_key(&id) {
            return Err(StatusCode::BAD_REQUEST); // Session already exists
        }
        transport.create_session().await
    } else {
        transport.create_session().await
    };

    // Send initial session info
    let _ = transport
        .send_to_session(
            &session_id,
            json!({
                "session_id": session_id
            }),
        )
        .await;

    let transport_clone = transport.clone();
    let _session_id_clone = session_id.clone();

    let stream = SseStream {
        session_id: session_id.clone(),
        receiver: rx,
        transport: transport_clone,
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30))))
}

struct SseStream {
    session_id: String,
    receiver: mpsc::Receiver<Result<Event, Infallible>>,
    transport: SseTransport,
}

impl Stream for SseStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(event)) => {
                debug!("Sending SSE event for session {}", self.session_id);
                Poll::Ready(Some(event))
            }
            Poll::Ready(None) => {
                info!("SSE stream closed for session {}", self.session_id);
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for SseStream {
    fn drop(&mut self) {
        let transport = self.transport.clone();
        let session_id = self.session_id.clone();
        tokio::spawn(async move {
            transport.remove_session(&session_id).await;
        });
    }
}

async fn handle_messages(
    Query(query): Query<SessionQuery>,
    State(transport): State<SseTransport>,
    Json(message): Json<JsonRpcMessage>,
) -> Result<StatusCode, StatusCode> {
    let session_id = query.session_id.ok_or(StatusCode::BAD_REQUEST)?;

    debug!("Received message for session {}: {:?}", session_id, message);

    // For now, just echo the message back
    transport
        .send_to_session(&session_id, serde_json::to_value(message).unwrap())
        .await?;

    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let transport = SseTransport::new();
        let (session_id, _rx) = transport.create_session().await;
        assert!(!session_id.is_empty());

        let sessions = transport.sessions.read().await;
        assert!(sessions.contains_key(&session_id));
    }

    #[tokio::test]
    async fn test_session_removal() {
        let transport = SseTransport::new();
        let (session_id, _rx) = transport.create_session().await;

        transport.remove_session(&session_id).await;

        let sessions = transport.sessions.read().await;
        assert!(!sessions.contains_key(&session_id));
    }
}
