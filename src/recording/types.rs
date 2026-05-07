// Keep in sync with docs/api-contracts.md and web/src/lib/api-client/types.ts.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Typed error for recording operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecordingError {
    #[error("channel login cannot be empty")]
    EmptyChannelLogin,
    #[error("invalid quality")]
    InvalidQuality,
    #[error("recording already active")]
    AlreadyActive,
    #[error("recording not active")]
    NotActive,
    #[error("recording file not found")]
    FileNotFound,
    #[error("filename cannot be empty")]
    EmptyFilename,
    #[error("invalid filename")]
    InvalidFilename,
    #[error("recording delete failed: {0}")]
    DeleteFailed(String),
    #[error("recording pin failed: {0}")]
    PinFailed(String),
    #[error("recording unpin failed: {0}")]
    UnpinFailed(String),
    #[error("streamlink spawn failed: {0}")]
    SpawnFailed(String),
    #[error("recordings directory not writable: {0}")]
    DirectoryNotWritable(String),
    #[error("io error: {0}")]
    Io(String),
}

pub(super) const QUALITY_OPTIONS: [&str; 9] = [
    "best", "source", "1080p60", "1080p", "720p60", "720p", "480p", "360p", "160p",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct ChannelMetadataCache {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) poster_url: Option<String>,
    #[serde(default)]
    pub(super) tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingMode {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveRecording {
    pub channel_login: String,
    pub quality: String,
    pub started_at_unix: u64,
    pub output_path: String,
    pub pid: Option<u32>,
    pub mode: RecordingMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingFileEntry {
    pub channel_login: String,
    pub filename: String,
    pub path_display: String,
    pub status: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum RecordingBucket {
    Completed,
    Incomplete,
}

impl RecordingBucket {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Incomplete => "incomplete",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingsOverview {
    pub active: Vec<ActiveRecording>,
    pub completed: Vec<RecordingFileEntry>,
    pub incomplete: Vec<RecordingFileEntry>,
}

#[derive(Debug)]
pub(super) struct ActiveProcess {
    pub(super) metadata: ActiveRecording,
    pub(super) stream_title: Option<String>,
    pub(super) last_observed_game: Option<String>,
    pub(super) pending_game: Option<String>,
    pub(super) pending_game_confirmations: u64,
    pub(super) chapter_events: Vec<ChapterEvent>,
    pub(super) child: tokio::process::Child,
}

#[derive(Debug, Clone)]
pub(super) struct ChapterEvent {
    pub(super) offset_secs: u64,
    pub(super) title: String,
}

#[derive(Debug, Clone)]
pub struct RecordingProcessingConfig {
    pub ffmpeg_path: String,
    pub chapter_min_gap_secs: u64,
    pub chapter_change_confirmations: u64,
}
