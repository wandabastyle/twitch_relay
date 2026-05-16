use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    recording_rules::{self, RecordingRule},
    routes::error::error_response,
};

use super::{
    RecordingState,
    types::{RecordingJobStatusResponse, RecordingRulesResponse, UpsertRecordingRuleRequest},
};

pub async fn get_recording_rules() -> Response {
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

pub async fn upsert_recording_rule(
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
    let quality = match crate::recording::RecordingService::validate_quality(&quality_input) {
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

pub async fn delete_recording_rule(Path(channel_login): Path<String>) -> Response {
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

pub async fn get_recording_job_status(
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
