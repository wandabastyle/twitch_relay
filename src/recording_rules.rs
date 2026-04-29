use std::{fs, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

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
    let dirs = ProjectDirs::from("", "", "twitch-relay")?;
    Some(dirs.data_local_dir().join("recording_rules.json"))
}

pub fn load_rules() -> Result<Vec<RecordingRule>, String> {
    let path = ensure_store_file()?;
    let text =
        fs::read_to_string(&path).map_err(|e| format!("read recording rules failed: {e}"))?;

    if text.trim().is_empty() {
        return Ok(Vec::new());
    }

    let payload: RecordingRulesPayload =
        serde_json::from_str(&text).map_err(|e| format!("parse recording rules failed: {e}"))?;

    Ok(normalize_dedup_rules(payload.rules))
}

pub fn save_rules(rules: &[RecordingRule]) -> Result<(), String> {
    let path = ensure_store_file()?;
    let normalized = normalize_dedup_rules(rules.to_vec());
    let payload = RecordingRulesPayload { rules: normalized };
    let encoded = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("encode recording rules failed: {e}"))?;

    atomic_write(&path, &encoded)
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

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create recording rules directory failed: {e}"))?;
    }

    if !path.exists() {
        atomic_write(&path, "{\n  \"rules\": []\n}")?;
    }

    Ok(path)
}

fn atomic_write(path: &PathBuf, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content).map_err(|e| format!("write recording rules temp file failed: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("replace recording rules file failed: {e}"))
}
