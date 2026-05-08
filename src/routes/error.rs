use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Standard API error response body with `{ "error": "message" }` shape.
#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: String,
}

/// Create a JSON error response with the standard `{ "error": message }` shape.
///
/// This is the shared helper for consistent API error responses across route modules.
/// For rate-limiting with `Retry-After` headers, use local error handling in `auth.rs`.
pub fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(ApiErrorBody {
            error: message.to_string(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn error_response_produces_json_shape() {
        let response: Response = error_response(StatusCode::BAD_REQUEST, "test error message");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse json");

        assert_eq!(json["error"], "test error message");
    }
}
