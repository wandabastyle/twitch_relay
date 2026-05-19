use axum::{
   Json,
   http::{
      HeaderValue,
      StatusCode,
      header,
   },
   response::{
      IntoResponse,
      Response,
   },
};
use serde::Serialize;

/// Standard API error response body with `{ "error": "message" }` shape.
#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
   pub error: String,
}

/// Create a JSON error response with the standard `{ "error": message }` shape.
///
/// This is the shared helper for consistent API error responses across route
/// modules. Supports optional `Retry-After` header for rate-limiting responses.
pub fn error_response(status: StatusCode, message: &str, retry_after: Option<u64>) -> Response {
   let mut response = (
      status,
      Json(ApiErrorBody {
         error: message.to_string(),
      }),
   )
      .into_response();

   if let Some(seconds) = retry_after
      && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
   {
      response.headers_mut().insert(header::RETRY_AFTER, value);
   }

   response
}

#[cfg(test)]
mod tests {
   use super::*;

   #[tokio::test]
   async fn error_response_produces_json_shape() {
      let response: Response = error_response(StatusCode::BAD_REQUEST, "test error message", None);

      assert_eq!(response.status(), StatusCode::BAD_REQUEST);

      let body_bytes = axum::body::to_bytes(response.into_body(), 1024)
         .await
         .expect("read body");
      let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse json");

      assert_eq!(json["error"], "test error message");
   }
}
