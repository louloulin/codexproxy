//! API handlers for Chat Completions and Responses endpoints
//!
//! This module implements the actual API endpoints with:
//! - Provider integration
//! - Transform layer for API conversion
//! - Streaming support

use axum::{
    extract::State,
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    Json,
};
use futures::stream;
use std::sync::Arc;

use crate::config::Config;
use crate::error::Error;
use crate::models::chat::{ChatCompletionChunk, ChatRequest};
use crate::models::response::{ResponsesRequest, ResponsesStreamChunk};
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
                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status
                    ))
                    .into_response();
                }

                let stream = stream::iter(
                    streaming
                        .chunks
                        .into_iter()
                        .filter_map(|chunk| responses_stream_event_from_chat_chunk(&chunk))
                        .map(Ok::<_, std::convert::Infallible>),
                );

                Sse::new(stream).into_response()
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
}
