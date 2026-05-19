use axum::{
   Json,
   http::StatusCode,
   response::IntoResponse,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
   #[error("configuration error: {0}")]
   Config(String),
   #[error("I/O error: {0}")]
   Io(#[from] std::io::Error),
   #[error("HTTP error: {0}")]
   Http(#[from] reqwest::Error),
   #[error("invidious not configured")]
   InvidiousNotConfigured,
   #[error("invidious authentication failed")]
   InvidiousAuthFailed,
   #[error("invidious unreachable")]
   InvidiousUnreachable,
   #[error("invidious bad response")]
   InvidiousBadResponse,
   #[error("invidious rate limited")]
   InvidiousRateLimited,
   #[error("serialization error: {0}")]
   Serialization(String),
}

impl From<toml::ser::Error> for AppError {
   fn from(err: toml::ser::Error) -> Self {
      Self::Serialization(err.to_string())
   }
}

impl From<toml::de::Error> for AppError {
   fn from(err: toml::de::Error) -> Self {
      Self::Serialization(err.to_string())
   }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
   error: String,
}

impl IntoResponse for AppError {
   fn into_response(self) -> axum::response::Response {
      let status = match &self {
         Self::Config(_) | Self::Io(_) | Self::Serialization(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
         }
         Self::Http(_) | Self::InvidiousUnreachable | Self::InvidiousBadResponse => {
            StatusCode::BAD_GATEWAY
         }
         Self::InvidiousNotConfigured => StatusCode::SERVICE_UNAVAILABLE,
         Self::InvidiousAuthFailed => StatusCode::UNAUTHORIZED,
         Self::InvidiousRateLimited => StatusCode::TOO_MANY_REQUESTS,
      };

      let body = Json(ErrorBody {
         error: self.to_string(),
      });

      (status, body).into_response()
   }
}
