//! MiniMax Provider implementation
//!
//! Implements the LLMProvider trait for MiniMax's API.
//! MiniMax supports thinking/reasoning mode and has specific API requirements.

use async_trait::async_trait;
use futures::{stream, StreamExt};
use futures::stream::BoxStream;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatCompletionChunk, ChatRequest, ChatResponse};
use crate::models::response::{ResponsesRequest, ResponsesResponse, ResponsesStreamChunk};
use crate::protocol::capabilities::ProviderCapabilities;
use crate::transform::{
    transform_responses_to_chat_request, transform_chat_to_responses_response,
    transform_chat_stream_to_responses_stream,
};

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
            base_url: "https://api.minimaxi.com/v1".to_string(),
            default_model: "MiniMax-M2.7".to_string(),
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
            "MiniMax-M2.7".to_string(),  // Text model with reasoning
            "MiniMax-Text-01".to_string(),    // Text model
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
        // MiniMax may need specific request modifications
        // For now, just log the request
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
        let url = self.build_url("/chat/completions");

        let mut request = request;

        let request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        // MiniMax requires response_format as {"type": "text"}
        // Fix the serialized response_format to match MiniMax's expected format
        let mut request_body = request_body;
        if let Some(obj) = request_body.as_object_mut() {
            // Remove any existing response_format and add the correct one
            obj.remove("response_format");
            obj.insert("response_format".to_string(), serde_json::json!({"type": "text"}));
        }

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
        let url = self.build_url("/chat/completions");

        // MiniMax may need request preparation
        let mut request = request;
        self.prepare_minimax_request(&mut request);

        let mut request_with_stream = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        // MiniMax requires response_format as {"type": "text"}
        // Fix the serialized response_format to match MiniMax's expected format
        let mut request_with_stream = request_with_stream;
        if let Some(obj) = request_with_stream.as_object_mut() {
            obj.remove("response_format");
            obj.insert("response_format".to_string(), serde_json::json!({"type": "text"}));
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

        // Process SSE stream with true streaming (low latency)
        // Spawn a task to read from the HTTP stream and yield chunks in real-time
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ChatCompletionChunk, ProviderError>>(32);

        // Spawn task to read SSE stream and send chunks to channel
        tokio::spawn(async move {
            let mut buffer = String::new();

            // Read the response body as a stream
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
                                    if tx.send(Ok(chunk)).await.is_err() {
                                        // Receiver dropped, stop sending
                                        return;
                                    }
                                } else {
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
                        let _ = tx.send(Err(ProviderError::RequestFailed(e.to_string()))).await;
                        return;
                    }
                }
            }
            // Stream ended normally
        });

        // Create the stream from the receiver
        let stream = tokio_stream:: wrappers::ReceiverStream::new(rx)
            .map(|result| result.expect("channel error should be handled in spawn"));
        let boxed_stream: BoxStream<'static, ChatCompletionChunk> = Box::pin(stream);

        tracing::info!("MiniMax streaming started (true streaming mode)");

        Ok(StreamingChat::Streamed {
            status: 200,
            stream: boxed_stream,
        })
    }

    async fn responses(&self, request: ResponsesRequest) -> Result<ResponsesResponse, ProviderError> {
        // MiniMax doesn't have a native Responses API - convert to Chat and call chat()
        let mut chat_req = transform_responses_to_chat_request(&request);

        // Use the converted chat request
        if chat_req.model.is_empty() {
            chat_req.model = request.model.clone();
        }
        if chat_req.model.is_empty() {
            chat_req.model = "MiniMax-M2.7".to_string();
        }

        let chat_resp = self.chat(chat_req).await?;

        // Convert Chat response back to Responses format
        let responses_resp = transform_chat_to_responses_response(&chat_resp);
        Ok(responses_resp)
    }

    async fn responses_streaming(&self, request: ResponsesRequest) -> Result<StreamingResponses, ProviderError> {
        // MiniMax doesn't have a native Responses API - convert to Chat and call chat_streaming()
        let mut chat_req = transform_responses_to_chat_request(&request);

        // Use the converted chat request
        if chat_req.model.is_empty() {
            chat_req.model = request.model.clone();
        }
        if chat_req.model.is_empty() {
            chat_req.model = "MiniMax-M2.7".to_string();
        }

        // Get chat streaming
        let streaming_chat = self.chat_streaming(chat_req).await?;

        // Chat streaming uses StreamingChat enum, need to handle both Collected and Streamed cases
        match streaming_chat {
            StreamingChat::Streamed { status, stream } => {
                // Transform each ChatCompletionChunk to ResponsesStreamChunk then to SSE string
                // Use filter_map to drop errors silently (log them instead)
                let transformed_stream = stream
                    .filter_map(|chat_chunk| async move {
                        let responses_chunk = transform_chat_stream_to_responses_stream(&chat_chunk);
                        match serde_json::to_string(&responses_chunk) {
                            Ok(payload) => Some(format!("data: {}\n\n", payload)),
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to serialize responses chunk");
                                None
                            }
                        }
                    });

                let boxed_stream: BoxStream<'static, String> = Box::pin(transformed_stream);
                Ok(StreamingResponses { status, events: boxed_stream })
            }
            StreamingChat::Collected { status, chunks } => {
                // For collected, transform each chunk then emit
                let responses_chunks: Vec<ResponsesStreamChunk> = chunks
                    .iter()
                    .map(|chunk| transform_chat_stream_to_responses_stream(chunk))
                    .collect();

                let events: Vec<String> = responses_chunks
                    .iter()
                    .filter_map(|chunk| serde_json::to_string(chunk).ok())
                    .map(|payload| format!("data: {}\n\n", payload))
                    .collect();

                let boxed_stream: BoxStream<'static, String> = Box::pin(stream::iter(events));
                Ok(StreamingResponses { status, events: boxed_stream })
            }
        }
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
