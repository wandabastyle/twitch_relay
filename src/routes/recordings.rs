use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::StatusCode,
    http::{HeaderMap, HeaderValue, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use futures_util::stream;
use serde::{Deserialize, Serialize};

use crate::{
    auth::{self, WebAuthConfig},
    recording::{
        ActiveRecording, RecordingBucket, RecordingError, RecordingMode, RecordingService,
    },
    recording_rules::{self, RecordingRule},
    routes::error::error_response,
};

/// State for recording routes.
#[derive(Debug, Clone)]
pub struct RecordingState {
    pub service: RecordingService,
    pub default_quality: String,
}

/// Request DTO for starting a recording.
#[derive(Debug, Deserialize)]
pub struct StartRecordingRequest {
    pub channel_login: String,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub stream_title: Option<String>,
}

/// Request DTO for stopping a recording.
#[derive(Debug, Deserialize)]
pub struct StopRecordingRequest {
    pub channel_login: String,
}

/// Request DTO for upserting a recording rule.
#[derive(Debug, Deserialize)]
pub struct UpsertRecordingRuleRequest {
    pub channel_login: String,
    pub enabled: bool,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub stop_when_offline: Option<bool>,
    #[serde(default)]
    pub max_duration_minutes: Option<u64>,
    #[serde(default)]
    pub keep_last_videos: Option<u64>,
}

/// Request DTO for deleting a recording file.
#[derive(Debug, Deserialize)]
pub struct DeleteRecordingFileRequest {
    pub bucket: String,
    pub channel_login: String,
    pub filename: String,
}

/// Request DTO for pinning/unpinning a recording file.
#[derive(Debug, Deserialize)]
pub struct PinRecordingFileRequest {
    pub bucket: String,
    pub channel_login: String,
    pub filename: String,
}

/// Query parameters for playing a recording asset.
#[derive(Debug, Deserialize)]
pub struct PlayRecordingAssetQuery {
    pub channel_login: String,
    pub filename: String,
}

/// Response DTO for recording rules.
#[derive(Debug, Serialize)]
pub struct RecordingRulesResponse {
    pub rules: Vec<RecordingRule>,
}

/// Response DTO for recordings overview.
#[derive(Debug, Serialize)]
pub struct RecordingsResponse {
    pub active: Vec<ActiveRecording>,
    pub completed: Vec<crate::recording::RecordingFileEntry>,
    pub incomplete: Vec<crate::recording::RecordingFileEntry>,
}

/// Query parameters for serving an HLS playlist.
#[derive(Debug, Deserialize)]
pub struct ServeHlsPlaylistQuery {
    pub channel_login: String,
    pub filename: String,
}

/// Build recording routes.
pub fn recording_routes(state: RecordingState, auth_config: WebAuthConfig) -> Router {
    Router::new()
        .route("/api/recordings/start", post(start_recording))
        .route("/api/recordings/stop", post(stop_recording))
        .route("/api/recordings/pin", post(pin_recording_file))
        .route("/api/recordings/unpin", post(unpin_recording_file))
        .route("/api/recordings/delete", post(delete_recording_file))
        .route("/api/recordings/playback-file", get(play_recording_asset))
        .route("/api/recordings/hls-playlist", get(serve_hls_playlist))
        .route("/api/recordings", get(get_recordings))
        .route("/api/recording-rules", get(get_recording_rules))
        .route("/api/recording-rules", post(upsert_recording_rule))
        .route(
            "/api/recording-rules/{channel_login}",
            delete(delete_recording_rule),
        )
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            auth_config,
            auth::require_session_middleware,
        ))
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

        // HLS uses exact byte ranges from the m3u8 - open-ended ranges not supported
        let end: u64 = if end_str.is_empty() {
            return error_response(StatusCode::BAD_REQUEST, "open-ended ranges not supported");
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
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=2592000, immutable"),
        );
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
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=2592000, immutable"),
        );
        response
    }
}

async fn serve_hls_playlist(
    State(state): State<RecordingState>,
    Query(query): Query<ServeHlsPlaylistQuery>,
) -> Response {
    // Resolve the MP4 path
    let mp4_path = match state
        .service
        .resolve_completed_file_path(&query.channel_login, &query.filename)
    {
        Ok(path) => path,
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            return error_response(status, message);
        }
    };

    if !mp4_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "recording not found");
    }

    // Look for the .m3u8 playlist file
    let playlist_path = mp4_path.with_extension("m3u8");
    if !playlist_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "hls playlist not found");
    }

    // Read and serve the playlist
    let playlist_content = match tokio::fs::read_to_string(&playlist_path).await {
        Ok(content) => content,
        Err(error) => {
            tracing::error!(error = %error, path = %playlist_path.display(), "failed to read hls playlist");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to read playlist");
        }
    };

    let mut response = Response::new(Body::from(playlist_content));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.apple.mpegurl"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, must-revalidate"),
    );
    response
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
        const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4 MiB chunks for HDD optimization

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

fn classify_recording_error(error: &RecordingError) -> (StatusCode, &'static str) {
    match error {
        RecordingError::EmptyChannelLogin => {
            (StatusCode::BAD_REQUEST, "channel login cannot be empty")
        }
        RecordingError::InvalidQuality => (StatusCode::BAD_REQUEST, "invalid quality"),
        RecordingError::AlreadyActive => (StatusCode::CONFLICT, "recording already active"),
        RecordingError::NotActive => (StatusCode::NOT_FOUND, "recording not active"),
        RecordingError::FileNotFound => (StatusCode::NOT_FOUND, "recording file not found"),
        RecordingError::EmptyFilename => (StatusCode::BAD_REQUEST, "filename cannot be empty"),
        RecordingError::InvalidFilename => (StatusCode::BAD_REQUEST, "invalid filename"),
        RecordingError::DeleteFailed(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "recording delete failed")
        }
        RecordingError::SpawnFailed(_) => (StatusCode::BAD_GATEWAY, "streamlink spawn failed"),
        RecordingError::DirectoryNotWritable(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "recordings directory not writable",
        ),
        RecordingError::PinFailed(_) | RecordingError::UnpinFailed(_) | RecordingError::Io(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "recording operation failed",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording::RecordingError;

    #[test]
    fn classify_recording_error_maps_empty_channel_login() {
        let (status, message) = classify_recording_error(&RecordingError::EmptyChannelLogin);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(message, "channel login cannot be empty");
    }

    #[test]
    fn classify_recording_error_maps_invalid_quality() {
        let (status, message) = classify_recording_error(&RecordingError::InvalidQuality);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(message, "invalid quality");
    }

    #[test]
    fn classify_recording_error_maps_already_active() {
        let (status, message) = classify_recording_error(&RecordingError::AlreadyActive);
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(message, "recording already active");
    }

    #[test]
    fn classify_recording_error_maps_not_active() {
        let (status, message) = classify_recording_error(&RecordingError::NotActive);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(message, "recording not active");
    }

    #[test]
    fn classify_recording_error_maps_file_not_found() {
        let (status, message) = classify_recording_error(&RecordingError::FileNotFound);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(message, "recording file not found");
    }

    #[test]
    fn classify_recording_error_maps_empty_filename() {
        let (status, message) = classify_recording_error(&RecordingError::EmptyFilename);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(message, "filename cannot be empty");
    }

    #[test]
    fn classify_recording_error_maps_invalid_filename() {
        let (status, message) = classify_recording_error(&RecordingError::InvalidFilename);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(message, "invalid filename");
    }

    #[test]
    fn classify_recording_error_maps_delete_failed() {
        let (status, message) =
            classify_recording_error(&RecordingError::DeleteFailed("some error".to_string()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(message, "recording delete failed");
    }

    #[test]
    fn classify_recording_error_maps_spawn_failed() {
        let (status, message) =
            classify_recording_error(&RecordingError::SpawnFailed("some error".to_string()));
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(message, "streamlink spawn failed");
    }

    #[test]
    fn classify_recording_error_maps_directory_not_writable() {
        let (status, message) = classify_recording_error(&RecordingError::DirectoryNotWritable(
            "some error".to_string(),
        ));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(message, "recordings directory not writable");
    }

    #[test]
    fn classify_recording_error_maps_pin_failed_to_fallback() {
        let (status, message) =
            classify_recording_error(&RecordingError::PinFailed("some error".to_string()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(message, "recording operation failed");
    }

    #[test]
    fn classify_recording_error_maps_unpin_failed_to_fallback() {
        let (status, message) =
            classify_recording_error(&RecordingError::UnpinFailed("some error".to_string()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(message, "recording operation failed");
    }

    #[test]
    fn classify_recording_error_maps_io_to_fallback() {
        let (status, message) =
            classify_recording_error(&RecordingError::Io("some error".to_string()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(message, "recording operation failed");
    }
}
