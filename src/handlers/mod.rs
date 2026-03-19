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
use crate::models::chat::ChatRequest;
use crate::models::response::ResponsesRequest;
use crate::providers::{LLMProvider, OpenAIProvider, ZhipuProvider};
use crate::transform;

// Re-export provider type for internal use
type ProviderType = Arc<dyn LLMProvider>;

/// Application state containing providers
pub struct AppState {
    pub config: Config,
    pub openai_provider: Option<ProviderType>,
    pub zhipu_provider: Option<ProviderType>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let openai_provider = if !config.providers.openai.api_key.is_empty() {
            Some(Arc::new(OpenAIProvider::new(config.providers.openai.clone())) as ProviderType)
        } else {
            None
        };

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
    let provider = match state.get_provider(&body.model) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };

    // For now, pass through to Chat Completions API
    // The transform layer (transform_chat_to_responses_request) is available
    // for future implementation when we want to call Responses API
    let stream = body.stream.unwrap_or(false);

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
                let stream = stream::iter(streaming.chunks.into_iter().map(|chunk| {
                    let responses_chunk =
                        transform::transform_chat_stream_to_responses_stream(&chunk);
                    // Convert back to Chat format for client compatibility
                    let chat_chunk =
                        transform::transform_responses_stream_to_chat_stream(&responses_chunk);
                    Ok::<_, std::convert::Infallible>(
                        Event::default()
                            .data(serde_json::to_string(&chat_chunk).unwrap_or_default()),
                    )
                }));

                Sse::new(stream).into_response()
            }
            Err(e) => Error::Provider(e.to_string()).into_response(),
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
            Err(e) => Error::Provider(e.to_string()).into_response(),
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
            body_len = body_str.len(),
            "responses request body"
        );
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

    let stream = body.stream.unwrap_or(false);

    if stream {
        // Handle streaming - convert to chat format, call provider, convert back
        let chat_request = transform::transform_responses_to_chat_request(&body);

        match provider.chat_streaming(chat_request).await {
            Ok(streaming) => {
                if !streaming.is_success() {
                    return Error::Provider(format!(
                        "Streaming request failed with status: {}",
                        streaming.status
                    ))
                    .into_response();
                }

                let stream = stream::iter(streaming.chunks.into_iter().map(|chunk| {
                    let responses_chunk =
                        transform::transform_chat_stream_to_responses_stream(&chunk);
                    Ok::<_, std::convert::Infallible>(
                        Event::default()
                            .data(serde_json::to_string(&responses_chunk).unwrap_or_default()),
                    )
                }));

                Sse::new(stream).into_response()
            }
            Err(e) => Error::Provider(e.to_string()).into_response(),
        }
    } else {
        // Handle non-streaming
        let chat_request = transform::transform_responses_to_chat_request(&body);

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
            Err(e) => Error::Provider(e.to_string()).into_response(),
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
    // Get Zhipu provider directly
    let provider = match state.zhipu_provider.clone() {
        Some(p) => p,
        None => {
            return Error::Provider("Zhipu provider not configured".to_string()).into_response()
        }
    };

    let stream = body.stream.unwrap_or(false);

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
            Err(e) => Error::Provider(e.to_string()).into_response(),
        }
    } else {
        // Handle non-streaming - direct pass-through
        match provider.chat(body).await {
            Ok(chat_response) => Json(chat_response).into_response(),
            Err(e) => Error::Provider(e.to_string()).into_response(),
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
                rate_limit: RateLimitConfig::default(),
            },
            providers: ProvidersConfig {
                openai: ProviderConfig {
                    api_key: "test-key".to_string(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    default_model: "gpt-4o".to_string(),
                    timeout: 60,
                },
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
                rate_limit: RateLimitConfig::default(),
            },
            providers: ProvidersConfig {
                openai: ProviderConfig {
                    api_key: "".to_string(), // Empty API key
                    base_url: "https://api.openai.com/v1".to_string(),
                    default_model: "gpt-4o".to_string(),
                    timeout: 60,
                },
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
}
