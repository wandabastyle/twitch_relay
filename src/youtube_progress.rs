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
pub struct YoutubeWatchProgressEntry {
    pub position_secs: f64,
    pub duration_secs: Option<f64>,
    pub completed: bool,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone)]
pub struct YoutubeProgressUpsertResult {
    pub previous: Option<YoutubeWatchProgressEntry>,
    pub current: YoutubeWatchProgressEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredYoutubeWatchProgress {
    sessions: HashMap<String, HashMap<String, YoutubeWatchProgressEntry>>,
}

#[derive(Debug, Clone)]
pub struct YoutubeProgressStore {
    inner: Arc<RwLock<StoredYoutubeWatchProgress>>,
}

impl YoutubeProgressStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(load_watch_progress())),
        }
    }

    pub fn get(&self, session_token: &str, video_id: &str) -> Option<YoutubeWatchProgressEntry> {
        let guard = self.inner.read().ok()?;
        guard
            .sessions
            .get(session_token)
            .and_then(|videos| videos.get(video_id))
            .cloned()
    }

    pub fn upsert(
        &self,
        session_token: &str,
        video_id: &str,
        position_secs: f64,
        duration_secs: Option<f64>,
        completed: Option<bool>,
    ) -> Option<YoutubeProgressUpsertResult> {
        let normalized_position = normalize_secs(position_secs)?;
        let normalized_duration = duration_secs.and_then(normalize_secs);
        let normalized_completed = completed.unwrap_or_else(|| {
            normalized_duration
                .map(|duration| duration > 0.0 && duration - normalized_position <= 20.0)
                .unwrap_or(false)
        });

        let entry = YoutubeWatchProgressEntry {
            position_secs: normalized_position,
            duration_secs: normalized_duration,
            completed: normalized_completed,
            updated_at_unix: now_unix_secs(),
        };

        if let Ok(mut guard) = self.inner.write() {
            let videos = guard
                .sessions
                .entry(session_token.to_string())
                .or_insert_with(HashMap::new);
            let previous = videos.insert(video_id.to_string(), entry.clone());
            let _ = save_watch_progress(&guard);
            return Some(YoutubeProgressUpsertResult {
                previous,
                current: entry,
            });
        }

        None
    }
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

fn load_watch_progress() -> StoredYoutubeWatchProgress {
    let Some(path) = paths::youtube_watch_progress_path() else {
        return StoredYoutubeWatchProgress::default();
    };

    match files::load_json_optional::<StoredYoutubeWatchProgress>(&path) {
        Ok(Some(data)) => data,
        Ok(None) => StoredYoutubeWatchProgress::default(),
        Err(error) => {
            tracing::warn!(error = %error, path = %path.display(), "failed to load youtube watch progress");
            StoredYoutubeWatchProgress::default()
        }
    }
}

fn save_watch_progress(data: &StoredYoutubeWatchProgress) -> Result<(), String> {
    let Some(path) = paths::youtube_watch_progress_path() else {
        return Err("failed to resolve youtube watch progress path".to_string());
    };

    files::write_json_pretty_atomic(&path, data)
}
