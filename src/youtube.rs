use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{self, WebAuthConfig},
    config::AppConfig,
    error::AppError,
    invidious::{InvidiousClient, YoutubeChannel, YoutubeVideo, parse_youtube_url},
};

/// State for YouTube routes
#[derive(Debug, Clone)]
pub struct YoutubeState {
    auth: WebAuthConfig,
    invidious: Option<InvidiousClient>,
}

impl YoutubeState {
    pub fn new(auth: WebAuthConfig, config: &AppConfig) -> Self {
        let invidious = config.invidious.as_ref().map(InvidiousClient::new);
        Self { auth, invidious }
    }

    fn require_client(&self) -> Result<&InvidiousClient, AppError> {
        self.invidious
            .as_ref()
            .ok_or(AppError::InvidiousNotConfigured)
    }
}

/// Request to resolve a video
#[derive(Debug, Deserialize)]
pub struct ResolveVideoRequest {
    #[serde(default)]
    video_id: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

/// Resolved video response
#[derive(Debug, Serialize)]
pub struct ResolveVideoResponse {
    pub title: String,
    pub duration: i64,
    pub stream_url: String,
    pub mime_type: String,
    pub is_hls: bool,
    pub expires_at: Option<i64>,
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

/// Build YouTube API routes
pub fn build_routes(auth: WebAuthConfig, config: &AppConfig) -> Router {
    let state = YoutubeState::new(auth.clone(), config);

    Router::new()
        .route("/api/youtube/subscriptions", get(get_subscriptions))
        .route(
            "/api/youtube/channel/{channel_id}/videos",
            get(get_channel_videos),
        )
        .route("/api/youtube/resolve", post(resolve_video))
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

/// Resolve a video ID or URL to a playable stream
async fn resolve_video(
    State(state): State<YoutubeState>,
    Json(req): Json<ResolveVideoRequest>,
) -> Response {
    let client = match state.require_client() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    // Get video ID from request (either direct ID or parsed from URL)
    let video_id = if let Some(id) = req.video_id {
        id
    } else if let Some(url) = req.url {
        match parse_youtube_url(&url) {
            Some(id) => id,
            None => return AppError::InvalidYouTubeUrl.into_response(),
        }
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "either video_id or url is required"
            })),
        )
            .into_response();
    };

    match client.resolve_video(&video_id).await {
        Ok(stream) => {
            let response = ResolveVideoResponse {
                title: stream.title,
                duration: stream.duration,
                stream_url: stream.stream_url,
                mime_type: stream.mime_type,
                is_hls: stream.is_hls,
                expires_at: stream.expires_at,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => e.into_response(),
    }
}
