use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct StoredChannels {
    channels: Vec<String>,
}

pub fn stored_channels_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "twitch-relay")?;
    Some(dirs.data_local_dir().join("channels.toml"))
}

pub fn load_stored_channels() -> Vec<String> {
    let path = match stored_channels_path() {
        Some(p) => p,
        None => return Vec::new(),
    };

    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    match toml::from_str::<StoredChannels>(&text) {
        Ok(stored) => stored.channels,
        Err(_) => Vec::new(),
    }
}

pub fn save_stored_channels(channels: &[String]) -> Result<(), String> {
    let Some(path) = stored_channels_path() else {
        return Err("unable to resolve config directory".to_string());
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create config directory failed: {e}"))?;
    }

    let payload = StoredChannels {
        channels: channels.to_vec(),
    };
    let encoded = toml::to_string_pretty(&payload)
        .map_err(|e| format!("encode channels config failed: {e}"))?;
    fs::write(path, encoded).map_err(|e| format!("write channels config failed: {e}"))
}
