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
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::{
    auth::{self, WebAuthConfig},
    recording::{
        ActiveRecording, RecordingBucket, RecordingError, RecordingFileEntry, RecordingJob,
        RecordingJobKind, RecordingJobStatus, RecordingMode, RecordingService,
    },
    recording_progress::{RecordingProgressStore, RecordingWatchProgressEntry},
    recording_rules::{self, RecordingRule},
    routes::error::error_response,
};

use std::future::Future;

/// State for recording routes.
#[derive(Debug, Clone)]
pub struct RecordingState {
    pub auth: WebAuthConfig,
    pub service: RecordingService,
    pub default_quality: String,
    pub progress: RecordingProgressStore,
    pub active_processing_guard: Arc<RwLock<HashSet<String>>>,
    pub recording_jobs: Arc<RwLock<HashMap<String, crate::recording::RecordingJob>>>,
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

/// Request DTO for merging incomplete recordings.
#[derive(Debug, Deserialize)]
pub struct MergeRecordingsRequest {
    pub channel_login: String,
    pub filenames: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RepairRecordingRequest {
    pub channel_login: String,
    pub filename: String,
}

#[derive(Debug, Deserialize)]
pub struct FinalizeIncompleteRecordingRequest {
    pub channel_login: String,
    pub filename: String,
}

/// Response DTO for async recording job accept.
#[derive(Debug, Serialize)]
pub struct RecordingJobAcceptedResponse {
    pub job_id: String,
    pub kind: crate::recording::RecordingJobKind,
    pub channel_login: String,
    pub expected_filename: String,
    pub source_count: usize,
}

#[derive(Debug, Serialize)]
pub struct RecordingJobStatusResponse<'a> {
    pub job_id: &'a str,
    pub kind: crate::recording::RecordingJobKind,
    pub status: crate::recording::RecordingJobStatus,
    pub channel_login: &'a str,
    pub expected_filename: &'a str,
    pub final_filename: Option<&'a str>,
    pub error: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct RepairRecordingResponse {
    pub repaired_file: crate::recording::RecordingFileEntry,
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

/// Query parameters for recording watch progress.
#[derive(Debug, Deserialize)]
pub struct RecordingWatchProgressQuery {
    pub channel_login: String,
    pub filename: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordingWatchProgressUpdateRequest {
    pub channel_login: String,
    pub filename: String,
    pub position_secs: f64,
    #[serde(default)]
    pub duration_secs: Option<f64>,
    #[serde(default)]
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize)]
struct RecordingWatchProgressResponse {
    channel_login: String,
    filename: String,
    position_secs: Option<f64>,
    duration_secs: Option<f64>,
    updated_at_unix: Option<u64>,
    completed: bool,
}

/// Build recording routes.
pub fn recording_routes(state: RecordingState, auth_config: WebAuthConfig) -> Router {
    Router::new()
        .route("/api/recordings/start", post(start_recording))
        .route("/api/recordings/stop", post(stop_recording))
        .route("/api/recordings/pin", post(pin_recording_file))
        .route("/api/recordings/unpin", post(unpin_recording_file))
        .route("/api/recordings/delete", post(delete_recording_file))
        .route("/api/recordings/merge", post(merge_recordings))
        .route(
            "/api/recordings/finalize-incomplete",
            post(finalize_incomplete_recording),
        )
        .route(
            "/api/recordings/jobs/{job_id}",
            get(get_recording_job_status),
        )
        .route("/api/recordings/repair", post(repair_recording))
        .route("/api/recordings/playback-file", get(play_recording_asset))
        .route("/api/recordings/hls-playlist", get(serve_hls_playlist))
        .route(
            "/api/recordings/progress",
            get(get_recording_progress).put(put_recording_progress),
        )
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

fn recording_progress_response(
    channel_login: String,
    filename: String,
    entry: Option<RecordingWatchProgressEntry>,
) -> Response {
    let payload = if let Some(entry) = entry {
        RecordingWatchProgressResponse {
            channel_login,
            filename,
            position_secs: Some(entry.position_secs),
            duration_secs: entry.duration_secs,
            updated_at_unix: Some(entry.updated_at_unix),
            completed: entry.completed,
        }
    } else {
        RecordingWatchProgressResponse {
            channel_login,
            filename,
            position_secs: None,
            duration_secs: None,
            updated_at_unix: None,
            completed: false,
        }
    };

    (StatusCode::OK, Json(payload)).into_response()
}

async fn get_recording_progress(
    State(state): State<RecordingState>,
    Query(query): Query<RecordingWatchProgressQuery>,
    request_headers: HeaderMap,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&request_headers) else {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    };

    let progress = state
        .progress
        .get(&session_token, &query.channel_login, &query.filename);
    recording_progress_response(query.channel_login, query.filename, progress)
}

async fn put_recording_progress(
    State(state): State<RecordingState>,
    request_headers: HeaderMap,
    Json(payload): Json<RecordingWatchProgressUpdateRequest>,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&request_headers) else {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    };

    let saved = state.progress.upsert(
        &session_token,
        &payload.channel_login,
        &payload.filename,
        payload.position_secs,
        payload.duration_secs,
        payload.completed,
    );
    let Some(entry) = saved else {
        return (
            StatusCode::BAD_REQUEST,
            "invalid progress payload or failed to persist",
        )
            .into_response();
    };

    recording_progress_response(payload.channel_login, payload.filename, Some(entry))
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
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid quality", None),
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
            error_response(status, message, None)
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
            error_response(status, message, None)
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
        _ => return error_response(StatusCode::BAD_REQUEST, "invalid recording bucket", None),
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
            error_response(status, message, None)
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
            None,
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
            error_response(status, message, None)
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
            None,
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
            error_response(status, message, None)
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
            return error_response(status, message, None);
        }
    };

    if !media_path.exists() {
        return error_response(
            StatusCode::NOT_FOUND,
            "recording playback asset not found",
            None,
        );
    }

    let file_size = match tokio::fs::metadata(&media_path).await {
        Ok(meta) => meta.len(),
        Err(error) => {
            tracing::error!(error = %error, path = %media_path.display(), "failed to read playback media metadata");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording playback failed",
                None,
            );
        }
    };

    if let Some(range_header) = headers.get(header::RANGE) {
        let Ok(range_str) = range_header.to_str() else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header", None);
        };
        let Some(range_spec) = range_str.strip_prefix("bytes=") else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header", None);
        };
        let Some((start_str, end_str)) = range_spec.split_once('-') else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header", None);
        };

        let start: u64 = match start_str.parse() {
            Ok(v) => v,
            Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid range start", None),
        };

        // HLS uses exact byte ranges from the m3u8 - open-ended ranges not supported
        let end: u64 = if end_str.is_empty() {
            return error_response(
                StatusCode::BAD_REQUEST,
                "open-ended ranges not supported",
                None,
            );
        } else {
            match end_str.parse() {
                Ok(v) => v,
                Err(_) => {
                    return error_response(StatusCode::BAD_REQUEST, "invalid range end", None);
                }
            }
        };

        if start >= file_size || end >= file_size || end < start {
            return error_response(
                StatusCode::RANGE_NOT_SATISFIABLE,
                "range not satisfiable",
                None,
            );
        }

        let length = end - start + 1;
        let media_stream = match stream_file_range(&media_path, start, length).await {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(error = %error, path = %media_path.display(), "failed to read playback range");
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "recording playback failed",
                    None,
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
                    None,
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
            return error_response(status, message, None);
        }
    };

    if !mp4_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "recording not found", None);
    }

    // Look for the .m3u8 playlist file
    let playlist_path = mp4_path.with_extension("m3u8");
    if !playlist_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "hls playlist not found", None);
    }

    // Read and serve the playlist
    let playlist_content = match tokio::fs::read_to_string(&playlist_path).await {
        Ok(content) => content,
        Err(error) => {
            tracing::error!(error = %error, path = %playlist_path.display(), "failed to read hls playlist");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read playlist",
                None,
            );
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
                None,
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
        return error_response(
            StatusCode::BAD_REQUEST,
            "channel login cannot be empty",
            None,
        );
    }

    let quality_input = payload
        .quality
        .unwrap_or_else(|| state.default_quality.clone());
    let quality = match RecordingService::validate_quality(&quality_input) {
        Ok(value) => value,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid quality", None),
    };

    if payload.keep_last_videos == Some(0) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "keep_last_videos must be >= 1",
            None,
        );
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
                None,
            )
        }
    }
}

async fn delete_recording_rule(Path(channel_login): Path<String>) -> Response {
    let normalized = channel_login.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "channel login cannot be empty",
            None,
        );
    }

    match recording_rules::delete_rule(&normalized) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(StatusCode::NOT_FOUND, "recording rule not found", None),
        Err(error) => {
            tracing::error!(error = %error, "recording rule delete failed");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording rules file read/write failure",
                None,
            )
        }
    }
}

/// Spawns a recording job with standardized setup, execution, and cleanup.
///
/// Handles:
/// - Guard acquisition and conflict checking
/// - Job creation with unique UUID
/// - Status transitions (Queued -> Running -> Completed/Failed)
/// - Guard cleanup on completion/error
///
/// Returns the job ID on success, or an error response tuple on conflict.
async fn spawn_recording_job<F>(
    state: &RecordingState,
    kind: RecordingJobKind,
    channel_login: &str,
    source_filenames: Vec<String>,
    expected_filename: &str,
    job_future: F,
) -> Result<String, (StatusCode, &'static str)>
where
    F: Future<Output = Result<RecordingFileEntry, RecordingError>> + Send + 'static,
{
    // Acquire guard and check for conflicts
    {
        let mut guard = state.active_processing_guard.write().await;
        if guard.contains(channel_login) {
            return Err((
                StatusCode::CONFLICT,
                "recording job already running for channel",
            ));
        }
        guard.insert(channel_login.to_string());
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let now = crate::util::time::now_unix_secs();
    let job = RecordingJob {
        job_id: job_id.clone(),
        kind,
        channel_login: channel_login.to_string(),
        source_filenames: source_filenames.clone(),
        status: RecordingJobStatus::Queued,
        expected_filename: expected_filename.to_string(),
        final_filename: None,
        error: None,
        created_at: now,
        updated_at: now,
    };

    {
        let mut jobs = state.recording_jobs.write().await;
        jobs.insert(job_id.clone(), job);
    }

    let state_for_task = state.clone();
    let job_id_for_task = job_id.clone();
    let channel_for_task = channel_login.to_string();

    tokio::spawn(async move {
        // Transition to Running
        {
            let mut jobs = state_for_task.recording_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id_for_task) {
                job.status = RecordingJobStatus::Running;
                job.updated_at = crate::util::time::now_unix_secs();
            }
        }

        // Execute the job future
        let result = job_future.await;

        // Update job status and cleanup guard
        let mut jobs = state_for_task.recording_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id_for_task) {
            match result {
                Ok(file) => {
                    job.status = RecordingJobStatus::Completed;
                    job.final_filename = Some(file.filename);
                    job.updated_at = crate::util::time::now_unix_secs();
                }
                Err(error) => {
                    job.status = RecordingJobStatus::Failed;
                    job.error = Some(error.to_string());
                    job.updated_at = crate::util::time::now_unix_secs();
                }
            }
        }

        let mut guard = state_for_task.active_processing_guard.write().await;
        guard.remove(&channel_for_task);
    });

    Ok(job_id)
}

async fn merge_recordings(
    State(state): State<RecordingState>,
    Json(payload): Json<MergeRecordingsRequest>,
) -> Response {
    if payload.filenames.len() < 2 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "at least 2 files are required for merging",
            None,
        );
    }

    let (normalized_channel, expected_filename) = match state
        .service
        .validate_merge_request(&payload.channel_login, &payload.filenames)
    {
        Ok(value) => value,
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            return error_response(status, message, None);
        }
    };

    let source_filenames = payload.filenames.clone();
    let source_count = source_filenames.len();

    // Create the service call as a future
    let service = state.service.clone();
    let channel_for_task = normalized_channel.clone();
    let filenames_for_task = source_filenames.clone();
    let expected_for_task = expected_filename.clone();

    let job_id = match spawn_recording_job(
        &state,
        RecordingJobKind::Merge,
        &normalized_channel,
        source_filenames,
        &expected_filename,
        async move {
            service
                .merge_incomplete_recordings(
                    &channel_for_task,
                    filenames_for_task,
                    &expected_for_task,
                )
                .await
        },
    )
    .await
    {
        Ok(id) => id,
        Err((status, message)) => return error_response(status, message, None),
    };

    (
        StatusCode::ACCEPTED,
        Json(RecordingJobAcceptedResponse {
            job_id,
            kind: RecordingJobKind::Merge,
            channel_login: normalized_channel,
            expected_filename,
            source_count,
        }),
    )
        .into_response()
}

async fn finalize_incomplete_recording(
    State(state): State<RecordingState>,
    Json(payload): Json<FinalizeIncompleteRecordingRequest>,
) -> Response {
    let (normalized_channel, expected_filename) = match state
        .service
        .validate_finalize_request(&payload.channel_login, &payload.filename)
    {
        Ok(value) => value,
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            return error_response(status, message, None);
        }
    };

    let source_filename = payload.filename;

    // Create the service call as a future
    let service = state.service.clone();
    let channel_for_task = normalized_channel.clone();
    let filename_for_task = source_filename.clone();
    let expected_for_task = expected_filename.clone();

    let job_id = match spawn_recording_job(
        &state,
        RecordingJobKind::Finalize,
        &normalized_channel,
        vec![source_filename],
        &expected_filename,
        async move {
            service
                .finalize_incomplete_recording(
                    &channel_for_task,
                    &filename_for_task,
                    &expected_for_task,
                )
                .await
        },
    )
    .await
    {
        Ok(id) => id,
        Err((status, message)) => return error_response(status, message, None),
    };

    (
        StatusCode::ACCEPTED,
        Json(RecordingJobAcceptedResponse {
            job_id,
            kind: RecordingJobKind::Finalize,
            channel_login: normalized_channel,
            expected_filename,
            source_count: 1,
        }),
    )
        .into_response()
}

async fn get_recording_job_status(
    State(state): State<RecordingState>,
    Path(job_id): Path<String>,
) -> Response {
    let jobs = state.recording_jobs.read().await;
    let Some(job) = jobs.get(&job_id) else {
        return error_response(StatusCode::NOT_FOUND, "recording job not found", None);
    };

    (
        StatusCode::OK,
        Json(RecordingJobStatusResponse {
            job_id: &job.job_id,
            kind: job.kind,
            status: job.status,
            channel_login: &job.channel_login,
            expected_filename: &job.expected_filename,
            final_filename: job.final_filename.as_deref(),
            error: job.error.as_deref(),
        }),
    )
        .into_response()
}

async fn repair_recording(
    State(state): State<RecordingState>,
    Json(payload): Json<RepairRecordingRequest>,
) -> Response {
    match state
        .service
        .repair_completed_recording(&payload.channel_login, &payload.filename)
        .await
    {
        Ok(repaired_file) => (
            StatusCode::OK,
            Json(RepairRecordingResponse { repaired_file }),
        )
            .into_response(),
        Err(error) => {
            let (status, message) = classify_recording_error(&error);
            if status == StatusCode::INTERNAL_SERVER_ERROR {
                tracing::error!(error = %error, "recording repair failed");
            }
            error_response(status, message, None)
        }
    }
}

fn classify_recording_error(error: &RecordingError) -> (StatusCode, &'static str) {
    match error {
        RecordingError::InvalidChannelLogin(_) => {
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
        RecordingError::MergeFailed(_) => {
            (StatusCode::BAD_REQUEST, "recording merge request failed")
        }
        RecordingError::RepairFailed(_) => {
            (StatusCode::BAD_REQUEST, "recording repair request failed")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording::RecordingError;

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

    #[test]
    fn classify_recording_error_maps_merge_failed() {
        let (status, message) =
            classify_recording_error(&RecordingError::MergeFailed("some error".to_string()));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(message, "recording merge request failed");
    }

    #[test]
    fn classify_recording_error_maps_repair_failed() {
        let (status, message) =
            classify_recording_error(&RecordingError::RepairFailed("some error".to_string()));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(message, "recording repair request failed");
    }
}
