use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{self, WebAuthConfig},
    config::AppConfig,
    error::AppError,
    playback::{WatchTicketError, WatchTicketService},
};

pub fn build_router(config: &AppConfig) -> Result<Router, AppError> {
    let auth_config = WebAuthConfig::from_app_config(config)?;
    let playback = WatchTicketService::new(
        config.playback.channels.clone(),
        config.playback.watch_ticket_ttl_secs,
    );

    let protected_state = ProtectedState {
        auth: auth_config.clone(),
        playback,
    };

    let protected_routes = Router::new()
        .route("/api/channels", get(list_channels))
        .route("/api/watch-ticket", post(create_watch_ticket))
        .route("/watch/{ticket}", get(render_watch_page))
        .with_state(protected_state)
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
    channels: Vec<ChannelItem>,
}

#[derive(Debug, Serialize)]
struct ChannelItem {
    login: String,
}

#[derive(Debug, Deserialize)]
struct WatchTicketRequest {
    channel_login: String,
}

#[derive(Debug, Serialize)]
struct WatchTicketResponse {
    watch_url: String,
    expires_at_unix: u64,
}

#[derive(Debug, Clone)]
struct ProtectedState {
    auth: WebAuthConfig,
    playback: WatchTicketService,
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

async fn list_channels(State(state): State<ProtectedState>) -> Json<ChannelsResponse> {
    let channels = state
        .playback
        .channel_list()
        .into_iter()
        .map(|login| ChannelItem { login })
        .collect::<Vec<_>>();

    Json(ChannelsResponse { channels })
}

async fn create_watch_ticket(
    State(state): State<ProtectedState>,
    headers: HeaderMap,
    Json(payload): Json<WatchTicketRequest>,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    match state
        .playback
        .issue_ticket(&session_token, &payload.channel_login)
    {
        Ok((ticket, expires_at_unix)) => {
            let response = WatchTicketResponse {
                watch_url: format!("/watch/{ticket}"),
                expires_at_unix,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(WatchTicketError::UnknownChannel) => {
            error_response(StatusCode::BAD_REQUEST, "channel is not in allowlist")
        }
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to issue watch ticket",
        ),
    }
}

async fn render_watch_page(
    State(state): State<ProtectedState>,
    headers: HeaderMap,
    Path(ticket): Path<String>,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    let validated = match state.playback.validate_ticket(&ticket, &session_token) {
        Ok(validated) => validated,
        Err(WatchTicketError::InvalidTicket) | Err(WatchTicketError::ExpiredTicket) => {
            return error_response(StatusCode::UNAUTHORIZED, "invalid or expired watch ticket");
        }
        Err(WatchTicketError::SessionMismatch) => {
            return error_response(
                StatusCode::FORBIDDEN,
                "watch ticket belongs to a different session",
            );
        }
        Err(WatchTicketError::UnknownChannel) => {
            return error_response(StatusCode::BAD_REQUEST, "channel is not in allowlist");
        }
    };

    let parent = parent_domain(&headers).unwrap_or_else(|| "localhost".to_string());
    let embed_src = format!(
        "https://player.twitch.tv/?channel={}&parent={}&autoplay=true&muted=false",
        validated.channel_login, parent
    );

    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Watch {channel}</title><style>body{{margin:0;background:#0b0f14;color:#f3f6fa;font-family:system-ui,-apple-system,Segoe UI,sans-serif}}main{{display:grid;grid-template-rows:auto 1fr;min-height:100vh}}header{{padding:0.75rem 1rem;border-bottom:1px solid #2a3442}}iframe{{width:100%;height:calc(100vh - 3.2rem);border:0;background:#000}}small{{color:#93a1b5}}</style></head><body><main><header><strong>{channel}</strong> <small>ticket expires: {expires}</small></header><iframe src=\"{src}\" allowfullscreen></iframe></main></body></html>",
        channel = validated.channel_login,
        expires = validated.expires_at_unix,
        src = embed_src
    );

    Html(html).into_response()
}

fn parent_domain(headers: &HeaderMap) -> Option<String> {
    let forwarded_host = headers
        .get("x-forwarded-host")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let host = forwarded_host.or_else(|| {
        headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })?;

    host.split(':').next().map(ToString::to_string)
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
