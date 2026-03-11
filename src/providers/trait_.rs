//! LLM Provider trait definition
//!
//! Defines the interface for LLM providers.

use async_trait::async_trait;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatCompletionChunk, ChatRequest, ChatResponse};

use super::ProviderError;

/// LLM Provider trait
///
/// This trait defines the interface for LLM providers.
/// Each provider (OpenAI, Zhipu, etc.) implements this trait.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the provider configuration
    fn config(&self) -> &ProviderConfig;

    /// Get the HTTP client
    fn client(&self) -> &reqwest::Client;

    /// Chat completions (non-streaming)
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;

    /// Chat completions (streaming) - returns iterator of chunks
    async fn chat_streaming(&self, request: ChatRequest) -> Result<StreamingChat, ProviderError>;

    /// List available models (optional, returns empty vec if not supported)
    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec![])
    }

    /// Check if the provider is available
    async fn health_check(&self) -> bool;

    /// Build the full URL for the chat completions endpoint
    fn build_url(&self, path: &str) -> String {
        let base_url = self.config().base_url.trim_end_matches('/');
        format!("{}{}", base_url, path)
    }

    /// Build the authorization header
    fn build_auth_header(&self) -> String {
        format!("Bearer {}", self.config().api_key)
    }

    /// Convert provider error to app error
    fn to_app_error(&self, err: ProviderError) -> crate::error::Error {
        crate::error::Error::Provider(err.to_string())
    }
}

/// Streaming chat response wrapper
pub struct StreamingChat {
    /// HTTP status code
    pub status: u16,
    /// Stream iterator of chunks
    pub chunks: Vec<ChatCompletionChunk>,
}

impl StreamingChat {
    /// Create a new streaming response
    pub fn new(status: u16, chunks: Vec<ChatCompletionChunk>) -> Self {
        Self { status, chunks }
    }

    /// Check if the response is successful
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}
