//! Zhipu AI (智谱) Provider implementation
//!
//! Implements the LLMProvider trait for Zhipu AI's GLM models.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::config::ProviderConfig;
use crate::models::{ChatRequest, ChatResponse};

use super::{parse_provider_error, LLMProvider, ProviderError, StreamingChat};

/// Zhipu AI Provider
pub struct ZhipuProvider {
    config: ProviderConfig,
    client: Client,
}

impl ZhipuProvider {
    /// Create a new Zhipu provider
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .expect("Failed to create HTTP client");

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

        // Serialize request
        let request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

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
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

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

        // Serialize request with stream: true
        let mut request_body = serde_json::to_value(&request)
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        if let Some(obj) = request_body.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(true));
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

        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;
            return Err(parse_provider_error(status, &body));
        }

        // Return streaming response
        Ok(StreamingChat::new(status.as_u16(), vec![]))
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
}
