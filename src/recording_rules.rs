use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingRule {
    pub channel_login: String,
    pub enabled: bool,
    pub quality: String,
    pub stop_when_offline: bool,
    pub max_duration_minutes: Option<u64>,
    pub keep_last_videos: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RecordingRulesPayload {
    rules: Vec<RecordingRule>,
}

pub fn recording_rules_store_path() -> Option<PathBuf> {
    storage::paths::recording_rules_path()
}

pub fn load_rules() -> Result<Vec<RecordingRule>, String> {
    let path = ensure_store_file()?;

    match storage::files::load_json_optional::<RecordingRulesPayload>(&path)? {
        Some(payload) => Ok(normalize_dedup_rules(payload.rules)),
        None => Ok(Vec::new()),
    }
}

pub fn save_rules(rules: &[RecordingRule]) -> Result<(), String> {
    let path = ensure_store_file()?;
    let normalized = normalize_dedup_rules(rules.to_vec());
    let payload = RecordingRulesPayload { rules: normalized };

    storage::files::write_json_pretty_atomic(&path, &payload)
        .map_err(|e| format!("save recording rules failed: {e}"))
}

pub fn upsert_rule(rule: RecordingRule) -> Result<RecordingRule, String> {
    let mut rules = load_rules()?;
    let normalized_login = normalize_login(&rule.channel_login);
    let mut updated = RecordingRule {
        channel_login: normalized_login.clone(),
        ..rule
    };

    if let Some(existing) = rules
        .iter_mut()
        .find(|r| r.channel_login == normalized_login)
    {
        existing.enabled = updated.enabled;
        existing.quality = updated.quality.clone();
        existing.stop_when_offline = updated.stop_when_offline;
        existing.max_duration_minutes = updated.max_duration_minutes;
        existing.keep_last_videos = updated.keep_last_videos;
        updated = existing.clone();
    } else {
        rules.push(updated.clone());
    }

    save_rules(&rules)?;
    Ok(updated)
}

pub fn delete_rule(channel_login: &str) -> Result<bool, String> {
    let normalized = normalize_login(channel_login);
    let mut rules = load_rules()?;
    let original_len = rules.len();
    rules.retain(|r| r.channel_login != normalized);
    let removed = rules.len() != original_len;
    if removed {
        save_rules(&rules)?;
    }
    Ok(removed)
}

fn normalize_dedup_rules(rules: Vec<RecordingRule>) -> Vec<RecordingRule> {
    let mut out: Vec<RecordingRule> = Vec::new();
    for mut rule in rules {
        rule.channel_login = normalize_login(&rule.channel_login);
        if rule.channel_login.is_empty() {
            continue;
        }

        if let Some(existing) = out
            .iter_mut()
            .find(|r| r.channel_login == rule.channel_login)
        {
            *existing = rule;
        } else {
            out.push(rule);
        }
    }
    out.sort_by(|a, b| a.channel_login.cmp(&b.channel_login));
    out
}

fn normalize_login(login: &str) -> String {
    login.trim().to_ascii_lowercase()
}

fn ensure_store_file() -> Result<PathBuf, String> {
    let Some(path) = recording_rules_store_path() else {
        return Err("unable to resolve recording rules directory".to_string());
    };

    storage::files::ensure_parent_dir(&path)
        .map_err(|e| format!("create recording rules directory failed: {e}"))?;

    if !path.exists() {
        storage::files::write_atomic_text(&path, "{\n  \"rules\": []\n}")
            .map_err(|e| format!("create recording rules file failed: {e}"))?;
    }

    Ok(path)
}
