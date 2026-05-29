//! Auth API handlers
//!
//! Handlers for:
//! - POST /admin/api/auth/login
//! - POST /admin/api/auth/logout
//! - GET /admin/api/me
//! - GET /admin/api/me/api-keys
//! - POST /admin/api/me/api-keys
//! - DELETE /admin/api/me/api-keys/:id

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::{Duration, Utc};
use uuid::Uuid;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::db::UserRepository;
use crate::error::Error;

/// Auth state shared across auth handlers
#[derive(Clone)]
pub struct AuthState {
    pub user_repo: Arc<UserRepository>,
    pub jwt_secret: String,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserSummary,
}

/// User summary (safe to expose)
#[derive(Debug, Serialize)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
}

/// Register request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

/// Create API key request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

/// Create API key response (full key only shown once)
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: i64,
    pub name: String,
    pub key: String,        // Full key - only shown once!
    pub key_prefix: String,
    pub created_at: String,
}

/// Simple hash function for demo purposes (use bcrypt/argon2 in production)
fn hash_password(password: &str, secret: &str) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{}:{}", password, secret).hash(&mut hasher);
    format!("scrypt${:x}", hasher.finish())
}

/// Simple token generation (use JWT in production)
fn generate_token(user_id: i64, secret: &str) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{}:{}:{}", user_id, secret, Utc::now().timestamp()).hash(&mut hasher);
    format!("m2c_{:x}", hasher.finish())
}

/// Generate API key
fn generate_api_key() -> (String, String, String) {
    let mut hasher = DefaultHasher::new();
    format!("m2c:{}:{}", Uuid::new_v4(), Utc::now().timestamp_nanos()).hash(&mut hasher);
    let key = format!("m2c_{:x}", hasher.finish());
    let prefix = key.chars().take(8).collect::<String>();
    let hash = format!("sha256:{:x}", {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        h.finish()
    });
    (key, prefix, hash)
}

/// Extract user from auth header/cookie
fn extract_user_from_request(
    headers: &HeaderMap,
    auth_state: &AuthState,
) -> Result<i64, Error> {
    // Check Authorization header (Bearer token)
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let session = auth_state.user_repo.find_session(token)
                    .map_err(|e| Error::Internal(format!("DB error: {}", e)))?;
                if let Some(session) = session {
                    return Ok(session.user_id);
                }
            }
        }
    }

    // Check cookie
    if let Some(cookie_header) = headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix("m2c_session=") {
                    let session = auth_state.user_repo.find_session(token)
                        .map_err(|e| Error::Internal(format!("DB error: {}", e)))?;
                    if let Some(session) = session {
                        return Ok(session.user_id);
                    }
                }
            }
        }
    }

    Err(Error::Auth("Authentication required".to_string()))
}

/// POST /admin/api/auth/login
pub async fn login(
    State(state): State<Arc<crate::handlers::AppState>>,
    Json(body): Json<LoginRequest>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    let user = match auth_state.user_repo.find_by_username(&body.username) {
        Ok(Some(u)) => u,
        Ok(None) => return Error::Auth("Invalid username or password".to_string()).into_response(),
        Err(e) => return Error::Internal(format!("DB error: {}", e)).into_response(),
    };

    // Verify password
    let expected_hash = hash_password(&body.password, &auth_state.jwt_secret);
    if user.password_hash.as_deref() != Some(&expected_hash) {
        return Error::Auth("Invalid username or password".to_string()).into_response();
    }

    if user.status != "active" {
        return Error::Auth("Account is disabled".to_string()).into_response();
    }

    // Create session
    let token = generate_token(user.id, &auth_state.jwt_secret);
    let expires_at = Utc::now() + Duration::days(7);

    if let Err(e) = auth_state.user_repo.create_session(
        &token,
        user.id,
        &expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    ) {
        return Error::Internal(format!("Failed to create session: {}", e)).into_response();
    }

    let response = LoginResponse {
        token: token.clone(),
        user: UserSummary {
            id: user.id,
            username: user.username.clone(),
            display_name: user.display_name,
            email: user.email,
            avatar_url: user.avatar_url,
            is_admin: user.is_admin,
        },
    };

    // Set cookie
    let cookie = format!(
        "m2c_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        token,
        7 * 24 * 3600
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

    (StatusCode::OK, headers, Json(response)).into_response()
}

/// POST /admin/api/auth/register
pub async fn register(
    State(state): State<Arc<crate::handlers::AppState>>,
    Json(body): Json<RegisterRequest>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    if body.username.len() < 3 || body.username.len() > 32 {
        return Error::InvalidRequest("Username must be 3-32 characters".to_string()).into_response();
    }
    if body.password.len() < 6 {
        return Error::InvalidRequest("Password must be at least 6 characters".to_string()).into_response();
    }

    let user_count = match auth_state.user_repo.count_users() {
        Ok(c) => c,
        Err(e) => return Error::Internal(format!("DB error: {}", e)).into_response(),
    };

    let is_first_user = user_count == 0;
    let password_hash = hash_password(&body.password, &auth_state.jwt_secret);

    let user_id = match auth_state.user_repo.create_user(
        &body.username,
        &password_hash,
        body.display_name.as_deref(),
        body.email.as_deref(),
    ) {
        Ok(id) => id,
        Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
            return Error::InvalidRequest("Username already exists".to_string()).into_response();
        }
        Err(e) => return Error::Internal(format!("DB error: {}", e)).into_response(),
    };

    if is_first_user {
        let _ = auth_state.user_repo.update_user(user_id, None, None, Some(true), Some("active"), None);
    }

    let token = generate_token(user_id, &auth_state.jwt_secret);
    let expires_at = Utc::now() + Duration::days(7);

    let _ = auth_state.user_repo.create_session(
        &token,
        user_id,
        &expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    );

    let response = LoginResponse {
        token: token.clone(),
        user: UserSummary {
            id: user_id,
            username: body.username.clone(),
            display_name: body.display_name.clone(),
            email: body.email.clone(),
            avatar_url: None,
            is_admin: is_first_user,
        },
    };

    let cookie = format!(
        "m2c_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        token,
        7 * 24 * 3600
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

    (StatusCode::CREATED, headers, Json(response)).into_response()
}

/// POST /admin/api/auth/logout
pub async fn logout(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    if let Some(cookie_header) = headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix("m2c_session=") {
                    let _ = auth_state.user_repo.delete_session(token);
                }
            }
        }
    }

    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let _ = auth_state.user_repo.delete_session(token);
            }
        }
    }

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        header::SET_COOKIE,
        "m2c_session=; Path=/; HttpOnly; Max-Age=0".parse().unwrap(),
    );

    (StatusCode::OK, resp_headers, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// GET /admin/api/me
pub async fn get_me(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    let user = match auth_state.user_repo.find_by_id(user_id) {
        Ok(Some(u)) => u,
        Ok(None) => return Error::Auth("User not found".to_string()).into_response(),
        Err(e) => return Error::Internal(format!("DB error: {}", e)).into_response(),
    };

    Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "displayName": user.display_name,
        "email": user.email,
        "avatarUrl": user.avatar_url,
        "isAdmin": user.is_admin,
        "status": user.status,
    })).into_response()
}

/// GET /admin/api/me/api-keys
pub async fn list_api_keys(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match auth_state.user_repo.list_api_keys(user_id) {
        Ok(keys) => Json(serde_json::json!({
            "keys": keys.iter().map(|k| serde_json::json!({
                "id": k.id,
                "name": k.name,
                "keyPrefix": k.key_prefix,
                "scopes": k.scopes,
                "createdAt": k.created_at,
                "lastUsedAt": k.last_used_at,
                "isRevoked": k.is_revoked,
            })).collect::<Vec<_>>()
        })).into_response(),
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

/// POST /admin/api/me/api-keys
pub async fn create_api_key(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
    Json(body): Json<CreateApiKeyRequest>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    if body.name.trim().is_empty() {
        return Error::InvalidRequest("Key name is required".to_string()).into_response();
    }

    let (key, prefix, hash) = generate_api_key();

    match auth_state.user_repo.create_api_key(user_id, &body.name, &prefix, &hash) {
        Ok(id) => {
            let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
            (StatusCode::CREATED, Json(serde_json::json!({
                "id": id,
                "name": body.name,
                "key": key,
                "keyPrefix": prefix,
                "createdAt": now,
            }))).into_response()
        }
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

/// DELETE /admin/api/me/api-keys/:id
pub async fn revoke_api_key(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
    Path(key_id): Path<i64>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match auth_state.user_repo.revoke_api_key(key_id, user_id) {
        Ok(true) => Json(serde_json::json!({"status": "ok"})).into_response(),
        Ok(false) => Error::NotFound("API key not found".to_string()).into_response(),
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

// ─── Upstream Key handlers ───────────────────────────────────────────────

/// Upstream key set/update request
#[derive(Debug, Deserialize)]
pub struct SetUpstreamKeyRequest {
    pub api_key: String,
}

/// GET /admin/api/me/upstream-keys - List upstream keys
pub async fn list_upstream_keys(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match auth_state.user_repo.list_upstream_keys(user_id) {
        Ok(keys) => Json(serde_json::json!({ "upstream_keys": keys })).into_response(),
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

/// PUT /admin/api/me/upstream-keys/:providerId - Set upstream key
pub async fn set_upstream_key(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
    Path(provider_id): Path<String>,
    Json(body): Json<SetUpstreamKeyRequest>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    if body.api_key.trim().is_empty() {
        return Error::InvalidRequest("api_key is required".to_string()).into_response();
    }

    // Simple encryption (use real encrypt in production via masterKey)
    let encrypted = format!("enc:{}", body.api_key.trim());

    match auth_state.user_repo.set_upstream_key(user_id, &provider_id, &encrypted) {
        Ok(()) => Json(serde_json::json!({"ok": true, "provider_id": provider_id})).into_response(),
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

/// DELETE /admin/api/me/upstream-keys/:providerId - Delete upstream key
pub async fn delete_upstream_key(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
    Path(provider_id): Path<String>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match auth_state.user_repo.delete_upstream_key(user_id, &provider_id) {
        Ok(true) => Json(serde_json::json!({"deleted": true})).into_response(),
        Ok(false) => Json(serde_json::json!({"deleted": false})).into_response(),
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

// ─── OAuth Client handlers ──────────────────────────────────────────────

/// OAuth client upsert request
#[derive(Debug, Deserialize)]
pub struct UpsertOAuthClientRequest {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    pub callback_url: String,
    #[serde(default)]
    pub enabled: bool,
}

/// GET /admin/api/oauth-clients - List OAuth clients (admin-only)
pub async fn list_oauth_clients(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");

    // Verify user is admin
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    let user = match auth_state.user_repo.find_by_id(user_id) {
        Ok(Some(u)) => u,
        _ => return Error::Auth("User not found".to_string()).into_response(),
    };
    if !user.is_admin {
        return Error::Auth("Admin access required".to_string()).into_response();
    }

    match auth_state.user_repo.list_oauth_clients() {
        Ok(clients) => Json(serde_json::json!({ "clients": clients })).into_response(),
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

/// PUT /admin/api/oauth-clients/:provider - Upsert OAuth client
pub async fn upsert_oauth_client(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
    Path(provider): Path<String>,
    Json(body): Json<UpsertOAuthClientRequest>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");

    // Verify user is admin
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    let user = match auth_state.user_repo.find_by_id(user_id) {
        Ok(Some(u)) => u,
        _ => return Error::Auth("User not found".to_string()).into_response(),
    };
    if !user.is_admin {
        return Error::Auth("Admin access required".to_string()).into_response();
    }

    if body.client_id.is_empty() || body.callback_url.is_empty() {
        return Error::InvalidRequest("clientId and callbackUrl are required".to_string()).into_response();
    }

    // Validate provider
    if provider != "github" && provider != "gitee" {
        return Error::InvalidRequest("provider must be 'github' or 'gitee'".to_string()).into_response();
    }

    match auth_state.user_repo.upsert_oauth_client(
        &provider,
        &body.client_id,
        body.client_secret.as_deref().filter(|s| !s.is_empty()),
        &body.callback_url,
        body.enabled,
    ) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

/// DELETE /admin/api/oauth-clients/:provider - Delete OAuth client
pub async fn delete_oauth_client(
    headers: HeaderMap,
    State(state): State<Arc<crate::handlers::AppState>>,
    Path(provider): Path<String>,
) -> Response {
    let auth_state = state.auth_state.as_ref().expect("AuthState not configured");

    // Verify user is admin
    let user_id = match extract_user_from_request(&headers, auth_state) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    let user = match auth_state.user_repo.find_by_id(user_id) {
        Ok(Some(u)) => u,
        _ => return Error::Auth("User not found".to_string()).into_response(),
    };
    if !user.is_admin {
        return Error::Auth("Admin access required".to_string()).into_response();
    }

    match auth_state.user_repo.delete_oauth_client(&provider) {
        Ok(deleted) => Json(serde_json::json!({ "deleted": deleted })).into_response(),
        Err(e) => Error::Internal(format!("DB error: {}", e)).into_response(),
    }
}

/// GET /admin/api/auth/oauth-providers - Public list of enabled OAuth providers
pub async fn get_oauth_providers(
    State(state): State<Arc<crate::handlers::AppState>>,
) -> Response {
    let auth_state = match state.auth_state.as_ref() {
        Some(auth) => auth,
        None => return Json(serde_json::json!({ "providers": [] })).into_response(),
    };

    match auth_state.user_repo.list_oauth_clients() {
        Ok(clients) => {
            let providers: Vec<serde_json::Value> = clients
                .into_iter()
                .filter(|c| c.enabled)
                .map(|c| serde_json::json!({
                    "provider": c.provider,
                    "callback_url": c.callback_url,
                }))
                .collect();
            Json(serde_json::json!({ "providers": providers })).into_response()
        }
        Err(_) => Json(serde_json::json!({ "providers": [] })).into_response(),
    }
}
