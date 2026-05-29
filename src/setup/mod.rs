//!
//! Setup Snippets Module
//! 
//! Rust implementation aligned with mimo2codex setup.snippets.test.ts
//! 
//! Features:
//! - Build CC Switch files (auth.json, config.toml)
//! - Snippet bundle generation
//! - Provider-specific configurations

use serde::{Deserialize, Serialize};

/// Server host configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostConfig {
    pub host: String,
    pub port: u16,
}

impl HostConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }
    
    pub fn url(&self) -> String {
        format!("http://{}:{}/v1", self.host, self.port)
    }
}

/// Provider target
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderTarget {
    Mimo,
    DeepSeek,
    MiniMax,
}

impl ProviderTarget {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mimo" | "m" => Some(ProviderTarget::Mimo),
            "ds" | "deepseek" => Some(ProviderTarget::DeepSeek),
            "minimax" | "mm" => Some(ProviderTarget::MiniMax),
            _ => None,
        }
    }
    
    pub fn model(&self) -> &str {
        match self {
            ProviderTarget::Mimo => "mimo-v2.5-pro",
            ProviderTarget::DeepSeek => "deepseek-v4-pro",
            ProviderTarget::MiniMax => "MiniMax-M2.7",
        }
    }
    
    pub fn requires_openai_auth(&self) -> bool {
        true
    }
}

/// Resolve snippet target from provider string
pub fn resolve_snippet_target(provider: &str) -> ProviderTarget {
    ProviderTarget::from_str(provider).unwrap_or(ProviderTarget::Mimo)
}

/// CC Switch files bundle
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcSwitchFiles {
    pub auth_json: String,
    pub config_toml: String,
}

/// Build CC Switch files for a provider target
pub fn build_cc_switch_files(host: &HostConfig, target: &ProviderTarget) -> CcSwitchFiles {
    // Build auth.json with sentinel
    let auth_json = format!(
        r#"{{"OPENAI_API_KEY": "mimo2codex-local"}}"#
    );
    
    // Build config.toml based on provider
    let (model_provider, model) = match target {
        ProviderTarget::Mimo => ("mimo2codex", "mimo-v2.5-pro"),
        ProviderTarget::DeepSeek => ("mimo2codex", "deepseek-v4-pro"),
        ProviderTarget::MiniMax => ("mimo2codex", "MiniMax-M2.7"),
    };
    
    let config_toml = format!(
        r#"[provider]
model_provider = "{}"
model = "{}"
base_url = "http://{}:{}/v1"
requires_openai_auth = true"#,
        model_provider,
        model,
        host.host,
        host.port
    );
    
    CcSwitchFiles {
        auth_json,
        config_toml,
    }
}

/// CC Switch snippet (markdown)
pub fn cc_switch_snippet(host: &HostConfig, target: &ProviderTarget) -> String {
    let files = build_cc_switch_files(host, target);
    
    format!(
        r#"## Codex CLI Configuration for {}

### auth.json
```
{}
```

### config.toml
```
{}
```
"#,
        match target {
            ProviderTarget::Mimo => "MiMo",
            ProviderTarget::DeepSeek => "DeepSeek",
            ProviderTarget::MiniMax => "MiniMax",
        },
        files.auth_json,
        files.config_toml
    )
}

/// Snippet bundle for API response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetBundle {
    pub provider: String,
    pub cc_switch_auth_json: String,
    pub cc_switch_config_toml: String,
    pub cc_switch_snippet: String,
}

/// Build complete snippet bundle for a provider
pub fn build_snippet_bundle(provider: &str, host: &HostConfig) -> SnippetBundle {
    let target = resolve_snippet_target(provider);
    let files = build_cc_switch_files(host, &target);
    let snippet = cc_switch_snippet(host, &target);
    
    SnippetBundle {
        provider: provider.to_string(),
        cc_switch_auth_json: files.auth_json,
        cc_switch_config_toml: files.config_toml,
        cc_switch_snippet: snippet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_config_url() {
        let host = HostConfig::new("127.0.0.1", 8788);
        assert_eq!(host.url(), "http://127.0.0.1:8788/v1");
    }

    #[test]
    fn test_provider_target_from_str() {
        assert_eq!(ProviderTarget::from_str("mimo"), Some(ProviderTarget::Mimo));
        assert_eq!(ProviderTarget::from_str("m"), Some(ProviderTarget::Mimo));
        assert_eq!(ProviderTarget::from_str("ds"), Some(ProviderTarget::DeepSeek));
        assert_eq!(ProviderTarget::from_str("deepseek"), Some(ProviderTarget::DeepSeek));
        assert_eq!(ProviderTarget::from_str("unknown"), None);
    }

    #[test]
    fn test_provider_target_model() {
        assert_eq!(ProviderTarget::Mimo.model(), "mimo-v2.5-pro");
        assert_eq!(ProviderTarget::DeepSeek.model(), "deepseek-v4-pro");
    }

    #[test]
    fn test_resolve_snippet_target() {
        assert_eq!(resolve_snippet_target("mimo"), ProviderTarget::Mimo);
        assert_eq!(resolve_snippet_target("ds"), ProviderTarget::DeepSeek);
        assert_eq!(resolve_snippet_target("unknown"), ProviderTarget::Mimo); // default
    }

    #[test]
    fn test_build_cc_switch_files_auth_json_sentinel() {
        let host = HostConfig::new("127.0.0.1", 8788);
        let files = build_cc_switch_files(&host, &ProviderTarget::Mimo);
        
        // Auth JSON should contain the sentinel
        assert!(files.auth_json.contains("mimo2codex-local"));
        
        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&files.auth_json).unwrap();
        assert_eq!(parsed["OPENAI_API_KEY"], "mimo2codex-local");
    }

    #[test]
    fn test_build_cc_switch_files_mimo_config() {
        let host = HostConfig::new("127.0.0.1", 8788);
        let files = build_cc_switch_files(&host, &ProviderTarget::Mimo);
        
        assert!(files.config_toml.contains("model = \"mimo-v2.5-pro\""));
        assert!(files.config_toml.contains(&host.url()));
        assert!(files.config_toml.contains("requires_openai_auth = true"));
    }

    #[test]
    fn test_build_cc_switch_files_deepseek_config() {
        let host = HostConfig::new("127.0.0.1", 8788);
        let files = build_cc_switch_files(&host, &ProviderTarget::DeepSeek);
        
        assert!(files.config_toml.contains("model_provider = \"mimo2codex\""));
        assert!(files.config_toml.contains("model = \"deepseek-v4-pro\""));
    }

    #[test]
    fn test_cc_switch_snippet_contains_files() {
        let host = HostConfig::new("127.0.0.1", 8788);
        let snippet = cc_switch_snippet(&host, &ProviderTarget::Mimo);
        
        assert!(snippet.contains("auth.json"));
        assert!(snippet.contains("config.toml"));
        assert!(snippet.contains("mimo2codex-local"));
        assert!(snippet.contains("mimo-v2.5-pro"));
    }

    #[test]
    fn test_build_snippet_bundle_fields() {
        let host = HostConfig::new("127.0.0.1", 8788);
        let bundle = build_snippet_bundle("mimo", &host);
        
        assert_eq!(bundle.provider, "mimo");
        assert!(!bundle.cc_switch_auth_json.is_empty());
        assert!(!bundle.cc_switch_config_toml.is_empty());
        assert!(!bundle.cc_switch_snippet.is_empty());
    }

    #[test]
    fn test_build_snippet_bundle_matches_cc_switch_files() {
        let host = HostConfig::new("127.0.0.1", 8788);
        let bundle = build_snippet_bundle("mimo", &host);
        let files = build_cc_switch_files(&host, &ProviderTarget::Mimo);
        
        assert_eq!(bundle.cc_switch_auth_json, files.auth_json);
        assert_eq!(bundle.cc_switch_config_toml, files.config_toml);
    }
}

pub mod update_method;

pub use update_method::{detect_update_method, package_root, UpdateInfo, UpdateMethod, UpdateStep};
