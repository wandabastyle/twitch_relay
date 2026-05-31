use axum::{
   Json,
   extract::{
      Query,
      State,
   },
   http::{
      HeaderMap,
      StatusCode,
   },
   response::{
      IntoResponse,
      Response,
   },
};

use super::{
   RecordingState,
   types::{
      DeleteRecordingFileRequest,
      PinRecordingFileRequest,
      RecordingWatchProgressQuery,
      RecordingWatchProgressResponse,
      RecordingWatchProgressUpdateRequest,
      RecordingsResponse,
      RepairRecordingRequest,
      StartRecordingRequest,
      StopRecordingRequest,
   },
};
use crate::{
   recording::{
      RecordingBucket,
      RecordingMode,
      RecordingService,
   },
   recording_progress::RecordingWatchProgressEntry,
   routes::error::error_response,
};

pub async fn start_recording(
   State(state): State<RecordingState>,
   Json(payload): Json<StartRecordingRequest>,
) -> Response {
   let quality = payload
      .quality
      .unwrap_or_else(|| state.default_quality.clone());
   let Ok(quality) = RecordingService::validate_quality(&quality) else {
      return error_response(StatusCode::BAD_REQUEST, "invalid quality", None);
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
         let (status, message) = super::classify_recording_error(&error);
         if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %error, "manual recording start failed");
         }
         error_response(status, message, None)
      },
   }
}

pub async fn stop_recording(
   State(state): State<RecordingState>,
   Json(payload): Json<StopRecordingRequest>,
) -> Response {
   match state.service.stop_recording(&payload.channel_login).await {
      Ok(active) => (StatusCode::OK, Json(active)).into_response(),
      Err(error) => {
         let (status, message) = super::classify_recording_error(&error);
         if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %error, "recording stop failed");
         }
         error_response(status, message, None)
      },
   }
}

pub async fn get_recordings(State(state): State<RecordingState>) -> Json<RecordingsResponse> {
   let overview = state.service.list_overview(15).await;
   Json(RecordingsResponse {
      active:     overview.active,
      completed:  overview.completed,
      incomplete: overview.incomplete,
   })
}

pub async fn delete_recording_file(
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
         let (status, message) = super::classify_recording_error(&error);
         if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %error, "recording file delete failed");
         }
         error_response(status, message, None)
      },
   }
}

pub async fn pin_recording_file(
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
         let (status, message) = super::classify_recording_error(&error);
         if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %error, "recording file pin failed");
         }
         error_response(status, message, None)
      },
   }
}

pub async fn unpin_recording_file(
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
         let (status, message) = super::classify_recording_error(&error);
         if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %error, "recording file unpin failed");
         }
         error_response(status, message, None)
      },
   }
}

pub async fn repair_recording(
   State(state): State<RecordingState>,
   Json(payload): Json<RepairRecordingRequest>,
) -> Response {
   match state
      .service
      .repair_completed_recording(&payload.channel_login, &payload.filename)
   {
      Ok(repaired_file) => {
         (
            StatusCode::OK,
            Json(super::types::RepairRecordingResponse { repaired_file }),
         )
            .into_response()
      },
      Err(error) => {
         let (status, message) = super::classify_recording_error(&error);
         if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %error, "recording repair failed");
         }
         error_response(status, message, None)
      },
   }
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

pub async fn get_recording_progress(
   State(state): State<RecordingState>,
   Query(query): Query<RecordingWatchProgressQuery>,
   request_headers: HeaderMap,
) -> Response {
   let Some(session_token) = state.auth.session_token_from_headers(&request_headers) else {
      return error_response(StatusCode::UNAUTHORIZED, "unauthorized", None);
   };

   let progress = state
      .progress
      .get(&session_token, &query.channel_login, &query.filename);
   recording_progress_response(query.channel_login, query.filename, progress)
}

pub async fn put_recording_progress(
   State(state): State<RecordingState>,
   request_headers: HeaderMap,
   Json(payload): Json<RecordingWatchProgressUpdateRequest>,
) -> Response {
   let Some(session_token) = state.auth.session_token_from_headers(&request_headers) else {
      return error_response(StatusCode::UNAUTHORIZED, "unauthorized", None);
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
      return error_response(
         StatusCode::BAD_REQUEST,
         "invalid progress payload or failed to persist",
         None,
      );
   };

   recording_progress_response(payload.channel_login, payload.filename, Some(entry))
}
