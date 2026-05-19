use axum::{
   Json,
   extract::State,
   http::StatusCode,
   response::{
      IntoResponse,
      Response,
   },
};

use super::{
   RecordingState,
   spawn_recording_job,
   types::{
      FinalizeIncompleteRecordingRequest,
      MergeRecordingsRequest,
      RecordingJobAcceptedResponse,
   },
};
use crate::{
   recording::RecordingJobKind,
   routes::error::error_response,
};

pub async fn merge_recordings(
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
         let (status, message) = super::classify_recording_error(&error);
         return error_response(status, message, None);
      },
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
            .merge_incomplete_recordings(&channel_for_task, filenames_for_task, &expected_for_task)
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

pub async fn finalize_incomplete_recording(
   State(state): State<RecordingState>,
   Json(payload): Json<FinalizeIncompleteRecordingRequest>,
) -> Response {
   let (normalized_channel, expected_filename) = match state
      .service
      .validate_finalize_request(&payload.channel_login, &payload.filename)
   {
      Ok(value) => value,
      Err(error) => {
         let (status, message) = super::classify_recording_error(&error);
         return error_response(status, message, None);
      },
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
