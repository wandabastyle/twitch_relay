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

use crate::{
    auth::{self, WebAuthConfig},
    channel_catalog::CatalogChannel,
    playback::PlaybackTicketError,
    routes::APP_VERSION,
    stream_proxy::{self, RelayQuery},
};

/// State for watch routes (shared with channels).
/// Imported from crate::app since ProtectedState is defined there.
pub use crate::app::ProtectedState;

/// Response DTO for channel list.
#[derive(Debug, Serialize)]
pub struct ChannelsResponse {
    pub channels: Vec<ChannelItem>,
}

/// Individual channel item in the response.
#[derive(Debug, Serialize)]
pub struct ChannelItem {
    pub login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub source: String,
    pub removable: bool,
}

/// Request DTO for creating a watch ticket.
#[derive(Debug, Deserialize)]
pub struct WatchTicketRequest {
    pub channel_login: String,
}

/// Response DTO for watch ticket creation.
#[derive(Debug, Serialize)]
pub struct WatchTicketResponse {
    pub watch_url: String,
}

/// Query parameters for quality switch.
#[derive(Debug, Deserialize)]
pub struct QualitySwitchQuery {
    pub channel_login: String,
    pub quality: String,
}

/// Response DTO for quality switch.
#[derive(Debug, Serialize)]
pub struct QualitySwitchResponse {
    pub watch_url: String,
    pub quality: String,
}

/// Build watch routes.
pub fn watch_routes(state: ProtectedState, auth_config: WebAuthConfig) -> Router {
    Router::new()
        .route("/api/channels", get(list_channels))
        .route("/api/watch-ticket", post(create_watch_ticket))
        .route("/api/quality-switch", get(quality_switch_handler))
        .route("/watch/{ticket}", get(render_watch_page))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            auth_config,
            auth::require_session_middleware,
        ))
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
    Query(query): Query<RelayQuery>,
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
    let template = include_str!("../templates/watch.html");
    let mut watch_config = serde_json::json!({
        "channel": channel,
        "manifestUrl": manifest_url,
        "relay": force_relay,
    })
    .to_string();
    watch_config = watch_config.replace("</script>", "<\\/script>");
    let bootstrap = format!("window.__WATCH_CONFIG__ = {watch_config};");

    template
        .replace("__CHANNEL__", channel)
        .replace("__APP_VERSION__", APP_VERSION)
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
  /* Tokyo Night Moon theme tokens */
  :root {{
    --bg: #1e2030;
    --bg-soft: #222436;
    --surface: #2f334d;
    --fg: #c8d3f5;
    --muted: #a9b8e8;
    --border: #444a73;
  }}
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    background: var(--bg);
    color: var(--fg);
    font-family: system-ui, -apple-system, 'Segoe UI', sans-serif;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }}
  header {{
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
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
    background: rgba(47, 51, 77, 0.95);
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.5rem;
  }}
  .error-box p {{
    color: var(--muted);
    line-height: 1.6;
  }}
</style>
</head>
<body>
<header>
  <strong>{channel}</strong>
  <span>via Twitch Relay · v{version}</span>
</header>
<div class="error-screen">
  <div class="error-box">
    <p>{message}</p>
  </div>
</div>
</body>
</html>"#,
        channel = channel,
        message = message,
        version = APP_VERSION
    );

    (StatusCode::OK, Html(html)).into_response()
}

/// Minimal shared error helper for route modules.
fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
