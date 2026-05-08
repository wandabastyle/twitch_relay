//! File I/O helpers for atomic writes and optional loading.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Ensures the parent directory of the given path exists.
///
/// Creates parent directories recursively if they don't exist.
/// Returns an error if the directory creation fails.
pub fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create directory failed: {e}"))?;
    }
    Ok(())
}

/// Loads and deserializes JSON from a file, returning `None` if the file doesn't exist.
///
/// Returns an error for other IO errors or JSON parse failures.
pub fn load_json_optional<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read file failed: {e}")),
    };

    if text.trim().is_empty() {
        return Ok(None);
    }

    serde_json::from_str(&text).map_err(|e| format!("parse json failed: {e}"))
}

/// Loads and deserializes TOML from a file, returning `None` if the file doesn't exist.
///
/// Returns an error for other IO errors or TOML parse failures.
pub fn load_toml_optional<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read file failed: {e}")),
    };

    if text.trim().is_empty() {
        return Ok(None);
    }

    toml::from_str(&text).map_err(|e| format!("parse toml failed: {e}"))
}

/// Writes text directly to a file, creating parent directories if needed.
pub fn write_text(path: &Path, content: &str) -> Result<(), String> {
    ensure_parent_dir(path)?;
    fs::write(path, content).map_err(|e| format!("write file failed: {e}"))
}

/// Writes text atomically to a file using a temp file and rename.
///
/// Creates parent directories if needed. The temp file uses the same extension
/// as the target path with ".tmp" appended (e.g., "file.json" -> "file.json.tmp").
pub fn write_atomic_text(path: &Path, content: &str) -> Result<(), String> {
    ensure_parent_dir(path)?;

    let tmp = path.with_extension(format!(
        "{}.{}",
        path.extension().and_then(|e| e.to_str()).unwrap_or(""),
        "tmp"
    ));

    fs::write(&tmp, content).map_err(|e| format!("write temp file failed: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("replace file failed: {e}"))
}

/// Writes a JSON value atomically to a file.
///
/// Serializes with pretty formatting and uses atomic write.
pub fn write_json_pretty_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let encoded =
        serde_json::to_string_pretty(value).map_err(|e| format!("encode json failed: {e}"))?;
    write_atomic_text(path, &encoded)
}

/// Writes a TOML value atomically to a file.
///
/// Serializes with pretty formatting and uses atomic write.
pub fn write_toml_pretty_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let encoded = toml::to_string_pretty(value).map_err(|e| format!("encode toml failed: {e}"))?;
    write_atomic_text(path, &encoded)
}
