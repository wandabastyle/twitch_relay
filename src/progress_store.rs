use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::storage::files;
use crate::util::time::now_unix_secs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProgressEntry {
    pub position_secs: f64,
    pub duration_secs: Option<f64>,
    pub completed: bool,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone)]
pub struct ProgressUpsertResult {
    pub previous: Option<ProgressEntry>,
    pub current: ProgressEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoredProgress {
    pub sessions: HashMap<String, HashMap<String, ProgressEntry>>,
}

#[derive(Clone)]
pub struct ProgressStore {
    inner: Arc<RwLock<StoredProgress>>,
    path_resolver: Arc<dyn Fn() -> Option<std::path::PathBuf> + Send + Sync>,
}

impl std::fmt::Debug for ProgressStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressStore")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl ProgressStore {
    pub fn new<F>(path_resolver: F) -> Self
    where
        F: Fn() -> Option<std::path::PathBuf> + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(RwLock::new(Self::load(&path_resolver))),
            path_resolver: Arc::new(path_resolver),
        }
    }

    pub fn get(&self, session_token: &str, key: &str) -> Option<ProgressEntry> {
        let guard = self.inner.read().ok()?;
        guard
            .sessions
            .get(session_token)
            .and_then(|items| items.get(key))
            .cloned()
    }

    pub fn upsert(
        &self,
        session_token: &str,
        key: &str,
        position_secs: f64,
        duration_secs: Option<f64>,
        completed: Option<bool>,
    ) -> Option<ProgressUpsertResult> {
        let normalized_position = normalize_secs(position_secs)?;
        let normalized_duration = duration_secs.and_then(normalize_secs);
        let normalized_completed = completed.unwrap_or_else(|| {
            normalized_duration
                .map(|duration| duration > 0.0 && duration - normalized_position <= 20.0)
                .unwrap_or(false)
        });

        let entry = ProgressEntry {
            position_secs: normalized_position,
            duration_secs: normalized_duration,
            completed: normalized_completed,
            updated_at_unix: now_unix_secs(),
        };

        if let Ok(mut guard) = self.inner.write() {
            let items = guard
                .sessions
                .entry(session_token.to_string())
                .or_insert_with(HashMap::new);
            let previous = items.insert(key.to_string(), entry.clone());
            let _ = self.save(&guard);
            return Some(ProgressUpsertResult {
                previous,
                current: entry,
            });
        }

        None
    }

    fn load<F>(path_resolver: &F) -> StoredProgress
    where
        F: Fn() -> Option<std::path::PathBuf>,
    {
        let Some(path) = path_resolver() else {
            return StoredProgress::default();
        };

        match files::load_json_optional::<StoredProgress>(&path) {
            Ok(Some(data)) => data,
            Ok(None) => StoredProgress::default(),
            Err(error) => {
                tracing::warn!(error = %error, path = %path.display(), "failed to load progress");
                StoredProgress::default()
            }
        }
    }

    fn save(&self, data: &StoredProgress) -> Result<(), String> {
        let Some(path) = (self.path_resolver)() else {
            return Err("failed to resolve progress path".to_string());
        };

        files::write_json_pretty_atomic(&path, data)
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
