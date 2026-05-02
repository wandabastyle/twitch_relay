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

use crate::{config::RecordingNfoStyle, recording_rules};

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
    pub channel_login: String,
    pub filename: String,
    pub path_display: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy)]
pub enum RecordingBucket {
    Completed,
    Incomplete,
}

impl RecordingBucket {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Incomplete => "incomplete",
        }
    }
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
    stream_title: Option<String>,
    child: tokio::process::Child,
}

#[derive(Debug, Clone)]
pub struct RecordingService {
    streamlink_path: String,
    recordings_dir: PathBuf,
    write_nfo: bool,
    nfo_style: RecordingNfoStyle,
    active: Arc<RwLock<HashMap<String, ActiveProcess>>>,
}

impl RecordingService {
    pub fn new(
        streamlink_path: String,
        recordings_dir: String,
        write_nfo: bool,
        nfo_style: RecordingNfoStyle,
    ) -> Result<Self, String> {
        let service = Self {
            streamlink_path,
            recordings_dir: PathBuf::from(recordings_dir),
            write_nfo,
            nfo_style,
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
        let filename = build_recording_filename(
            &channel_login,
            started_at_unix,
            &quality,
            mode,
            stream_title,
        );
        let output_path = self
            .channel_bucket_dir("tmp", &channel_login)
            .join(filename);
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
                    stream_title: stream_title
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
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
            self.write_nfo_if_enabled(
                &channel_login,
                &final_path,
                &process.metadata,
                process.stream_title.as_deref(),
            );
            self.prune_completed_for_channel(&channel_login);
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
            incomplete: list_recording_files(
                &self.incomplete_dir(),
                "incomplete",
                limit_per_bucket,
            ),
        }
    }

    pub fn delete_recording_file(
        &self,
        bucket: RecordingBucket,
        channel_login: &str,
        filename: &str,
    ) -> Result<(), String> {
        let target_path = self.resolve_recording_file_path(bucket, channel_login, filename)?;

        if !target_path.exists() {
            return Err("recording file not found".to_string());
        }

        fs::remove_file(&target_path)
            .map_err(|error| format!("recording delete failed: {error}"))?;

        if matches!(bucket, RecordingBucket::Completed) {
            let nfo_path = target_path.with_extension("nfo");
            if nfo_path.exists() {
                fs::remove_file(&nfo_path)
                    .map_err(|error| format!("recording delete failed: {error}"))?;
            }
        }

        Ok(())
    }

    pub fn resolve_completed_file_path(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<PathBuf, String> {
        self.resolve_recording_file_path(RecordingBucket::Completed, channel_login, filename)
    }

    fn resolve_recording_file_path(
        &self,
        bucket: RecordingBucket,
        channel_login: &str,
        filename: &str,
    ) -> Result<PathBuf, String> {
        let channel_login = Self::normalize_channel_login(channel_login)?;
        let filename = validate_recording_filename(filename)?;
        Ok(self
            .channel_bucket_dir(bucket.as_str(), &channel_login)
            .join(filename))
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
            self.finalize_exited_process(&channel_login, &process, exit);
        }
    }

    fn finalize_exited_process(
        &self,
        channel_login: &str,
        process: &ActiveProcess,
        exit: std::process::ExitStatus,
    ) {
        let output_path = PathBuf::from(&process.metadata.output_path);
        if !output_path.exists() {
            tracing::info!(channel = %channel_login, status = ?exit, "recording process exited with no output file present");
            return;
        }

        let filename = output_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("recording.ts");

        if exit.success() {
            let final_path = self
                .channel_bucket_dir("completed", channel_login)
                .join(filename);
            move_file_if_exists(&output_path, &final_path);
            self.write_nfo_if_enabled(
                channel_login,
                &final_path,
                &process.metadata,
                process.stream_title.as_deref(),
            );
            self.prune_completed_for_channel(channel_login);
            tracing::info!(
                channel = %channel_login,
                status = ?exit,
                from = %output_path.display(),
                to = %final_path.display(),
                "recording exited cleanly"
            );
            return;
        }

        let final_path = self
            .channel_bucket_dir("incomplete", channel_login)
            .join(filename);
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
        fs::create_dir_all(self.tmp_dir())
            .map_err(|e| format!("recordings directory not writable: {e}"))?;
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

    fn prune_completed_for_channel(&self, channel_login: &str) {
        let keep_last = match recording_rules::load_rules() {
            Ok(rules) => rules
                .into_iter()
                .find(|rule| rule.channel_login == channel_login)
                .and_then(|rule| rule.keep_last_videos),
            Err(error) => {
                tracing::warn!(
                    channel = %channel_login,
                    error = %error,
                    "failed to load recording rules for pruning"
                );
                None
            }
        };

        let Some(keep_last) = keep_last else {
            return;
        };

        if keep_last == 0 {
            return;
        }

        prune_completed_channel_dir(
            &self.channel_bucket_dir("completed", channel_login),
            keep_last as usize,
        );
    }

    fn write_nfo_if_enabled(
        &self,
        channel_login: &str,
        recording_path: &Path,
        metadata: &ActiveRecording,
        stream_title: Option<&str>,
    ) {
        if !self.write_nfo {
            return;
        }

        if self.nfo_style != RecordingNfoStyle::Tv {
            return;
        }

        if let Err(error) = write_tv_nfo_file(channel_login, recording_path, metadata, stream_title)
        {
            tracing::warn!(
                channel = %channel_login,
                path = %recording_path.display(),
                error = %error,
                "failed to write recording nfo"
            );
        }
    }
}

fn write_tv_nfo_file(
    channel_login: &str,
    recording_path: &Path,
    metadata: &ActiveRecording,
    stream_title: Option<&str>,
) -> Result<(), String> {
    let Some(stem) = recording_path.file_stem().and_then(|value| value.to_str()) else {
        return Err("failed to derive recording basename".to_string());
    };

    let started = datetime_from_unix(metadata.started_at_unix);
    let season = started.year();
    let month = started.month() as u8;
    let day = started.day();
    let base_episode: u16 = u16::from(month) * 100 + u16::from(day);
    let aired = format!("{season:04}-{month:02}-{day:02}");

    let channel_dir = recording_path
        .parent()
        .ok_or_else(|| "recording file has no parent directory".to_string())?;
    let suffix_index = next_same_day_suffix_index(channel_dir, &aired, base_episode);
    let display_episode = if suffix_index == 0 {
        base_episode.to_string()
    } else {
        format!("{base_episode}-{suffix_index}")
    };

    let chosen_title = stream_title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{channel_login} stream {aired}"));
    let title = if suffix_index == 0 {
        chosen_title.clone()
    } else {
        format!("{chosen_title} ({display_episode})")
    };
    let mode = match metadata.mode {
        RecordingMode::Manual => "manual",
        RecordingMode::Auto => "auto",
    };
    let plot = format!(
        "Twitch recording for {channel_login}. Title: {chosen_title}. Quality: {}. Mode: {mode}.",
        metadata.quality
    );
    let uniqueid = format!(
        "{}-{}",
        sanitize_filename(channel_login),
        metadata.started_at_unix
    );

    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<episodedetails>\n  <title>{}</title>\n  <showtitle>{}</showtitle>\n  <season>{season}</season>\n  <episode>{base_episode}</episode>\n  <displayepisode>{}</displayepisode>\n  <aired>{aired}</aired>\n  <plot>{}</plot>\n  <uniqueid type=\"twitch-relay\" default=\"true\">{}</uniqueid>\n</episodedetails>\n",
        xml_escape(&title),
        xml_escape(channel_login),
        xml_escape(&display_episode),
        xml_escape(&plot),
        xml_escape(&uniqueid)
    );

    let nfo_path = recording_path.with_file_name(format!("{stem}.nfo"));
    fs::write(&nfo_path, xml)
        .map_err(|error| format!("failed to write nfo file {}: {error}", nfo_path.display()))
}

fn next_same_day_suffix_index(channel_dir: &Path, aired: &str, episode: u16) -> u16 {
    let Ok(entries) = fs::read_dir(channel_dir) else {
        return 0;
    };

    let mut max_suffix: i32 = -1;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_nfo = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("nfo"))
            .unwrap_or(false);
        if !is_nfo {
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if xml_tag_value(&content, "aired").as_deref() != Some(aired) {
            continue;
        }
        let Some(episode_value) = xml_tag_value(&content, "episode") else {
            continue;
        };
        if episode_value.trim().parse::<u16>().ok() != Some(episode) {
            continue;
        }

        let display =
            xml_tag_value(&content, "displayepisode").unwrap_or_else(|| episode_value.clone());
        let parsed = parse_display_episode_suffix(&display, episode);
        if parsed > max_suffix {
            max_suffix = parsed;
        }
    }

    (max_suffix + 1).max(0) as u16
}

fn parse_display_episode_suffix(display_episode: &str, episode: u16) -> i32 {
    let trimmed = display_episode.trim();
    let base = episode.to_string();
    if trimmed == base {
        return 0;
    }

    let Some(suffix) = trimmed.strip_prefix(&format!("{base}-")) else {
        return 0;
    };
    suffix.parse::<i32>().unwrap_or(0)
}

fn xml_tag_value(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = content.find(&open)? + open.len();
    let end_rel = content[start..].find(&close)?;
    Some(content[start..start + end_rel].trim().to_string())
}

fn datetime_from_unix(unix_secs: u64) -> OffsetDateTime {
    i64::try_from(unix_secs)
        .ok()
        .and_then(|unix| OffsetDateTime::from_unix_timestamp(unix).ok())
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
    let dt = datetime_from_unix(unix_secs);

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
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_file() {
                entries.push(("unknown".to_string(), path));
                continue;
            }

            if path.is_dir()
                && let Some(channel_login) = path.file_name().and_then(|f| f.to_str())
                && let Ok(nested) = fs::read_dir(&path)
            {
                for nested_entry in nested.flatten() {
                    let nested_path = nested_entry.path();
                    if nested_path.is_file() {
                        entries.push((channel_login.to_string(), nested_path));
                    }
                }
            }
        }
    }

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
        })
        .collect()
}

fn validate_recording_filename(filename: &str) -> Result<String, String> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return Err("filename cannot be empty".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("invalid filename".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("invalid filename".to_string());
    }

    Ok(trimmed.to_string())
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

fn prune_completed_channel_dir(dir: &Path, keep_last: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();

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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_display_episode_suffix_handles_base_and_indexed() {
        assert_eq!(parse_display_episode_suffix("502", 502), 0);
        assert_eq!(parse_display_episode_suffix("502-1", 502), 1);
        assert_eq!(parse_display_episode_suffix("502-12", 502), 12);
        assert_eq!(parse_display_episode_suffix("bad", 502), 0);
    }

    #[test]
    fn xml_escape_escapes_special_characters() {
        let escaped = xml_escape("A&B <C> \"D\" 'E'");
        assert_eq!(escaped, "A&amp;B &lt;C&gt; &quot;D&quot; &apos;E&apos;");
    }

    #[test]
    fn xml_tag_value_extracts_trimmed_value() {
        let xml = "<episodedetails><displayepisode> 502-1 </displayepisode></episodedetails>";
        assert_eq!(
            xml_tag_value(xml, "displayepisode").as_deref(),
            Some("502-1")
        );
    }
}
