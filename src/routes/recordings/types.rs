use serde::{
   Deserialize,
   Serialize,
};

use crate::{
   recording::{
      ActiveRecording,
      RecordingFileEntry,
      RecordingJobKind,
      RecordingJobStatus,
   },
   recording_rules::RecordingRule,
};

/// Request DTO for starting a recording.
#[derive(Debug, Deserialize)]
pub struct StartRecordingRequest {
   pub channel_login: String,
   #[serde(default)]
   pub quality:       Option<String>,
   #[serde(default)]
   pub stream_title:  Option<String>,
}

/// Request DTO for stopping a recording.
#[derive(Debug, Deserialize)]
pub struct StopRecordingRequest {
   pub channel_login: String,
}

/// Request DTO for upserting a recording rule.
#[derive(Debug, Deserialize)]
pub struct UpsertRecordingRuleRequest {
   pub channel_login:        String,
   pub enabled:              bool,
   #[serde(default)]
   pub quality:              Option<String>,
   #[serde(default)]
   pub stop_when_offline:    Option<bool>,
   #[serde(default)]
   pub max_duration_minutes: Option<u64>,
   #[serde(default)]
   pub keep_last_videos:     Option<u64>,
}

/// Request DTO for deleting a recording file.
#[derive(Debug, Deserialize)]
pub struct DeleteRecordingFileRequest {
   pub bucket:        String,
   pub channel_login: String,
   pub filename:      String,
}

/// Request DTO for pinning/unpinning a recording file.
#[derive(Debug, Deserialize)]
pub struct PinRecordingFileRequest {
   pub bucket:        String,
   pub channel_login: String,
   pub filename:      String,
}

/// Request DTO for merging incomplete recordings.
#[derive(Debug, Deserialize)]
pub struct MergeRecordingsRequest {
   pub channel_login: String,
   pub filenames:     Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RepairRecordingRequest {
   pub channel_login: String,
   pub filename:      String,
}

#[derive(Debug, Deserialize)]
pub struct FinalizeIncompleteRecordingRequest {
   pub channel_login: String,
   pub filename:      String,
}

/// Response DTO for async recording job accept.
#[derive(Debug, Serialize)]
pub struct RecordingJobAcceptedResponse {
   pub job_id:            String,
   pub kind:              RecordingJobKind,
   pub channel_login:     String,
   pub expected_filename: String,
   pub source_count:      usize,
}

#[derive(Debug, Serialize)]
pub struct RecordingJobStatusResponse<'a> {
   pub job_id:            &'a str,
   pub kind:              RecordingJobKind,
   pub status:            RecordingJobStatus,
   pub channel_login:     &'a str,
   pub expected_filename: &'a str,
   pub final_filename:    Option<&'a str>,
   pub error:             Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct RepairRecordingResponse {
   pub repaired_file: RecordingFileEntry,
}

/// Query parameters for playing a recording asset.
#[derive(Debug, Deserialize)]
pub struct PlayRecordingAssetQuery {
   pub channel_login: String,
   pub filename:      String,
}

/// Response DTO for recording rules.
#[derive(Debug, Serialize)]
pub struct RecordingRulesResponse {
   pub rules: Vec<RecordingRule>,
}

/// Response DTO for recordings overview.
#[derive(Debug, Serialize)]
pub struct RecordingsResponse {
   pub active:     Vec<ActiveRecording>,
   pub completed:  Vec<RecordingFileEntry>,
   pub incomplete: Vec<RecordingFileEntry>,
}

/// Query parameters for serving an HLS playlist.
#[derive(Debug, Deserialize)]
pub struct ServeHlsPlaylistQuery {
   pub channel_login: String,
   pub filename:      String,
}

/// Query parameters for recording watch progress.
#[derive(Debug, Deserialize)]
pub struct RecordingWatchProgressQuery {
   pub channel_login: String,
   pub filename:      String,
}

#[derive(Debug, Deserialize)]
pub struct RecordingWatchProgressUpdateRequest {
   pub channel_login: String,
   pub filename:      String,
   pub position_secs: f64,
   #[serde(default)]
   pub duration_secs: Option<f64>,
   #[serde(default)]
   pub completed:     Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RecordingWatchProgressResponse {
   pub channel_login:   String,
   pub filename:        String,
   pub position_secs:   Option<f64>,
   pub duration_secs:   Option<f64>,
   pub updated_at_unix: Option<u64>,
   pub completed:       bool,
}
