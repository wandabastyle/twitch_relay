use std::path::PathBuf;

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::StatusCode,
    http::{HeaderMap, HeaderValue, header},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};
use futures_util::stream;
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
    recording::{
        ActiveRecording, RecordingBucket, RecordingMode, RecordingProcessingConfig,
        RecordingService,
    },
    recording_rules::{self, RecordingRule},
    recording_scheduler::RecordingScheduler,
    stream_proxy, twitch_auth,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        stream_service.clone(),
    );
    prewarm.trigger_now();

    let recording_service = RecordingService::new(
        streamlink_path,
        config.recording.recordings_dir.clone(),
        config.recording.write_nfo,
        config.recording.nfo_style,
        twitch_auth_service.clone(),
        RecordingProcessingConfig {
            ffmpeg_path: config.recording.ffmpeg_path.clone(),
            chapter_min_gap_secs: config.recording.chapter_min_gap_secs,
            chapter_change_confirmations: config.recording.chapter_change_confirmations,
        },
    )
    .map_err(AppError::Config)?;
    RecordingScheduler::start(
        config.recording.clone(),
        live_status_state.service.clone(),
        recording_service.clone(),
    );

    let twitch_state = twitch_auth::TwitchAuthState {
        auth: auth_config.clone(),
        twitch: twitch_auth_service,
        prewarm: Some(prewarm.clone()),
    };

    let stream_proxy_state = stream_proxy::StreamProxyState::new(stream_service.clone());

    let recording_state = RecordingState {
        service: recording_service,
        default_quality: config.recording.default_quality.clone(),
    };

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

    let recording_routes = Router::new()
        .route("/api/recordings/start", post(start_recording))
        .route("/api/recordings/stop", post(stop_recording))
        .route("/api/recordings/pin", post(pin_recording_file))
        .route("/api/recordings/unpin", post(unpin_recording_file))
        .route("/api/recordings/delete", post(delete_recording_file))
        .route("/api/recordings/playback-file", get(play_recording_asset))
        .route("/api/recordings", get(get_recordings))
        .route("/api/recording-rules", get(get_recording_rules))
        .route("/api/recording-rules", post(upsert_recording_rule))
        .route(
            "/api/recording-rules/{channel_login}",
            delete(delete_recording_rule),
        )
        .with_state(recording_state)
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
        .route("/api/version", get(get_version))
        .merge(auth_routes)
        .merge(channel_routes)
        .merge(live_status_routes)
        .merge(protected_routes)
        .merge(recording_routes)
        .merge(twitch_routes)
        .merge(chat_routes)
        .merge(stream_routes)
        .nest_service("/static/images", ServeDir::new(&images_path))
        .nest_service("/static", ServeDir::new(&assets_path))
        .fallback_service(
            ServeDir::new(&static_path).fallback(ServeFile::new(static_path.join("index.html"))),
        );

    Ok(router)
}

#[derive(Debug, Serialize)]
struct ProbeResponse<'a> {
    status: &'a str,
    service: &'a str,
}

#[derive(Debug, Serialize)]
struct VersionResponse<'a> {
    version: &'a str,
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

#[derive(Debug, Clone)]
struct RecordingState {
    service: RecordingService,
    default_quality: String,
}

#[derive(Debug, Deserialize)]
struct AddChannelRequest {
    login: String,
}

#[derive(Debug, Deserialize)]
struct StartRecordingRequest {
    channel_login: String,
    #[serde(default)]
    quality: Option<String>,
    #[serde(default)]
    stream_title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StopRecordingRequest {
    channel_login: String,
}

#[derive(Debug, Deserialize)]
struct UpsertRecordingRuleRequest {
    channel_login: String,
    enabled: bool,
    #[serde(default)]
    quality: Option<String>,
    #[serde(default)]
    stop_when_offline: Option<bool>,
    #[serde(default)]
    max_duration_minutes: Option<u64>,
    #[serde(default)]
    keep_last_videos: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DeleteRecordingFileRequest {
    bucket: String,
    channel_login: String,
    filename: String,
}

#[derive(Debug, Deserialize)]
struct PinRecordingFileRequest {
    bucket: String,
    channel_login: String,
    filename: String,
}

#[derive(Debug, Deserialize)]
struct PlayRecordingAssetQuery {
    channel_login: String,
    filename: String,
}

#[derive(Debug, Serialize)]
struct RecordingRulesResponse {
    rules: Vec<RecordingRule>,
}

#[derive(Debug, Serialize)]
struct RecordingsResponse {
    active: Vec<ActiveRecording>,
    completed: Vec<crate::recording::RecordingFileEntry>,
    incomplete: Vec<crate::recording::RecordingFileEntry>,
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

async fn start_recording(
    State(state): State<RecordingState>,
    Json(payload): Json<StartRecordingRequest>,
) -> Response {
    let quality = payload
        .quality
        .unwrap_or_else(|| state.default_quality.clone());
    let quality = match RecordingService::validate_quality(&quality) {
        Ok(value) => value,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid quality"),
    };

    match state
        .service
        .start_recording(
            &payload.channel_login,
            &quality,
            RecordingMode::Manual,
            payload.stream_title.as_deref(),
        )
        .await
    {
        Ok(active) => (StatusCode::OK, Json(active)).into_response(),
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            if status == StatusCode::INTERNAL_SERVER_ERROR {
                tracing::error!(error = %error, "manual recording start failed");
            }
            error_response(status, message)
        }
    }
}

async fn stop_recording(
    State(state): State<RecordingState>,
    Json(payload): Json<StopRecordingRequest>,
) -> Response {
    match state.service.stop_recording(&payload.channel_login).await {
        Ok(active) => (StatusCode::OK, Json(active)).into_response(),
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            if status == StatusCode::INTERNAL_SERVER_ERROR {
                tracing::error!(error = %error, "recording stop failed");
            }
            error_response(status, message)
        }
    }
}

async fn get_recordings(State(state): State<RecordingState>) -> Json<RecordingsResponse> {
    let overview = state.service.list_overview(15).await;
    Json(RecordingsResponse {
        active: overview.active,
        completed: overview.completed,
        incomplete: overview.incomplete,
    })
}

async fn delete_recording_file(
    State(state): State<RecordingState>,
    Json(payload): Json<DeleteRecordingFileRequest>,
) -> Response {
    let bucket = match payload.bucket.as_str() {
        "completed" => RecordingBucket::Completed,
        "incomplete" => RecordingBucket::Incomplete,
        _ => return error_response(StatusCode::BAD_REQUEST, "invalid recording bucket"),
    };

    match state
        .service
        .delete_recording_file(bucket, &payload.channel_login, &payload.filename)
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            if status == StatusCode::INTERNAL_SERVER_ERROR {
                tracing::error!(error = %error, "recording file delete failed");
            }
            error_response(status, message)
        }
    }
}

async fn pin_recording_file(
    State(state): State<RecordingState>,
    Json(payload): Json<PinRecordingFileRequest>,
) -> Response {
    if payload.bucket != "completed" {
        return error_response(
            StatusCode::BAD_REQUEST,
            "pinning is only supported for completed recordings",
        );
    }

    match state
        .service
        .pin_recording_file(&payload.channel_login, &payload.filename)
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            if status == StatusCode::INTERNAL_SERVER_ERROR {
                tracing::error!(error = %error, "recording file pin failed");
            }
            error_response(status, message)
        }
    }
}

async fn unpin_recording_file(
    State(state): State<RecordingState>,
    Json(payload): Json<PinRecordingFileRequest>,
) -> Response {
    if payload.bucket != "completed" {
        return error_response(
            StatusCode::BAD_REQUEST,
            "unpinning is only supported for completed recordings",
        );
    }

    match state
        .service
        .unpin_recording_file(&payload.channel_login, &payload.filename)
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            if status == StatusCode::INTERNAL_SERVER_ERROR {
                tracing::error!(error = %error, "recording file unpin failed");
            }
            error_response(status, message)
        }
    }
}

async fn play_recording_asset(
    State(state): State<RecordingState>,
    Query(query): Query<PlayRecordingAssetQuery>,
    headers: HeaderMap,
) -> Response {
    const INITIAL_RANGE_BYTES: u64 = 4 * 1024 * 1024;
    const FOLLOWUP_RANGE_BYTES: u64 = 8 * 1024 * 1024;

    let media_path = match state
        .service
        .resolve_completed_file_path(&query.channel_login, &query.filename)
    {
        Ok(path) => path,
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            return error_response(status, message);
        }
    };

    if !media_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "recording playback asset not found");
    }

    let file_size = match tokio::fs::metadata(&media_path).await {
        Ok(meta) => meta.len(),
        Err(error) => {
            tracing::error!(error = %error, path = %media_path.display(), "failed to read playback media metadata");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording playback failed",
            );
        }
    };

    if let Some(range_header) = headers.get(header::RANGE) {
        let Ok(range_str) = range_header.to_str() else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header");
        };
        let Some(range_spec) = range_str.strip_prefix("bytes=") else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header");
        };
        let Some((start_str, end_str)) = range_spec.split_once('-') else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header");
        };

        let start: u64 = match start_str.parse() {
            Ok(v) => v,
            Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid range start"),
        };
        let end: u64 = if end_str.is_empty() {
            let max_open_ended_bytes = if start == 0 {
                INITIAL_RANGE_BYTES
            } else {
                FOLLOWUP_RANGE_BYTES
            };
            start
                .saturating_add(max_open_ended_bytes.saturating_sub(1))
                .min(file_size.saturating_sub(1))
        } else {
            match end_str.parse() {
                Ok(v) => v,
                Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid range end"),
            }
        };

        if start >= file_size || end >= file_size || end < start {
            return error_response(StatusCode::RANGE_NOT_SATISFIABLE, "range not satisfiable");
        }

        let length = end - start + 1;
        let media_stream = match stream_file_range(&media_path, start, length).await {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(error = %error, path = %media_path.display(), "failed to read playback range");
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "recording playback failed",
                );
            }
        };

        let content_range = format!("bytes {start}-{end}/{file_size}");
        let mut response = Response::new(Body::from_stream(media_stream));
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("video/mp4"));
        response.headers_mut().insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&content_range).unwrap(),
        );
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&length.to_string()).unwrap(),
        );
        response
            .headers_mut()
            .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        response
    } else {
        let media_stream = match stream_file_range(&media_path, 0, file_size).await {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(error = %error, path = %media_path.display(), "failed to read playback media");
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "recording playback failed",
                );
            }
        };
        let mut response = Response::new(Body::from_stream(media_stream));
        *response.status_mut() = StatusCode::OK;
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("video/mp4"));
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&file_size.to_string()).unwrap(),
        );
        response
            .headers_mut()
            .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        response
    }
}

async fn stream_file_range(
    path: &std::path::Path,
    start: u64,
    length: u64,
) -> Result<
    std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
    String,
> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| format!("failed to open playback media: {error}"))?;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|error| format!("failed to seek playback media: {error}"))?;
    let stream = stream::try_unfold((file, length), |(mut file, remaining)| async move {
        const CHUNK_SIZE: usize = 256 * 1024;

        if remaining == 0 {
            return Ok(None);
        }

        let next_len = usize::try_from(remaining.min(CHUNK_SIZE as u64)).unwrap_or(CHUNK_SIZE);
        let mut chunk = vec![0_u8; next_len];
        let read = file.read(&mut chunk).await?;
        if read == 0 {
            return Ok(None);
        }
        chunk.truncate(read);
        let read_u64 = u64::try_from(read).unwrap_or(0);
        let next_remaining = remaining.saturating_sub(read_u64);
        Ok(Some((Bytes::from(chunk), (file, next_remaining))))
    });

    Ok(Box::pin(stream))
}

async fn get_recording_rules() -> Response {
    match recording_rules::load_rules() {
        Ok(rules) => (StatusCode::OK, Json(RecordingRulesResponse { rules })).into_response(),
        Err(error) => {
            tracing::error!(error = %error, "recording rules load failed");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording rules file read/write failure",
            )
        }
    }
}

async fn upsert_recording_rule(
    State(state): State<RecordingState>,
    Json(payload): Json<UpsertRecordingRuleRequest>,
) -> Response {
    let channel_login = payload.channel_login.trim().to_ascii_lowercase();
    if channel_login.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "channel login cannot be empty");
    }

    let quality_input = payload
        .quality
        .unwrap_or_else(|| state.default_quality.clone());
    let quality = match RecordingService::validate_quality(&quality_input) {
        Ok(value) => value,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid quality"),
    };

    if payload.keep_last_videos == Some(0) {
        return error_response(StatusCode::BAD_REQUEST, "keep_last_videos must be >= 1");
    }

    let rule = RecordingRule {
        channel_login,
        enabled: payload.enabled,
        quality,
        stop_when_offline: payload.stop_when_offline.unwrap_or(true),
        max_duration_minutes: payload.max_duration_minutes,
        keep_last_videos: payload.keep_last_videos,
    };

    match recording_rules::upsert_rule(rule) {
        Ok(saved) => (StatusCode::OK, Json(saved)).into_response(),
        Err(error) => {
            tracing::error!(error = %error, "recording rule save failed");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording rules file read/write failure",
            )
        }
    }
}

async fn delete_recording_rule(Path(channel_login): Path<String>) -> Response {
    let normalized = channel_login.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "channel login cannot be empty");
    }

    match recording_rules::delete_rule(&normalized) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(StatusCode::NOT_FOUND, "recording rule not found"),
        Err(error) => {
            tracing::error!(error = %error, "recording rule delete failed");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording rules file read/write failure",
            )
        }
    }
}

fn classify_recording_error(error: &str) -> (StatusCode, &str) {
    if error.contains("channel login cannot be empty") {
        return (StatusCode::BAD_REQUEST, "channel login cannot be empty");
    }
    if error.contains("invalid quality") {
        return (StatusCode::BAD_REQUEST, "invalid quality");
    }
    if error.contains("already active") {
        return (StatusCode::CONFLICT, "recording already active");
    }
    if error.contains("not active") {
        return (StatusCode::NOT_FOUND, "recording not active");
    }
    if error.contains("file not found") {
        return (StatusCode::NOT_FOUND, "recording file not found");
    }
    if error.contains("filename cannot be empty") {
        return (StatusCode::BAD_REQUEST, "filename cannot be empty");
    }
    if error.contains("invalid filename") {
        return (StatusCode::BAD_REQUEST, "invalid filename");
    }
    if error.contains("delete failed") {
        return (StatusCode::INTERNAL_SERVER_ERROR, "recording delete failed");
    }
    if error.contains("spawn failed") {
        return (StatusCode::BAD_GATEWAY, "streamlink spawn failed");
    }
    if error.contains("not writable") {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "recordings directory not writable",
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "recording operation failed",
    )
}

async fn healthz() -> Json<ProbeResponse<'static>> {
    Json(ProbeResponse {
        status: "ok",
        service: "twitch-relay",
    })
}

async fn get_version() -> Json<VersionResponse<'static>> {
    Json(VersionResponse {
        version: APP_VERSION,
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

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
