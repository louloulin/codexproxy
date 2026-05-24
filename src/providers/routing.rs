//!
//! Provider Routing Module
//! 
//! Rust implementation aligned with mimo2codex server.selectProvider.test.ts
//! 
//! Features:
//! - Select provider based on model ID and configuration
//! - Generic provider routing with priority
//! - Runtime override support
//! - forceDefaultModel fallback for minimax-compat

use std::collections::HashMap;

/// Provider specification from generic registry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericProviderSpec {
    pub id: String,
    pub display_name: String,
    pub base_url: String,
    pub env_key: String,
    pub default_model: String,
    pub models: Option<Vec<ModelSpec>>,
    pub force_default_model: bool,
}

/// Model specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    pub id: String,
    pub context_window: Option<u64>,
}

/// Provider runtime configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRuntime {
    pub base_url: String,
    pub api_key: String,
    pub flags: ProviderFlags,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderFlags {
    pub is_token_plan: bool,
}

/// Rewrite notice when model is rewritten
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteNotice {
    pub from: String,
    pub to: String,
    pub reason: Option<String>,
}

/// Selection result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionResult {
    pub provider_id: String,
    pub upstream_model: String,
    pub rewrite_notice: Option<RewriteNotice>,
}

/// Runtime override from client
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOverride {
    pub provider_id: String,
    pub model_id: String,
}

/// Global provider registry (generic providers)
static GENERIC_REGISTRY: std::sync::RwLock<Vec<GenericProviderSpec>> = std::sync::RwLock::new(Vec::new());

/// Initialize/reset the generic provider registry
pub fn init_registry(providers: Vec<GenericProviderSpec>) {
    let mut registry = GENERIC_REGISTRY.write().unwrap();
    *registry = providers;
}

/// Get a clone of the current registry
pub fn get_registry() -> Vec<GenericProviderSpec> {
    GENERIC_REGISTRY.read().unwrap().clone()
}

/// Check if a generic provider claims a model (has it in its models list or is forceDefaultModel)
fn generic_claims_model(spec: &GenericProviderSpec, model_id: &str) -> bool {
    if spec.force_default_model {
        return spec.default_model == model_id;
    }
    
    if let Some(models) = &spec.models {
        return models.iter().any(|m| m.id == model_id);
    }
    
    spec.models.is_none()
}

/// Check if model is in built-in catalog
fn is_builtin_model(model_id: &str) -> bool {
    matches!(model_id, "mimo-v2.5-pro" | "deepseek-v4-pro" | "glm-4-flash" | "glm-4")
}

/// Get the default model for a built-in provider
fn builtin_default_model(provider_id: &str) -> Option<String> {
    match provider_id {
        "mimo" => Some("mimo-v2.5-pro".to_string()),
        "deepseek" => Some("deepseek-v4-pro".to_string()),
        "zhipu" => Some("glm-4-flash".to_string()),
        _ => None,
    }
}

/// Select provider for a given model ID
pub fn select_provider(
    model_id: &str,
    default_provider_id: &str,
    providers: &HashMap<String, Option<ProviderRuntime>>,
    runtime_override: Option<RuntimeOverride>,
) -> SelectionResult {
    if let Some(override_) = runtime_override {
        return select_with_override(model_id, default_provider_id, providers, &override_);
    }
    
    let registry = get_registry();
    
    let matching_generics: Vec<_> = registry.iter()
        .filter(|spec| {
            if !generic_claims_model(spec, model_id) {
                return false;
            }
            providers.get(&spec.id)
                .and_then(|r| r.as_ref())
                .is_some()
        })
        .collect();
    
    if !matching_generics.is_empty() {
        let spec = &matching_generics[0];
        return SelectionResult {
            provider_id: spec.id.clone(),
            upstream_model: model_id.to_string(),
            rewrite_notice: None,
        };
    }
    
    if is_builtin_model(model_id) {
        let provider_id = match model_id {
            "mimo-v2.5-pro" => "mimo",
            "deepseek-v4-pro" => "deepseek",
            "glm-4-flash" | "glm-4" => "zhipu",
            _ => default_provider_id,
        };
        
        if providers.get(provider_id).and_then(|r| r.as_ref()).is_some() {
            return SelectionResult {
                provider_id: provider_id.to_string(),
                upstream_model: model_id.to_string(),
                rewrite_notice: None,
            };
        }
    }
    
    let default_model = builtin_default_model(default_provider_id)
        .or_else(|| {
            registry.iter()
                .find(|spec| spec.id == default_provider_id)
                .map(|spec| spec.default_model.clone())
        })
        .unwrap_or_else(|| model_id.to_string());
    
    SelectionResult {
        provider_id: default_provider_id.to_string(),
        upstream_model: default_model.clone(),
        rewrite_notice: Some(RewriteNotice {
            from: model_id.to_string(),
            to: default_model,
            reason: None,
        }),
    }
}

fn select_with_override(
    model_id: &str,
    default_provider_id: &str,
    providers: &HashMap<String, Option<ProviderRuntime>>,
    override_: &RuntimeOverride,
) -> SelectionResult {
    if let Some(runtime) = providers.get(&override_.provider_id).and_then(|r| r.as_ref()) {
        let is_known = is_builtin_model(&override_.model_id) || 
            get_registry().iter().any(|spec| {
                spec.id == override_.provider_id && 
                (spec.models.as_ref().map(|m| m.iter().any(|m| m.id == override_.model_id)).unwrap_or(false))
            });
        
        SelectionResult {
            provider_id: override_.provider_id.clone(),
            upstream_model: override_.model_id.clone(),
            rewrite_notice: if is_known { None } else {
                Some(RewriteNotice {
                    from: model_id.to_string(),
                    to: override_.model_id.clone(),
                    reason: Some("runtime override".to_string()),
                })
            },
        }
    } else {
        select_provider(model_id, default_provider_id, providers, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_runtime() -> ProviderRuntime {
        ProviderRuntime {
            base_url: "https://example.test/v1".to_string(),
            api_key: "sk-test".to_string(),
            flags: ProviderFlags::default(),
        }
    }

    fn make_providers() -> HashMap<String, Option<ProviderRuntime>> {
        let mut map = HashMap::new();
        map.insert("mimo".to_string(), Some(make_runtime()));
        map.insert("deepseek".to_string(), Some(make_runtime()));
        map
    }

    #[test]
    fn test_case_a_generic_with_model_and_key_wins() {
        init_registry(vec![
            GenericProviderSpec {
                id: "company-mimo".to_string(),
                display_name: "Company MiMo Proxy".to_string(),
                base_url: "https://internal.example/v1".to_string(),
                env_key: "COMPANY_MIMO_API_KEY".to_string(),
                default_model: "mimo-v2.5-pro".to_string(),
                models: Some(vec![ModelSpec { id: "mimo-v2.5-pro".to_string(), context_window: Some(128000) }]),
                force_default_model: false,
            }
        ]);
        
        let mut providers = make_providers();
        providers.insert("company-mimo".to_string(), Some(make_runtime()));
        
        let result = select_provider("mimo-v2.5-pro", "mimo", &providers, None);
        assert_eq!(result.provider_id, "company-mimo");
        assert_eq!(result.upstream_model, "mimo-v2.5-pro");
        assert!(result.rewrite_notice.is_none());
    }

    #[test]
    fn test_case_b_generic_claims_model_but_no_key_falls_through() {
        init_registry(vec![
            GenericProviderSpec {
                id: "company-mimo".to_string(),
                display_name: "Company MiMo Proxy".to_string(),
                base_url: "https://internal.example/v1".to_string(),
                env_key: "COMPANY_MIMO_API_KEY".to_string(),
                default_model: "mimo-v2.5-pro".to_string(),
                models: Some(vec![ModelSpec { id: "mimo-v2.5-pro".to_string(), context_window: Some(128000) }]),
                force_default_model: false,
            }
        ]);
        
        let providers = make_providers();
        
        let result = select_provider("mimo-v2.5-pro", "mimo", &providers, None);
        assert_eq!(result.provider_id, "mimo");
        assert_eq!(result.upstream_model, "mimo-v2.5-pro");
    }

    #[test]
    fn test_case_e_unknown_model_falls_back_to_default() {
        init_registry(vec![]);
        
        let providers = make_providers();
        
        let result = select_provider("gpt-99-turbo", "mimo", &providers, None);
        assert_eq!(result.provider_id, "mimo");
        assert_eq!(result.upstream_model, "mimo-v2.5-pro");
        assert!(result.rewrite_notice.is_some());
        assert_eq!(result.rewrite_notice.as_ref().unwrap().from, "gpt-99-turbo");
    }

    #[test]
    fn test_case_f_builtin_catalog_hit_no_rewrite() {
        init_registry(vec![]);
        
        let providers = make_providers();
        
        let result = select_provider("deepseek-v4-pro", "mimo", &providers, None);
        assert_eq!(result.provider_id, "deepseek");
        assert_eq!(result.upstream_model, "deepseek-v4-pro");
        assert!(result.rewrite_notice.is_none());
    }

    #[test]
    fn test_case_g_valid_override_wins() {
        init_registry(vec![]);
        
        let providers = make_providers();
        
        let result = select_provider("mimo-v2.5-pro", "mimo", &providers, Some(RuntimeOverride {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-v4-pro".to_string(),
        }));
        assert_eq!(result.provider_id, "deepseek");
        assert_eq!(result.upstream_model, "deepseek-v4-pro");
    }

    #[test]
    fn test_case_i_override_at_provider_with_no_runtime_ignored() {
        init_registry(vec![]);
        
        let mut providers = make_providers();
        providers.insert("deepseek".to_string(), None);
        
        let result = select_provider("mimo-v2.5-pro", "mimo", &providers, Some(RuntimeOverride {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-v4-pro".to_string(),
        }));
        assert_eq!(result.provider_id, "mimo");
    }

    #[test]
    fn test_case_j_override_with_unknown_model_still_routes() {
        init_registry(vec![]);
        
        let providers = make_providers();
        
        let result = select_provider("mimo-v2.5-pro", "mimo", &providers, Some(RuntimeOverride {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-experimental-99".to_string(),
        }));
        assert_eq!(result.provider_id, "deepseek");
        assert_eq!(result.upstream_model, "deepseek-experimental-99");
        assert!(result.rewrite_notice.is_some());
    }

    #[test]
    fn test_case_k_override_null_preserves_behavior() {
        init_registry(vec![
            GenericProviderSpec {
                id: "company-mimo".to_string(),
                display_name: "Company MiMo Proxy".to_string(),
                base_url: "https://internal.example/v1".to_string(),
                env_key: "COMPANY_MIMO_API_KEY".to_string(),
                default_model: "mimo-v2.5-pro".to_string(),
                models: Some(vec![ModelSpec { id: "mimo-v2.5-pro".to_string(), context_window: Some(128000) }]),
                force_default_model: false,
            }
        ]);
        
        let mut providers = make_providers();
        providers.insert("company-mimo".to_string(), Some(make_runtime()));
        
        let result = select_provider("mimo-v2.5-pro", "mimo", &providers, None);
        assert_eq!(result.provider_id, "company-mimo");
    }

    #[test]
    fn test_case_l_force_default_model_rewrites_unknown_ids() {
        init_registry(vec![
            GenericProviderSpec {
                id: "minimax".to_string(),
                display_name: "MiniMax M2.7".to_string(),
                base_url: "https://api.minimaxi.com/v1".to_string(),
                env_key: "MINIMAX_API_KEY".to_string(),
                default_model: "MiniMax-M2.7".to_string(),
                models: None,
                force_default_model: true,
            }
        ]);
        
        let mut providers = make_providers();
        providers.insert("mimo".to_string(), None);
        providers.insert("deepseek".to_string(), None);
        providers.insert("minimax".to_string(), Some(make_runtime()));
        
        let result = select_provider("gpt-5.5", "minimax", &providers, None);
        assert_eq!(result.provider_id, "minimax");
        assert_eq!(result.upstream_model, "MiniMax-M2.7");
        assert!(result.rewrite_notice.is_some());
    }

    #[test]
    fn test_case_m_open_catalog_passthrough() {
        init_registry(vec![
            GenericProviderSpec {
                id: "ollama".to_string(),
                display_name: "Ollama".to_string(),
                base_url: "http://127.0.0.1:11434/v1".to_string(),
                env_key: "OLLAMA_API_KEY".to_string(),
                default_model: "qwen2.5-coder:7b".to_string(),
                models: None,
                force_default_model: false,
            }
        ]);
        
        let mut providers = make_providers();
        providers.insert("mimo".to_string(), None);
        providers.insert("deepseek".to_string(), None);
        providers.insert("ollama".to_string(), Some(make_runtime()));
        
        let result = select_provider("custom:tag", "ollama", &providers, None);
        assert_eq!(result.provider_id, "ollama");
        assert_eq!(result.upstream_model, "custom:tag");
        assert!(result.rewrite_notice.is_none());
    }
}
