//! OpenAI Provider implementation
//!
//! Implements the LLMProvider trait for OpenAI's API.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::config::ProviderConfig;
use crate::models::{ChatRequest, ChatResponse};

use super::{
    build_http_client, current_proxy_env_summary, extract_provider_error_details,
    format_request_body_for_log,
    parse_provider_error, summarize_chat_request, summarize_response_headers, truncate_for_log,
    LLMProvider, ProviderError, StreamingChat,
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

        // Return streaming response
        Ok(StreamingChat::new(status.as_u16(), vec![]))
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
