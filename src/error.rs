use thiserror::Error;

/// Error with optional Chinese translation
#[derive(Error, Debug)]
pub enum Error {
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Invalid Request: {0}")]
    InvalidRequest(String),

    #[error("Not Found: {0}")]
    NotFound(String),
}

impl Error {
    /// Get localized error message based on Accept-Language header
    pub fn localized(&self, accept_language: Option<&str>) -> String {
        let lang = accept_language.unwrap_or("en");

        if lang.starts_with("zh") {
            self.zh_message()
        } else {
            self.to_string()
        }
    }

    /// Get Chinese error message
    pub fn zh_message(&self) -> String {
        match self {
            Error::Request(e) => format!("请求错误: {}", e),
            Error::Provider(msg) => format!("Provider 错误: {}", msg),
            Error::Io(e) => format!("IO 错误: {}", e),
            Error::Json(e) => format!("JSON 错误: {}", e),
            Error::Yaml(e) => format!("YAML 错误: {}", e),
            Error::BadRequest(msg) => format!("请求格式错误: {}", msg),
            Error::Auth(msg) => format!("认证错误: {}", msg),
            Error::Internal(msg) => format!("内部错误: {}", msg),
            Error::InvalidRequest(msg) => format!("无效请求: {}", msg),
            Error::NotFound(msg) => format!("未找到: {}", msg),
        }
    }
}

/// Common error messages in both English and Chinese
pub mod messages {
    /// Context length exceeded error
    pub fn context_length_exceeded() -> &'static str {
        "Context length exceeded (上下文超出模型限制)"
    }

    /// Rate limit error
    pub fn rate_limit() -> &'static str {
        "Rate limit exceeded (请求频率超限)"
    }

    /// Authentication error
    pub fn auth_failed() -> &'static str {
        "Authentication failed (认证失败)"
    }

    /// Model not found error
    pub fn model_not_found(model: &str) -> String {
        format!("Model not found: {} (模型不存在: {})", model, model)
    }

    /// Invalid API key error
    pub fn invalid_api_key() -> &'static str {
        "Invalid API key (API密钥无效)"
    }

    /// Network error
    pub fn network_error() -> &'static str {
        "Network error, please check your connection (网络错误，请检查网络连接)"
    }

    /// Service unavailable error
    pub fn service_unavailable() -> &'static str {
        "Service temporarily unavailable (服务暂时不可用)"
    }

    /// Invalid request error
    pub fn invalid_request(msg: &str) -> String {
        format!("Invalid request: {} (无效请求: {})", msg, msg)
    }

    /// Provider not configured error
    pub fn provider_not_configured(provider: &str) -> String {
        format!(
            "Provider '{}' not configured (Provider '{}' 未配置)",
            provider, provider
        )
    }

    /// Provider not supported feature error
    pub fn feature_not_supported(provider: &str, feature: &str) -> String {
        format!(
            "Feature '{}' not supported by provider '{}' (Provider '{}' 不支持 '{}' 功能)",
            feature, provider, provider, feature
        )
    }
}
