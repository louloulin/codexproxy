use axum::{extract::State, Json};
use crate::config::Config;

pub async fn chat_completions(
    State(_config): State<Config>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, crate::error::Error> {
    // TODO: Implement chat completions handler
    Ok(Json(serde_json::json!({
        "message": "Chat completions endpoint - Phase 1 implementation"
    })))
}

pub async fn responses(
    State(_config): State<Config>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, crate::error::Error> {
    // TODO: Implement responses handler
    Ok(Json(serde_json::json!({
        "message": "Responses endpoint - Phase 1 implementation"
    })))
}
