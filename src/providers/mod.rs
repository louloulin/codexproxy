//! Provider abstraction layer
//!
//! This module defines the LLMProvider trait and implements providers for:
//! - OpenAI
//! - Zhipu AI (智谱)

#![allow(dead_code)]

use reqwest::{header::HeaderMap, Client, ClientBuilder, Proxy};
use serde_json::{json, Value};
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

pub(crate) fn truncate_for_log(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }

    let truncated: String = text.chars().take(limit).collect();
    format!(
        "{}...(truncated, total {} chars)",
        truncated,
        text.chars().count()
    )
}

pub(crate) fn format_request_body_for_log(text: &str) -> String {
    text.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderErrorDetails {
    pub message: String,
    pub error_type: Option<String>,
    pub code: Option<String>,
    pub param: Option<String>,
}

pub(crate) fn extract_provider_error_details(body: &str) -> Option<ProviderErrorDetails> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    let error = value.get("error")?;

    Some(ProviderErrorDetails {
        message: error.get("message")?.as_str()?.to_string(),
        error_type: error
            .get("type")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        code: match error.get("code") {
            Some(Value::String(code)) => Some(code.clone()),
            Some(Value::Number(code)) => Some(code.to_string()),
            _ => None,
        },
        param: error
            .get("param")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
    })
}

pub(crate) fn summarize_chat_request(request: &crate::models::chat::ChatRequest) -> Value {
    json!({
        "model": request.model,
        "message_count": request.messages.len(),
        "message_roles": request.messages.iter().map(|message| message.role.clone()).collect::<Vec<_>>(),
        "tool_count": request.tools.as_ref().map(|tools| tools.len()).unwrap_or(0),
        "stream": request.stream.unwrap_or(false),
        "max_tokens": request.max_tokens,
        "temperature": request.temperature,
    })
}

fn proxy_env_summary(proxy_env: &ProxyEnv) -> Value {
    json!({
        "all": proxy_env.all,
        "http": proxy_env.http,
        "https": proxy_env.https,
    })
}

pub(crate) fn current_proxy_env_summary() -> Value {
    proxy_env_summary(&proxy_urls_from_env())
}

pub(crate) fn classify_zhipu_base_url(base_url: &str) -> &'static str {
    if base_url.contains("/api/coding/") {
        "coding"
    } else if base_url.contains("/api/paas/") {
        "general"
    } else {
        "custom"
    }
}

pub(crate) fn summarize_response_headers(headers: &HeaderMap) -> Value {
    let interesting = [
        "content-type",
        "x-request-id",
        "request-id",
        "x-glm-request-id",
        "x-ratelimit-limit-requests",
        "x-ratelimit-remaining-requests",
    ];

    let mut summary = serde_json::Map::new();
    for name in interesting {
        let value = headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        summary.insert(name.to_string(), value.map(Value::String).unwrap_or(Value::Null));
    }

    Value::Object(summary)
}

/// Helper function to parse provider error from response
pub fn parse_provider_error(status: reqwest::StatusCode, body: &str) -> ProviderError {
    if let Some(details) = extract_provider_error_details(body) {
        return match status.as_u16() {
            401 => ProviderError::AuthFailed(details.message),
            429 => ProviderError::RateLimited(details.message),
            404 => ProviderError::ModelNotFound(details.message),
            400 => ProviderError::InvalidRequest(details.message),
            _ => ProviderError::RequestFailed(details.message),
        };
    }

    // Fallback to raw body
    ProviderError::RequestFailed(body.to_string())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ProxyEnv {
    all: Option<String>,
    http: Option<String>,
    https: Option<String>,
}

fn first_env_var(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .filter(|value| !value.is_empty())
}

fn proxy_urls_from_env() -> ProxyEnv {
    ProxyEnv {
        all: first_env_var(&["ALL_PROXY", "all_proxy"]),
        http: first_env_var(&["HTTP_PROXY", "http_proxy"]),
        https: first_env_var(&["HTTPS_PROXY", "https_proxy"]),
    }
}

pub fn build_http_client(timeout_secs: u64) -> Client {
    let proxy_env = proxy_urls_from_env();
    tracing::info!(
        timeout_secs,
        proxy_env = %proxy_env_summary(&proxy_env),
        "configuring provider http client"
    );
    let mut builder = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        // Disable reqwest's automatic proxy discovery because it panics
        // under the sandboxed macOS environment used for local verification.
        .no_proxy();

    builder = apply_proxy(builder, proxy_env.all.as_deref(), |url| Proxy::all(url));
    builder = apply_proxy(builder, proxy_env.http.as_deref(), |url| Proxy::http(url));
    builder = apply_proxy(builder, proxy_env.https.as_deref(), |url| Proxy::https(url));

    builder.build().expect("Failed to create HTTP client")
}

fn apply_proxy<F>(builder: ClientBuilder, value: Option<&str>, create_proxy: F) -> ClientBuilder
where
    F: FnOnce(&str) -> Result<Proxy, reqwest::Error>,
{
    match value {
        Some(url) => match create_proxy(url) {
            Ok(proxy) => builder.proxy(proxy),
            Err(err) => {
                tracing::warn!(proxy_url = %url, error = %err, "Ignoring invalid proxy URL");
                builder
            }
        },
        None => builder,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_zhipu_base_url, extract_provider_error_details, format_request_body_for_log,
        proxy_env_summary, proxy_urls_from_env, truncate_for_log, ProxyEnv,
    };
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_proxy_urls_from_env_reads_https_proxy() {
        let _guard = env_lock().lock().unwrap();
        let original_https = std::env::var("HTTPS_PROXY").ok();
        let original_https_lower = std::env::var("https_proxy").ok();

        std::env::set_var("HTTPS_PROXY", "http://proxy.example.com:8080");
        std::env::remove_var("https_proxy");

        let proxy_env = proxy_urls_from_env();

        if let Some(value) = original_https {
            std::env::set_var("HTTPS_PROXY", value);
        } else {
            std::env::remove_var("HTTPS_PROXY");
        }

        if let Some(value) = original_https_lower {
            std::env::set_var("https_proxy", value);
        } else {
            std::env::remove_var("https_proxy");
        }

        assert_eq!(
            proxy_env.https.as_deref(),
            Some("http://proxy.example.com:8080")
        );
    }

    #[test]
    fn test_truncate_for_log_marks_truncated_output() {
        let text = "abcdefghijklmnopqrstuvwxyz";

        let truncated = truncate_for_log(text, 10);

        assert_eq!(truncated, "abcdefghij...(truncated, total 26 chars)");
    }

    #[test]
    fn test_classify_zhipu_base_url_detects_coding_endpoint() {
        let endpoint_kind = classify_zhipu_base_url("https://open.bigmodel.cn/api/coding/paas/v4");

        assert_eq!(endpoint_kind, "coding");
    }

    #[test]
    fn test_extract_provider_error_details_reads_message_type_and_code() {
        let body = r#"{"error":{"message":"模型不存在，请检查模型代码。","type":"invalid_request_error","code":"model_not_found"}} "#;

        let details = extract_provider_error_details(body).expect("expected json error details");

        assert_eq!(details.message, "模型不存在，请检查模型代码。");
        assert_eq!(details.error_type.as_deref(), Some("invalid_request_error"));
        assert_eq!(details.code.as_deref(), Some("model_not_found"));
    }

    #[test]
    fn test_proxy_env_summary_includes_present_values() {
        let summary = proxy_env_summary(&ProxyEnv {
            all: None,
            http: Some("http://proxy.example.com:8080".to_string()),
            https: Some("http://secure-proxy.example.com:8443".to_string()),
        });

        assert_eq!(summary["http"], "http://proxy.example.com:8080");
        assert_eq!(summary["https"], "http://secure-proxy.example.com:8443");
        assert_eq!(summary["all"], serde_json::Value::Null);
    }

    #[test]
    fn test_format_request_body_for_log_keeps_full_body() {
        let body = "abcdefghijklmnopqrstuvwxyz";

        let logged = format_request_body_for_log(body);

        assert_eq!(logged, body);
    }
}
