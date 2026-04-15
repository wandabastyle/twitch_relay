use axum::{
    Json, Router, middleware,
    routing::{get, post},
};
use serde::Serialize;

use crate::{
    auth::{self, WebAuthConfig},
    config::AppConfig,
    error::AppError,
};

pub fn build_router(config: &AppConfig) -> Result<Router, AppError> {
    let auth_config = WebAuthConfig::from_app_config(config)?;

    let protected_routes = Router::new()
        .route("/api/channels", get(list_channels))
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth::require_session_middleware,
        ));

    let auth_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/session", get(auth::session_status))
        .with_state(auth_config);

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .merge(auth_routes)
        .merge(protected_routes);

    Ok(router)
}

#[derive(Debug, Serialize)]
struct ProbeResponse<'a> {
    status: &'a str,
    service: &'a str,
}

#[derive(Debug, Serialize)]
struct ChannelsResponse {
    channels: Vec<ChannelSummary>,
}

#[derive(Debug, Serialize)]
struct ChannelSummary {
    login: &'static str,
    is_live: bool,
}

async fn healthz() -> Json<ProbeResponse<'static>> {
    Json(ProbeResponse {
        status: "ok",
        service: "twitch-relay",
    })
}

async fn readyz() -> Json<ProbeResponse<'static>> {
    Json(ProbeResponse {
        status: "ready",
        service: "twitch-relay",
    })
}

async fn list_channels() -> Json<ChannelsResponse> {
    Json(ChannelsResponse {
        channels: vec![ChannelSummary {
            login: "demo_channel",
            is_live: false,
        }],
    })
}
