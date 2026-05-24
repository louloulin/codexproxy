//! Codex CLI streaming handler
//!
//! This module handles Codex CLI's event-streaming protocol where events
//! are sent before the actual request.

use crate::error::Error;
use crate::handlers::AppState;
use crate::models::response::ResponsesRequest;
use crate::models::streaming::{CodexCliStreamState, try_parse_codex_cli_event};
use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use bytes::Bytes;
use futures::stream;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Parse SSE stream body into lines
async fn parse_sse_body(body: Bytes) -> Vec<String> {
    let text = String::from_utf8_lossy(&body);
    text.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Extract actual request from Codex CLI event stream
fn extract_request_from_events(
    lines: &[String],
) -> (CodexCliStreamState, Option<ResponsesRequest>) {
    let mut state = CodexCliStreamState::new();
    let mut request_json = None;

    for line in lines {
        // Skip event: prefix lines
        if line.starts_with("event:") {
            continue;
        }

        // Try to parse as Codex CLI event
        if let Some(event) = try_parse_codex_cli_event(line) {
            state.update(&event);

            // If we see a standard request format, capture it
            if request_json.is_none() {
                // Check if this looks like a standard ResponsesRequest
                if line.contains("\"model\"") && line.contains("\"input\"") {
                    request_json = Some(line.clone());
                }
            }
        } else if request_json.is_none() {
            // Try parsing as JSON directly
            if serde_json::from_str::<serde_json::Value>(line).is_ok() {
                if line.contains("\"model\"") && line.contains("\"input\"") {
                    request_json = Some(line.clone());
                }
            }
        }
    }

    // Try to parse the request
    let request = request_json.and_then(|json| {
        serde_json::from_str::<ResponsesRequest>(&json).ok()
    });

    (state, request)
}

/// Build thread.started response event
fn build_thread_started_response(thread_id: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    format!(
        r#"{{"type":"thread.started","thread_id":"{}","timestamp":{}}}"#,
        thread_id, timestamp
    )
}

/// Build turn.started response event
fn build_turn_started_response(turn_id: &str, thread_id: Option<&str>) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Some(tid) = thread_id {
        format!(
            r#"{{"type":"turn.started","turn_id":"{}","thread_id":"{}","timestamp":{}}}"#,
            turn_id, tid, timestamp
        )
    } else {
        format!(
            r#"{{"type":"turn.started","turn_id":"{}","timestamp":{}}}"#,
            turn_id, timestamp
        )
    }
}

/// Build response.created event
fn build_response_created_event(response_id: &str, model: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    format!(
        r#"{{"type":"response.created","response":{{"id":"{}","object":"response","created_at":{},"status":"in_progress","error":null,"model":"{}","output":[],"usage":null}}}}"#,
        response_id, timestamp, model
    )
}

/// Build response.completed event
fn build_response_completed_event(
    response_id: &str,
    model: &str,
    output_text: &str,
    usage: Option<(u64, u64)>,
) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let usage_str = if let Some((input, output)) = usage {
        format!(r#""usage":{{"input_tokens":{},"output_tokens":{}}}"#, input, output)
    } else {
        r#""usage":null"#.to_string()
    };

    let escaped_text = output_text.replace('"', r#"\""#);
    let inner = format!(
        r#"{{"type":"message","index":0,"role":"assistant","content":[{{"type":"output_text","text":"{}"}}]}}"#,
        escaped_text
    );
    format!(
        r#"{{"type":"response.completed","response":{{"id":"{}","object":"response","created_at":{},"status":"completed","error":null,"model":"{}","output":[{}],{}}}}}"#,
        response_id, timestamp, model, inner, usage_str
    )
}

/// Build turn.completed response event
fn build_turn_completed_response(turn_id: &str, thread_id: &str) -> String {
    format!(
        r#"{{"type":"turn.completed","turn_id":"{}","thread_id":"{}"}}"#,
        turn_id, thread_id
    )
}

/// Codex CLI streaming handler
/// Handles the event-streaming protocol from Codex CLI
pub async fn handle_codex_cli_stream(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> impl IntoResponse {
    tracing::info!(
        length = body.len(),
        "received Codex CLI event stream"
    );

    // Parse the SSE body
    let lines = parse_sse_body(body).await;
    tracing::debug!(
        line_count = lines.len(),
        "parsed {} lines from Codex CLI stream",
        lines.len()
    );

    // Extract state and request
    let (mut cli_state, request) = extract_request_from_events(&lines);

    // Log state
    tracing::info!(
        thread_id = ?cli_state.thread_id,
        turn_id = ?cli_state.turn_id,
        has_request = request.is_some(),
        "Codex CLI stream state"
    );

    // If no request found, return error
    let Some(request) = request else {
        tracing::error!("No valid request found in Codex CLI stream");
        return Error::BadRequest("No valid request found in stream".to_string()).into_response();
    };

    // Generate IDs if not present
    if cli_state.thread_id.is_none() {
        cli_state.thread_id = Some(format!("thread_{}", uuid::Uuid::new_v4()));
    }
    if cli_state.turn_id.is_none() {
        cli_state.turn_id = Some(format!("turn_{}", uuid::Uuid::new_v4()));
    }

    let thread_id = cli_state.thread_id.clone().unwrap();
    let turn_id = cli_state.turn_id.clone().unwrap();
    let response_id = format!("resp_{}", uuid::Uuid::new_v4());
    let model = request.model.clone();

    tracing::info!(
        thread_id = %thread_id,
        turn_id = %turn_id,
        response_id = %response_id,
        model = %model,
        stream = request.stream.unwrap_or(false),
        "dispatching Codex CLI request"
    );

    // Handle streaming via chat fallback (since providers don't support native Responses streaming)
    // Set stream to false and handle response as non-streaming
    let mut chat_fallback_request = request.clone();
    chat_fallback_request.stream = Some(false);

    // Get provider
    let provider = match state.get_provider(&chat_fallback_request.model) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };

    // Use chat fallback for streaming
    if request.stream.unwrap_or(false) {
        // Get the provider's chat streaming response
        let chat_request = crate::models::chat::ChatRequest::from_responses_request(&chat_fallback_request);
        let streaming_response = match provider.chat_streaming(chat_request).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "Codex CLI chat streaming request failed");
                return Error::Provider(e.to_string()).into_response();
            }
        };

        // Collect all chunks from the streaming response
        let mut full_text = String::new();
        let mut input_tokens = 0u32;
        let mut output_tokens = 0u32;

        // Handle both collected and streamed responses
        if let Some(chunks) = streaming_response.chunks() {
            // Collected response (backward compatible)
            for chunk in chunks {
                // Extract content delta
                if let Some(choice) = chunk.choices.first() {
                    if let Some(delta) = &choice.delta {
                        if let Some(content) = &delta.content {
                            full_text.push_str(content);
                        }
                    }
                }
                // Track usage if available
                if let Some(usage) = &chunk.usage {
                    input_tokens = usage.prompt_tokens;
                    output_tokens = usage.completion_tokens;
                }
            }
        } else if let Some(stream) = streaming_response.into_stream() {
            use futures::StreamExt;
            let mut chunks = stream;
            while let Some(chunk) = chunks.next().await {
                // Extract content delta
                if let Some(choice) = chunk.choices.first() {
                    if let Some(delta) = &choice.delta {
                        if let Some(content) = &delta.content {
                            full_text.push_str(content);
                        }
                    }
                }
                // Track usage if available
                if let Some(usage) = &chunk.usage {
                    input_tokens = usage.prompt_tokens;
                    output_tokens = usage.completion_tokens;
                }
            }
        }

        // Build SSE stream with collected response
        let thread_id_clone = thread_id.clone();
        let turn_id_clone = turn_id.clone();
        let response_id_clone = response_id.clone();
        let model_clone = model.clone();

        let escaped_text = full_text.replace('"', r#"\""#);
        let item_added = format!(
            r#"{{"type":"response.output_item.added","output_index":0,"item":{{"type":"message","id":"msg_{}","role":"assistant","content":[]}}}}"#,
            response_id_clone
        );
        let text_delta = format!(
            r#"{{"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"{}"}}"#,
            escaped_text
        );
        let usage = if input_tokens > 0 || output_tokens > 0 {
            Some((input_tokens as u64, output_tokens as u64))
        } else {
            None
        };

        let stream = stream::iter(vec![
            Ok::<_, std::convert::Infallible>(Event::default().data(build_thread_started_response(&thread_id_clone))),
            Ok::<_, std::convert::Infallible>(Event::default().data(build_turn_started_response(&turn_id_clone, Some(&thread_id_clone)))),
            Ok::<_, std::convert::Infallible>(Event::default().data(build_response_created_event(&response_id_clone, &model_clone))),
            Ok::<_, std::convert::Infallible>(Event::default().data(item_added)),
            Ok::<_, std::convert::Infallible>(Event::default().data(text_delta)),
            Ok::<_, std::convert::Infallible>(Event::default().data(build_response_completed_event(&response_id_clone, &model_clone, &full_text, usage))),
            Ok::<_, std::convert::Infallible>(Event::default().data(build_turn_completed_response(&turn_id_clone, &thread_id_clone))),
            Ok::<_, std::convert::Infallible>(Event::default().data("[DONE]")),
        ]);

        return Sse::new(stream)
            .keep_alive(KeepAlive::new())
            .into_response();
    }

    // Non-streaming via chat fallback
    let chat_request = crate::models::chat::ChatRequest::from_responses_request(&chat_fallback_request);
    match provider.chat(chat_request).await {
        Ok(chat_response) => {
            // Build SSE response for Codex CLI
            let output_text = chat_response
                .choices
                .first()
                .and_then(|c| c.message.content.clone())
                .unwrap_or_else(|| "Response received".to_string());
            let usage = chat_response.usage.map(|u| (u.prompt_tokens as u64, u.completion_tokens as u64));
            let resp_id = response_id.clone();
            let model_str = model.clone();

            let stream = stream::iter(vec![
                Ok::<_, std::convert::Infallible>(Event::default().data(build_thread_started_response(&thread_id))),
                Ok::<_, std::convert::Infallible>(Event::default().data(build_turn_started_response(&turn_id, Some(&thread_id)))),
                Ok::<_, std::convert::Infallible>(Event::default().data(build_response_created_event(&resp_id, &model_str))),
                Ok::<_, std::convert::Infallible>(Event::default().data(build_response_completed_event(&resp_id, &model_str, &output_text, usage))),
                Ok::<_, std::convert::Infallible>(Event::default().data(build_turn_completed_response(&turn_id, &thread_id))),
                Ok::<_, std::convert::Infallible>(Event::default().data("[DONE]")),
            ]);

            return Sse::new(stream)
                .keep_alive(KeepAlive::new())
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "Codex CLI request failed");
            return Error::Provider(e.to_string()).into_response();
        }
    }
}

/// Extract text content from a ResponsesResponse
fn extract_text_from_response(
    response: &crate::models::response::ResponsesResponse,
) -> String {
    use crate::models::response::{ContentBlock, OutputItem};

    let mut text = String::new();

    for item in &response.output {
        match item {
            OutputItem::Message(msg) => {
                for block in &msg.content {
                    match block {
                        ContentBlock::OutputText(output) => {
                            text.push_str(&output.text);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    if text.is_empty() {
        text = "Response received".to_string();
    }

    text
}
