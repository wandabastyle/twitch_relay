use axum::{
    Router,
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::{
    auth::{self, WebAuthConfig},
    recording::{
        RecordingError, RecordingFileEntry, RecordingJob, RecordingJobKind, RecordingJobStatus,
        RecordingService,
    },
    recording_progress::RecordingProgressStore,
};

// Submodules
mod handlers;
mod hls;
mod jobs;
mod playback;
mod rules;
pub mod types;

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

/// Build recording routes.
pub fn recording_routes(state: RecordingState, auth_config: WebAuthConfig) -> Router {
    Router::new()
        .route("/api/recordings/start", post(handlers::start_recording))
        .route("/api/recordings/stop", post(handlers::stop_recording))
        .route("/api/recordings/pin", post(handlers::pin_recording_file))
        .route(
            "/api/recordings/unpin",
            post(handlers::unpin_recording_file),
        )
        .route(
            "/api/recordings/delete",
            post(handlers::delete_recording_file),
        )
        .route("/api/recordings/merge", post(jobs::merge_recordings))
        .route(
            "/api/recordings/finalize-incomplete",
            post(jobs::finalize_incomplete_recording),
        )
        .route(
            "/api/recordings/jobs/{job_id}",
            get(rules::get_recording_job_status),
        )
        .route("/api/recordings/repair", post(handlers::repair_recording))
        .route(
            "/api/recordings/playback-file",
            get(playback::play_recording_asset),
        )
        .route("/api/recordings/hls-playlist", get(hls::serve_hls_playlist))
        .route(
            "/api/recordings/progress",
            get(handlers::get_recording_progress).put(handlers::put_recording_progress),
        )
        .route("/api/recordings", get(handlers::get_recordings))
        .route("/api/recording-rules", get(rules::get_recording_rules))
        .route("/api/recording-rules", post(rules::upsert_recording_rule))
        .route(
            "/api/recording-rules/{channel_login}",
            delete(rules::delete_recording_rule),
        )
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            auth_config,
            auth::require_session_middleware,
        ))
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
pub async fn spawn_recording_job<F>(
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
    use axum::http::StatusCode;

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

pub fn classify_recording_error(error: &RecordingError) -> (axum::http::StatusCode, &'static str) {
    use axum::http::StatusCode;

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
    use axum::http::StatusCode;

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
