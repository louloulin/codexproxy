//!
//! Upstream Proxy Dispatcher Module
//! 
//! Rust implementation aligned with mimo2codex upstream.proxyDispatcher.test.ts
//! 
//! Features:
//! - Install proxy dispatcher from environment variables
//! - Redact proxy URLs (mask passwords)
//! - Support HTTP_PROXY, HTTPS_PROXY, and lowercase variants

/// Proxy configuration status
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyStatus {
    /// Whether proxy is enabled
    pub enabled: bool,
    /// HTTPS proxy URL
    pub https_proxy: Option<String>,
    /// HTTP proxy URL
    pub http_proxy: Option<String>,
    /// No-proxy list
    pub no_proxy: Option<String>,
    /// Reason for disabled state
    pub reason: Option<String>,
}

/// Install proxy dispatcher from environment variables
/// 
/// Supports:
/// - HTTPS_PROXY / https_proxy
/// - HTTP_PROXY / http_proxy
/// - NO_PROXY / no_proxy
/// - MIMO2CODEX_NO_PROXY_FROM_ENV to opt out
/// 
/// Returns ProxyStatus with enabled state and proxy URLs
pub fn install_proxy_dispatcher_from_env() -> ProxyStatus {
    // Check for opt-out first
    if std::env::var("MIMO2CODEX_NO_PROXY_FROM_ENV").is_ok() {
        return ProxyStatus {
            enabled: false,
            https_proxy: None,
            http_proxy: None,
            no_proxy: None,
            reason: Some("opted-out".to_string()),
        };
    }
    
    // Prefer uppercase (curl convention)
    let https_proxy = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .ok();
    let http_proxy = std::env::var("HTTP_PROXY")
        .or_else(|_| std::env::var("http_proxy"))
        .ok();
    let no_proxy = std::env::var("NO_PROXY")
        .or_else(|_| std::env::var("no_proxy"))
        .ok();
    
    if https_proxy.is_none() && http_proxy.is_none() {
        return ProxyStatus {
            enabled: false,
            https_proxy: None,
            http_proxy: None,
            no_proxy: None,
            reason: Some("no-env".to_string()),
        };
    }
    
    ProxyStatus {
        enabled: true,
        https_proxy,
        http_proxy,
        no_proxy,
        reason: None,
    }
}

/// Redact proxy URL by masking passwords
/// 
/// Example:
/// - "http://user:secret@p:8080" -> "http://user:***@p:8080/"
/// - "http://p:8080/" -> "http://p:8080/" (unchanged)
pub fn redact_proxy_url(url: &str) -> String {
    // Parse URL and redact userinfo password
    if let Ok(parsed) = url::Url::parse(url) {
        let username = parsed.username();
        // Check if there's a password and username exists
        if !username.is_empty() && parsed.password().is_some() {
            // Reconstruct with redacted password
            let host = parsed.host_str().unwrap_or("");
            let port = parsed.port().map(|p| format!(":{}", p)).unwrap_or_default();
            let path = parsed.path();
            return format!("http://{}:***@{}{}{}", username, host, port, path);
        }
    }
    
    // If parsing failed or no userinfo, return as-is
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_no_env_when_no_proxy_set() {
        // Clear proxy env vars
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("https_proxy");
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("http_proxy");
        std::env::remove_var("MIMO2CODEX_NO_PROXY_FROM_ENV");
        
        let status = install_proxy_dispatcher_from_env();
        assert!(!status.enabled);
        assert_eq!(status.reason, Some("no-env".to_string()));
    }

    #[test]
    fn test_installs_dispatcher_when_https_proxy_set() {
        std::env::remove_var("MIMO2CODEX_NO_PROXY_FROM_ENV");
        std::env::set_var("HTTPS_PROXY", "http://proxy.local:8080");
        
        let status = install_proxy_dispatcher_from_env();
        assert!(status.enabled);
        assert_eq!(status.https_proxy, Some("http://proxy.local:8080".to_string()));
        
        std::env::remove_var("HTTPS_PROXY");
    }

    #[test]
    fn test_recognizes_lowercase_aliases() {
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("NO_PROXY");
        std::env::remove_var("MIMO2CODEX_NO_PROXY_FROM_ENV");
        std::env::set_var("https_proxy", "http://lower.local:1080");
        std::env::set_var("http_proxy", "http://lower.local:1080");
        std::env::set_var("no_proxy", "localhost,127.0.0.1");
        
        let status = install_proxy_dispatcher_from_env();
        assert!(status.enabled);
        assert_eq!(status.https_proxy, Some("http://lower.local:1080".to_string()));
        assert_eq!(status.http_proxy, Some("http://lower.local:1080".to_string()));
        assert_eq!(status.no_proxy, Some("localhost,127.0.0.1".to_string()));
        
        std::env::remove_var("https_proxy");
        std::env::remove_var("http_proxy");
        std::env::remove_var("no_proxy");
    }

    #[test]
    fn test_prefers_uppercase_env_vars() {
        std::env::set_var("HTTPS_PROXY", "http://upper.local:8080");
        std::env::set_var("https_proxy", "http://lower.local:1080");
        std::env::remove_var("MIMO2CODEX_NO_PROXY_FROM_ENV");
        
        let status = install_proxy_dispatcher_from_env();
        assert_eq!(status.https_proxy, Some("http://upper.local:8080".to_string()));
        
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("https_proxy");
    }

    #[test]
    fn test_opts_out_when_mimo2codex_no_proxy_from_env_set() {
        std::env::set_var("HTTPS_PROXY", "http://proxy.local:8080");
        std::env::set_var("MIMO2CODEX_NO_PROXY_FROM_ENV", "1");
        
        let status = install_proxy_dispatcher_from_env();
        assert!(!status.enabled);
        assert_eq!(status.reason, Some("opted-out".to_string()));
        
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("MIMO2CODEX_NO_PROXY_FROM_ENV");
    }

    #[test]
    fn test_redact_proxy_url_masks_password() {
        let result = redact_proxy_url("http://user:secret@p:8080");
        assert_eq!(result, "http://user:***@p:8080/");
    }

    #[test]
    fn test_redact_proxy_url_leaves_passwordless_untouched() {
        let result = redact_proxy_url("http://p:8080/");
        assert_eq!(result, "http://p:8080/");
    }

    #[test]
    fn test_redact_proxy_url_handles_no_username() {
        let result = redact_proxy_url("http://proxy.local:8080");
        assert_eq!(result, "http://proxy.local:8080");
    }
}
