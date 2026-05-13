use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{self, WebAuthConfig},
    channel_catalog::CatalogChannel,
    playback::PlaybackTicketError,
    routes::{APP_VERSION, error::error_response},
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

/// Response DTO for watch session bootstrap data.
#[derive(Debug, Serialize)]
pub struct WatchSessionResponse {
    pub channel: String,
    pub manifest_url: String,
    pub relay: bool,
    pub app_version: &'static str,
}

/// Build watch routes.
pub fn watch_routes(state: ProtectedState, auth_config: WebAuthConfig) -> Router {
    Router::new()
        .route("/api/channels", get(list_channels))
        .route("/api/watch-ticket", post(create_watch_ticket))
        .route("/api/quality-switch", get(quality_switch_handler))
        .route("/api/watch-session/{ticket}", get(watch_session_handler))
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

async fn watch_session_handler(
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
                error_response(
                    StatusCode::BAD_GATEWAY,
                    "stream unavailable. the channel may be offline or not accessible",
                )
            }
            _ => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to open stream session",
            ),
        };
    }

    let force_relay = query.force_relay();
    let relay_suffix = if force_relay { "?relay=1" } else { "" };
    let response = WatchSessionResponse {
        channel: validated.channel_login,
        manifest_url: format!("/stream/{ticket}/{session_token}/manifest{relay_suffix}"),
        relay: force_relay,
        app_version: APP_VERSION,
    };

    (StatusCode::OK, Json(response)).into_response()
}
