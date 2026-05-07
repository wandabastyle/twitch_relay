use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use time::format_description;

use super::nfo::{datetime_from_unix, next_same_day_suffix_index};
use super::types::*;

pub(super) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(super) fn sanitize_filename(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_ascii_lowercase();

    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }

    sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.len() > 64 {
        sanitized.truncate(64);
    }
    sanitized
}

pub(super) fn build_recording_filename(
    channel: &str,
    timestamp: u64,
    quality: &str,
    mode: RecordingMode,
    stream_title: Option<&str>,
) -> String {
    let mode = match mode {
        RecordingMode::Manual => "manual",
        RecordingMode::Auto => "auto",
    };
    let safe_channel = sanitize_filename(channel);
    let safe_quality = sanitize_filename(quality);
    let formatted_timestamp = format_filename_timestamp(timestamp);
    if let Some(title) = stream_title {
        let safe_title = sanitize_filename(title);
        if !safe_title.is_empty() {
            return format!(
                "{safe_channel}_{formatted_timestamp}_{safe_quality}_{mode}_{safe_title}.ts"
            );
        }
    }
    format!("{safe_channel}_{formatted_timestamp}_{safe_quality}_{mode}.ts")
}

pub(super) fn format_filename_timestamp(unix_secs: u64) -> String {
    let dt = datetime_from_unix(unix_secs);

    let Ok(format) = format_description::parse("[year]-[month]-[day]-[hour][minute]") else {
        return unix_secs.to_string();
    };

    dt.format(&format).unwrap_or_else(|_| unix_secs.to_string())
}

pub(super) fn validate_recording_filename(filename: &str) -> Result<String, RecordingError> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return Err(RecordingError::EmptyFilename);
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(RecordingError::InvalidFilename);
    }
    if trimmed == "." || trimmed == ".." {
        return Err(RecordingError::InvalidFilename);
    }

    Ok(trimmed.to_string())
}

pub(super) fn build_completed_recording_path(
    channel_dir: &Path,
    channel_login: &str,
    metadata: &ActiveRecording,
    stream_title: Option<&str>,
) -> PathBuf {
    let started = datetime_from_unix(metadata.started_at_unix);
    let season = started.year();
    let month = started.month() as u8;
    let day = started.day();
    let base_episode: u16 = u16::from(month) * 100 + u16::from(day);
    let aired = format!("{season:04}-{month:02}-{day:02}");
    let season_dir = channel_dir.join(format!("Season {season}"));
    let suffix = next_same_day_suffix_index(&season_dir, &aired, base_episode);
    let episode_number = base_episode.saturating_add(suffix);

    let title_slug = stream_title
        .map(sanitize_filename)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "stream".to_string());
    season_dir.join(format!(
        "{}_S{season:04}E{episode_number:04}_{title_slug}.ts",
        sanitize_filename(channel_login)
    ))
}

pub(super) fn move_file_if_exists(from: &Path, to: &Path) -> bool {
    if !from.exists() {
        return false;
    }
    if let Some(parent) = to.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::rename(from, to).is_ok()
}

pub(super) fn find_file_by_name_recursive(dir: &Path, filename: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path.file_name().and_then(|f| f.to_str()) == Some(filename) {
                return Some(path);
            }
            continue;
        }
        if path.is_dir()
            && let Some(found) = find_file_by_name_recursive(&path, filename)
        {
            return Some(found);
        }
    }
    None
}

pub(super) fn list_recording_files(
    dir: &Path,
    status: &str,
    limit: usize,
) -> Vec<RecordingFileEntry> {
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    collect_recording_files(dir, &mut entries);

    entries.sort_by_key(|(_, path)| {
        std::cmp::Reverse(
            fs::metadata(path)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH),
        )
    });

    entries
        .into_iter()
        .take(limit)
        .map(|(channel_login, path)| RecordingFileEntry {
            channel_login,
            filename: path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path_display: path.display().to_string(),
            status: status.to_string(),
            pinned: is_recording_pinned(&path),
        })
        .collect()
}

fn collect_recording_files(dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };

    for entry in read.flatten() {
        let path = entry.path();
        if path.is_file() {
            if !is_visible_recording_file(&path) {
                continue;
            }
            out.push((channel_login_for_recording(&path), path));
            continue;
        }
        if path.is_dir() {
            collect_recording_files(&path, out);
        }
    }
}

fn channel_login_for_recording(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect();
    for (index, part) in parts.iter().enumerate() {
        if (part == "completed" || part == "incomplete") && index + 1 < parts.len() {
            return parts[index + 1].clone();
        }
    }
    "unknown".to_string()
}

pub(super) fn is_visible_recording_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "ts" | "mp4" | "mkv" | "m4v" | "mov" | "webm"
            )
        })
        .unwrap_or(false)
}

pub(super) fn pin_marker_path_for_recording(recording_path: &Path) -> PathBuf {
    let file_name = recording_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("recording");
    recording_path.with_file_name(format!("{file_name}.pin"))
}

pub(super) fn is_recording_pinned(recording_path: &Path) -> bool {
    pin_marker_path_for_recording(recording_path).exists()
}

pub(super) fn prune_completed_channel_dir(dir: &Path, keep_last: usize) {
    let mut files: Vec<PathBuf> = Vec::new();
    collect_recording_media_paths(dir, &mut files);

    files.retain(|path| !is_recording_pinned(path));

    files.sort_by_key(|path| {
        std::cmp::Reverse(
            fs::metadata(path)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH),
        )
    });

    for old_path in files.into_iter().skip(keep_last) {
        if let Err(error) = fs::remove_file(&old_path) {
            tracing::warn!(
                path = %old_path.display(),
                error = %error,
                "failed to prune old completed recording"
            );
            continue;
        }
        let nfo = old_path.with_extension("nfo");
        if nfo.exists() {
            let _ = fs::remove_file(nfo);
        }
    }
}

fn collect_recording_media_paths(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if is_visible_recording_file(&path) {
                out.push(path);
            }
            continue;
        }
        if path.is_dir() {
            collect_recording_media_paths(&path, out);
        }
    }
}
