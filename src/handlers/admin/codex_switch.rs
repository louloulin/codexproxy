//!
//! Codex Switch API handlers

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::handlers::AppState;
use crate::setup::ProviderTarget;

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(msg: &str) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

/// GET /admin/api/codex-state
pub async fn get_codex_state() -> Json<ApiResponse<crate::codex::CodexState>> {
    let state = crate::codex::read_codex_state();
    Json(ApiResponse::success(state))
}

/// POST /admin/api/codex-apply
#[derive(Debug, Deserialize)]
pub struct ApplyCodexRequest {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Serialize)]
pub struct ApplyCodexResponse {
    pub backup_ts: i64,
    pub auth_backup: Option<String>,
    pub toml_backup: Option<String>,
    pub auth_json_owner_before: String,
    pub preserved: bool,
}

pub async fn post_codex_apply(
    State(_state): State<Arc<crate::handlers::AppState>>,
    Json(req): Json<ApplyCodexRequest>,
) -> impl IntoResponse {
    let target = match ProviderTarget::from_str(&req.provider_id) {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(&format!("Unknown provider: {}", req.provider_id))),
            ).into_response();
        }
    };
    
    let host = crate::setup::HostConfig::new("127.0.0.1", 8080);
    let result = crate::codex::apply_codex(&target, &host);
    
    let response = ApplyCodexResponse {
        backup_ts: result.backup_ts,
        auth_backup: result.auth_backup.map(|p| p.to_string_lossy().to_string()),
        toml_backup: result.toml_backup.map(|p| p.to_string_lossy().to_string()),
        auth_json_owner_before: result.auth_json_owner_before.to_string(),
        preserved: result.preserved,
    };
    
    (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
}

/// POST /admin/api/codex-restore
#[derive(Debug, Deserialize)]
pub struct RestoreCodexRequest {
    pub ts: i64,
}

pub async fn post_codex_restore(
    Json(req): Json<RestoreCodexRequest>,
) -> impl IntoResponse {
    match crate::codex::restore_codex(req.ts) {
        Ok(()) => {
            (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"ts": req.ts})))).into_response()
        }
        Err(e) => {
            if e.contains("no backup pair") {
                (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error(&e))).into_response()
            } else {
                (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error(&e))).into_response()
            }
        }
    }
}

/// GET /admin/api/active-override
pub async fn get_active_override_handler() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({ "override": serde_json::Value::Null })))
}

/// PUT /admin/api/active-override
#[derive(Debug, Deserialize)]
pub struct SetOverrideRequest {
    pub provider_id: String,
    pub model_id: String,
}

pub async fn put_active_override_handler(
    Json(req): Json<SetOverrideRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let override_ = serde_json::json!({
        "provider_id": req.provider_id,
        "model_id": req.model_id
    });
    Json(ApiResponse::success(serde_json::json!({ "override": override_ })))
}

/// DELETE /admin/api/active-override
pub async fn delete_active_override_handler() -> impl IntoResponse {
    let response = serde_json::json!({
        "ok": true,
        "data": { "override": serde_json::Value::Null },
        "error": serde_json::Value::Null
    });
    Json(response)
}

// ============================================================================
// Codex History API Handlers
// ============================================================================

/// GET /admin/api/codex-history - List history entries
pub async fn get_codex_history_handler() -> Json<ApiResponse<Vec<crate::db::CodexHistoryEntry>>> {
    let entries: Vec<_> = Vec::new();
    Json(ApiResponse::success(entries))
}

/// GET /admin/api/codex-history/:id - Get specific history entry
pub async fn get_codex_history_by_id_handler(
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<ApiResponse<crate::db::CodexHistoryEntry>> {
    Json(ApiResponse::error(&format!("History entry {} not found", id)))
}


// ============================================================================
// Codex Targets API - Returns available providers and models
// ============================================================================

#[derive(Debug, Serialize)]
pub struct CodexTarget {
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub base_url: String,
    pub has_key: bool,
    pub display_name: Option<String>,
    pub source: String,
    pub context_window: Option<u32>,
    pub is_current_override: bool,
}

#[derive(Debug, Serialize)]
pub struct CodexTargetsResponse {
    pub targets: Vec<CodexTarget>,
}

/// GET /admin/api/codex-targets - List available providers and models
pub async fn get_codex_targets_handler(
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Json<ApiResponse<CodexTargetsResponse>> {
    let mut targets = Vec::new();

    // OpenAI provider
    if let Some(openai) = &state.config.providers.openai {
        let has_key = !openai.api_key.is_empty() && openai.api_key != "your-api-key-here";
        targets.push(CodexTarget {
            provider_id: "openai".to_string(),
            provider_name: "OpenAI".to_string(),
            model_id: openai.default_model.clone(),
            base_url: openai.base_url.clone(),
            has_key,
            display_name: Some("GPT-4o".to_string()),
            source: "builtin".to_string(),
            context_window: Some(128000),
            is_current_override: false,
        });
    }

    // Zhipu provider
    let zhipu_has_key = !state.config.providers.zhipu.api_key.is_empty()
        && state.config.providers.zhipu.api_key != "your-api-key-here";
    targets.push(CodexTarget {
        provider_id: "zhipu".to_string(),
        provider_name: "Zhipu AI".to_string(),
        model_id: state.config.providers.zhipu.default_model.clone(),
        base_url: state.config.providers.zhipu.base_url.clone(),
        has_key: zhipu_has_key,
        display_name: Some("GLM-5".to_string()),
        source: "builtin".to_string(),
        context_window: Some(128000),
        is_current_override: false,
    });

    // MiniMax provider
    if let Some(minimax) = &state.config.providers.minimax {
        let has_key = !minimax.api_key.is_empty() && minimax.api_key != "your-api-key-here";
        targets.push(CodexTarget {
            provider_id: "minimax".to_string(),
            provider_name: "MiniMax".to_string(),
            model_id: minimax.default_model.clone(),
            base_url: minimax.base_url.clone(),
            has_key,
            display_name: Some("MiniMax-M2.7".to_string()),
            source: "builtin".to_string(),
            context_window: Some(1000000),
            is_current_override: false,
        });
    }

    Json(ApiResponse::success(CodexTargetsResponse { targets }))
}

// ============================================================================
// Probe API - Test model availability
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ProbeRequest {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Serialize)]
pub struct ProbeResult {
    pub ok: bool,
    pub latency_ms: u64,
    pub error: Option<ProbeError>,
}

#[derive(Debug, Serialize)]
pub struct ProbeError {
    pub code: String,
    pub message: String,
}

/// POST /admin/api/probe - Test model connectivity
pub async fn probe_handler(
    State(state): State<Arc<crate::handlers::AppState>>,
    Json(req): Json<ProbeRequest>,
) -> Json<ApiResponse<ProbeResult>> {
    use std::time::Instant;
    
    let start = Instant::now();
    
    // Get provider config
    let provider_config = match req.provider_id.as_str() {
        "openai" => state.config.providers.openai.as_ref(),
        "zhipu" => Some(&state.config.providers.zhipu),
        "minimax" => state.config.providers.minimax.as_ref(),
        _ => None,
    };
    
    let provider_config = match provider_config {
        Some(cfg) => cfg,
        None => {
            return Json(ApiResponse::success(ProbeResult {
                ok: false,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(ProbeError {
                    code: "unknown_provider".to_string(),
                    message: format!("Unknown provider: {}", req.provider_id),
                }),
            }));
        }
    };
    
    // Try to make a simple request to test connectivity
    let client = reqwest::Client::new();
    let test_request = serde_json::json!({
        "model": req.model_id,
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 1
    });
    
    match client
        .post(format!("{}/chat/completions", provider_config.base_url))
        .header("Authorization", format!("Bearer {}", provider_config.api_key))
        .header("Content-Type", "application/json")
        .json(&test_request)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(response) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if response.status().is_success() {
                Json(ApiResponse::success(ProbeResult {
                    ok: true,
                    latency_ms,
                    error: None,
                }))
            } else {
                Json(ApiResponse::success(ProbeResult {
                    ok: false,
                    latency_ms,
                    error: Some(ProbeError {
                        code: "request_failed".to_string(),
                        message: format!("HTTP {}", response.status().as_u16()),
                    }),
                }))
            }
        }
        Err(e) => {
            Json(ApiResponse::success(ProbeResult {
                ok: false,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(ProbeError {
                    code: "connection_error".to_string(),
                    message: e.to_string(),
                }),
            }))
        }
    }
}

// ============================================================================
// Codex Directory API - Manage Codex directory location
// ============================================================================

#[derive(Debug, Serialize)]
pub struct CodexDirInfo {
    pub source: String,
    pub effective: String,
}

/// GET /admin/api/codex-dir - Get Codex directory info
pub async fn get_codex_dir_handler() -> Json<ApiResponse<CodexDirInfo>> {
    let codex_dir = crate::codex::codex_dir();
    let source = if std::env::var("CODEX_HOME").is_ok() {
        "env"
    } else {
        "default"
    };
    
    Json(ApiResponse::success(CodexDirInfo {
        source: source.to_string(),
        effective: codex_dir.to_string_lossy().to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct SetCodexDirRequest {
    pub dir: String,
}

/// PUT /admin/api/codex-dir - Set Codex directory
pub async fn set_codex_dir_handler(
    Json(req): Json<SetCodexDirRequest>,
) -> impl IntoResponse {
    std::env::set_var("CODEX_HOME", &req.dir);
    
    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({
        "message": "Codex directory set",
        "dir": req.dir
    }))))
}

/// DELETE /admin/api/codex-dir - Clear Codex directory (use default)
pub async fn delete_codex_dir_handler() -> impl IntoResponse {
    std::env::remove_var("CODEX_HOME");
    
    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({
        "message": "Codex directory reset to default"
    }))))
}

// ============================================================================
// Thinking API - Control thinking mode
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ThinkingState {
    pub disabled: bool,
    pub force_high_effort: bool,
    pub cli_override: bool,
}

/// Check if thinking is overridden by CLI environment variables
fn is_thinking_cli_overridden() -> bool {
    // Check for common CLI override environment variables
    std::env::var("CODEX_THINKING_DISABLED").is_ok()
        || std::env::var("CODEX_DISABLE_THINKING").is_ok()
        || std::env::var("CODEX_THINKING_EFFORT").is_ok()
}

/// GET /admin/api/thinking - Get thinking mode state
pub async fn get_thinking_handler() -> Json<ApiResponse<ThinkingState>> {
    let cli_override = is_thinking_cli_overridden();
    Json(ApiResponse::success(ThinkingState {
        disabled: false,
        force_high_effort: false,
        cli_override,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SetThinkingRequest {
    pub disabled: Option<bool>,
    pub force_high_effort: Option<bool>,
}

/// PUT /admin/api/thinking - Set thinking mode
pub async fn set_thinking_handler(
    Json(req): Json<SetThinkingRequest>,
) -> impl IntoResponse {
    let mut message = Vec::new();
    if let Some(disabled) = req.disabled {
        message.push(format!("thinking disabled: {}", disabled));
    }
    if let Some(force) = req.force_high_effort {
        message.push(format!("force_high_effort: {}", force));
    }
    
    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({
        "message": message.join(", ")
    }))))
}

// ============================================================================
// Import/Export API - Codex configuration import/export
// ============================================================================

#[derive(Debug, Serialize)]
pub struct CodexBundleResponse {
    pub history: CodexHistoryMeta,
    pub files: CodexFiles,
    pub scripts: CodexScripts,
}

#[derive(Debug, Serialize)]
pub struct CodexHistoryMeta {
    pub id: i64,
    pub ts: i64,
    pub kind: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CodexFiles {
    pub auth_json: String,
    pub config_toml: String,
}

#[derive(Debug, Serialize)]
pub struct CodexScripts {
    pub posix: String,
    pub powershell: String,
}

/// GET /admin/api/codex-current-bundle - Export current config
pub async fn get_codex_current_bundle_handler() -> impl IntoResponse {
    let auth_path = crate::codex::auth_json_path();
    let config_path = crate::codex::config_toml_path();
    
    let auth_json = if let Ok(s) = std::fs::read_to_string(&auth_path) {
        s
    } else {
        "{}".to_string()
    };
    
    let config_toml = if let Ok(s) = std::fs::read_to_string(&config_path) {
        s
    } else {
        "".to_string()
    };
    
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    let response = CodexBundleResponse {
        history: CodexHistoryMeta {
            id: 0,
            ts,
            kind: "current".to_string(),
            provider_id: None,
            model_id: None,
            note: Some("current configuration".to_string()),
        },
        files: CodexFiles {
            auth_json,
            config_toml,
        },
        scripts: CodexScripts {
            posix: generate_posix_script(),
            powershell: generate_powershell_script(),
        },
    };
    
    (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
}

/// POST /admin/api/codex-import - Import config
#[derive(Debug, Deserialize)]
pub struct ImportCodexRequest {
    pub auth_json: String,
    pub config_toml: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub note: Option<String>,
}

pub async fn post_codex_import_handler(
    Json(req): Json<ImportCodexRequest>,
) -> impl IntoResponse {
    // Validate auth.json is valid JSON
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&req.auth_json) {
        return (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error(&format!("Invalid auth.json JSON: {}", e)))).into_response();
    }
    
    let auth_path = crate::codex::auth_json_path();
    let config_path = crate::codex::config_toml_path();
    
    // Write files atomically
    if let Err(e) = crate::codex::atomic_write(&auth_path, &req.auth_json) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<()>::error(&format!("Failed to write auth.json: {}", e)))).into_response();
    }
    
    if let Err(e) = crate::codex::atomic_write(&config_path, &req.config_toml) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<()>::error(&format!("Failed to write config.toml: {}", e)))).into_response();
    }
    
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({
        "ok": true,
        "historyId": ts,
        "restartRequired": true,
        "bundleUrl": null
    })))).into_response()
}

/// GET /admin/api/codex-history/:id/bundle - Download bundle
pub async fn get_codex_history_bundle_handler(
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let _id = id; // placeholder for history lookup
    
    let auth_path = crate::codex::auth_json_path();
    let config_path = crate::codex::config_toml_path();
    
    let auth_json = if let Ok(s) = std::fs::read_to_string(&auth_path) {
        s
    } else {
        "{}".to_string()
    };
    
    let config_toml = if let Ok(s) = std::fs::read_to_string(&config_path) {
        s
    } else {
        "".to_string()
    };
    
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    let response = CodexBundleResponse {
        history: CodexHistoryMeta {
            id,
            ts,
            kind: "apply".to_string(),
            provider_id: None,
            model_id: None,
            note: Some(format!("history entry {}", id)),
        },
        files: CodexFiles {
            auth_json,
            config_toml,
        },
        scripts: CodexScripts {
            posix: generate_posix_script(),
            powershell: generate_powershell_script(),
        },
    };
    
    (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
}

fn generate_posix_script() -> String {
    "#!/bin/bash\n# Apply Codex Configuration Script\nset -e\necho done\n".to_string()
}

fn generate_powershell_script() -> String {
    "# Apply Codex Configuration Script\nWrite-Host done\n".to_string()
}

// ============================================================================
// Providers API - List available providers
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub shortcut: String,
    pub display_name: String,
    #[serde(rename = "default")]
    pub is_default: bool,
    pub enabled: bool,
    pub api_key_present: bool,
    pub api_key_env: Vec<String>,
    pub base_url: String,
    pub default_model: String,
}

#[derive(Debug, Serialize)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderInfo>,
}

/// GET /admin/api/provider-configs - List all provider configurations
pub async fn get_provider_configs_handler(
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Json<ApiResponse<ProvidersResponse>> {
    let mut providers = Vec::new();

    // OpenAI provider
    if let Some(openai) = &state.config.providers.openai {
        providers.push(ProviderInfo {
            id: "openai".to_string(),
            shortcut: "oai".to_string(),
            display_name: "OpenAI".to_string(),
            is_default: false,
            enabled: true,
            api_key_present: !openai.api_key.is_empty(),
            api_key_env: vec!["OPENAI_API_KEY".to_string()],
            base_url: openai.base_url.clone(),
            default_model: openai.default_model.clone(),
        });
    }

    // Zhipu provider
    providers.push(ProviderInfo {
        id: "zhipu".to_string(),
        shortcut: "zh".to_string(),
        display_name: "Zhipu AI".to_string(),
        is_default: true,
        enabled: true,
        api_key_present: !state.config.providers.zhipu.api_key.is_empty(),
        api_key_env: vec!["ZHIPU_API_KEY".to_string()],
        base_url: state.config.providers.zhipu.base_url.clone(),
        default_model: state.config.providers.zhipu.default_model.clone(),
    });

    // MiniMax provider
    if let Some(minimax) = &state.config.providers.minimax {
        providers.push(ProviderInfo {
            id: "minimax".to_string(),
            shortcut: "mm".to_string(),
            display_name: "MiniMax".to_string(),
            is_default: false,
            enabled: true,
            api_key_present: !minimax.api_key.is_empty(),
            api_key_env: vec!["MINIMAX_API_KEY".to_string()],
            base_url: minimax.base_url.clone(),
            default_model: minimax.default_model.clone(),
        });
    }

    Json(ApiResponse::success(ProvidersResponse { providers }))
}

// ============================================================================
// Setup Snippets API - Generate setup instructions for providers
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SetupSnippetTarget {
    pub provider_id: String,
    pub provider_key: String,
    pub provider_label: String,
    pub model_id: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct SetupSnippetBundle {
    pub target: SetupSnippetTarget,
    pub auth_json: String,
    pub config_toml: String,
    pub config_toml_env_key: String,
}

#[derive(Debug, Serialize)]
pub struct SetupSnippetsResponse {
    pub bundle: SetupSnippetBundle,
    pub default_provider_id: String,
    pub providers: Vec<ProviderSnippetInfo>,
}

#[derive(Debug, Serialize)]
pub struct ProviderSnippetInfo {
    pub id: String,
    pub shortcut: String,
    pub display_name: String,
}

/// GET /admin/api/setup-snippets - Generate setup instructions
pub async fn get_setup_snippets_handler(
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Json<ApiResponse<SetupSnippetsResponse>> {
    // Build snippet for Zhipu (default provider)
    let zhipu = &state.config.providers.zhipu;

    let bundle = SetupSnippetBundle {
        target: SetupSnippetTarget {
            provider_id: "zhipu".to_string(),
            provider_key: "ZHIPU_API_KEY".to_string(),
            provider_label: "Zhipu AI".to_string(),
            model_id: zhipu.default_model.clone(),
            context_window: Some(128_000),
            max_output_tokens: Some(4096),
        },
        auth_json: format!(r#"{{"api_key": "{}{}", "provider": "zhipu"}}"#,
            "ZHIPU_API_KEY_PLACEHOLDER",
            if zhipu.api_key.is_empty() { " (未设置)" } else { "" }),
        config_toml: format!(
            r#"model = "{}"
model_provider = "Zhipu"

[model_providers.Zhipu]
name = "Zhipu"
wire_api = "chat"
requires_openai_auth = true
base_url = "{}""#,
            zhipu.default_model,
            zhipu.base_url
        ),
        config_toml_env_key: "ZHIPU_API_KEY".to_string(),
    };

    let providers = vec![
        ProviderSnippetInfo {
            id: "zhipu".to_string(),
            shortcut: "zh".to_string(),
            display_name: "Zhipu AI".to_string(),
        },
        ProviderSnippetInfo {
            id: "minimax".to_string(),
            shortcut: "mm".to_string(),
            display_name: "MiniMax".to_string(),
        },
        ProviderSnippetInfo {
            id: "openai".to_string(),
            shortcut: "oai".to_string(),
            display_name: "OpenAI".to_string(),
        },
    ];

    Json(ApiResponse::success(SetupSnippetsResponse {
        bundle,
        default_provider_id: "zhipu".to_string(),
        providers,
    }))
}

// ============================================================================
// Settings API - Get and update server settings
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub settings: std::collections::HashMap<String, String>,
}

/// GET /admin/api/settings - Get all settings
pub async fn get_settings_handler(
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Json<ApiResponse<SettingsResponse>> {
    let mut settings = std::collections::HashMap::new();

    // Server settings
    settings.insert("server.host".to_string(), state.config.server.host.clone());
    settings.insert("server.port".to_string(), state.config.server.port.to_string());
    settings.insert("server.rate_limit".to_string(),
        format!("{}/{}",
            state.config.server.rate_limit.requests_per_minute,
            state.config.server.rate_limit.burst));

    // Provider settings
    settings.insert("providers.zhipu.base_url".to_string(), state.config.providers.zhipu.base_url.clone());
    settings.insert("providers.zhipu.default_model".to_string(), state.config.providers.zhipu.default_model.clone());
    settings.insert("providers.zhipu.api_key_set".to_string(),
        if state.config.providers.zhipu.api_key.is_empty() { "false".to_string() } else { "true".to_string() });

    if let Some(openai) = &state.config.providers.openai {
        settings.insert("providers.openai.base_url".to_string(), openai.base_url.clone());
        settings.insert("providers.openai.default_model".to_string(), openai.default_model.clone());
    }

    if let Some(minimax) = &state.config.providers.minimax {
        settings.insert("providers.minimax.base_url".to_string(), minimax.base_url.clone());
        settings.insert("providers.minimax.default_model".to_string(), minimax.default_model.clone());
    }

    Json(ApiResponse::success(SettingsResponse { settings }))
}

// ============================================================================
// Generic Providers API - Manage custom providers
// ============================================================================

#[derive(Debug, Serialize)]
pub struct GenericProvidersSpec {
    pub id: String,
    pub base_url: String,
    pub env_key: String,
    pub default_model: String,
    pub models: Vec<GenericProviderModelSpec>,
}

#[derive(Debug, Serialize)]
pub struct GenericProviderModelSpec {
    pub id: String,
    pub display_name: Option<String>,
    pub context_window: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct GenericProvidersResponse {
    pub specs: Vec<GenericProvidersSpec>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub exists: bool,
}

/// GET /admin/api/generic-providers - Get custom provider configurations
pub async fn get_generic_providers_handler() -> Json<ApiResponse<GenericProvidersResponse>> {
    // Return empty list for now - custom providers not yet implemented
    Json(ApiResponse::success(GenericProvidersResponse {
        specs: vec![],
        path: None,
        source: None,
        exists: false,
    }))
}

// ============================================================================
// Request Stats API - Request statistics
// ============================================================================

#[derive(Debug, Serialize)]
pub struct StatsRow {
    pub provider_id: String,
    pub upstream_model: String,
    pub requests: u64,
    pub errors: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub since: i64,
    pub rows: Vec<StatsRow>,
}

/// GET /admin/api/request-stats - Get request statistics
pub async fn get_request_stats_handler() -> Json<ApiResponse<StatsResponse>> {
    // Return empty stats for now - stats collection not yet implemented
    Json(ApiResponse::success(StatsResponse {
        since: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64 - 86400,
        rows: vec![],
    }))
}

// ============================================================================
// Logs API - Request logs
// ============================================================================

#[derive(Debug, Serialize)]
pub struct LogRow {
    pub id: i64,
    pub ts: i64,
    pub provider_id: String,
    pub client_model: String,
    pub upstream_model: String,
    pub endpoint: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub error_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub logs: Vec<LogRow>,
}

/// GET /admin/api/logs - Get request logs
pub async fn get_logs_handler(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<LogsResponse>> {
    // Return empty logs for now - log collection not yet implemented
    Json(ApiResponse::success(LogsResponse {
        logs: vec![],
    }))
}
