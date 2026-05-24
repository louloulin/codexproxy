//! OpenAI Provider implementation
//!
//! Implements the LLMProvider trait for OpenAI's API.

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
    build_http_client, current_proxy_env_summary, extract_provider_error_details,
    format_request_body_for_log,
    parse_provider_error, summarize_chat_request, summarize_response_headers, truncate_for_log,
    LLMProvider, ProviderError, StreamingChat, StreamingResponses,
};

/// OpenAI Provider
pub struct OpenAIProvider {
    config: ProviderConfig,
    client: Client,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(config: ProviderConfig) -> Self {
        let client = build_http_client(config.timeout);

        Self { config, client }
    }

    /// Create from environment variables
    pub fn from_env() -> Self {
        Self::new(ProviderConfig {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: "gpt-4o".to_string(),
            timeout: 120,
        })
    }
}

pub(crate) fn find_sse_frame_terminator(buffer: &str) -> Option<(usize, usize)> {
    let lf = buffer.find("\n\n").map(|idx| (idx, 2));
    let crlf = buffer.find("\r\n\r\n").map(|idx| (idx, 4));

    match (lf, crlf) {
        (Some(left), Some(right)) => Some(if left.0 <= right.0 { left } else { right }),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

pub(crate) fn extract_sse_data_payload(frame: &str) -> Option<String> {
    let mut payload_lines = Vec::new();

    for line in frame.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(data) = line.strip_prefix("data: ") {
            payload_lines.push(data.to_string());
        }
    }

    if payload_lines.is_empty() {
        None
    } else {
        Some(payload_lines.join("\n"))
    }
}

pub(crate) fn drain_complete_sse_payloads(buffer: &mut String) -> Vec<String> {
    let mut payloads = Vec::new();

    while let Some((frame_end, separator_len)) = find_sse_frame_terminator(buffer) {
        let frame = buffer[..frame_end].to_string();
        buffer.drain(..frame_end + separator_len);

        if let Some(payload) = extract_sse_data_payload(&frame) {
            payloads.push(payload);
        }
    }

    payloads
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    fn client(&self) -> &Client {
        &self.client
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::native_responses(vec![
            "function".to_string(),
            "web_search".to_string(),
            "file_search".to_string(),
            "computer_use".to_string(),
            "mcp".to_string(),
        ])
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = self.build_url("/chat/completions");

        // Serialize request
        let request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;
        let request_body_text = serde_json::to_string(&request_body)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::info!(
            provider = self.name(),
            url = %url,
            base_url = %self.config.base_url,
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

        // Convert to ChatResponse
        let chat_response: ChatResponse = serde_json::from_value(response_data)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        Ok(chat_response)
    }

    async fn chat_streaming(&self, request: ChatRequest) -> Result<StreamingChat, ProviderError> {
        let url = self.build_url("/chat/completions");

        // Serialize request with stream: true
        let mut request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        if let Some(obj) = request_body.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(true));
        }

        let request_body_text = serde_json::to_string(&request_body)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::info!(
            provider = self.name(),
            url = %url,
            base_url = %self.config.base_url,
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
                model = %request.model,
                response_headers = %response_headers,
                upstream_error = ?upstream_error,
                response_body = %truncate_for_log(&body, 4000),
                "provider streaming response"
            );
            return Err(parse_provider_error(status, &body));
        }

        // Process SSE stream and collect chunks
        let mut chunks = Vec::new();
        let body = response
            .bytes()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let body_str = String::from_utf8_lossy(&body);
        let line_count = body_str.lines().count();

        tracing::info!(
            provider = self.name(),
            total_lines = line_count,
            body_length = body_str.len(),
            "processing SSE stream"
        );

        // Parse SSE format: "data: {...}\n\n" or "data: [DONE]\n\n"
        let mut data_lines = 0;
        let mut parsed_chunks = 0;
        let mut errors = 0;

        for line in body_str.lines() {
            let line = line.trim();
            if line.starts_with("data: ") {
                data_lines += 1;
                let data = &line[6..]; // Remove "data: " prefix
                if data == "[DONE]" {
                    tracing::info!(
                        provider = self.name(),
                        "found [DONE] marker in SSE stream"
                    );
                    break;
                }
                // Parse the chunk
                match serde_json::from_str::<ChatCompletionChunk>(data) {
                    Ok(chunk) => {
                        tracing::debug!(
                            provider = self.name(),
                            chunk_id = %chunk.id,
                            choices_count = chunk.choices.len(),
                            has_usage = chunk.usage.is_some(),
                            "parsed chunk successfully"
                        );
                        chunks.push(chunk);
                        parsed_chunks += 1;
                    }
                    Err(e) => {
                        errors += 1;
                        tracing::warn!(
                            provider = self.name(),
                            error = %e,
                            data_preview = &data[..data.len().min(200)],
                            "failed to parse chunk"
                        );
                    }
                }
            }
        }

        tracing::info!(
            provider = self.name(),
            data_lines = data_lines,
            parsed_chunks = parsed_chunks,
            parse_errors = errors,
            "SSE stream processing complete"
        );

        // Return streaming response with collected chunks
        Ok(StreamingChat::new(status.as_u16(), chunks))
    }

    async fn responses(
        &self,
        request: ResponsesRequest,
    ) -> Result<ResponsesResponse, ProviderError> {
        let url = self.build_url("/responses");
        let request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;
        let request_body_text = serde_json::to_string(&request_body)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::info!(
            provider = self.name(),
            url = %url,
            base_url = %self.config.base_url,
            model = %request.model,
            stream = request.stream.unwrap_or(false),
            input_count = request.input.len(),
            tool_count = request.tools.len(),
            include_count = request.include.len(),
            has_previous_response_id = request.previous_response_id.is_some(),
            proxy_env = %current_proxy_env_summary(),
            "provider responses request"
        );
        tracing::debug!(
            provider = self.name(),
            request_body = %format_request_body_for_log(&request_body_text),
            "provider responses request body"
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
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;
        let upstream_error = extract_provider_error_details(&body);

        tracing::info!(
            provider = self.name(),
            url = %url,
            status = %status,
            model = %request.model,
            response_headers = %response_headers,
            upstream_error = ?upstream_error,
            response_body = %truncate_for_log(&body, 4000),
            "provider responses response"
        );

        if !status.is_success() {
            return Err(parse_provider_error(status, &body));
        }

        let response_data: Value = serde_json::from_str(&body)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        let responses_response: ResponsesResponse = serde_json::from_value(response_data)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        Ok(responses_response)
    }

    async fn responses_streaming(
        &self,
        request: ResponsesRequest,
    ) -> Result<StreamingResponses, ProviderError> {
        let url = self.build_url("/responses");
        let mut request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;
        if let Some(obj) = request_body.as_object_mut() {
            obj.insert("stream".to_string(), Value::Bool(true));
        }
        let request_body_text = serde_json::to_string(&request_body)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        tracing::info!(
            provider = self.name(),
            url = %url,
            base_url = %self.config.base_url,
            model = %request.model,
            input_count = request.input.len(),
            tool_count = request.tools.len(),
            include_count = request.include.len(),
            has_previous_response_id = request.previous_response_id.is_some(),
            proxy_env = %current_proxy_env_summary(),
            "provider streaming responses request"
        );
        tracing::debug!(
            provider = self.name(),
            request_body = %format_request_body_for_log(&request_body_text),
            "provider streaming responses request body"
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
                model = %request.model,
                response_headers = %response_headers,
                upstream_error = ?upstream_error,
                response_body = %truncate_for_log(&body, 4000),
                "provider streaming responses response"
            );
            return Err(parse_provider_error(status, &body));
        }

        let provider_name = self.name().to_string();
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let mut upstream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut event_count = 0usize;

            while let Some(chunk) = upstream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        for payload in drain_complete_sse_payloads(&mut buffer) {
                            event_count += 1;
                            if tx.send(payload).is_err() {
                                return;
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!(
                            provider = %provider_name,
                            error = %error,
                            "provider streaming responses chunk read failed"
                        );
                        return;
                    }
                }
            }

            if !buffer.trim().is_empty() {
                if let Some(payload) = extract_sse_data_payload(&buffer) {
                    let _ = tx.send(payload);
                    event_count += 1;
                }
            }

            tracing::info!(
                provider = %provider_name,
                total_events = event_count,
                "provider streaming responses parsing complete"
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
            "provider streaming responses connected"
        );

        Ok(StreamingResponses::new(status.as_u16(), event_stream))
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = self.build_url("/models");

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.build_auth_header())
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        if !status.is_success() {
            return Err(parse_provider_error(status, &body));
        }

        let data: Value = serde_json::from_str(&body)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let models: Vec<String> = data
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    async fn health_check(&self) -> bool {
        let url = self.build_url("/models");

        match self
            .client
            .get(&url)
            .header("Authorization", self.build_auth_header())
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

    #[test]
    fn test_drain_complete_sse_payloads_handles_split_frames() {
        let mut buffer = "data: {\"type\":\"response.created\"".to_string();
        assert!(drain_complete_sse_payloads(&mut buffer).is_empty());

        buffer.push_str(",\"response\":{\"id\":\"resp_1\"}}\n\ndata: [DONE]\n\n");
        let payloads = drain_complete_sse_payloads(&mut buffer);

        assert_eq!(
            payloads,
            vec![
                "{\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\"}}".to_string(),
                "[DONE]".to_string(),
            ]
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_drain_complete_sse_payloads_supports_crlf_frames() {
        let mut buffer = "data: first\r\n\r\ndata: second\r\n\r\n".to_string();
        let payloads = drain_complete_sse_payloads(&mut buffer);

        assert_eq!(payloads, vec!["first".to_string(), "second".to_string()]);
        assert!(buffer.is_empty());
    }
}
