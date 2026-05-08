use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Minimal shared error helper for route modules.
/// Returns a JSON response with the shape: `{ "error": message }`
pub fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
