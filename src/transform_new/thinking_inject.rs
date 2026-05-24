//! Thinking Mode Injection
//!
//! This module provides utilities for automatically injecting thinking mode
//! configuration based on the model being used.
//!
//! Some models support thinking/reasoning mode, while others (like mimo-v2-flash)
//! should have thinking mode disabled.

use crate::models::chat_extended::ThinkingConfig;

/// Models that support thinking/reasoning mode
const THINKING_ENABLED_MODELS: &[&str] = &[
    "mimo-v2",
    "mimo-v2.5",
    "mimo-pro",
    "deepseek-chat",
    "deepseek-coder",
    "o1-mini",
    "o1-preview",
    "o1",
    "o3-mini",
    "o3",
    "claude-3-5-sonnet",
    "claude-3-5-sonnet-20241022",
    "claude-3-opus",
    "gemini-2.0-flash-thinking",
    "gemini-2.5-pro",
];

/// Models that should have thinking mode disabled
const THINKING_DISABLED_MODELS: &[&str] = &[
    "mimo-v2-flash",
    "mimo-mini",
    "gpt-4o-mini",
    "gpt-4o-mini-2024-07-18",
    "gpt-3.5-turbo",
    "gpt-3.5-turbo-0301",
    "gpt-3.5-turbo-0613",
    "gpt-3.5-turbo-16k",
    "gpt-4-turbo",
    "gpt-4-turbo-2024-04-09",
];

/// Check if a model should have thinking mode enabled
pub fn should_enable_thinking(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    
    // Check explicit disables first
    for disabled in THINKING_DISABLED_MODELS {
        if model_lower.contains(&disabled.to_lowercase()) {
            return false;
        }
    }
    
    // Check explicit enables
    for enabled in THINKING_ENABLED_MODELS {
        if model_lower.contains(&enabled.to_lowercase()) {
            return true;
        }
    }
    
    // Default: check for reasoning-related keywords
    if model_lower.contains("reasoning") 
        || model_lower.contains("think") 
        || model_lower.contains("o1") 
        || model_lower.contains("o3") 
        || model_lower.contains("deepseek")
    {
        return true;
    }
    
    // Default to disabled for unknown models
    false
}

/// Get the appropriate thinking config for a model
pub fn get_thinking_config(model: &str) -> Option<ThinkingConfig> {
    if should_enable_thinking(model) {
        Some(ThinkingConfig::enabled())
    } else {
        // Some models don't support thinking at all, return None for those
        let model_lower = model.to_lowercase();
        if model_lower.contains("mimo-mini") || model_lower.contains("mimo-v2-flash") {
            Some(ThinkingConfig::disabled())
        } else {
            None // Don't include thinking field for models that don't support it
        }
    }
}

/// Check if a model supports thinking mode at all
pub fn supports_thinking(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    
    // Check disabled models first
    for disabled in THINKING_DISABLED_MODELS {
        if model_lower.contains(&disabled.to_lowercase()) {
            return false;
        }
    }
    
    true
}

/// Get reasoning effort for models that support it
pub fn get_reasoning_effort(model: &str, requested: Option<&str>) -> Option<String> {
    if !should_enable_thinking(model) {
        return None;
    }
    
    // Use requested effort if provided, otherwise use default
    requested.map(|s| s.to_string())
        .or_else(|| Some("high".to_string()))
}

/// In thinking mode, temperature should often be removed or set to a specific value
pub fn should_remove_temperature(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    
    // OpenAI o-series models don't support temperature in reasoning mode
    model_lower.starts_with("o1")
        || model_lower.starts_with("o3")
        || model_lower.contains("deepseek-r1")
}

/// Get recommended temperature for thinking mode
pub fn get_thinking_temperature(model: &str) -> Option<f32> {
    if should_remove_temperature(model) {
        // Models that don't support temperature should return None to omit it
        None
    } else {
        // Default temperature for thinking models that support it
        Some(0.7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mimo_v2_flash_disabled() {
        assert!(!should_enable_thinking("mimo-v2-flash"));
        assert!(!supports_thinking("mimo-v2-flash"));
    }

    #[test]
    fn test_mimo_v2_enabled() {
        assert!(should_enable_thinking("mimo-v2"));
        assert!(supports_thinking("mimo-v2"));
    }

    #[test]
    fn test_deepseek_enabled() {
        assert!(should_enable_thinking("deepseek-chat"));
        assert!(should_enable_thinking("deepseek-coder"));
    }

    #[test]
    fn test_o_series_enabled() {
        assert!(should_enable_thinking("o1-mini"));
        assert!(should_enable_thinking("o1-preview"));
        assert!(should_enable_thinking("o3"));
    }

    #[test]
    fn test_o_series_temperature_removed() {
        assert!(should_remove_temperature("o1-mini"));
        assert!(should_remove_temperature("o3"));
        assert!(!should_remove_temperature("gpt-4"));
    }

    #[test]
    fn test_get_thinking_config() {
        let config = get_thinking_config("mimo-v2");
        assert!(config.is_some());
        assert_eq!(config.unwrap().thinking_type, crate::models::chat_extended::ThinkingType::Enabled);
        
        let config_disabled = get_thinking_config("mimo-v2-flash");
        assert!(config_disabled.is_some());
        assert_eq!(config_disabled.unwrap().thinking_type, crate::models::chat_extended::ThinkingType::Disabled);
    }

    #[test]
    fn test_unknown_model_disabled() {
        assert!(!should_enable_thinking("unknown-model"));
    }
}
