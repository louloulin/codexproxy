//! Admin user management handlers
//!
//! Provides CRUD operations for user management via Admin API.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::user_repository::UserWithStats;
use crate::handlers::AppState;

/// Admin context extracted from request
#[derive(Clone)]
pub struct AdminContext {
    pub user_id: Option<i64>,
    pub is_admin: bool,
}

/// List users response
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersResponse {
    pub users: Vec<UserWithStatsResponse>,
}

/// User response with stats
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserWithStatsResponse {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub request_count: i64,
    pub total_tokens: i64,
    pub last_activity: Option<String>,
}

impl From<UserWithStats> for UserWithStatsResponse {
    fn from(u: UserWithStats) -> Self {
        Self {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            avatar_url: u.avatar_url,
            is_admin: u.is_admin,
            status: u.status,
            created_at: u.created_at,
            updated_at: u.updated_at,
            request_count: u.request_count,
            total_tokens: u.total_tokens,
            last_activity: u.last_activity,
        }
    }
}

/// Create user request
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub is_admin: Option<bool>,
}

/// Create user response
#[derive(Serialize)]
pub struct CreateUserResponse {
    pub user: UserWithStatsResponse,
}

/// Update user request
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub is_admin: Option<bool>,
    pub status: Option<String>,
    pub password: Option<String>,
}

/// Update user response
#[derive(Serialize)]
pub struct UpdateUserResponse {
    pub user: UserWithStatsResponse,
}

/// Error response
#[derive(Serialize)]
pub struct AdminError {
    pub error: String,
    pub message: String,
}

impl AdminError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            error: code.to_string(),
            message: message.to_string(),
        }
    }
}

fn error_response(status: StatusCode, code: &str, message: &str) -> impl IntoResponse {
    (status, Json(AdminError::new(code, message)))
}

// Helper function to hash password
fn hash_password(password: &str, secret: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    format!("{}:{}", password, secret).hash(&mut hasher);
    format!("scrypt${:x}", hasher.finish())
}

/// Extract admin context from request headers
fn extract_admin_context(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Option<AdminContext> {
    let auth_state = state.auth_state.as_ref()?;

    // Check Authorization header (Bearer token)
    if let Some(auth_header) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Ok(Some(session)) = auth_state.user_repo.find_session(token) {
                    if let Ok(Some(user)) = auth_state.user_repo.find_by_id(session.user_id) {
                        return Some(AdminContext {
                            user_id: Some(user.id),
                            is_admin: user.is_admin,
                        });
                    }
                }
            }
        }
    }

    // Check cookie
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix("m2c_session=") {
                    if let Ok(Some(session)) = auth_state.user_repo.find_session(token) {
                        if let Ok(Some(user)) = auth_state.user_repo.find_by_id(session.user_id) {
                            return Some(AdminContext {
                                user_id: Some(user.id),
                                is_admin: user.is_admin,
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

/// GET /admin/api/users - List all users with stats
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Check auth state exists
    let auth_state = state.auth_state.as_ref().ok_or_else(|| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "auth_not_configured",
            "Authentication not configured",
        )
    })?;

    // Extract admin context
    let admin = extract_admin_context(&state, &headers).ok_or_else(|| {
        error_response(StatusCode::UNAUTHORIZED, "unauthorized", "Authentication required")
    })?;

    // Check admin permission
    if !admin.is_admin {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Admin access required",
        ));
    }

    let users = auth_state
        .user_repo
        .list_users_with_stats()
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &format!("Failed to list users: {}", e),
            )
        })?;

    let response = ListUsersResponse {
        users: users.into_iter().map(UserWithStatsResponse::from).collect(),
    };

    Ok(Json(response))
}

/// POST /admin/api/users - Create a new user
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, impl IntoResponse), impl IntoResponse> {
    // Check auth state exists
    let auth_state = state.auth_state.as_ref().ok_or_else(|| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "auth_not_configured",
            "Authentication not configured",
        )
    })?;

    // Extract admin context
    let admin = extract_admin_context(&state, &headers).ok_or_else(|| {
        error_response(StatusCode::UNAUTHORIZED, "unauthorized", "Authentication required")
    })?;

    // Check admin permission
    if !admin.is_admin {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Admin access required",
        ));
    }

    // Validate request
    if req.username.trim().is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "invalid_username",
            "Username is required",
        ));
    }

    if req.password.len() < 8 {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "weak_password",
            "Password must be at least 8 characters",
        ));
    }

    // Check if username already exists
    let existing = auth_state.user_repo.find_by_username(&req.username);
    match existing {
        Ok(Some(_)) => {
            return Err(error_response(
                StatusCode::CONFLICT,
                "username_taken",
                "Username is already in use",
            ));
        }
        Ok(None) => {}
        Err(e) => {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &format!("Database error: {}", e),
            ));
        }
    }

    // Hash password
    let password_hash = hash_password(&req.password, &auth_state.jwt_secret);

    // Create user
    let user_id = auth_state
        .user_repo
        .create_user(
            &req.username,
            &password_hash,
            req.display_name.as_deref(),
            req.email.as_deref(),
        )
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &format!("Failed to create user: {}", e),
            )
        })?;

    // Update is_admin if specified
    if req.is_admin == Some(true) {
        let _ = auth_state.user_repo.update_user(
            user_id,
            None,
            None,
            Some(true),
            None,
            None,
        );
    }

    // Get the created user with stats
    let user = auth_state.user_repo.get_user_with_stats(user_id).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("Failed to fetch created user: {}", e),
        )
    })?;

    match user {
        Some(u) => {
            let response = CreateUserResponse {
                user: UserWithStatsResponse::from(u),
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        None => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Failed to fetch created user",
        )),
    }
}

/// PATCH /admin/api/users/:id - Update a user
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(user_id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Check auth state exists
    let auth_state = state.auth_state.as_ref().ok_or_else(|| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "auth_not_configured",
            "Authentication not configured",
        )
    })?;

    // Extract admin context
    let admin = extract_admin_context(&state, &headers).ok_or_else(|| {
        error_response(StatusCode::UNAUTHORIZED, "unauthorized", "Authentication required")
    })?;

    // Check admin permission
    if !admin.is_admin {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Admin access required",
        ));
    }

    // Validate password if provided
    if let Some(ref pwd) = req.password {
        if pwd.len() < 8 {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "weak_password",
                "Password must be at least 8 characters",
            ));
        }
    }

    // Validate status if provided
    if let Some(ref status) = req.status {
        if status != "active" && status != "disabled" {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "invalid_status",
                "Status must be 'active' or 'disabled'",
            ));
        }
    }

    // Hash password if provided
    let password_hash = if let Some(ref pwd) = req.password {
        Some(hash_password(pwd, &auth_state.jwt_secret))
    } else {
        None
    };

    // Update user
    let updated = auth_state
        .user_repo
        .update_user(
            user_id,
            req.display_name.as_deref(),
            req.email.as_deref(),
            req.is_admin,
            req.status.as_deref(),
            password_hash.as_deref(),
        )
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &format!("Failed to update user: {}", e),
            )
        })?;

    if !updated {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            &format!("User with id {} not found", user_id),
        ));
    }

    // Get the updated user with stats
    let user = auth_state.user_repo.get_user_with_stats(user_id).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("Failed to fetch updated user: {}", e),
        )
    })?;

    match user {
        Some(u) => {
            let response = UpdateUserResponse {
                user: UserWithStatsResponse::from(u),
            };
            Ok(Json(response))
        }
        None => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Failed to fetch updated user",
        )),
    }
}

/// DELETE /admin/api/users/:id - Delete a user
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(user_id): Path<i64>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Check auth state exists
    let auth_state = state.auth_state.as_ref().ok_or_else(|| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "auth_not_configured",
            "Authentication not configured",
        )
    })?;

    // Extract admin context
    let admin = extract_admin_context(&state, &headers).ok_or_else(|| {
        error_response(StatusCode::UNAUTHORIZED, "unauthorized", "Authentication required")
    })?;

    // Check admin permission
    if !admin.is_admin {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Admin access required",
        ));
    }

    // Prevent deleting yourself
    if admin.user_id == Some(user_id) {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "self_delete",
            "Cannot delete your own account",
        ));
    }

    let deleted = auth_state.user_repo.delete_user(user_id).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("Failed to delete user: {}", e),
        )
    })?;

    if !deleted {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            &format!("User with id {} not found", user_id),
        ));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_response_from_user_with_stats() {
        let user = UserWithStats {
            id: 1,
            username: "test".to_string(),
            display_name: Some("Test User".to_string()),
            email: Some("test@example.com".to_string()),
            avatar_url: None,
            is_admin: true,
            status: "active".to_string(),
            created_at: "2024-01-01 00:00:00".to_string(),
            updated_at: "2024-01-01 00:00:00".to_string(),
            request_count: 100,
            total_tokens: 50000,
            last_activity: Some("2024-01-02 00:00:00".to_string()),
        };

        let response = UserWithStatsResponse::from(user.clone());
        assert_eq!(response.id, 1);
        assert_eq!(response.username, "test");
        assert_eq!(response.display_name, Some("Test User".to_string()));
        assert!(response.is_admin);
        assert_eq!(response.request_count, 100);
    }

    #[test]
    fn test_admin_error() {
        let err = AdminError::new("test_error", "Test error message");
        assert_eq!(err.error, "test_error");
        assert_eq!(err.message, "Test error message");
    }
}
