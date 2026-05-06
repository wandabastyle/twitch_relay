use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{self, WebAuthConfig},
    config::AppConfig,
    error::AppError,
    invidious::{
        InvidiousClient, YoutubeChannel, YoutubeChannelInfo, YoutubeVideo, YoutubeVideoMeta,
    },
};

/// State for YouTube routes
#[derive(Debug, Clone)]
pub struct YoutubeState {
    invidious: Option<InvidiousClient>,
    invidious_base_url: Option<String>,
    basic_auth_user: Option<String>,
    basic_auth_password: Option<String>,
}

impl YoutubeState {
    pub fn new(_auth: WebAuthConfig, config: &AppConfig) -> Self {
        let invidious = config.invidious.as_ref().map(InvidiousClient::new);
        let invidious_base_url = config.invidious.as_ref().map(|c| c.base_url.clone());
        let basic_auth_user = config
            .invidious
            .as_ref()
            .and_then(|c| c.basic_auth_user.clone());
        let basic_auth_password = config
            .invidious
            .as_ref()
            .and_then(|c| c.basic_auth_password.clone());
        Self {
            invidious,
            invidious_base_url,
            basic_auth_user,
            basic_auth_password,
        }
    }

    fn require_client(&self) -> Result<&InvidiousClient, AppError> {
        self.invidious
            .as_ref()
            .ok_or(AppError::InvidiousNotConfigured)
    }
}

/// Channel list response
#[derive(Debug, Serialize)]
pub struct SubscriptionsResponse {
    pub channels: Vec<YoutubeChannel>,
}

/// Channel videos query parameters
#[derive(Debug, Deserialize)]
pub struct ChannelVideosQuery {
    #[serde(default = "default_max_results")]
    max_results: u32,
}

fn default_max_results() -> u32 {
    20
}

/// Video list response
#[derive(Debug, Serialize)]
pub struct ChannelVideosResponse {
    pub videos: Vec<YoutubeVideo>,
}

/// Channel info response
#[derive(Debug, Serialize)]
pub struct ChannelInfoResponse {
    pub channel: YoutubeChannelInfo,
}

/// Video metadata response
#[derive(Debug, Serialize)]
pub struct VideoMetaResponse {
    pub video: YoutubeVideoMeta,
}

/// Frontend embed configuration
#[derive(Debug, Serialize)]
pub struct EmbedConfigResponse {
    pub invidious_base_url: String,
    pub defaults: EmbedDefaults,
    pub referrer_policy: String,
    pub basic_auth_user: Option<String>,
    pub basic_auth_password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmbedDefaults {
    pub autoplay: u8,
    pub quality: String,
    pub quality_dash: String,
}

/// Build YouTube API routes
pub fn build_routes(auth: WebAuthConfig, config: &AppConfig) -> Router {
    let state = YoutubeState::new(auth.clone(), config);

    Router::new()
        .route("/api/youtube/subscriptions", get(get_subscriptions))
        .route(
            "/api/youtube/channel/{channel_id}/videos",
            get(get_channel_videos),
        )
        .route(
            "/api/youtube/channel/{channel_id}/info",
            get(get_channel_info),
        )
        .route("/api/youtube/video/{video_id}/meta", get(get_video_meta))
        .route("/api/youtube/thumbnail/{video_id}", get(get_thumbnail))
        .route("/api/youtube/embed-config", get(get_embed_config))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            auth,
            auth::require_session_middleware,
        ))
}

/// Get authenticated user's subscriptions
async fn get_subscriptions(State(state): State<YoutubeState>) -> Response {
    let client = match state.require_client() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    match client.get_subscriptions().await {
        Ok(channels) => (StatusCode::OK, Json(SubscriptionsResponse { channels })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Get latest videos for a channel
async fn get_channel_videos(
    State(state): State<YoutubeState>,
    Path(channel_id): Path<String>,
    query: axum::extract::Query<ChannelVideosQuery>,
) -> Response {
    let client = match state.require_client() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    match client
        .get_channel_videos(&channel_id, Some(query.max_results))
        .await
    {
        Ok(videos) => (StatusCode::OK, Json(ChannelVideosResponse { videos })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Get channel info including description
async fn get_channel_info(
    State(state): State<YoutubeState>,
    Path(channel_id): Path<String>,
) -> Response {
    let client = match state.require_client() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    match client.get_channel_info(&channel_id).await {
        Ok(channel) => (StatusCode::OK, Json(ChannelInfoResponse { channel })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Get video metadata for watch page.
async fn get_video_meta(
    State(state): State<YoutubeState>,
    Path(video_id): Path<String>,
) -> Response {
    let client = match state.require_client() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    match client.get_video_meta(&video_id).await {
        Ok(video) => (StatusCode::OK, Json(VideoMetaResponse { video })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Get frontend embed configuration.
async fn get_embed_config(State(state): State<YoutubeState>) -> Response {
    if let Err(e) = state.require_client() {
        return e.into_response();
    }

    let Some(base_url) = state.invidious_base_url.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    (
        StatusCode::OK,
        Json(EmbedConfigResponse {
            invidious_base_url: base_url.clone(),
            defaults: EmbedDefaults {
                autoplay: 1,
                quality: "dash".to_string(),
                quality_dash: "auto".to_string(),
            },
            referrer_policy: "no-referrer".to_string(),
            basic_auth_user: state.basic_auth_user.clone(),
            basic_auth_password: state.basic_auth_password.clone(),
        }),
    )
        .into_response()
}

/// Validate YouTube video ID format (11 alphanumeric chars)
fn is_valid_video_id(video_id: &str) -> bool {
    video_id.len() == 11
        && video_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Proxy thumbnail requests to avoid basic auth popup in browser
async fn get_thumbnail(
    State(state): State<YoutubeState>,
    Path(video_id): Path<String>,
) -> Response {
    // Validate video_id format
    if !is_valid_video_id(&video_id) {
        return (StatusCode::BAD_REQUEST, "invalid video_id format").into_response();
    }

    let Some(base_url) = state.invidious_base_url.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    let Some(client) = state.invidious.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    // Construct Invidious thumbnail URL
    let invidious_url = format!("{}/vi/{}/hqdefault.jpg", base_url, video_id);

    // Fetch thumbnail through InvidiousClient (handles Basic auth + SID cookie)
    let response = match client.http.get(&invidious_url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, video_id = %video_id, "Failed to fetch thumbnail from Invidious");
            return (StatusCode::BAD_GATEWAY, "Failed to fetch thumbnail").into_response();
        }
    };

    // Check if request succeeded
    if !response.status().is_success() {
        let status = response.status();
        tracing::warn!(
            status = %status,
            video_id = %video_id,
            "Invidious returned error for thumbnail"
        );
        return (
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            "Thumbnail not available",
        )
            .into_response();
    }

    // Get content type from response, default to image/jpeg
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .unwrap_or_else(|| "image/jpeg".to_string());

    // Get image bytes
    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, video_id = %video_id, "Failed to read thumbnail bytes");
            return (StatusCode::BAD_GATEWAY, "Failed to read thumbnail").into_response();
        }
    };

    // Build response with cache headers
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        HeaderValue::from_str(&content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("image/jpeg")),
    );
    // Cache for 24 hours since thumbnails rarely change
    headers.insert(
        "cache-control",
        HeaderValue::from_static("public, max-age=86400"),
    );

    (headers, bytes).into_response()
}
