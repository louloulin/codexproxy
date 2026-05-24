//! LLM Provider trait definition
//!
//! Defines the interface for LLM providers.

use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatCompletionChunk, ChatRequest, ChatResponse};
use crate::models::response::{ResponsesRequest, ResponsesResponse};
use crate::protocol::capabilities::ProviderCapabilities;

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

    /// Describe which Responses features this provider preserves.
    ///
    /// Handlers use this to decide whether a request can stay on the native
    /// Responses path, requires a lossy Chat fallback, or should be rejected
    /// explicitly before any provider call happens.
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::chat_fallback(vec!["function".to_string()])
    }

    /// Whether this provider supports the native Responses API.
    fn supports_responses_api(&self) -> bool {
        self.capabilities().native_responses
    }

    /// Native Responses API (non-streaming)
    async fn responses(
        &self,
        _request: ResponsesRequest,
    ) -> Result<ResponsesResponse, ProviderError> {
        Err(ProviderError::InvalidRequest(
            "Responses API is not supported by this provider".to_string(),
        ))
    }

    /// Native Responses API (streaming)
    async fn responses_streaming(
        &self,
        _request: ResponsesRequest,
    ) -> Result<StreamingResponses, ProviderError> {
        Err(ProviderError::InvalidRequest(
            "Streaming Responses API is not supported by this provider".to_string(),
        ))
    }

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
/// Supports both collected chunks (backward compatible) and true streaming
pub enum StreamingChat {
    /// Collected chunks (for backward compatibility)
    Collected {
        status: u16,
        chunks: Vec<ChatCompletionChunk>,
    },
    /// True stream using BoxStream
    Streamed {
        status: u16,
        stream: BoxStream<'static, ChatCompletionChunk>,
    },
}

impl StreamingChat {
    /// Create a new streaming response with collected chunks
    pub fn new(status: u16, chunks: Vec<ChatCompletionChunk>) -> Self {
        Self::Collected { status, chunks }
    }

    /// Create a new streaming response with a true stream
    pub fn with_stream(status: u16, stream: BoxStream<'static, ChatCompletionChunk>) -> Self {
        Self::Streamed { status, stream }
    }

    /// Check if the response is successful
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status())
    }

    /// Get the status code
    pub fn status(&self) -> u16 {
        match self {
            Self::Collected { status, .. } => *status,
            Self::Streamed { status, .. } => *status,
        }
    }

    /// Check if this is a collected response
    pub fn is_collected(&self) -> bool {
        matches!(self, Self::Collected { .. })
    }

    /// Check if this is a streamed response
    pub fn is_streamed(&self) -> bool {
        matches!(self, Self::Streamed { .. })
    }

    /// Get the collected chunks (if any)
    pub fn chunks(&self) -> Option<&Vec<ChatCompletionChunk>> {
        match self {
            Self::Collected { chunks, .. } => Some(chunks),
            Self::Streamed { .. } => None,
        }
    }

    /// Get the stream (if any)
    pub fn into_stream(self) -> Option<BoxStream<'static, ChatCompletionChunk>> {
        match self {
            Self::Streamed { stream, .. } => Some(stream),
            Self::Collected { .. } => None,
        }
    }
}

/// Native Responses streaming wrapper
pub struct StreamingResponses {
    /// HTTP status code
    pub status: u16,
    /// Raw SSE payload stream, one item per `data:` frame.
    pub events: BoxStream<'static, String>,
}

impl StreamingResponses {
    pub fn new(status: u16, events: BoxStream<'static, String>) -> Self {
        Self { status, events }
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}
