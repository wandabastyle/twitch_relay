use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
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

    let stream_proxy_state = stream_proxy::StreamProxyState::new(stream_service.clone());

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

    let html = render_stream_page(&validated.channel_login, &ticket, &session_token);

    Html(html).into_response()
}

fn render_stream_page(channel: &str, stream_id: &str, session_token: &str) -> String {
    let manifest_url = format!("/stream/{stream_id}/{session_token}/manifest");
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
    flex: 1;
    position: relative;
    background: #000;
    min-height: 200px;
  }}
  video {{
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
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
  }}
  .video-container:not(:hover) .controls-bar {{
    opacity: 0;
    transition: opacity 0.3s;
  }}
  .controls-bar {{
    opacity: 1;
    transition: opacity 0.3s;
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
  .progress-bar {{
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 5px;
    background: rgba(255,255,255,0.2);
    cursor: pointer;
    z-index: 5;
  }}
  .progress-bar:hover {{
    height: 8px;
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
<script src="/static/hls.js"></script>
<script>
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
  const qualityBtn = document.getElementById('qualityBtn');
  const qualityMenu = document.getElementById('qualityMenu');
  const fullscreenBtn = document.getElementById('fullscreenBtn');
  
  let hlsInstance = null;
  let debugVisible = false;
  let controlsTimeout = null;
  
  const debugOverlay = document.createElement('div');
  debugOverlay.style.cssText = 'position:fixed;top:50px;left:10px;background:rgba(0,0,0,0.9);color:#0f0;padding:10px;font-family:monospace;font-size:11px;z-index:99999;display:none;max-width:350px;border-radius:4px;';
  document.body.appendChild(debugOverlay);

  function showControls() {{
    controlsBar.classList.add('visible');
    clearTimeout(controlsTimeout);
    if (!video.paused) {{
      controlsTimeout = setTimeout(hideControls, 3000);
    }}
  }}

  function hideControls() {{
    if (!video.paused) {{
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

  function updateTime() {{
    currentTimeEl.textContent = formatTime(video.currentTime);
    var duration = video.duration || 0;
    durationEl.textContent = formatTime(duration);
    if (duration > 0) {{
      progressPlayed.style.width = (video.currentTime / duration * 100) + '%';
    }}
  }}

  function updateBuffer() {{
    if (video.buffered.length > 0 && video.duration > 0) {{
      var bufferedEnd = video.buffered.end(video.buffered.length - 1);
      progressBuffered.style.width = (bufferedEnd / video.duration * 100) + '%';
    }}
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
    var rect = progressBar.getBoundingClientRect();
    var percent = (e.clientX - rect.left) / rect.width;
    video.currentTime = percent * video.duration;
  }}

  function toggleFullscreen() {{
    if (document.fullscreenElement) {{
      document.exitFullscreen();
    }} else {{
      videoContainer.requestFullscreen();
    }}
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
  volumeSlider.addEventListener('input', function() {{
    video.volume = this.value;
    video.muted = false;
    updateVolumeButton();
  }});
  progressBar.addEventListener('click', seek);
  fullscreenBtn.addEventListener('click', toggleFullscreen);

  video.addEventListener('play', function() {{
    updatePlayButton();
    controlsTimeout = setTimeout(hideControls, 3000);
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
  video.addEventListener('loadedmetadata', function() {{
    durationEl.textContent = formatTime(video.duration);
    updatePlayButton();
    updateVolumeButton();
  }});
  video.addEventListener('volumechange', updateVolumeButton);
  video.addEventListener('waiting', function() {{ video.style.opacity = '0.7'; }});
  video.addEventListener('playing', function() {{ video.style.opacity = '1'; }});
  videoContainer.addEventListener('mousemove', showControls);
  videoContainer.addEventListener('mouseleave', function() {{ if (!video.paused) hideControls(); }});

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
    qualityMenu.classList.remove('open');
    if (levelIdx === -1) {{
      qualityBtn.textContent = 'Auto';
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
    hlsInstance = new Hls({{ startPosition: -10, maxBufferLength: 30, maxMaxBufferLength: 60 }});
    hlsInstance.on(Hls.Events.MANIFEST_PARSED, function(e, data) {{
      console.log('[HLS] ' + data.levels.length + ' quality levels loaded');
      buildQualityMenu(data.levels, hlsInstance.currentLevel);
    }});
    hlsInstance.on(Hls.Events.LEVEL_SWITCHED, function(e, data) {{
      var level = hlsInstance.levels[data.level];
      if (level) {{ qualityBtn.textContent = level.height + 'p'; }}
      buildQualityMenu(hlsInstance.levels, data.level);
    }});
    hlsInstance.on(Hls.Events.ERROR, function(e, data) {{
      console.error('[HLS] ERROR:', data.details, data.fatal ? '(fatal)' : '');
      if (data.fatal) {{ video.dispatchEvent(new CustomEvent('stream-error', {{ detail: data }})); }}
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
