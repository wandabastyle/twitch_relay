use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    auth::{self, WebAuthConfig},
    channel_catalog::{CatalogChannel, ChannelCatalogService},
    channels, chat,
    config::AppConfig,
    error::AppError,
    live_status::{LiveStatusResponse, LiveStatusService},
    playback::{PlaybackTicketError, PlaybackTicketService},
    prewarm::PrewarmCoordinator,
    stream_proxy, twitch_auth,
};

pub fn build_router(config: &AppConfig, access_code_hash: String) -> Result<Router, AppError> {
    let auth_config = WebAuthConfig::new(
        access_code_hash,
        config.auth.cookie_name.clone(),
        config.auth.cookie_secure,
    );

    let twitch_auth_service = twitch_auth::TwitchAuthService::new(config.twitch_oauth.clone())?;
    let catalog_service = ChannelCatalogService::new(twitch_auth_service.clone());
    let playback = PlaybackTicketService::new(config.playback.watch_ticket_ttl_secs);
    let streamlink_path = config
        .playback
        .streamlink_path
        .clone()
        .unwrap_or_else(|| "streamlink".to_string());

    let stream_service = stream_proxy::StreamSessionService::new(
        streamlink_path.clone(),
        config.playback.stream_resolver_mode.clone(),
        config.playback.stream_delivery_mode.clone(),
        config.playback.twitch_client_id.clone(),
    );

    let protected_state = ProtectedState {
        auth: auth_config.clone(),
        playback: playback.clone(),
        stream: stream_service.clone(),
        catalog: catalog_service.clone(),
    };

    let live_status_service = LiveStatusService::new();
    let channel_state = ChannelState {
        live_status: live_status_service.clone(),
    };

    let live_status_state = LiveStatusState {
        service: live_status_service,
        catalog: catalog_service.clone(),
    };

    let chat_service = chat::ChatService::new(twitch_auth_service.clone());
    let chat_state = chat::ChatState {
        service: chat_service,
    };

    let prewarm = PrewarmCoordinator::new(
        catalog_service.clone(),
        live_status_state.service.clone(),
        chat_state.service.clone(),
    );
    prewarm.trigger_now();

    let twitch_state = twitch_auth::TwitchAuthState {
        auth: auth_config.clone(),
        twitch: twitch_auth_service,
        prewarm: Some(prewarm.clone()),
    };

    let stream_proxy_state = stream_proxy::StreamProxyState::new(stream_service.clone());

    let channel_routes = Router::new()
        .route("/api/channels", post(add_channel))
        .route("/api/channels/{login}", delete(remove_channel))
        .with_state(channel_state)
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth::require_session_middleware,
        ));

    let live_status_routes = Router::new()
        .route("/api/live-status", get(get_live_status))
        .with_state(live_status_state)
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth::require_session_middleware,
        ));

    let protected_routes = Router::new()
        .route("/api/channels", get(list_channels))
        .route("/api/watch-ticket", post(create_watch_ticket))
        .route("/api/quality-switch", get(quality_switch_handler))
        .route("/watch/{ticket}", get(render_watch_page))
        .with_state(protected_state)
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth::require_session_middleware,
        ));

    let twitch_routes = Router::new()
        .route("/api/twitch/status", get(twitch_auth::get_status))
        .route("/api/twitch/connect", get(twitch_auth::connect))
        .route("/api/twitch/callback", get(twitch_auth::callback))
        .route("/api/twitch/disconnect", post(twitch_auth::disconnect))
        .with_state(twitch_state)
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth::require_session_middleware,
        ));

    let chat_routes = Router::new()
        .route("/api/chat/status", get(chat::status))
        .route("/api/chat/emotes", get(chat::emotes))
        .route("/api/chat/subscribe", post(chat::subscribe))
        .route("/api/chat/subscribe/{login}", delete(chat::unsubscribe))
        .route("/api/chat/events/{login}", get(chat::events))
        .route("/api/chat/send", post(chat::send))
        .with_state(chat_state)
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth::require_session_middleware,
        ));

    let stream_routes = Router::new()
        .route(
            "/stream/{stream_id}/{session_token}/manifest",
            get(stream_proxy::proxy_manifest),
        )
        .route(
            "/stream/{stream_id}/{session_token}/manifest/{quality}",
            get(stream_proxy::proxy_variant_manifest),
        )
        .route(
            "/stream/{stream_id}/{session_token}/{quality}/{*segment}",
            get(stream_proxy::proxy_segment),
        )
        .with_state(stream_proxy_state);

    let auth_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/session", get(auth::session_status))
        .with_state(auth_config);

    let base_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let static_path = base_path.join("web").join("build");
    let assets_path = base_path.join("web").join("static");

    let images_path = channels::images_dir().unwrap_or_else(|| PathBuf::from("/tmp"));

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .merge(auth_routes)
        .merge(channel_routes)
        .merge(live_status_routes)
        .merge(protected_routes)
        .merge(twitch_routes)
        .merge(chat_routes)
        .merge(stream_routes)
        .nest_service("/static/images", ServeDir::new(&images_path))
        .nest_service("/static", ServeDir::new(&assets_path))
        .fallback_service(
            ServeDir::new(&static_path)
                .not_found_service(ServeFile::new(static_path.join("index.html"))),
        );

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
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    source: String,
    removable: bool,
}

#[derive(Debug, Deserialize)]
struct WatchTicketRequest {
    channel_login: String,
}

#[derive(Debug, Serialize)]
struct WatchTicketResponse {
    watch_url: String,
}

#[derive(Debug, Clone)]
struct ProtectedState {
    auth: WebAuthConfig,
    playback: PlaybackTicketService,
    stream: stream_proxy::StreamSessionService,
    catalog: ChannelCatalogService,
}

#[derive(Debug, Clone)]
struct ChannelState {
    live_status: LiveStatusService,
}

#[derive(Debug, Clone)]
struct LiveStatusState {
    service: LiveStatusService,
    catalog: ChannelCatalogService,
}

#[derive(Debug, Deserialize)]
struct AddChannelRequest {
    login: String,
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
    let mut channels_list: Vec<ChannelItem> = state
        .catalog
        .list_channels()
        .await
        .into_iter()
        .map(channel_item_from_catalog)
        .collect();

    channels_list.sort_by_key(|c| c.login.to_lowercase());

    Json(ChannelsResponse {
        channels: channels_list,
    })
}

fn channel_item_from_catalog(item: CatalogChannel) -> ChannelItem {
    let source = match item.source {
        crate::channel_catalog::ChannelSource::Manual => "manual",
        crate::channel_catalog::ChannelSource::Followed => "followed",
        crate::channel_catalog::ChannelSource::Both => "both",
    };

    ChannelItem {
        login: item.login,
        image_url: item.image_url,
        display_name: item.display_name,
        source: source.to_string(),
        removable: item.removable,
    }
}

async fn create_watch_ticket(
    State(state): State<ProtectedState>,
    headers: HeaderMap,
    Json(payload): Json<WatchTicketRequest>,
) -> Response {
    if !state.catalog.has_channel(&payload.channel_login).await {
        return error_response(StatusCode::BAD_REQUEST, "channel is not in channel list");
    }

    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    match state
        .playback
        .issue_ticket(&session_token, &payload.channel_login)
    {
        Ok(ticket) => {
            let response = WatchTicketResponse {
                watch_url: format!("/watch/{ticket}"),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to issue watch ticket",
        ),
    }
}

#[derive(Debug, Deserialize)]
struct QualitySwitchQuery {
    channel_login: String,
    quality: String,
}

#[derive(Debug, Serialize)]
struct QualitySwitchResponse {
    watch_url: String,
    quality: String,
}

async fn quality_switch_handler(
    State(state): State<ProtectedState>,
    headers: HeaderMap,
    Query(query): Query<QualitySwitchQuery>,
) -> Response {
    if !state.catalog.has_channel(&query.channel_login).await {
        return error_response(StatusCode::BAD_REQUEST, "channel is not in channel list");
    }

    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    match state
        .playback
        .issue_ticket(&session_token, &query.channel_login)
    {
        Ok(ticket) => {
            let stream_id = &ticket;

            if let Err(e) = state
                .stream
                .open_session(
                    stream_id,
                    &query.channel_login,
                    &session_token,
                    &query.quality,
                )
                .await
            {
                tracing::error!(error = ?e, channel = %query.channel_login, quality = %query.quality, "failed to open stream session for quality switch");
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    "failed to open stream with requested quality",
                );
            }

            let response = QualitySwitchResponse {
                watch_url: format!("/watch/{ticket}"),
                quality: query.quality,
            };
            (StatusCode::OK, Json(response)).into_response()
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
    Query(query): Query<stream_proxy::RelayQuery>,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    let validated = match state.playback.validate_ticket(&ticket, &session_token) {
        Ok(v) => v,
        Err(PlaybackTicketError::InvalidTicket) | Err(PlaybackTicketError::ExpiredTicket) => {
            return error_response(StatusCode::UNAUTHORIZED, "invalid or expired watch ticket");
        }
        Err(PlaybackTicketError::SessionMismatch) => {
            return error_response(
                StatusCode::FORBIDDEN,
                "watch ticket belongs to a different session",
            );
        }
    };

    if let Err(e) = state
        .stream
        .open_session(&ticket, &validated.channel_login, &session_token, "best")
        .await
    {
        return match e {
            stream_proxy::StreamError::HlsFetchFailed(msg) => {
                tracing::error!(error = %msg, channel = %validated.channel_login, "failed to open stream session");
                render_error_page(
                    &validated.channel_login,
                    "Stream unavailable. The channel may be offline or not accessible.",
                )
            }
            _ => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to open stream session",
            ),
        };
    }

    let html = render_stream_page(
        &validated.channel_login,
        &ticket,
        &session_token,
        query.force_relay(),
    );

    Html(html).into_response()
}

fn render_stream_page(
    channel: &str,
    stream_id: &str,
    session_token: &str,
    force_relay: bool,
) -> String {
    let relay_suffix = if force_relay { "?relay=1" } else { "" };
    let manifest_url = format!("/stream/{stream_id}/{session_token}/manifest{relay_suffix}");
    let template = include_str!("templates/watch.html");
    let bootstrap = format!(
        "window.WATCH_CHANNEL = {channel};\nwindow.WATCH_MANIFEST_URL = {manifest_url};",
        channel = serde_json::to_string(channel).unwrap_or_else(|_| "\"\"".to_string()),
        manifest_url = serde_json::to_string(&manifest_url).unwrap_or_else(|_| "\"\"".to_string()),
    );

    template
        .replace("__CHANNEL__", channel)
        .replace("__WATCH_BOOTSTRAP__", &bootstrap)
}

fn render_error_page(channel: &str, message: &str) -> Response {
    let html = format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Watch {channel}</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    background: #0b0f14;
    color: #f3f6fa;
    font-family: system-ui, -apple-system, 'Segoe UI', sans-serif;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }}
  header {{
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #2a3442;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }}
  header strong {{ font-size: 1rem; font-weight: 700; text-transform: lowercase; }}
  .error-screen {{
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
  }}
  .error-box {{
    text-align: center;
    max-width: 28rem;
    background: rgba(20, 28, 43, 0.95);
    border: 1px solid rgba(164, 182, 216, 0.25);
    border-radius: 1rem;
    padding: 1.5rem;
  }}
  .error-box p {{
    color: #9eb3d6;
    line-height: 1.6;
  }}
</style>
</head>
<body>
<header>
  <strong>{channel}</strong>
  <span>via Twitch Relay</span>
</header>
<div class="error-screen">
  <div class="error-box">
    <p>{message}</p>
  </div>
</div>
</body>
</html>"#,
        channel = channel,
        message = message
    );

    (StatusCode::OK, Html(html)).into_response()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
