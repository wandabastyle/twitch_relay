use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use time::{OffsetDateTime, format_description};
use tokio::{process::Command, sync::RwLock};

const QUALITY_OPTIONS: [&str; 9] = [
    "best", "source", "1080p60", "1080p", "720p60", "720p", "480p", "360p", "160p",
];

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingMode {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveRecording {
    pub channel_login: String,
    pub quality: String,
    pub started_at_unix: u64,
    pub output_path: String,
    pub pid: Option<u32>,
    pub mode: RecordingMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingFileEntry {
    pub filename: String,
    pub path_display: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingsOverview {
    pub active: Vec<ActiveRecording>,
    pub completed: Vec<RecordingFileEntry>,
    pub incomplete: Vec<RecordingFileEntry>,
}

#[derive(Debug)]
struct ActiveProcess {
    metadata: ActiveRecording,
    child: tokio::process::Child,
}

#[derive(Debug, Clone)]
pub struct RecordingService {
    streamlink_path: String,
    recordings_dir: PathBuf,
    active: Arc<RwLock<HashMap<String, ActiveProcess>>>,
}

impl RecordingService {
    pub fn new(streamlink_path: String, recordings_dir: String) -> Result<Self, String> {
        let service = Self {
            streamlink_path,
            recordings_dir: PathBuf::from(recordings_dir),
            active: Arc::new(RwLock::new(HashMap::new())),
        };
        service.ensure_directories()?;
        service.cleanup_startup_tmp()?;
        Ok(service)
    }

    pub fn validate_quality(quality: &str) -> Result<String, String> {
        let normalized = quality.trim().to_ascii_lowercase();
        if QUALITY_OPTIONS.contains(&normalized.as_str()) {
            Ok(normalized)
        } else {
            Err("invalid quality".to_string())
        }
    }

    pub fn normalize_channel_login(channel_login: &str) -> Result<String, String> {
        let normalized = channel_login.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err("channel login cannot be empty".to_string());
        }
        Ok(normalized)
    }

    pub async fn start_recording(
        &self,
        channel_login: &str,
        quality: &str,
        mode: RecordingMode,
        stream_title: Option<&str>,
    ) -> Result<ActiveRecording, String> {
        let channel_login = Self::normalize_channel_login(channel_login)?;
        let quality = Self::validate_quality(quality)?;

        self.reconcile_exited_recordings().await;

        {
            let active = self.active.read().await;
            if active.contains_key(&channel_login) {
                return Err("recording already active for this channel".to_string());
            }
        }

        let started_at_unix = now_unix();
        let filename =
            build_recording_filename(&channel_login, started_at_unix, &quality, mode, stream_title);
        let output_path = self.channel_bucket_dir("tmp", &channel_login).join(filename);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("recordings directory not writable: {e}"))?;
        }

        let mut command = Command::new(&self.streamlink_path);
        command
            .arg(format!("https://twitch.tv/{channel_login}"))
            .arg(&quality)
            .arg("-o")
            .arg(&output_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let child = command
            .spawn()
            .map_err(|e| format!("streamlink spawn failed: {e}"))?;

        let pid = child.id();
        let metadata = ActiveRecording {
            channel_login: channel_login.clone(),
            quality: quality.clone(),
            started_at_unix,
            output_path: output_path.display().to_string(),
            pid,
            mode,
            error: None,
        };

        {
            let mut active = self.active.write().await;
            active.insert(
                channel_login.clone(),
                ActiveProcess {
                    metadata: metadata.clone(),
                    child,
                },
            );
        }

        tracing::info!(
            channel = %channel_login,
            quality = %quality,
            mode = ?mode,
            output_path = %metadata.output_path,
            "recording started"
        );

        Ok(metadata)
    }

    pub async fn stop_recording(&self, channel_login: &str) -> Result<ActiveRecording, String> {
        self.reconcile_exited_recordings().await;

        let channel_login = Self::normalize_channel_login(channel_login)?;
        let mut process = {
            let mut active = self.active.write().await;
            active.remove(&channel_login)
        }
        .ok_or_else(|| "recording not active for this channel".to_string())?;

        let _ = process.child.kill().await;
        let _ = process.child.wait().await;

        let output_path = PathBuf::from(&process.metadata.output_path);
        if output_path.exists() {
            let final_path = self.channel_bucket_dir("completed", &channel_login).join(
                output_path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("recording.ts"),
            );
            move_file_if_exists(&output_path, &final_path);
            tracing::info!(from = %output_path.display(), to = %final_path.display(), "recording moved to completed");
        }

        tracing::info!(channel = %channel_login, "recording stopped");
        Ok(process.metadata)
    }

    pub async fn active_recordings(&self) -> Vec<ActiveRecording> {
        self.reconcile_exited_recordings().await;

        let active = self.active.read().await;
        let mut items: Vec<ActiveRecording> = active.values().map(|p| p.metadata.clone()).collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.started_at_unix));
        items
    }

    pub async fn get_active_recording(&self, channel_login: &str) -> Option<ActiveRecording> {
        self.reconcile_exited_recordings().await;

        let active = self.active.read().await;
        active
            .get(&channel_login.trim().to_ascii_lowercase())
            .map(|p| p.metadata.clone())
    }

    pub async fn list_overview(&self, limit_per_bucket: usize) -> RecordingsOverview {
        self.reconcile_exited_recordings().await;

        RecordingsOverview {
            active: self.active_recordings().await,
            completed: list_recording_files(&self.completed_dir(), "completed", limit_per_bucket),
            incomplete: list_recording_files(&self.incomplete_dir(), "incomplete", limit_per_bucket),
        }
    }

    async fn reconcile_exited_recordings(&self) {
        let mut finished: Vec<(String, ActiveProcess, std::process::ExitStatus)> = Vec::new();

        {
            let mut active = self.active.write().await;
            let keys: Vec<String> = active.keys().cloned().collect();
            for key in keys {
                let status = match active.get_mut(&key) {
                    Some(process) => match process.child.try_wait() {
                        Ok(status) => status,
                        Err(error) => {
                            tracing::error!(channel = %key, error = %error, "failed to poll recording process status");
                            None
                        }
                    },
                    None => None,
                };

                if let Some(status) = status
                    && let Some(process) = active.remove(&key)
                {
                    finished.push((key, process, status));
                }
            }
        }

        for (channel_login, process, exit) in finished {
            self.finalize_exited_process(&channel_login, &process.metadata.output_path, exit);
        }
    }

    fn finalize_exited_process(
        &self,
        channel_login: &str,
        output_path: &str,
        exit: std::process::ExitStatus,
    ) {
        let output_path = PathBuf::from(output_path);
        if !output_path.exists() {
            tracing::info!(channel = %channel_login, status = ?exit, "recording process exited with no output file present");
            return;
        }

        let filename = output_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("recording.ts");

        if exit.success() {
            let final_path = self.channel_bucket_dir("completed", channel_login).join(filename);
            move_file_if_exists(&output_path, &final_path);
            tracing::info!(
                channel = %channel_login,
                status = ?exit,
                from = %output_path.display(),
                to = %final_path.display(),
                "recording exited cleanly"
            );
            return;
        }

        let final_path = self.channel_bucket_dir("incomplete", channel_login).join(filename);
        move_file_if_exists(&output_path, &final_path);
        tracing::warn!(
            channel = %channel_login,
            status = ?exit,
            from = %output_path.display(),
            to = %final_path.display(),
            "recording exited abnormally"
        );
    }

    fn ensure_directories(&self) -> Result<(), String> {
        fs::create_dir_all(self.tmp_dir()).map_err(|e| format!("recordings directory not writable: {e}"))?;
        fs::create_dir_all(self.completed_dir())
            .map_err(|e| format!("recordings directory not writable: {e}"))?;
        fs::create_dir_all(self.incomplete_dir())
            .map_err(|e| format!("recordings directory not writable: {e}"))?;
        Ok(())
    }

    fn cleanup_startup_tmp(&self) -> Result<(), String> {
        let tmp = self.tmp_dir();
        let incomplete = self.incomplete_dir();
        let entries =
            fs::read_dir(&tmp).map_err(|e| format!("read recordings tmp directory failed: {e}"))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let target = incomplete.join(
                    path.file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("unknown.ts"),
                );
                if move_file_if_exists(&path, &target) {
                    tracing::info!(from = %path.display(), to = %target.display(), "startup recording tmp cleanup moved file");
                }
                continue;
            }

            if path.is_dir() {
                let Some(channel_dir) = path.file_name().and_then(|f| f.to_str()) else {
                    continue;
                };
                let nested = match fs::read_dir(&path) {
                    Ok(entries) => entries,
                    Err(_) => continue,
                };
                for nested_entry in nested.flatten() {
                    let nested_path = nested_entry.path();
                    if !nested_path.is_file() {
                        continue;
                    }
                    let target = self.channel_bucket_dir("incomplete", channel_dir).join(
                        nested_path
                            .file_name()
                            .and_then(|f| f.to_str())
                            .unwrap_or("unknown.ts"),
                    );
                    if move_file_if_exists(&nested_path, &target) {
                        tracing::info!(from = %nested_path.display(), to = %target.display(), "startup recording tmp cleanup moved file");
                    }
                }
            }
        }
        Ok(())
    }

    pub fn tmp_dir(&self) -> PathBuf {
        self.recordings_dir.join("tmp")
    }

    pub fn completed_dir(&self) -> PathBuf {
        self.recordings_dir.join("completed")
    }

    pub fn incomplete_dir(&self) -> PathBuf {
        self.recordings_dir.join("incomplete")
    }

    fn channel_bucket_dir(&self, bucket: &str, channel_login: &str) -> PathBuf {
        self.recordings_dir
            .join(bucket)
            .join(sanitize_filename(channel_login))
    }
}

fn build_recording_filename(
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

fn format_filename_timestamp(unix_secs: u64) -> String {
    let Ok(unix) = i64::try_from(unix_secs) else {
        return unix_secs.to_string();
    };

    let Ok(dt) = OffsetDateTime::from_unix_timestamp(unix) else {
        return unix_secs.to_string();
    };

    let Ok(format) = format_description::parse("[year]-[month]-[day]-[hour][minute]") else {
        return unix_secs.to_string();
    };

    dt.format(&format).unwrap_or_else(|_| unix_secs.to_string())
}

fn sanitize_filename(value: &str) -> String {
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

fn list_recording_files(dir: &Path, status: &str, limit: usize) -> Vec<RecordingFileEntry> {
    let mut entries = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_file() {
                entries.push(path);
                continue;
            }

            if path.is_dir() && let Ok(nested) = fs::read_dir(path) {
                for nested_entry in nested.flatten() {
                    let nested_path = nested_entry.path();
                    if nested_path.is_file() {
                        entries.push(nested_path);
                    }
                }
            }
        }
    }

    entries.sort_by_key(|path| {
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
        .map(|path| RecordingFileEntry {
            filename: path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path_display: path.display().to_string(),
            status: status.to_string(),
        })
        .collect()
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn move_file_if_exists(from: &Path, to: &Path) -> bool {
    if !from.exists() {
        return false;
    }
    if let Some(parent) = to.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::rename(from, to).is_ok()
}
