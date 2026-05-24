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
use futures::{stream, StreamExt};
use std::sync::Arc;

use crate::config::Config;
use crate::error::Error;
use crate::models::chat::{ChatCompletionChunk, ChatRequest};
use crate::models::response::{ResponsesRequest, ResponsesStreamChunk};
use crate::protocol::capabilities::ResponsesExecutionPlan;
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

fn build_responses_execution_plan(
    provider: &dyn LLMProvider,
    request: &ResponsesRequest,
) -> ResponsesExecutionPlan {
    ResponsesExecutionPlan::from_request(&provider.capabilities(), request)
}

fn unsupported_responses_features_message(
    provider_name: &str,
    unsupported_features: &[String],
) -> String {
    format!(
        "Provider {provider_name} cannot preserve Responses features in chat fallback mode: {}",
        unsupported_features.join(", ")
    )
}

fn responses_protocol_payloads_from_chat_chunks(chunks: &[ChatCompletionChunk]) -> Vec<String> {
    crate::protocol::events::responses_protocol_payloads_from_chat_chunks(chunks)
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
        .filter_map(|payload| {
            if payload == "[DONE]" {
                return Some(Event::default().data("[DONE]"));
            }
            // Extract event type from payload JSON for SSE event name
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) {
                let event_type = json.get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("message");
                Some(Event::default().event(event_type).data(payload))
            } else {
                Some(Event::default().data(payload))
            }
        })
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
                        streaming.status()
                    ))
                    .into_response();
                }

                // Transform streaming chunks to Responses format to demonstrate conversion
                let stream = match streaming {
                    crate::providers::StreamingChat::Collected { chunks, .. } => {
                        stream::iter(
                            chunks
                                .into_iter()
                                .filter_map(|chunk| chat_stream_event_from_chat_chunk(&chunk))
                                .map(Ok::<_, std::convert::Infallible>),
                        )
                        .boxed()
                    }
                    crate::providers::StreamingChat::Streamed { stream, .. } => {
                        stream
                            .filter_map(|chunk| async move {
                                chat_stream_event_from_chat_chunk(&chunk)
                            })
                            .map(|event| Ok::<_, std::convert::Infallible>(event))
                            .boxed()
                    }
                };

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
    let requested_model_for_response = body.model.clone();

    let execution_plan = build_responses_execution_plan(provider.as_ref(), &body);
    tracing::info!(
        route = "/v1/responses",
        provider = provider.name(),
        model = %requested_model_for_response,
        stream,
        execution_plan = ?execution_plan,
        "planned responses execution"
    );

    match &execution_plan {
        ResponsesExecutionPlan::NativeResponses => {
            tracing::info!(
                route = "/v1/responses",
                provider = provider.name(),
                model = %requested_model_for_response,
                stream,
                include_count = body.include.len(),
                has_previous_response_id = body.previous_response_id.is_some(),
                "using native responses provider path"
            );

            if stream {
                match provider.responses_streaming(body).await {
                    Ok(streaming) => {
                        if !streaming.is_success() {
                            return Error::Provider(format!(
                                "Streaming responses request failed with status: {}",
                                streaming.status
                            ))
                            .into_response();
                        }

                        let stream = streaming.events.map(|payload| {
                            Ok::<_, std::convert::Infallible>(Event::default().data(payload))
                        });

                        return Sse::new(stream)
                            .keep_alive(KeepAlive::new())
                            .into_response();
                    }
                    Err(e) => {
                        tracing::error!(
                            route = "/v1/responses",
                            provider = provider.name(),
                            model = %requested_model_for_response,
                            error = %e,
                            "provider native responses streaming request failed"
                        );
                        return Error::Provider(e.to_string()).into_response();
                    }
                }
            }

            match provider.responses(body).await {
                Ok(responses_response) => return Json(responses_response).into_response(),
                Err(e) => {
                    tracing::error!(
                        route = "/v1/responses",
                        provider = provider.name(),
                        model = %requested_model_for_response,
                        error = %e,
                        "provider native responses request failed"
                    );
                    return Error::Provider(e.to_string()).into_response();
                }
            }
        }
        ResponsesExecutionPlan::Reject {
            unsupported_features,
        } => {
            let message =
                unsupported_responses_features_message(provider.name(), unsupported_features);
            tracing::warn!(
                route = "/v1/responses",
                provider = provider.name(),
                model = %requested_model_for_response,
                unsupported_features = ?unsupported_features,
                "rejecting responses request before lossy chat fallback"
            );
            return Error::Provider(message).into_response();
        }
        ResponsesExecutionPlan::ChatFallback => {}
    }

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
                    status = streaming.status(),
                    is_streamed = streaming.is_streamed(),
                    is_collected = streaming.is_collected(),
                    "provider streaming response received"
                );

                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status()
                    ))
                    .into_response();
                }

                // Handle based on streaming type
                match streaming {
                    crate::providers::StreamingChat::Collected { chunks, .. } => {
                        // Log chunk details
                        for (i, chunk) in chunks.iter().enumerate() {
                            tracing::debug!(
                                route = "/v1/responses",
                                chunk_index = i,
                                chunk_id = %chunk.id,
                                choices_count = chunk.choices.len(),
                                has_usage = chunk.usage.is_some(),
                                "streaming chunk detail"
                            );
                        }

                        let events = responses_stream_events_from_chat_chunks(&chunks);
                        tracing::info!(
                            route = "/v1/responses",
                            event_count = events.len(),
                            chunk_count = chunks.len(),
                            "generated SSE events from chunks"
                        );

                        // 保险检查：确保至少有 [DONE]
                        if events.is_empty() {
                            tracing::error!(
                                route = "/v1/responses",
                                chunk_count = chunks.len(),
                                "CRITICAL: No events generated from chunks, using fallback"
                            );
                            let fallback_events = responses_stream_events_from_chat_chunks(&[]);

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
                    crate::providers::StreamingChat::Streamed { stream, .. } => {
                        let chunks: Vec<_> = stream.collect().await;
                        let events = responses_stream_events_from_chat_chunks(&chunks);
                        tracing::info!(
                            route = "/v1/responses",
                            event_count = events.len(),
                            chunk_count = chunks.len(),
                            "generated SSE events from streamed chunks"
                        );

                        let stream = stream::iter(
                            events
                                .into_iter()
                                .map(Ok::<_, std::convert::Infallible>),
                        );

                        Sse::new(stream)
                            .keep_alive(KeepAlive::new())
                            .into_response()
                    }
                }
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
                        streaming.status()
                    ))
                    .into_response();
                }

                // Pass through streaming chunks without transformation
                let stream = match streaming {
                    crate::providers::StreamingChat::Collected { chunks, .. } => {
                        stream::iter(chunks.into_iter().map(|chunk| {
                            Ok::<_, std::convert::Infallible>(
                                Event::default().data(serde_json::to_string(&chunk).unwrap_or_default()),
                            )
                        }))
                        .boxed()
                    }
                    crate::providers::StreamingChat::Streamed { stream, .. } => {
                        stream
                            .map(|chunk| {
                                Ok::<_, std::convert::Infallible>(
                                    Event::default().data(serde_json::to_string(&chunk).unwrap_or_default()),
                                )
                            })
                            .boxed()
                    }
                };

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
    use axum::{
        body::Body,
        extract::State,
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::post,
        Json, Router,
    };
    use crate::config::{
        CodexCliConfig, LoggingConfig, ProviderConfig, ProvidersConfig, RateLimitConfig,
        RoutingConfig, ServerConfig,
    };
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    #[derive(Clone)]
    struct MockResponsesUpstreamState {
        recorded_requests: Arc<Mutex<Vec<Value>>>,
        non_stream_response: Value,
        stream_payloads: Vec<String>,
    }

    async fn mock_responses_endpoint(
        State(state): State<MockResponsesUpstreamState>,
        Json(body): Json<Value>,
    ) -> impl IntoResponse {
        state.recorded_requests.lock().unwrap().push(body.clone());

        if body.get("stream").and_then(Value::as_bool).unwrap_or(false) {
            let mut sse_body = String::new();
            for payload in &state.stream_payloads {
                sse_body.push_str("data: ");
                sse_body.push_str(payload);
                sse_body.push_str("\n\n");
            }

            return (
                StatusCode::OK,
                [("content-type", "text/event-stream")],
                sse_body,
            )
                .into_response();
        }

        Json(state.non_stream_response.clone()).into_response()
    }

    async fn unexpected_chat_endpoint() -> impl IntoResponse {
        (
            StatusCode::IM_A_TEAPOT,
            Json(json!({
                "error": {
                    "message": "chat completions endpoint should not be called for native responses requests"
                }
            })),
        )
            .into_response()
    }

    async fn spawn_mock_openai_responses_upstream(
        non_stream_response: Value,
        stream_payloads: Vec<String>,
    ) -> (String, Arc<Mutex<Vec<Value>>>) {
        let recorded_requests = Arc::new(Mutex::new(Vec::new()));
        let state = MockResponsesUpstreamState {
            recorded_requests: Arc::clone(&recorded_requests),
            non_stream_response,
            stream_payloads,
        };

        let app = Router::new()
            .route("/v1/responses", post(mock_responses_endpoint))
            .route("/v1/chat/completions", post(unexpected_chat_endpoint))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock upstream listener should bind");
        let addr = listener
            .local_addr()
            .expect("mock upstream listener should have addr");
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("mock upstream server should serve");
        });

        (format!("http://{addr}"), recorded_requests)
    }

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
            codex_cli: CodexCliConfig::default(),
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
    fn test_provider_capabilities_differ_between_openai_and_zhipu() {
        let config = create_test_config();
        let state = AppState::new(config);

        let openai = state
            .get_provider_by_name("openai")
            .expect("openai provider should exist");
        let zhipu = state
            .get_provider_by_name("zhipu")
            .expect("zhipu provider should exist");

        let openai_capabilities = openai.capabilities();
        let zhipu_capabilities = zhipu.capabilities();

        assert!(openai_capabilities.native_responses);
        assert!(!zhipu_capabilities.native_responses);
        assert!(openai_capabilities.supports_previous_response_id);
        assert!(!zhipu_capabilities.supports_previous_response_id);
        assert!(
            openai_capabilities.supported_tool_types.len()
                > zhipu_capabilities.supported_tool_types.len()
        );
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
            codex_cli: CodexCliConfig::default(),
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
    fn test_responses_protocol_payloads_emit_output_item_done_for_snake_case_terminal_chunk() {
        let raw = json!({
            "id": "resp_snake_case",
            "object": "chat.completion.chunk",
            "created": 1234567891u64,
            "model": "glm-4-flash",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": ""
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        let chunk: ChatCompletionChunk =
            serde_json::from_value(raw).expect("terminal chunk should deserialize");
        let payloads = responses_protocol_payloads_from_chat_chunks(&[chunk]);

        assert!(payloads.iter().any(|payload| {
            payload.contains(r#""type":"response.output_item.done""#)
                && payload.contains(r#""status":"completed""#)
        }));
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

    #[test]
    fn test_responses_protocol_payloads_include_function_call_semantic_events() {
        let chunks = vec![
            ChatCompletionChunk {
                id: "resp_tool".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567890,
                model: "glm-5".to_string(),
                choices: vec![crate::models::chat::StreamingChoice {
                    index: 0,
                    delta: Some(crate::models::chat::Delta {
                        role: Some("assistant".to_string()),
                        content: None,
                        tool_calls: Some(vec![crate::models::chat::ToolCall {
                            id: "call_weather".to_string(),
                            call_type: "function".to_string(),
                            function: crate::models::chat::FunctionCall {
                                name: "get_weather".to_string(),
                                arguments: "{\"city\":\"Par".to_string(),
                            },
                        }]),
                    }),
                    finish_reason: None,
                    logprobs: None,
                }],
                usage: None,
            },
            ChatCompletionChunk {
                id: "resp_tool".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567891,
                model: "glm-5".to_string(),
                choices: vec![crate::models::chat::StreamingChoice {
                    index: 0,
                    delta: Some(crate::models::chat::Delta {
                        role: None,
                        content: None,
                        tool_calls: Some(vec![crate::models::chat::ToolCall {
                            id: "call_weather".to_string(),
                            call_type: "function".to_string(),
                            function: crate::models::chat::FunctionCall {
                                name: "get_weather".to_string(),
                                arguments: "is\"}".to_string(),
                            },
                        }]),
                    }),
                    finish_reason: Some("tool_calls".to_string()),
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

        assert!(payloads.iter().any(|payload| {
            payload.contains(r#""type":"response.output_item.added""#)
                && payload.contains(r#""type":"function_call""#)
                && payload.contains(r#""call_id":"call_weather""#)
        }));
        assert!(payloads.iter().any(|payload| {
            payload.contains(r#""type":"response.function_call_arguments.delta""#)
                && payload.contains(r#""call_id":"call_weather""#)
                && payload.contains(r#""delta":"{\"city\":\"Par""#)
        }));
        assert!(payloads.iter().any(|payload| {
            payload.contains(r#""type":"response.function_call_arguments.done""#)
                && payload.contains(r#""call_id":"call_weather""#)
                && payload.contains(r#""arguments":"{\"city\":\"Paris\"}""#)
        }));
        assert!(payloads.iter().any(|payload| {
            payload.contains(r#""type":"response.output_item.done""#)
                && payload.contains(r#""type":"function_call""#)
                && payload.contains(r#""status":"completed""#)
        }));
    }

    #[tokio::test]
    async fn test_openai_responses_handler_uses_native_responses_api_and_preserves_request_fields() {
        let upstream_response = json!({
            "id": "resp_native",
            "object": "response",
            "created_at": 1234567890u64,
            "model": "gpt-4o",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "native responses path"
                        }
                    ]
                }
            ],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 7,
                "total_tokens": 18
            }
        });
        let (upstream_base_url, recorded_requests) =
            spawn_mock_openai_responses_upstream(upstream_response, vec![]).await;

        let mut config = create_test_config();
        config.providers.openai = Some(ProviderConfig {
            api_key: "test-key".to_string(),
            base_url: format!("{upstream_base_url}/v1"),
            default_model: "gpt-4o".to_string(),
            timeout: 5,
        });
        config.providers.zhipu.api_key = "".to_string();
        config.routing.default = "openai".to_string();

        let app = crate::server::router::create_router(Arc::new(AppState::new(config)));
        let request_body = json!({
            "model": "gpt-4o",
            "instructions": "system prompt",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": "hello"
                        }
                    ]
                }
            ],
            "previous_response_id": "resp_prev_123",
            "include": ["reasoning.encrypted_content", "web_search_call.action.sources"],
            "tool_choice": "auto",
            "parallel_tool_calls": false,
            "stream": false
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&request_body).expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("response should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let body_json: Value =
            serde_json::from_slice(&body).expect("response body should be json");
        assert_eq!(body_json.get("id").and_then(Value::as_str), Some("resp_native"));

        let recorded = recorded_requests.lock().unwrap();
        assert_eq!(recorded.len(), 1, "native responses path should hit /v1/responses once");
        let upstream_request = &recorded[0];
        assert_eq!(
            upstream_request
                .get("previous_response_id")
                .and_then(Value::as_str),
            Some("resp_prev_123")
        );
        assert_eq!(
            upstream_request.get("include").and_then(Value::as_array).map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
            }),
            Some(vec![
                "reasoning.encrypted_content",
                "web_search_call.action.sources",
            ])
        );
        assert_eq!(
            upstream_request
                .get("parallel_tool_calls")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            upstream_request.get("tool_choice").and_then(Value::as_str),
            Some("auto")
        );
    }

    #[tokio::test]
    async fn test_openai_responses_streaming_handler_forwards_native_responses_events() {
        let stream_payloads = vec![
            json!({
                "type": "response.created",
                "response": {
                    "id": "resp_stream",
                    "object": "response",
                    "created_at": 1234567890u64,
                    "status": "in_progress",
                    "model": "gpt-4o",
                    "output": [],
                    "usage": null
                }
            })
            .to_string(),
            json!({
                "type": "response.completed",
                "response": {
                    "id": "resp_stream",
                    "object": "response",
                    "created_at": 1234567891u64,
                    "status": "completed",
                    "model": "gpt-4o",
                    "output": [],
                    "usage": {
                        "input_tokens": 5,
                        "output_tokens": 3,
                        "total_tokens": 8
                    }
                }
            })
            .to_string(),
            "[DONE]".to_string(),
        ];
        let (upstream_base_url, recorded_requests) =
            spawn_mock_openai_responses_upstream(json!({}), stream_payloads).await;

        let mut config = create_test_config();
        config.providers.openai = Some(ProviderConfig {
            api_key: "test-key".to_string(),
            base_url: format!("{upstream_base_url}/v1"),
            default_model: "gpt-4o".to_string(),
            timeout: 5,
        });
        config.providers.zhipu.api_key = "".to_string();
        config.routing.default = "openai".to_string();

        let app = crate::server::router::create_router(Arc::new(AppState::new(config)));
        let request_body = json!({
            "model": "gpt-4o",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": "stream hello"
                        }
                    ]
                }
            ],
            "previous_response_id": "resp_prev_stream",
            "stream": true
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&request_body).expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("response should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body should collect")
            .to_bytes();
        let body_str = String::from_utf8(body.to_vec()).expect("sse body should be utf8");
        assert!(
            body_str.contains(r#""type":"response.created""#),
            "stream should forward native response.created event"
        );
        assert!(
            body_str.contains(r#""type":"response.completed""#),
            "stream should forward native response.completed event"
        );
        assert!(
            body_str.contains("[DONE]"),
            "stream should forward native done marker"
        );

        let recorded = recorded_requests.lock().unwrap();
        assert_eq!(recorded.len(), 1, "streaming path should hit /v1/responses once");
        assert_eq!(
            recorded[0]
                .get("previous_response_id")
                .and_then(Value::as_str),
            Some("resp_prev_stream")
        );
    }

    #[tokio::test]
    async fn test_zhipu_responses_rejects_previous_response_id_in_chat_fallback() {
        let mut config = create_test_config();
        config.providers.openai = None;
        config.routing.default = "zhipu".to_string();

        let app = crate::server::router::create_router(Arc::new(AppState::new(config)));
        let request_body = json!({
            "model": "glm-5",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": "hello"
                        }
                    ]
                }
            ],
            "previous_response_id": "resp_prev_unsupported"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&request_body).expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("response should succeed");

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let body_json: Value =
            serde_json::from_slice(&body).expect("response body should be json");
        let message = body_json
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        assert!(
            message.contains("previous_response_id"),
            "expected previous_response_id rejection, got: {message}"
        );
    }

    #[tokio::test]
    async fn test_zhipu_responses_rejects_non_function_tools_in_chat_fallback() {
        let mut config = create_test_config();
        config.providers.openai = None;
        config.routing.default = "zhipu".to_string();

        let app = crate::server::router::create_router(Arc::new(AppState::new(config)));
        let request_body = json!({
            "model": "glm-5",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": "search docs"
                        }
                    ]
                }
            ],
            "tools": [
                {
                    "type": "file_search",
                    "vector_store_ids": ["vs_123"]
                }
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&request_body).expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("response should succeed");

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let body_json: Value =
            serde_json::from_slice(&body).expect("response body should be json");
        let message = body_json
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        assert!(
            message.contains("file_search"),
            "expected file_search rejection, got: {message}"
        );
    }

    #[tokio::test]
    async fn test_zhipu_responses_rejects_include_in_chat_fallback() {
        let mut config = create_test_config();
        config.providers.openai = None;
        config.routing.default = "zhipu".to_string();

        let app = crate::server::router::create_router(Arc::new(AppState::new(config)));
        let request_body = json!({
            "model": "glm-5",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": "think deeply"
                        }
                    ]
                }
            ],
            "include": ["reasoning.encrypted_content"],
            "reasoning": {
                "effort": "high"
            }
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&request_body).expect("request should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("response should succeed");

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let body_json: Value =
            serde_json::from_slice(&body).expect("response body should be json");
        let message = body_json
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        // include is now allowed (just logged as warning) - the error is from provider auth
        assert!(
            !message.contains("include"),
            "expected include NOT to be rejected, got: {message}"
        );
    }
}
