use axum::{
   Json,
   Router,
   extract::State,
   routing::get,
};
use serde::Serialize;

use crate::{
   recording::{
      ActiveRecording,
      RecordingJobKind,
      RecordingJobStatus,
   },
   routes::recordings::RecordingState,
   storage::paths::drain_sentinel_path,
};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize)]
pub struct ProbeResponse<'a> {
   pub status:  &'a str,
   pub service: &'a str,
}

#[derive(Debug, Serialize)]
pub struct VersionResponse<'a> {
   pub version: &'a str,
}

#[derive(Debug, Serialize)]
pub struct DeployPendingJob {
   pub job_id:        String,
   pub channel_login: String,
   pub kind:          RecordingJobKind,
   pub status:        RecordingJobStatus,
}

#[derive(Debug, Serialize)]
pub struct DeployInfo {
   pub drain_active:           bool,
   pub deploy_safe:            bool,
   pub active_recording_count: usize,
   pub processing_count:       usize,
   pub running_job_count:      usize,
   pub active_recordings:      Vec<ActiveRecording>,
   pub processing_channels:    Vec<String>,
   pub pending_jobs:           Vec<DeployPendingJob>,
}

pub fn health_routes(state: RecordingState) -> Router {
   Router::new()
      .route("/healthz", get(healthz))
      .route("/readyz", get(readyz))
      .route("/api/version", get(get_version))
      .route("/deployz", get(get_deploy_info))
      .with_state(state)
}

async fn healthz() -> Json<ProbeResponse<'static>> {
   Json(ProbeResponse {
      status:  "ok",
      service: "twitch-relay",
   })
}

async fn readyz() -> Json<ProbeResponse<'static>> {
   Json(ProbeResponse {
      status:  "ready",
      service: "twitch-relay",
   })
}

async fn get_version() -> Json<VersionResponse<'static>> {
   Json(VersionResponse {
      version: APP_VERSION,
   })
}

/// Public read‑only endpoint reporting deploy‑drain status and current
/// activity.
async fn get_deploy_info(State(state): State<RecordingState>) -> Json<DeployInfo> {
   let drain_active = drain_sentinel_path().is_some_and(|path| path.exists());
   let active_recordings = state.service.active_recordings().await;
   let mut processing_channels = state.service.post_processing_channels().await;

   {
      let guard = state.active_processing_guard.read().await;
      for channel in &*guard {
         if !processing_channels.contains(channel) {
            processing_channels.push(channel.clone());
         }
      }
   }
   processing_channels.sort();

   let pending_jobs = {
      let jobs = state.recording_jobs.read().await;
      let mut pending_jobs: Vec<DeployPendingJob> = jobs
         .values()
         .filter(|job| {
            matches!(
               job.status,
               RecordingJobStatus::Queued | RecordingJobStatus::Running
            )
         })
         .map(|job| {
            DeployPendingJob {
               job_id:        job.job_id.clone(),
               channel_login: job.channel_login.clone(),
               kind:          job.kind,
               status:        job.status,
            }
         })
         .collect();
      pending_jobs.sort_by(|left, right| left.job_id.cmp(&right.job_id));
      pending_jobs
   };

   let active_recording_count = active_recordings.len();
   let processing_count = processing_channels.len();
   let running_job_count = pending_jobs.len();

   Json(DeployInfo {
      drain_active,
      deploy_safe: active_recording_count == 0 && processing_count == 0 && running_job_count == 0,
      active_recording_count,
      processing_count,
      running_job_count,
      active_recordings,
      processing_channels,
      pending_jobs,
   })
}
