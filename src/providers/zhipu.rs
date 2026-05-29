//! Zhipu AI (智谱) Provider implementation
//!
//! Implements the LLMProvider trait for Zhipu AI's GLM models.

use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatCompletionChunk, ChatRequest, ChatResponse};
use crate::models::response::{ResponsesRequest, ResponsesResponse};
use crate::protocol::capabilities::ProviderCapabilities;

use super::{
    build_http_client, classify_zhipu_base_url, current_proxy_env_summary,
    drain_complete_sse_payloads, extract_provider_error_details, extract_sse_data_payload,
    format_request_body_for_log, parse_provider_error, summarize_chat_request,
    summarize_response_headers, truncate_for_log, LLMProvider, ProviderError, StreamingChat,
    StreamingResponses,
};

/// Zhipu AI Provider
pub struct ZhipuProvider {
    config: ProviderConfig,
    client: Client,
}

impl ZhipuProvider {
    /// Create a new Zhipu provider
    pub fn new(config: ProviderConfig) -> Self {
        let client = build_http_client(config.timeout);

        Self { config, client }
    }

    /// Create from environment variables
    pub fn from_env() -> Self {
        Self::new(ProviderConfig {
            api_key: std::env::var("ZHIPU_API_KEY").unwrap_or_default(),
            base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
            default_model: "glm-4".to_string(),
            timeout: 60,
        })
    }

    /// Get capabilities for this provider
    /// Supports reasoning and most Responses features through chat fallback for Codex CLI
    fn get_capabilities() -> ProviderCapabilities {
        let caps = ProviderCapabilities::chat_fallback_with_full_support(vec![
            "function".to_string(),
            "web_search".to_string(),
            "namespace".to_string(), // Codex CLI uses namespace as a tool type
            "custom".to_string(), // Codex CLI may use custom tools
        ]);
        tracing::info!(
            supports_namespace = caps.supports_namespace,
            supports_store = caps.supports_store,
            supports_reasoning = caps.supports_reasoning,
            "Zhipu provider capabilities"
        );
        caps
    }
}

fn prepare_zhipu_request_body(request_body: Value) -> Value {
    // Start with only the essential fields
    let mut cleaned = serde_json::Map::new();

    if let Some(obj) = request_body.as_object() {
        // Only include non-null fields
        for (key, value) in obj {
            if !value.is_null() {
                cleaned.insert(key.clone(), value.clone());
            }
        }

        // Handle tools - GLM API requires web_search to have nested object
        // and must filter out unsupported tool types like "namespace"
        if let Some(tools) = cleaned.get_mut("tools").and_then(|t| t.as_array_mut()) {
            // Filter out tools with types that Zhipu API doesn't support
            // Zhipu supports: "function", "web_search"
            // Zhipu does NOT support: "namespace", "file_search", "computer_use", "mcp"
            tools.retain(|tool| {
                if let Some(tool_obj) = tool.as_object() {
                    let tool_type = tool_obj
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or_default();
                    // Keep only supported types
                    tool_type == "function" || tool_type == "web_search"
                } else {
                    true
                }
            });

            for tool in tools {
                if let Some(tool_obj) = tool.as_object_mut() {
                    let tool_type = tool_obj
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or_default();

                    // For web_search type, remove function field and ensure there's a nested web_search object
                    if tool_type == "web_search" {
                        tool_obj.remove("function");
                        if !tool_obj.contains_key("web_search") {
                            tool_obj.insert("web_search".to_string(), serde_json::json!({}));
                        }
                    }
                }
            }
        }
    }

    serde_json::Value::Object(cleaned)
}

#[async_trait]
impl LLMProvider for ZhipuProvider {
    fn name(&self) -> &str {
        "zhipu"
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    fn client(&self) -> &Client {
        &self.client
    }

    fn capabilities(&self) -> ProviderCapabilities {
        let caps = ZhipuProvider::get_capabilities();
        tracing::info!(
            supports_namespace = caps.supports_namespace,
            supports_store = caps.supports_store,
            supports_reasoning = caps.supports_reasoning,
            "Zhipu provider capabilities"
        );
        caps
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = self.build_url("/chat/completions");
        let endpoint_kind = classify_zhipu_base_url(&self.config.base_url);

        // Serialize request
        let request_body = prepare_zhipu_request_body(
            serde_json::to_value(&request)
                .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?,
        );
        let request_body_text = serde_json::to_string(&request_body)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::info!(
            provider = self.name(),
            url = %url,
            base_url = %self.config.base_url,
            endpoint_kind = endpoint_kind,
            model = %request.model,
            proxy_env = %current_proxy_env_summary(),
            request_summary = %summarize_chat_request(&request),
            "provider chat request"
        );
        tracing::debug!(
            provider = self.name(),
            request_body = %format_request_body_for_log(&request_body_text),
            "provider chat request body"
        );

        if endpoint_kind == "coding" && request.model.starts_with("glm-") {
            tracing::warn!(
                provider = self.name(),
                base_url = %self.config.base_url,
                model = %request.model,
                "glm model is being sent to Zhipu coding endpoint; verify the configured base_url"
            );
        }

        // Make request
        let response = self
            .client
            .post(&url)
            .header("Authorization", self.build_auth_header())
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let response_headers = summarize_response_headers(response.headers());
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;
        let upstream_error = extract_provider_error_details(&body);

        tracing::info!(
            provider = self.name(),
            url = %url,
            status = %status,
            endpoint_kind = endpoint_kind,
            model = %request.model,
            response_headers = %response_headers,
            upstream_error = ?upstream_error,
            response_body = %truncate_for_log(&body, 4000),
            "provider chat response"
        );

        if !status.is_success() {
            return Err(parse_provider_error(status, &body));
        }

        // Parse response
        let response_data: Value = serde_json::from_str(&body)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // Zhipu returns OpenAI-compatible format, convert directly
        let chat_response: ChatResponse = serde_json::from_value(response_data)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        Ok(chat_response)
    }

    async fn chat_streaming(&self, request: ChatRequest) -> Result<StreamingChat, ProviderError> {
        let url = self.build_url("/chat/completions");
        let endpoint_kind = classify_zhipu_base_url(&self.config.base_url);

        // Serialize request with stream: true
        let mut request_body = prepare_zhipu_request_body(
            serde_json::to_value(&request)
                .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?,
        );

        if let Some(obj) = request_body.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(true));
        }

        let request_body_text = serde_json::to_string(&request_body)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::info!(
            provider = self.name(),
            url = %url,
            base_url = %self.config.base_url,
            endpoint_kind = endpoint_kind,
            model = %request.model,
            proxy_env = %current_proxy_env_summary(),
            request_summary = %summarize_chat_request(&request),
            "provider streaming request"
        );
        tracing::debug!(
            provider = self.name(),
            request_body = %format_request_body_for_log(&request_body_text),
            "provider streaming request body"
        );

        if endpoint_kind == "coding" && request.model.starts_with("glm-") {
            tracing::warn!(
                provider = self.name(),
                base_url = %self.config.base_url,
                model = %request.model,
                "glm model is being sent to Zhipu coding endpoint; verify the configured base_url"
            );
        }

        // Make request
        let response = self
            .client
            .post(&url)
            .header("Authorization", self.build_auth_header())
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let response_headers = summarize_response_headers(response.headers());

        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;
            let upstream_error = extract_provider_error_details(&body);
            tracing::info!(
                provider = self.name(),
                url = %url,
                status = %status,
                endpoint_kind = endpoint_kind,
                model = %request.model,
                response_headers = %response_headers,
                upstream_error = ?upstream_error,
                response_body = %truncate_for_log(&body, 4000),
                "provider streaming response"
            );
            return Err(parse_provider_error(status, &body));
        }

        // Real streaming: use bytes_stream() with mpsc channel
        let provider_name = self.name().to_string();
        let (tx, rx) = mpsc::unbounded_channel::<ChatCompletionChunk>();
        let mut upstream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut parsed_chunks = 0usize;
            let mut errors = 0usize;

            while let Some(chunk_result) = upstream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete SSE frames
                        for payload in drain_complete_sse_payloads(&mut buffer) {
                            if payload == "[DONE]" {
                                tracing::info!(
                                    provider = %provider_name,
                                    parsed_chunks = parsed_chunks,
                                    parse_errors = errors,
                                    "streaming completed with [DONE]"
                                );
                                return;
                            }

                            match serde_json::from_str::<ChatCompletionChunk>(&payload) {
                                Ok(chunk) => {
                                    tracing::trace!(
                                        provider = %provider_name,
                                        chunk_id = %chunk.id,
                                        choices_count = chunk.choices.len(),
                                        "streaming chunk"
                                    );
                                    if tx.send(chunk).is_err() {
                                        tracing::debug!(provider = %provider_name, "receiver dropped");
                                        return;
                                    }
                                    parsed_chunks += 1;
                                }
                                Err(e) => {
                                    errors += 1;
                                    tracing::warn!(
                                        provider = %provider_name,
                                        error = %e,
                                        payload_preview = %payload[..payload.len().min(200)],
                                        "failed to parse streaming chunk"
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            provider = %provider_name,
                            error = %e,
                            "streaming read error"
                        );
                        return;
                    }
                }
            }

            // Handle any remaining data in buffer
            if !buffer.trim().is_empty() {
                if let Some(payload) = extract_sse_data_payload(&buffer) {
                    if payload != "[DONE]" {
                        if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(&payload) {
                            let _ = tx.send(chunk);
                            parsed_chunks += 1;
                        }
                    }
                }
            }

            tracing::info!(
                provider = %provider_name,
                parsed_chunks = parsed_chunks,
                parse_errors = errors,
                "streaming ended"
            );
        });

        let stream = stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|chunk| (chunk, rx))
        })
        .boxed();

        tracing::info!(
            provider = self.name(),
            status = %status,
            response_headers = %response_headers,
            "provider streaming connected"
        );

        Ok(StreamingChat::with_stream(status.as_u16(), stream))
    }

    async fn responses(
        &self,
        request: ResponsesRequest,
    ) -> Result<ResponsesResponse, ProviderError> {
        // Transform Responses API request to Chat API request
        let chat_request = crate::transform::transform_responses_to_chat_request(&request);

        // Use the existing chat method
        let chat_response = self.chat(chat_request).await?;

        // Transform Chat API response back to Responses API format
        let responses_response =
            crate::transform::transform_chat_to_responses_response(&chat_response);

        Ok(responses_response)
    }

    async fn responses_streaming(
        &self,
        request: ResponsesRequest,
    ) -> Result<StreamingResponses, ProviderError> {
        // Transform Responses API request to Chat API request
        let chat_request = crate::transform::transform_responses_to_chat_request(&request);
        let endpoint_kind = classify_zhipu_base_url(&self.config.base_url);

        let url = self.build_url("/chat/completions");
        let mut request_body = prepare_zhipu_request_body(
            serde_json::to_value(&chat_request)
                .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?,
        );

        if let Some(obj) = request_body.as_object_mut() {
            obj.insert("stream".to_string(), Value::Bool(true));
        }

        let request_body_text = serde_json::to_string(&request_body)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::info!(
            provider = self.name(),
            url = %url,
            base_url = %self.config.base_url,
            endpoint_kind = endpoint_kind,
            model = %chat_request.model,
            proxy_env = %current_proxy_env_summary(),
            "provider responses streaming request"
        );
        tracing::debug!(
            provider = self.name(),
            request_body = %format_request_body_for_log(&request_body_text),
            "provider responses streaming request body"
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.build_auth_header())
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let response_headers = summarize_response_headers(response.headers());

        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;
            let upstream_error = extract_provider_error_details(&body);
            tracing::info!(
                provider = self.name(),
                url = %url,
                status = %status,
                endpoint_kind = endpoint_kind,
                response_headers = %response_headers,
                upstream_error = ?upstream_error,
                response_body = %truncate_for_log(&body, 4000),
                "provider responses streaming response"
            );
            return Err(parse_provider_error(status, &body));
        }

        // Real streaming: forward SSE events directly
        let provider_name = self.name().to_string();
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let mut upstream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut event_count = 0usize;

            while let Some(chunk_result) = upstream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        for payload in drain_complete_sse_payloads(&mut buffer) {
                            event_count += 1;
                            if tx.send(payload).is_err() {
                                tracing::debug!(provider = %provider_name, "receiver dropped");
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            provider = %provider_name,
                            error = %e,
                            "responses streaming read error"
                        );
                        return;
                    }
                }
            }

            // Handle remaining buffer
            if !buffer.trim().is_empty() {
                if let Some(payload) = extract_sse_data_payload(&buffer) {
                    let _ = tx.send(payload);
                    event_count += 1;
                }
            }

            tracing::info!(
                provider = %provider_name,
                total_events = event_count,
                "responses streaming ended"
            );
        });

        let event_stream = stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|payload| (payload, rx))
        })
        .boxed();

        tracing::info!(
            provider = self.name(),
            status = %status,
            response_headers = %response_headers,
            "provider responses streaming connected"
        );

        Ok(StreamingResponses::new(status.as_u16(), event_stream))
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        // Zhipu doesn't have a public models list endpoint
        // Return known models
        Ok(vec![
            "glm-5".to_string(),
            "glm-4-flash".to_string(),
            "glm-4".to_string(),
            "glm-4-plus".to_string(),
            "glm-4-vision".to_string(),
            "glm-3-turbo".to_string(),
        ])
    }

    async fn health_check(&self) -> bool {
        let url = self.build_url("/chat/completions");

        // Try a minimal request to check connectivity
        let test_request = serde_json::json!({
            "model": self.config.default_model,
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 1
        });

        match self
            .client
            .post(&url)
            .header("Authorization", self.build_auth_header())
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send()
            .await
        {
            Ok(_response) => {
                // Any response (even errors) means connectivity is OK
                // Only network failures mean health check failed
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{ChatRequest, Message, Tool};
    use serde_json::json;

    #[test]
    fn test_zhipu_url_building() {
        let provider = ZhipuProvider::from_env();
        let url = provider.build_url("/chat/completions");
        assert_eq!(url, "https://open.bigmodel.cn/api/paas/v4/chat/completions");
    }

    #[test]
    fn test_zhipu_auth_header() {
        let provider = ZhipuProvider::new(ProviderConfig {
            api_key: "test-key".to_string(),
            base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
            default_model: "glm-4".to_string(),
            timeout: 60,
        });
        let header = provider.build_auth_header();
        assert_eq!(header, "Bearer test-key");
    }

    #[test]
    fn test_prepare_zhipu_request_body_adds_web_search_object() {
        let request = ChatRequest {
            model: "glm-5".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Some("hi".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            temperature: None,
            top_p: None,
            max_tokens: Some(1),
            stream: None,
            stop: None,
            n: 1,
            stream_options: None,
            include_usage: Some(true),
            response_format: None,
            seed: None,
            organization: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            tools: Some(vec![Tool {
                tool_type: "web_search".to_string(),
                function: None,
            }]),
            tool_choice: None,
            parallel_tool_calls: true,
            reasoning_effort: None,
            thinking: None,
        };

        let body = serde_json::to_value(&request).unwrap();
        let prepared = prepare_zhipu_request_body(body);

        assert_eq!(
            prepared["tools"][0],
            json!({
                "type": "web_search",
                "web_search": {}
            })
        );
    }
}
