//! Generic Provider Specification and Loader
//! 
//! This module provides a generic provider implementation that can be configured
//! via JSON. It supports loading providers from a config file and registering
//! them dynamically at runtime.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::models::chat::{ChatRequest, Message, Tool, FunctionDefinition};
use crate::providers::extended_provider::{ExtendedLLMProvider, ProviderModel, ModelFeatures};
use crate::transform::compat::{CompatOptions, apply_compat};
use crate::transform::req_to_chat::ReqToChatOptions;

/// Wire API type for upstream communication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WireApi {
    /// Use Chat Completions API with translation
    Chat,
    /// Use native Responses API (passthrough)
    Responses,
}

impl Default for WireApi {
    fn default() -> Self {
        WireApi::Chat
    }
}

/// Generic provider features
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenericFeatures {
    /// Enable web search support
    #[serde(default)]
    pub web_search: Option<bool>,
    
    /// Force parallel tool calls
    #[serde(default)]
    pub force_parallel_tool_calls: Option<bool>,
    
    /// Error enhancement preset (sensenova, minimax, etc.)
    #[serde(default)]
    pub enhance_error_preset: Option<String>,
    
    /// Minimax compatibility features
    #[serde(rename = "minimaxCompat", default)]
    pub minimax_compat: Option<bool>,
    
    /// Drop null strict from function definitions
    #[serde(rename = "dropNullStrict", default)]
    pub drop_null_strict: Option<bool>,
    
    /// Drop null content from assistant messages
    #[serde(rename = "dropNullContent", default)]
    pub drop_null_content: Option<bool>,
    
    /// Drop tool_choice when set to "auto"
    #[serde(rename = "dropToolChoiceAuto", default)]
    pub drop_tool_choice_auto: Option<bool>,
    
    /// Drop stream_options field
    #[serde(rename = "dropStreamOptions", default)]
    pub drop_stream_options: Option<bool>,
    
    /// Drop parallel_tool_calls field
    #[serde(rename = "dropParallelToolCalls", default)]
    pub drop_parallel_tool_calls: Option<bool>,
    
    /// Merge multiple system messages into one
    #[serde(rename = "mergeSystemMessages", default)]
    pub merge_system_messages: Option<bool>,
    
    /// Drop response_format field
    #[serde(rename = "dropResponseFormat", default)]
    pub drop_response_format: Option<bool>,
    
    /// Drop non-function tools
    #[serde(rename = "dropNonFunctionTools", default)]
    pub drop_non_function_tools: Option<bool>,
    
    /// Extract inline think tags (<think>...)
    #[serde(rename = "extractThinkTags", default)]
    pub extract_think_tags: Option<bool>,
}

/// Generic provider model definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericProviderModel {
    /// Model ID
    pub id: String,
    
    /// Optional model aliases
    #[serde(default)]
    pub aliases: Option<Vec<String>>,
    
    /// Display name
    #[serde(default)]
    pub display_name: Option<String>,
    
    /// Whether this model supports images
    #[serde(rename = "supportsImages", default)]
    pub supports_images: Option<bool>,
    
    /// Whether this model supports reasoning
    #[serde(rename = "supportsReasoning", default)]
    pub supports_reasoning: Option<bool>,
    
    /// Whether this model supports web search
    #[serde(rename = "supportsWebSearch", default)]
    pub supports_web_search: Option<bool>,
    
    /// Context window size
    #[serde(rename = "contextWindow", default)]
    pub context_window: Option<u32>,
    
    /// Max output tokens
    #[serde(rename = "maxOutputTokens", default)]
    pub max_output_tokens: Option<u32>,
    
    /// Deprecated after date
    #[serde(rename = "deprecatedAfter", default)]
    pub deprecated_after: Option<String>,
}

impl From<&GenericProviderModel> for ProviderModel {
    fn from(spec: &GenericProviderModel) -> Self {
        ProviderModel {
            id: spec.id.clone(),
            name: spec.display_name.clone().unwrap_or(spec.id.clone()),
            aliases: spec.aliases.clone().unwrap_or_default(),
            context_window: spec.context_window.unwrap_or(128_000),
            max_output_tokens: spec.max_output_tokens.unwrap_or(8192),
            features: ModelFeatures {
                streaming: true,
                function_calling: true,
                vision: spec.supports_images.unwrap_or(false),
                web_search: spec.supports_web_search.unwrap_or(false),
                thinking: spec.supports_reasoning.unwrap_or(false),
                audio_output: false,
            },
        }
    }
}

/// Generic provider specification from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericProviderSpec {
    /// Provider ID (required, unique)
    pub id: String,
    
    /// Shortcut for quick access
    #[serde(default)]
    pub shortcut: Option<String>,
    
    /// Display name
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
    
    /// Base URL for API
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    
    /// Environment variable name for API key
    #[serde(rename = "envKey")]
    pub env_key: String,
    
    /// Default model
    #[serde(rename = "defaultModel", default)]
    pub default_model: Option<String>,
    
    /// Wire API type
    #[serde(rename = "wireApi", default)]
    pub wire_api: Option<WireApi>,
    
    /// Provider models
    #[serde(default)]
    pub models: Option<Vec<GenericProviderModel>>,
    
    /// Provider features
    #[serde(default)]
    pub features: Option<GenericFeatures>,
    
    /// Documentation URL
    #[serde(rename = "docsUrl", default)]
    pub docs_url: Option<String>,
    
    /// Force using default model (skip open-catalog matching)
    #[serde(rename = "forceDefaultModel", default)]
    pub force_default_model: Option<bool>,
}

impl GenericProviderSpec {
    /// Get the API key from environment
    pub fn get_api_key(&self) -> Option<String> {
        std::env::var(&self.env_key).ok()
    }
    
    /// Get the wire API type
    pub fn wire_api(&self) -> WireApi {
        self.wire_api.unwrap_or(WireApi::Chat)
    }
    
    /// Get the default model
    pub fn default_model(&self) -> String {
        self.default_model.clone().unwrap_or_else(|| "gpt-4".to_string())
    }
    
    /// Get provider models or default to open catalog
    pub fn models(&self) -> Vec<ProviderModel> {
        self.models
            .as_ref()
            .map(|m| m.iter().map(ProviderModel::from).collect())
            .unwrap_or_default()
    }
    
    /// Check if this is an open catalog (no models defined)
    pub fn is_open_catalog(&self) -> bool {
        self.models.is_none() || self.models.as_ref().map(|m| m.is_empty()).unwrap_or(true)
    }
    
    /// Resolve model by ID or alias
    pub fn resolve_model(&self, client_model: &str) -> Option<ProviderModel> {
        for model in self.models() {
            if model.id == client_model {
                return Some(model);
            }
            if model.aliases.contains(&client_model.to_string()) {
                return Some(model);
            }
        }
        None
    }
    
    /// Convert to CompatOptions
    pub fn to_compat_options(&self) -> CompatOptions {
        let features = self.features.as_ref();
        let mut opts = CompatOptions::default();
        
        if features.map(|f| f.minimax_compat.unwrap_or(false)).unwrap_or(false) {
            opts.minimax_compat = true;
        }
        
        if features.map(|f| f.drop_null_content.unwrap_or(false)).unwrap_or(false) {
            opts.drop_null_content = true;
        }
        
        if features.map(|f| f.drop_tool_choice_auto.unwrap_or(false)).unwrap_or(false) {
            opts.drop_tool_choice_auto = true;
        }
        
        if features.map(|f| f.merge_system_messages.unwrap_or(false)).unwrap_or(false) {
            opts.merge_system_messages = true;
        }
        
        opts
    }
}

/// Providers file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersFile {
    #[serde(default)]
    pub providers: Vec<GenericProviderSpec>,
}

/// Generic provider loader
pub struct GenericProviderLoader {
    /// Registered providers by ID
    providers: HashMap<String, GenericProviderSpec>,
}

impl GenericProviderLoader {
    /// Create a new loader
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }
    
    /// Load providers from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), GenericLoaderError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| GenericLoaderError::FileRead(e.to_string()))?;
        
        let file: ProvidersFile = serde_json::from_str(&content)
            .map_err(|e| GenericLoaderError::Parse(e.to_string()))?;
        
        for spec in file.providers {
            self.register(spec)?;
        }
        
        Ok(())
    }
    
    /// Load providers from environment variables
    pub fn load_from_env(&mut self) -> Result<(), GenericLoaderError> {
        // Support GENERIC_BASE_URL, GENERIC_API_KEY pattern
        if let Some(base_url) = std::env::var("GENERIC_BASE_URL").ok() {
            let api_key_env = std::env::var("GENERIC_API_KEY").ok();
            
            let spec = GenericProviderSpec {
                id: "generic".to_string(),
                shortcut: Some("generic".to_string()),
                display_name: Some("Generic Provider".to_string()),
                base_url,
                env_key: api_key_env.unwrap_or_else(|| "GENERIC_API_KEY".to_string()),
                default_model: std::env::var("GENERIC_DEFAULT_MODEL").ok(),
                wire_api: None,
                models: None,
                features: None,
                docs_url: None,
                force_default_model: None,
            };
            
            self.register(spec)?;
        }
        
        Ok(())
    }
    
    /// Register a provider
    pub fn register(&mut self, spec: GenericProviderSpec) -> Result<(), GenericLoaderError> {
        let id = &spec.id;
        
        // Check for reserved IDs
        if id == "mimo" || id == "deepseek" {
            return Err(GenericLoaderError::ReservedId(id.clone()));
        }
        
        // Check for duplicate IDs
        if self.providers.contains_key(id) {
            return Err(GenericLoaderError::DuplicateId(id.clone()));
        }
        
        // Validate ID format
        if !is_valid_provider_id(id) {
            return Err(GenericLoaderError::InvalidId(id.clone()));
        }
        
        self.providers.insert(id.clone(), spec);
        Ok(())
    }
    
    /// Get a provider by ID
    pub fn get(&self, id: &str) -> Option<&GenericProviderSpec> {
        self.providers.get(id)
    }
    
    /// Get all registered provider IDs
    pub fn ids(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
    
    /// Find provider by client model
    pub fn by_client_model(&self, model: &str) -> Option<&GenericProviderSpec> {
        for spec in self.providers.values() {
            // Skip open catalog providers for model matching
            if spec.is_open_catalog() {
                continue;
            }
            if spec.resolve_model(model).is_some() {
                return Some(spec);
            }
        }
        None
    }
    
    /// Clear all registered providers
    pub fn clear(&mut self) {
        self.providers.clear();
    }

    /// Iterate over all registered providers
    pub fn iter(&self) -> impl Iterator<Item = &GenericProviderSpec> {
        self.providers.values()
    }
}

impl Default for GenericProviderLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic provider loader errors
#[derive(Debug, thiserror::Error)]
pub enum GenericLoaderError {
    #[error("Failed to read providers file: {0}")]
    FileRead(String),
    
    #[error("Failed to parse providers file: {0}")]
    Parse(String),
    
    #[error("Provider ID '{0}' conflicts with built-in provider")]
    ReservedId(String),
    
    #[error("Duplicate provider ID: {0}")]
    DuplicateId(String),
    
    #[error("Invalid provider ID '{0}': must match [a-z0-9][a-z0-9_-]*")]
    InvalidId(String),
}

/// Check if provider ID is valid
fn is_valid_provider_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    
    let mut chars = id.chars();
    let first = chars.next().unwrap();
    
    // First character must be alphanumeric
    if !first.is_alphanumeric() {
        return false;
    }
    
    // Remaining characters must be alphanumeric, underscore, or hyphen
    for c in chars {
        if !c.is_alphanumeric() && c != '_' && c != '-' {
            return false;
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_provider_id() {
        assert!(is_valid_provider_id("mimo"));
        assert!(is_valid_provider_id("deepseek"));
        assert!(is_valid_provider_id("my-provider"));
        assert!(is_valid_provider_id("provider_123"));
        assert!(is_valid_provider_id("Provider2"));
    }

    #[test]
    fn test_invalid_provider_id() {
        assert!(!is_valid_provider_id(""));
        assert!(!is_valid_provider_id("my provider"));
        assert!(!is_valid_provider_id("my-provider!"));
        assert!(!is_valid_provider_id("-invalid"));
    }

    #[test]
    fn test_generic_provider_spec_defaults() {
        let spec = GenericProviderSpec {
            id: "test".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api.example.com".to_string(),
            env_key: "TEST_API_KEY".to_string(),
            default_model: None,
            wire_api: None,
            models: None,
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        assert_eq!(spec.wire_api(), WireApi::Chat);
        assert_eq!(spec.default_model(), "gpt-4");
        assert!(spec.is_open_catalog());
    }

    #[test]
    fn test_generic_provider_loader() {
        let mut loader = GenericProviderLoader::new();
        
        let spec = GenericProviderSpec {
            id: "test-provider".to_string(),
            shortcut: Some("test".to_string()),
            display_name: Some("Test Provider".to_string()),
            base_url: "https://api.test.com".to_string(),
            env_key: "TEST_API_KEY".to_string(),
            default_model: Some("gpt-4".to_string()),
            wire_api: Some(WireApi::Chat),
            models: Some(vec![GenericProviderModel {
                id: "gpt-4".to_string(),
                aliases: Some(vec!["gpt4".to_string()]),
                display_name: Some("GPT-4".to_string()),
                supports_images: Some(false),
                supports_reasoning: Some(true),
                supports_web_search: Some(false),
                context_window: Some(128_000),
                max_output_tokens: Some(8192),
                deprecated_after: None,
            }]),
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        loader.register(spec).unwrap();
        
        assert_eq!(loader.ids(), vec!["test-provider"]);
        
        let loaded = loader.get("test-provider").unwrap();
        assert_eq!(loaded.default_model(), "gpt-4");
        assert!(!loaded.is_open_catalog());
        
        let resolved = loaded.resolve_model("gpt-4");
        assert!(resolved.is_some());
        
        let resolved_alias = loaded.resolve_model("gpt4");
        assert!(resolved_alias.is_some());
    }

    #[test]
    fn test_loader_rejects_duplicate() {
        let mut loader = GenericProviderLoader::new();
        
        let spec1 = GenericProviderSpec {
            id: "test-dup".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api1.test.com".to_string(),
            env_key: "KEY1".to_string(),
            default_model: None,
            wire_api: None,
            models: None,
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        let spec2 = GenericProviderSpec {
            id: "test-dup".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api2.test.com".to_string(),
            env_key: "KEY2".to_string(),
            default_model: None,
            wire_api: None,
            models: None,
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        loader.register(spec1).unwrap();
        let result = loader.register(spec2);
        assert!(result.is_err());
    }

    #[test]
    fn test_loader_rejects_reserved_id() {
        let mut loader = GenericProviderLoader::new();
        
        let spec = GenericProviderSpec {
            id: "mimo".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api.test.com".to_string(),
            env_key: "KEY".to_string(),
            default_model: None,
            wire_api: None,
            models: None,
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        let result = loader.register(spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_model_alias_match() {
        use crate::providers::generic_provider::{GenericProviderSpec, GenericProviderModel};
        
        let spec = GenericProviderSpec {
            id: "q".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api.example.com/v1".to_string(),
            env_key: "Q_API_KEY".to_string(),
            default_model: Some("q3-max".to_string()),
            wire_api: None,
            models: Some(vec![
                GenericProviderModel {
                    id: "q3-max".to_string(),
                    aliases: None,
                    display_name: None,
                    supports_images: None,
                    supports_reasoning: None,
                    supports_web_search: None,
                    context_window: None,
                    max_output_tokens: None,
                    deprecated_after: None,
                },
                GenericProviderModel {
                    id: "q3-flash".to_string(),
                    aliases: Some(vec!["q-flash".to_string()]),
                    display_name: None,
                    supports_images: None,
                    supports_reasoning: None,
                    supports_web_search: None,
                    context_window: None,
                    max_output_tokens: None,
                    deprecated_after: None,
                },
            ]),
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        // Test exact match
        let resolved = spec.resolve_model("q3-max");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().id, "q3-max");
        
        // Test alias match
        let resolved = spec.resolve_model("q-flash");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().id, "q3-flash");
        
        // Test unknown model returns None
        let resolved = spec.resolve_model("unknown-model");
        assert!(resolved.is_none());
    }

    #[test]
    fn test_resolve_model_no_models_declared_returns_none() {
        use crate::providers::generic_provider::GenericProviderSpec;
        
        let spec = GenericProviderSpec {
            id: "g".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api.example.com/v1".to_string(),
            env_key: "G_API_KEY".to_string(),
            default_model: Some("x".to_string()),
            wire_api: None,
            models: None,
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        // When no models declared, resolve_model returns None (empty model list)
        let resolved = spec.resolve_model("anything");
        assert!(resolved.is_none());
        
        // But the provider is an open catalog
        assert!(spec.is_open_catalog());
    }

    #[test]
    fn test_provider_is_open_catalog() {
        use crate::providers::generic_provider::GenericProviderSpec;
        
        // With no models - should be open catalog
        let spec_no_models = GenericProviderSpec {
            id: "g".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api.example.com/v1".to_string(),
            env_key: "G_API_KEY".to_string(),
            default_model: Some("x".to_string()),
            wire_api: None,
            models: None,
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        assert!(spec_no_models.is_open_catalog());
        
        // With empty models - should be open catalog
        let spec_empty = GenericProviderSpec {
            id: "h".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api.example.com/v1".to_string(),
            env_key: "H_API_KEY".to_string(),
            default_model: Some("y".to_string()),
            wire_api: None,
            models: Some(vec![]),
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        assert!(spec_empty.is_open_catalog());
    }

    #[test]
    fn test_generic_provider_spec_env_key_derivation() {
        use crate::providers::generic_provider::GenericProviderSpec;
        
        let spec = GenericProviderSpec {
            id: "test".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api.example.com/v1".to_string(),
            env_key: "TEST_API_KEY".to_string(),
            default_model: Some("model-x".to_string()),
            wire_api: None,
            models: None,
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        // env_key should be stored as-is
        assert_eq!(spec.env_key, "TEST_API_KEY");
    }

    #[test]
    fn test_generic_provider_spec_with_shortcut() {
        use crate::providers::generic_provider::GenericProviderSpec;
        
        let spec = GenericProviderSpec {
            id: "custom-provider".to_string(),
            shortcut: Some("cp".to_string()),
            display_name: None,
            base_url: "https://api.example.com/v1".to_string(),
            env_key: "CP_API_KEY".to_string(),
            default_model: None,
            wire_api: None,
            models: None,
            features: None,
            docs_url: None,
            force_default_model: None,
        };
        
        assert_eq!(spec.shortcut, Some("cp".to_string()));
    }
}

    // Additional mimo2codex aligned tests
    
    #[test]
    fn test_provider_id_validation() {
        use crate::providers::generic_provider::GenericProviderSpec;
        
        // Valid provider ID
        let spec = GenericProviderSpec {
            id: "valid-provider".to_string(),
            shortcut: None,
            display_name: None,
            base_url: "https://api.example.com/v1".to_string(),
            env_key: "TEST_API_KEY".to_string(),
            default_model: Some("test-model".to_string()),
            wire_api: None,
            models: None,
            features: None,
            force_default_model: None,
            docs_url: None,
        };
        
        assert_eq!(spec.id, "valid-provider");
    }
    
    #[test]
    fn test_generic_features_default() {
        use crate::providers::generic_provider::GenericFeatures;
        
        let features = GenericFeatures::default();
        assert!(features.web_search.is_none());
        assert!(features.minimax_compat.is_none());
    }
    
    #[test]
    fn test_wire_api_default() {
        use crate::providers::generic_provider::WireApi;
        
        let api = WireApi::default();
        assert_eq!(api, WireApi::Chat);
    }
    
    #[test]
    fn test_provider_model_from_generic() {
        use crate::providers::generic_provider::GenericProviderModel;
        use crate::providers::extended_provider::ProviderModel;
        
        let generic = GenericProviderModel {
            id: "test-model".to_string(),
            aliases: Some(vec!["alias1".to_string(), "alias2".to_string()]),
            display_name: Some("Test Model".to_string()),
            supports_images: Some(true),
            supports_reasoning: Some(true),
            supports_web_search: Some(false),
            context_window: Some(128_000),
            max_output_tokens: Some(8192),
            deprecated_after: None,
        };
        
        let model: ProviderModel = (&generic).into();
        assert_eq!(model.id, "test-model");
        assert_eq!(model.aliases.len(), 2);
        assert!(model.features.vision);
        assert!(model.features.thinking);
        assert!(!model.features.web_search);
    }
