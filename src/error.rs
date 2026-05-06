use axum::{Json, http::StatusCode, response::IntoResponse};
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
    #[error("invalid youtube url")]
    InvalidYouTubeUrl,
    #[error("resolve failed")]
    ResolveFailed,
    #[error("no compatible format")]
    NoCompatibleFormat,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Http(_) => StatusCode::BAD_GATEWAY,
            AppError::InvidiousNotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            AppError::InvidiousAuthFailed => StatusCode::UNAUTHORIZED,
            AppError::InvidiousUnreachable => StatusCode::BAD_GATEWAY,
            AppError::InvidiousBadResponse => StatusCode::BAD_GATEWAY,
            AppError::InvidiousRateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::InvalidYouTubeUrl => StatusCode::BAD_REQUEST,
            AppError::ResolveFailed => StatusCode::BAD_GATEWAY,
            AppError::NoCompatibleFormat => StatusCode::NOT_ACCEPTABLE,
        };

        let body = Json(ErrorBody {
            error: self.to_string(),
        });

        (status, body).into_response()
    }
}
