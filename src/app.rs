use axum::{Json, Router, routing::get};
use serde::Serialize;

pub fn build_router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}

#[derive(Debug, Serialize)]
struct ProbeResponse<'a> {
    status: &'a str,
    service: &'a str,
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
