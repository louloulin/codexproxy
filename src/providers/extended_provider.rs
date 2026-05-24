//! Extended LLM Provider Trait
//! 
//! This module defines the extended provider trait with preprocessing
//! capabilities and error enhancement.

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatRequest, ChatResponse, ChatCompletionChunk};
use crate::models::response::{ResponsesRequest, ResponsesResponse};
use crate::protocol::capabilities::ProviderCapabilities;
use crate::providers::ProviderError;
use crate::providers::error_enhancer::EnhancedError;

/// Provider model information
#[derive(Debug, Clone)]
pub struct ProviderModel {
    /// Model ID (e.g., "gpt-4", "glm-4-flash")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Supported aliases (e.g., "gpt-4-turbo" -> "gpt-4")
    pub aliases: Vec<String>,
    /// Context window size in tokens
    pub context_window: u32,
    /// Maximum output tokens
    pub max_output_tokens: u32,
    /// Supported features
    pub features: ModelFeatures,
}

/// Supported model features
#[derive(Debug, Clone, Default)]
pub struct ModelFeatures {
    /// Supports streaming
    pub streaming: bool,
    /// Supports function calling
    pub function_calling: bool,
    /// Supports vision/images
    pub vision: bool,
    /// Supports web search
    pub web_search: bool,
    /// Supports thinking/reasoning
    pub thinking: bool,
    /// Supports audio output
    pub audio_output: bool,
}

/// Extended LLM Provider trait
/// 
/// This extends the base LLMProvider with preprocessing, postprocessing,
/// and error enhancement capabilities.
#[async_trait]
pub trait ExtendedLLMProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the provider configuration
    fn config(&self) -> &ProviderConfig;

    /// Get the HTTP client
    fn client(&self) -> &reqwest::Client;

    /// Get the list of supported models
    fn supported_models(&self) -> Vec<ProviderModel>;

    /// Check if this provider supports a specific model
    fn supports_model(&self, model: &str) -> bool {
        let models = self.supported_models();
        models.iter().any(|m| m.id == model || m.aliases.iter().any(|a| a == model))
    }

    /// Get model info for a specific model
    fn get_model(&self, model: &str) -> Option<ProviderModel> {
        self.supported_models()
            .iter()
            .find(|m| m.id == model || m.aliases.iter().any(|a| a == model))
            .cloned()
    }

    /// Preprocess a Responses request before sending to the provider
    fn preprocess_responses(&self, request: &ResponsesRequest) -> ResponsesRequest {
        // Default implementation: no preprocessing
        request.clone()
    }

    /// Preprocess a Chat request before sending to the provider
    fn preprocess_chat(&self, request: &ChatRequest) -> ChatRequest {
        // Default implementation: no preprocessing
        request.clone()
    }

    /// Postprocess a Responses response
    fn postprocess_responses(&self, response: &ResponsesResponse) -> ResponsesResponse {
        // Default implementation: no postprocessing
        response.clone()
    }

    /// Postprocess a Chat response
    fn postprocess_chat(&self, response: &ChatResponse) -> ChatResponse {
        // Default implementation: no postprocessing
        response.clone()
    }

    /// Enhance an error with provider-specific information
    fn enhance_error(&self, error: ProviderError, request: Option<&dyn std::any::Any>) -> EnhancedError {
        EnhancedError::from_provider_error(error, self.name())
    }

    /// Detect runtime flags from a response
    fn detect_flags(&self, response: &ResponsesResponse) -> RuntimeFlags {
        // Default implementation: no flags detected
        RuntimeFlags::default()
    }

    /// Detect runtime flags from a Chat response
    fn detect_flags_chat(&self, response: &ChatResponse) -> RuntimeFlags {
        // Default implementation: no flags detected
        RuntimeFlags::default()
    }

    // Delegate to base LLMProvider methods
    /// Chat completions (non-streaming)
    fn chat(&self, request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<ChatResponse, ProviderError>> + Send + '_>>
    where 
        Self: Sized;

    /// Chat completions (streaming)
    fn chat_streaming(&self, request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<crate::providers::StreamingChat, ProviderError>> + Send + '_>>
    where
        Self: Sized;

    /// Native Responses API
    #[allow(unused_variables)]
    async fn responses(&self, request: ResponsesRequest) -> Result<ResponsesResponse, ProviderError> {
        Err(ProviderError::InvalidRequest(
            "Responses API is not supported by this provider".to_string(),
        ))
    }

    /// Native Responses API streaming
    #[allow(unused_variables)]
    async fn responses_streaming(&self, request: ResponsesRequest) -> Result<crate::providers::StreamingResponses, ProviderError> {
        Err(ProviderError::InvalidRequest(
            "Streaming Responses API is not supported by this provider".to_string(),
        ))
    }

    /// Describe which Responses features this provider preserves
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::chat_fallback(vec!["function".to_string()])
    }

    /// Whether this provider supports the native Responses API
    fn supports_responses_api(&self) -> bool {
        self.capabilities().native_responses
    }

    /// Build the full URL for an endpoint
    fn build_url(&self, path: &str) -> String {
        let base_url = self.config().base_url.trim_end_matches('/');
        format!("{}{}", base_url, path)
    }

    /// Build the authorization header
    fn build_auth_header(&self) -> String {
        format!("Bearer {}", self.config().api_key)
    }

    /// Health check
    async fn health_check(&self) -> bool;
}

/// Runtime flags detected from provider responses
#[derive(Debug, Clone, Default)]
pub struct RuntimeFlags {
    /// Whether context might be overflowed
    pub context_overflow_risk: bool,
    /// Whether the response was truncated
    pub truncated: bool,
    /// Whether content was filtered
    pub content_filtered: bool,
    /// Whether web search was used
    pub web_search_used: bool,
    /// Custom flags
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

impl RuntimeFlags {
    /// Create new runtime flags
    pub fn new() -> Self {
        Self::default()
    }

    /// Set context overflow risk
    pub fn with_context_risk(mut self, risk: bool) -> Self {
        self.context_overflow_risk = risk;
        self
    }

    /// Set truncation flag
    pub fn with_truncated(mut self, truncated: bool) -> Self {
        self.truncated = truncated;
        self
    }

    /// Set content filter flag
    pub fn with_filtered(mut self, filtered: bool) -> Self {
        self.content_filtered = filtered;
        self
    }

    /// Set web search flag
    pub fn with_web_search(mut self, used: bool) -> Self {
        self.web_search_used = used;
        self
    }

    /// Add a custom flag
    pub fn with_custom(mut self, key: &str, value: serde_json::Value) -> Self {
        self.custom.insert(key.to_string(), value);
        self
    }

    /// Check if any warning flags are set
    pub fn has_warnings(&self) -> bool {
        self.context_overflow_risk || self.truncated || self.content_filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_flags() {
        let flags = RuntimeFlags::new()
            .with_context_risk(true)
            .with_web_search(true);
        
        assert!(flags.context_overflow_risk);
        assert!(flags.web_search_used);
        assert!(flags.has_warnings());
    }

    #[test]
    fn test_model_features_default() {
        let features = ModelFeatures::default();
        assert!(!features.streaming);
        assert!(!features.function_calling);
    }
}
