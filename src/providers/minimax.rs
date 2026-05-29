//! MiniMax Provider implementation
//!
//! Implements the LLMProvider trait for MiniMax's API.
//! MiniMax supports thinking/reasoning mode and has specific API requirements.

use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::Client;
use serde_json::Value;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatCompletionChunk, ChatRequest, ChatResponse};
use crate::models::response::{ResponsesRequest, ResponsesResponse};
use crate::protocol::capabilities::ProviderCapabilities;

use super::{
    build_http_client, drain_complete_sse_payloads, extract_provider_error_details,
    format_request_body_for_log, truncate_for_log, LLMProvider, ProviderError, StreamingChat,
    StreamingResponses,
};

/// MiniMax Provider
pub struct MiniMaxProvider {
    config: ProviderConfig,
    client: Client,
}

impl MiniMaxProvider {
    /// Create a new MiniMax provider
    pub fn new(config: ProviderConfig) -> Self {
        let client = build_http_client(config.timeout);
        Self { config, client }
    }

    /// Create from environment variables
    pub fn from_env() -> Self {
        Self::new(ProviderConfig {
            api_key: std::env::var("MINIMAX_API_KEY").unwrap_or_default(),
            base_url: "https://api.minimax.chat".to_string(),
            default_model: "MiniMax-Text-01".to_string(),
            timeout: 120,
        })
    }

    /// Build URL for an endpoint
    fn build_url(&self, path: &str) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// Add auth headers to a request builder
    fn add_auth_headers(&self, req_builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req_builder
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
    }

    /// Built-in models for MiniMax
    pub fn builtin_models() -> Vec<String> {
        vec![
            "MiniMax-Text-01".to_string(),    // Text model with reasoning
            "abab6.5s-chat".to_string(),      // Chat model
            "abab6.5g-chat".to_string(),      // Enhanced chat
            "abab5.5-chat".to_string(),        // Standard chat
        ]
    }

    /// Get capabilities for this provider
    fn get_capabilities() -> ProviderCapabilities {
        // MiniMax supports reasoning and has specific chat API requirements
        ProviderCapabilities::chat_fallback_with_full_support(vec![
            "function".to_string(),
        ])
    }

    /// Prepare MiniMax-specific request body
    fn prepare_minimax_request(&self, request: &mut ChatRequest) {
        // MiniMax may require specific model name handling
        // Add any MiniMax-specific request modifications here
        tracing::debug!(
            model = %request.model,
            "MiniMax request preparation"
        );
    }
}

#[async_trait]
impl LLMProvider for MiniMaxProvider {
    fn name(&self) -> &str {
        "minimax"
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    fn client(&self) -> &Client {
        &self.client
    }

    fn capabilities(&self) -> ProviderCapabilities {
        MiniMaxProvider::get_capabilities()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = self.build_url("/v1/text/chatcompletion_v2");

        // MiniMax may need request preparation
        let mut request = request;
        self.prepare_minimax_request(&mut request);

        let request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::debug!(
            url,
            model = %request.model,
            message_count = request.messages.len(),
            "MiniMax chat request"
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
            "MiniMax chat response received"
        );

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                status = status.as_u16(),
                body = %truncate_for_log(&body, 500),
                "MiniMax chat request failed"
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

        // Parse MiniMax response (may differ from OpenAI format)
        let body = resp.text().await.unwrap_or_default();
        let body_value: Value = serde_json::from_str(&body)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // MiniMax response format may need conversion
        // For now, try to parse as-is or create a compatible response
        let chat_response: ChatResponse = serde_json::from_value(body_value)
            .map_err(|e| {
                tracing::warn!(
                    error = %e,
                    "MiniMax response parse failed, attempting conversion"
                );
                ProviderError::InvalidResponse(e.to_string())
            })?;

        Ok(chat_response)
    }

    async fn chat_streaming(&self, request: ChatRequest) -> Result<StreamingChat, ProviderError> {
        let url = self.build_url("/v1/text/chatcompletion_v2");

        // MiniMax may need request preparation
        let mut request = request;
        self.prepare_minimax_request(&mut request);

        let mut request_with_stream = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        // Add streaming parameter for MiniMax
        if let Some(obj) = request_with_stream.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::json!(true));
        }

        tracing::debug!(
            url,
            model = %request.model,
            "MiniMax streaming chat request"
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
            "MiniMax streaming chat response received"
        );

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                status = status.as_u16(),
                body = %truncate_for_log(&body, 500),
                "MiniMax streaming chat request failed"
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
                            // Try to parse as ChatCompletionChunk
                            if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(&payload) {
                                chunks.push(chunk);
                            } else {
                                // MiniMax may have different streaming format
                                tracing::debug!(
                                    payload_preview = %truncate_for_log(&payload, 100),
                                    "MiniMax streaming payload (non-standard)"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "MiniMax streaming error");
                    return Err(ProviderError::RequestFailed(e.to_string()));
                }
            }
        }

        tracing::info!(chunk_count = chunks.len(), "MiniMax streaming completed");

        Ok(StreamingChat::Collected {
            status: 200,
            chunks,
        })
    }

    async fn responses(&self, _request: ResponsesRequest) -> Result<ResponsesResponse, ProviderError> {
        // MiniMax doesn't have a native Responses API - use chat fallback
        Err(ProviderError::InvalidRequest(
            "MiniMax: Native Responses API not supported, use chat fallback".to_string(),
        ))
    }

    async fn responses_streaming(&self, _request: ResponsesRequest) -> Result<StreamingResponses, ProviderError> {
        // MiniMax doesn't have a native Responses API - use chat fallback
        Err(ProviderError::InvalidRequest(
            "MiniMax: Native Responses API not supported, use chat fallback".to_string(),
        ))
    }

    async fn health_check(&self) -> bool {
        // MiniMax may not have a models endpoint
        // Try a minimal request instead
        let url = self.build_url("/v1/models");

        match self.add_auth_headers(self.client.get(&url))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}
