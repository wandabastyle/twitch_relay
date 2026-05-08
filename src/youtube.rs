use axum::{
    Json, Router,
    body::Body,
    extract::{Path, RawQuery, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

use crate::{
    auth::{self, WebAuthConfig},
    config::AppConfig,
    error::AppError,
    invidious::{
        InvidiousClient, YoutubeChannel, YoutubeChannelInfo, YoutubePlaylist, YoutubeVideo,
        YoutubeVideoMeta, is_valid_video_id,
    },
    youtube_embed::{get_embed, get_embed_config, rewrite_dash_manifest},
    youtube_quality::{
        QualityObservation, QualityObservedResponse, get_video_quality_observed,
        get_video_quality_stream, observe_quality_from_request,
    },
};

/// State for YouTube routes
#[derive(Debug, Clone)]
pub struct YoutubeState {
    invidious: Option<InvidiousClient>,
    invidious_base_url: Option<String>,
    pub(crate) quality_observations: Arc<Mutex<HashMap<String, QualityObservation>>>,
    pub(crate) quality_streams:
        Arc<Mutex<HashMap<String, broadcast::Sender<QualityObservedResponse>>>>,
}

impl YoutubeState {
    pub fn new(_auth: WebAuthConfig, config: &AppConfig) -> Self {
        let invidious = config.invidious.as_ref().map(InvidiousClient::new);
        let invidious_base_url = config.invidious.as_ref().map(|c| c.base_url.clone());
        Self {
            invidious,
            invidious_base_url,
            quality_observations: Arc::new(Mutex::new(HashMap::new())),
            quality_streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn require_client(&self) -> Result<&InvidiousClient, AppError> {
        self.invidious
            .as_ref()
            .ok_or(AppError::InvidiousNotConfigured)
    }

    pub(crate) fn invidious_base_url(&self) -> Option<&str> {
        self.invidious_base_url.as_deref()
    }

    pub(crate) fn invidious_client(&self) -> Option<&InvidiousClient> {
        self.invidious.as_ref()
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

/// Playlist list response
#[derive(Debug, Serialize)]
pub struct PlaylistsResponse {
    pub playlists: Vec<YoutubePlaylist>,
}

/// Playlist videos response
#[derive(Debug, Serialize)]
pub struct PlaylistVideosResponse {
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
        .route(
            "/api/youtube/channel/{channel_id}/info",
            get(get_channel_info),
        )
        .route("/api/youtube/video/{video_id}/meta", get(get_video_meta))
        .route(
            "/api/youtube/video/{video_id}/quality-observed",
            get(get_video_quality_observed),
        )
        .route(
            "/api/youtube/video/{video_id}/quality-stream",
            get(get_video_quality_stream),
        )
        .route("/api/youtube/thumbnail/{video_id}", get(get_thumbnail))
        .route("/api/youtube/proxy/{*path}", get(proxy_video_segment))
        .route("/api/youtube/static/{*path}", get(proxy_static_asset))
        .route("/api/youtube/latest_version", get(drop_latest_version))
        .route(
            "/api/youtube/companion/api/{*path}",
            get(proxy_companion_api),
        )
        .route("/api/youtube/embed/{video_id}", get(get_embed))
        .route("/api/youtube/embed-config", get(get_embed_config))
        .route("/api/youtube/playlists", get(get_playlists))
        .route(
            "/api/youtube/playlist/{playlist_id}/videos",
            get(get_playlist_videos),
        )
        .route(
            "/api/youtube/playlist-thumbnail/{playlist_id}",
            get(get_playlist_thumbnail),
        )
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

/// Get authenticated user's playlists
async fn get_playlists(State(state): State<YoutubeState>) -> Response {
    let client = match state.require_client() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    match client.get_playlists().await {
        Ok(playlists) => (StatusCode::OK, Json(PlaylistsResponse { playlists })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Get videos from a playlist
async fn get_playlist_videos(
    State(state): State<YoutubeState>,
    Path(playlist_id): Path<String>,
) -> Response {
    let client = match state.require_client() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    match client.get_playlist_videos(&playlist_id).await {
        Ok(videos) => (StatusCode::OK, Json(PlaylistVideosResponse { videos })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Proxy playlist thumbnail requests to avoid basic auth popup in browser
async fn get_playlist_thumbnail(
    State(state): State<YoutubeState>,
    Path(playlist_id): Path<String>,
) -> Response {
    use crate::invidious::is_valid_playlist_id;

    // Validate playlist_id format
    if !is_valid_playlist_id(&playlist_id) {
        return (StatusCode::BAD_REQUEST, "invalid playlist_id format").into_response();
    }

    let Some(base_url) = state.invidious_base_url.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    let Some(client) = state.invidious.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    // Fetch playlist to get first video for thumbnail
    let videos = match client.get_playlist_videos(&playlist_id).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, playlist_id = %playlist_id, "Failed to fetch playlist for thumbnail");
            return e.into_response();
        }
    };

    // Use first video's thumbnail
    let Some(first_video) = videos.first() else {
        return (StatusCode::NOT_FOUND, "Playlist has no videos").into_response();
    };

    // Construct thumbnail URL using the video thumbnail proxy
    let invidious_url = format!("{}/vi/{}/mqdefault.jpg", base_url, first_video.video_id);

    proxy_invidious_image(
        client,
        &invidious_url,
        "thumbnail",
        "playlist_id",
        &playlist_id,
    )
    .await
}

/// Shared helper to proxy image requests through Invidious
async fn proxy_invidious_image(
    client: &InvidiousClient,
    image_url: &str,
    log_target: &str,
    _id_field: &str,
    id_value: &str,
) -> Response {
    // Fetch image through InvidiousClient (handles Basic auth + SID cookie)
    let response = match client
        .with_basic_auth(client.http.get(image_url))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, id = %id_value, "Failed to fetch {log_target} from Invidious");
            return (StatusCode::BAD_GATEWAY, "Failed to fetch thumbnail").into_response();
        }
    };

    // Check if request succeeded
    if !response.status().is_success() {
        let status = response.status();
        tracing::warn!(
            status = %status,
            id = %id_value,
            "Invidious returned error for {log_target}"
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
            tracing::error!(error = %e, id = %id_value, "Failed to read {log_target} bytes");
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

    proxy_invidious_image(client, &invidious_url, "thumbnail", "video_id", &video_id).await
}

async fn drop_latest_version() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

async fn proxy_video_segment(
    State(state): State<YoutubeState>,
    Path(path): Path<String>,
    RawQuery(raw_query): RawQuery,
    request_headers: HeaderMap,
) -> Response {
    observe_quality_from_request(&state, raw_query.as_deref(), &request_headers);

    let Some(base_url) = state.invidious_base_url.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    let Some(client) = state.invidious.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    let trimmed_path = path.trim_start_matches('/');
    let mut upstream_url = format!("{}/companion/{}", base_url, trimmed_path);
    if let Some(query_string) = raw_query
        && !query_string.is_empty()
    {
        upstream_url.push('?');
        upstream_url.push_str(&query_string);
    }

    let mut upstream_request = client.http.get(&upstream_url);
    upstream_request = client.with_basic_auth(upstream_request);

    if let Some(range) = request_headers.get(header::RANGE) {
        upstream_request = upstream_request.header(header::RANGE, range);
    }
    if let Some(accept) = request_headers.get(header::ACCEPT) {
        upstream_request = upstream_request.header(header::ACCEPT, accept);
    }
    if let Some(user_agent) = request_headers.get(header::USER_AGENT) {
        upstream_request = upstream_request.header(header::USER_AGENT, user_agent);
    }

    // Request identity encoding to avoid mismatched decode/headers on binary media.
    upstream_request = upstream_request.header(header::ACCEPT_ENCODING, "identity");

    let response = match upstream_request.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, path = %trimmed_path, "Failed to fetch video segment from Invidious");
            return (StatusCode::BAD_GATEWAY, "Failed to fetch video segment").into_response();
        }
    };

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return (
            StatusCode::BAD_GATEWAY,
            "Invidious video segment upstream authentication failed",
        )
            .into_response();
    }
    let upstream_headers = response.headers().clone();
    let content_type = header_value_string(upstream_headers.get(header::CONTENT_TYPE));
    let content_encoding = header_value_string(upstream_headers.get(header::CONTENT_ENCODING));
    let content_length = header_value_string(upstream_headers.get(header::CONTENT_LENGTH));
    let content_range = header_value_string(upstream_headers.get(header::CONTENT_RANGE));
    let accept_ranges = header_value_string(upstream_headers.get(header::ACCEPT_RANGES));
    let range_request = request_headers.get(header::RANGE).is_some();

    tracing::debug!(
        path = %trimmed_path,
        status = %status,
        content_type = %content_type,
        content_encoding = %content_encoding,
        content_length = %content_length,
        content_range = %content_range,
        accept_ranges = %accept_ranges,
        range_request = range_request,
        "Segment proxy upstream response"
    );

    let mut builder = axum::response::Response::builder().status(status);
    for (name, value) in &upstream_headers {
        if should_forward_segment_response_header(name) {
            builder = builder.header(name, value);
        }
    }

    let stream = response.bytes_stream();
    match builder.body(Body::from_stream(stream)) {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!(error = %e, path = %trimmed_path, "Failed to build segment proxy response");
            (
                StatusCode::BAD_GATEWAY,
                "Failed to proxy video segment response",
            )
                .into_response()
        }
    }
}

fn should_forward_segment_response_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_header(name) {
        return false;
    }

    matches!(
        name.as_str().to_ascii_lowercase().as_str(),
        "content-type"
            | "content-length"
            | "content-range"
            | "accept-ranges"
            | "cache-control"
            | "etag"
            | "last-modified"
            | "date"
    )
}

fn is_hop_by_hop_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str().to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn header_value_string(value: Option<&HeaderValue>) -> String {
    value
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<none>")
        .to_string()
}

fn should_forward_static_response_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_header(name) {
        return false;
    }

    matches!(
        name.as_str().to_ascii_lowercase().as_str(),
        "content-type"
            | "content-length"
            | "content-encoding"
            | "cache-control"
            | "etag"
            | "last-modified"
            | "date"
    )
}

async fn proxy_companion_api(
    State(state): State<YoutubeState>,
    Path(path): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Response {
    let Some(base_url) = state.invidious_base_url.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    let Some(client) = state.invidious.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    let trimmed_path = path.trim_start_matches('/');
    let mut upstream_url = format!("{}/companion/api/{}", base_url, trimmed_path);
    if let Some(query_string) = raw_query
        && !query_string.is_empty()
    {
        upstream_url.push('?');
        upstream_url.push_str(&query_string);
    }

    tracing::debug!(path = %trimmed_path, "Proxying companion API request to Invidious");

    let response = match client
        .with_basic_auth(client.http.get(&upstream_url))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, path = %trimmed_path, "Failed to fetch companion API from Invidious");
            return (
                StatusCode::BAD_GATEWAY,
                "Failed to fetch companion API from Invidious",
            )
                .into_response();
        }
    };

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        tracing::warn!(status = %status, path = %trimmed_path, "Invidious companion API upstream authentication failed");
        return (
            StatusCode::BAD_GATEWAY,
            "Invidious companion API upstream authentication failed",
        )
            .into_response();
    }

    if status.is_server_error() {
        tracing::warn!(status = %status, path = %trimmed_path, "Invidious companion API upstream error");
        return (
            StatusCode::BAD_GATEWAY,
            "Failed to fetch companion API from Invidious",
        )
            .into_response();
    }

    if !status.is_success() {
        return (
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            "Companion API request failed",
        )
            .into_response();
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let body = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, path = %trimmed_path, "Failed to read companion API response");
            return (
                StatusCode::BAD_GATEWAY,
                "Failed to read companion API response",
            )
                .into_response();
        }
    };

    if content_type.starts_with("application/dash+xml") {
        let manifest_xml = match String::from_utf8(body.to_vec()) {
            Ok(s) => s,
            Err(_) => {
                return (StatusCode::BAD_GATEWAY, "Invalid DASH manifest encoding").into_response();
            }
        };
        let rewritten = match rewrite_dash_manifest(&manifest_xml) {
            Ok(xml) => xml,
            Err(e) => return e.into_response(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "content-type",
            HeaderValue::from_static("application/dash+xml"),
        );
        return (headers, rewritten).into_response();
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        HeaderValue::from_str(&content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    (headers, body).into_response()
}

async fn proxy_static_asset(
    State(state): State<YoutubeState>,
    Path(path): Path<String>,
    RawQuery(raw_query): RawQuery,
    request_headers: HeaderMap,
) -> Response {
    let Some(base_url) = state.invidious_base_url.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    let Some(client) = state.invidious.as_ref() else {
        return AppError::InvidiousNotConfigured.into_response();
    };

    let trimmed_path = path.trim_start_matches('/');
    let is_allowed_static_path = trimmed_path.starts_with("videojs/")
        || trimmed_path.starts_with("css/")
        || trimmed_path.starts_with("js/")
        || trimmed_path.starts_with("vi/");
    if !is_allowed_static_path {
        return (StatusCode::NOT_FOUND, "static asset path not allowed").into_response();
    }

    let mut upstream_url = format!("{}/{}", base_url, trimmed_path);
    if let Some(query_string) = raw_query
        && !query_string.is_empty()
    {
        upstream_url.push('?');
        upstream_url.push_str(&query_string);
    }

    let mut upstream_request = client.http.get(&upstream_url);
    upstream_request = client.with_basic_auth(upstream_request);
    if let Some(accept) = request_headers.get(header::ACCEPT) {
        upstream_request = upstream_request.header(header::ACCEPT, accept);
    }
    if let Some(user_agent) = request_headers.get(header::USER_AGENT) {
        upstream_request = upstream_request.header(header::USER_AGENT, user_agent);
    }

    let response = match upstream_request.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, path = %trimmed_path, "Failed to fetch static asset from Invidious");
            return (StatusCode::BAD_GATEWAY, "Failed to fetch static asset").into_response();
        }
    };

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return (
            StatusCode::BAD_GATEWAY,
            "Invidious static asset upstream authentication failed",
        )
            .into_response();
    }

    let upstream_headers = response.headers().clone();
    let mut builder = axum::response::Response::builder().status(status);
    for (name, value) in &upstream_headers {
        if should_forward_static_response_header(name) {
            builder = builder.header(name, value);
        }
    }

    let stream = response.bytes_stream();
    match builder.body(Body::from_stream(stream)) {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!(error = %e, path = %trimmed_path, "Failed to build static asset proxy response");
            (
                StatusCode::BAD_GATEWAY,
                "Failed to proxy static asset response",
            )
                .into_response()
        }
    }
}
