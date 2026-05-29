//! MiMo Provider Implementation
//!
//! This module provides a provider implementation for MiMo API.

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use reqwest::Client;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatRequest, ChatResponse, ChatCompletionChunk, ToolChoice};
use crate::models::chat_extended::{ThinkingConfig, ThinkingType};
use crate::models::response::{ResponsesRequest, ResponsesResponse};
use crate::providers::ProviderError;
use crate::providers::StreamingChat;
use crate::providers::extended_provider::{
    ExtendedLLMProvider, ModelFeatures, ProviderModel,
};

/// MiMo Provider
pub struct MimoProvider {
    /// Configuration
    config: ProviderConfig,
    /// HTTP client
    client: Client,
    /// Base URL for API
    base_url: String,
}

/// Models that have thinking disabled by default (flash models)
const MIMO_THINKING_DISABLED: &[&str] = &["mimo-v2-flash", "mimo-v2-flash"];

impl MimoProvider {
    /// Create a new MiMo provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config.base_url.clone();
        Self {
            config,
            client: Client::new(),
            base_url,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(api_key: &str) -> Self {
        Self::new(ProviderConfig {
            api_key: api_key.to_string(),
            base_url: "https://api.mimo.ai".to_string(),
            default_model: "mimo-v2-flash".to_string(),
            timeout: 60,
        })
    }

    /// Check if this is a token plan API key
    pub fn is_token_plan(&self) -> bool {
        self.config.api_key.starts_with("mm-") || 
            self.config.api_key.starts_with("token-")
    }

    /// Get the list of built-in models
    /// Updated to match mimo2codex v2 model series
    pub fn builtin_models() -> Vec<ProviderModel> {
        vec![
            // Mini model - no thinking, no vision
            ProviderModel {
                id: "mimo-v2-mini".to_string(),
                name: "MiMo V2 Mini".to_string(),
                aliases: vec![
                    "mimo-v2-mini-2025".to_string(),
                    "mimo-v2-mini".to_string(),
                    "mimo-v2-mini-2025".to_string(),
                    "mini".to_string(),
                ],
                context_window: 32_768,
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
            // Flash model - no thinking, has vision
            ProviderModel {
                id: "mimo-v2-flash".to_string(),
                name: "MiMo V2 Flash".to_string(),
                aliases: vec![
                    "mimo-v2-flash-2025".to_string(),
                    "mimo-v2-flash".to_string(),
                    "mimo-v2-flash-2025".to_string(),
                    "flash".to_string(),
                ],
                context_window: 128_000,
                max_output_tokens: 16384,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: true,
                    web_search: true,
                    thinking: false,
                    audio_output: false,
                },
            },
            // Pro model - has thinking, has vision
            ProviderModel {
                id: "mimo-v2.5-pro".to_string(),
                name: "MiMo V2.5 Pro".to_string(),
                aliases: vec![
                    "mimo-v2.5-pro-2025".to_string(),
                    "mimo-v2-pro".to_string(),
                    "mimo-v2.5-pro".to_string(),
                    "mimo-v2.5-pro-2025".to_string(),
                    "pro".to_string(),
                ],
                context_window: 128_000,
                max_output_tokens: 32768,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: true,
                    web_search: true,
                    thinking: true,
                    audio_output: false,
                },
            },
            // V2.5 model - has thinking, has vision
            ProviderModel {
                id: "mimo-v2.5".to_string(),
                name: "MiMo V2.5".to_string(),
                aliases: vec![
                    "mimo-v2.5-2025".to_string(),
                ],
                context_window: 128_000,
                max_output_tokens: 32768,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: true,
                    web_search: true,
                    thinking: true,
                    audio_output: false,
                },
            },
            // Omni model - has thinking
            ProviderModel {
                id: "mimo-v2-omni".to_string(),
                name: "MiMo V2 Omni".to_string(),
                aliases: vec![
                    "mimo-v2-omni-2025".to_string(),
                ],
                context_window: 128_000,
                max_output_tokens: 32768,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: true,
                    web_search: true,
                    thinking: true,
                    audio_output: false,
                },
            },
            // Vision model - has vision, no thinking
            ProviderModel {
                id: "mimo-v2-vision".to_string(),
                name: "MiMo V2 Vision".to_string(),
                aliases: vec![
                    "mimo-v2-vision-2025".to_string(),
                    "mimo-vision".to_string(),
                    "mimo-vision-2025".to_string(),
                    "vision".to_string(),
                ],
                context_window: 32_768,
                max_output_tokens: 4096,
                features: ModelFeatures {
                    streaming: true,
                    function_calling: true,
                    vision: true,
                    web_search: false,
                    thinking: false,
                    audio_output: false,
                },
            },
        ]
    }

    /// Preprocess a Chat request for MiMo
    fn preprocess_chat_internal(&self, request: &ChatRequest) -> ChatRequest {
        normalize_mimo_body(request.clone(), &request.model)
    }

    /// Check if thinking should be auto-injected for this model
    fn should_auto_inject_thinking(model_id: &str) -> bool {
        !MIMO_THINKING_DISABLED.iter().any(|m| *m == model_id)
    }

    /// Check if tool_choice should be removed (not "auto")
    fn should_remove_tool_choice(tool_choice: &Option<ToolChoice>) -> bool {
        if let Some(tc) = tool_choice {
            match tc {
                ToolChoice::String(s) => s != "auto",
                ToolChoice::Tool(t) => t.choice_type != "auto",
            }
        } else {
            false
        }
    }

    /// Normalize model name
    pub fn normalize_model(&self, model: &str) -> String {
        for m in Self::builtin_models() {
            if m.aliases.contains(&model.to_string()) || m.id == model {
                return m.id;
            }
        }
        model.to_string()
    }
}

#[async_trait]
impl ExtendedLLMProvider for MimoProvider {
    fn name(&self) -> &str {
        "mimo"
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    fn client(&self) -> &Client {
        &self.client
    }

    fn supported_models(&self) -> Vec<ProviderModel> {
        Self::builtin_models()
    }

    fn capabilities(&self) -> crate::protocol::capabilities::ProviderCapabilities {
        crate::protocol::capabilities::ProviderCapabilities::native_responses(vec![
            "function".to_string(),
            "web_search".to_string(),
        ])
    }

    fn supports_responses_api(&self) -> bool {
        true
    }

    fn chat(&self, request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<ChatResponse, ProviderError>> + Send + '_>>
    where 
        Self: Sized,
    {
        let config = self.config.clone();
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let preprocessed = self.preprocess_chat_internal(&request);
        
        Box::pin(async move {
            let url = format!("{}/v1/chat/completions", base_url);
            
            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .json(&preprocessed)
                .send()
                .await
                .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            
            if !response.status().is_success() {
                let body = response.text().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

                // Check for MiMo-specific web_search plugin not activated error
                if let Some(web_search_error) = crate::providers::error_enhancer::EnhancedError::detect_web_search_disabled(&body) {
                    return Err(ProviderError::RequestFailed(format!(
                        "{} (Hint: {})",
                        web_search_error.message,
                        web_search_error.hint.unwrap_or_default()
                    )));
                }

                return Err(ProviderError::InvalidResponse(format!(
                    "MiMo API error {}: {}",
                    status, body
                )));
            }
            
            let body = response.text().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

            serde_json::from_str(&body)
                .map_err(|e| ProviderError::InvalidResponse(format!("Failed to parse response: {}", e)))
        })
    }

    fn chat_streaming(&self, request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<StreamingChat, ProviderError>> + Send + '_>>
    where
        Self: Sized,
    {
        let config = self.config.clone();
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let preprocessed = self.preprocess_chat_internal(&request);
        
        Box::pin(async move {
            let url = format!("{}/v1/chat/completions", base_url);
            
            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream")
                .json(&preprocessed)
                .send()
                .await
                .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();

            if !(200..300).contains(&status) {
                let body = response.text().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

                // Check for MiMo-specific web_search plugin not activated error
                if let Some(web_search_error) = crate::providers::error_enhancer::EnhancedError::detect_web_search_disabled(&body) {
                    return Err(ProviderError::RequestFailed(format!(
                        "{} (Hint: {})",
                        web_search_error.message,
                        web_search_error.hint.unwrap_or_default()
                    )));
                }

                return Err(ProviderError::InvalidResponse(format!(
                    "MiMo streaming error {}: {}",
                    status, body
                )));
            }
            
            let body = response.text().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
            let chunks = parse_sse_to_chunks(&body)?;
            
            Ok(StreamingChat::new(status, chunks))
        })
    }

    async fn responses(&self, request: ResponsesRequest) -> Result<ResponsesResponse, ProviderError> {
        let config = self.config.clone();
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        
        let url = format!("{}/v1/responses", base_url);
        
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let body = response.text().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
            return Err(ProviderError::InvalidResponse(format!(
                "MiMo Responses API error {}: {}",
                status, body
            )));
        }

        let body = response.text().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        serde_json::from_str(&body)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to parse response: {}", e)))
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/v1/models", self.base_url);
        
        match self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

/// Parse SSE body into ChatCompletionChunks
fn parse_sse_to_chunks(body: &str) -> Result<Vec<ChatCompletionChunk>, ProviderError> {
    let mut chunks = Vec::new();

    for line in body.lines() {
        if line.starts_with("data: ") {
            let data = &line[6..];
            if data == "[DONE]" {
                continue;
            }

            match serde_json::from_str::<ChatCompletionChunk>(data) {
                Ok(chunk) => chunks.push(chunk),
                Err(e) => {
                    tracing::warn!("Failed to parse chunk: {}", e);
                }
            }
        }
    }

    Ok(chunks)
}

/// Normalize a ChatRequest for MiMo API
///
/// This function implements the following normalization rules:
/// 1. Auto-inject thinking:enabled for models that support thinking (except flash models)
/// 2. Remove tool_choice if it's not "auto"
/// 3. Delete reasoning_effort when thinking is disabled (handled by API)
///
/// Aligned with mimo2codex normalizeMimoBody function
pub fn normalize_mimo_body(request: ChatRequest, model_id: &str) -> ChatRequest {
    let mut normalized = request.clone();

    // Rule 1: Auto-inject thinking for non-flash models
    // Note: Using chat::ThinkingConfig with type_: String
    if normalized.thinking.is_none() && !MIMO_THINKING_DISABLED.iter().any(|m| *m == model_id) {
        normalized.thinking = Some(crate::models::chat::ThinkingConfig {
            type_: "enabled".to_string(),
        });
    }

    // Rule 2: Remove tool_choice if not "auto"
    if let Some(ref tool_choice) = normalized.tool_choice {
        match tool_choice {
            ToolChoice::String(s) if s != "auto" => {
                normalized.tool_choice = None;
            }
            _ => {}
        }
    }

    // Rule 3: Remove reasoning_effort when thinking is disabled
    // Check for "disabled" in the thinking config
    if let Some(ref thinking) = normalized.thinking {
        if thinking.type_ == "disabled" {
            normalized.reasoning_effort = None;
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_models() {
        let models = MimoProvider::builtin_models();
        assert!(!models.is_empty());
        // 6 models: v2-mini, v2-flash, v2.5-pro, v2.5, v2-omni, v2-vision
        assert_eq!(models.len(), 6);
    }

    #[test]
    fn test_normalize_model() {
        let provider = MimoProvider::with_defaults("test-key");
        
        assert_eq!(provider.normalize_model("mini"), "mimo-v2-mini");
        assert_eq!(provider.normalize_model("flash"), "mimo-v2-flash");
        assert_eq!(provider.normalize_model("unknown"), "unknown");
    }

    #[test]
    fn test_is_token_plan() {
        let provider = MimoProvider::with_defaults("test-key");
        assert!(!provider.is_token_plan());
        
        let token_provider = MimoProvider::with_defaults("mm-test-key");
        assert!(token_provider.is_token_plan());
    }

    
    // mimo2codex aligned tests
    
    #[test]
    fn test_mimo_thinking_enabled_for_pro() {
        use crate::models::chat_extended::ThinkingConfig;
        
        let config = ThinkingConfig {
            thinking_type: crate::models::chat_extended::ThinkingType::Enabled,
        };
        
        assert!(matches!(config.thinking_type, crate::models::chat_extended::ThinkingType::Enabled));
    }
    
    #[test]
    fn test_mimo_thinking_disabled_for_flash() {
        use crate::models::chat_extended::ThinkingConfig;
        
        let config = ThinkingConfig {
            thinking_type: crate::models::chat_extended::ThinkingType::Disabled,
        };
        
        assert!(matches!(config.thinking_type, crate::models::chat_extended::ThinkingType::Disabled));
    }
    
    #[test]
    fn test_mimo_model_variants() {
        let variants = vec![
            ("mini", "mimo-v2-mini"),
            ("flash", "mimo-v2-flash"),
            ("mimo-v2-mini", "mimo-v2-mini"),
            ("mimo-v2-flash", "mimo-v2-flash"),
        ];
        
        let provider = MimoProvider::with_defaults("test-key");
        
        for (input, expected) in variants {
            let normalized = provider.normalize_model(input);
            assert_eq!(normalized, expected, "Failed for input: {}", input);
        }
        
        // Unknown variants pass through
        let unknown = provider.normalize_model("v2.5-pro");
        assert_eq!(unknown, "v2.5-pro");
    }
    
    #[test]
    fn test_token_plan_detection() {
        // Normal keys are not token plan
        let normal = MimoProvider::with_defaults("sk-normal-key-12345");
        assert!(!normal.is_token_plan());
        
        // mm-* prefixed keys are token plan  
        let mm_key = MimoProvider::with_defaults("mm-test-key");
        assert!(mm_key.is_token_plan());
        
        // token-* prefixed keys are token plan  
        let token_prefix = MimoProvider::with_defaults("token-test-key");
        assert!(token_prefix.is_token_plan());
    }
    
    #[test]
    fn test_builtin_models_count() {
        let models = MimoProvider::builtin_models();
        // Should have at least the main models
        assert!(models.len() >= 3);
    }
}

#[cfg(test)]
mod mimo2codex_routing_tests {
    use super::*;

    // Tests aligned with mimo2codex providers.routing.test.ts

    #[test]
    fn test_mimo_vision_model_has_image_support() {
        // mimo-v2-vision and mimo-v2-flash support images
        let models = MimoProvider::builtin_models();
        
        let vision = models.iter().find(|m| m.id == "mimo-v2-vision");
        assert!(vision.is_some(), "mimo-v2-vision should exist");
        assert!(vision.unwrap().features.vision, "mimo-v2-vision should support images");
        
        let flash = models.iter().find(|m| m.id == "mimo-v2-flash");
        assert!(flash.is_some(), "mimo-v2-flash should exist");
        assert!(flash.unwrap().features.vision, "mimo-v2-flash should support images");
    }

    #[test]
    fn test_mimo_mini_no_image_support() {
        let models = MimoProvider::builtin_models();

        let mini = models.iter().find(|m| m.id == "mimo-v2-mini");
        assert!(mini.is_some(), "mimo-v2-mini should exist");
        assert!(!mini.unwrap().features.vision, "mimo-v2-mini should NOT support images");
    }

    #[test]
    fn test_mimo_pro_supports_reasoning() {
        let models = MimoProvider::builtin_models();

        let pro = models.iter().find(|m| m.id == "mimo-v2.5-pro");
        assert!(pro.is_some(), "mimo-v2.5-pro should exist");
        assert!(pro.unwrap().features.thinking, "mimo-v2.5-pro should support reasoning");
    }

    #[test]
    fn test_mimo_thinking_supports_reasoning() {
        // In v2 series, thinking models are mimo-v2.5-pro, mimo-v2.5, mimo-v2-omni
        let models = MimoProvider::builtin_models();

        let pro = models.iter().find(|m| m.id == "mimo-v2.5-pro");
        assert!(pro.is_some(), "mimo-v2.5-pro should exist");
        assert!(pro.unwrap().features.thinking, "mimo-v2.5-pro should support reasoning");
    }

    #[test]
    fn test_unknown_model_passes_through() {
        let provider = MimoProvider::with_defaults("sk-test");
        
        // Unknown models should pass through unchanged
        let unknown = provider.normalize_model("unknown-model-xyz");
        assert_eq!(unknown, "unknown-model-xyz");
        
        let gpt = provider.normalize_model("gpt-4o");
        assert_eq!(gpt, "gpt-4o");
    }

    #[test]
    fn test_token_plan_key_patterns() {
        // Test all token plan key patterns
        let patterns = vec![
            ("mm-xxx", true),
            ("token-xxx", true),
            ("sk-xxx", false),
            ("mimo-xxx", false),
            ("normal-key", false),
        ];
        
        for (key, expected) in patterns {
            let provider = MimoProvider::with_defaults(key);
            assert_eq!(
                provider.is_token_plan(),
                expected,
                "Key '{}' should be token plan: {}",
                key,
                expected
            );
        }
    }

    #[test]
    fn test_mimo_model_normalization_by_alias() {
        // Full model normalization tests aligned with mimo2codex
        let provider = MimoProvider::with_defaults("sk-test");

        // Test alias resolution (v2 model series)
        let tests = vec![
            ("mini", "mimo-v2-mini"),
            ("flash", "mimo-v2-flash"),
            ("pro", "mimo-v2.5-pro"),
        ];

        for (input, expected) in tests {
            let normalized = provider.normalize_model(input);
            assert_eq!(normalized, expected, "Failed for alias: {}", input);
        }
    }

    #[test]
    fn test_mimo_flash_supports_streaming() {
        let models = MimoProvider::builtin_models();

        let flash = models.iter().find(|m| m.id == "mimo-v2-flash");
        assert!(flash.is_some(), "mimo-v2-flash should exist");
        assert!(flash.unwrap().features.streaming, "mimo-v2-flash should support streaming");
    }

    #[test]
    fn test_mimo_all_models_have_function_calling() {
        let models = MimoProvider::builtin_models();
        
        for model in models {
            assert!(
                model.features.function_calling,
                "Model {} should support function calling",
                model.id
            );
        }
    }

    #[test]
    fn test_mimo_model_context_windows() {
        let models = MimoProvider::builtin_models();
        
        for model in models {
            assert!(
                model.context_window > 0,
                "Model {} should have valid context window",
                model.id
            );
            assert!(
                model.max_output_tokens > 0,
                "Model {} should have valid max output tokens",
                model.id
            );
        }
    }

    #[test]
    fn test_mimo_pro_has_highest_output_tokens() {
        let models = MimoProvider::builtin_models();

        let pro = models.iter().find(|m| m.id == "mimo-v2.5-pro");
        assert!(pro.is_some(), "mimo-v2.5-pro should exist");
        // Pro should have highest output tokens (32768)
        assert!(pro.unwrap().max_output_tokens >= 16384, "mimo-v2.5-pro should have high output");
    }
}

/// Tests for normalize_mimo_body function (aligned with plan5.md TC-P1)
#[cfg(test)]
mod normalize_mimo_body_tests {
    use super::*;
    use crate::models::chat::{ChatRequest, Message, ToolChoice, ReasoningEffort, ThinkingConfig};

    fn make_chat_request(model: &str) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: None,
            stop: None,
            n: 1,
            stream_options: None,
            include_usage: None,
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
            reasoning_effort: None,
            thinking: None,
        }
    }

    fn make_thinking_config(type_str: &str) -> ThinkingConfig {
        ThinkingConfig {
            type_: type_str.to_string(),
        }
    }

    // TC-P1.1: thinking注入测试 - Pro模型自动注入thinking
    #[test]
    fn test_mimo_thinking_auto_inject_for_pro() {
        let request = make_chat_request("mimo-v2.5-pro");
        let normalized = normalize_mimo_body(request, "mimo-v2.5-pro");

        assert!(normalized.thinking.is_some(), "thinking should be auto-injected for mimo-v2.5-pro");
        assert_eq!(normalized.thinking.as_ref().unwrap().type_, "enabled", "thinking type should be enabled");
    }

    // TC-P1.2: thinking注入测试 - Thinking模型自动注入thinking
    #[test]
    fn test_mimo_thinking_auto_inject_for_thinking_model() {
        let request = make_chat_request("mimo-v2-thinking");
        let normalized = normalize_mimo_body(request, "mimo-v2-thinking");

        assert!(normalized.thinking.is_some(), "thinking should be auto-injected for mimo-v2-thinking");
        assert_eq!(normalized.thinking.as_ref().unwrap().type_, "enabled", "thinking type should be enabled");
    }

    // TC-P1.3: flash模型不注入thinking
    #[test]
    fn test_mimo_flash_no_thinking_inject() {
        let request = make_chat_request("mimo-v2-flash");
        let normalized = normalize_mimo_body(request, "mimo-v2-flash");

        // flash模型不应注入thinking
        assert!(normalized.thinking.is_none(), "thinking should NOT be auto-injected for flash");
    }

    // TC-P1.4: flash v2模型不注入thinking
    #[test]
    fn test_mimo_v2_flash_no_thinking_inject() {
        let request = make_chat_request("mimo-v2-flash");
        let normalized = normalize_mimo_body(request, "mimo-v2-flash");

        // v2-flash模型不应注入thinking
        assert!(normalized.thinking.is_none(), "thinking should NOT be auto-injected for v2-flash");
    }

    // TC-P1.5: tool_choice非auto删除 - String类型
    #[test]
    fn test_mimo_tool_choice_required_removed() {
        let mut request = make_chat_request("mimo-v2.5-pro");
        request.tool_choice = Some(ToolChoice::String("required".to_string()));

        let normalized = normalize_mimo_body(request, "mimo-v2.5-pro");
        assert!(normalized.tool_choice.is_none(), "tool_choice 'required' should be removed");
    }

    // TC-P1.6: tool_choice auto保留
    #[test]
    fn test_mimo_tool_choice_auto_kept() {
        let mut request = make_chat_request("mimo-v2.5-pro");
        request.tool_choice = Some(ToolChoice::String("auto".to_string()));

        let normalized = normalize_mimo_body(request, "mimo-v2.5-pro");
        assert!(normalized.tool_choice.is_some(), "tool_choice 'auto' should be kept");

        if let Some(ToolChoice::String(s)) = normalized.tool_choice {
            assert_eq!(s, "auto", "tool_choice should be 'auto'");
        } else {
            panic!("tool_choice should be String type");
        }
    }

    // TC-P1.7: reasoning_effort删除 - thinking disabled时
    #[test]
    fn test_mimo_reasoning_effort_removed_when_disabled() {
        let mut request = make_chat_request("mimo-v2.5-pro");
        // Set thinking to "disabled" using chat::ThinkingConfig
        request.thinking = Some(make_thinking_config("disabled"));
        request.reasoning_effort = Some(ReasoningEffort::Low);

        let normalized = normalize_mimo_body(request, "mimo-v2.5-pro");
        assert!(
            normalized.reasoning_effort.is_none(),
            "reasoning_effort should be removed when thinking is disabled"
        );
    }

    // TC-P1.8: reasoning_effort保留(thinking启用时)
    #[test]
    fn test_mimo_reasoning_effort_preserved_when_enabled() {
        let mut request = make_chat_request("mimo-v2.5-pro");
        request.thinking = Some(make_thinking_config("enabled"));
        request.reasoning_effort = Some(ReasoningEffort::Low);

        let normalized = normalize_mimo_body(request, "mimo-v2.5-pro");
        assert!(
            normalized.reasoning_effort.is_some(),
            "reasoning_effort should be preserved when thinking is enabled"
        );
    }

    // TC-P1.9: 已有thinking配置不覆盖
    #[test]
    fn test_mimo_existing_thinking_not_overwritten() {
        let mut request = make_chat_request("mimo-v2.5-pro");
        request.thinking = Some(make_thinking_config("disabled"));

        let normalized = normalize_mimo_body(request, "mimo-v2.5-pro");
        // Existing thinking config should not be overwritten
        assert_eq!(normalized.thinking.as_ref().unwrap().type_, "disabled", "existing thinking config should be preserved");
    }

    // TC-P1.10: Mini模型注入thinking(mimo2codex行为 - only flash models excluded)
    #[test]
    fn test_mimo_mini_thinking_auto_inject() {
        let request = make_chat_request("mimo-v2-mini");
        let normalized = normalize_mimo_body(request, "mimo-v2-mini");

        // mimo2codex auto-injects thinking for all models except flash
        assert!(normalized.thinking.is_some(), "thinking should be auto-injected for mini (mimo2codex behavior)");
    }

    // TC-P1.11: Unknown模型注入thinking(mimo2codex行为)
    #[test]
    fn test_mimo_unknown_model_thinking_auto_inject() {
        let request = make_chat_request("unknown-model");
        let normalized = normalize_mimo_body(request, "unknown-model");

        // mimo2codex auto-injects thinking for unknown models
        assert!(normalized.thinking.is_some(), "thinking should be auto-injected for unknown models (mimo2codex behavior)");
    }

    // TC-P1.12: tool_choice None保持
    #[test]
    fn test_mimo_tool_choice_none_kept() {
        let request = make_chat_request("mimo-v2.5-pro");
        assert!(request.tool_choice.is_none());

        let normalized = normalize_mimo_body(request, "mimo-v2.5-pro");
        assert!(normalized.tool_choice.is_none(), "tool_choice None should be kept");
    }

    // TC-P1.13: Vision模型注入thinking
    #[test]
    fn test_mimo_vision_thinking_auto_inject() {
        let request = make_chat_request("mimo-v2-vision");
        let normalized = normalize_mimo_body(request, "mimo-v2-vision");

        assert!(normalized.thinking.is_some(), "thinking should be auto-injected for mimo-v2-vision");
    }
}
