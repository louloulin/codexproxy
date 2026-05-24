//!
//! Base URL Resolution and Configuration
//! 
//! Rust implementation aligned with mimo2codex config.baseUrl.test.ts
//! 
//! Features:
//! - Token plan detection (tp-* vs sk-* keys)
//! - Base URL resolution priority: CLI > ENV > Key inference
//! - Provider-specific defaults

use serde::{Deserialize, Serialize};

/// Provider base URLs
pub const MIMO_BASE_URL_DEFAULT: &str = "https://api.xiaomimimo.com/v1";
pub const MIMO_BASE_URL_TOKEN_PLAN: &str = "https://token-plan-cn.xiaomimimo.com/v1";
pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";

/// Check if API key indicates token plan (starts with "tp-")
pub fn is_token_plan_key(api_key: &str) -> bool {
    api_key.starts_with("tp-")
}

/// Determine the base URL based on API key type and explicit settings
/// 
/// Priority:
/// 1. Explicit --base-url CLI argument (highest)
/// 2. MIMO_BASE_URL environment variable
/// 3. Key-based inference (tp-* → token-plan, sk-* → default)
pub fn resolve_mimo_base_url(
    explicit_base_url: Option<&str>,
    env_base_url: Option<&str>,
    api_key: &str,
) -> (String, bool) {
    // Priority 1: Explicit CLI argument
    if let Some(url) = explicit_base_url {
        let is_tp = url.contains("token-plan");
        return (url.to_string(), is_tp);
    }
    
    // Priority 2: Environment variable
    if let Some(url) = env_base_url {
        let is_tp = url.contains("token-plan");
        return (url.to_string(), is_tp);
    }
    
    // Priority 3: Key-based inference
    if is_token_plan_key(api_key) {
        (MIMO_BASE_URL_TOKEN_PLAN.to_string(), true)
    } else {
        (MIMO_BASE_URL_DEFAULT.to_string(), false)
    }
}

/// Parse API key prefix to determine provider type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeyType {
    /// Token plan key (tp-*)
    TokenPlan,
    /// Standard pay-as-you-go key (sk-*)
    Standard,
    /// Unknown prefix
    Unknown,
}

impl ApiKeyType {
    pub fn from_key(key: &str) -> Self {
        if key.starts_with("tp-") {
            ApiKeyType::TokenPlan
        } else if key.starts_with("sk-") {
            ApiKeyType::Standard
        } else {
            ApiKeyType::Unknown
        }
    }
}

/// Provider preset identifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPreset {
    pub id: String,
    pub match_base_url: Vec<String>,
    pub match_model_prefix: Vec<String>,
    pub recommended_spec: ProviderSpec,
}

/// Provider specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSpec {
    pub base_url: String,
    pub features: ProviderFeatures,
}

/// Provider-specific features
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderFeatures {
    #[serde(default)]
    pub drop_reasoning_effort: bool,
    #[serde(default)]
    pub drop_response_format: bool,
    #[serde(default)]
    pub enhance_error_preset: Option<String>,
}

/// Built-in provider presets
pub fn get_provider_presets() -> Vec<ProviderPreset> {
    vec![
        ProviderPreset {
            id: "kimi".to_string(),
            match_base_url: vec![
                "api.moonshot.cn".to_string(),
                "api.moonshot.ai".to_string(),
            ],
            match_model_prefix: vec!["kimi-".to_string(), "moonshot-v1-".to_string()],
            recommended_spec: ProviderSpec {
                base_url: "https://api.moonshot.cn/v1".to_string(),
                features: ProviderFeatures {
                    drop_reasoning_effort: true,
                    drop_response_format: false,
                    enhance_error_preset: None,
                },
            },
        },
        ProviderPreset {
            id: "sensenova".to_string(),
            match_base_url: vec!["sensenova.cn".to_string()],
            match_model_prefix: vec![
                "sensenova-".to_string(),
                "deepseek-v4-flash".to_string(), // Note: sensenova uses this prefix
            ],
            recommended_spec: ProviderSpec {
                base_url: "https://token.sensenova.cn/v1".to_string(),
                features: ProviderFeatures {
                    drop_reasoning_effort: false,
                    drop_response_format: true,
                    enhance_error_preset: Some("sensenova".to_string()),
                },
            },
        },
        ProviderPreset {
            id: "minimax".to_string(),
            match_base_url: vec!["api.minimaxi.com".to_string()],
            match_model_prefix: vec!["MiniMax-".to_string(), "abab".to_string()],
            recommended_spec: ProviderSpec {
                base_url: "https://api.minimaxi.com/v1".to_string(),
                features: ProviderFeatures {
                    drop_reasoning_effort: false,
                    drop_response_format: false,
                    enhance_error_preset: None,
                },
            },
        },
    ]
}

/// Match a preset by base URL or model prefix
pub fn match_preset(base_url: &str, model: &str) -> Option<ProviderPreset> {
    let presets = get_provider_presets();
    
    // First try base URL match (higher priority)
    for preset in &presets {
        for pattern in &preset.match_base_url {
            if base_url.to_lowercase().contains(&pattern.to_lowercase()) {
                return Some(preset.clone());
            }
        }
    }
    
    // Then try model prefix match
    for preset in &presets {
        for pattern in &preset.match_model_prefix {
            if model.to_lowercase().starts_with(&pattern.to_lowercase()) {
                return Some(preset.clone());
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_token_plan_key() {
        assert!(is_token_plan_key("tp-abc123"));
        assert!(!is_token_plan_key("sk-abc123"));
        assert!(!is_token_plan_key(""));
    }

    #[test]
    fn test_resolve_mimo_base_url_explicit_cli() {
        let (url, is_tp) = resolve_mimo_base_url(
            Some("https://custom.example.com/v1"),
            None,
            "tp-abc",
        );
        assert_eq!(url, "https://custom.example.com/v1");
        assert!(!is_tp);
    }

    #[test]
    fn test_resolve_mimo_base_url_token_plan_key() {
        let (url, is_tp) = resolve_mimo_base_url(None, None, "tp-abc");
        assert_eq!(url, MIMO_BASE_URL_TOKEN_PLAN);
        assert!(is_tp);
    }

    #[test]
    fn test_resolve_mimo_base_url_standard_key() {
        let (url, is_tp) = resolve_mimo_base_url(None, None, "sk-abc");
        assert_eq!(url, MIMO_BASE_URL_DEFAULT);
        assert!(!is_tp);
    }

    #[test]
    fn test_resolve_mimo_base_url_env_beats_key() {
        let (url, is_tp) = resolve_mimo_base_url(
            None,
            Some("https://envset.example.com/v1"),
            "tp-abc",
        );
        assert_eq!(url, "https://envset.example.com/v1");
        assert!(!is_tp);
    }

    #[test]
    fn test_resolve_mimo_base_url_cli_beats_env() {
        let (url, is_tp) = resolve_mimo_base_url(
            Some("https://cli.example.com/v1"),
            Some("https://envset.example.com/v1"),
            "tp-abc",
        );
        assert_eq!(url, "https://cli.example.com/v1");
        assert!(!is_tp);
    }

    #[test]
    fn test_api_key_type_from_key() {
        assert_eq!(ApiKeyType::from_key("tp-abc"), ApiKeyType::TokenPlan);
        assert_eq!(ApiKeyType::from_key("sk-abc"), ApiKeyType::Standard);
        assert_eq!(ApiKeyType::from_key("other"), ApiKeyType::Unknown);
    }

    #[test]
    fn test_get_provider_presets_contains_expected() {
        let presets = get_provider_presets();
        let ids: Vec<&str> = presets.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"kimi"));
        assert!(ids.contains(&"minimax"));
        assert!(ids.contains(&"sensenova"));
    }

    #[test]
    fn test_match_preset_by_base_url_sensenova() {
        let preset = match_preset("https://TOKEN.SenseNova.cn/v1", "");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "sensenova");
    }

    #[test]
    fn test_match_preset_by_base_url_kimi() {
        let preset = match_preset("https://api.moonshot.cn/v1", "");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "kimi");
    }

    #[test]
    fn test_match_preset_by_base_url_minimax() {
        let preset = match_preset("https://api.minimaxi.com/v1", "");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "minimax");
    }

    #[test]
    fn test_match_preset_by_model_prefix_kimi() {
        let preset = match_preset("", "kimi-k2.6");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "kimi");
    }

    #[test]
    fn test_match_preset_by_model_prefix_moonshot() {
        let preset = match_preset("", "moonshot-v1-128k");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "kimi");
    }

    #[test]
    fn test_match_preset_by_model_prefix_sensenova() {
        let preset = match_preset("", "sensenova-6.7-flash-lite");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "sensenova");
    }

    #[test]
    fn test_match_preset_by_model_prefix_minimax() {
        let preset = match_preset("", "MiniMax-M2.7");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "minimax");
    }

    #[test]
    fn test_match_preset_by_model_prefix_abab() {
        let preset = match_preset("", "abab6.5");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "minimax");
    }

    #[test]
    fn test_match_preset_returns_none_for_unknown() {
        assert!(match_preset("https://api.example.com/v1", "qwen3-max").is_none());
        assert!(match_preset("", "").is_none());
    }

    #[test]
    fn test_base_url_match_wins_over_model_match() {
        // sensenova baseUrl should match even with minimax model prefix
        let preset = match_preset("https://token.sensenova.cn/v1", "MiniMax-M2.7");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().id, "sensenova");
    }

    #[test]
    fn test_kimi_preset_has_drop_reasoning_effort() {
        let preset = match_preset("https://api.moonshot.cn/v1", "").unwrap();
        assert!(preset.recommended_spec.features.drop_reasoning_effort);
        assert!(preset.recommended_spec.base_url.contains("moonshot"));
    }

    #[test]
    fn test_sensenova_preset_has_drop_response_format() {
        let preset = match_preset("", "sensenova-6.7").unwrap();
        assert!(preset.recommended_spec.features.drop_response_format);
        assert!(preset.recommended_spec.features.enhance_error_preset.is_some());
    }

    #[test]
    fn test_deepseek_uses_own_host() {
        // DeepSeek should not inherit MiMo URL
        let deepseek_url = DEEPSEEK_BASE_URL;
        assert!(deepseek_url.contains("deepseek"));
    }
}
