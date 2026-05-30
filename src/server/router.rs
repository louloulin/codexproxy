use crate::auth::AuthState;
use crate::config::Config;
use crate::db::schema::init_database;
use crate::db::user_repository::UserRepository;
use crate::db::repository::RequestRepository;
use crate::handlers::{self, AppState};
use crate::handlers::admin::AdminState;
use crate::server::static_files;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post, delete, put, patch},
    Router,
};
use governor::{DefaultKeyedRateLimiter, Quota};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::{CorsLayer, AllowOrigin, AllowHeaders, AllowMethods, ExposeHeaders}, trace::TraceLayer};

/// Rate limiting state wrapper
pub struct RateLimitState {
    pub limiter: Arc<DefaultKeyedRateLimiter<String>>,
}

impl RateLimitState {
    pub fn new(requests_per_minute: u32, burst: u32) -> Self {
        let per_second = requests_per_minute / 60;
        let quota = Quota::per_second(NonZeroU32::new(per_second.max(1)).unwrap())
            .allow_burst(NonZeroU32::new(burst.max(1)).unwrap());

        let limiter = Arc::new(DefaultKeyedRateLimiter::keyed(quota));
        Self { limiter }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(rate_limit_state): State<Arc<RateLimitState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    let path = request.uri().path();
    if path == "/" || path == "/health" || path.starts_with("/admin") {
        return Ok(next.run(request).await);
    }

    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    match rate_limit_state.limiter.check_key(&client_ip) {
        Ok(_) => Ok(next.run(request).await),
        Err(_) => {
            tracing::warn!("Rate limit exceeded for client: {}", client_ip);
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded. Please try again later.",
            ))
        }
    }
}

/// Create the application router with all routes configured
pub fn create_router(
    state: Arc<AppState>,
    _admin_state: Arc<AdminState>,
) -> Router {
    let config = &state.config;
    let rate_limit_state = Arc::new(RateLimitState::new(
        config.server.rate_limit.requests_per_minute,
        config.server.rate_limit.burst,
    ));

    // Build the main router with all routes
    Router::new()
        .route("/", get(handlers::health_check))
        .route("/health", get(handlers::health_check))
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/responses", post(handlers::responses))
        .route("/v1/models", get(handlers::models))
        .route("/responses", post(handlers::responses))
        .route(
            "/v1/providers/zhipu/chat/completions",
            post(handlers::zhipu_chat_completions),
        )
        // MiniMax API - uses same endpoint as OpenAI
        .route(
            "/v1/text/chatcompletion_v2",
            post(handlers::minimax_chat_completions),
        )
        // Admin SPA
        .route("/admin", axum::routing::get(static_files::serve_spa))
        .route("/api/status", get(crate::handlers::admin::api_status))
        .route("/api/providers", get(crate::handlers::admin::api_providers))
        .route("/api/stats", get(crate::handlers::admin::api_stats))
        .route("/api/test-marker", get(crate::handlers::admin::test_stats_marker))
        .route("/api/codex-state", get(crate::handlers::admin::get_codex_state))
        .route("/api/codex-apply", post(crate::handlers::admin::post_codex_apply))
        .route("/api/codex-restore", post(crate::handlers::admin::post_codex_restore))
        .route("/api/active-override", get(crate::handlers::admin::get_active_override_handler))
        .route("/api/active-override", put(crate::handlers::admin::put_active_override_handler))
        .route("/api/active-override", delete(crate::handlers::admin::delete_active_override_handler))
        .route("/api/codex-history", get(crate::handlers::admin::get_codex_history_handler))
        .route("/api/codex-history/:id/bundle", get(crate::handlers::admin::get_codex_history_bundle_handler))
        .route("/api/codex-current-bundle", get(crate::handlers::admin::get_codex_current_bundle_handler))
        .route("/api/codex-import", post(crate::handlers::admin::post_codex_import_handler))
        .route("/api/codex-history/:id", get(crate::handlers::admin::get_codex_history_by_id_handler))
        // Codex targets and probe
        .route("/api/codex-targets", get(crate::handlers::admin::get_codex_targets_handler))
        .route("/api/probe", post(crate::handlers::admin::probe_handler))
        // Codex directory management
        .route("/api/codex-dir", get(crate::handlers::admin::get_codex_dir_handler))
        .route("/api/codex-dir", put(crate::handlers::admin::set_codex_dir_handler))
        .route("/api/codex-dir", delete(crate::handlers::admin::delete_codex_dir_handler))
        // Thinking mode
        .route("/api/thinking", get(crate::handlers::admin::get_thinking_handler))
        .route("/api/thinking", put(crate::handlers::admin::set_thinking_handler))
        // Provider configs (detailed)
        .route("/api/provider-configs", get(crate::handlers::admin::get_provider_configs_handler))
        // Setup snippets
        .route("/api/setup-snippets", get(crate::handlers::admin::get_setup_snippets_handler))
        // Provider models (custom model management)
        .route("/api/providers/:id/models", get(crate::handlers::admin::get_provider_models_handler))
        .route("/api/providers/:id/models", post(crate::handlers::admin::post_provider_model_handler))
        .route("/api/models/:id", axum::routing::patch(crate::handlers::admin::patch_model_handler))
        .route("/api/models/:id", delete(crate::handlers::admin::delete_model_handler))
        // Settings
        .route("/api/settings", get(crate::handlers::admin::get_settings_handler))
        // Generic providers
        .route("/api/generic-providers", get(crate::handlers::admin::get_generic_providers_handler))
        // Request stats
        .route("/api/request-stats", get(crate::handlers::admin::get_request_stats_handler))
        // Test route
        .route("/api/test-stats", get(crate::handlers::admin::get_request_stats_handler))
        // Stats detailed
        .route("/api/stats/errors", get(crate::handlers::admin::get_stats_errors_handler))
        .route("/api/stats/latency", get(crate::handlers::admin::get_stats_latency_handler))
        .route("/api/stats/timeseries", get(crate::handlers::admin::get_stats_timeseries_handler))
        .route("/api/stats/models", get(crate::handlers::admin::get_stats_models_handler))
        .route("/api/mappings", get(crate::handlers::admin::get_mappings_handler))
        .route("/api/provider-presets", get(crate::handlers::admin::get_provider_presets_handler))
        // Config management
        .route("/api/reload", post(crate::handlers::admin::reload_config_handler))
        .route("/api/config", get(crate::handlers::admin::get_config_summary_handler))
        // Update check
        .route("/api/update-status", get(crate::handlers::admin::get_update_status_handler))
        // Bootstrap (first-run setup)
        .route("/api/bootstrap", post(crate::handlers::admin::bootstrap_handler))
        .route("/api/bootstrap-status", get(crate::handlers::admin::bootstrap_status_handler))
        // Thinking state
        .route("/api/thinking-state", put(crate::handlers::admin::set_thinking_state_handler))
        // Data migration
        .route("/api/data-dir/info", get(crate::handlers::admin::get_data_dir_info_handler))
        .route("/api/data-dir/preview", post(crate::handlers::admin::preview_migration_handler))
        .route("/api/data-dir/migrate", post(crate::handlers::admin::migrate_data_handler))
        // Generic providers update
        .route("/api/generic-providers", put(crate::handlers::admin::put_generic_providers_handler))
        // Single log detail
        .route("/api/logs/:id", get(crate::handlers::admin::get_log_detail_handler))
        // Codex backup management
        .route("/api/codex-backups/:ts", post(crate::handlers::admin::delete_codex_backup_handler))
        // Update management
        .route("/api/check-update", post(crate::handlers::admin::check_update_handler))
        .route("/api/update-preference", post(crate::handlers::admin::update_preference_handler))
        // Logs
        .route("/api/logs", get(crate::handlers::admin::get_logs_handler))
        .route("/api/logs", delete(crate::handlers::admin::delete_logs_handler))
        // Provider health
        .route("/api/provider-health", get(crate::handlers::admin::provider_health_handler))
        // Auth register policy
        .route("/api/auth/register-policy", get(crate::handlers::admin::get_register_policy_handler))
        .route("/api/auth/register-policy", put(crate::handlers::admin::put_register_policy_handler))
        // Settings (individual key operations)
        .route("/api/settings/:key", put(crate::handlers::admin::set_setting_handler))
        // User management (admin-only)
        .route("/api/users", get(crate::handlers::admin::list_users))
        .route("/api/users", post(crate::handlers::admin::create_user))
        .route("/api/users/:id", patch(crate::handlers::admin::update_user))
        .route("/api/users/:id", delete(crate::handlers::admin::delete_user))
        // Auth routes (use AppState which contains auth_state)
        .route("/api/auth/login", post(crate::auth::handlers::login))
        .route("/api/auth/register", post(crate::auth::handlers::register))
        .route("/api/auth/logout", post(crate::auth::handlers::logout))
        .route("/api/me", get(crate::auth::handlers::get_me))
        .route("/api/me/api-keys", get(crate::auth::handlers::list_api_keys))
        .route("/api/me/api-keys", post(crate::auth::handlers::create_api_key))
        .route("/api/me/api-keys/:id", delete(crate::auth::handlers::revoke_api_key))
        // Upstream keys (BYOK)
        .route("/api/me/upstream-keys", get(crate::auth::handlers::list_upstream_keys))
        .route("/api/me/upstream-keys/:providerId", put(crate::auth::handlers::set_upstream_key))
        .route("/api/me/upstream-keys/:providerId", delete(crate::auth::handlers::delete_upstream_key))
        // OAuth clients (admin-only)
        .route("/api/oauth-clients", get(crate::auth::handlers::list_oauth_clients))
        .route("/api/oauth-clients/:provider", put(crate::auth::handlers::upsert_oauth_client))
        .route("/api/oauth-clients/:provider", delete(crate::auth::handlers::delete_oauth_client))
        .route("/api/auth/oauth-providers", get(crate::auth::handlers::get_oauth_providers))
        // Health check
        .route("/api/health", get(handlers::health_check))
        // Data migration
        .route("/api/data-dir", get(crate::handlers::admin::get_data_dir_handler))
        .route("/api/data-dir", put(crate::handlers::admin::set_data_dir_handler))
        // Static file serving for admin SPA (embedded)
        .route("/admin/assets/*path", axum::routing::get(static_files::serve_spa))
        .route("/admin/*path", axum::routing::get(static_files::serve_spa))
        .layer(axum::extract::DefaultBodyLimit::max(
            config.server.body_limit,
        ))
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|request: &Request<Body>| {
                            let content_length = request
                                .headers()
                                .get(axum::http::header::CONTENT_LENGTH)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown");
                            tracing::info_span!(
                                "http_request",
                                method = %request.method(),
                                uri = %request.uri(),
                                content_length
                            )
                        }),
                )
                .layer(CorsLayer::new()
                    .allow_origin(AllowOrigin::any())
                    .allow_headers(AllowHeaders::any())
                    .allow_methods(AllowMethods::any())
                    .expose_headers(ExposeHeaders::any())
                ),
        )
        .layer(middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit_middleware,
        ))
        .with_state(state)
}

pub async fn create_server(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database
    let db_path = Path::new(&config.server.data_dir).join("rcodex.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = init_database(&db_path)
        .map_err(|e| format!("Failed to initialize database at {:?}: {}", db_path, e))?;
    let request_repo = Arc::new(RequestRepository::new(conn));
    // Re-open for user repo (SQLite single-writer constraint)
    let conn2 = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    let user_repo = Arc::new(UserRepository::new(conn2));

    let auth_state = Arc::new(AuthState {
        user_repo,
        jwt_secret: std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "rcodex-dev-secret-change-in-production".to_string()),
    });

    let mut app_state = AppState::new_with_auth(config.clone(), Some(auth_state));
    app_state.request_repo = Some(request_repo);
    let state = Arc::new(app_state);
    let admin_state = Arc::new(AdminState::new());
    let app = create_router(state, admin_state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!(
        "Server listening on {} (rate limit: {} req/min, burst: {})",
        addr,
        config.server.rate_limit.requests_per_minute,
        config.server.rate_limit.burst
    );
    tracing::info!("Admin UI available at http://{}/admin", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
