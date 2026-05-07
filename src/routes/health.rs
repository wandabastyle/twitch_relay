use axum::{Json, Router, routing::get};
use serde::Serialize;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize)]
pub struct ProbeResponse<'a> {
    pub status: &'a str,
    pub service: &'a str,
}

#[derive(Debug, Serialize)]
pub struct VersionResponse<'a> {
    pub version: &'a str,
}

pub fn health_routes() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/version", get(get_version))
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

async fn get_version() -> Json<VersionResponse<'static>> {
    Json(VersionResponse {
        version: APP_VERSION,
    })
}
