//! Recording module - manages stream recordings, file operations, and metadata.

mod files;
mod merge;
mod metadata;
mod nfo;
mod playback;
mod process;
mod service;
mod types;

pub use service::RecordingService;
pub use types::{
    ActiveRecording, RecordingBucket, RecordingError, RecordingFileEntry, RecordingJob,
    RecordingJobKind, RecordingJobStatus, RecordingMode, RecordingProcessingConfig,
};
