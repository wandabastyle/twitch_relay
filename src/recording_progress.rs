use crate::{progress_store::ProgressStore, storage::paths};

// Re-export for backward compatibility
pub use crate::progress_store::ProgressEntry as RecordingWatchProgressEntry;

#[derive(Debug, Clone)]
pub struct RecordingProgressStore {
    inner: ProgressStore,
}

impl RecordingProgressStore {
    pub fn new() -> Self {
        Self {
            inner: ProgressStore::new(|| paths::recording_watch_progress_path()),
        }
    }

    pub fn get(
        &self,
        session_token: &str,
        channel_login: &str,
        filename: &str,
    ) -> Option<RecordingWatchProgressEntry> {
        self.inner
            .get(session_token, &recording_key(channel_login, filename))
    }

    pub fn upsert(
        &self,
        session_token: &str,
        channel_login: &str,
        filename: &str,
        position_secs: f64,
        duration_secs: Option<f64>,
        completed: Option<bool>,
    ) -> Option<RecordingWatchProgressEntry> {
        let key = recording_key(channel_login, filename);
        self.inner
            .upsert(session_token, &key, position_secs, duration_secs, completed)
            .map(|result| result.current)
    }
}

impl Default for RecordingProgressStore {
    fn default() -> Self {
        Self::new()
    }
}

fn recording_key(channel_login: &str, filename: &str) -> String {
    format!("{channel_login}/{filename}")
}
