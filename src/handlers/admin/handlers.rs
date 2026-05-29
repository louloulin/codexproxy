//! Admin UI request handlers

use axum::{
    extract::State,
    response::Redirect,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::handlers::AppState;

/// Admin-specific state for tracking start time
pub struct AdminState {
    pub start_time: std::time::Instant,
}

impl AdminState {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
        }
    }
    
    pub fn uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for AdminState {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider status information
#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

/// Server health information
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub providers: Vec<ProviderStatus>,
    pub uptime_seconds: u64,
}

/// Admin statistics
#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub total_providers: usize,
    pub uptime_seconds: u64,
}

/// GET /admin - Redirect to SPA
pub async fn admin_dashboard() -> Redirect {
    Redirect::temporary("/admin/spa/")
}

/// GET /admin/api/status - JSON status endpoint
pub async fn api_status(
    State(state): State<Arc<AppState>>,
) -> axum::Json<HealthStatus> {
    let mut providers = Vec::new();
    
    if state.openai_provider.is_some() {
        providers.push(ProviderStatus {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            enabled: true,
        });
    }
    
    if state.zhipu_provider.is_some() {
        providers.push(ProviderStatus {
            id: "zhipu".to_string(),
            name: "Zhipu".to_string(),
            enabled: true,
        });
    }
    
    axum::Json(HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        providers,
        uptime_seconds: 0u64,
    })
}

/// GET /admin/api/providers - List all providers (dynamic + configured)
pub async fn api_providers(
    State(state): State<Arc<AppState>>,
) -> axum::Json<HashMap<String, serde_json::Value>> {
    let mut providers = HashMap::new();

    // Add configured providers
    if state.openai_provider.is_some() {
        providers.insert("openai".to_string(), serde_json::json!({"id":"openai","name":"OpenAI","enabled":true,"source":"config"}));
    }
    if state.zhipu_provider.is_some() {
        providers.insert("zhipu".to_string(), serde_json::json!({"id":"zhipu","name":"Zhipu AI","enabled":true,"source":"config"}));
    }
    if state.minimax_provider.is_some() {
        providers.insert("minimax".to_string(), serde_json::json!({"id":"minimax","name":"MiniMax","enabled":true,"source":"config"}));
    }

    // Add dynamic registry providers
    if let Some(ref registry) = state.provider_registry {
        if let Ok(reg) = registry.lock() {
            for name in reg.provider_names() {
                providers.entry(name.to_string()).or_insert_with(|| {
                    serde_json::json!({"id":name,"name":name,"enabled":true,"source":"registry"})
                });
            }
        }
    }

    axum::Json(providers)
}

/// GET /admin/api/stats - Admin statistics (with dynamic provider count)
pub async fn api_stats(
    State(state): State<Arc<AppState>>,
) -> axum::Json<AdminStats> {
    let mut count = 0;
    if state.openai_provider.is_some() { count += 1; }
    if state.zhipu_provider.is_some() { count += 1; }
    if state.minimax_provider.is_some() { count += 1; }
    if let Some(ref registry) = state.provider_registry {
        if let Ok(reg) = registry.lock() {
            count += reg.provider_names().len();
        }
    }

    // Return a unique marker to identify this handler
    tracing::info!("api_stats handler called");
    axum::Json(AdminStats {
        total_providers: count,
        uptime_seconds: 42u64,  // Unique marker: uptime should be 42!
    })
}

/// TEST route - unique marker endpoint
pub async fn test_stats_marker() -> &'static str {
    "TEST_MARKER_UNIQUE_12345"
}
