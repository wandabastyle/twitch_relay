use axum::{
    Json, Router,
    body::Body,
    extract::{Path, RawQuery, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    middleware,
    response::{
        IntoResponse, Response,
        sse::{Event as SseEvent, KeepAlive, Sse},
    },
    routing::get,
};
use futures_util::StreamExt;
use quick_xml::{Reader, Writer, events::Event as XmlEvent};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    auth::{self, WebAuthConfig},
    config::AppConfig,
    error::AppError,
    invidious::{
        InvidiousClient, YoutubeChannel, YoutubeChannelInfo, YoutubePlaylist, YoutubeVideo,
        YoutubeVideoMeta, is_valid_video_id,
    },
};

/// State for YouTube routes
#[derive(Debug, Clone)]
pub struct YoutubeState {
    invidious: Option<InvidiousClient>,
    invidious_base_url: Option<String>,
    quality_observations: Arc<Mutex<HashMap<String, QualityObservation>>>,
    quality_streams: Arc<Mutex<HashMap<String, broadcast::Sender<QualityObservedResponse>>>>,
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

#[derive(Debug, Clone)]
struct QualityObservation {
    current_itag: String,
    current_quality_label: String,
    counts: HashMap<String, u64>,
    last_updated_unix_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QualityObservedResponse {
    pub current_quality_label: Option<String>,
    pub seen_itags: Vec<QualityObservedItag>,
    pub last_updated_unix_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QualityObservedItag {
    pub quality_label: String,
    pub count: u64,
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

/// Frontend embed configuration
#[derive(Debug, Serialize)]
pub struct EmbedConfigResponse {
    pub invidious_base_url: String,
    pub defaults: EmbedDefaults,
    pub referrer_policy: String,
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

async fn get_video_quality_observed(
    State(state): State<YoutubeState>,
    Path(video_id): Path<String>,
) -> Response {
    if !is_valid_video_id(&video_id) {
        return (StatusCode::BAD_REQUEST, "invalid video_id format").into_response();
    }

    let observations = match state.quality_observations.lock() {
        Ok(o) => o,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "quality observation store unavailable",
            )
                .into_response();
        }
    };

    let Some(obs) = observations.get(&video_id) else {
        return (
            StatusCode::OK,
            Json(QualityObservedResponse {
                current_quality_label: None,
                seen_itags: Vec::new(),
                last_updated_unix_secs: None,
            }),
        )
            .into_response();
    };

    (StatusCode::OK, Json(build_quality_observed_response(obs))).into_response()
}

async fn get_video_quality_stream(
    State(state): State<YoutubeState>,
    Path(video_id): Path<String>,
) -> Response {
    if !is_valid_video_id(&video_id) {
        return (StatusCode::BAD_REQUEST, "invalid video_id format").into_response();
    }

    let sender = get_or_create_quality_sender(&state, &video_id);
    let receiver = sender.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|msg| async move {
        match msg {
            Ok(update) => serde_json::to_string(&update).ok().map(|json| {
                Ok::<SseEvent, std::convert::Infallible>(SseEvent::default().data(json))
            }),
            Err(_) => None,
        }
    });

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(25))
                .text("hb"),
        )
        .into_response()
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
        }),
    )
        .into_response()
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

/// Query parameters for embed proxy
#[derive(Debug, Deserialize)]
struct EmbedQuery {
    autoplay: Option<String>,
    quality: Option<String>,
    quality_dash: Option<String>,
}

static HTML_ATTR_DOUBLE_RE: OnceLock<Regex> = OnceLock::new();
static HTML_ATTR_SINGLE_RE: OnceLock<Regex> = OnceLock::new();
static COMPANION_ROOT_DOUBLE_RE: OnceLock<Regex> = OnceLock::new();
static COMPANION_ROOT_SINGLE_RE: OnceLock<Regex> = OnceLock::new();
static STATIC_ROOT_DOUBLE_RE: OnceLock<Regex> = OnceLock::new();
static STATIC_ROOT_SINGLE_RE: OnceLock<Regex> = OnceLock::new();
static EMBED_REFERER_VIDEO_ID_RE: OnceLock<Regex> = OnceLock::new();

const COMPANION_PROXY_PREFIX: &str = "/api/youtube/companion/api/";
const VIDEO_PROXY_PREFIX: &str = "/api/youtube/proxy/";
const LATEST_VERSION_PROXY_PATH: &str = "/api/youtube/latest_version";
const STATIC_PROXY_PREFIX: &str = "/api/youtube/static/";

/// Rewrite root-relative URLs in HTML to absolute URLs.
/// Matches src="/...", href="/...", poster="/..." and prepends the base URL.
/// Excludes protocol-relative URLs (starting with //) and fragment-only URLs.
fn rewrite_html_urls(html: &str, base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');

    let re_double = HTML_ATTR_DOUBLE_RE.get_or_init(|| {
        // Match "/path" where path is not empty and not starting with //
        // The pattern requires at least one char after / that is not another /
        Regex::new(r#"(src|href|poster)="(/[^"/][^"]*?)""#)
            .expect("valid double-quoted HTML attr regex")
    });

    let re_single = HTML_ATTR_SINGLE_RE.get_or_init(|| {
        // Match '/path' where path is not empty and not starting with //
        Regex::new(r#"(src|href|poster)='(/[^'/][^']*?)'"#)
            .expect("valid single-quoted HTML attr regex")
    });

    let html = re_double.replace_all(html, |caps: &Captures| {
        let attr = &caps[1];
        let path = &caps[2];
        format!(r#"{}="{}{}""#, attr, base, path)
    });

    let html = re_single
        .replace_all(&html, |caps: &Captures| {
            let attr = &caps[1];
            let path = &caps[2];
            format!("{}='{}{}'", attr, base, path)
        })
        .into_owned();

    rewrite_companion_api_urls(&html, base)
}

fn rewrite_companion_api_urls(html: &str, base_url: &str) -> String {
    let mut rewritten = html.to_string();

    let absolute_prefix = format!("{}/companion/api/", base_url);
    rewritten = rewritten.replace(&absolute_prefix, COMPANION_PROXY_PREFIX);
    let absolute_latest_version = format!("{}/companion/latest_version", base_url);
    rewritten = rewritten.replace(&absolute_latest_version, LATEST_VERSION_PROXY_PATH);
    rewritten = rewritten.replace("/companion/latest_version", LATEST_VERSION_PROXY_PATH);
    let absolute_videojs = format!("{}/videojs/", base_url);
    rewritten = rewritten.replace(&absolute_videojs, &format!("{STATIC_PROXY_PREFIX}videojs/"));
    let absolute_css = format!("{}/css/", base_url);
    rewritten = rewritten.replace(&absolute_css, &format!("{STATIC_PROXY_PREFIX}css/"));
    let absolute_js = format!("{}/js/", base_url);
    rewritten = rewritten.replace(&absolute_js, &format!("{STATIC_PROXY_PREFIX}js/"));
    let absolute_vi = format!("{}/vi/", base_url);
    rewritten = rewritten.replace(&absolute_vi, &format!("{STATIC_PROXY_PREFIX}vi/"));

    let root_double = COMPANION_ROOT_DOUBLE_RE.get_or_init(|| {
        Regex::new(r#"\"/companion/api/"#).expect("valid double-quoted companion root regex")
    });
    let root_single = COMPANION_ROOT_SINGLE_RE.get_or_init(|| {
        Regex::new(r#"'/companion/api/"#).expect("valid single-quoted companion root regex")
    });

    let rewritten = root_double
        .replace_all(&rewritten, format!("\"{}", COMPANION_PROXY_PREFIX))
        .into_owned();
    let rewritten = STATIC_ROOT_DOUBLE_RE
        .get_or_init(|| {
            Regex::new(r#"\"/(videojs|css|js|vi)/"#).expect("valid double-quoted static root regex")
        })
        .replace_all(&rewritten, format!("\"{STATIC_PROXY_PREFIX}$1/"))
        .into_owned();

    let rewritten = root_single
        .replace_all(&rewritten, format!("'{}", COMPANION_PROXY_PREFIX))
        .into_owned();

    STATIC_ROOT_SINGLE_RE
        .get_or_init(|| {
            Regex::new(r#"'/(videojs|css|js|vi)/"#).expect("valid single-quoted static root regex")
        })
        .replace_all(&rewritten, format!("'{STATIC_PROXY_PREFIX}$1/"))
        .into_owned()
}

async fn drop_latest_version() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

fn rewrite_dash_manifest(manifest_xml: &str) -> Result<String, AppError> {
    let mut reader = Reader::from_str(manifest_xml);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Vec::with_capacity(manifest_xml.len()));
    let mut in_base_url = false;

    loop {
        match reader.read_event() {
            Ok(XmlEvent::Start(e)) => {
                in_base_url = e.name().as_ref() == b"BaseURL";
                writer
                    .write_event(XmlEvent::Start(e.into_owned()))
                    .map_err(|_| AppError::InvidiousBadResponse)?;
            }
            Ok(XmlEvent::End(e)) => {
                if e.name().as_ref() == b"BaseURL" {
                    in_base_url = false;
                }
                writer
                    .write_event(XmlEvent::End(e.into_owned()))
                    .map_err(|_| AppError::InvidiousBadResponse)?;
            }
            Ok(XmlEvent::Text(e)) => {
                if in_base_url {
                    let original = e
                        .decode()
                        .map_err(|_| AppError::InvidiousBadResponse)?
                        .into_owned();
                    let rewritten = if let Some(rest) = original.strip_prefix("/companion/") {
                        format!("{VIDEO_PROXY_PREFIX}{rest}")
                    } else {
                        original
                    };
                    writer
                        .write_event(XmlEvent::Text(quick_xml::events::BytesText::new(
                            &rewritten,
                        )))
                        .map_err(|_| AppError::InvidiousBadResponse)?;
                } else {
                    writer
                        .write_event(XmlEvent::Text(e.into_owned()))
                        .map_err(|_| AppError::InvidiousBadResponse)?;
                }
            }
            Ok(XmlEvent::Eof) => break,
            Ok(e) => {
                writer
                    .write_event(e.into_owned())
                    .map_err(|_| AppError::InvidiousBadResponse)?;
            }
            Err(_) => return Err(AppError::InvidiousBadResponse),
        }
    }

    String::from_utf8(writer.into_inner()).map_err(|_| AppError::InvidiousBadResponse)
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

fn build_quality_observed_response(obs: &QualityObservation) -> QualityObservedResponse {
    let mut seen_itags: Vec<QualityObservedItag> = obs
        .counts
        .iter()
        .map(|(itag, count)| QualityObservedItag {
            quality_label: itag_quality_label(itag).to_string(),
            count: *count,
        })
        .collect();
    seen_itags.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.quality_label.cmp(&b.quality_label))
    });

    QualityObservedResponse {
        current_quality_label: Some(obs.current_quality_label.clone()),
        seen_itags,
        last_updated_unix_secs: Some(obs.last_updated_unix_secs),
    }
}

fn is_video_itag(itag: &str) -> bool {
    matches!(
        itag,
        "160"
            | "278"
            | "394"
            | "133"
            | "242"
            | "395"
            | "134"
            | "243"
            | "396"
            | "18"
            | "135"
            | "244"
            | "397"
            | "136"
            | "247"
            | "398"
            | "22"
            | "298"
            | "137"
            | "248"
            | "399"
            | "299"
    )
}

fn get_or_create_quality_sender(
    state: &YoutubeState,
    video_id: &str,
) -> broadcast::Sender<QualityObservedResponse> {
    let mut streams = match state.quality_streams.lock() {
        Ok(s) => s,
        Err(poisoned) => poisoned.into_inner(),
    };

    streams
        .entry(video_id.to_string())
        .or_insert_with(|| {
            let (tx, _) = broadcast::channel(128);
            tx
        })
        .clone()
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn decode_query_value(value: &str) -> Option<String> {
    urlencoding::decode(value).ok().map(|v| v.into_owned())
}

fn query_param(raw_query: Option<&str>, key: &str) -> Option<String> {
    let query = raw_query?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let k = parts.next().unwrap_or_default();
        if k != key {
            continue;
        }
        let value = parts.next().unwrap_or_default();
        return decode_query_value(value).or_else(|| Some(value.to_string()));
    }
    None
}

fn video_id_from_embed_referer(request_headers: &HeaderMap) -> Option<String> {
    let referer = request_headers.get(header::REFERER)?.to_str().ok()?;
    let re = EMBED_REFERER_VIDEO_ID_RE.get_or_init(|| {
        Regex::new(r"/api/youtube/embed/([A-Za-z0-9_-]{11})(?:[/?#]|$)")
            .expect("valid embed referer video_id regex")
    });
    let captures = re.captures(referer)?;
    let video_id = captures.get(1)?.as_str();
    if is_valid_video_id(video_id) {
        Some(video_id.to_string())
    } else {
        None
    }
}

fn itag_quality_label(itag: &str) -> &'static str {
    match itag {
        "137" => "1080p (avc1)",
        "248" => "1080p (vp9)",
        "399" => "1080p (av1)",
        "299" => "1080p60 (avc1)",
        "136" => "720p (avc1)",
        "247" => "720p (vp9)",
        "398" => "720p (av1)",
        "22" => "720p (avc1)",
        "298" => "720p60",
        "135" => "480p (avc1)",
        "244" => "480p (vp9)",
        "397" => "480p (av1)",
        "18" => "360p (avc1)",
        "134" => "360p (avc1)",
        "243" => "360p (vp9)",
        "396" => "360p (av1)",
        "133" => "240p (avc1)",
        "242" => "240p (vp9)",
        "395" => "240p (av1)",
        "160" => "144p (avc1)",
        "278" => "144p (vp9)",
        "394" => "144p (av1)",
        _ => "unknown",
    }
}

fn observe_quality_from_request(
    state: &YoutubeState,
    raw_query: Option<&str>,
    request_headers: &HeaderMap,
) {
    let Some(itag) = query_param(raw_query, "itag") else {
        return;
    };
    if !is_video_itag(&itag) {
        return;
    }
    let video_id = if let Some(id) = video_id_from_embed_referer(request_headers) {
        id
    } else if let Some(raw_video_id) = query_param(raw_query, "id") {
        let base = raw_video_id.split('.').next().unwrap_or(&raw_video_id);
        if is_valid_video_id(base) {
            base.to_string()
        } else {
            return;
        }
    } else {
        return;
    };

    let mut observations = match state.quality_observations.lock() {
        Ok(o) => o,
        Err(_) => return,
    };

    let now = now_unix_secs();
    observations.retain(|_, v| now.saturating_sub(v.last_updated_unix_secs) <= 3600);

    let quality_label = itag_quality_label(&itag).to_string();
    let obs = observations
        .entry(video_id.clone())
        .or_insert_with(|| QualityObservation {
            current_itag: itag.clone(),
            current_quality_label: quality_label.clone(),
            counts: HashMap::new(),
            last_updated_unix_secs: now,
        });

    obs.current_itag = itag.clone();
    obs.current_quality_label = quality_label;
    obs.last_updated_unix_secs = now;
    *obs.counts.entry(itag).or_insert(0) += 1;

    let snapshot = build_quality_observed_response(obs);
    drop(observations);

    let sender = get_or_create_quality_sender(state, &video_id);
    let _ = sender.send(snapshot);
}

fn inject_quality_indicator_script(html: &str, video_id: &str) -> String {
    let mut script = r#"<script>(function(){const videoId='__VIDEO_ID__';const observedEndpoint=`/api/youtube/video/${encodeURIComponent(videoId)}/quality-observed`;const streamEndpoint=`/api/youtube/video/${encodeURIComponent(videoId)}/quality-stream`;const badgeId='relay-quality-indicator';let pollTimer=null;let pollDelayMs=2000;let eventSource=null;let sseRetryTimer=null;function ensureBadge(){const controlBar=document.querySelector('.vjs-control-bar');if(!controlBar)return null;let badge=document.getElementById(badgeId);if(!badge){badge=document.createElement('div');badge.id=badgeId;badge.style.marginLeft='auto';badge.style.padding='0 0.65rem';badge.style.display='flex';badge.style.alignItems='center';badge.style.color='#fff';badge.style.fontWeight='normal';badge.style.fontStyle='normal';badge.style.fontFamily='Arial, Helvetica, sans-serif';badge.style.wordBreak='initial';badge.style.cursor='none';badge.style.visibility='visible';badge.style.wordWrap='break-word';badge.style.textAlign='center';badge.style.fontSize='1em';badge.style.lineHeight='3em';badge.style.boxSizing='inherit';badge.style.whiteSpace='nowrap';controlBar.appendChild(badge);}return badge;}function setBadgeText(data){const badge=ensureBadge();if(!badge)return;if(data&&data.current_quality_label){badge.textContent=`Quality: ${data.current_quality_label}`;}else{badge.textContent='Quality: detecting...';}}async function refreshObserved(){try{const res=await fetch(observedEndpoint,{credentials:'same-origin'});if(!res.ok){setBadgeText(null);return;}const data=await res.json();setBadgeText(data);}catch(_){setBadgeText(null);}}function stopPolling(){if(pollTimer!==null){window.clearTimeout(pollTimer);pollTimer=null;}}function schedulePolling(){stopPolling();pollTimer=window.setTimeout(async()=>{await refreshObserved();pollDelayMs=Math.min(Math.round(pollDelayMs*1.5),10000);schedulePolling();},pollDelayMs);}function startPollingFallback(){if(pollTimer!==null)return;pollDelayMs=2000;schedulePolling();}function stopSseRetry(){if(sseRetryTimer!==null){window.clearTimeout(sseRetryTimer);sseRetryTimer=null;}}function scheduleSseReconnect(){if(sseRetryTimer!==null)return;sseRetryTimer=window.setTimeout(()=>{sseRetryTimer=null;startSse();},5000);}function startSse(){if(eventSource!==null)return;try{eventSource=new EventSource(streamEndpoint);}catch(_){startPollingFallback();scheduleSseReconnect();return;}eventSource.onmessage=(event)=>{if(!event||typeof event.data!=='string')return;try{const data=JSON.parse(event.data);setBadgeText(data);stopPolling();pollDelayMs=2000;}catch(_){}};eventSource.onerror=()=>{if(eventSource!==null){eventSource.close();eventSource=null;}startPollingFallback();scheduleSseReconnect();};}function shutdown(){stopPolling();stopSseRetry();if(eventSource!==null){eventSource.close();eventSource=null;}}function boot(){refreshObserved();startSse();window.addEventListener('beforeunload',shutdown,{once:true});}if(document.readyState==='loading'){document.addEventListener('DOMContentLoaded',boot,{once:true});}else{boot();}})();</script>"#.to_string();
    script = script.replace("__VIDEO_ID__", video_id);

    if let Some(idx) = html.rfind("</body>") {
        let mut out = String::with_capacity(html.len() + script.len());
        out.push_str(&html[..idx]);
        out.push_str(&script);
        out.push_str(&html[idx..]);
        return out;
    }

    format!("{html}{script}")
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

/// Proxy embed requests to avoid basic auth popup in browser.
/// Fetches the Invidious embed page with backend authentication.
async fn get_embed(
    State(state): State<YoutubeState>,
    Path(video_id): Path<String>,
    query: axum::extract::Query<EmbedQuery>,
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

    // Build upstream Invidious embed URL with whitelisted query parameters
    let mut upstream_url = format!("{}/embed/{}", base_url, video_id);
    let mut params = Vec::new();

    if let Some(autoplay) = &query.autoplay {
        params.push(format!("autoplay={}", urlencoding::encode(autoplay)));
    }
    if let Some(quality) = &query.quality {
        params.push(format!("quality={}", urlencoding::encode(quality)));
    }
    if let Some(quality_dash) = &query.quality_dash {
        params.push(format!(
            "quality_dash={}",
            urlencoding::encode(quality_dash)
        ));
    }

    if !params.is_empty() {
        upstream_url.push('?');
        upstream_url.push_str(&params.join("&"));
    }

    // Log request (without credentials)
    tracing::debug!(video_id = %video_id, "Proxying embed request to Invidious");

    // Fetch embed page through InvidiousClient (handles Basic auth + SID cookie)
    let response = match client
        .with_basic_auth(client.http.get(&upstream_url))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, video_id = %video_id, "Failed to fetch embed from Invidious");
            return (
                StatusCode::BAD_GATEWAY,
                "Failed to fetch embed from Invidious",
            )
                .into_response();
        }
    };

    // Handle upstream response status codes
    let status = response.status();

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        tracing::warn!(
            status = %status,
            video_id = %video_id,
            "Invidious embed upstream authentication failed"
        );
        return (
            StatusCode::BAD_GATEWAY,
            "Invidious embed upstream authentication failed",
        )
            .into_response();
    }

    if status == StatusCode::NOT_FOUND {
        return (StatusCode::NOT_FOUND, "Embed not found").into_response();
    }

    if !status.is_success() {
        tracing::warn!(
            status = %status,
            video_id = %video_id,
            "Invidious returned error for embed"
        );
        return (
            StatusCode::BAD_GATEWAY,
            "Failed to fetch embed from Invidious",
        )
            .into_response();
    }

    // Get content type from response, default to text/html
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .unwrap_or_else(|| "text/html; charset=utf-8".to_string());

    // Get response body as string for URL rewriting
    let body_bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, video_id = %video_id, "Failed to read embed response");
            return (StatusCode::BAD_GATEWAY, "Failed to read embed response").into_response();
        }
    };

    // Convert to string for URL rewriting
    let html = match String::from_utf8(body_bytes.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, video_id = %video_id, "Embed response is not valid UTF-8");
            // Return original bytes if we can't parse as UTF-8
            let mut headers = HeaderMap::new();
            headers.insert(
                "content-type",
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
            headers.insert(
                "cache-control",
                HeaderValue::from_static("no-store, no-cache, must-revalidate"),
            );
            return (headers, body_bytes).into_response();
        }
    };

    // Rewrite root-relative URLs to absolute URLs
    let rewritten_html = rewrite_html_urls(&html, base_url);
    let rewritten_html = inject_quality_indicator_script(&rewritten_html, &video_id);
    let rewritten_bytes = rewritten_html.into_bytes();

    // Build response with appropriate headers
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        HeaderValue::from_str(&content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("text/html; charset=utf-8")),
    );
    // No cache headers since embed may depend on session/auth state
    headers.insert(
        "cache-control",
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );

    (headers, rewritten_bytes).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_html_urls_double_quoted_root_relative() {
        let input = r#"<script src="/player.js"></script>"#;
        let expected = r#"<script src="https://inv.example.com/player.js"></script>"#;
        assert_eq!(
            rewrite_html_urls(input, "https://inv.example.com"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_single_quoted_root_relative() {
        let input = "<script src='/embed.js'></script>";
        let expected = "<script src='https://inv.example.com/embed.js'></script>";
        assert_eq!(
            rewrite_html_urls(input, "https://inv.example.com"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_preserves_query_strings() {
        let input = r#"<script src="/player.js?v=123"></script>"#;
        let expected = r#"<script src="https://inv.example.com/player.js?v=123"></script>"#;
        assert_eq!(
            rewrite_html_urls(input, "https://inv.example.com"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_does_not_touch_absolute_urls() {
        let input = r#"<script src="https://cdn.example.com/player.js"></script>"#;
        assert_eq!(rewrite_html_urls(input, "https://inv.example.com"), input);
    }

    #[test]
    fn rewrite_html_urls_does_not_touch_protocol_relative_urls() {
        let input = r#"<script src="//cdn.example.com/player.js"></script>"#;
        assert_eq!(rewrite_html_urls(input, "https://inv.example.com"), input);
    }

    #[test]
    fn rewrite_html_urls_does_not_touch_fragment_only_urls() {
        let input = "<a href=\"#settings\">Settings</a>";
        assert_eq!(rewrite_html_urls(input, "https://inv.example.com"), input);
    }

    #[test]
    fn rewrite_html_urls_does_not_touch_data_urls() {
        let input = r#"<img src="data:image/png;base64,abc">"#;
        assert_eq!(rewrite_html_urls(input, "https://inv.example.com"), input);
    }

    #[test]
    fn rewrite_html_urls_handles_multiple_attributes() {
        let input =
            r#"<link href="/css/default.css" rel="stylesheet"><script src="/player.js"></script>"#;
        let expected = r#"<link href="/api/youtube/static/css/default.css" rel="stylesheet"><script src="https://inv.example.com/player.js"></script>"#;
        assert_eq!(
            rewrite_html_urls(input, "https://inv.example.com"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_handles_poster_attribute() {
        let input = r#"<video poster="/vi/example/maxres.jpg"></video>"#;
        let expected = r#"<video poster="/api/youtube/static/vi/example/maxres.jpg"></video>"#;
        assert_eq!(
            rewrite_html_urls(input, "https://inv.example.com"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_handles_trailing_slash_in_base() {
        let input = r#"<script src="/player.js"></script>"#;
        let expected = r#"<script src="https://inv.example.com/player.js"></script>"#;
        assert_eq!(
            rewrite_html_urls(input, "https://inv.example.com/"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_rewrites_absolute_companion_api_url() {
        let input = "https://inv.wandabanet.de/companion/api/manifest/dash/id/VIDEO_ID?local=true";
        let expected = "/api/youtube/companion/api/manifest/dash/id/VIDEO_ID?local=true";
        assert_eq!(
            rewrite_html_urls(input, "https://inv.wandabanet.de"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_rewrites_root_relative_companion_api_url() {
        let input = "\"/companion/api/manifest/dash/id/VIDEO_ID?local=true\"";
        let expected = "\"/api/youtube/companion/api/manifest/dash/id/VIDEO_ID?local=true\"";
        assert_eq!(
            rewrite_html_urls(input, "https://inv.wandabanet.de"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_static_root_relative_asset_still_rewrites_to_absolute() {
        let input = r#"<script src="/player.js"></script>"#;
        let expected = r#"<script src="https://inv.wandabanet.de/player.js"></script>"#;
        assert_eq!(
            rewrite_html_urls(input, "https://inv.wandabanet.de"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_unrelated_absolute_url_unchanged() {
        let input = r#"<script src="https://cdn.example.com/player.js"></script>"#;
        assert_eq!(rewrite_html_urls(input, "https://inv.wandabanet.de"), input);
    }

    #[test]
    fn rewrite_html_urls_rewrites_static_asset_absolute_urls() {
        let input = r#"<script src="https://inv.wandabanet.de/videojs/player.js"></script><link href="https://inv.wandabanet.de/css/default.css" rel="stylesheet"><script src="https://inv.wandabanet.de/js/embed.js"></script><img src="https://inv.wandabanet.de/vi/vYy4em2fQ8Q/maxres.jpg">"#;
        let expected = r#"<script src="/api/youtube/static/videojs/player.js"></script><link href="/api/youtube/static/css/default.css" rel="stylesheet"><script src="/api/youtube/static/js/embed.js"></script><img src="/api/youtube/static/vi/vYy4em2fQ8Q/maxres.jpg">"#;
        assert_eq!(
            rewrite_html_urls(input, "https://inv.wandabanet.de"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_rewrites_static_asset_root_relative_urls() {
        let input = r#""/videojs/player.js" '/css/default.css' "/js/embed.js" "/vi/vYy4em2fQ8Q/maxres.jpg""#;
        let expected = r#""/api/youtube/static/videojs/player.js" '/api/youtube/static/css/default.css' "/api/youtube/static/js/embed.js" "/api/youtube/static/vi/vYy4em2fQ8Q/maxres.jpg""#;
        assert_eq!(
            rewrite_html_urls(input, "https://inv.wandabanet.de"),
            expected
        );
    }

    #[test]
    fn rewrite_dash_manifest_rewrites_base_url_companion_path() {
        let input = r#"<?xml version="1.0"?><MPD><Period><BaseURL>/companion/videoplayback?foo=bar&amp;x=1</BaseURL></Period></MPD>"#;
        let output = rewrite_dash_manifest(input).expect("manifest rewrite should succeed");
        assert!(
            output.contains("<BaseURL>/api/youtube/proxy/videoplayback?foo=bar&amp;x=1</BaseURL>")
        );
    }

    #[test]
    fn rewrite_dash_manifest_keeps_non_companion_base_url_unchanged() {
        let input = r#"<?xml version="1.0"?><MPD><Period><BaseURL>https://cdn.example.com/file.mp4</BaseURL></Period></MPD>"#;
        let output = rewrite_dash_manifest(input).expect("manifest rewrite should succeed");
        assert!(output.contains("<BaseURL>https://cdn.example.com/file.mp4</BaseURL>"));
    }

    #[test]
    fn rewrite_html_urls_rewrites_absolute_latest_version_url() {
        let input = "https://inv.wandabanet.de/companion/latest_version?id=vYy4em2fQ8Q&itag=18";
        let expected = "/api/youtube/latest_version?id=vYy4em2fQ8Q&itag=18";
        assert_eq!(
            rewrite_html_urls(input, "https://inv.wandabanet.de"),
            expected
        );
    }

    #[test]
    fn rewrite_html_urls_rewrites_root_relative_latest_version_url() {
        let input = "\"/companion/latest_version?id=vYy4em2fQ8Q&itag=18\"";
        let expected = "\"/api/youtube/latest_version?id=vYy4em2fQ8Q&itag=18\"";
        assert_eq!(
            rewrite_html_urls(input, "https://inv.wandabanet.de"),
            expected
        );
    }

    #[test]
    fn video_id_from_embed_referer_extracts_video_id() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::REFERER,
            HeaderValue::from_static(
                "http://localhost:8080/api/youtube/embed/vYy4em2fQ8Q?autoplay=1",
            ),
        );
        assert_eq!(
            video_id_from_embed_referer(&headers),
            Some("vYy4em2fQ8Q".to_string())
        );
    }

    #[test]
    fn inject_quality_indicator_script_inserts_before_body_end() {
        let input = "<html><body><div>player</div></body></html>";
        let output = inject_quality_indicator_script(input, "vYy4em2fQ8Q");
        assert!(output.contains("relay-quality-indicator"));
        assert!(
            output.contains("/api/youtube/video/${encodeURIComponent(videoId)}/quality-observed")
        );
        assert!(output.contains("</body></html>"));
    }
}
