//! DeepSeek Provider implementation
//!
//! Implements the LLMProvider trait for DeepSeek's API.

use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::Client;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatCompletionChunk, ChatRequest, ChatResponse, ThinkingConfig as ChatThinkingConfig};
use crate::models::response::{ResponsesRequest, ResponsesResponse};
use crate::protocol::capabilities::ProviderCapabilities;

use super::{
    build_http_client, drain_complete_sse_payloads, extract_provider_error_details,
    format_request_body_for_log, truncate_for_log, LLMProvider, ProviderError, ProviderModel, ModelFeatures,
    StreamingChat, StreamingResponses,
};

/// DeepSeek Provider
pub struct DeepSeekProvider {
    /// Configuration
    config: ProviderConfig,
    /// HTTP client
    client: Client,
    /// Base URL for API
    base_url: String,
}

/// Models that have thinking disabled by default (legacy models)
const DEEPSEEK_THINKING_DISABLED: &[&str] = &["deepseek-chat", "deepseek-v3"];

/// Models that require stripping reasoning_content (legacy R1)
const DEEPSEEK_LEGACY_R1_MODELS: &[&str] = &["deepseek-reasoner", "deepseek-r1"];

impl DeepSeekProvider {
    /// Create a new DeepSeek provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config.base_url.clone();
        let timeout = config.timeout;
        Self {
            config,
            client: build_http_client(timeout),
            base_url,
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Self {
        Self::new(ProviderConfig {
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            base_url: "https://api.deepseek.com".to_string(),
            default_model: "deepseek-v4-flash".to_string(),
            timeout: 120,
        })
    }

    /// Create with default configuration
    pub fn with_defaults(api_key: &str) -> Self {
        Self::new(ProviderConfig {
            api_key: api_key.to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            default_model: "deepseek-v4-flash".to_string(),
            timeout: 120,
        })
    }

    /// Build URL for an endpoint
    fn build_url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// Add auth headers to a request builder
    fn add_auth_headers(&self, req_builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req_builder
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
    }

    /// Check if model is a legacy R1 model (needs reasoning_content stripping)
    fn is_legacy_r1_model(&self) -> bool {
        DEEPSEEK_LEGACY_R1_MODELS.contains(&self.config.default_model.as_str())
    }

    /// Get the list of built-in models
    /// Updated to match mimo2codex with aliases and V4 support
    pub fn builtin_models() -> Vec<ProviderModel> {
        vec![
            // DeepSeek V4 Pro - thinking enabled
            ProviderModel {
                id: "deepseek-v4-pro".to_string(),
                name: "DeepSeek V4 Pro".to_string(),
                aliases: vec![],
                context_window: 64000,
                max_output_tokens: 8192,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: false,
                    web_search: false,
                    thinking: true,
                    audio_output: false,
                },
            },
            // DeepSeek V4 Flash - thinking enabled, supports legacy aliases
            ProviderModel {
                id: "deepseek-v4-flash".to_string(),
                name: "DeepSeek V4 Flash".to_string(),
                aliases: vec![
                    "deepseek-chat".to_string(),
                    "deepseek-reasoner".to_string(),
                ],
                context_window: 64000,
                max_output_tokens: 8192,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: false,
                    web_search: false,
                    thinking: true,
                    audio_output: false,
                },
            },
            // Legacy Chat (deprecated after 2026-07-24)
            ProviderModel {
                id: "deepseek-chat".to_string(),
                name: "DeepSeek Chat (Legacy)".to_string(),
                aliases: vec![],
                context_window: 64000,
                max_output_tokens: 8192,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: false,
                    web_search: false,
                    thinking: false,
                    audio_output: false,
                },
            },
            // Legacy Reasoner (deprecated after 2026-07-24)
            ProviderModel {
                id: "deepseek-reasoner".to_string(),
                name: "DeepSeek Reasoner (Legacy)".to_string(),
                aliases: vec![],
                context_window: 64000,
                max_output_tokens: 8192,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: false,
                    web_search: false,
                    thinking: true,
                    audio_output: false,
                },
            },
        ]
    }

    /// Normalize model ID, resolving aliases to canonical IDs
    pub fn normalize_model(&self, model: &str) -> String {
        for provider_model in Self::builtin_models() {
            if provider_model.id == model {
                return model.to_string();
            }
            if provider_model.aliases.contains(&model.to_string()) {
                return provider_model.id.clone();
            }
        }
        model.to_string()
    }

    /// Get capabilities for this provider
    fn get_capabilities() -> ProviderCapabilities {
        ProviderCapabilities::native_responses(vec!["function".to_string()])
    }
}

/// Normalize a DeepSeek chat request body
/// This handles:
/// 1. Auto-inject thinking: enabled for V4 models
/// 2. Set reasoning_effort: high when thinking is enabled
/// 3. Delete temperature/top_p when thinking is enabled
/// 4. Handle reasoning_effort: none when thinking is disabled
/// 5. Strip reasoning_content for legacy R1 models
pub fn normalize_deepseek_body(request: ChatRequest) -> ChatRequest {
    let mut normalized = request;

    // 1. Auto-inject thinking: enabled (unless already set)
    if normalized.thinking.is_none() {
        normalized.thinking = Some(ChatThinkingConfig {
            type_: "enabled".to_string(),
        });
    }

    // 2. Get thinking type for downstream logic
    let is_thinking_enabled = normalized.thinking.as_ref()
        .map(|t| t.type_ == "enabled")
        .unwrap_or(true);

    let is_thinking_disabled = normalized.thinking.as_ref()
        .map(|t| t.type_ == "disabled")
        .unwrap_or(false);

    // 3. Handle reasoning_effort based on thinking state
    if is_thinking_disabled {
        // When thinking is disabled, remove reasoning_effort if it's "none"
        // Note: DeepSeek uses "none" as a string value in API
        if normalized.reasoning_effort.is_some() {
            normalized.reasoning_effort = None;
        }
    } else if normalized.reasoning_effort.is_none() && is_thinking_enabled {
        // When thinking is enabled and reasoning_effort not set, default to "high"
        normalized.reasoning_effort = Some(crate::models::chat::ReasoningEffort::High);
    }

    // 4. Delete temperature/top_p/penalties when thinking is enabled
    if is_thinking_enabled {
        normalized.temperature = None;
        normalized.top_p = None;
        normalized.presence_penalty = None;
        normalized.frequency_penalty = None;
    }

    // 5. Strip reasoning_content for legacy R1 models (deepseek-reasoner)
    if normalized.model == "deepseek-reasoner" {
        for msg in &mut normalized.messages {
            msg.reasoning_content = None;
        }
    }

    normalized
}

#[async_trait]
impl LLMProvider for DeepSeekProvider {
    fn name(&self) -> &str {
        "deepseek"
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    fn client(&self) -> &Client {
        &self.client
    }

    fn capabilities(&self) -> ProviderCapabilities {
        DeepSeekProvider::get_capabilities()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = self.build_url("/chat/completions");

        let request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::debug!(
            url,
            model = %request.model,
            message_count = request.messages.len(),
            "DeepSeek chat request"
        );

        let start_time = std::time::Instant::now();
        let resp = self.add_auth_headers(self.client.post(&url))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let elapsed = start_time.elapsed();
        let status = resp.status();
        tracing::info!(
            status = status.as_u16(),
            elapsed_ms = elapsed.as_millis(),
            "DeepSeek chat response received"
        );

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                status = status.as_u16(),
                body = %truncate_for_log(&body, 500),
                "DeepSeek chat request failed"
            );

            if let Some(details) = extract_provider_error_details(&body) {
                return Err(ProviderError::RequestFailed(details.message));
            }
            return Err(ProviderError::RequestFailed(format!(
                "HTTP {}: {}",
                status.as_u16(),
                truncate_for_log(&body, 200)
            )));
        }

        let chat_response: ChatResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        Ok(chat_response)
    }

    async fn chat_streaming(&self, request: ChatRequest) -> Result<StreamingChat, ProviderError> {
        let url = self.build_url("/chat/completions");

        let mut request_with_stream = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        if let Some(obj) = request_with_stream.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::json!(true));
        }

        tracing::debug!(
            url,
            model = %request.model,
            "DeepSeek streaming chat request"
        );

        let start_time = std::time::Instant::now();
        let resp = self.add_auth_headers(self.client.post(&url))
            .json(&request_with_stream)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let elapsed = start_time.elapsed();
        let status = resp.status();
        tracing::info!(
            status = status.as_u16(),
            elapsed_ms = elapsed.as_millis(),
            "DeepSeek streaming chat response received"
        );

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                status = status.as_u16(),
                body = %truncate_for_log(&body, 500),
                "DeepSeek streaming chat request failed"
            );

            if let Some(details) = extract_provider_error_details(&body) {
                return Err(ProviderError::RequestFailed(details.message));
            }
            return Err(ProviderError::RequestFailed(format!(
                "HTTP {}: {}",
                status.as_u16(),
                truncate_for_log(&body, 200)
            )));
        }

        // Process SSE stream
        let mut buffer = String::new();
        let mut chunks: Vec<ChatCompletionChunk> = Vec::new();

        let mut stream = resp.bytes_stream();
        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        buffer.push_str(&text);
                        let payloads = drain_complete_sse_payloads(&mut buffer);
                        for payload in payloads {
                            if payload == "[DONE]" {
                                continue;
                            }
                            if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(&payload) {
                                chunks.push(chunk);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "DeepSeek streaming error");
                    return Err(ProviderError::RequestFailed(e.to_string()));
                }
            }
        }

        tracing::info!(chunk_count = chunks.len(), "DeepSeek streaming completed");

        Ok(StreamingChat::Collected {
            status: 200,
            chunks,
        })
    }

    async fn responses(&self, _request: ResponsesRequest) -> Result<ResponsesResponse, ProviderError> {
        // DeepSeek doesn't have a native Responses API - use chat fallback
        Err(ProviderError::InvalidRequest(
            "DeepSeek: Native Responses API not supported, use chat fallback".to_string(),
        ))
    }

    async fn responses_streaming(&self, _request: ResponsesRequest) -> Result<StreamingResponses, ProviderError> {
        // DeepSeek doesn't have a native Responses API - use chat fallback
        Err(ProviderError::InvalidRequest(
            "DeepSeek: Native Responses API not supported, use chat fallback".to_string(),
        ))
    }

    async fn health_check(&self) -> bool {
        let url = self.build_url("/models");

        match self.add_auth_headers(self.client.get(&url))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{Message, ReasoningEffort};

    fn create_test_request(model: &str) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: Some("Test message".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                }
            ],
            temperature: Some(0.7),
            top_p: Some(0.9),
            thinking: None,
            reasoning_effort: None,
            ..Default::default()
        }
    }

    #[test]
    fn test_normalize_injects_thinking_enabled() {
        let request = create_test_request("deepseek-v4-flash");
        let normalized = normalize_deepseek_body(request);

        assert!(normalized.thinking.is_some());
        assert_eq!(normalized.thinking.as_ref().unwrap().type_, "enabled");
    }

    #[test]
    fn test_normalize_sets_reasoning_effort_high() {
        let request = create_test_request("deepseek-v4-flash");
        let normalized = normalize_deepseek_body(request);

        assert!(normalized.reasoning_effort.is_some());
        assert_eq!(*normalized.reasoning_effort.as_ref().unwrap(), ReasoningEffort::High);
    }

    #[test]
    fn test_normalize_removes_temperature_when_thinking_enabled() {
        let request = create_test_request("deepseek-v4-flash");
        let normalized = normalize_deepseek_body(request);

        assert!(normalized.temperature.is_none());
        assert!(normalized.top_p.is_none());
    }

    #[test]
    fn test_normalize_removes_reasoning_effort_when_disabled() {
        let mut request = create_test_request("deepseek-chat");
        request.thinking = Some(ChatThinkingConfig {
            type_: "disabled".to_string(),
        });
        request.reasoning_effort = Some(ReasoningEffort::Low);

        let normalized = normalize_deepseek_body(request);

        assert!(normalized.reasoning_effort.is_none());
    }

    #[test]
    fn test_normalize_preserves_reasoning_effort_when_enabled() {
        let mut request = create_test_request("deepseek-v4-flash");
        request.thinking = Some(ChatThinkingConfig {
            type_: "enabled".to_string(),
        });
        request.reasoning_effort = Some(ReasoningEffort::Low);

        let normalized = normalize_deepseek_body(request);

        assert_eq!(*normalized.reasoning_effort.as_ref().unwrap(), ReasoningEffort::Low);
    }

    #[test]
    fn test_normalize_strips_reasoning_content_for_legacy_r1() {
        let mut request = create_test_request("deepseek-reasoner");
        request.messages[0].reasoning_content = Some("Thinking process".to_string());

        let normalized = normalize_deepseek_body(request);

        assert!(normalized.messages[0].reasoning_content.is_none());
    }

    #[test]
    fn test_normalize_preserves_reasoning_content_for_v4() {
        let mut request = create_test_request("deepseek-v4-flash");
        request.messages[0].reasoning_content = Some("Thinking process".to_string());

        let normalized = normalize_deepseek_body(request);

        // V4 models should preserve reasoning_content
        assert!(normalized.messages[0].reasoning_content.is_some());
    }

    #[test]
    fn test_normalize_removes_penalties_when_thinking_enabled() {
        let mut request = create_test_request("deepseek-v4-flash");
        request.presence_penalty = Some(0.5);
        request.frequency_penalty = Some(0.5);

        let normalized = normalize_deepseek_body(request);

        assert!(normalized.presence_penalty.is_none());
        assert!(normalized.frequency_penalty.is_none());
    }

    #[test]
    fn test_builtin_models() {
        let models = DeepSeekProvider::builtin_models();
        assert!(!models.is_empty());
        assert_eq!(models.len(), 4);

        // Check V4 flash has the right aliases
        let v4_flash = models.iter().find(|m| m.id == "deepseek-v4-flash").unwrap();
        assert!(v4_flash.aliases.contains(&"deepseek-chat".to_string()));
        assert!(v4_flash.aliases.contains(&"deepseek-reasoner".to_string()));
    }

    #[test]
    fn test_normalize_model_resolves_aliases() {
        let provider = DeepSeekProvider::with_defaults("test-key");

        assert_eq!(provider.normalize_model("deepseek-chat"), "deepseek-v4-flash");
        assert_eq!(provider.normalize_model("deepseek-reasoner"), "deepseek-v4-flash");
        assert_eq!(provider.normalize_model("deepseek-v4-flash"), "deepseek-v4-flash");
        assert_eq!(provider.normalize_model("deepseek-v4-pro"), "deepseek-v4-pro");
    }
}
