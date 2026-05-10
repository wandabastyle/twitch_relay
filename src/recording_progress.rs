use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::{
    storage::{files, paths},
    util::time::now_unix_secs,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecordingWatchProgressEntry {
    pub position_secs: f64,
    pub duration_secs: Option<f64>,
    pub completed: bool,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredRecordingWatchProgress {
    sessions: HashMap<String, HashMap<String, RecordingWatchProgressEntry>>,
}

#[derive(Debug, Clone)]
pub struct RecordingProgressStore {
    inner: Arc<RwLock<StoredRecordingWatchProgress>>,
}

impl RecordingProgressStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(load_watch_progress())),
        }
    }

    pub fn get(
        &self,
        session_token: &str,
        channel_login: &str,
        filename: &str,
    ) -> Option<RecordingWatchProgressEntry> {
        let guard = self.inner.read().ok()?;
        guard
            .sessions
            .get(session_token)
            .and_then(|recordings| recordings.get(&recording_key(channel_login, filename)))
            .cloned()
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
        let normalized_position = normalize_secs(position_secs)?;
        let normalized_duration = duration_secs.and_then(normalize_secs);
        let normalized_completed = completed.unwrap_or_else(|| {
            normalized_duration
                .map(|duration| duration > 0.0 && duration - normalized_position <= 20.0)
                .unwrap_or(false)
        });
        let key = recording_key(channel_login, filename);

        let entry = RecordingWatchProgressEntry {
            position_secs: normalized_position,
            duration_secs: normalized_duration,
            completed: normalized_completed,
            updated_at_unix: now_unix_secs(),
        };

        if let Ok(mut guard) = self.inner.write() {
            let recordings = guard
                .sessions
                .entry(session_token.to_string())
                .or_insert_with(HashMap::new);
            recordings.insert(key, entry.clone());
            let _ = save_watch_progress(&guard);
            return Some(entry);
        }

        None
    }
}

fn recording_key(channel_login: &str, filename: &str) -> String {
    format!("{channel_login}/{filename}")
}

fn normalize_secs(value: f64) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    if value < 0.0 {
        return Some(0.0);
    }
    Some(value)
}

fn load_watch_progress() -> StoredRecordingWatchProgress {
    let Some(path) = paths::recording_watch_progress_path() else {
        return StoredRecordingWatchProgress::default();
    };

    match files::load_json_optional::<StoredRecordingWatchProgress>(&path) {
        Ok(Some(data)) => data,
        Ok(None) => StoredRecordingWatchProgress::default(),
        Err(error) => {
            tracing::warn!(
                error = %error,
                path = %path.display(),
                "failed to load recording watch progress"
            );
            StoredRecordingWatchProgress::default()
        }
    }
}

fn save_watch_progress(data: &StoredRecordingWatchProgress) -> Result<(), String> {
    let Some(path) = paths::recording_watch_progress_path() else {
        return Err("failed to resolve recording watch progress path".to_string());
    };

    files::write_json_pretty_atomic(&path, data)
}
