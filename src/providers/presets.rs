//!
//! Provider Presets Module
//! 
//! Rust implementation aligned with mimo2codex providers/presets.js
//! 
//! Features:
//! - Provider presets for known Chinese AI providers
//! - Preset matching by base URL or model prefix
//! - Provider-specific error enhancement

use serde::{Deserialize, Serialize};

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

/// Get all provider presets
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
                "deepseek-v4-flash".to_string(),
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
/// Base URL match takes priority over model prefix match
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

/// Enhanced error information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnhancedError {
    pub code: String,
    pub message: String,
}

/// Apply provider-specific error enhancement rules
pub fn apply_enhance_error_preset(
    provider: &str,
    status: u16,
    body: Option<&str>,
) -> Option<EnhancedError> {
    let body = body?;
    if body.is_empty() {
        return None;
    }
    
    if provider == "sensenova" && status == 400 {
        // sensenova specific error patterns
        if body.contains("Errors in message queue response") {
            return Some(EnhancedError {
                code: "sensenova_request_validation_failed".to_string(),
                message: format!(
                    "Request validation failed (response_format may be required): {}",
                    body
                ),
            });
        }
        
        if body.contains("invalid temperature") && body.contains("[0,2]") {
            return Some(EnhancedError {
                code: "sensenova_temperature_out_of_range".to_string(),
                message: format!(
                    "Temperature out of range. sensenova requires: {}",
                    body
                ),
            });
        }
        
        if body.contains("max_tokens exceeds upper bound 65536") {
            return Some(EnhancedError {
                code: "sensenova_max_tokens_out_of_range".to_string(),
                message: format!(
                    "max_tokens exceeds limit: {}",
                    body
                ),
            });
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_presets_contains_expected_ids() {
        let presets = get_provider_presets();
        let mut ids: Vec<&str> = presets.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        assert_eq!(ids, vec!["kimi", "minimax", "sensenova"]);
    }

    #[test]
    fn test_kimi_preset_has_drop_reasoning_effort() {
        let preset = match_preset("https://api.moonshot.cn/v1", "").unwrap();
        assert!(preset.recommended_spec.features.drop_reasoning_effort);
        assert!(preset.recommended_spec.base_url.contains("moonshot"));
    }

    #[test]
    fn test_sensenova_preset_has_drop_response_format() {
        let presets = get_provider_presets();
        let sn = presets.iter().find(|p| p.id == "sensenova").unwrap();
        assert!(sn.recommended_spec.features.drop_response_format);
        assert_eq!(
            sn.recommended_spec.features.enhance_error_preset,
            Some("sensenova".to_string())
        );
        assert_eq!(sn.recommended_spec.base_url, "https://token.sensenova.cn/v1");
    }

    #[test]
    fn test_match_sensenova_by_base_url_case_insensitive() {
        let preset = match_preset("https://TOKEN.SenseNova.cn/v1", "").unwrap();
        assert_eq!(preset.id, "sensenova");
    }

    #[test]
    fn test_match_kimi_by_moonshot_url() {
        assert!(match_preset("https://api.moonshot.cn/v1", "").is_some());
        assert!(match_preset("https://api.moonshot.ai/v1", "").is_some());
    }

    #[test]
    fn test_match_kimi_by_model_prefix() {
        assert_eq!(match_preset("", "kimi-k2.6").unwrap().id, "kimi");
        assert_eq!(match_preset("", "moonshot-v1-128k").unwrap().id, "kimi");
    }

    #[test]
    fn test_match_sensenova_by_model_prefix() {
        assert_eq!(match_preset("", "sensenova-6.7-flash-lite").unwrap().id, "sensenova");
        assert_eq!(match_preset("", "deepseek-v4-flash").unwrap().id, "sensenova");
    }

    #[test]
    fn test_match_minimax_by_base_url() {
        assert_eq!(
            match_preset("https://api.minimaxi.com/v1", "").unwrap().id,
            "minimax"
        );
    }

    #[test]
    fn test_match_minimax_by_model_prefix() {
        assert_eq!(match_preset("", "MiniMax-M2.7").unwrap().id, "minimax");
        assert_eq!(match_preset("", "abab6.5").unwrap().id, "minimax");
    }

    #[test]
    fn test_match_returns_none_for_unknown() {
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
    fn test_enhance_error_sensenova_message_queue() {
        let body = r#"{"error":{"message":"Errors in message queue response","type":"invalid_request_error","code":"3"}}"#;
        let result = apply_enhance_error_preset("sensenova", 400, Some(body));
        assert!(result.is_some());
        let err = result.unwrap();
        assert_eq!(err.code, "sensenova_request_validation_failed");
        assert!(err.message.contains("response_format"));
        assert!(err.message.contains("Errors in message queue response"));
    }

    #[test]
    fn test_enhance_error_sensenova_temperature() {
        let body = "invalid temperature, should in [0,2].";
        let result = apply_enhance_error_preset("sensenova", 400, Some(body));
        assert!(result.is_some());
        let err = result.unwrap();
        assert_eq!(err.code, "sensenova_temperature_out_of_range");
        assert!(err.message.contains("[0,2]"));
    }

    #[test]
    fn test_enhance_error_sensenova_max_tokens() {
        let body = "max_tokens exceeds upper bound 65536";
        let result = apply_enhance_error_preset("sensenova", 400, Some(body));
        assert!(result.is_some());
        let err = result.unwrap();
        assert_eq!(err.code, "sensenova_max_tokens_out_of_range");
        assert!(err.message.contains("65536"));
    }

    #[test]
    fn test_enhance_error_sensenova_no_false_positives_401() {
        assert!(apply_enhance_error_preset("sensenova", 401, Some("unauthorized")).is_none());
    }

    #[test]
    fn test_enhance_error_sensenova_no_false_positives_500() {
        assert!(apply_enhance_error_preset("sensenova", 500, Some("internal")).is_none());
    }

    #[test]
    fn test_enhance_error_sensenova_no_false_positives_random_body() {
        assert!(apply_enhance_error_preset("sensenova", 400, Some("some random body")).is_none());
    }

    #[test]
    fn test_enhance_error_sensenova_empty_body() {
        assert!(apply_enhance_error_preset("sensenova", 400, Some("")).is_none());
        assert!(apply_enhance_error_preset("sensenova", 400, None).is_none());
    }

    #[test]
    fn test_enhance_error_minimax_no_rules() {
        assert!(apply_enhance_error_preset("minimax", 400, Some("invalid chat setting")).is_none());
    }
}
