//! Handler utility functions
//!
//! Shared helper functions used by chat and responses handlers.

use axum::response::sse::Event;
use std::sync::Arc;

use crate::config::Config;
use crate::error::Error;
use crate::models::chat::{ChatCompletionChunk, ChatRequest};
use crate::models::response::{ResponsesRequest, ResponsesStreamChunk};
use crate::protocol::capabilities::ResponsesExecutionPlan;
use crate::transform;

// Re-export provider type for internal use
pub type ProviderType = Arc<dyn crate::providers::LLMProvider>;

/// Rewrite chat request model for provider-specific handling
pub fn rewrite_chat_request_model_for_provider(
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

/// Check if a responses stream chunk should be emitted
pub fn should_emit_responses_stream_chunk(chunk: &ResponsesStreamChunk) -> bool {
    !chunk.output.is_empty() || chunk.usage.is_some()
}

/// Build responses execution plan based on provider capabilities
pub fn build_responses_execution_plan(
    provider: &dyn crate::providers::LLMProvider,
    request: &ResponsesRequest,
) -> ResponsesExecutionPlan {
    ResponsesExecutionPlan::from_request(&provider.capabilities(), request)
}

/// Generate error message for unsupported responses features
pub fn unsupported_responses_features_message(
    provider_name: &str,
    unsupported_features: &[String],
) -> String {
    format!(
        "Provider {provider_name} cannot preserve Responses features in chat fallback mode: {}",
        unsupported_features.join(", ")
    )
}

/// Convert chat chunks to responses protocol payloads
pub fn responses_protocol_payloads_from_chat_chunks(chunks: &[ChatCompletionChunk]) -> Vec<String> {
    crate::protocol::events::responses_protocol_payloads_from_chat_chunks(chunks)
}

/// Convert chat chunks to SSE events for streaming responses
pub fn responses_stream_events_from_chat_chunks(chunks: &[ChatCompletionChunk]) -> Vec<Event> {
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
                let event_type = json
                    .get("type")
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

/// Convert chat chunk to SSE event
pub fn chat_stream_event_from_chat_chunk(chunk: &ChatCompletionChunk) -> Option<Event> {
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
    pub auth_state: Option<std::sync::Arc<crate::auth::AuthState>>,
    pub request_repo: Option<std::sync::Arc<crate::db::repository::RequestRepository>>,
    pub provider_registry: Option<std::sync::Arc<std::sync::Mutex<crate::providers::ProviderRegistry>>>,
    pub generic_provider_loader: Option<std::sync::Arc<std::sync::Mutex<crate::providers::generic_provider::GenericProviderLoader>>>,
}

impl AppState {
    /// Create new AppState from Config (no auth)
    pub fn new(config: Config) -> Self {
        Self::new_with_auth(config, None)
    }

    /// Create new AppState with optional auth state
    pub fn new_with_auth(config: Config, auth_state: Option<std::sync::Arc<crate::auth::AuthState>>) -> Self {
        use crate::providers::{OpenAIProvider, ZhipuProvider};

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
            auth_state,
            request_repo: None,
            provider_registry: None,
            generic_provider_loader: None,
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
    pub fn get_provider_by_name(&self, name: &str) -> Result<ProviderType, Error> {
        // Check dynamic registry first (if available)
        if let Some(ref registry) = self.provider_registry {
            if let Ok(reg) = registry.lock() {
                if let Some(ext_provider) = reg.get(name) {
                    // Convert ExtendedLLMProvider to LLMProvider - just return any configured provider
                    // The registry provides routing info but actual API calls use configured providers
                }
            }
        }
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

    /// List all available providers (both configured and registered)
    pub fn list_provider_infos(&self) -> Vec<ProviderInfo> {
        let mut providers = Vec::new();

        // Add configured providers
        if self.openai_provider.is_some() {
            providers.push(ProviderInfo { id: "openai".into(), name: "OpenAI".into(), source: "config".into(), model_count: 5 });
        }
        if self.zhipu_provider.is_some() {
            providers.push(ProviderInfo { id: "zhipu".into(), name: "Zhipu AI".into(), source: "config".into(), model_count: 3 });
        }

        // Add registry providers (dynamic)
        if let Some(ref registry) = self.provider_registry {
            if let Ok(reg) = registry.lock() {
                for name in reg.provider_names() {
                    if !providers.iter().any(|p| p.id == name) {
                        providers.push(ProviderInfo { id: name.to_string(), name: name.to_string(), source: "registry".into(), model_count: 0 });
                    }
                }
            }
        }

        providers
    }
}

/// Provider info for listing
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub source: String,
    pub model_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        use crate::config::{
            LoggingConfig, ProviderConfig, ProvidersConfig, RateLimitConfig, RoutingConfig,
            ServerConfig,
        };
        use std::collections::HashMap;

        Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                body_limit: 10 * 1024 * 1024,
                data_dir: "data/db".to_string(),
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
                minimax: None,
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
            codex_cli: crate::config::CodexCliConfig::default(),
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
    fn test_get_provider_by_name() {
        let config = create_test_config();
        let state = AppState::new(config);

        let openai = state.get_provider_by_name("openai");
        assert!(openai.is_ok());

        let zhipu = state.get_provider_by_name("zhipu");
        assert!(zhipu.is_ok());

        let unknown = state.get_provider_by_name("unknown");
        assert!(unknown.is_err());
    }

    #[test]
    fn test_provider_not_configured_error() {
        use crate::config::{
            LoggingConfig, ProviderConfig, ProvidersConfig, RateLimitConfig, RoutingConfig,
            ServerConfig,
        };

        let config = Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                body_limit: 10 * 1024 * 1024,
                data_dir: "data/db".to_string(),
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
                minimax: None,
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
            codex_cli: crate::config::CodexCliConfig::default(),
        };

        let state = AppState::new(config);

        // Should fail because no providers are configured
        let result = state.get_provider("gpt-4o");
        assert!(result.is_err());
    }

    #[test]
    fn test_model_mapping_routing() {
        use crate::config::{
            LoggingConfig, ProviderConfig, ProvidersConfig, RateLimitConfig, RoutingConfig,
            ServerConfig,
        };
        use std::collections::HashMap;

        let mut mapping = HashMap::new();
        mapping.insert("gpt-3.5".to_string(), "openai".to_string());
        mapping.insert("glm-4".to_string(), "zhipu".to_string());

        let config = Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                body_limit: 10 * 1024 * 1024,
                data_dir: "data/db".to_string(),
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
                minimax: None,
            },
            routing: RoutingConfig {
                default: "openai".to_string(),
                model_mapping: Some(mapping),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                file_path: "logs/server.log".to_string(),
            },
            codex_cli: crate::config::CodexCliConfig::default(),
        };

        let state = AppState::new(config);

        // gpt-3.5 should route to openai
        let provider = state.get_provider("gpt-3.5");
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().name(), "openai");

        // glm-4 should route to zhipu
        let provider = state.get_provider("glm-4");
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().name(), "zhipu");

        // Unknown model should use default (openai)
        let provider = state.get_provider("unknown-model");
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().name(), "openai");
    }
}
