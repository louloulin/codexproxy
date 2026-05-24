//!
//! Authentication Module
//! 
//! Rust implementation with tests aligned to mimo2codex auth.flow.test.ts
//! 
//! Features:
//! - Auth mode: "off" (local mode) vs "on" (authenticated)
//! - Session management
//! - Cookie parsing
//! - Bearer token validation
//! - User /me endpoints

pub mod me;

use serde::{Deserialize, Serialize};

pub use me::{ApiKeyInfo, MeResponse, UserProfile, UserSettings, build_me_response, get_user_profile, get_user_settings, list_api_keys};

/// Authentication mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMode {
    /// Local mode - no authentication required
    Off,
    /// Authenticated mode - requires session cookies or bearer tokens
    On,
}

impl AuthMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "on" | "1" | "true" => AuthMode::On,
            _ => AuthMode::Off,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthMode::Off => "off",
            AuthMode::On => "on",
        }
    }
}

/// Session cookie parser
pub struct SessionCookie;

impl SessionCookie {
    /// Extract session ID from Set-Cookie header
    pub fn extract_session_id(set_cookie: &str) -> Option<String> {
        let prefix = "m2c_session=";
        if let Some(start) = set_cookie.find(prefix) {
            let after_prefix = &set_cookie[start + prefix.len()..];
            if let Some(end) = after_prefix.find(';') {
                return Some(after_prefix[..end].to_string());
            } else {
                return Some(after_prefix.to_string());
            }
        }
        None
    }
    
    /// Check if cookie string contains valid session
    pub fn has_valid_session(cookie: &str) -> bool {
        cookie.contains("m2c_session=")
    }
}

/// Bearer token parser
pub struct BearerToken;

impl BearerToken {
    /// Extract token from Authorization header value
    pub fn extract_token(auth_header: &str) -> Option<String> {
        let prefix = "Bearer ";
        if auth_header.starts_with(prefix) {
            Some(auth_header[prefix.len()..].to_string())
        } else {
            None
        }
    }
    
    /// Check if header value is a bearer token
    pub fn is_bearer(auth_header: &str) -> bool {
        auth_header.starts_with("Bearer ")
    }
}

/// Auth context - user information extracted from auth headers
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthContext {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub is_authenticated: bool,
}

impl AuthContext {
    pub fn anonymous() -> Self {
        Self {
            user_id: None,
            username: None,
            is_authenticated: false,
        }
    }
    
    pub fn authenticated(user_id: i64, username: &str) -> Self {
        Self {
            user_id: Some(user_id),
            username: Some(username.to_string()),
            is_authenticated: true,
        }
    }
}

/// Check if a path is a public endpoint (no auth required)
pub fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/admin/api/health" 
        | "/admin/api/version"
        | "/health"
        | "/"
    )
}

/// Auth guard result
#[derive(Debug, Clone)]
pub struct AuthGuardResult {
    pub handled: bool,
    pub context: AuthContext,
}

impl AuthGuardResult {
    /// Create a result that indicates the request was handled (auth required, 401 sent)
    pub fn handled() -> Self {
        Self {
            handled: true,
            context: AuthContext::anonymous(),
        }
    }
    
    /// Create a result that allows the request to proceed
    pub fn proceed(context: AuthContext) -> Self {
        Self {
            handled: false,
            context,
        }
    }
}

/// Check auth mode and determine if request should proceed
pub fn check_auth(
    auth_mode: &AuthMode,
    cookie: Option<&str>,
    bearer: Option<&str>,
) -> AuthGuardResult {
    match auth_mode {
        AuthMode::Off => {
            // In local mode, always allow
            AuthGuardResult::proceed(AuthContext::anonymous())
        }
        AuthMode::On => {
            // In auth mode, require either cookie or bearer token
            if let Some(cookie_val) = cookie {
                if SessionCookie::has_valid_session(cookie_val) {
                    // In real impl, would look up session in DB
                    return AuthGuardResult::proceed(AuthContext::anonymous());
                }
            }
            
            if let Some(bearer_val) = bearer {
                if let Some(token) = BearerToken::extract_token(bearer_val) {
                    if !token.is_empty() {
                        // In real impl, would validate token
                        return AuthGuardResult::proceed(AuthContext::anonymous());
                    }
                }
            }
            
            // No valid auth credentials
            AuthGuardResult::handled()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_mode_from_str() {
        assert_eq!(AuthMode::from_str("off"), AuthMode::Off);
        assert_eq!(AuthMode::from_str("on"), AuthMode::On);
        assert_eq!(AuthMode::from_str("1"), AuthMode::On);
        assert_eq!(AuthMode::from_str("true"), AuthMode::On);
        assert_eq!(AuthMode::from_str("anything"), AuthMode::Off);
    }

    #[test]
    fn test_auth_mode_as_str() {
        assert_eq!(AuthMode::Off.as_str(), "off");
        assert_eq!(AuthMode::On.as_str(), "on");
    }

    #[test]
    fn test_session_cookie_extract_session_id() {
        let cookie = "m2c_session=abc123; Path=/";
        assert_eq!(SessionCookie::extract_session_id(cookie), Some("abc123".to_string()));
        
        let cookie_no_path = "m2c_session=xyz789";
        assert_eq!(SessionCookie::extract_session_id(cookie_no_path), Some("xyz789".to_string()));
        
        let no_session = "other=value";
        assert_eq!(SessionCookie::extract_session_id(no_session), None);
    }

    #[test]
    fn test_session_cookie_has_valid_session() {
        assert!(SessionCookie::has_valid_session("m2c_session=abc123"));
        assert!(!SessionCookie::has_valid_session("other=value"));
    }

    #[test]
    fn test_bearer_token_extract_token() {
        assert_eq!(BearerToken::extract_token("Bearer abc123"), Some("abc123".to_string()));
        assert_eq!(BearerToken::extract_token("Bearer "), Some("".to_string()));
        assert_eq!(BearerToken::extract_token("Basic abc"), None);
    }

    #[test]
    fn test_bearer_token_is_bearer() {
        assert!(BearerToken::is_bearer("Bearer abc123"));
        assert!(!BearerToken::is_bearer("Basic abc"));
        assert!(!BearerToken::is_bearer("bearer lowercase"));
    }

    #[test]
    fn test_auth_context_anonymous() {
        let ctx = AuthContext::anonymous();
        assert!(!ctx.is_authenticated);
        assert!(ctx.user_id.is_none());
        assert!(ctx.username.is_none());
    }

    #[test]
    fn test_auth_context_authenticated() {
        let ctx = AuthContext::authenticated(42, "testuser");
        assert!(ctx.is_authenticated);
        assert_eq!(ctx.user_id, Some(42));
        assert_eq!(ctx.username, Some("testuser".to_string()));
    }

    #[test]
    fn test_is_public_path() {
        assert!(is_public_path("/admin/api/health"));
        assert!(is_public_path("/admin/api/version"));
        assert!(is_public_path("/health"));
        assert!(is_public_path("/"));
        assert!(!is_public_path("/v1/chat"));
        assert!(!is_public_path("/admin/api/keys"));
    }

    #[test]
    fn test_check_auth_local_mode_always_allows() {
        let result = check_auth(&AuthMode::Off, None, None);
        assert!(!result.handled);
        assert!(!result.context.is_authenticated);
        
        // Even with valid creds, local mode allows
        let result = check_auth(
            &AuthMode::Off,
            Some("m2c_session=abc"),
            Some("Bearer xyz")
        );
        assert!(!result.handled);
    }

    #[test]
    fn test_check_auth_on_mode_requires_credentials() {
        // No credentials - handled
        let result = check_auth(&AuthMode::On, None, None);
        assert!(result.handled);
        
        // Empty cookie - handled
        let result = check_auth(&AuthMode::On, Some(""), None);
        assert!(result.handled);
        
        // Valid session cookie - allowed
        let result = check_auth(&AuthMode::On, Some("m2c_session=abc123"), None);
        assert!(!result.handled);
        
        // Valid bearer token - allowed
        let result = check_auth(&AuthMode::On, None, Some("Bearer xyz789"));
        assert!(!result.handled);
    }

    #[test]
    fn test_auth_guard_result_handled() {
        let result = AuthGuardResult::handled();
        assert!(result.handled);
        assert!(!result.context.is_authenticated);
    }

    #[test]
    fn test_auth_guard_result_proceed() {
        let ctx = AuthContext::authenticated(1, "admin");
        let result = AuthGuardResult::proceed(ctx);
        assert!(!result.handled);
        assert!(result.context.is_authenticated);
    }
}
