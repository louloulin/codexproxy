//! Zhipu AI (智谱) Provider implementation
//!
//! Implements the LLMProvider trait for Zhipu AI's GLM models.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::config::ProviderConfig;
use crate::models::chat::{ChatCompletionChunk, ChatRequest, ChatResponse};

use super::{
    build_http_client, classify_zhipu_base_url, current_proxy_env_summary,
    extract_provider_error_details, format_request_body_for_log, parse_provider_error,
    summarize_chat_request,
    summarize_response_headers, truncate_for_log, LLMProvider, ProviderError, StreamingChat,
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
}

fn prepare_zhipu_request_body(mut request_body: Value) -> Value {
    if let Some(tools) = request_body.get_mut("tools").and_then(Value::as_array_mut) {
        for tool in tools.iter_mut() {
            let tool_type = tool
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            if let Some(object) = tool.as_object_mut() {
                if tool_type != "function" {
                    object.remove("function");
                }

                if tool_type == "web_search" && !object.contains_key("web_search") {
                    object.insert("web_search".to_string(), serde_json::json!({}));
                }
            }
        }
    }

    request_body
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
