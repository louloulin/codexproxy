use crate::error::Error;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Request(e) => (StatusCode::BAD_REQUEST, &e.to_string()),
            Error::Provider(msg) => (StatusCode::BAD_GATEWAY, msg),
            Error::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            Error::Json(e) => (StatusCode::BAD_REQUEST, &e.to_string()),
            Error::Yaml(e) => (StatusCode::BAD_REQUEST, &e.to_string()),
        };

        let body = json!({
            "error": {
                "message": message,
                "type": "invalid_request_error",
                "code": status.as_u16()
            }
        });

        (status, Json(body)).into_response()
    }
}
