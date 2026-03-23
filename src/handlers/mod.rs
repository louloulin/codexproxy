//! API handlers for Chat Completions and Responses endpoints
//!
//! This module implements the actual API endpoints with:
//! - Provider integration
//! - Transform layer for API conversion
//! - Streaming support

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use futures::stream;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::error::Error;
use crate::models::chat::{ChatCompletionChunk, ChatRequest};
use crate::models::response::{ContentBlock, MessageOutput, OutputText, ResponsesRequest, ResponsesStreamChunk};
use crate::providers::{format_request_body_for_log, LLMProvider, OpenAIProvider, ZhipuProvider};
use crate::transform;

// Re-export provider type for internal use
type ProviderType = Arc<dyn LLMProvider>;

fn rewrite_chat_request_model_for_provider(
    state: &AppState,
    provider_name: &str,
    request: &mut ChatRequest,
) -> Option<String> {
    if provider_name != "zhipu" || request.model.starts_with("glm-") {
        return None;
    }

    let original_model = request.model.clone();
    request.model = state.config.providers.zhipu.default_model.clone();
    Some(original_model)
}

fn should_emit_responses_stream_chunk(chunk: &ResponsesStreamChunk) -> bool {
    !chunk.output.is_empty() || chunk.usage.is_some()
}

fn responses_usage_from_chat_usage(usage: &crate::models::chat::Usage) -> crate::models::response::Usage {
    crate::models::response::Usage {
        input_tokens: u64::from(usage.prompt_tokens),
        input_tokens_details: None,
        output_tokens: u64::from(usage.completion_tokens),
        output_tokens_details: None,
        total_tokens: u64::from(usage.total_tokens),
    }
}

fn message_output_for_stream(
    id: String,
    index: u32,
    role: String,
    text: String,
    status: Option<String>,
) -> crate::models::response::OutputItem {
    crate::models::response::OutputItem::Message(MessageOutput {
        index,
        id: Some(id),
        role,
        content: vec![ContentBlock::OutputText(OutputText {
            text,
            annotations: None,
        })],
        status,
        end_turn: None,
        phase: None,
        tool_calls: None,
    })
}

fn response_snapshot_json(
    id: &str,
    created_at: u64,
    model: &str,
    status: &str,
    output: Vec<crate::models::response::OutputItem>,
    usage: Option<crate::models::response::Usage>,
) -> serde_json::Value {
    json!({
        "id": id,
        "object": "response",
        "created_at": created_at,
        "status": status,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "max_output_tokens": null,
        "model": model,
        "output": output,
        "parallel_tool_calls": true,
        "previous_response_id": null,
        "reasoning": {
            "effort": null,
            "summary": null
        },
        "store": false,
        "temperature": null,
        "text": {
            "format": {
                "type": "text"
            }
        },
        "tool_choice": "auto",
        "tools": [],
        "top_p": null,
        "truncation": "disabled",
        "usage": usage,
        "user": null,
        "metadata": {}
    })
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn responses_protocol_payloads_from_chat_chunks(chunks: &[ChatCompletionChunk]) -> Vec<String> {
    tracing::info!(
        chunk_count = chunks.len(),
        "responses_protocol_payloads_from_chat_chunks called"
    );

    if chunks.is_empty() {
        tracing::warn!("Empty chunks, sending minimal response.completed");
        let completed_event = json!({
            "type": "response.completed",
            "response": response_snapshot_json(
                "resp_empty",
                current_unix_timestamp(),
                "unknown",
                "completed",
                vec![],
                None
            )
        });
        let completed_str = serde_json::to_string(&completed_event)
            .expect("Failed to serialize response.completed");

        return vec![completed_str, "[DONE]".to_string()];
    }

    let first = &chunks[0];
    tracing::info!(
        first_chunk_id = %first.id,
        first_chunk_model = %first.model,
        first_chunk_choices = first.choices.len(),
        "processing chunks starting from first"
    );
    let mut payloads = vec![
        serde_json::to_string(&json!({
            "type": "response.created",
            "response": response_snapshot_json(
                &first.id,
                first.created,
                &first.model,
                "in_progress",
                vec![],
                None
            )
        }))
        .expect("Failed to serialize response.created event"),
        serde_json::to_string(&json!({
            "type": "response.in_progress",
            "response": response_snapshot_json(
                &first.id,
                first.created,
                &first.model,
                "in_progress",
                vec![],
                None
            )
        }))
        .expect("Failed to serialize response.in_progress event"),
    ];

    let mut text_by_index: HashMap<u32, String> = HashMap::new();
    let mut role_by_index: HashMap<u32, String> = HashMap::new();
    let mut item_id_by_index: HashMap<u32, String> = HashMap::new();
    let mut added_indices: HashMap<u32, bool> = HashMap::new();
    let mut done_indices: HashMap<u32, bool> = HashMap::new();
    let mut final_usage: Option<crate::models::response::Usage> = None;
    let mut final_output: Vec<crate::models::response::OutputItem> = Vec::new();

    for chunk in chunks {
        if let Some(usage) = &chunk.usage {
            final_usage = Some(responses_usage_from_chat_usage(usage));
        }

        for choice in &chunk.choices {
            let Some(delta) = &choice.delta else {
                continue;
            };

            let role = delta
                .role
                .clone()
                .or_else(|| role_by_index.get(&choice.index).cloned())
                .unwrap_or_else(|| "assistant".to_string());
            role_by_index.insert(choice.index, role.clone());
            let item_id = item_id_by_index
                .entry(choice.index)
                .or_insert_with(|| format!("msg_{}_{}", first.id, choice.index))
                .clone();

            if let Some(content) = &delta.content {
                if !added_indices.get(&choice.index).copied().unwrap_or(false) {
                    payloads.push(
                        serde_json::to_string(&json!({
                            "type": "response.output_item.added",
                            "output_index": choice.index,
                            "item": message_output_for_stream(
                                item_id.clone(),
                                choice.index,
                                role.clone(),
                                String::new(),
                                Some("in_progress".to_string()),
                            )
                        }))
                        .expect("Failed to serialize response.output_item.added event"),
                    );
                    payloads.push(
                        serde_json::to_string(&json!({
                            "type": "response.content_part.added",
                            "item_id": item_id,
                            "output_index": choice.index,
                            "content_index": 0,
                            "part": {
                                "type": "output_text",
                                "text": "",
                                "annotations": []
                            }
                        }))
                        .expect("Failed to serialize response.content_part.added event"),
                    );
                    added_indices.insert(choice.index, true);
                }

                text_by_index
                    .entry(choice.index)
                    .and_modify(|text| text.push_str(content))
                    .or_insert_with(|| content.clone());

                payloads.push(
                    serde_json::to_string(&json!({
                        "type": "response.output_text.delta",
                        "item_id": item_id_by_index.get(&choice.index).cloned().unwrap_or_default(),
                        "output_index": choice.index,
                        "content_index": 0,
                        "delta": content.clone(),
                    }))
                    .expect("Failed to serialize response.output_text.delta event"),
                );
            }

            if choice.finish_reason.is_some() && !done_indices.get(&choice.index).copied().unwrap_or(false) {
                let full_text = text_by_index.get(&choice.index).cloned().unwrap_or_default();
                if !added_indices.get(&choice.index).copied().unwrap_or(false) {
                    payloads.push(
                        serde_json::to_string(&json!({
                            "type": "response.output_item.added",
                            "output_index": choice.index,
                            "item": message_output_for_stream(
                                item_id.clone(),
                                choice.index,
                                role.clone(),
                                String::new(),
                                Some("in_progress".to_string()),
                            )
                        }))
                        .expect("Failed to serialize response.output_item.added event"),
                    );
                    payloads.push(
                        serde_json::to_string(&json!({
                            "type": "response.content_part.added",
                            "item_id": item_id,
                            "output_index": choice.index,
                            "content_index": 0,
                            "part": {
                                "type": "output_text",
                                "text": "",
                                "annotations": []
                            }
                        }))
                        .expect("Failed to serialize response.content_part.added event"),
                    );
                    added_indices.insert(choice.index, true);
                }
                let final_item = message_output_for_stream(
                    item_id_by_index.get(&choice.index).cloned().unwrap_or_default(),
                    choice.index,
                    role.clone(),
                    full_text.clone(),
                    Some("completed".to_string()),
                );
                payloads.push(
                    serde_json::to_string(&json!({
                        "type": "response.output_text.done",
                        "item_id": item_id_by_index.get(&choice.index).cloned().unwrap_or_default(),
                        "output_index": choice.index,
                        "content_index": 0,
                        "text": full_text.clone(),
                    }))
                    .expect("Failed to serialize response.output_text.done event"),
                );
                payloads.push(
                    serde_json::to_string(&json!({
                        "type": "response.content_part.done",
                        "item_id": item_id_by_index.get(&choice.index).cloned().unwrap_or_default(),
                        "output_index": choice.index,
                        "content_index": 0,
                        "part": {
                            "type": "output_text",
                            "text": full_text.clone(),
                            "annotations": []
                        }
                    }))
                    .expect("Failed to serialize response.content_part.done event"),
                );
                payloads.push(
                    serde_json::to_string(&json!({
                        "type": "response.output_item.done",
                        "output_index": choice.index,
                        "item": final_item.clone(),
                    }))
                    .expect("Failed to serialize response.output_item.done event"),
                );
                final_output.push(final_item);
                done_indices.insert(choice.index, true);
            }
        }
    }

    tracing::info!(
        payload_count = payloads.len(),
        final_usage = ?final_usage,
        "generated payloads, adding response.completed"
    );

    let completed_event = json!({
        "type": "response.completed",
        "response": response_snapshot_json(
            &first.id,
            first.created,
            &first.model,
            "completed",
            final_output,
            final_usage
        )
    });

    let completed_str = serde_json::to_string(&completed_event)
        .expect("Failed to serialize response.completed event");

    payloads.push(completed_str);
    payloads.push("[DONE]".to_string());

    tracing::info!(
        total_payloads = payloads.len(),
        "responses_protocol_payloads_from_chat_chunks complete"
    );

    payloads
}

fn responses_stream_events_from_chat_chunks(chunks: &[ChatCompletionChunk]) -> Vec<Event> {
    let payloads = responses_protocol_payloads_from_chat_chunks(chunks);

    tracing::info!(
        chunk_count = chunks.len(),
        payload_count = payloads.len(),
        "converting payloads to events"
    );

    let events: Vec<Event> = payloads
        .into_iter()
        .filter(|payload| {
            if payload.is_empty() {
                tracing::warn!("Skipping empty payload");
                false
            } else {
                true
            }
        })
        .map(|payload| Event::default().data(payload))
        .collect();

    tracing::info!(
        event_count = events.len(),
        "generated events (after filtering empty payloads)"
    );

    // 最后的保险：确保至少有 [DONE]
    if events.is_empty() {
        tracing::error!("No events generated! Adding fallback [DONE]");
        return vec![Event::default().data("[DONE]")];
    }

    events
}

fn responses_stream_event_from_chat_chunk(chunk: &ChatCompletionChunk) -> Option<Event> {
    let responses_chunk = transform::transform_chat_stream_to_responses_stream(chunk);
    if !should_emit_responses_stream_chunk(&responses_chunk) {
        return None;
    }

    Some(Event::default().data(serde_json::to_string(&responses_chunk).unwrap_or_default()))
}

fn chat_stream_event_from_chat_chunk(chunk: &ChatCompletionChunk) -> Option<Event> {
    let responses_chunk = transform::transform_chat_stream_to_responses_stream(chunk);
    if !should_emit_responses_stream_chunk(&responses_chunk) {
        return None;
    }

    let chat_chunk = transform::transform_responses_stream_to_chat_stream(&responses_chunk);
    Some(Event::default().data(serde_json::to_string(&chat_chunk).unwrap_or_default()))
}

/// Application state containing providers
pub struct AppState {
    pub config: Config,
    pub openai_provider: Option<ProviderType>,
    pub zhipu_provider: Option<ProviderType>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let openai_provider = config
            .providers
            .openai
            .as_ref()
            .filter(|provider| !provider.api_key.is_empty())
            .map(|provider| Arc::new(OpenAIProvider::new(provider.clone())) as ProviderType);

        let zhipu_provider = if !config.providers.zhipu.api_key.is_empty() {
            Some(Arc::new(ZhipuProvider::new(config.providers.zhipu.clone())) as ProviderType)
        } else {
            None
        };

        Self {
            config,
            openai_provider,
            zhipu_provider,
        }
    }

    /// Get provider based on model name
    pub fn get_provider(&self, model: &str) -> Result<ProviderType, Error> {
        // Check model mapping in routing config
        if let Some(ref mapping) = self.config.routing.model_mapping {
            if let Some(provider_name) = mapping.get(model) {
                return self.get_provider_by_name(provider_name);
            }
        }

        // Use default provider
        self.get_provider_by_name(&self.config.routing.default)
    }

    /// Get provider by name
    fn get_provider_by_name(&self, name: &str) -> Result<ProviderType, Error> {
        match name {
            "openai" => self
                .openai_provider
                .clone()
                .ok_or_else(|| Error::Provider("OpenAI provider not configured".to_string())),
            "zhipu" => self
                .zhipu_provider
                .clone()
                .ok_or_else(|| Error::Provider("Zhipu provider not configured".to_string())),
            _ => Err(Error::Provider(format!("Unknown provider: {}", name))),
        }
    }
}

/// Chat Completions handler
/// POST /v1/chat/completions
///
/// This endpoint accepts Chat Completions format and passes it to the provider.
/// The transform layer is available for future use when calling Responses API.
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatRequest>,
) -> impl IntoResponse {
    let requested_model = body.model.clone();

    let provider = match state.get_provider(&body.model) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };

    let provider_name = provider.name().to_string();
    let mut body = body;
    let rewritten_from = rewrite_chat_request_model_for_provider(&state, &provider_name, &mut body);
    let effective_model = body.model.clone();
    let stream = body.stream.unwrap_or(false);
    let message_count = body.messages.len();
    let tool_count = body.tools.as_ref().map(|tools| tools.len()).unwrap_or(0);

    tracing::info!(
        route = "/v1/chat/completions",
        provider = provider_name,
        model = %requested_model,
        effective_model = %effective_model,
        rewritten_from = rewritten_from.as_deref().unwrap_or(""),
        stream,
        message_count,
        tool_count,
        "dispatching chat completion request"
    );

    // For now, pass through to Chat Completions API
    // The transform layer (transform_chat_to_responses_request) is available
    // for future implementation when we want to call Responses API
    if stream {
        // Handle streaming
        match provider.chat_streaming(body).await {
            Ok(streaming) => {
                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status
                    ))
                    .into_response();
                }

                // Transform streaming chunks to Responses format to demonstrate conversion
                let stream = stream::iter(
                    streaming
                        .chunks
                        .into_iter()
                        .filter_map(|chunk| chat_stream_event_from_chat_chunk(&chunk))
                        .map(Ok::<_, std::convert::Infallible>),
                );

                Sse::new(stream).into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/chat/completions",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "provider chat streaming request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    } else {
        // Handle non-streaming
        match provider.chat(body).await {
            Ok(chat_response) => {
                // Transform to Responses format and back to demonstrate conversion capability
                let responses_response =
                    transform::transform_chat_to_responses_response(&chat_response);
                let chat_response =
                    transform::transform_responses_to_chat_response(&responses_response);
                Json(chat_response).into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/chat/completions",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "provider chat request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    }
}

/// Responses API handler
/// POST /v1/responses
pub async fn responses(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResponsesRequest>,
) -> impl IntoResponse {
    // 请求体调试日志：记录模型与序列化后大小
    if let Ok(body_str) = serde_json::to_string(&body) {
        tracing::debug!(
            model = %body.model,
            stream = %body.stream.unwrap_or(false),
            input_count = body.input.len(),
            body_len = body_str.len(),
            request_body = %format_request_body_for_log(&body_str),
            "responses request body"
        );
        // Log each input item's type for debugging
        for (i, item) in body.input.iter().enumerate() {
            tracing::debug!(
                index = i,
                item = ?item,
                "input item"
            );
        }
    } else {
        tracing::debug!(
            model = %body.model,
            stream = %body.stream.unwrap_or(false),
            "responses request body (serialize failed)"
        );
    }

    let provider = match state.get_provider(&body.model) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let provider_name = provider.name().to_string();

    tracing::info!(
        route = "/v1/responses",
        provider = provider_name,
        model = %body.model,
        stream = body.stream.unwrap_or(false),
        input_count = body.input.len(),
        tool_count = body.tools.len(),
        instructions = body.instructions.is_some(),
        "dispatching responses request"
    );

    let stream = body.stream.unwrap_or(false);

    if stream {
        // Handle streaming - convert to chat format, call provider, convert back
        let mut chat_request = transform::transform_responses_to_chat_request(&body);
        let rewritten_from =
            rewrite_chat_request_model_for_provider(&state, provider.name(), &mut chat_request);
        let effective_model = chat_request.model.clone();
        let transformed_body = serde_json::to_string(&chat_request).ok();
        tracing::debug!(
            route = "/v1/responses",
            provider = provider.name(),
            transformed_model = %effective_model,
            rewritten_from = rewritten_from.as_deref().unwrap_or(""),
            transformed_message_count = chat_request.messages.len(),
            transformed_tool_count = chat_request.tools.as_ref().map(|tools| tools.len()).unwrap_or(0),
            transformed_request_body = transformed_body
                .as_deref()
                .map(format_request_body_for_log)
                .unwrap_or_else(|| "<serialize failed>".to_string()),
            "responses request transformed to chat request"
        );

        match provider.chat_streaming(chat_request).await {
            Ok(streaming) => {
                tracing::info!(
                    route = "/v1/responses",
                    provider = provider.name(),
                    chunk_count = streaming.chunks.len(),
                    status = streaming.status,
                    "provider streaming response received"
                );

                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status
                    ))
                    .into_response();
                }

                // Log chunk details
                for (i, chunk) in streaming.chunks.iter().enumerate() {
                    tracing::debug!(
                        route = "/v1/responses",
                        chunk_index = i,
                        chunk_id = %chunk.id,
                        choices_count = chunk.choices.len(),
                        has_usage = chunk.usage.is_some(),
                        "streaming chunk detail"
                    );
                }

                let events = responses_stream_events_from_chat_chunks(&streaming.chunks);
                tracing::info!(
                    route = "/v1/responses",
                    event_count = events.len(),
                    chunk_count = streaming.chunks.len(),
                    "generated SSE events from chunks"
                );

                // 保险检查：确保至少有 [DONE]
                if events.is_empty() {
                    tracing::error!(
                        route = "/v1/responses",
                        chunk_count = streaming.chunks.len(),
                        "CRITICAL: No events generated from chunks, using fallback"
                    );
                    let fallback_events = vec![
                        Event::default().data(json!({
                            "type": "response.completed",
                            "response": response_snapshot_json(
                                "resp_fallback",
                                current_unix_timestamp(),
                                "unknown",
                                "completed",
                                vec![],
                                None
                            )
                        }).to_string()),
                        Event::default().data("[DONE]"),
                    ];

                    let stream = stream::iter(
                        fallback_events.into_iter().map(Ok::<_, std::convert::Infallible>),
                    );

                    // 使用 KeepAlive 防止连接提前关闭
                    return Sse::new(stream)
                        .keep_alive(KeepAlive::new())
                        .into_response();
                }

                // 记录每个事件的内容（前100字符）
                for (i, event) in events.iter().enumerate() {
                    tracing::debug!(
                        route = "/v1/responses",
                        event_index = i,
                        event_preview = ?event,
                        "SSE event detail"
                    );
                }

                let stream = stream::iter(
                    events
                        .into_iter()
                        .map(Ok::<_, std::convert::Infallible>),
                );

                // 添加 KeepAlive 以防止连接提前关闭
                Sse::new(stream)
                    .keep_alive(KeepAlive::new())
                    .into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/responses",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "provider responses streaming request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    } else {
        // Handle non-streaming
        let mut chat_request = transform::transform_responses_to_chat_request(&body);
        let rewritten_from =
            rewrite_chat_request_model_for_provider(&state, provider.name(), &mut chat_request);
        let effective_model = chat_request.model.clone();
        let transformed_body = serde_json::to_string(&chat_request).ok();
        tracing::debug!(
            route = "/v1/responses",
            provider = provider.name(),
            transformed_model = %effective_model,
            rewritten_from = rewritten_from.as_deref().unwrap_or(""),
            transformed_message_count = chat_request.messages.len(),
            transformed_tool_count = chat_request.tools.as_ref().map(|tools| tools.len()).unwrap_or(0),
            transformed_request_body = transformed_body
                .as_deref()
                .map(format_request_body_for_log)
                .unwrap_or_else(|| "<serialize failed>".to_string()),
            "responses request transformed to chat request"
        );

        match provider.chat(chat_request).await {
            Ok(chat_response) => {
                let responses_response =
                    transform::transform_chat_to_responses_response(&chat_response);
                // 响应体调试日志：记录序列化后大小
                if let Ok(resp_str) = serde_json::to_string(&responses_response) {
                    tracing::debug!(
                        status = %"200",
                        resp_len = resp_str.len(),
                        "responses non-streaming response body"
                    );
                } else {
                    tracing::debug!(
                        status = %"200",
                        "responses non-streaming response body (serialize failed)"
                    );
                }
                Json(responses_response).into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/responses",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "provider responses request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    }
}

/// Health check endpoint
pub async fn health_check() -> &'static str {
    "OK"
}

/// Zhipu direct endpoint - bypasses transform layer
/// POST /v1/providers/zhipu/chat/completions
///
/// This endpoint directly proxies to Zhipu AI without any format transformation.
/// Useful for clients that want native GLM model access.
pub async fn zhipu_chat_completions(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatRequest>,
) -> impl IntoResponse {
    let requested_model = body.model.clone();

    // Get Zhipu provider directly
    let provider = match state.zhipu_provider.clone() {
        Some(p) => p,
        None => {
            return Error::Provider("Zhipu provider not configured".to_string()).into_response()
        }
    };

    let mut body = body;
    let rewritten_from = rewrite_chat_request_model_for_provider(&state, provider.name(), &mut body);
    let effective_model = body.model.clone();
    let stream = body.stream.unwrap_or(false);
    let message_count = body.messages.len();
    let tool_count = body.tools.as_ref().map(|tools| tools.len()).unwrap_or(0);

    tracing::info!(
        route = "/v1/providers/zhipu/chat/completions",
        provider = provider.name(),
        model = %requested_model,
        effective_model = %effective_model,
        rewritten_from = rewritten_from.as_deref().unwrap_or(""),
        stream,
        message_count,
        tool_count,
        "dispatching direct zhipu request"
    );

    if stream {
        // Handle streaming
        match provider.chat_streaming(body).await {
            Ok(streaming) => {
                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status
                    ))
                    .into_response();
                }

                // Pass through streaming chunks without transformation
                let stream = stream::iter(streaming.chunks.into_iter().map(|chunk| {
                    Ok::<_, std::convert::Infallible>(
                        Event::default().data(serde_json::to_string(&chunk).unwrap_or_default()),
                    )
                }));

                Sse::new(stream).into_response()
            }
            Err(e) => {
                tracing::error!(
                    route = "/v1/providers/zhipu/chat/completions",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "direct zhipu streaming request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    } else {
        // Handle non-streaming - direct pass-through
        match provider.chat(body).await {
            Ok(chat_response) => Json(chat_response).into_response(),
            Err(e) => {
                tracing::error!(
                    route = "/v1/providers/zhipu/chat/completions",
                    provider = provider.name(),
                    model = %effective_model,
                    error = %e,
                    "direct zhipu request failed"
                );
                Error::Provider(e.to_string()).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        LoggingConfig, ProviderConfig, ProvidersConfig, RateLimitConfig, RoutingConfig,
        ServerConfig,
    };
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                body_limit: 10 * 1024 * 1024,
                rate_limit: RateLimitConfig::default(),
            },
            providers: ProvidersConfig {
                openai: Some(ProviderConfig {
                    api_key: "test-key".to_string(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    default_model: "gpt-4o".to_string(),
                    timeout: 60,
                }),
                zhipu: ProviderConfig {
                    api_key: "test-key".to_string(),
                    base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                    default_model: "glm-4".to_string(),
                    timeout: 60,
                },
            },
            routing: RoutingConfig {
                default: "openai".to_string(),
                model_mapping: Some(HashMap::new()),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                file_path: "logs/server.log".to_string(),
            },
        }
    }

    #[test]
    fn test_app_state_creation() {
        let config = create_test_config();
        let state = AppState::new(config);

        assert!(state.openai_provider.is_some());
        assert!(state.zhipu_provider.is_some());
    }

    #[test]
    fn test_get_default_provider() {
        let config = create_test_config();
        let state = AppState::new(config);

        let provider = state.get_provider("gpt-4o");
        assert!(provider.is_ok());
    }

    #[test]
    fn test_get_provider_with_model_mapping() {
        let mut config = create_test_config();
        let mut mapping = HashMap::new();
        mapping.insert("gpt-4".to_string(), "zhipu".to_string());
        config.routing.model_mapping = Some(mapping);

        let state = AppState::new(config);

        // Should use zhipu for gpt-4 due to mapping
        let provider = state.get_provider("gpt-4").unwrap();
        // Provider should be zhipu
        let provider_name = provider.name();
        assert_eq!(provider_name, "zhipu");
    }

    #[test]
    fn test_provider_not_configured_error() {
        let config = Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                body_limit: 10 * 1024 * 1024,
                rate_limit: RateLimitConfig::default(),
            },
            providers: ProvidersConfig {
                openai: Some(ProviderConfig {
                    api_key: "".to_string(), // Empty API key
                    base_url: "https://api.openai.com/v1".to_string(),
                    default_model: "gpt-4o".to_string(),
                    timeout: 60,
                }),
                zhipu: ProviderConfig {
                    api_key: "".to_string(), // Empty API key
                    base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                    default_model: "glm-4".to_string(),
                    timeout: 60,
                },
            },
            routing: RoutingConfig {
                default: "openai".to_string(),
                model_mapping: None,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                file_path: "logs/server.log".to_string(),
            },
        };

        let state = AppState::new(config);

        // Should fail because no providers are configured
        let result = state.get_provider("gpt-4o");
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_provider_error() {
        let config = create_test_config();
        let state = AppState::new(config);

        // Manually try to get unknown provider
        let result = state.get_provider_by_name("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_rewrite_chat_request_model_for_zhipu_uses_default_model_for_non_glm_requests() {
        let config = create_test_config();
        let state = AppState::new(config);
        let mut request = ChatRequest {
            model: "gpt-5.1-codex".to_string(),
            messages: vec![],
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: Some(true),
            stop: None,
            n: 1,
            stream_options: None,
            include_usage: Some(true),
            response_format: None,
            seed: None,
            organization: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: true,
        };

        let rewritten_from = rewrite_chat_request_model_for_provider(&state, "zhipu", &mut request);

        assert_eq!(rewritten_from.as_deref(), Some("gpt-5.1-codex"));
        assert_eq!(request.model, "glm-4");
    }

    #[test]
    fn test_rewrite_chat_request_model_for_zhipu_keeps_glm_model() {
        let config = create_test_config();
        let state = AppState::new(config);
        let mut request = ChatRequest {
            model: "glm-5".to_string(),
            messages: vec![],
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: Some(true),
            stop: None,
            n: 1,
            stream_options: None,
            include_usage: Some(true),
            response_format: None,
            seed: None,
            organization: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: true,
        };

        let rewritten_from = rewrite_chat_request_model_for_provider(&state, "zhipu", &mut request);

        assert!(rewritten_from.is_none());
        assert_eq!(request.model, "glm-5");
    }

    #[test]
    fn test_responses_stream_event_skips_empty_metadata_chunk() {
        let chunk = ChatCompletionChunk {
            id: "chatcmpl-empty".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![crate::models::chat::StreamingChoice {
                index: 0,
                delta: Some(crate::models::chat::Delta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: None,
                }),
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
        };

        assert!(responses_stream_event_from_chat_chunk(&chunk).is_none());
    }

    #[test]
    fn test_responses_stream_event_keeps_usage_only_terminal_chunk() {
        let chunk = ChatCompletionChunk {
            id: "chatcmpl-terminal".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![crate::models::chat::StreamingChoice {
                index: 0,
                delta: Some(crate::models::chat::Delta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: None,
                }),
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: Some(crate::models::chat::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        assert!(responses_stream_event_from_chat_chunk(&chunk).is_some());
    }

    #[test]
    fn test_responses_protocol_payloads_include_completed_event() {
        let chunks = vec![
            ChatCompletionChunk {
                id: "resp_123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567890,
                model: "glm-5".to_string(),
                choices: vec![crate::models::chat::StreamingChoice {
                    index: 0,
                    delta: Some(crate::models::chat::Delta {
                        role: Some("assistant".to_string()),
                        content: Some("Hello".to_string()),
                        tool_calls: None,
                    }),
                    finish_reason: None,
                    logprobs: None,
                }],
                usage: None,
            },
            ChatCompletionChunk {
                id: "resp_123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567891,
                model: "glm-5".to_string(),
                choices: vec![crate::models::chat::StreamingChoice {
                    index: 0,
                    delta: Some(crate::models::chat::Delta {
                        role: None,
                        content: None,
                        tool_calls: None,
                    }),
                    finish_reason: Some("stop".to_string()),
                    logprobs: None,
                }],
                usage: Some(crate::models::chat::Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
            },
        ];

        let payloads = responses_protocol_payloads_from_chat_chunks(&chunks);

        assert!(payloads.iter().any(|payload| payload.contains(r#""type":"response.created""#)));
        assert!(payloads.iter().any(|payload| payload.contains(r#""type":"response.output_text.delta""#)));
        assert!(payloads.iter().any(|payload| payload.contains(r#""type":"response.completed""#)));
        assert_eq!(payloads.last().map(String::as_str), Some("[DONE]"));
    }

    #[test]
    fn test_responses_protocol_payloads_use_nested_response_objects_for_lifecycle_events() {
        let chunks = vec![ChatCompletionChunk {
            id: "resp_nested".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![crate::models::chat::StreamingChoice {
                index: 0,
                delta: Some(crate::models::chat::Delta {
                    role: Some("assistant".to_string()),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                }),
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: Some(crate::models::chat::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        }];

        let payloads = responses_protocol_payloads_from_chat_chunks(&chunks);

        for expected_type in ["response.created", "response.in_progress", "response.completed"] {
            let payload = payloads
                .iter()
                .find(|payload| payload.contains(&format!(r#""type":"{expected_type}""#)))
                .expect("missing lifecycle payload");
            let payload_json: serde_json::Value =
                serde_json::from_str(payload).expect("payload should be valid json");

            let response = payload_json
                .get("response")
                .and_then(serde_json::Value::as_object)
                .expect("lifecycle payload should contain nested response object");

            assert_eq!(
                response.get("id").and_then(serde_json::Value::as_str),
                Some("resp_nested")
            );
            assert!(
                payload_json.get("response_id").is_none(),
                "lifecycle payload should not use legacy response_id field"
            );
        }
    }

    #[test]
    fn test_empty_chunk_fallback_uses_nested_completed_response_object() {
        let payloads = responses_protocol_payloads_from_chat_chunks(&[]);
        let completed_payload = payloads
            .iter()
            .find(|payload| payload.contains(r#""type":"response.completed""#))
            .expect("missing completed payload");
        let payload_json: serde_json::Value =
            serde_json::from_str(completed_payload).expect("payload should be valid json");

        assert!(payload_json.get("response").is_some());
        assert!(payload_json.get("response_id").is_none());
    }
}
