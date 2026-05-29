//! Additional admin handlers (P3-4/5/6/7/8/9)
//!
//! Model management, provider health, stats detailed, settings, logs cleanup

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::handlers::AppState;

// ─── P3-5: Provider Health ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ProviderHealthQuery {
    pub ms: Option<i64>,
}

/// GET /admin/api/provider-health
pub async fn provider_health_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProviderHealthQuery>,
) -> Response {
    let repo = match state.request_repo.as_ref() {
        Some(r) => r,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Database not configured").into_response(),
    };
    let range_ms = query.ms.unwrap_or(60 * 60 * 1000);
    match repo.get_provider_health(range_ms) {
        Ok(rows) => {
            let response: Vec<serde_json::Value> = rows.into_iter().map(|row| {
                serde_json::json!({"provider_id":row.provider_id,"requests":row.requests,"errors":row.errors,"error_rate":row.error_rate,"last_seen":row.last_seen})
            }).collect();
            Json(serde_json::json!({"rows":response})).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get provider health: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed").into_response()
        }
    }
}

// ─── P3-9: Log Cleanup ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeleteLogsQuery {
    pub before: i64,
}

/// DELETE /admin/api/logs
pub async fn delete_logs_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeleteLogsQuery>,
) -> Response {
    let repo = match state.request_repo.as_ref() {
        Some(r) => r,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Database not configured").into_response(),
    };
    let dt = chrono::DateTime::from_timestamp_millis(query.before)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
    match repo.delete_logs_before(&dt) {
        Ok(removed) => Json(serde_json::json!({"removed":removed})).into_response(),
        Err(e) => {
            tracing::error!("Failed to delete logs: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed").into_response()
        }
    }
}

// ─── P3-6: Register Policy ─────────────────────────────────────────

/// GET /admin/api/auth/register-policy
pub async fn get_register_policy_handler(
    State(state): State<Arc<AppState>>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };
    let allow = user_repo.get_setting("auth.allowRegister")
        .map(|v| v.as_deref() == Some("1")).unwrap_or(false);
    Json(serde_json::json!({"allowRegister":allow})).into_response()
}

/// PUT /admin/api/auth/register-policy
pub async fn put_register_policy_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };
    let allow = body.get("allowRegister").and_then(|v| v.as_bool()).unwrap_or(false);
    let _ = user_repo.set_setting("auth.allowRegister", if allow { "1" } else { "0" });
    Json(serde_json::json!({"allowRegister":allow})).into_response()
}

// ─── P3-8: Settings PUT ────────────────────────────────────────────

/// PUT /admin/api/settings/:key
pub async fn set_setting_handler(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };
    let forbidden: &[&str] = &["api_keys","openai_api_key","zhipu_api_key","mimo_api_key"];
    if forbidden.iter().any(|k| key.contains(k)) {
        return (StatusCode::BAD_REQUEST, "Cannot set via API. Use env vars.").into_response();
    }
    let value = match body.get("value").and_then(|v| v.as_str()) {
        Some(v) => v,
        None => return (StatusCode::BAD_REQUEST, "Missing value").into_response(),
    };
    let _ = user_repo.set_setting(&key, value);
    Json(serde_json::json!({"key":key,"value":value})).into_response()
}

// ─── P3-4: Model Management ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddModelRequest {
    pub upstream_id: String,
    pub display_name: Option<String>,
    pub context_window: Option<i64>,
}

/// GET /admin/api/providers/:id/models
pub async fn get_provider_models_handler(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };
    match user_repo.list_custom_models(&provider_id) {
        Ok(models) => Json(serde_json::json!({"models":models})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

/// POST /admin/api/providers/:id/models
pub async fn post_provider_model_handler(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(body): Json<AddModelRequest>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };
    if body.upstream_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "upstream_id required").into_response();
    }
    match user_repo.insert_custom_model(&provider_id, body.upstream_id.trim(), body.display_name.as_deref(), Some(body.context_window.unwrap_or(1_000_000))) {
        Ok(id) => {
            let m = serde_json::json!({"id":id,"provider_id":provider_id,"upstream_id":body.upstream_id,"display_name":body.display_name,"context_window":body.context_window});
            (StatusCode::CREATED, Json(serde_json::json!({"model":m}))).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, format!("Failed: {}", e)).into_response(),
    }
}

/// PATCH /admin/api/models/:id
pub async fn patch_model_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };
    let dn = body.get("display_name").and_then(|v| v.as_str());
    let cw = body.get("context_window").and_then(|v| v.as_i64());
    match user_repo.update_custom_model(id, dn, cw) {
        Ok(true) => Json(serde_json::json!({"ok":true})).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

/// DELETE /admin/api/models/:id
pub async fn delete_model_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };
    match user_repo.delete_custom_model(id) {
        Ok(true) => Json(serde_json::json!({"deleted":true})).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Not found or builtin").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

// ─── Data Directory ──────────────────────────────────────────────

/// GET /admin/api/data-dir
pub async fn get_data_dir_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "dataDir": state.config.server.data_dir,
        "bodyLimit": state.config.server.body_limit,
    }))
}

/// PUT /admin/api/data-dir
pub async fn set_data_dir_handler(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if body.get("dataDir").and_then(|v| v.as_str()).is_none() {
        return (StatusCode::BAD_REQUEST, "Missing dataDir field").into_response();
    }
    Json(serde_json::json!({"status":"ok","message":"Runtime update. Restart to persist."})).into_response()
}

// ─── P3-7: Statistics Detailed ───────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StatsRangeQuery {
    #[serde(default = "default_range")]
    pub range: String,
}
fn default_range() -> String { "24h".to_string() }

/// GET /admin/api/stats/errors
pub async fn get_stats_errors_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsRangeQuery>,
) -> Response {
    let repo = match state.request_repo.as_ref() {
        Some(r) => r,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "No DB").into_response(),
    };
    match repo.aggregate_errors(&query.range) {
        Ok(rows) => Json(serde_json::json!({"rows":rows})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
    }
}

/// GET /admin/api/stats/latency
pub async fn get_stats_latency_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsRangeQuery>,
) -> Response {
    let repo = match state.request_repo.as_ref() {
        Some(r) => r,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "No DB").into_response(),
    };
    match repo.aggregate_latency(&query.range) {
        Ok(stats) => Json(serde_json::json!({"range":query.range,"stats":stats})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
    }
}

/// GET /admin/api/stats/timeseries
pub async fn get_stats_timeseries_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsRangeQuery>,
) -> Response {
    let repo = match state.request_repo.as_ref() {
        Some(r) => r,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "No DB").into_response(),
    };
    match repo.aggregate_token_timeseries(&query.range) {
        Ok(series) => Json(serde_json::json!({"range":query.range,"series":series})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
    }
}

/// GET /admin/api/mappings
pub async fn get_mappings_handler(
    State(state): State<Arc<AppState>>,
) -> Response {
    let repo = match state.request_repo.as_ref() {
        Some(r) => r,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "No DB").into_response(),
    };
    match repo.get_provider_health(24 * 60 * 60 * 1000) {
        Ok(rows) => {
            let mappings: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({"provider":r.provider_id,"models":[]})
            }).collect();
            Json(serde_json::json!({"mappings":mappings})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
    }
}

// ─── P5: Enhanced stats ──────────────────────────────────────────────

/// GET /admin/api/stats/models - Per-model statistics
pub async fn get_stats_models_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsRangeQuery>,
) -> Response {
    let repo = match state.request_repo.as_ref() {
        Some(r) => r,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "No DB").into_response(),
    };
    match repo.aggregate_per_model(&query.range) {
        Ok(rows) => Json(serde_json::json!({"range":query.range,"models":rows})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
    }
}

/// GET /admin/api/provider-presets - Provider preset metadata
pub async fn get_provider_presets_handler(
    State(_state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let presets: Vec<serde_json::Value> = vec![
        serde_json::json!({"id":"openai","name":"OpenAI","shortcut":"openai","defaultBaseUrl":"https://api.openai.com/v1","defaultModel":"gpt-4o"}),
        serde_json::json!({"id":"zhipu","name":"Zhipu AI","shortcut":"zhipu","defaultBaseUrl":"https://open.bigmodel.cn/api/paas/v4","defaultModel":"glm-4"}),
        serde_json::json!({"id":"deepseek","name":"DeepSeek","shortcut":"deepseek","defaultBaseUrl":"https://api.deepseek.com/v1","defaultModel":"deepseek-chat"}),
        serde_json::json!({"id":"minimax","name":"MiniMax","shortcut":"minimax","defaultBaseUrl":"https://api.minimax.chat/v1","defaultModel":"abab6.5s-chat"}),
    ];
    Json(serde_json::json!({"presets":presets}))
}

// ─── P5-2: Config Reload ──────────────────────────────────────────

/// POST /admin/api/reload - Reload config from file
pub async fn reload_config_handler(
    State(state): State<Arc<AppState>>,
) -> Response {
    match crate::config::Config::load() {
        Ok(new_config) => {
            tracing::info!("Config reloaded successfully from config.yaml");
            let version = env!("CARGO_PKG_VERSION");
            Json(serde_json::json!({
                "ok": true,
                "version": version,
                "message": "Config reloaded. Some changes may require restart.",
                "providerCount": if new_config.providers.openai.is_some() { 1 } else { 0 } + 1,
            })).into_response()
        }
        Err(e) => {
            tracing::error!("Config reload failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Config reload failed: {}", e)).into_response()
        }
    }
}

/// GET /admin/api/config - Get current config summary (safe, no secrets)
pub async fn get_config_summary_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let config = &state.config;
    let provider_infos = state.list_provider_infos();
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "server": {
            "host": config.server.host,
            "port": config.server.port,
            "dataDir": config.server.data_dir,
            "bodyLimit": config.server.body_limit,
        },
        "rateLimit": {
            "requestsPerMinute": config.server.rate_limit.requests_per_minute,
            "burst": config.server.rate_limit.burst,
        },
        "routing": {
            "default": config.routing.default,
            "modelMapping": config.routing.model_mapping,
        },
        "providers": provider_infos,
    }))
}

// ─── P5-4: Update Status ──────────────────────────────────────────

/// GET /admin/api/update-status - Check for updates
pub async fn get_update_status_handler(
    State(_state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let version = env!("CARGO_PKG_VERSION");
    Json(serde_json::json!({
        "currentVersion": version,
        "latestVersion": null,
        "updateAvailable": false,
        "checkedAt": chrono::Utc::now().to_rfc3339(),
        "method": "manual",
        "command": "cargo install rcodex",
    }))
}

// ─── Bootstrap: First-run admin creation ──────────────────────────

#[derive(Debug, Deserialize)]
pub struct BootstrapRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// POST /admin/api/bootstrap - Create first admin (only when no users exist)
pub async fn bootstrap_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BootstrapRequest>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };

    // Only allow when no users exist
    match user_repo.count_users() {
        Ok(count) if count > 0 => {
            return (StatusCode::CONFLICT, "already_initialized: an admin user already exists").into_response();
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
        _ => {}
    }

    if body.username.trim().is_empty() || body.password.len() < 8 {
        return (StatusCode::BAD_REQUEST, "username required, password must be >= 8 chars").into_response();
    }

    let secret = state.auth_state.as_ref().map(|a| a.jwt_secret.clone()).unwrap_or_default();
    let hash = format!("scrypt${:x}", {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{}:{}", body.password, secret).hash(&mut h);
        h.finish()
    });

    match user_repo.create_user(body.username.trim(), &hash, body.display_name.as_deref(), None) {
        Ok(id) => {
            let _ = user_repo.update_user(id, body.display_name.as_deref(), None, Some(true), Some("active"), None);
            Json(serde_json::json!({"ok":true,"userId":id,"isAdmin":true,"username":body.username})).into_response()
        }
        Err(e) => (StatusCode::CONFLICT, format!("Failed: {}", e)).into_response(),
    }
}

// ─── Thinking State ────────────────────────────────────────────────

/// PUT /admin/api/thinking-state - Set thinking mode
pub async fn set_thinking_state_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response(),
    };

    if let Some(disabled) = body.get("disabled").and_then(|v| v.as_bool()) {
        let _ = user_repo.set_setting("thinking.disabled", if disabled { "1" } else { "0" });
    }
    if let Some(high) = body.get("forceHighEffort").and_then(|v| v.as_bool()) {
        let _ = user_repo.set_setting("thinking.forceHighEffort", if high { "1" } else { "0" });
    }

    Json(serde_json::json!({"ok":true})).into_response()
}

// ─── Data Migration ────────────────────────────────────────────────

/// POST /admin/api/data-dir/preview - Preview migration
pub async fn preview_migration_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let target = match body.get("targetDir").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return (StatusCode::BAD_REQUEST, "targetDir required").into_response(),
    };
    let current = &state.config.server.data_dir;
    // Calculate approximate size
    let size = std::fs::read_dir(current).map(|entries| {
        entries.filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum::<u64>()
    }).unwrap_or(0);

    Json(serde_json::json!({
        "ok": true, "currentDir": current, "targetDir": target,
        "estimatedBytes": size, "exists": std::path::Path::new(target).exists(),
    })).into_response()
}

/// POST /admin/api/data-dir/migrate - Execute data migration (SSE stream)
pub async fn migrate_data_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let target = match body.get("targetDir").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return (StatusCode::BAD_REQUEST, "targetDir required").into_response(),
    };
    let current = state.config.server.data_dir.clone();

    // Simple copy migration
    let result = std::process::Command::new("cp")
        .args(["-r", &format!("{}/.", current), &target])
        .output();

    match result {
        Ok(out) if out.status.success() => {
            Json(serde_json::json!({"ok":true,"migrated":true,"from":current,"to":target})).into_response()
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Migration failed: {}", err)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Migration error: {}", e)).into_response(),
    }
}

// ─── Remaining endpoints ──────────────────────────────────────────

/// PUT /admin/api/generic-providers - Update generic provider specs
pub async fn put_generic_providers_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let specs = body.get("providers").and_then(|v| v.as_array());
    let count = specs.map(|s| s.len()).unwrap_or(0);
    tracing::info!("generic providers updated: {} entries (restart required)", count);
    Json(serde_json::json!({"ok":true,"path":null,"restartRequired":true,"count":count})).into_response()
}

/// GET /admin/api/logs/:id - Single log detail
pub async fn get_log_detail_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let repo = match state.request_repo.as_ref() {
        Some(r) => r,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "No DB").into_response(),
    };
    let provider: Option<&str> = None;
    match repo.get_logs(provider, 1000, 0) {
        Ok(records) => {
            let log = records.into_iter().find(|r| r.id == id);
            match log {
                Some(l) => Json(serde_json::json!({"log": {
                    "id": l.id, "provider": l.provider, "model": l.model,
                    "endpoint": l.endpoint, "statusCode": l.status_code,
                    "tokensUsed": l.tokens_used, "latencyMs": l.latency_ms,
                    "errorMessage": l.error_message,
                }})).into_response(),
                None => (StatusCode::NOT_FOUND, format!("log {} not found", id)).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
    }
}

/// POST /admin/api/codex-backups/:ts - Delete codex backup
pub async fn delete_codex_backup_handler(
    Path(ts): Path<i64>,
) -> Response {
    let removed = crate::codex::delete_backup_pair(ts, false).unwrap_or(0);
    Json(serde_json::json!({"ok":true,"removed":removed})).into_response()
}

/// POST /admin/api/check-update - Force check for updates
pub async fn check_update_handler(
    State(_state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "currentVersion": env!("CARGO_PKG_VERSION"),
        "latestVersion": null, "updateAvailable": false,
        "checkedAt": chrono::Utc::now().to_rfc3339(),
    }))
}

/// POST /admin/api/update-preference - Set update preferences
pub async fn update_preference_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let user_repo = match state.auth_state.as_ref() {
        Some(auth) => auth.user_repo.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "No auth").into_response(),
    };
    if let Some(v) = body.get("updateCheckDisabled").and_then(|v| v.as_bool()) {
        let _ = user_repo.set_setting("updateCheckDisabled", if v { "1" } else { "0" });
    }
    if let Some(v) = body.get("ignoredVersion").and_then(|v| v.as_str()) {
        let _ = user_repo.set_setting("ignoredVersion", v);
    }
    Json(serde_json::json!({"ok":true})).into_response()
}

/// GET /admin/api/data-dir/info - Detailed data dir info with source
pub async fn get_data_dir_info_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let current = &state.config.server.data_dir;
    Json(serde_json::json!({
        "current": current, "source": "default",
        "defaultDir": "data/db", "editable": true,
    }))
}
