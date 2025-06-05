// ABOUTME: This module provides an SSE-based MCP server implementation that wraps stdio transport
// ABOUTME: It bridges SSE HTTP requests to the existing stdio-based MCP server implementation

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    routing::{get, post},
    Router,
};
use futures::stream::Stream;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap, convert::Infallible, net::SocketAddr, process::Stdio, sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    process::Command as TokioCommand,
    sync::{mpsc, RwLock},
};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

#[derive(Clone)]
pub struct SseServer {
    sessions: Arc<RwLock<HashMap<String, SseSession>>>,
}

#[derive(Debug)]
struct SseSession {
    #[allow(dead_code)]
    id: String,
    tx: mpsc::Sender<Result<Event, Infallible>>,
    stdin_tx: mpsc::Sender<String>,
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
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

impl Default for SseServer {
    fn default() -> Self {
        Self::new()
    }
}

impl SseServer {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/sse", get(handle_sse_connection))
            .route("/messages", post(handle_message))
            .with_state(Arc::new(self))
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.router();

        info!("Starting SSE MCP server on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    async fn create_session(
        &self,
    ) -> Result<(String, mpsc::Receiver<Result<Event, Infallible>>), StatusCode> {
        let session_id = Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel(100);
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(100);

        // Get the current executable path
        let exe_path = std::env::current_exe().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Spawn the stdio server as a subprocess
        let mut child = TokioCommand::new(exe_path)
            .arg("--transport")
            .arg("stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        let stdout = child
            .stdout
            .take()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        // Task to read from stdout and send to SSE
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if !line.trim().is_empty() {
                            let event = Event::default().data(line.trim()).event("message");
                            let _ = tx_clone.send(Ok(event)).await;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Task to write to stdin
        tokio::spawn(async move {
            while let Some(msg) = stdin_rx.recv().await {
                let _ = stdin.write_all(msg.as_bytes()).await;
                let _ = stdin.write_all(b"\n").await;
                let _ = stdin.flush().await;
            }
        });

        let session = SseSession {
            id: session_id.clone(),
            tx,
            stdin_tx,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);

        info!("Created new SSE session: {}", session_id);
        Ok((session_id, rx))
    }

    async fn send_to_session(&self, session_id: &str, message: &str) -> Result<(), StatusCode> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            session
                .stdin_tx
                .send(message.to_string())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(())
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }
}

async fn handle_sse_connection(
    Query(_query): Query<SessionQuery>,
    State(server): State<Arc<SseServer>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let (session_id, rx) = server.create_session().await?;

    // Send initial session info
    let sessions = server.sessions.read().await;
    if let Some(session) = sessions.get(&session_id) {
        let _ = session
            .tx
            .send(Ok(Event::default()
                .data(
                    json!({
                        "session_id": session_id
                    })
                    .to_string(),
                )
                .event("session")))
            .await;
    }

    let stream = ReceiverStream::new(rx);

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30))))
}

async fn handle_message(
    Query(query): Query<SessionQuery>,
    State(server): State<Arc<SseServer>>,
    body: String,
) -> Result<StatusCode, StatusCode> {
    let session_id = query.session_id.ok_or(StatusCode::BAD_REQUEST)?;

    debug!("Received message for session {}: {}", session_id, body);

    // Forward the raw JSON-RPC message to the stdio subprocess
    server.send_to_session(&session_id, &body).await?;

    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sse_server_creation() {
        let server = SseServer::new();
        assert!(server.sessions.read().await.is_empty());
    }
}
