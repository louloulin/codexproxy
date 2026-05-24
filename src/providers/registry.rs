//! Provider Registry - Dynamic provider routing
//! 
//! This module provides a registry for dynamically routing requests
//! to the appropriate LLM provider based on model names.

use std::collections::HashMap;
use std::sync::Arc;
use crate::providers::extended_provider::{ExtendedLLMProvider, ProviderModel};

/// A registered LLM provider with its supported models
#[derive(Clone)]
pub struct RegisteredProvider {
    /// The provider instance
    pub provider: Arc<dyn ExtendedLLMProvider>,
    /// Models supported by this provider
    pub models: Vec<ProviderModel>,
}

impl RegisteredProvider {
    /// Create a new registered provider
    pub fn new(provider: Arc<dyn ExtendedLLMProvider>) -> Self {
        let models = provider.supported_models();
        Self { provider, models }
    }
}

/// Provider Registry for dynamic routing
/// 
/// This registry allows registering multiple providers and routing
/// requests to the appropriate provider based on model names.
#[derive(Default)]
pub struct ProviderRegistry {
    /// Registered providers by name
    providers: HashMap<String, RegisteredProvider>,
    /// Model to provider mapping for fast lookup
    model_to_provider: HashMap<String, String>,
    /// Default provider name
    default_provider: Option<String>,
}

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider
    pub fn register(&mut self, provider: Arc<dyn ExtendedLLMProvider>) -> &mut Self {
        let name = provider.name().to_string();
        let registered = RegisteredProvider::new(provider);
        
        // Add model mappings
        for model in &registered.models {
            self.model_to_provider.insert(model.id.clone(), name.clone());
            // Also map aliases
            for alias in &model.aliases {
                self.model_to_provider.insert(alias.clone(), name.clone());
            }
        }
        
        self.providers.insert(name.clone(), registered);
        
        // Set as default if no default is set
        if self.default_provider.is_none() {
            self.default_provider = Some(name);
        }
        
        self
    }

    /// Set the default provider
    pub fn set_default(&mut self, name: &str) -> &mut Self {
        if self.providers.contains_key(name) {
            self.default_provider = Some(name.to_string());
        }
        self
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<&dyn ExtendedLLMProvider> {
        self.providers.get(name).map(|p| p.provider.as_ref())
    }

    /// Get a provider by model name
    /// 
    /// Searches in order:
    /// 1. Exact model match
    /// 2. Model prefix match (e.g., "gpt-4" matches "gpt-4-0613")
    /// 3. Default provider
    pub fn select(&self, model: &str) -> Option<&dyn ExtendedLLMProvider> {
        // Try exact match first
        if let Some(provider_name) = self.model_to_provider.get(model) {
            if let Some(provider) = self.providers.get(provider_name) {
                return Some(provider.provider.as_ref());
            }
        }
        
        // Try prefix match
        for (provider_name, _) in &self.model_to_provider {
            if model.starts_with(provider_name) || provider_name.starts_with(model) {
                if let Some(provider) = self.providers.get(provider_name) {
                    return Some(provider.provider.as_ref());
                }
            }
        }
        
        // Fall back to default
        self.default_provider
            .as_ref()
            .and_then(|name| self.providers.get(name))
            .map(|p| p.provider.as_ref())
    }

    /// Resolve a model and return the provider with model info
    pub fn resolve(&self, model: &str) -> Option<(&dyn ExtendedLLMProvider, &ProviderModel)> {
        // Try exact match
        if let Some(provider_name) = self.model_to_provider.get(model) {
            if let Some(provider) = self.providers.get(provider_name) {
                if let Some(model_info) = provider.models.iter().find(|m| &m.id == model) {
                    return Some((provider.provider.as_ref(), model_info));
                }
            }
        }
        
        // Try prefix match
        for (provider_name, _) in &self.model_to_provider {
            if model.starts_with(provider_name) || provider_name.starts_with(model) {
                if let Some(provider) = self.providers.get(provider_name) {
                    // Find first model from this provider as fallback
                    if let Some(model_info) = provider.models.first() {
                        return Some((provider.provider.as_ref(), model_info));
                    }
                }
            }
        }
        
        // Fall back to default
        self.default_provider
            .as_ref()
            .and_then(|name| self.providers.get(name))
            .map(|p| (p.provider.as_ref(), p.models.first().unwrap()))
    }

    /// List all registered provider names
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    /// List all known models
    pub fn all_models(&self) -> Vec<&ProviderModel> {
        self.providers.values()
            .flat_map(|p| p.models.iter())
            .collect()
    }

    /// Check if a model is supported
    pub fn supports_model(&self, model: &str) -> bool {
        self.model_to_provider.contains_key(model) || 
            self.model_to_provider.keys().any(|m| model.starts_with(m))
    }

    /// Get the count of registered providers
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test would require implementing ExtendedLLMProvider
    // This is a structural test placeholder
    
    #[test]
    fn test_registry_empty() {
        let registry = ProviderRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }
}

    #[test]
    fn test_registry_len() {
        let registry = ProviderRegistry::new();
        assert_eq!(registry.len(), 0);
    }
    
    #[test]
    fn test_registry_empty() {
        let registry = ProviderRegistry::new();
        assert!(registry.is_empty());
    }
    
    #[test]
    fn test_registry_provider_names() {
        let registry = ProviderRegistry::new();
        assert!(registry.provider_names().is_empty());
    }
    
    #[test]
    fn test_registry_all_models() {
        let registry = ProviderRegistry::new();
        assert!(registry.all_models().is_empty());
    }
