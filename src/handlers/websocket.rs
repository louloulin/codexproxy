//! WebSocket handler for Responses API
//!
//! Codex CLI 0.121.0 uses WebSocket protocol to connect to /v1/responses
//! This module implements WebSocket support by reusing the existing SSE event generation logic.

use crate::handlers::{AppState, ResponsesStreamState};
use crate::protocol::capabilities::ResponsesExecutionPlan;
use axum::{
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;

/// WebSocket handler for /v1/responses
/// Codex CLI uses GET with Upgrade: websocket header to establish WebSocket connection
pub async fn ws_responses_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    tracing::info!(route = "/v1/responses", "WebSocket upgrade request received");

    // Build the WebSocket response
    ws.on_upgrade(move |socket| {
        ws_responses_stream(socket, state)
    })
}

/// Handle the WebSocket stream after upgrade
async fn ws_responses_stream(
    socket: WebSocket,
    state: Arc<AppState>,
) {
    let (mut sender, mut receiver) = socket.split();

    tracing::info!(route = "/v1/responses", "WebSocket connection established");

    // Wait for the first message which should be the JSON request body
    let Some(Ok(Message::Text(request_text))) = receiver.next().await else {
        tracing::error!(route = "/v1/responses", "Failed to receive initial request message");
        let _ = sender.send(Message::Close(Some(axum::extract::ws::CloseFrame{
            code: axum::extract::ws::close_code::PROTOCOL,
            reason: "Expected JSON request body as first message".into(),
        }))).await;
        return;
    };

    // Parse the request
    let body: crate::models::response::ResponsesRequest = match serde_json::from_str(&request_text) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!(route = "/v1/responses", error = %e, "Failed to parse request JSON");
            let _ = sender.send(Message::Close(Some(axum::extract::ws::CloseFrame{
                code: axum::extract::ws::close_code::PROTOCOL,
                reason: format!("Invalid JSON request: {}", e).into(),
            }))).await;
            return;
        }
    };

    tracing::debug!(
        route = "/v1/responses",
        model = %body.model,
        stream = body.stream.unwrap_or(false),
        input_count = body.input.len(),
        "WebSocket request parsed"
    );

    let provider = match state.get_provider(&body.model) {
        Ok(p) => p,
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!(route = "/v1/responses", error = %error_msg, "Provider not found");
            let _ = sender.send(Message::Text(serde_json::json!({
                "type": "error",
                "error": error_msg
            }).to_string())).await;
            let _ = sender.send(Message::Close(Some(axum::extract::ws::CloseFrame{
                code: axum::extract::ws::close_code::POLICY,
                reason: error_msg.into(),
            }))).await;
            return;
        }
    };

    let stream_enabled = body.stream.unwrap_or(false);
    let _execution_plan = ResponsesExecutionPlan::ChatFallback;

    tracing::info!(
        route = "/v1/responses",
        provider = provider.name(),
        model = %body.model,
        stream = stream_enabled,
        "WebSocket responses execution planned"
    );

    // Transform Responses request to Chat request and stream via WebSocket
    let chat_request = crate::transform::transform_responses_to_chat_request(&body);

    tracing::debug!(
        route = "/v1/responses",
        transformed_model = %chat_request.model,
        "WebSocket: transformed to chat request"
    );

    match provider.chat_streaming(chat_request).await {
        Ok(streaming) => {
            tracing::info!(route = "/v1/responses", status = streaming.status(), "Streaming response received");
            if !streaming.is_success() {
                let error_msg = format!("Streaming failed with status: {}", streaming.status());
                let _ = sender.send(Message::Text(serde_json::json!({
                    "type": "error",
                    "error": error_msg
                }).to_string())).await;
                return;
            }

            // Use state machine to transform chat chunks to Responses SSE events
            let mut state_machine = ResponsesStreamState::new();

            match streaming {
                crate::providers::StreamingChat::Collected { chunks, .. } => {
                    tracing::info!(route = "/v1/responses", chunk_count = chunks.len(), "Processing collected chunks");
                    // Process all chunks and send events
                    for chunk in &chunks {
                        let events_payloads = state_machine.process_chunk(chunk);
                        tracing::debug!(route = "/v1/responses", event_count = events_payloads.len(), "Generated events from chunk");
                        for payload in events_payloads {
                            if payload == "[DONE]" {
                                if sender.send(Message::Text("[DONE]".to_string())).await.is_err() {
                                    tracing::debug!(route = "/v1/responses", "WebSocket client disconnected");
                                    return;
                                }
                            } else {
                                // Send as SSE format wrapped in WebSocket message
                                let sse_payload = format!("data: {}\n\n", payload);
                                if sender.send(Message::Text(sse_payload)).await.is_err() {
                                    tracing::debug!(route = "/v1/responses", "WebSocket client disconnected");
                                    return;
                                }
                            }
                        }
                    }
                    tracing::info!(route = "/v1/responses", "All collected chunks processed");
                }
                crate::providers::StreamingChat::Streamed { stream, .. } => {
                    tracing::info!(route = "/v1/responses", "Processing as Streamed mode");
                    // Process each chunk as it arrives
                    let mut stream = stream;
                    let mut chunk_count = 0;
                    while let Some(chunk) = stream.next().await {
                        chunk_count += 1;
                        tracing::info!(route = "/v1/responses", chunk_count = chunk_count, chunk_id = %chunk.id, "Received chunk");
                        let events_payloads = state_machine.process_chunk(&chunk);
                        tracing::info!(route = "/v1/responses", event_count = events_payloads.len(), "Generated events from chunk");
                        for payload in events_payloads {
                            if payload == "[DONE]" {
                                // Send [DONE] as plain text
                                if sender.send(Message::Text("[DONE]".to_string())).await.is_err() {
                                    tracing::warn!(route = "/v1/responses", "WebSocket client disconnected");
                                    return;
                                }
                            } else {
                                // Send as SSE format wrapped in WebSocket message
                                let sse_payload = format!("data: {}\n\n", payload);
                                if sender.send(Message::Text(sse_payload)).await.is_err() {
                                    tracing::warn!(route = "/v1/responses", "WebSocket client disconnected");
                                    return;
                                }
                            }
                        }
                    }
                    tracing::info!(route = "/v1/responses", total_chunks = chunk_count, "Stream processing complete");
                }
            }
        }
        Err(e) => {
            tracing::error!(route = "/v1/responses", error = %e, "Chat streaming failed");
            let _ = sender.send(Message::Text(serde_json::json!({
                "type": "error",
                "error": e.to_string()
            }).to_string())).await;
        }
    }

    // Send close message
    let _ = sender.send(Message::Close(Some(axum::extract::ws::CloseFrame{
        code: axum::extract::ws::close_code::NORMAL,
        reason: "Response complete".into(),
    }))).await;

    tracing::info!(route = "/v1/responses", "WebSocket connection closed");
}
