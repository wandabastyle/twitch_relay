use std::{
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    http::{HeaderMap, HeaderValue, header},
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
    recording::{ActiveRecording, RecordingBucket, RecordingMode, RecordingService},
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
    );
    prewarm.trigger_now();

    let recording_service =
        RecordingService::new(streamlink_path, config.recording.recordings_dir.clone())
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
        .route("/api/recordings/delete", post(delete_recording_file))
        .route(
            "/api/recordings/playlist.m3u8",
            get(play_recording_playlist),
        )
        .route("/api/recordings/play", get(play_recording_file))
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
struct PlayRecordingFileQuery {
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

async fn play_recording_file(
    State(state): State<RecordingState>,
    Query(query): Query<PlayRecordingFileQuery>,
    headers: HeaderMap,
) -> Response {
    let path = match state
        .service
        .resolve_completed_file_path(&query.channel_login, &query.filename)
    {
        Ok(path) => path,
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            return error_response(status, message);
        }
    };

    if !path.exists() {
        return error_response(StatusCode::NOT_FOUND, "recording file not found");
    }

    let file_size = match std::fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(error) => {
            tracing::error!(error = %error, path = %path.display(), "failed to read recording metadata");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording playback failed",
            );
        }
    };

    let content_type = if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ts"))
    {
        HeaderValue::from_static("video/mp2t")
    } else {
        HeaderValue::from_static("application/octet-stream")
    };

    let range_header = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let (start, end, partial) = match range_header {
        Some(raw) => match parse_byte_range(&raw, file_size) {
            Ok((start, end)) => (start, end, true),
            Err(()) => {
                let mut response =
                    error_response(StatusCode::RANGE_NOT_SATISFIABLE, "invalid range");
                if let Ok(value) = HeaderValue::from_str(&format!("bytes */{file_size}")) {
                    response.headers_mut().insert(header::CONTENT_RANGE, value);
                }
                return response;
            }
        },
        None => {
            if file_size == 0 {
                (0, 0, false)
            } else {
                (0, file_size - 1, false)
            }
        }
    };

    let length = if file_size == 0 { 0 } else { end - start + 1 };
    let length_usize = match usize::try_from(length) {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording playback failed",
            );
        }
    };

    let mut file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            tracing::error!(error = %error, path = %path.display(), "failed to open recording file");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording playback failed",
            );
        }
    };

    if let Err(error) = file.seek(SeekFrom::Start(start)) {
        tracing::error!(error = %error, path = %path.display(), "failed to seek recording file");
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "recording playback failed",
        );
    }

    let mut chunk = Vec::with_capacity(length_usize);
    if let Err(error) = file.take(length).read_to_end(&mut chunk) {
        tracing::error!(error = %error, path = %path.display(), "failed to read recording file bytes");
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "recording playback failed",
        );
    }

    let mut response = if partial {
        Response::builder().status(StatusCode::PARTIAL_CONTENT)
    } else {
        Response::builder().status(StatusCode::OK)
    }
    .header(header::ACCEPT_RANGES, "bytes")
    .header(header::CONTENT_TYPE, content_type)
    .header(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate")
    .header(header::PRAGMA, "no-cache")
    .header(header::EXPIRES, "0")
    .header(header::CONTENT_LENGTH, chunk.len().to_string());

    if partial && let Ok(value) = HeaderValue::from_str(&format!("bytes {start}-{end}/{file_size}"))
    {
        response = response.header(header::CONTENT_RANGE, value);
    }

    match response.body(Body::from(chunk)) {
        Ok(response) => response.into_response(),
        Err(error) => {
            tracing::error!(error = %error, "failed to build playback response");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording playback failed",
            )
        }
    }
}

async fn play_recording_playlist(
    State(state): State<RecordingState>,
    Query(query): Query<PlayRecordingFileQuery>,
) -> Response {
    let path = match state
        .service
        .resolve_completed_file_path(&query.channel_login, &query.filename)
    {
        Ok(path) => path,
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            return error_response(status, message);
        }
    };

    if !path.exists() {
        return error_response(StatusCode::NOT_FOUND, "recording file not found");
    }

    let file_size = match std::fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(error) => {
            tracing::error!(error = %error, path = %path.display(), "failed to read recording metadata");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording playback failed",
            );
        }
    };

    if file_size == 0 {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "recording playback failed",
        );
    }

    let media_uri = format!(
        "/api/recordings/play?channel_login={}&filename={}",
        percent_encode_query_component(&query.channel_login),
        percent_encode_query_component(&query.filename)
    );

    let segment_size = 2 * 1024 * 1024_u64;
    let assumed_bitrate_bps = 6_000_000_f64;
    let segment_duration = ((segment_size as f64) * 8.0 / assumed_bitrate_bps).max(1.0);
    let target_duration = segment_duration.ceil() as u64;

    let mut playlist = String::from(
        "#EXTM3U\n#EXT-X-VERSION:4\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-MEDIA-SEQUENCE:0\n",
    );
    playlist.push_str(&format!("#EXT-X-TARGETDURATION:{target_duration}\n"));

    let mut offset = 0_u64;
    while offset < file_size {
        let byte_len = (file_size - offset).min(segment_size);
        let extinf = ((byte_len as f64) * 8.0 / assumed_bitrate_bps).max(0.05);

        playlist.push_str(&format!("#EXTINF:{extinf:.3},\n"));
        playlist.push_str(&format!("#EXT-X-BYTERANGE:{byte_len}@{offset}\n"));
        playlist.push_str(&media_uri);
        playlist.push('\n');

        offset += byte_len;
    }
    playlist.push_str("#EXT-X-ENDLIST\n");

    let mut response = Response::new(Body::from(playlist));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.apple.mpegurl"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(header::EXPIRES, HeaderValue::from_static("0"));
    response
}

fn percent_encode_query_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());

    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }

    out
}

fn parse_byte_range(value: &str, file_size: u64) -> Result<(u64, u64), ()> {
    if file_size == 0 {
        return Err(());
    }
    let raw = value.trim();
    let Some(range) = raw.strip_prefix("bytes=") else {
        return Err(());
    };
    let Some((start_raw, end_raw)) = range.split_once('-') else {
        return Err(());
    };

    if start_raw.is_empty() {
        let suffix_len = end_raw.parse::<u64>().map_err(|_| ())?;
        if suffix_len == 0 {
            return Err(());
        }
        if suffix_len >= file_size {
            return Ok((0, file_size - 1));
        }
        return Ok((file_size - suffix_len, file_size - 1));
    }

    let start = start_raw.parse::<u64>().map_err(|_| ())?;
    if start >= file_size {
        return Err(());
    }

    let end = if end_raw.is_empty() {
        file_size - 1
    } else {
        end_raw.parse::<u64>().map_err(|_| ())?
    };

    if end < start {
        return Err(());
    }

    Ok((start, end.min(file_size - 1)))
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
