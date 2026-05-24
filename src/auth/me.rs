//!
//! User /me Endpoints Module
//! 
//! Rust implementation aligned with mimo2codex me.endpoints.test.ts
//! 
//! Features:
//! - User profile retrieval
//! - User settings management
//! - API key management

use serde::{Deserialize, Serialize};

/// User profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub created_at: String,
}

impl UserProfile {
    pub fn new(id: i64, username: &str) -> Self {
        Self {
            id,
            username: username.to_string(),
            email: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    pub fn with_email(mut self, email: &str) -> Self {
        self.email = Some(email.to_string());
        self
    }
}

/// User settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSettings {
    pub user_id: i64,
    pub disable_thinking: Option<bool>,
    pub theme: Option<String>,
    pub language: Option<String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            user_id: 0,
            disable_thinking: None,
            theme: Some("system".to_string()),
            language: Some("en".to_string()),
        }
    }
}

/// API key info
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub created_at: String,
    pub last_used: Option<String>,
    pub is_active: bool,
}

impl ApiKeyInfo {
    pub fn new(name: &str, key_prefix: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            key_prefix: key_prefix.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_used: None,
            is_active: true,
        }
    }
}

/// Me endpoint response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeResponse {
    pub user: Option<UserProfile>,
    pub settings: Option<UserSettings>,
    pub api_keys: Vec<ApiKeyInfo>,
    pub is_authenticated: bool,
}

impl MeResponse {
    pub fn authenticated(user: UserProfile) -> Self {
        Self {
            user: Some(user),
            settings: None,
            api_keys: vec![],
            is_authenticated: true,
        }
    }
    
    pub fn anonymous() -> Self {
        Self {
            user: None,
            settings: None,
            api_keys: vec![],
            is_authenticated: false,
        }
    }
}

/// Get user profile by ID
pub fn get_user_profile(user_id: i64) -> Option<UserProfile> {
    // In real implementation, would query database
    if user_id > 0 {
        Some(UserProfile::new(user_id, "user"))
    } else {
        None
    }
}

/// Get user settings
pub fn get_user_settings(user_id: i64) -> Option<UserSettings> {
    // In real implementation, would query database
    if user_id > 0 {
        let mut settings = UserSettings::default();
        settings.user_id = user_id;
        Some(settings)
    } else {
        None
    }
}

/// List user's API keys
pub fn list_api_keys(user_id: i64) -> Vec<ApiKeyInfo> {
    // In real implementation, would query database
    if user_id > 0 {
        vec![
            ApiKeyInfo::new("Default Key", "sk-test"),
        ]
    } else {
        vec![]
    }
}

/// Build me response for authenticated user
pub fn build_me_response(user_id: i64) -> MeResponse {
    if let Some(user) = get_user_profile(user_id) {
        let settings = get_user_settings(user_id);
        let api_keys = list_api_keys(user_id);
        MeResponse {
            user: Some(user),
            settings,
            api_keys,
            is_authenticated: true,
        }
    } else {
        MeResponse::anonymous()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_profile_new() {
        let profile = UserProfile::new(1, "testuser");
        assert_eq!(profile.id, 1);
        assert_eq!(profile.username, "testuser");
        assert!(profile.email.is_none());
    }

    #[test]
    fn test_user_profile_with_email() {
        let profile = UserProfile::new(1, "testuser").with_email("test@example.com");
        assert_eq!(profile.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_user_settings_default() {
        let settings = UserSettings::default();
        assert_eq!(settings.user_id, 0);
        assert_eq!(settings.theme, Some("system".to_string()));
        assert_eq!(settings.language, Some("en".to_string()));
    }

    #[test]
    fn test_api_key_info_new() {
        let key = ApiKeyInfo::new("My Key", "sk-abc");
        assert!(!key.id.is_empty());
        assert_eq!(key.name, "My Key");
        assert_eq!(key.key_prefix, "sk-abc");
        assert!(key.is_active);
    }

    #[test]
    fn test_me_response_authenticated() {
        let user = UserProfile::new(1, "testuser");
        let response = MeResponse::authenticated(user);
        assert!(response.is_authenticated);
        assert!(response.user.is_some());
        assert_eq!(response.user.unwrap().username, "testuser");
    }

    #[test]
    fn test_me_response_anonymous() {
        let response = MeResponse::anonymous();
        assert!(!response.is_authenticated);
        assert!(response.user.is_none());
        assert!(response.api_keys.is_empty());
    }

    #[test]
    fn test_get_user_profile_valid_id() {
        let profile = get_user_profile(42);
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().id, 42);
    }

    #[test]
    fn test_get_user_profile_invalid_id() {
        let profile = get_user_profile(0);
        assert!(profile.is_none());
    }

    #[test]
    fn test_get_user_settings_valid_id() {
        let settings = get_user_settings(42);
        assert!(settings.is_some());
        assert_eq!(settings.unwrap().user_id, 42);
    }

    #[test]
    fn test_list_api_keys_valid_user() {
        let keys = list_api_keys(42);
        assert!(!keys.is_empty());
    }

    #[test]
    fn test_list_api_keys_invalid_user() {
        let keys = list_api_keys(0);
        assert!(keys.is_empty());
    }

    #[test]
    fn test_build_me_response_authenticated() {
        let response = build_me_response(42);
        assert!(response.is_authenticated);
        assert!(response.user.is_some());
        assert!(response.settings.is_some());
    }

    #[test]
    fn test_build_me_response_anonymous() {
        let response = build_me_response(0);
        assert!(!response.is_authenticated);
        assert!(response.user.is_none());
    }
}
