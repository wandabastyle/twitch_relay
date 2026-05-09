use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::storage;
use crate::util::time::now_unix_secs;

const RESUME_MIN_SECS: u32 = 30;
const FRESHNESS_TTL_SECS: u64 = 90 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeProgressEntry {
    pub session_id: String,
    pub video_id: String,
    pub position_secs: u32,
    pub duration_secs: Option<u32>,
    pub updated_at_unix: u64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct YoutubeProgressFile {
    entries: Vec<YoutubeProgressEntry>,
}

#[derive(Debug, Clone)]
pub struct YoutubeWatchProgressStore {
    path: Option<PathBuf>,
    entries: Arc<RwLock<HashMap<(String, String), YoutubeProgressEntry>>>,
}

#[derive(Debug, Clone)]
pub struct YoutubeProgressSnapshot {
    pub position_secs: u32,
    pub duration_secs: Option<u32>,
    pub should_resume: bool,
}

impl YoutubeWatchProgressStore {
    pub fn new() -> Self {
        let path = storage::paths::youtube_watch_progress_path();
        let map = path
            .as_ref()
            .and_then(|p| load_file_entries(p).ok())
            .unwrap_or_default();

        Self {
            path,
            entries: Arc::new(RwLock::new(map)),
        }
    }

    pub fn get(&self, session_id: &str, video_id: &str) -> Option<YoutubeProgressSnapshot> {
        let key = (session_id.to_string(), video_id.to_string());
        let guard = self.entries.read().ok()?;
        let entry = guard.get(&key)?;
        if entry.completed {
            return None;
        }

        Some(YoutubeProgressSnapshot {
            position_secs: entry.position_secs,
            duration_secs: entry.duration_secs,
            should_resume: should_resume(entry.position_secs, entry.duration_secs),
        })
    }

    pub fn get_entry(&self, session_id: &str, video_id: &str) -> Option<YoutubeProgressEntry> {
        let key = (session_id.to_string(), video_id.to_string());
        let guard = self.entries.read().ok()?;
        guard.get(&key).cloned()
    }

    pub fn upsert(
        &self,
        session_id: &str,
        video_id: &str,
        position_secs: u32,
        duration_secs: Option<u32>,
        completed: bool,
    ) {
        let now = now_unix_secs();
        if let Ok(mut guard) = self.entries.write() {
            prune_stale(&mut guard, now);
            let key = (session_id.to_string(), video_id.to_string());
            guard.insert(
                key,
                YoutubeProgressEntry {
                    session_id: session_id.to_string(),
                    video_id: video_id.to_string(),
                    position_secs,
                    duration_secs,
                    updated_at_unix: now,
                    completed,
                },
            );
            persist_entries(self.path.as_deref(), &guard);
        }
    }
}

fn load_file_entries(
    path: &Path,
) -> Result<HashMap<(String, String), YoutubeProgressEntry>, String> {
    let file = storage::files::load_toml_optional::<YoutubeProgressFile>(path)?.unwrap_or_default();
    let now = now_unix_secs();
    let mut map = HashMap::new();
    for entry in file.entries {
        if now.saturating_sub(entry.updated_at_unix) > FRESHNESS_TTL_SECS {
            continue;
        }
        map.insert((entry.session_id.clone(), entry.video_id.clone()), entry);
    }
    Ok(map)
}

fn persist_entries(path: Option<&Path>, entries: &HashMap<(String, String), YoutubeProgressEntry>) {
    let Some(path) = path else {
        return;
    };

    let mut values: Vec<YoutubeProgressEntry> = entries.values().cloned().collect();
    values.sort_by(|a, b| {
        a.session_id
            .cmp(&b.session_id)
            .then_with(|| a.video_id.cmp(&b.video_id))
    });

    let file = YoutubeProgressFile { entries: values };
    let _ = storage::files::write_toml_pretty_atomic(path, &file);
}

fn prune_stale(entries: &mut HashMap<(String, String), YoutubeProgressEntry>, now: u64) {
    entries.retain(|_, entry| now.saturating_sub(entry.updated_at_unix) <= FRESHNESS_TTL_SECS);
}

fn should_resume(position_secs: u32, duration_secs: Option<u32>) -> bool {
    if position_secs <= RESUME_MIN_SECS {
        return false;
    }

    if let Some(duration) = duration_secs {
        if duration > position_secs {
            return duration.saturating_sub(position_secs) > RESUME_MIN_SECS;
        }
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::should_resume;

    #[test]
    fn resume_requires_meaningful_progress() {
        assert!(!should_resume(0, None));
        assert!(!should_resume(30, None));
        assert!(should_resume(31, None));
    }

    #[test]
    fn resume_rejects_near_end() {
        assert!(should_resume(200, Some(400)));
        assert!(!should_resume(370, Some(400)));
        assert!(!should_resume(400, Some(400)));
    }
}
