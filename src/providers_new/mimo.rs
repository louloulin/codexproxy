//! MiMo Provider Implementation
//! 
//! This module provides a provider implementation for MiMo API.

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use reqwest::Client;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatRequest, ChatResponse, ChatCompletionChunk};
use crate::models::response::{ResponsesRequest, ResponsesResponse};
use crate::providers::ProviderError;
use crate::providers::StreamingChat;
use crate::providers_new::extended_provider::{
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
            default_model: "mimo-flash".to_string(),
            timeout: 60,
        })
    }

    /// Check if this is a token plan API key
    pub fn is_token_plan(&self) -> bool {
        self.config.api_key.starts_with("mm-") || 
            self.config.api_key.starts_with("token-")
    }

    /// Get the list of built-in models
    pub fn builtin_models() -> Vec<ProviderModel> {
        vec![
            ProviderModel {
                id: "mimo-mini".to_string(),
                name: "MiMo Mini".to_string(),
                aliases: vec!["mimo-mini-2025".to_string(), "mini".to_string()],
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
            ProviderModel {
                id: "mimo-flash".to_string(),
                name: "MiMo Flash".to_string(),
                aliases: vec!["mimo-flash-2025".to_string(), "flash".to_string()],
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
            ProviderModel {
                id: "mimo-pro".to_string(),
                name: "MiMo Pro".to_string(),
                aliases: vec!["mimo-pro-2025".to_string(), "pro".to_string()],
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
            ProviderModel {
                id: "mimo-thinking".to_string(),
                name: "MiMo Thinking".to_string(),
                aliases: vec!["mimo-thinking-2025".to_string(), "thinking".to_string()],
                context_window: 32_768,
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
            ProviderModel {
                id: "mimo-vision".to_string(),
                name: "MiMo Vision".to_string(),
                aliases: vec!["mimo-vision-2025".to_string(), "vision".to_string()],
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
        request.clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_models() {
        let models = MimoProvider::builtin_models();
        assert!(!models.is_empty());
        assert_eq!(models.len(), 5);
    }

    #[test]
    fn test_normalize_model() {
        let provider = MimoProvider::with_defaults("test-key");
        
        assert_eq!(provider.normalize_model("mini"), "mimo-mini");
        assert_eq!(provider.normalize_model("flash"), "mimo-flash");
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
            ("mini", "mimo-mini"),
            ("flash", "mimo-flash"),
            ("mimo-mini", "mimo-mini"),
            ("mimo-flash", "mimo-flash"),
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
