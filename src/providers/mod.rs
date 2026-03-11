//! Provider abstraction layer
//!
//! This module defines the LLMProvider trait and implements providers for:
//! - OpenAI
//! - Zhipu AI (智谱)

#![allow(dead_code)]

use thiserror::Error;

pub mod openai;
pub mod trait_;
pub mod zhipu;

// Re-export types
pub use openai::OpenAIProvider;
pub use trait_::{LLMProvider, StreamingChat};
pub use zhipu::ZhipuProvider;

/// Provider-specific error types
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Helper function to parse provider error from response
pub fn parse_provider_error(status: reqwest::StatusCode, body: &str) -> ProviderError {
    // Try to parse JSON error response
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");

            return match status.as_u16() {
                401 => ProviderError::AuthFailed(message.to_string()),
                429 => ProviderError::RateLimited(message.to_string()),
                404 => ProviderError::ModelNotFound(message.to_string()),
                400 => ProviderError::InvalidRequest(message.to_string()),
                _ => ProviderError::RequestFailed(message.to_string()),
            };
        }
    }

    // Fallback to raw body
    ProviderError::RequestFailed(body.to_string())
}
