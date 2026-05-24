//! Admin UI request handlers

use axum::{
    extract::State,
    response::Html,
    routing::get,
    Router,
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

/// Admin dashboard HTML
const ADMIN_DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>rcodex Admin</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; color: #333; }
        .container { max-width: 1200px; margin: 0 auto; padding: 20px; }
        header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; margin-bottom: 20px; border-radius: 8px; }
        header h1 { font-size: 2em; margin-bottom: 5px; }
        .card { background: white; border-radius: 8px; padding: 20px; margin-bottom: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .card h2 { margin-bottom: 15px; color: #1a1a2e; border-bottom: 2px solid #eee; padding-bottom: 10px; }
        .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin-bottom: 20px; }
        .stat-card { background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%); color: white; padding: 20px; border-radius: 8px; }
        .stat-card h3 { font-size: 0.9em; opacity: 0.9; }
        .stat-card .value { font-size: 2em; font-weight: bold; margin-top: 5px; }
        .status { display: inline-block; padding: 4px 8px; border-radius: 4px; font-size: 0.85em; }
        .status.healthy { background: #d4edda; color: #155724; }
        button { background: #667eea; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; }
        button:hover { background: #5a6fd6; }
        #error { color: #dc3545; padding: 10px; background: #f8d7da; border-radius: 4px; display: none; margin-top: 10px; }
        table { width: 100%; border-collapse: collapse; }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #eee; }
        th { background: #f8f9fa; font-weight: 600; }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>rcodex Admin Dashboard</h1>
            <p>OpenAI Proxy Management Interface</p>
        </header>
        
        <div class="stats">
            <div class="stat-card">
                <h3>Providers</h3>
                <div class="value" id="provider-count">-</div>
            </div>
            <div class="stat-card">
                <h3>Uptime</h3>
                <div class="value" id="uptime">-</div>
            </div>
            <div class="stat-card" style="background: linear-gradient(135deg, #fc4a1a 0%, #f7b733 100%);">
                <h3>Status</h3>
                <div class="value" id="status-text">-</div>
            </div>
        </div>
        
        <div class="card">
            <h2>System Status</h2>
            <button onclick="loadStatus()">Refresh Status</button>
            <div id="error"></div>
        </div>
        
        <div class="card">
            <h2>Available Providers</h2>
            <div id="providers"></div>
        </div>
    </div>
    
    <script>
        async function loadStatus() {
            const errorEl = document.getElementById('error');
            errorEl.style.display = 'none';
            
            try {
                const response = await fetch('/admin/api/status');
                if (!response.ok) throw new Error('Failed to load status');
                
                const data = await response.json();
                
                document.getElementById('provider-count').textContent = data.providers.length;
                document.getElementById('uptime').textContent = formatUptime(data.uptime_seconds);
                document.getElementById('status-text').textContent = data.status;
                
                const providersEl = document.getElementById('providers');
                if (data.providers.length === 0) {
                    providersEl.innerHTML = '<p>No providers configured</p>';
                } else {
                    providersEl.innerHTML = `
                        <table>
                            <thead>
                                <tr>
                                    <th>ID</th>
                                    <th>Name</th>
                                    <th>Status</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${data.providers.map(p => `
                                    <tr>
                                        <td>${p.id}</td>
                                        <td>${p.name}</td>
                                        <td><span class="status healthy">${p.enabled ? 'Active' : 'Disabled'}</span></td>
                                    </tr>
                                `).join('')}
                            </tbody>
                        </table>
                    `;
                }
            } catch (err) {
                errorEl.textContent = 'Error loading status: ' + err.message;
                errorEl.style.display = 'block';
            }
        }
        
        function formatUptime(seconds) {
            const days = Math.floor(seconds / 86400);
            const hours = Math.floor((seconds % 86400) / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            if (days > 0) return `${days}d ${hours}h`;
            if (hours > 0) return `${hours}h ${mins}m`;
            return `${mins}m`;
        }
        
        loadStatus();
        setInterval(loadStatus, 30000);
    </script>
</body>
</html>
"#;

/// GET /admin - Admin dashboard
pub async fn admin_dashboard() -> Html<String> {
    Html(ADMIN_DASHBOARD_HTML.to_string())
}

/// GET /admin/api/status - JSON status endpoint
pub async fn api_status(
    State((state, admin_state)): State<(Arc<AppState>, Arc<AdminState>)>,
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
        uptime_seconds: admin_state.uptime(),
    })
}

/// GET /admin/api/providers - List all providers
pub async fn api_providers(
    State((state, _admin_state)): State<(Arc<AppState>, Arc<AdminState>)>,
) -> axum::Json<HashMap<String, serde_json::Value>> {
    let mut providers = HashMap::new();
    
    if state.openai_provider.is_some() {
        providers.insert(
            "openai".to_string(),
            serde_json::json!({
                "id": "openai",
                "name": "OpenAI",
                "enabled": true,
            }),
        );
    }
    
    if state.zhipu_provider.is_some() {
        providers.insert(
            "zhipu".to_string(),
            serde_json::json!({
                "id": "zhipu", 
                "name": "Zhipu",
                "enabled": true,
            }),
        );
    }
    
    axum::Json(providers)
}

/// GET /admin/api/stats - Admin statistics
pub async fn api_stats(
    State((state, admin_state)): State<(Arc<AppState>, Arc<AdminState>)>,
) -> axum::Json<AdminStats> {
    let mut count = 0;
    if state.openai_provider.is_some() { count += 1; }
    if state.zhipu_provider.is_some() { count += 1; }
    
    axum::Json(AdminStats {
        total_providers: count,
        uptime_seconds: admin_state.uptime(),
    })
}
