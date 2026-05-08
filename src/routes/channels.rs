use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use serde::Deserialize;

use crate::{
    auth::{self, WebAuthConfig},
    channel_catalog::ChannelCatalogService,
    channels,
    live_status::{LiveStatusResponse, LiveStatusService},
    routes::error::error_response,
};

#[derive(Debug, Deserialize)]
pub struct AddChannelRequest {
    pub login: String,
}

#[derive(Debug, Clone)]
pub struct ChannelState {
    pub live_status: LiveStatusService,
}

#[derive(Debug, Clone)]
pub struct LiveStatusState {
    pub service: LiveStatusService,
    pub catalog: ChannelCatalogService,
}

pub fn channel_routes(state: ChannelState, auth_config: WebAuthConfig) -> Router {
    Router::new()
        .route("/api/channels", post(add_channel))
        .route("/api/channels/{login}", delete(remove_channel))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            auth_config,
            auth::require_session_middleware,
        ))
}

pub fn live_status_routes(state: LiveStatusState, auth_config: WebAuthConfig) -> Router {
    Router::new()
        .route("/api/live-status", get(get_live_status))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            auth_config,
            auth::require_session_middleware,
        ))
}

async fn get_live_status(State(state): State<LiveStatusState>) -> Json<LiveStatusResponse> {
    let channels = state.catalog.channel_logins().await;
    let response = state.service.check_multiple(&channels).await;
    Json(response)
}

async fn add_channel(
    State(state): State<ChannelState>,
    Json(payload): Json<AddChannelRequest>,
) -> Response {
    let normalized = payload.login.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "channel login cannot be empty");
    }

    if channels::channel_exists(&normalized) {
        return error_response(StatusCode::CONFLICT, "channel already exists");
    }

    match channels::add_channel(normalized.clone()) {
        Ok(_channel) => {
            let _ = state.live_status.fetch_profile_image(&normalized).await;
            (
                StatusCode::CREATED,
                Json(serde_json::json!({ "login": normalized })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to add channel to storage");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to add channel")
        }
    }
}

async fn remove_channel(State(_state): State<ChannelState>, Path(login): Path<String>) -> Response {
    let normalized = login.trim().to_ascii_lowercase();

    match channels::remove_channel(&normalized) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            if e.contains("not found") {
                return error_response(StatusCode::NOT_FOUND, "channel not found");
            }
            tracing::error!(error = %e, "failed to remove channel");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to remove channel",
            )
        }
    }
}
