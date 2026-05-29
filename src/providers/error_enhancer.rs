//! Error Enhancement System
//! 
//! This module provides enhanced error messages with hints and
//! provider-specific error handling.

use crate::providers::ProviderError;

/// Enhanced error with hints and provider-specific information
#[derive(Debug, Clone)]
pub struct EnhancedError {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional hint for fixing the error
    pub hint: Option<String>,
    /// Provider name (if applicable)
    pub provider: Option<String>,
    /// Original error (for debugging)
    pub original: Option<String>,
}

impl EnhancedError {
    /// Create a new enhanced error
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            hint: None,
            provider: None,
            original: None,
        }
    }

    /// Create with a hint
    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string());
        self
    }

    /// Create with provider info
    pub fn with_provider(mut self, provider: &str) -> Self {
        self.provider = Some(provider.to_string());
        self
    }

    /// Create with original error
    pub fn with_original(mut self, original: &str) -> Self {
        self.original = Some(original.to_string());
        self
    }

    /// Convert from ProviderError
    pub fn from_provider_error(error: ProviderError, provider: &str) -> Self {
        match &error {
            ProviderError::InvalidRequest(msg) => {
                Self::new("invalid_request", msg)
                    .with_provider(provider)
                    .with_original(&error.to_string())
            }
            ProviderError::AuthFailed(msg) => {
                Self::new("authentication_error", msg)
                    .with_hint("Check your API key is correct and has not expired.")
                    .with_provider(provider)
                    .with_original(&error.to_string())
            }
            ProviderError::RateLimited(msg) => {
                Self::new("rate_limit_exceeded", msg)
                    .with_hint("Wait a moment and retry, or implement exponential backoff.")
                    .with_provider(provider)
                    .with_original(&error.to_string())
            }
            ProviderError::InvalidResponse(msg) => {
                Self::new("provider_error", msg)
                    .with_provider(provider)
                    .with_original(&error.to_string())
            }
            ProviderError::RequestFailed(msg) => {
                Self::new("request_failed", msg)
                    .with_hint("Check your internet connection and try again.")
                    .with_provider(provider)
                    .with_original(&error.to_string())
            }
            _ => {
                Self::new("unknown_error", &error.to_string())
                    .with_provider(provider)
            }
        }
    }

    /// Detect context overflow from error response
    pub fn detect_context_overflow(status: u16, body: &str) -> Option<Self> {
        // Common context overflow patterns
        let overflow_patterns = [
            "context_length_exceeded",
            "context_overflow",
            "maximum context length",
            "too many tokens",
            "token limit",
            "maximum tokens",
            "context window exceeded",
            "input too long",
            "exceeds maximum length",
            "exceeds the maximum",
            "prompt is too long",
            "input length",
            "exceed context",
            "上下文",
        ];

        let body_lower = body.to_lowercase();
        
        if status == 400 || status == 422 {
            for pattern in &overflow_patterns {
                if body_lower.contains(&pattern.to_lowercase()) {
                    return Some(Self::new(
                        "context_overflow",
                        "Request exceeds model's context window.",
                    )
                    .with_hint("Reduce conversation length or use a model with larger context window."));
                }
            }
        }

        None
    }

    /// Detect web search disabled error
    pub fn detect_web_search_disabled(body: &str) -> Option<Self> {
        let patterns = [
            // MiMo specific marker
            "webSearchEnabled is false",
            // Standard patterns
            "web_search_enabled is false",
            "web_search_enabled is false",
            "web search is not enabled",
            "search is not enabled",
            "search_enabled is false",
            "web search plugin",
            "plugin not activated",
        ];

        let body_lower = body.to_lowercase();
        for pattern in &patterns {
            if body_lower.contains(&pattern.to_lowercase()) {
                return Some(Self::new(
                    "web_search_plugin_not_activated",
                    "MiMo Web Search Plugin is not activated for this account.",
                )
                .with_hint("Activate the Web Search Plugin at https://platform.xiaomimimo.com/#/console/plugin (separately billed) and restart the proxy."));
            }
        }

        None
    }

    /// Convert to JSON for API response
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "hint": self.hint,
                "provider": self.provider,
            }
        })
    }
}

impl std::fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\nHint: {}", hint)?;
        }
        if let Some(provider) = &self.provider {
            write!(f, "\nProvider: {}", provider)?;
        }
        Ok(())
    }
}

impl std::error::Error for EnhancedError {}

/// Error enhancer trait for provider-specific error handling
pub trait ErrorEnhancer: Send + Sync {
    /// Enhance an error with provider-specific information
    fn enhance(&self, error: ProviderError) -> EnhancedError;
    
    /// Check if this enhancer handles a specific error code
    fn handles(&self, error: &ProviderError) -> bool;
}

/// Default error enhancer
pub struct DefaultErrorEnhancer;

impl ErrorEnhancer for DefaultErrorEnhancer {
    fn enhance(&self, error: ProviderError) -> EnhancedError {
        EnhancedError::from_provider_error(error, "unknown")
    }

    fn handles(&self, _error: &ProviderError) -> bool {
        true // Default enhancer handles all errors
    }
}

/// Context overflow detector
pub struct ContextOverflowDetector;

impl ContextOverflowDetector {
    /// Check if an error indicates context overflow
    pub fn is_context_overflow(status: u16, body: &str) -> bool {
        EnhancedError::detect_context_overflow(status, body).is_some()
    }

    /// Create a context overflow hint based on the model
    pub fn create_hint(model: &str) -> String {
        format!(
            "For {} model, try:\n\
            1. Shortening the conversation history\n\
            2. Using a summary to condense context\n\
            3. Splitting into multiple smaller requests",
            model
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = EnhancedError::new("test_error", "Test message");
        assert_eq!(error.code, "test_error");
        assert_eq!(error.message, "Test message");
        assert!(error.hint.is_none());
    }

    #[test]
    fn test_error_with_hints() {
        let error = EnhancedError::new("test", "message")
            .with_hint("Try this instead")
            .with_provider("test_provider");
        
        assert!(error.hint.is_some());
        assert!(error.provider.is_some());
    }

    #[test]
    fn test_context_overflow_detection() {
        let body = "Error: context_length_exceeded for model gpt-4";
        let result = EnhancedError::detect_context_overflow(400, body);
        
        assert!(result.is_some());
        let error = result.unwrap();
        assert_eq!(error.code, "context_overflow");
    }

    #[test]
    fn test_web_search_disabled() {
        let body = "web_search_enabled is false for this model";
        let result = EnhancedError::detect_web_search_disabled(body);

        assert!(result.is_some());
        let error = result.unwrap();
        assert_eq!(error.code, "web_search_plugin_not_activated");
        assert!(error.hint.is_some()); // Should have activation hint
    }
}

#[cfg(test)]
mod mimo2codex_tests {
    use super::*;

    // Tests from mimo2codex/test/upstream.contextOverflow.test.ts

    #[test]
    fn test_matches_openai_context_length_exceeded_code() {
        let body = r#"{"error":{"code":"context_length_exceeded","message":"This model's maximum context length is 8192 tokens."}}"#;
        let result = EnhancedError::detect_context_overflow(400, body);
        assert!(result.is_some());
        // Code should indicate context overflow (exact code may vary)
        let err = result.as_ref().unwrap(); assert!(err.code.contains("context") || err.code.contains("overflow"));
    }

    #[test]
    fn test_matches_classic_maximum_context_length() {
        let body = "This model's maximum context length is 128000 tokens, but the request exceeded that.";
        let result = EnhancedError::detect_context_overflow(400, body);
        assert!(result.is_some());
    }

    #[test]
    fn test_matches_prompt_too_long() {
        let body = "prompt is too long: 12345 tokens > 8192 maximum";
        let result = EnhancedError::detect_context_overflow(400, body);
        assert!(result.is_some());
    }

    #[test]
    fn test_matches_deepseek_input_length_exceed() {
        let body = "input length and `max_tokens` exceed context limit";
        let result = EnhancedError::detect_context_overflow(400, body);
        assert!(result.is_some());
    }

    #[test]
    fn test_matches_chinese_context_length() {
        let body = "请求失败：上下文长度超出模型限制";
        let result = EnhancedError::detect_context_overflow(400, body);
        assert!(result.is_some());
    }

    #[test]
    fn test_matches_tokens_exceeds_maximum() {
        let body = "Requested 200000 tokens exceeds the maximum of 128000 supported by this model.";
        let result = EnhancedError::detect_context_overflow(400, body);
        assert!(result.is_some());
    }

    #[test]
    fn test_does_not_match_websearch_disabled() {
        // This should NOT match - web search disabled is handled separately
        let body = r#"{"error":{"code":"plugin_disabled","message":"webSearchEnabled is false, please enable Web Search Plugin"}}"#;
        let result = EnhancedError::detect_context_overflow(400, body);
        assert!(result.is_none());
    }

    #[test]
    fn test_does_not_match_model_not_found() {
        let body = "invalid request: model not found";
        let result = EnhancedError::detect_context_overflow(400, body);
        assert!(result.is_none());
    }

    #[test]
    fn test_does_not_trigger_on_non_400() {
        let body = "context length exceeded the cap";
        let result = EnhancedError::detect_context_overflow(401, body);
        assert!(result.is_none());
    }

    #[test]
    fn test_does_not_trigger_on_5xx() {
        let body = "context_length_exceeded";
        let result = EnhancedError::detect_context_overflow(500, body);
        assert!(result.is_none());
    }

    #[test]
    fn test_returns_null_on_empty_body() {
        let result = EnhancedError::detect_context_overflow(400, "");
        assert!(result.is_none());
    }

    #[test]
    fn test_websearch_error_detection() {
        let body = "web_search_enabled is false, please enable Web Search Plugin";
        let result = EnhancedError::detect_web_search_disabled(body);
        assert!(result.is_some());
        assert_eq!(result.unwrap().code, "web_search_plugin_not_activated");
    }

    #[test]
    fn test_websearch_error_case_insensitive() {
        let body = "WEB_SEARCH_ENABLED IS FALSE";
        let result = EnhancedError::detect_web_search_disabled(body);
        assert!(result.is_some());
    }

    #[test]
    fn test_mimo_websearch_error_detection() {
        // MiMo specific marker
        let body = "webSearchEnabled is false, please enable Web Search Plugin";
        let result = EnhancedError::detect_web_search_disabled(body);
        assert!(result.is_some());
        let error = result.unwrap();
        assert_eq!(error.code, "web_search_plugin_not_activated");
        assert!(error.hint.is_some()); // Should have activation hint
    }

    #[test]
    fn test_plugin_not_activated_error() {
        // mimo2codex error pattern
        let body = r#"{"error":{"code":"plugin_disabled","message":"webSearchEnabled is false, please enable Web Search Plugin"}}"#;
        let result = EnhancedError::detect_web_search_disabled(body);
        assert!(result.is_some());
        let error = result.unwrap();
        assert_eq!(error.code, "web_search_plugin_not_activated");
    }
}
