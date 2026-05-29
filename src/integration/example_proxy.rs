//! Codex Proxy Example
//! 
//! This module demonstrates how to use the new architecture
//! to implement a Codex proxy that converts between
//! Responses API and Chat Completions API.

use std::sync::Arc;

use crate::models::chat::{ChatRequest, ChatResponse, ChatCompletionChunk};
use crate::models::response::{ResponsesRequest, ResponsesObject};
use crate::providers::{
    ProviderRegistry, MimoProvider, ExtendedLLMProvider, ErrorEnhancer, EnhancedError,
};
use crate::transform::{req_to_chat, chat_to_responses};
use crate::streaming_new::SseEventBuilder;

/// A complete Codex proxy implementation using the new architecture
pub struct CodexProxy {
    /// Provider registry for routing
    registry: ProviderRegistry,
}

impl CodexProxy {
    /// Create a new Codex proxy
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
        }
    }

    /// Register the default MiMo provider
    pub fn with_mimo(mut self, api_key: &str) -> Self {
        let mimo = Arc::new(MimoProvider::with_defaults(api_key));
        self.registry.register(mimo);
        self.registry.set_default("mimo");
        self
    }

    /// Get the provider registry for inspection
    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }

    /// List available models
    pub fn available_models(&self) -> Vec<String> {
        self.registry.all_models()
            .iter()
            .map(|m| m.id.clone())
            .collect()
    }

    /// Check if a model is supported
    pub fn supports_model(&self, model: &str) -> bool {
        self.registry.supports_model(model)
    }

    /// Get model info for a specific model
    pub fn get_model_info(&self, model: &str) -> Option<crate::providers::ProviderModel> {
        self.registry.resolve(model).map(|(_, m)| m.clone())
    }

    /// Process a streaming request and return SSE events
    pub fn handle_streaming_request(
        &self,
        chunks: Vec<ChatCompletionChunk>,
        response_id: String,
        model: String,
    ) -> Vec<String> {
        let mut builder = SseEventBuilder::new(response_id, model);
        let mut events = Vec::new();
        
        for chunk in chunks {
            let chunk_events = builder.process_chunk(&chunk);
            for event in chunk_events {
                events.push(event.to_sse_string());
            }
        }
        
        // Add done events
        for event in builder.generate_done_events() {
            events.push(event.to_sse_string());
        }
        
        events
    }
}

impl Default for CodexProxy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_creation() {
        let proxy = CodexProxy::new()
            .with_mimo("test-api-key");

        assert!(proxy.supports_model("mimo-v2-mini"));
        assert!(proxy.supports_model("mimo-v2-pro"));
        assert!(proxy.supports_model("mini")); // alias
    }

    #[test]
    fn test_available_models() {
        let proxy = CodexProxy::new()
            .with_mimo("test-api-key");

        let models = proxy.available_models();
        assert!(!models.is_empty());
        assert!(models.contains(&"mimo-v2-mini".to_string()));
    }

    #[test]
    fn test_model_info() {
        let proxy = CodexProxy::new()
            .with_mimo("test-api-key");

        let info = proxy.get_model_info("mimo-v2-mini");
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.id, "mimo-v2-mini");
        assert!(info.features.streaming);
        assert!(info.features.function_calling);
    }
}
