use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    auth::{self, WebAuthConfig},
    config::AppConfig,
    error::AppError,
    playback::{PlaybackTicketError, PlaybackTicketService},
    stream_proxy,
};

pub fn build_router(config: &AppConfig) -> Result<Router, AppError> {
    let auth_config = WebAuthConfig::from_app_config(config)?;
    let playback = PlaybackTicketService::new(
        config.playback.channels.clone(),
        config.playback.watch_ticket_ttl_secs,
    );
    let streamlink_path = config
        .playback
        .streamlink_path
        .clone()
        .unwrap_or_else(|| "streamlink".to_string());

    let stream_service = stream_proxy::StreamSessionService::new(streamlink_path.clone());

    let protected_state = ProtectedState {
        auth: auth_config.clone(),
        playback,
        stream: stream_service.clone(),
    };

    let stream_proxy_state =
        stream_proxy::StreamProxyState::new(auth_config.clone(), stream_service.clone());

    let protected_routes = Router::new()
        .route("/api/channels", get(list_channels))
        .route("/api/watch-ticket", post(create_watch_ticket))
        .route("/watch/{ticket}", get(render_watch_page))
        .with_state(protected_state)
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth::require_session_middleware,
        ));

    let stream_routes = Router::new()
        .route(
            "/stream/{stream_id}/manifest",
            get(stream_proxy::proxy_manifest),
        )
        .route(
            "/stream/{stream_id}/segment/{*segment}",
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

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .merge(auth_routes)
        .merge(protected_routes)
        .merge(stream_routes)
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
        Ok(ticket) => {
            let response = WatchTicketResponse {
                watch_url: format!("/watch/{ticket}"),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(PlaybackTicketError::UnknownChannel) => {
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
        Err(PlaybackTicketError::UnknownChannel) => {
            return error_response(StatusCode::BAD_REQUEST, "channel is not in allowlist");
        }
    };

    if let Err(e) = state
        .stream
        .open_session(&ticket, &validated.channel_login, &session_token)
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

    let html = render_stream_page(&validated.channel_login, &ticket);

    Html(html).into_response()
}

fn render_stream_page(channel: &str, stream_id: &str) -> String {
    let manifest_url = format!("/stream/{stream_id}/manifest");
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
  video {{
    flex: 1;
    width: 100%;
    background: #000;
    display: block;
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
<video id="player" controls autoplay></video>
<script src="/static/hls.js"></script>
<script>
  const video = document.getElementById('player');
  if (Hls && video.canPlayType('application/vnd.apple.mpegurl')) {{
    if (Hls.isSupported()) {{
      const hls = new Hls({{
        startPosition: -10,
        maxBufferLength: 30,
        maxMaxBufferLength: 60,
      }});
      hls.loadSource('{manifest_url}');
      hls.attachMedia(video);
      hls.on(Hls.Events.ERROR, function(event, data) {{
        if (data.fatal) {{
          video.dispatchEvent(new CustomEvent('stream-error', {{ detail: data }}));
        }}
      }});
    }} else if (video.canPlayType('application/vnd.apple.mpegurl')) {{
      video.src = '{manifest_url}';
    }}
  }} else {{
    video.dispatchEvent(new CustomEvent('stream-error', {{ detail: {{ type: 'not-supported' }} }}));
  }}

  video.addEventListener('stream-error', function() {{
    document.body.innerHTML = '<div class="error-screen"><div class="error-box"><p>Stream unavailable. The channel may be offline or not accessible.</p></div></div>';
  }});

  video.addEventListener('waiting', function() {{
    video.style.opacity = '0.5';
  }});
  video.addEventListener('playing', function() {{
    video.style.opacity = '1';
  }});
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
