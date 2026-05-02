use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description};
use tokio::{process::Command, sync::RwLock};

use crate::{
    config::RecordingNfoStyle,
    recording_rules,
    twitch_auth::{HelixChannelMetadata, TwitchAuthService},
};

const QUALITY_OPTIONS: [&str; 9] = [
    "best", "source", "1080p60", "1080p", "720p60", "720p", "480p", "360p", "160p",
];
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ChannelMetadataCache {
    #[serde(skip_serializing_if = "Option::is_none")]
    poster_url: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

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
    pub pinned: bool,
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
    last_observed_game: Option<String>,
    pending_game: Option<String>,
    pending_game_confirmations: u64,
    chapter_events: Vec<ChapterEvent>,
    child: tokio::process::Child,
}

#[derive(Debug, Clone)]
struct ChapterEvent {
    offset_secs: u64,
    title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackIndex {
    pub version: u32,
    pub media_file: String,
    pub duration_secs: f64,
    pub target_duration: u64,
    pub init: PlaybackRange,
    pub segments: Vec<PlaybackSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackRange {
    pub start: u64,
    pub length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackSegment {
    pub index: usize,
    pub duration_secs: f64,
    pub start: u64,
    pub length: u64,
}

#[derive(Debug, Clone)]
pub struct RecordingService {
    streamlink_path: String,
    recordings_dir: PathBuf,
    write_nfo: bool,
    nfo_style: RecordingNfoStyle,
    twitch: TwitchAuthService,
    ffmpeg_path: String,
    chapter_min_gap_secs: u64,
    chapter_change_confirmations: u64,
    hls_segment_duration_secs: u64,
    hls_cache_ttl_secs: u64,
    active: Arc<RwLock<HashMap<String, ActiveProcess>>>,
}

#[derive(Debug, Clone)]
pub struct RecordingProcessingConfig {
    pub ffmpeg_path: String,
    pub chapter_min_gap_secs: u64,
    pub chapter_change_confirmations: u64,
    pub hls_segment_duration_secs: u64,
    pub hls_cache_ttl_secs: u64,
}

impl RecordingService {
    pub fn new(
        streamlink_path: String,
        recordings_dir: String,
        write_nfo: bool,
        nfo_style: RecordingNfoStyle,
        twitch: TwitchAuthService,
        processing: RecordingProcessingConfig,
    ) -> Result<Self, String> {
        let service = Self {
            streamlink_path,
            recordings_dir: PathBuf::from(recordings_dir),
            write_nfo,
            nfo_style,
            twitch,
            ffmpeg_path: processing.ffmpeg_path,
            chapter_min_gap_secs: processing.chapter_min_gap_secs,
            chapter_change_confirmations: processing.chapter_change_confirmations,
            hls_segment_duration_secs: processing.hls_segment_duration_secs,
            hls_cache_ttl_secs: processing.hls_cache_ttl_secs,
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
                    last_observed_game: None,
                    pending_game: None,
                    pending_game_confirmations: 0,
                    chapter_events: vec![ChapterEvent {
                        offset_secs: 0,
                        title: "Stream Start".to_string(),
                    }],
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
            let final_path = build_completed_recording_path(
                &self.channel_bucket_dir("completed", &channel_login),
                &channel_login,
                &process.metadata,
                process.stream_title.as_deref(),
            );
            move_file_if_exists(&output_path, &final_path);
            tracing::info!(from = %output_path.display(), to = %final_path.display(), "recording moved to completed");
            self.write_playback_assets(
                &channel_login,
                &final_path,
                &process.metadata,
                &process.chapter_events,
            )
            .await;
            self.write_nfo_if_enabled(
                &channel_login,
                &final_path,
                &process.metadata,
                process.stream_title.as_deref(),
            )
            .await;
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
        let playback_cache_hint = playback_cache_parent_prefix(&target_path);
        let playback_index_path = playback_index_path_for_recording(&target_path);

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

            if let Some((parent, prefix)) = playback_cache_hint.as_ref() {
                remove_playback_cache_dirs(parent, prefix)
                    .map_err(|error| format!("recording delete failed: {error}"))?;
            }

            if playback_index_path.exists() {
                fs::remove_file(&playback_index_path)
                    .map_err(|error| format!("recording delete failed: {error}"))?;
            }

            let pin_path = pin_marker_path_for_recording(&target_path);
            if pin_path.exists() {
                fs::remove_file(&pin_path)
                    .map_err(|error| format!("recording delete failed: {error}"))?;
            }
        }

        Ok(())
    }

    pub fn pin_recording_file(&self, channel_login: &str, filename: &str) -> Result<(), String> {
        let target_path =
            self.resolve_recording_file_path(RecordingBucket::Completed, channel_login, filename)?;

        if !target_path.exists() {
            return Err("recording file not found".to_string());
        }

        let pin_path = pin_marker_path_for_recording(&target_path);
        fs::write(&pin_path, b"pinned\n").map_err(|error| format!("recording pin failed: {error}"))
    }

    pub fn unpin_recording_file(&self, channel_login: &str, filename: &str) -> Result<(), String> {
        let target_path =
            self.resolve_recording_file_path(RecordingBucket::Completed, channel_login, filename)?;

        if !target_path.exists() {
            return Err("recording file not found".to_string());
        }

        let pin_path = pin_marker_path_for_recording(&target_path);
        if !pin_path.exists() {
            return Ok(());
        }

        fs::remove_file(&pin_path).map_err(|error| format!("recording unpin failed: {error}"))
    }

    pub fn resolve_completed_file_path(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<PathBuf, String> {
        self.resolve_recording_file_path(RecordingBucket::Completed, channel_login, filename)
    }

    pub fn resolve_playback_media_path(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<PathBuf, String> {
        self.resolve_completed_file_path(channel_login, filename)
    }

    pub async fn load_playback_index(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<PlaybackIndex, String> {
        let media_path = self.resolve_completed_file_path(channel_login, filename)?;
        let sidecar_path = playback_index_path_for_recording(&media_path);
        let payload = tokio::fs::read_to_string(&sidecar_path)
            .await
            .map_err(|error| format!("failed to read playback index: {error}"))?;
        serde_json::from_str::<PlaybackIndex>(&payload)
            .map_err(|error| format!("failed to decode playback index: {error}"))
    }

    fn prune_playback_cache_for_recording(&self, recording_path: &Path) {
        let Some((parent, prefix)) = playback_cache_parent_prefix(recording_path) else {
            return;
        };

        let Ok(entries) = fs::read_dir(parent) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if !name.starts_with(&prefix) {
                continue;
            }

            let is_expired = fs::metadata(&path)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|elapsed| elapsed.as_secs() >= self.hls_cache_ttl_secs);
            let is_current = playback_dir_for_recording(recording_path)
                .map(|current| current == path)
                .unwrap_or(false);
            if is_current || !is_expired {
                continue;
            }
            let _ = fs::remove_dir_all(path);
        }
    }

    pub async fn note_game_observation(
        &self,
        channel_login: &str,
        game: Option<&str>,
        observed_at_unix: u64,
    ) {
        let normalized = channel_login.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return;
        }

        let mut active = self.active.write().await;
        let Some(process) = active.get_mut(&normalized) else {
            return;
        };

        let candidate = game
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        if process.last_observed_game == candidate {
            process.pending_game = None;
            process.pending_game_confirmations = 0;
            return;
        }

        if process.pending_game != candidate {
            process.pending_game = candidate;
            process.pending_game_confirmations = 1;
            return;
        }

        process.pending_game_confirmations = process.pending_game_confirmations.saturating_add(1);
        if process.pending_game_confirmations < self.chapter_change_confirmations {
            return;
        }

        process.last_observed_game = process.pending_game.clone();
        process.pending_game = None;
        process.pending_game_confirmations = 0;

        let offset_secs = observed_at_unix.saturating_sub(process.metadata.started_at_unix);
        if let Some(last) = process.chapter_events.last()
            && offset_secs.saturating_sub(last.offset_secs) < self.chapter_min_gap_secs
        {
            return;
        }

        let chapter_title = match process.last_observed_game.as_deref() {
            Some(name) => format!("Game: {name}"),
            None => "Game: Unknown".to_string(),
        };
        process.chapter_events.push(ChapterEvent {
            offset_secs,
            title: chapter_title,
        });
    }

    fn resolve_recording_file_path(
        &self,
        bucket: RecordingBucket,
        channel_login: &str,
        filename: &str,
    ) -> Result<PathBuf, String> {
        let channel_login = Self::normalize_channel_login(channel_login)?;
        let filename = validate_recording_filename(filename)?;
        let channel_dir = self.channel_bucket_dir(bucket.as_str(), &channel_login);
        if matches!(bucket, RecordingBucket::Completed)
            && let Some(path) = find_file_by_name_recursive(&channel_dir, &filename)
        {
            return Ok(path);
        }
        Ok(channel_dir.join(filename))
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
            self.finalize_exited_process(&channel_login, &process, exit)
                .await;
        }
    }

    async fn finalize_exited_process(
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

        if exit.success() {
            let final_path = build_completed_recording_path(
                &self.channel_bucket_dir("completed", channel_login),
                channel_login,
                &process.metadata,
                process.stream_title.as_deref(),
            );
            move_file_if_exists(&output_path, &final_path);
            self.write_playback_assets(
                channel_login,
                &final_path,
                &process.metadata,
                &process.chapter_events,
            )
            .await;
            self.write_nfo_if_enabled(
                channel_login,
                &final_path,
                &process.metadata,
                process.stream_title.as_deref(),
            )
            .await;
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

        let filename = output_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("recording.ts");

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

    async fn write_nfo_if_enabled(
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

        if let Err(error) = self
            .write_tv_nfo_files(channel_login, recording_path, metadata, stream_title)
            .await
        {
            tracing::warn!(
                channel = %channel_login,
                path = %recording_path.display(),
                error = %error,
                "failed to write recording nfo"
            );
        }
    }

    async fn write_playback_assets(
        &self,
        channel_login: &str,
        recording_path: &Path,
        metadata: &ActiveRecording,
        chapter_events: &[ChapterEvent],
    ) {
        let mut chapters = chapter_events.to_vec();
        let end_offset = now_unix().saturating_sub(metadata.started_at_unix);
        chapters.push(ChapterEvent {
            offset_secs: end_offset,
            title: "Stream End".to_string(),
        });

        let chapter_file = recording_path.with_extension("ffmetadata");
        if let Err(error) = write_ffmetadata_chapters(&chapter_file, &chapters) {
            tracing::warn!(channel = %channel_login, error = %error, "failed to write ffmetadata chapters");
            return;
        }

        let mp4_path = recording_path.with_extension("mp4");
        let remux_ok = match Command::new(&self.ffmpeg_path)
            .arg("-y")
            .arg("-i")
            .arg(recording_path)
            .arg("-i")
            .arg(&chapter_file)
            .arg("-map_metadata")
            .arg("1")
            .arg("-map_chapters")
            .arg("1")
            .arg("-c")
            .arg("copy")
            .arg("-f")
            .arg("mp4")
            .arg("-movflags")
            .arg("cmaf+default_base_moof+global_sidx")
            .arg("-frag_duration")
            .arg((self.hls_segment_duration_secs.saturating_mul(1_000_000)).to_string())
            .arg(&mp4_path)
            .status()
            .await
        {
            Ok(status) => status.success(),
            Err(_) => false,
        };

        if !remux_ok {
            tracing::warn!(channel = %channel_login, path = %recording_path.display(), "ffmpeg fragmented mp4 remux failed");
            let _ = fs::remove_file(&chapter_file);
            return;
        }

        let index = match self
            .build_playback_index_from_media(&mp4_path, self.hls_segment_duration_secs)
            .await
        {
            Ok(index) => index,
            Err(error) => {
                tracing::warn!(
                    channel = %channel_login,
                    path = %mp4_path.display(),
                    error = %error,
                    "failed to build playback index"
                );
                let _ = fs::remove_file(&chapter_file);
                return;
            }
        };

        let index_path = playback_index_path_for_recording(&mp4_path);
        if let Err(error) = write_playback_index(&index_path, &index) {
            tracing::warn!(
                channel = %channel_login,
                path = %index_path.display(),
                error = %error,
                "failed to persist playback index"
            );
            let _ = fs::remove_file(&chapter_file);
            return;
        }

        let _ = fs::remove_file(recording_path);
        let _ = fs::remove_file(&chapter_file);
        self.prune_playback_cache_for_recording(&mp4_path);
    }
}

impl RecordingService {
    async fn write_tv_nfo_files(
        &self,
        channel_login: &str,
        recording_path: &Path,
        metadata: &ActiveRecording,
        stream_title: Option<&str>,
    ) -> Result<(), String> {
        let channel_dir = recording_path
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| "recording file has no season parent".to_string())?;

        let fetched = self.fetch_twitch_channel_metadata(channel_login).await;
        let mut cache = read_channel_metadata_cache(channel_dir);
        let tags = select_show_tags(fetched.as_ref(), &cache);

        if let Some(meta) = fetched.as_ref() {
            let http = self.twitch.api_client();
            update_channel_poster(channel_dir, &http, meta, &mut cache).await;
            write_tvshow_nfo_file(channel_login, channel_dir, meta, &tags)?;
        }
        cache.tags = tags.clone();
        let _ = write_channel_metadata_cache(channel_dir, &cache);

        write_episode_nfo_file(channel_login, recording_path, metadata, stream_title, &tags)
    }

    async fn fetch_twitch_channel_metadata(
        &self,
        channel_login: &str,
    ) -> Option<HelixChannelMetadata> {
        match self.twitch.fetch_channel_metadata(channel_login).await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(channel = %channel_login, error = %error, "helix metadata lookup failed");
                None
            }
        }
    }

    async fn build_playback_index_from_media(
        &self,
        media_path: &Path,
        segment_time_secs: u64,
    ) -> Result<PlaybackIndex, String> {
        let parent = media_path
            .parent()
            .ok_or_else(|| "recording file has no parent directory".to_string())?;
        let stem = media_path
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("recording");
        let tmp_dir = parent.join(format!(".hls_tmp_{stem}"));
        fs::create_dir_all(&tmp_dir)
            .map_err(|error| format!("failed to create hls temp directory: {error}"))?;

        let playlist_path = tmp_dir.join("master.m3u8");
        let single_file_path = tmp_dir.join("media.mp4");
        let status = Command::new(&self.ffmpeg_path)
            .arg("-y")
            .arg("-i")
            .arg(media_path)
            .arg("-c")
            .arg("copy")
            .arg("-hls_time")
            .arg(segment_time_secs.to_string())
            .arg("-hls_playlist_type")
            .arg("vod")
            .arg("-hls_segment_type")
            .arg("fmp4")
            .arg("-hls_flags")
            .arg("single_file")
            .arg("-hls_fmp4_init_filename")
            .arg("init.mp4")
            .arg("-hls_segment_filename")
            .arg(&single_file_path)
            .arg(&playlist_path)
            .status()
            .await
            .map_err(|error| format!("ffmpeg sidecar generation failed: {error}"))?;

        if !status.success() {
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err("ffmpeg sidecar generation failed".to_string());
        }

        let playlist = fs::read_to_string(&playlist_path)
            .map_err(|error| format!("failed to read temporary playlist: {error}"))?;
        let init_len = fs::metadata(tmp_dir.join("init.mp4"))
            .map_err(|error| format!("failed to read temporary init segment metadata: {error}"))?
            .len();
        let duration_secs = probe_media_duration_secs(media_path).await.unwrap_or(0.0);

        let parsed = parse_single_file_hls_playlist(&playlist, init_len)?;
        let index = PlaybackIndex {
            version: 1,
            media_file: media_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("recording.mp4")
                .to_string(),
            duration_secs,
            target_duration: parsed.0,
            init: PlaybackRange {
                start: 0,
                length: init_len,
            },
            segments: parsed.1,
        };

        let _ = fs::remove_dir_all(&tmp_dir);
        Ok(index)
    }
}

fn write_playback_index(path: &Path, index: &PlaybackIndex) -> Result<(), String> {
    let payload = serde_json::to_vec_pretty(index)
        .map_err(|error| format!("failed to encode playback index: {error}"))?;
    fs::write(path, payload).map_err(|error| format!("failed to write playback index: {error}"))
}

fn playback_index_path_for_recording(recording_path: &Path) -> PathBuf {
    let file_name = recording_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("recording.mp4");
    recording_path.with_file_name(format!("{file_name}.hls.json"))
}

fn parse_single_file_hls_playlist(
    playlist: &str,
    init_len: u64,
) -> Result<(u64, Vec<PlaybackSegment>), String> {
    let mut target_duration = 0_u64;
    let mut segments: Vec<PlaybackSegment> = Vec::new();
    let mut pending_duration: Option<f64> = None;
    for line in playlist.lines() {
        if let Some(value) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            target_duration = value.trim().parse::<u64>().unwrap_or(0);
            continue;
        }
        if let Some(value) = line.strip_prefix("#EXTINF:") {
            let raw = value.split(',').next().unwrap_or("0").trim();
            pending_duration = raw.parse::<f64>().ok();
            continue;
        }
        if let Some(value) = line.strip_prefix("#EXT-X-BYTERANGE:") {
            let Some((len_raw, start_raw)) = value.trim().split_once('@') else {
                continue;
            };
            let length = len_raw.parse::<u64>().unwrap_or(0);
            let mut start = start_raw.parse::<u64>().unwrap_or(0);
            if start >= init_len {
                start -= init_len;
            }
            segments.push(PlaybackSegment {
                index: segments.len(),
                duration_secs: pending_duration.unwrap_or(0.0),
                start,
                length,
            });
            pending_duration = None;
        }
    }
    if segments.is_empty() {
        return Err("temporary playlist had no byterange segments".to_string());
    }
    Ok((target_duration.max(1), segments))
}

fn write_episode_nfo_file(
    channel_login: &str,
    recording_path: &Path,
    metadata: &ActiveRecording,
    stream_title: Option<&str>,
    genres: &[String],
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
    let episode_number = base_episode.saturating_add(suffix_index);
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
    let genre_xml = genres
        .iter()
        .map(|genre| format!("  <genre>{}</genre>\n", xml_escape(genre)))
        .collect::<String>();

    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<episodedetails>\n  <title>{}</title>\n  <showtitle>{}</showtitle>\n  <season>{season}</season>\n  <episode>{episode_number}</episode>\n  <displayepisode>{}</displayepisode>\n  <aired>{aired}</aired>\n{}  <plot>{}</plot>\n  <uniqueid type=\"twitch\" default=\"true\">{}</uniqueid>\n</episodedetails>\n",
        xml_escape(&title),
        xml_escape(channel_login),
        xml_escape(&display_episode),
        genre_xml,
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

fn build_completed_recording_path(
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
        "{} - S{season:04}E{episode_number:04} - {title_slug}.ts",
        sanitize_filename(channel_login)
    ))
}

fn write_tvshow_nfo_file(
    channel_login: &str,
    channel_dir: &Path,
    metadata: &HelixChannelMetadata,
    genres: &[String],
) -> Result<(), String> {
    let title = metadata.display_name.trim();
    let plot = metadata
        .description
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("Twitch channel recordings.");
    let genre_xml = genres
        .iter()
        .map(|genre| format!("  <genre>{}</genre>\n", xml_escape(genre)))
        .collect::<String>();
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<tvshow>\n  <title>{}</title>\n  <plot>{}</plot>\n  <status>Continuing</status>\n  <studio>{}</studio>\n{}  <thumb>poster.jpg</thumb>\n  <uniqueid type=\"twitch\" default=\"true\">twitch_{}</uniqueid>\n</tvshow>\n",
        xml_escape(title),
        xml_escape(plot),
        xml_escape(title),
        genre_xml,
        xml_escape(channel_login)
    );
    let path = channel_dir.join("tvshow.nfo");
    fs::write(&path, xml)
        .map_err(|error| format!("failed to write tvshow.nfo {}: {error}", path.display()))
}

fn select_show_tags(
    metadata: Option<&HelixChannelMetadata>,
    cache: &ChannelMetadataCache,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(meta) = metadata {
        for tag in &meta.tags {
            append_unique_tag(&mut out, tag);
        }
        if out.is_empty() {
            for tag in &cache.tags {
                append_unique_tag(&mut out, tag);
            }
        }
        if out.is_empty()
            && let Some(game) = meta.game.as_deref()
        {
            append_unique_tag(&mut out, game);
        }
    } else {
        for tag in &cache.tags {
            append_unique_tag(&mut out, tag);
        }
    }
    out.truncate(10);
    out
}

fn append_unique_tag(tags: &mut Vec<String>, raw: &str) {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return;
    }
    if tags.iter().any(|tag| tag.eq_ignore_ascii_case(normalized)) {
        return;
    }
    tags.push(normalized.to_string());
}

fn read_channel_metadata_cache(channel_dir: &Path) -> ChannelMetadataCache {
    let path = channel_dir.join(".metadata-cache.json");
    let Ok(text) = fs::read_to_string(path) else {
        return ChannelMetadataCache::default();
    };
    serde_json::from_str::<ChannelMetadataCache>(&text).unwrap_or_default()
}

fn write_channel_metadata_cache(
    channel_dir: &Path,
    cache: &ChannelMetadataCache,
) -> Result<(), String> {
    let path = channel_dir.join(".metadata-cache.json");
    let payload = serde_json::to_string(cache)
        .map_err(|error| format!("failed to encode channel metadata cache: {error}"))?;
    fs::write(&path, payload).map_err(|error| {
        format!(
            "failed to write channel metadata cache {}: {error}",
            path.display()
        )
    })
}

async fn update_channel_poster(
    channel_dir: &Path,
    http: &reqwest::Client,
    metadata: &HelixChannelMetadata,
    cache: &mut ChannelMetadataCache,
) {
    let Some(url) = metadata.profile_image_url.as_deref() else {
        return;
    };
    if cache.poster_url.as_deref() == Some(url) {
        return;
    }
    let Ok(response) = http.get(url).send().await else {
        return;
    };
    if !response.status().is_success() {
        return;
    }
    let Ok(bytes) = response.bytes().await else {
        return;
    };
    let _ = fs::create_dir_all(channel_dir);
    let poster_path = channel_dir.join("poster.jpg");
    if fs::write(&poster_path, &bytes).is_ok() {
        cache.poster_url = Some(url.to_string());
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

fn is_visible_recording_file(path: &Path) -> bool {
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

fn playback_dir_for_recording(recording_path: &Path) -> Result<PathBuf, String> {
    let parent = recording_path
        .parent()
        .ok_or_else(|| "recording file has no parent directory".to_string())?;
    let stem = recording_path
        .file_stem()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty())
        .unwrap_or("recording");
    let modified = fs::metadata(recording_path)
        .and_then(|meta| meta.modified())
        .map_err(|error| format!("failed to read recording metadata: {error}"))?
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| format!("failed to derive recording timestamp: {error}"))?
        .as_secs();
    Ok(parent.join(format!(".playback_{stem}_{modified}")))
}

fn playback_cache_parent_prefix(recording_path: &Path) -> Option<(PathBuf, String)> {
    let parent = recording_path.parent()?.to_path_buf();
    let stem = recording_path
        .file_stem()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty())
        .unwrap_or("recording");
    Some((parent, format!(".playback_{stem}_")))
}

async fn probe_media_duration_secs(recording_path: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(recording_path)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    let duration = raw.trim().parse::<f64>().ok()?;
    if duration.is_finite() && duration > 0.0 {
        Some(duration)
    } else {
        None
    }
}

fn remove_playback_cache_dirs(parent: &Path, prefix: &str) -> Result<(), String> {
    let entries = fs::read_dir(parent)
        .map_err(|error| format!("failed to list playback cache directories: {error}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.starts_with(prefix) {
            continue;
        }
        fs::remove_dir_all(&path)
            .map_err(|error| format!("failed to remove playback cache directory: {error}"))?;
    }
    Ok(())
}

fn write_ffmetadata_chapters(path: &Path, events: &[ChapterEvent]) -> Result<(), String> {
    let mut content = String::from(";FFMETADATA1\n");
    for (index, event) in events.iter().enumerate() {
        let start_ms = event.offset_secs.saturating_mul(1000);
        let end_ms = events
            .get(index + 1)
            .map(|next| next.offset_secs.saturating_mul(1000))
            .unwrap_or(start_ms.saturating_add(1000));
        if end_ms <= start_ms {
            continue;
        }
        content.push_str("[CHAPTER]\nTIMEBASE=1/1000\n");
        content.push_str(&format!("START={start_ms}\nEND={end_ms}\n"));
        content.push_str(&format!("title={}\n", event.title.replace('\n', " ")));
    }
    fs::write(path, content).map_err(|error| {
        format!(
            "failed to write chapter metadata {}: {error}",
            path.display()
        )
    })
}

fn find_file_by_name_recursive(dir: &Path, filename: &str) -> Option<PathBuf> {
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

fn prune_completed_channel_dir(dir: &Path, keep_last: usize) {
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
        let playback_cache_hint = playback_cache_parent_prefix(&old_path);
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
        let playback_index = playback_index_path_for_recording(&old_path);
        if playback_index.exists() {
            let _ = fs::remove_file(playback_index);
        }
        if let Some((parent, prefix)) = playback_cache_hint.as_ref() {
            let _ = remove_playback_cache_dirs(parent, prefix);
        }
    }
}

fn pin_marker_path_for_recording(recording_path: &Path) -> PathBuf {
    let file_name = recording_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("recording");
    recording_path.with_file_name(format!("{file_name}.pin"))
}

fn is_recording_pinned(recording_path: &Path) -> bool {
    pin_marker_path_for_recording(recording_path).exists()
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

    #[test]
    fn visible_recording_file_excludes_nfo() {
        assert!(is_visible_recording_file(Path::new("video.ts")));
        assert!(is_visible_recording_file(Path::new("video.mp4")));
        assert!(!is_visible_recording_file(Path::new("video.nfo")));
        assert!(!is_visible_recording_file(Path::new("video.NFO")));
    }
}
