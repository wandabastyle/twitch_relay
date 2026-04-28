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
    format!(
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
    overflow: hidden;
  }}
  .watch-shell {{
    flex: 1;
    display: flex;
    min-height: 0;
    align-items: center;
    justify-content: center;
    padding: clamp(8px, 1.2vw, 16px);
    gap: 12px;
  }}
  header {{
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #2a3442;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-shrink: 0;
  }}
  header strong {{
    font-size: 1rem;
    font-weight: 700;
    text-transform: lowercase;
    color: #f2f7ff;
  }}
  header span {{
    font-size: 0.82rem;
    color: #9cb2d7;
  }}
  .video-container {{
    flex: 0 0 auto;
    position: relative;
    background: #000;
    width: min(1280px, 100%);
    aspect-ratio: 16 / 9;
    border: 1px solid #2a3442;
    cursor: none;
  }}
  .video-container.controls-visible {{
    cursor: default;
  }}
  .chat-panel {{
    width: min(360px, 38vw);
    min-width: 280px;
    border: 1px solid #2a3442;
    background: #0f141c;
    display: flex;
    flex-direction: column;
    min-height: 200px;
  }}
  .chat-header {{
    padding: 0.65rem 0.75rem;
    border-bottom: 1px solid #2a3442;
    color: #b7c6df;
    font-size: 0.82rem;
  }}
  .chat-messages {{
    flex: 1;
    overflow-y: auto;
    padding: 0.65rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }}
  .chat-messages::-webkit-scrollbar {{
    display: none;
  }}
  .chat-message {{
    line-height: 1.35;
    word-break: break-word;
    font-size: 0.9rem;
  }}
  .chat-emote {{
    height: 1.6em;
    width: auto;
    vertical-align: middle;
    margin: 0 0.05em;
  }}
  .chat-message .who {{
    color: #8eb6ff;
    font-weight: 600;
    margin-right: 0.35rem;
  }}
  .chat-message.notice .who {{
    color: #f3ba70;
  }}
  .chat-form {{
    display: flex;
    flex-wrap: nowrap;
    align-items: center;
    gap: 0.45rem;
    border-top: 1px solid #2a3442;
    padding: 0.65rem;
    position: relative;
  }}
  .chat-emote-btn {{
    width: 2.15rem;
    height: 2.15rem;
    min-width: 2.15rem;
    border: 1px solid #2f3f55;
    background: #101824;
    color: #d7e7ff;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
  }}
  .chat-emote-btn:hover {{
    border-color: #4d6487;
    background: #172233;
  }}
  .chat-input {{
    flex: 1 1 0%;
    background: #0b1017;
    border: 1px solid #29374b;
    color: #ecf4ff;
    border-radius: 6px;
    padding: 0.45rem 0.55rem;
    height: 2.15rem;
    min-height: 2.15rem;
    max-height: 2.15rem;
    line-height: 1.2;
    white-space: nowrap;
    overflow-x: auto;
    overflow-y: hidden;
    word-break: normal;
  }}
  .chat-input:focus {{
    outline: none;
    border-color: #4b668d;
  }}
  .chat-input:empty::before {{
    content: attr(data-placeholder);
    color: #8ea3c5;
    pointer-events: none;
  }}
  .chat-input .composer-emote {{
    height: 1em;
    width: auto;
    vertical-align: middle;
    margin: 0 0.06em;
  }}
  .chat-send {{
    background: #2c65f5;
    border: 0;
    color: #f8fbff;
    border-radius: 6px;
    padding: 0.45rem 0.75rem;
    font-weight: 600;
    cursor: pointer;
  }}
  .emote-popup {{
    position: absolute;
    left: 0.65rem;
    right: 0.65rem;
    bottom: calc(100% + 0.5rem);
    background: #0f141c;
    border: 1px solid #2a3442;
    border-radius: 8px;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.45);
    display: none;
    max-height: min(52vh, 420px);
    overflow: hidden;
    z-index: 40;
  }}
  .emote-popup.open {{
    display: flex;
    flex-direction: column;
  }}
  .emote-search {{
    margin: 0.6rem;
    border: 1px solid #2f3f55;
    background: #0b1017;
    color: #ecf4ff;
    border-radius: 6px;
    padding: 0.45rem 0.55rem;
  }}
  .emote-groups {{
    overflow-y: auto;
    padding: 0 0.6rem 0.6rem;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }}
  .emote-groups::-webkit-scrollbar {{
    display: none;
  }}
  .emote-group-title {{
    color: #97afcf;
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    margin: 0.5rem 0 0.38rem;
  }}
  .emote-grid {{
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 0.35rem;
  }}
  .emote-item {{
    border: 1px solid #2a3442;
    border-radius: 6px;
    background: #101824;
    color: #d7e7ff;
    min-height: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    position: relative;
  }}
  .emote-item:hover,
  .emote-item.active {{
    border-color: #4b668d;
    background: #182436;
  }}
  .emote-item img {{
    max-height: 30px;
    max-width: 30px;
  }}
  .emote-empty {{
    color: #9eb3d6;
    font-size: 0.85rem;
    padding: 0.75rem 0.2rem;
  }}
  .emote-suggestions {{
    position: absolute;
    left: 2.85rem;
    right: 0.65rem;
    bottom: calc(100% + 0.42rem);
    border: 1px solid #2f3f55;
    background: #0f141c;
    border-radius: 6px;
    box-shadow: 0 8px 20px rgba(0,0,0,0.42);
    display: none;
    max-height: 180px;
    overflow-y: auto;
    scrollbar-width: auto;
    -ms-overflow-style: auto;
    z-index: 45;
  }}
  .emote-suggestions::-webkit-scrollbar {{
    width: 10px;
  }}
  .emote-suggestions::-webkit-scrollbar-thumb {{
    background: #3a4a61;
    border-radius: 8px;
  }}
  .emote-suggestions::-webkit-scrollbar-track {{
    background: #101722;
  }}
  .emote-suggestions.open {{
    display: block;
  }}
  .emote-suggestion {{
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.35rem 0.5rem;
    cursor: pointer;
    color: #deebff;
    font-size: 0.86rem;
  }}
  .emote-suggestion:hover,
  .emote-suggestion.active {{
    background: #1a2537;
  }}
  .emote-suggestion img {{
    height: 22px;
    width: auto;
  }}
  video {{
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    cursor: inherit;
  }}
  .controls-bar {{
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: linear-gradient(transparent, rgba(0,0,0,0.9));
    padding: 20px 10px 8px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    z-index: 10;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s;
  }}
  .video-container.controls-visible .controls-bar {{
    opacity: 1;
    pointer-events: auto;
  }}
  .controls-left, .controls-right {{
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  .ctrl-btn {{
    background: transparent;
    border: none;
    color: white;
    cursor: pointer;
    padding: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: background 0.15s;
  }}
  .ctrl-btn:hover {{
    background: rgba(255,255,255,0.2);
  }}
  .ctrl-btn svg {{
    width: 20px;
    height: 20px;
  }}
  .volume-control {{
    display: flex;
    align-items: center;
    gap: 4px;
  }}
  .volume-slider {{
    width: 0;
    opacity: 0;
    transition: width 0.2s, opacity 0.2s;
    height: 4px;
    -webkit-appearance: none;
    background: rgba(255,255,255,0.3);
    border-radius: 2px;
    cursor: pointer;
  }}
  .volume-control:hover .volume-slider {{
    width: 60px;
    opacity: 1;
  }}
  .volume-slider::-webkit-slider-thumb {{
    -webkit-appearance: none;
    width: 12px;
    height: 12px;
    background: white;
    border-radius: 50%;
    cursor: pointer;
  }}
  .time-display {{
    color: white;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    min-width: 90px;
    text-align: center;
  }}
  .quality-btn {{
    background: rgba(255,255,255,0.1);
    border: none;
    color: white;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: background 0.15s;
  }}
  .quality-btn:hover {{
    background: rgba(255,255,255,0.2);
  }}
  .go-live-btn {{
    display: inline-flex;
    background: rgba(239, 68, 68, 0.25);
    border: 1px solid rgba(239, 68, 68, 0.55);
  }}
  .go-live-btn.live {{
    background: rgba(239, 68, 68, 0.4);
    border-color: rgba(248, 113, 113, 0.75);
  }}
  .go-live-btn:disabled {{
    opacity: 0.65;
    cursor: not-allowed;
  }}
  .quality-menu {{
    position: absolute;
    bottom: 50px;
    right: 8px;
    background: rgba(20, 24, 32, 0.95);
    border-radius: 6px;
    padding: 6px 0;
    min-width: 140px;
    display: none;
    z-index: 20;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  }}
  .quality-menu.open {{
    display: block;
  }}
  .quality-menu-item {{
    padding: 8px 14px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    justify-content: space-between;
  }}
  .quality-menu-item:hover {{
    background: rgba(255, 255, 255, 0.1);
  }}
  .quality-menu-item.active {{
    color: #9147ff;
    font-weight: 600;
  }}
  .quality-menu-item .bitrate {{
    color: #9cb2d7;
    font-size: 11px;
  }}
  .progress-bar {{
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 5px;
    background: rgba(255,255,255,0.2);
    cursor: pointer;
    z-index: 15;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s, height 0.15s;
  }}
  .video-container.controls-visible .progress-bar {{
    opacity: 1;
    pointer-events: auto;
  }}
  .progress-bar:hover {{
    height: 8px;
  }}
  .progress-bar.disabled {{
    cursor: not-allowed;
    background: rgba(255,255,255,0.12);
  }}
  .progress-bar.disabled:hover {{
    height: 5px;
  }}
  .progress-bar.disabled .progress-buffered,
  .progress-bar.disabled .progress-played {{
    opacity: 0.55;
  }}
  .progress-buffered {{
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    background: rgba(255,255,255,0.3);
  }}
  .progress-played {{
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    background: #9147ff;
  }}
  .controls-left, .controls-right {{
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  .ctrl-btn {{
    background: transparent;
    border: none;
    color: white;
    cursor: pointer;
    padding: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: background 0.15s;
  }}
  .ctrl-btn:hover {{
    background: rgba(255,255,255,0.2);
  }}
  .ctrl-btn svg {{
    width: 20px;
    height: 20px;
  }}
  .volume-control {{
    display: flex;
    align-items: center;
    gap: 4px;
  }}
  .volume-slider {{
    width: 0;
    opacity: 0;
    transition: width 0.2s, opacity 0.2s;
    height: 4px;
    -webkit-appearance: none;
    background: rgba(255,255,255,0.3);
    border-radius: 2px;
    cursor: pointer;
  }}
  .volume-control:hover .volume-slider {{
    width: 60px;
    opacity: 1;
  }}
  .volume-slider::-webkit-slider-thumb {{
    -webkit-appearance: none;
    width: 12px;
    height: 12px;
    background: white;
    border-radius: 50%;
    cursor: pointer;
  }}
  .time-display {{
    color: white;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    min-width: 90px;
    text-align: center;
  }}
  .quality-btn {{
    background: rgba(255,255,255,0.1);
    border: none;
    color: white;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: background 0.15s;
  }}
  .quality-btn:hover {{
    background: rgba(255,255,255,0.2);
  }}
  .quality-menu {{
    position: absolute;
    bottom: 50px;
    right: 8px;
    background: rgba(20, 24, 32, 0.95);
    border-radius: 6px;
    padding: 6px 0;
    min-width: 140px;
    display: none;
    z-index: 20;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  }}
  .quality-menu.open {{
    display: block;
  }}
  .quality-menu-item {{
    padding: 8px 14px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    justify-content: space-between;
  }}
  .quality-menu-item:hover {{
    background: rgba(255, 255, 255, 0.1);
  }}
  .quality-menu-item.active {{
    color: #9147ff;
    font-weight: 600;
  }}
  .quality-menu-item .bitrate {{
    color: #9cb2d7;
    font-size: 11px;
  }}
  @media (max-width: 700px) {{
    .watch-shell {{
      padding: 6px;
      flex-direction: column;
      align-items: stretch;
      justify-content: flex-start;
    }}
    .video-container {{
      width: 100%;
      height: auto;
      max-width: 100%;
    }}
    .chat-panel {{
      width: 100%;
      min-width: 0;
      height: 40vh;
    }}
  }}
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
  }}
  .error-box p {{
    color: #9eb3d6;
    line-height: 1.6;
    margin-top: 0.5rem;
  }}
</style>
</head>
<body>
<header>
  <strong>{channel}</strong>
  <span>via Twitch Relay</span>
</header>
<main class="watch-shell">
  <div class="video-container" id="videoContainer">
    <video id="player" autoplay></video>
    <div class="progress-bar" id="progressBar">
      <div class="progress-buffered" id="progressBuffered"></div>
      <div class="progress-played" id="progressPlayed"></div>
    </div>
    <div class="controls-bar" id="controlsBar">
      <div class="controls-left">
        <button class="ctrl-btn" id="playBtn" title="Play/Pause">
          <svg class="play-icon" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z"/>
          </svg>
          <svg class="pause-icon" viewBox="0 0 24 24" fill="currentColor" style="display:none">
            <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>
          </svg>
        </button>
        <div class="volume-control">
          <button class="ctrl-btn" id="volumeBtn" title="Mute">
            <svg class="volume-high" viewBox="0 0 24 24" fill="currentColor">
              <path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z"/>
            </svg>
            <svg class="volume-mute" viewBox="0 0 24 24" fill="currentColor" style="display:none">
              <path d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z"/>
            </svg>
          </button>
          <input type="range" class="volume-slider" id="volumeSlider" min="0" max="1" step="0.05" value="1">
        </div>
        <div class="time-display">
          <span id="currentTime">0:00</span> / <span id="duration">0:00</span>
        </div>
      </div>
      <div class="controls-right">
        <button class="quality-btn go-live-btn" id="goLiveBtn" title="Jump to live edge">Go Live</button>
        <button class="quality-btn" id="qualityBtn">Auto</button>
        <button class="ctrl-btn" id="fullscreenBtn" title="Fullscreen">
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z"/>
          </svg>
        </button>
      </div>
    </div>
    <div class="quality-menu" id="qualityMenu"></div>
  </div>
  <aside class="chat-panel">
    <div class="chat-header" id="chatStatus">Connecting chat...</div>
    <div class="chat-messages" id="chatMessages"></div>
    <form class="chat-form" id="chatForm">
      <button class="chat-emote-btn" type="button" id="chatEmoteBtn" title="Open emote picker">☺</button>
      <div class="chat-input" id="chatComposer" contenteditable="true" role="textbox" aria-label="Send a message" data-placeholder="Send a message"></div>
      <button class="chat-send" type="submit" id="chatSendBtn">Send</button>
      <div class="emote-suggestions" id="emoteSuggestions"></div>
      <div class="emote-popup" id="emotePopup" role="dialog" aria-label="Emote picker">
        <input class="emote-search" id="emoteSearch" type="text" placeholder="Search emotes" autocomplete="off" />
        <div class="emote-groups" id="emoteGroups"></div>
      </div>
    </form>
  </aside>
</main>
<script src="/static/hls.js"></script>
<script>
  const CHAT_EMOTE_SCALE = '2.0';
  const chatChannel = '{channel}';
  const video = document.getElementById('player');
  const videoContainer = document.getElementById('videoContainer');
  const controlsBar = document.getElementById('controlsBar');
  const playBtn = document.getElementById('playBtn');
  const playIcon = playBtn.querySelector('.play-icon');
  const pauseIcon = playBtn.querySelector('.pause-icon');
  const volumeBtn = document.getElementById('volumeBtn');
  const volumeHigh = volumeBtn.querySelector('.volume-high');
  const volumeMute = volumeBtn.querySelector('.volume-mute');
  const volumeSlider = document.getElementById('volumeSlider');
  const currentTimeEl = document.getElementById('currentTime');
  const durationEl = document.getElementById('duration');
  const progressBar = document.getElementById('progressBar');
  const progressBuffered = document.getElementById('progressBuffered');
  const progressPlayed = document.getElementById('progressPlayed');
  const goLiveBtn = document.getElementById('goLiveBtn');
  const qualityBtn = document.getElementById('qualityBtn');
  const qualityMenu = document.getElementById('qualityMenu');
  const chatStatus = document.getElementById('chatStatus');
  const chatMessages = document.getElementById('chatMessages');
  const chatForm = document.getElementById('chatForm');
  const chatComposer = document.getElementById('chatComposer');
  const chatSendBtn = document.getElementById('chatSendBtn');
  const chatEmoteBtn = document.getElementById('chatEmoteBtn');
  const emotePopup = document.getElementById('emotePopup');
  const emoteSearch = document.getElementById('emoteSearch');
  const emoteGroups = document.getElementById('emoteGroups');
  const emoteSuggestions = document.getElementById('emoteSuggestions');
  const chatPanel = document.querySelector('.chat-panel');
  const watchShell = document.querySelector('.watch-shell');
  let chatEvents = null;
  const fullscreenBtn = document.getElementById('fullscreenBtn');
  const MOBILE_LAYOUT_QUERY = window.matchMedia('(max-width: 700px)');
  const LIVE_STATUS_CACHE_KEY = 'twitchRelay.liveStatus';
  const LIVE_STATUS_REFRESH_MS = 45000;

  let hlsInstance = null;
  let debugVisible = false;
  let controlsTimeout = null;
  let liveStatusRefreshTimer = null;
  const CONTROLS_HIDE_DELAY_MS = 2000;
  const AUTO_LIVE_SEEK_LAG_SECS = 6;
  const AUTO_LIVE_TARGET_OFFSET_SECS = 2;
  const AUTO_LIVE_SEEK_CONSECUTIVE_CHECKS = 2;
  const AUTO_LIVE_SEEK_COOLDOWN_MS = 12000;
  const AUTO_LIVE_CHECK_INTERVAL_MS = 2000;
  const MANUAL_SEEK_SUPPRESS_MS = 30000;
  const LIVE_BUTTON_ENTER_LIVE_SECS = 4.2;
  const LIVE_BUTTON_EXIT_LIVE_SECS = 5.2;
  let currentPlayingLevelIdx = -1;
  let userSelectedAuto = true;
  let attemptedRelayFallback = new URLSearchParams(window.location.search).get('relay') === '1';
  let availableEmotes = [];
  let emotePickerLoaded = false;
  let emotePickerOpen = false;
  let emoteSearchTerm = '';
  let emoteSuggestionsOpen = false;
  let emoteSuggestionIndex = 0;
  let emoteSuggestionItems = [];
  let liveLagHighStreak = 0;
  let lastAutoLiveCheckAtMs = 0;
  let lastAutoLiveSeekAtMs = 0;
  let manualSeekSuppressUntilMs = 0;
  let liveButtonIsLive = true;
  
  const debugOverlay = document.createElement('div');
  debugOverlay.style.cssText = 'position:fixed;top:50px;left:10px;background:rgba(0,0,0,0.9);color:#0f0;padding:10px;font-family:monospace;font-size:11px;z-index:99999;display:none;max-width:350px;border-radius:4px;';
  document.body.appendChild(debugOverlay);

  function readNumericStyle(element, propertyName) {{
    const value = getComputedStyle(element).getPropertyValue(propertyName);
    const parsed = parseFloat(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }}

  function currentAspectRatio() {{
    if (video.videoWidth > 0 && video.videoHeight > 0) {{
      return video.videoWidth / video.videoHeight;
    }}

    const ratioText = (videoContainer.style.aspectRatio || getComputedStyle(videoContainer).aspectRatio || '16 / 9').trim();
    if (ratioText.includes('/')) {{
      const parts = ratioText.split('/');
      const w = parseFloat(parts[0]);
      const h = parseFloat(parts[1]);
      if (Number.isFinite(w) && Number.isFinite(h) && w > 0 && h > 0) {{
        return w / h;
      }}
    }}

    const numeric = parseFloat(ratioText);
    if (Number.isFinite(numeric) && numeric > 0) {{
      return numeric;
    }}

    return 16 / 9;
  }}

  function syncPlayerLayout() {{
    if (MOBILE_LAYOUT_QUERY.matches || document.fullscreenElement === videoContainer) {{
      videoContainer.style.removeProperty('width');
      videoContainer.style.removeProperty('height');
      videoContainer.style.removeProperty('max-height');
      chatPanel.style.removeProperty('height');
      return;
    }}

    const shellRect = watchShell.getBoundingClientRect();
    const shellPadX = readNumericStyle(watchShell, 'padding-left') + readNumericStyle(watchShell, 'padding-right');
    const shellPadY = readNumericStyle(watchShell, 'padding-top') + readNumericStyle(watchShell, 'padding-bottom');
    const availableWidth = Math.max(280, shellRect.width - shellPadX);
    const availableHeight = Math.max(220, shellRect.height - shellPadY);
    const chatWidth = chatPanel.getBoundingClientRect().width || 320;
    const gap = readNumericStyle(watchShell, 'column-gap') || 12;
    const ratio = currentAspectRatio();

    const widthByHeight = availableHeight * ratio;
    const widthBySpace = Math.max(280, availableWidth - chatWidth - gap);
    const videoWidth = Math.max(280, Math.min(widthByHeight, widthBySpace));
    const videoHeight = Math.max(160, videoWidth / ratio);

    videoContainer.style.width = Math.round(videoWidth) + 'px';
    videoContainer.style.height = Math.round(videoHeight) + 'px';
    videoContainer.style.maxHeight = Math.round(availableHeight) + 'px';
    chatPanel.style.height = Math.round(videoHeight) + 'px';
  }}

  function applyVideoAspectRatio() {{
    if (video.videoWidth > 0 && video.videoHeight > 0) {{
      videoContainer.style.aspectRatio = video.videoWidth + ' / ' + video.videoHeight;
    }}
    syncPlayerLayout();
  }}

  function showControls() {{
    videoContainer.classList.add('controls-visible');
    controlsBar.classList.add('visible');
    clearTimeout(controlsTimeout);
    if (!video.paused) {{
      controlsTimeout = setTimeout(hideControls, CONTROLS_HIDE_DELAY_MS);
    }}
  }}

  function hideControls() {{
    if (!video.paused) {{
      videoContainer.classList.remove('controls-visible');
      controlsBar.classList.remove('visible');
    }}
  }}

  function formatTime(seconds) {{
    if (!isFinite(seconds)) return '0:00';
    var h = Math.floor(seconds / 3600);
    var m = Math.floor((seconds % 3600) / 60);
    var s = Math.floor(seconds % 60);
    if (h > 0) {{
      return h + ':' + (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
    }}
    return m + ':' + (s < 10 ? '0' : '') + s;
  }}

  function formatBitrate(bitrate) {{
    if (!bitrate) return '';
    var mbps = (bitrate / 1000000).toFixed(1);
    return mbps + ' Mbps';
  }}

  function clamp(value, min, max) {{
    return Math.min(max, Math.max(min, value));
  }}

  function getTimelineModel() {{
    var duration = video.duration;
    if (isFinite(duration) && duration > 0) {{
      return {{ mode: 'vod', start: 0, end: duration, length: duration, seekable: true }};
    }}

    if (video.seekable.length > 0) {{
      var idx = video.seekable.length - 1;
      var start = video.seekable.start(idx);
      var end = video.seekable.end(idx);
      var length = end - start;
      if (isFinite(start) && isFinite(end) && length > 0) {{
        return {{ mode: 'live-dvr', start: start, end: end, length: length, seekable: true }};
      }}
    }}

    return {{ mode: 'live-not-seekable', start: 0, end: 0, length: 0, seekable: false }};
  }}

  function getTimelinePercent(time, timeline) {{
    if (!timeline || timeline.length <= 0) return 0;
    return clamp((time - timeline.start) / timeline.length, 0, 1) * 100;
  }}

  function updateTimelineInteractivity(timeline) {{
    var canSeek = !!(timeline && timeline.seekable);
    progressBar.classList.toggle('disabled', !canSeek);
    progressBar.setAttribute('aria-disabled', canSeek ? 'false' : 'true');
    if (canSeek) {{
      progressBar.removeAttribute('title');
    }} else {{
      progressBar.setAttribute('title', 'Live stream is not seekable');
    }}
  }}

  function updateTime() {{
    var timeline = getTimelineModel();
    updateTimelineInteractivity(timeline);
    updateGoLiveButton(timeline);
    maybeAutoCatchUpLive(timeline);
    currentTimeEl.textContent = formatTime(video.currentTime);
    if (timeline.mode === 'vod') {{
      durationEl.textContent = formatTime(timeline.end);
    }} else if (timeline.mode === 'live-dvr') {{
      durationEl.textContent = formatTime(timeline.length);
    }} else {{
      durationEl.textContent = 'LIVE';
    }}
    progressPlayed.style.width = getTimelinePercent(video.currentTime, timeline) + '%';
  }}

  function updateBuffer() {{
    var timeline = getTimelineModel();
    var bufferedPercent = 0;
    if (video.buffered.length > 0) {{
      var bufferedEnd = video.buffered.end(video.buffered.length - 1);
      if (timeline.mode === 'vod' && timeline.length > 0) {{
        bufferedPercent = clamp(bufferedEnd / timeline.length, 0, 1) * 100;
      }} else if (timeline.mode === 'live-dvr') {{
        bufferedPercent = getTimelinePercent(bufferedEnd, timeline);
      }}
    }}
    progressBuffered.style.width = bufferedPercent + '%';
  }}

  function updatePlayButton() {{
    if (video.paused) {{
      playIcon.style.display = 'block';
      pauseIcon.style.display = 'none';
    }} else {{
      playIcon.style.display = 'none';
      pauseIcon.style.display = 'block';
    }}
  }}

  function updateVolumeButton() {{
    if (video.muted || video.volume === 0) {{
      volumeHigh.style.display = 'none';
      volumeMute.style.display = 'block';
    }} else {{
      volumeHigh.style.display = 'block';
      volumeMute.style.display = 'none';
    }}
  }}

  function togglePlay() {{
    if (video.paused) {{
      video.play();
    }} else {{
      video.pause();
    }}
  }}

  function toggleMute() {{
    video.muted = !video.muted;
    updateVolumeButton();
  }}

  function seek(e) {{
    var timeline = getTimelineModel();
    if (!timeline.seekable || timeline.length <= 0) return;
    var rect = progressBar.getBoundingClientRect();
    if (rect.width <= 0) return;
    var percent = clamp((e.clientX - rect.left) / rect.width, 0, 1);
    if (timeline.mode === 'vod') {{
      video.currentTime = percent * timeline.length;
    }} else {{
      video.currentTime = timeline.start + (percent * timeline.length);
      manualSeekSuppressUntilMs = Date.now() + MANUAL_SEEK_SUPPRESS_MS;
      liveLagHighStreak = 0;
    }}
  }}

  function toggleFullscreen() {{
    if (document.fullscreenElement) {{
      document.exitFullscreen();
    }} else {{
      videoContainer.requestFullscreen();
    }}
  }}

  function updateGoLiveButton(timeline) {{
    if (!timeline.seekable) {{
      liveButtonIsLive = true;
      goLiveBtn.textContent = 'Live';
      goLiveBtn.classList.add('live');
      goLiveBtn.disabled = true;
      return;
    }}

    var lag = Math.max(0, timeline.end - video.currentTime);
    if (liveButtonIsLive) {{
      if (lag > LIVE_BUTTON_EXIT_LIVE_SECS) {{
        liveButtonIsLive = false;
      }}
    }} else if (lag < LIVE_BUTTON_ENTER_LIVE_SECS) {{
      liveButtonIsLive = true;
    }}

    goLiveBtn.textContent = liveButtonIsLive ? 'Live' : 'Go Live';
    goLiveBtn.classList.toggle('live', liveButtonIsLive);
    goLiveBtn.disabled = liveButtonIsLive;
  }}

  function maybeAutoCatchUpLive(timeline) {{
    if (!timeline || !timeline.seekable || timeline.mode === 'vod' || document.visibilityState !== 'visible') {{
      liveLagHighStreak = 0;
      return;
    }}

    const now = Date.now();
    if (now < manualSeekSuppressUntilMs) {{
      liveLagHighStreak = 0;
      return;
    }}

    if (now - lastAutoLiveCheckAtMs < AUTO_LIVE_CHECK_INTERVAL_MS) {{
      return;
    }}
    lastAutoLiveCheckAtMs = now;

    const lag = Math.max(0, timeline.end - video.currentTime);
    if (lag > AUTO_LIVE_SEEK_LAG_SECS) {{
      liveLagHighStreak += 1;
    }} else {{
      liveLagHighStreak = 0;
      return;
    }}

    if (liveLagHighStreak < AUTO_LIVE_SEEK_CONSECUTIVE_CHECKS) {{
      return;
    }}

    if (now - lastAutoLiveSeekAtMs < AUTO_LIVE_SEEK_COOLDOWN_MS) {{
      return;
    }}

    video.currentTime = Math.max(timeline.start, timeline.end - AUTO_LIVE_TARGET_OFFSET_SECS);
    lastAutoLiveSeekAtMs = now;
    liveLagHighStreak = 0;
  }}

  function goLive() {{
    var timeline = getTimelineModel();
    if (!timeline.seekable || timeline.length <= 0) return;
    video.currentTime = timeline.end;
    manualSeekSuppressUntilMs = 0;
    liveLagHighStreak = 0;
    showControls();
  }}

  function updateDebug() {{
    if (!debugVisible) return;
    var bufferedRanges = [];
    for (var i = 0; i < video.buffered.length; i++) {{
      bufferedRanges.push({{ start: video.buffered.start(i).toFixed(1), end: video.buffered.end(i).toFixed(1) }});
    }}
    var currentQuality = 'Auto';
    if (hlsInstance && hlsInstance.currentLevel >= 0 && hlsInstance.levels && hlsInstance.levels[hlsInstance.currentLevel]) {{
      currentQuality = hlsInstance.levels[hlsInstance.currentLevel].height + 'p';
    }}
    debugOverlay.innerHTML = '' +
      '<div style="margin-bottom:8px;font-weight:bold;">Debug (Shift+D)</div>' +
      '<div>Quality: ' + currentQuality + '</div>' +
      '<div>Levels: ' + (hlsInstance ? hlsInstance.levels.length : 0) + '</div>' +
      '<div>currentTime: ' + video.currentTime.toFixed(1) + '</div>' +
      '<div>paused: ' + video.paused + '</div>' +
      '<div>buffered: ' + JSON.stringify(bufferedRanges) + '</div>';
  }}

  function isObject(value) {{
    return value !== null && typeof value === 'object' && !Array.isArray(value);
  }}

  async function refreshLiveStatusCache() {{
    try {{
      const response = await fetch('/api/live-status', {{ credentials: 'same-origin' }});
      if (!response.ok) {{
        return;
      }}

      const payload = await response.json();
      if (!isObject(payload) || !isObject(payload.channels)) {{
        return;
      }}

      window.sessionStorage.setItem(
        LIVE_STATUS_CACHE_KEY,
        JSON.stringify({{
          timestamp: Date.now(),
          data: {{
            channels: payload.channels
          }}
        }})
      );
    }} catch (_) {{}}
  }}

  function handleVisibilityChange() {{
    if (document.visibilityState === 'visible') {{
      void refreshLiveStatusCache();
    }}
  }}

  function startLiveStatusRefreshLoop() {{
    void refreshLiveStatusCache();

    if (liveStatusRefreshTimer) {{
      clearInterval(liveStatusRefreshTimer);
    }}

    liveStatusRefreshTimer = setInterval(function() {{
      if (document.visibilityState !== 'visible') {{
        return;
      }}
      void refreshLiveStatusCache();
    }}, LIVE_STATUS_REFRESH_MS);
  }}

  document.addEventListener('keydown', function(e) {{
    if (e.shiftKey && (e.key === 'D' || e.key === 'd')) {{
      debugVisible = !debugVisible;
      debugOverlay.style.display = debugVisible ? 'block' : 'none';
      if (debugVisible) updateDebug();
    }}
  }});

  playBtn.addEventListener('click', togglePlay);
  video.addEventListener('click', function(e) {{
    if (e.target === video) togglePlay();
  }});
  volumeBtn.addEventListener('click', toggleMute);
  chatEmoteBtn.addEventListener('click', function() {{
    if (emotePickerOpen) {{
      closeEmotePicker();
      placeComposerCaretAtEnd();
    }} else {{
      openEmotePicker();
    }}
  }});
  emoteSearch.addEventListener('input', function() {{
    emoteSearchTerm = emoteSearch.value || '';
    renderEmotePicker();
  }});
  volumeSlider.addEventListener('input', function() {{
    video.volume = this.value;
    video.muted = false;
    updateVolumeButton();
  }});
  progressBar.addEventListener('click', seek);
  goLiveBtn.addEventListener('click', goLive);
  fullscreenBtn.addEventListener('click', toggleFullscreen);

  video.addEventListener('play', function() {{
    updatePlayButton();
    showControls();
  }});
  video.addEventListener('pause', function() {{
    updatePlayButton();
    showControls();
  }});
  video.addEventListener('timeupdate', function() {{
    updateTime();
    updateBuffer();
    updateDebug();
  }});
  video.addEventListener('progress', updateBuffer);
  video.addEventListener('durationchange', function() {{
    updateTime();
    updateBuffer();
  }});
  video.addEventListener('loadedmetadata', function() {{
    updateTime();
    updateBuffer();
    updatePlayButton();
    updateVolumeButton();
    applyVideoAspectRatio();
  }});
  video.addEventListener('volumechange', updateVolumeButton);
  video.addEventListener('waiting', function() {{ video.style.opacity = '0.7'; }});
  video.addEventListener('playing', function() {{ video.style.opacity = '1'; }});
  videoContainer.addEventListener('mouseenter', showControls);
  videoContainer.addEventListener('mousemove', showControls);
  videoContainer.addEventListener('mouseleave', function() {{ if (!video.paused) hideControls(); }});
  chatComposer.addEventListener('input', function() {{
    normalizeComposerInput();
    refreshEmoteSuggestions();
  }});
  chatComposer.addEventListener('click', function() {{
    placeComposerCaretAtEnd();
    refreshEmoteSuggestions();
  }});
  chatComposer.addEventListener('paste', function(e) {{
    e.preventDefault();
    const text = ((e.clipboardData && e.clipboardData.getData('text/plain')) || '').replace(/[\r\n]+/g, ' ');
    if (!text) return;

    const plain = getComposerPlainText();
    applyPlainTextToComposer((plain + text).slice(0, 500));
    refreshEmoteSuggestions();
  }});
  chatComposer.addEventListener('keydown', function(e) {{
    if (e.key === 'Enter' && !(emoteSuggestionsOpen && emoteSuggestionItems.length)) {{
      e.preventDefault();
      chatForm.requestSubmit();
      return;
    }}

    if (!emoteSuggestionsOpen || !emoteSuggestionItems.length) {{
      if (e.key === 'Escape') {{
        closeEmotePicker();
      }}
      return;
    }}

    if (e.key === 'ArrowDown') {{
      e.preventDefault();
      emoteSuggestionIndex = (emoteSuggestionIndex + 1) % emoteSuggestionItems.length;
      renderEmoteSuggestions();
      return;
    }}
    if (e.key === 'ArrowUp') {{
      e.preventDefault();
      emoteSuggestionIndex = (emoteSuggestionIndex - 1 + emoteSuggestionItems.length) % emoteSuggestionItems.length;
      renderEmoteSuggestions();
      return;
    }}
    if (e.key === 'Tab' || e.key === 'Enter') {{
      e.preventDefault();
      const selected = emoteSuggestionItems[emoteSuggestionIndex];
      const range = findActiveEmoteQuery();
      if (selected && range) {{
        applyEmoteCode(selected.code, range);
      }}
      closeEmoteSuggestions();
      return;
    }}
    if (e.key === 'Escape') {{
      e.preventDefault();
      closeEmoteSuggestions();
      return;
    }}
  }});
  window.addEventListener('resize', syncPlayerLayout);
  document.addEventListener('fullscreenchange', syncPlayerLayout);
  document.addEventListener('click', function(e) {{
    if (!chatForm.contains(e.target)) {{
      closeEmotePicker();
      closeEmoteSuggestions();
      return;
    }}

    if (e.target === chatComposer) {{
      placeComposerCaretAtEnd();
    }}
  }});
  if (typeof MOBILE_LAYOUT_QUERY.addEventListener === 'function') {{
    MOBILE_LAYOUT_QUERY.addEventListener('change', syncPlayerLayout);
  }}

  function buildQualityMenu(levels, currentLevelIdx) {{
    qualityMenu.innerHTML = '';
    var autoItem = document.createElement('div');
    autoItem.className = 'quality-menu-item' + (currentLevelIdx === -1 ? ' active' : '');
    autoItem.innerHTML = '<span>Auto</span>';
    autoItem.onclick = function() {{ setLevel(-1); }};
    qualityMenu.appendChild(autoItem);
    for (var i = 0; i < levels.length; i++) {{
      var level = levels[i];
      var item = document.createElement('div');
      item.className = 'quality-menu-item' + (currentLevelIdx === i ? ' active' : '');
      item.innerHTML = '<span>' + level.height + 'p</span><span class="bitrate">' + formatBitrate(level.bitrate) + '</span>';
      (function(idx) {{ item.onclick = function() {{ setLevel(idx); }}; }})(i);
      qualityMenu.appendChild(item);
    }}
  }}

  function setLevel(levelIdx) {{
    if (!hlsInstance) return;
    hlsInstance.currentLevel = levelIdx;
    userSelectedAuto = (levelIdx === -1);
    qualityMenu.classList.remove('open');
    if (levelIdx === -1) {{
      var level = hlsInstance.levels && hlsInstance.levels[currentPlayingLevelIdx];
      if (level) {{
        qualityBtn.textContent = 'Auto (' + level.height + 'p)';
      }} else {{
        qualityBtn.textContent = 'Auto';
      }}
    }} else if (hlsInstance.levels && hlsInstance.levels[levelIdx]) {{
      qualityBtn.textContent = hlsInstance.levels[levelIdx].height + 'p';
    }}
    buildQualityMenu(hlsInstance.levels || [], levelIdx);
  }}

  qualityBtn.addEventListener('click', function(e) {{
    e.stopPropagation();
    qualityMenu.classList.toggle('open');
  }});
  document.addEventListener('click', function(e) {{
    if (!qualityMenu.contains(e.target) && e.target !== qualityBtn) {{
      qualityMenu.classList.remove('open');
    }}
  }});

  if (Hls.isSupported()) {{
    hlsInstance = new Hls({{
      startPosition: -4,
      lowLatencyMode: true,
      liveSyncDurationCount: 1,
      liveMaxLatencyDurationCount: 3,
      maxLiveSyncPlaybackRate: 1.3,
      maxBufferLength: 30,
      maxMaxBufferLength: 60
    }});
    hlsInstance.currentLevel = -1;
    hlsInstance.on(Hls.Events.MANIFEST_PARSED, function(e, data) {{
      console.log('[HLS] ' + data.levels.length + ' quality levels loaded');
      qualityBtn.textContent = 'Auto';
      buildQualityMenu(data.levels, hlsInstance.currentLevel);
      setTimeout(function() {{
        var timeline = getTimelineModel();
        if (!timeline.seekable || timeline.mode === 'vod') return;
        if (Date.now() < manualSeekSuppressUntilMs) return;
        var lag = Math.max(0, timeline.end - video.currentTime);
        if (lag <= AUTO_LIVE_SEEK_LAG_SECS) return;
        video.currentTime = Math.max(timeline.start, timeline.end - AUTO_LIVE_TARGET_OFFSET_SECS);
        lastAutoLiveSeekAtMs = Date.now();
        liveLagHighStreak = 0;
      }}, 1800);
    }});
    hlsInstance.on(Hls.Events.LEVEL_SWITCHED, function(e, data) {{
      currentPlayingLevelIdx = data.level;
      buildQualityMenu(hlsInstance.levels, data.level);
      var level = hlsInstance.levels && hlsInstance.levels[data.level];
      if (level) {{
        if (userSelectedAuto) {{
          hlsInstance.currentLevel = -1;
          qualityBtn.textContent = 'Auto (' + level.height + 'p)';
        }} else {{
          qualityBtn.textContent = level.height + 'p';
        }}
      }}
    }});
    hlsInstance.on(Hls.Events.ERROR, function(e, data) {{
      console.error('[HLS] ERROR:', data.details, data.fatal ? '(fatal)' : '');
      if (data.fatal) {{
        if (!attemptedRelayFallback) {{
          attemptedRelayFallback = true;
          var fallbackUrl = new URL(window.location.href);
          fallbackUrl.searchParams.set('relay', '1');
          window.location.assign(fallbackUrl.toString());
          return;
        }}
        video.dispatchEvent(new CustomEvent('stream-error', {{ detail: data }}));
      }}
    }});
    hlsInstance.loadSource('{manifest_url}');
    hlsInstance.attachMedia(video);
  }} else if (video.canPlayType('application/vnd.apple.mpegurl')) {{
    video.src = '{manifest_url}';
  }} else {{
    video.dispatchEvent(new CustomEvent('stream-error', {{ detail: {{ type: 'not-supported' }} }}));
  }}

  video.addEventListener('stream-error', function() {{
    document.body.innerHTML = '<div class="error-screen"><div class="error-box"><p>Stream unavailable. The channel may be offline or not accessible.</p></div></div>';
  }});

  syncPlayerLayout();

  async function chatRequest(path, init) {{
    const response = await fetch(path, Object.assign({{ credentials: 'same-origin' }}, init || {{}}));
    if (!response.ok) {{
      let message = 'chat request failed';
      try {{
        const payload = await response.json();
        if (payload && typeof payload.error === 'string') {{
          message = payload.error;
        }}
      }} catch (_) {{}}
      throw new Error(message);
    }}
  }}

  function emoteUrl(emoteId) {{
    return 'https://static-cdn.jtvnw.net/emoticons/v2/' + encodeURIComponent(emoteId) + '/default/dark/' + CHAT_EMOTE_SCALE;
  }}

  function normalizeEmoteCode(code) {{
    if (typeof code !== 'string') return '';
    return code.trim();
  }}

  function scoreEmote(code, query) {{
    const c = code.toLowerCase();
    const q = query.toLowerCase();
    if (c === q) return 0;
    if (c.startsWith(q)) return 1;
    if (c.includes(q)) return 2;
    return 99;
  }}

  function placeComposerCaretAtEnd() {{
    chatComposer.focus();
    const range = document.createRange();
    range.selectNodeContents(chatComposer);
    range.collapse(false);
    const selection = window.getSelection();
    if (!selection) return;
    selection.removeAllRanges();
    selection.addRange(range);
  }}

  function composerTextFromNode(node) {{
    if (!node) return '';
    if (node.nodeType === Node.TEXT_NODE) {{
      return node.textContent || '';
    }}

    if (node.nodeType !== Node.ELEMENT_NODE) {{
      return '';
    }}

    const element = node;
    if (element.tagName === 'IMG') {{
      return element.dataset.code || '';
    }}
    if (element.tagName === 'BR') {{
      return '\n';
    }}

    let out = '';
    for (const child of Array.from(element.childNodes)) {{
      out += composerTextFromNode(child);
    }}
    return out;
  }}

  function getComposerPlainText() {{
    let out = '';
    for (const child of Array.from(chatComposer.childNodes)) {{
      out += composerTextFromNode(child);
    }}
    return out;
  }}

  function buildEmoteMapByCode() {{
    const emotesByCode = new Map();
    for (const item of availableEmotes) {{
      if (typeof item.code === 'string' && typeof item.image_url === 'string' && typeof item.id === 'string') {{
        emotesByCode.set(item.code, item);
      }}
    }}
    return emotesByCode;
  }}

  function renderComposerFromPlainText(text) {{
    const emotesByCode = buildEmoteMapByCode();
    chatComposer.innerHTML = '';

    if (!text) {{
      return;
    }}

    for (const segment of splitMessageSegments(text)) {{
      if (segment.whitespace) {{
        chatComposer.appendChild(document.createTextNode(segment.text));
        continue;
      }}

      const match = emotesByCode.get(segment.text);
      if (!match) {{
        chatComposer.appendChild(document.createTextNode(segment.text));
        continue;
      }}

      const img = document.createElement('img');
      img.className = 'composer-emote';
      img.src = match.image_url;
      img.alt = match.code;
      img.title = match.code;
      img.dataset.code = match.code;
      img.dataset.id = match.id;
      img.loading = 'lazy';
      img.decoding = 'async';
      img.contentEditable = 'false';
      chatComposer.appendChild(img);
    }}
  }}

  function applyPlainTextToComposer(text) {{
    renderComposerFromPlainText(text);
    placeComposerCaretAtEnd();
  }}

  function findActiveEmoteQuery() {{
    const full = getComposerPlainText();
    const match = full.match(/(^|\s):([A-Za-z0-9_]{{2,}})$/);
    if (!match) return null;
    const query = match[2];
    const tokenStart = full.length - query.length - 1;
    return {{ query: query, start: tokenStart, end: full.length }};
  }}

  function applyEmoteCode(code, queryRange) {{
    const safeCode = normalizeEmoteCode(code);
    if (!safeCode) return;

    const full = getComposerPlainText();
    if (queryRange) {{
      const before = full.slice(0, queryRange.start);
      const after = full.slice(queryRange.end);
      applyPlainTextToComposer(before + safeCode + ' ' + after);
      return;
    }}

    applyPlainTextToComposer(full + safeCode + ' ');
  }}

  function splitMessageSegments(input) {{
    const out = [];
    let current = '';
    let currentWhitespace = null;

    for (const ch of input) {{
      const isWhitespace = /\s/.test(ch);
      if (currentWhitespace === null || currentWhitespace === isWhitespace) {{
        current += ch;
        currentWhitespace = isWhitespace;
      }} else {{
        out.push({{ text: current, whitespace: currentWhitespace }});
        current = ch;
        currentWhitespace = isWhitespace;
      }}
    }}

    if (current.length > 0) {{
      out.push({{ text: current, whitespace: currentWhitespace }});
    }}

    return out;
  }}

  function normalizeComposerInput() {{
    let plain = getComposerPlainText();
    plain = plain.replace(/[\r\n]+/g, ' ');
    if (plain.length > 500) {{
      plain = plain.slice(0, 500);
    }}

    renderComposerFromPlainText(plain);
    placeComposerCaretAtEnd();
  }}

  function filteredPickerEmotes() {{
    const term = emoteSearchTerm.trim().toLowerCase();
    if (!term) return availableEmotes;
    return availableEmotes.filter(function(item) {{
      return item.code.toLowerCase().includes(term);
    }});
  }}

  function groupedPickerEmotes() {{
    const filtered = filteredPickerEmotes();
    const groupedMap = new Map();
    for (const item of filtered) {{
      const key = typeof item.group_key === 'string' ? item.group_key : 'global';
      const title = typeof item.group_name === 'string' && item.group_name.trim().length > 0
        ? item.group_name.trim()
        : 'Global';

      if (!groupedMap.has(key)) {{
        groupedMap.set(key, {{ key: key, title: title, items: [] }});
      }}
      groupedMap.get(key).items.push(item);
    }}

    return Array.from(groupedMap.values());
  }}

  function renderEmotePicker() {{
    emoteGroups.innerHTML = '';
    const grouped = groupedPickerEmotes();

    function renderGroup(group) {{
      if (!group.items.length) return;
      const heading = document.createElement('p');
      heading.className = 'emote-group-title';
      heading.textContent = group.title;
      emoteGroups.appendChild(heading);

      const grid = document.createElement('div');
      grid.className = 'emote-grid';
      for (const item of group.items) {{
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'emote-item';
        button.title = item.code;
        button.setAttribute('aria-label', item.code);
        button.addEventListener('click', function() {{
          applyEmoteCode(item.code, null);
          placeComposerCaretAtEnd();
        }});

        const img = document.createElement('img');
        img.src = item.image_url;
        img.alt = item.code;
        img.loading = 'lazy';
        img.decoding = 'async';
        button.appendChild(img);

        grid.appendChild(button);
      }}
      emoteGroups.appendChild(grid);
    }}

    for (const group of grouped) {{
      renderGroup(group);
    }}

    if (!grouped.length) {{
      const empty = document.createElement('div');
      empty.className = 'emote-empty';
      empty.textContent = emoteSearchTerm ? 'No emotes match your search.' : 'No emotes available.';
      emoteGroups.appendChild(empty);
    }}
  }}

  function renderEmoteSuggestions() {{
    emoteSuggestions.innerHTML = '';
    if (!emoteSuggestionsOpen || !emoteSuggestionItems.length) {{
      emoteSuggestions.classList.remove('open');
      return;
    }}

    emoteSuggestions.classList.add('open');
    for (let i = 0; i < emoteSuggestionItems.length; i++) {{
      const item = emoteSuggestionItems[i];
      const row = document.createElement('div');
      row.className = 'emote-suggestion' + (i === emoteSuggestionIndex ? ' active' : '');
      row.addEventListener('mousedown', function(e) {{
        e.preventDefault();
        const range = findActiveEmoteQuery();
        applyEmoteCode(item.code, range);
        closeEmoteSuggestions();
      }});

      const img = document.createElement('img');
      img.src = item.image_url;
      img.alt = item.code;
      img.loading = 'lazy';
      img.decoding = 'async';
      row.appendChild(img);

      const label = document.createElement('span');
      label.textContent = item.code;
      row.appendChild(label);
      emoteSuggestions.appendChild(row);
    }}
  }}

  function closeEmoteSuggestions() {{
    emoteSuggestionsOpen = false;
    emoteSuggestionItems = [];
    emoteSuggestionIndex = 0;
    renderEmoteSuggestions();
  }}

  function refreshEmoteSuggestions() {{
    const active = findActiveEmoteQuery();
    if (!active) {{
      closeEmoteSuggestions();
      return;
    }}

    if (!emotePickerLoaded) {{
      ensureEmotesLoaded()
        .then(function() {{
          refreshEmoteSuggestions();
        }})
        .catch(function(error) {{
          chatStatus.textContent = error && error.message ? error.message : 'Failed to load emotes';
        }});
      return;
    }}

    const q = active.query.toLowerCase();
    const ranked = availableEmotes
      .map(function(item) {{
        return {{ item: item, score: scoreEmote(item.code, q) }};
      }})
      .filter(function(entry) {{ return entry.score < 99; }})
      .sort(function(a, b) {{
        if (a.score !== b.score) return a.score - b.score;
        return a.item.code.toLowerCase().localeCompare(b.item.code.toLowerCase());
      }})
      .slice(0, 10)
      .map(function(entry) {{ return entry.item; }});

    if (!ranked.length) {{
      closeEmoteSuggestions();
      return;
    }}

    emoteSuggestionsOpen = true;
    emoteSuggestionItems = ranked;
    emoteSuggestionIndex = Math.min(emoteSuggestionIndex, ranked.length - 1);
    renderEmoteSuggestions();
  }}

  async function ensureEmotesLoaded() {{
    if (emotePickerLoaded) return;
    const response = await fetch('/api/chat/emotes?channel_login=' + encodeURIComponent(chatChannel), {{
      credentials: 'same-origin'
    }});

    if (!response.ok) {{
      let message = 'failed to load emotes';
      try {{
        const payload = await response.json();
        if (payload && typeof payload.error === 'string') message = payload.error;
      }} catch (_) {{}}
      throw new Error(message);
    }}

    const payload = await response.json();
    const incoming = Array.isArray(payload && payload.emotes) ? payload.emotes : [];
    availableEmotes = incoming
      .filter(function(item) {{
        return item && typeof item.id === 'string' && typeof item.code === 'string' && typeof item.image_url === 'string';
      }})
      .map(function(item) {{
        return {{
          id: item.id,
          code: normalizeEmoteCode(item.code),
          image_url: item.image_url,
          group_key: typeof item.group_key === 'string' ? item.group_key : 'global',
          group_name: typeof item.group_name === 'string' ? item.group_name : 'Global'
        }};
      }})
      .filter(function(item) {{ return item.code.length > 0; }});

    emotePickerLoaded = true;
    renderEmotePicker();
    normalizeComposerInput();
  }}

  async function openEmotePicker() {{
    closeEmoteSuggestions();
    emoteSearchTerm = '';
    emoteSearch.value = '';
    try {{
      await ensureEmotesLoaded();
      renderEmotePicker();
      emotePopup.classList.add('open');
      emotePickerOpen = true;
      emoteSearch.focus();
    }} catch (error) {{
      chatStatus.textContent = error && error.message ? error.message : 'Failed to load emotes';
    }}
  }}

  function closeEmotePicker() {{
    emotePopup.classList.remove('open');
    emotePickerOpen = false;
  }}

  function appendChatEvent(event) {{
    const row = document.createElement('div');
    row.className = 'chat-message' + (event.kind === 'notice' ? ' notice' : '');

    const who = document.createElement('span');
    who.className = 'who';
    who.textContent = event.sender_display_name || event.sender_login || 'system';
    if (event.kind === 'message' && typeof event.sender_color === 'string' && event.sender_color.trim().length > 0) {{
      who.style.color = event.sender_color;
    }}

    row.appendChild(who);

    const body = document.createElement('span');
    const parts = Array.isArray(event.parts) ? event.parts : [];

    if (parts.length > 0) {{
      for (const part of parts) {{
        if (part && part.kind === 'emote' && typeof part.id === 'string') {{
          const img = document.createElement('img');
          img.className = 'chat-emote';
          img.src = typeof part.image_url === 'string' && part.image_url.trim().length > 0
            ? part.image_url
            : emoteUrl(part.id);
          img.alt = typeof part.code === 'string' ? part.code : '';
          img.title = typeof part.code === 'string' ? part.code : '';
          img.loading = 'lazy';
          img.decoding = 'async';
          body.appendChild(img);
          continue;
        }}

        if (part && part.kind === 'text' && typeof part.text === 'string') {{
          body.appendChild(document.createTextNode(part.text));
        }}
      }}
    }} else {{
      body.textContent = event.text || '';
    }}

    row.appendChild(body);
    chatMessages.appendChild(row);
    chatMessages.scrollTop = chatMessages.scrollHeight;
  }}

  async function initChat() {{
    try {{
      await chatRequest('/api/chat/subscribe', {{
        method: 'POST',
        headers: {{ 'content-type': 'application/json' }},
        body: JSON.stringify({{ channel_login: chatChannel }})
      }});

      chatStatus.textContent = 'Connected to #' + chatChannel;
      ensureEmotesLoaded().catch(function() {{
        // Emote picker can still retry on demand.
      }});

      chatEvents = new EventSource('/api/chat/events/' + encodeURIComponent(chatChannel));
      chatEvents.addEventListener('chat', function(raw) {{
        try {{
          const event = JSON.parse(raw.data);
          appendChatEvent(event);
        }} catch (_) {{}}
      }});
      chatEvents.onerror = function() {{
        chatStatus.textContent = 'Chat reconnecting...';
      }};
      chatEvents.onopen = function() {{
        chatStatus.textContent = 'Connected to #' + chatChannel;
      }};
    }} catch (error) {{
      chatStatus.textContent = error && error.message ? error.message : 'Chat unavailable';
      chatComposer.contentEditable = 'false';
      chatSendBtn.disabled = true;
    }}
  }}

  chatForm.addEventListener('submit', async function(e) {{
    e.preventDefault();
    closeEmotePicker();
    closeEmoteSuggestions();
    const text = getComposerPlainText().trim();
    if (!text) return;

    chatSendBtn.disabled = true;
    try {{
      await chatRequest('/api/chat/send', {{
        method: 'POST',
        headers: {{ 'content-type': 'application/json' }},
        body: JSON.stringify({{ channel_login: chatChannel, message: text }})
      }});
      chatComposer.innerHTML = '';
      chatStatus.textContent = 'Connected to #' + chatChannel;
      placeComposerCaretAtEnd();
    }} catch (error) {{
      chatStatus.textContent = error && error.message ? error.message : 'Failed to send message';
    }} finally {{
      chatSendBtn.disabled = false;
    }}
  }});

  window.addEventListener('beforeunload', function() {{
    fetch('/api/chat/subscribe/' + encodeURIComponent(chatChannel), {{
      method: 'DELETE',
      credentials: 'same-origin',
      keepalive: true
    }});
    if (chatEvents) {{
      chatEvents.close();
    }}
    if (typeof MOBILE_LAYOUT_QUERY.removeEventListener === 'function') {{
      MOBILE_LAYOUT_QUERY.removeEventListener('change', syncPlayerLayout);
    }}
    document.removeEventListener('visibilitychange', handleVisibilityChange);
    if (liveStatusRefreshTimer) {{
      clearInterval(liveStatusRefreshTimer);
      liveStatusRefreshTimer = null;
    }}
  }});

  document.addEventListener('visibilitychange', handleVisibilityChange);
  startLiveStatusRefreshLoop();
  initChat();
</script>
</body>
</html>"#,
        channel = channel,
        manifest_url = manifest_url
    )
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
