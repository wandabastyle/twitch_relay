use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};

use tokio::{process::Command, sync::RwLock};

use crate::{
    config::RecordingNfoStyle,
    recording_rules,
    twitch_auth::{HelixChannelMetadata, TwitchAuthService},
    util::channel::normalize_channel_login,
    util::time::now_unix_secs,
};
use std::process::Command as StdCommand;

mod files;
mod nfo;
mod types;

pub use types::{
    ActiveRecording, RecordingBucket, RecordingError, RecordingFileEntry, RecordingJob,
    RecordingJobKind, RecordingJobStatus, RecordingMode, RecordingProcessingConfig,
    RecordingProcessingState, RecordingsOverview,
};

use files::*;
use nfo::*;
use types::*;

type MergeSourceItem = (ParsedRecordingFilename, PathBuf);

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
    active: Arc<RwLock<HashMap<String, ActiveProcess>>>,
}

impl RecordingService {
    pub fn new(
        streamlink_path: String,
        recordings_dir: String,
        write_nfo: bool,
        nfo_style: RecordingNfoStyle,
        twitch: TwitchAuthService,
        processing: RecordingProcessingConfig,
    ) -> Result<Self, RecordingError> {
        let service = Self {
            streamlink_path,
            recordings_dir: PathBuf::from(recordings_dir),
            write_nfo,
            nfo_style,
            twitch,
            ffmpeg_path: processing.ffmpeg_path,
            chapter_min_gap_secs: processing.chapter_min_gap_secs,
            chapter_change_confirmations: processing.chapter_change_confirmations,
            active: Arc::new(RwLock::new(HashMap::new())),
        };
        service.ensure_directories()?;
        service.cleanup_startup_tmp()?;
        Ok(service)
    }

    pub fn validate_quality(quality: &str) -> Result<String, RecordingError> {
        let normalized = quality.trim().to_ascii_lowercase();
        if QUALITY_OPTIONS.contains(&normalized.as_str()) {
            Ok(normalized)
        } else {
            Err(RecordingError::InvalidQuality)
        }
    }

    pub async fn start_recording(
        &self,
        channel_login: &str,
        quality: &str,
        mode: RecordingMode,
        stream_title: Option<&str>,
    ) -> Result<ActiveRecording, RecordingError> {
        let channel_login =
            normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
        let quality = Self::validate_quality(quality)?;

        self.reconcile_exited_recordings().await;

        {
            let active = self.active.read().await;
            if active.contains_key(&channel_login) {
                return Err(RecordingError::AlreadyActive);
            }
        }

        let started_at_unix = now_unix_secs();
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
            fs::create_dir_all(parent).map_err(|e| {
                RecordingError::DirectoryNotWritable(format!(
                    "recordings directory not writable: {e}"
                ))
            })?;
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
            .map_err(|e| RecordingError::SpawnFailed(format!("streamlink spawn failed: {e}")))?;

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

    pub async fn stop_recording(
        &self,
        channel_login: &str,
    ) -> Result<ActiveRecording, RecordingError> {
        self.reconcile_exited_recordings().await;

        let channel_login =
            normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
        let mut process = {
            let mut active = self.active.write().await;
            active.remove(&channel_login)
        }
        .ok_or(RecordingError::NotActive)?;

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
    ) -> Result<(), RecordingError> {
        let target_path = self.resolve_recording_file_path(bucket, channel_login, filename)?;

        if !target_path.exists() {
            return Err(RecordingError::FileNotFound);
        }

        fs::remove_file(&target_path).map_err(|error| {
            RecordingError::DeleteFailed(format!("recording delete failed: {error}"))
        })?;

        if matches!(bucket, RecordingBucket::Completed) {
            let nfo_path = target_path.with_extension("nfo");
            if nfo_path.exists() {
                fs::remove_file(&nfo_path).map_err(|error| {
                    RecordingError::DeleteFailed(format!("recording delete failed: {error}"))
                })?;
            }

            let pin_path = pin_marker_path_for_recording(&target_path);
            if pin_path.exists() {
                fs::remove_file(&pin_path).map_err(|error| {
                    RecordingError::DeleteFailed(format!("recording delete failed: {error}"))
                })?;
            }

            let m3u8_path = target_path.with_extension("m3u8");
            if m3u8_path.exists() {
                fs::remove_file(&m3u8_path).map_err(|error| {
                    RecordingError::DeleteFailed(format!("recording delete failed: {error}"))
                })?;
            }
        }

        Ok(())
    }

    pub fn pin_recording_file(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<(), RecordingError> {
        let target_path =
            self.resolve_recording_file_path(RecordingBucket::Completed, channel_login, filename)?;

        if !target_path.exists() {
            return Err(RecordingError::FileNotFound);
        }

        let pin_path = pin_marker_path_for_recording(&target_path);
        fs::write(&pin_path, b"pinned\n")
            .map_err(|error| RecordingError::PinFailed(format!("recording pin failed: {error}")))
    }

    pub fn unpin_recording_file(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<(), RecordingError> {
        let target_path =
            self.resolve_recording_file_path(RecordingBucket::Completed, channel_login, filename)?;

        if !target_path.exists() {
            return Err(RecordingError::FileNotFound);
        }

        let pin_path = pin_marker_path_for_recording(&target_path);
        if !pin_path.exists() {
            return Ok(());
        }

        fs::remove_file(&pin_path).map_err(|error| {
            RecordingError::UnpinFailed(format!("recording unpin failed: {error}"))
        })
    }

    pub async fn merge_incomplete_recordings(
        &self,
        channel_login: &str,
        filenames: Vec<String>,
        expected_filename: &str,
    ) -> Result<RecordingFileEntry, RecordingError> {
        let channel_login =
            normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
        let filename = validate_recording_filename(expected_filename)?;
        let (mut combined, stream_title) =
            self.validate_merge_sources(&channel_login, &filenames)?;
        combined.sort_by(|a, b| a.0.timestamp.cmp(&b.0.timestamp));

        // Create temporary directory
        let tmp_dir = self
            .recordings_dir
            .join(format!("tmp/merge-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp_dir).map_err(|e| {
            RecordingError::MergeFailed(format!("failed to create temp directory: {e}"))
        })?;

        // Create concat file
        let concat_path = tmp_dir.join("concat.txt");
        let mut concat_content = String::new();

        for (_, file_path) in &combined {
            // ffmpeg concat demuxer requires absolute paths
            let abs_path = fs::canonicalize(file_path).map_err(|e| {
                RecordingError::MergeFailed(format!("failed to resolve file path: {e}"))
            })?;
            concat_content.push_str(&format!("file '{}'\n", abs_path.display()));
        }

        fs::write(&concat_path, concat_content).map_err(|e| {
            RecordingError::MergeFailed(format!("failed to write concat file: {e}"))
        })?;

        // Merge directly to fragmented MP4 in temporary storage
        let merged_path = tmp_dir.join("merged.mp4");
        let output = StdCommand::new(&self.ffmpeg_path)
            .arg("-y")
            .arg("-f")
            .arg("concat")
            .arg("-safe")
            .arg("0")
            .arg("-i")
            .arg(&concat_path)
            .arg("-c")
            .arg("copy")
            .arg("-bsf:a")
            .arg("aac_adtstoasc")
            .arg("-movflags")
            .arg("frag_keyframe+empty_moov+delay_moov+default_base_moof")
            .arg("-frag_duration")
            .arg("10000000")
            .arg(&merged_path)
            .output()
            .map_err(|e| RecordingError::MergeFailed(format!("ffmpeg failed to start: {e}")))?;

        if !output.status.success() {
            let stderr =
                String::from_utf8(output.stderr).unwrap_or_else(|_| "unknown error".to_string());
            return Err(RecordingError::MergeFailed(format!(
                "ffmpeg merge failed: {stderr}"
            )));
        }

        // Determine metadata from first file
        let first_file = &combined[0];
        let started_at_unix =
            parse_filename_timestamp_to_unix(&first_file.0.timestamp).unwrap_or_else(now_unix_secs);
        let quality = first_file.0.quality.clone();
        let mode = match first_file.0.mode.as_str() {
            "manual" => RecordingMode::Manual,
            "auto" => RecordingMode::Auto,
            _ => RecordingMode::Manual,
        };
        let stream_title = stream_title.as_deref();

        let final_name = Path::new(&filename)
            .with_extension("mp4")
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("merged.mp4")
            .to_string();
        let final_path = self
            .channel_bucket_dir("completed", &channel_login)
            .join(final_name);
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RecordingError::MergeFailed(format!("failed to create destination directory: {e}"))
            })?;
        }
        fs::rename(&merged_path, &final_path).map_err(|e| {
            RecordingError::MergeFailed(format!("failed to finalize merged output: {e}"))
        })?;
        let processing_marker = processing_marker_path_for_recording(&final_path);
        let _ = fs::write(&processing_marker, b"processing\n");

        if let Err(error) = self.rebuild_hls_playlist(&channel_login, &final_path) {
            let _ = fs::remove_file(&processing_marker);
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err(RecordingError::MergeFailed(error));
        }
        let _ = fs::remove_file(&processing_marker);

        // Write NFO if enabled
        self.write_nfo_if_enabled(
            &channel_login,
            &final_path,
            &ActiveRecording {
                channel_login: channel_login.clone(),
                quality: quality.clone(),
                started_at_unix,
                output_path: final_path.display().to_string(),
                pid: None,
                mode,
                error: None,
            },
            stream_title,
        )
        .await;

        // Source files are kept intentionally (user will delete manually)

        // Cleanup temp directory
        if let Err(e) = fs::remove_dir_all(&tmp_dir) {
            tracing::warn!(path = %tmp_dir.display(), error = %e, "failed to cleanup temp directory");
        }

        Ok(RecordingFileEntry {
            channel_login,
            filename: final_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("merged.mp4")
                .to_string(),
            path_display: final_path.display().to_string(),
            status: "completed".to_string(),
            pinned: false,
            has_hls: final_path.with_extension("m3u8").exists(),
            processing_state: RecordingProcessingState::Ready,
        })
    }

    pub fn validate_merge_request(
        &self,
        channel_login: &str,
        filenames: &[String],
    ) -> Result<(String, String), RecordingError> {
        let normalized =
            normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
        let (combined, stream_title) = self.validate_merge_sources(&normalized, filenames)?;
        let expected_name = self.expected_completed_mp4_filename(
            &normalized,
            &combined[0].0,
            stream_title.as_deref(),
        );
        Ok((normalized, expected_name))
    }

    pub fn validate_finalize_request(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<(String, String), RecordingError> {
        let normalized =
            normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
        let validated = validate_recording_filename(filename)?;
        let parsed = parse_recording_filename(&validated)
            .map_err(|e| RecordingError::MergeFailed(format!("invalid filename format: {e}")))?;
        if parsed.channel != normalized {
            return Err(RecordingError::MergeFailed(format!(
                "filename channel '{}' does not match requested channel '{}'",
                parsed.channel, normalized
            )));
        }

        let source_path = self
            .channel_bucket_dir("incomplete", &normalized)
            .join(&validated);
        if !source_path.exists() {
            return Err(RecordingError::MergeFailed(format!(
                "file not found: {}",
                source_path.display()
            )));
        }

        let expected_name =
            self.expected_completed_mp4_filename(&normalized, &parsed, parsed.title.as_deref());
        Ok((normalized, expected_name))
    }

    pub async fn finalize_incomplete_recording(
        &self,
        channel_login: &str,
        filename: &str,
        expected_filename: &str,
    ) -> Result<RecordingFileEntry, RecordingError> {
        let normalized =
            normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
        let validated = validate_recording_filename(filename)?;
        let parsed = parse_recording_filename(&validated)
            .map_err(|e| RecordingError::MergeFailed(format!("invalid filename format: {e}")))?;
        if parsed.channel != normalized {
            return Err(RecordingError::MergeFailed(format!(
                "filename channel '{}' does not match requested channel '{}'",
                parsed.channel, normalized
            )));
        }

        let source_path = self
            .channel_bucket_dir("incomplete", &normalized)
            .join(&validated);
        if !source_path.exists() {
            return Err(RecordingError::MergeFailed(format!(
                "file not found: {}",
                source_path.display()
            )));
        }

        let final_name = Path::new(&validate_recording_filename(expected_filename)?)
            .with_extension("mp4")
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("recording.mp4")
            .to_string();

        let tmp_dir = self
            .recordings_dir
            .join(format!("tmp/finalize-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp_dir).map_err(|e| {
            RecordingError::MergeFailed(format!("failed to create temp directory: {e}"))
        })?;
        let tmp_output = tmp_dir.join("finalized.mp4");

        let output = StdCommand::new(&self.ffmpeg_path)
            .arg("-y")
            .arg("-i")
            .arg(&source_path)
            .arg("-c")
            .arg("copy")
            .arg("-bsf:a")
            .arg("aac_adtstoasc")
            .arg("-movflags")
            .arg("frag_keyframe+empty_moov+delay_moov+default_base_moof")
            .arg("-frag_duration")
            .arg("10000000")
            .arg(&tmp_output)
            .output()
            .map_err(|e| RecordingError::MergeFailed(format!("ffmpeg failed to start: {e}")))?;
        if !output.status.success() {
            let stderr =
                String::from_utf8(output.stderr).unwrap_or_else(|_| "unknown error".to_string());
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err(RecordingError::MergeFailed(format!(
                "ffmpeg finalize failed: {stderr}"
            )));
        }

        let final_path = self
            .channel_bucket_dir("completed", &normalized)
            .join(final_name);
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RecordingError::MergeFailed(format!("failed to create destination directory: {e}"))
            })?;
        }

        let processing_marker = processing_marker_path_for_recording(&final_path);
        let _ = fs::write(&processing_marker, b"processing\n");

        if let Err(error) = fs::rename(&tmp_output, &final_path) {
            let _ = fs::remove_file(&processing_marker);
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err(RecordingError::MergeFailed(format!(
                "failed to finalize output: {error}"
            )));
        }

        if let Err(error) = self.rebuild_hls_playlist(&normalized, &final_path) {
            let _ = fs::remove_file(&processing_marker);
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err(RecordingError::MergeFailed(error));
        }

        let started_at_unix =
            parse_filename_timestamp_to_unix(&parsed.timestamp).unwrap_or_else(now_unix_secs);
        let quality = parsed.quality.clone();
        let mode = match parsed.mode.as_str() {
            "manual" => RecordingMode::Manual,
            "auto" => RecordingMode::Auto,
            _ => RecordingMode::Manual,
        };
        self.write_nfo_if_enabled(
            &normalized,
            &final_path,
            &ActiveRecording {
                channel_login: normalized.clone(),
                quality,
                started_at_unix,
                output_path: final_path.display().to_string(),
                pid: None,
                mode,
                error: None,
            },
            parsed.title.as_deref(),
        )
        .await;

        let _ = fs::remove_file(&source_path);
        let _ = fs::remove_file(&processing_marker);
        let _ = fs::remove_dir_all(&tmp_dir);

        Ok(self.completed_entry_from_path(&normalized, &final_path))
    }

    pub async fn repair_completed_recording(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<RecordingFileEntry, RecordingError> {
        let channel_login =
            normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
        let target_path = self.resolve_completed_file_path(&channel_login, filename)?;
        if !target_path.exists() {
            return Err(RecordingError::FileNotFound);
        }

        let extension = target_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();

        match extension.as_str() {
            "mp4" => {
                self.rebuild_hls_playlist(&channel_login, &target_path)
                    .map_err(RecordingError::RepairFailed)?;
                let marker = processing_marker_path_for_recording(&target_path);
                let _ = fs::remove_file(marker);
                Ok(self.completed_entry_from_path(&channel_login, &target_path))
            }
            "ts" => {
                let tmp_output = self
                    .tmp_dir()
                    .join(format!("repair-{}.mp4", uuid::Uuid::new_v4()));
                if let Some(parent) = tmp_output.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        RecordingError::RepairFailed(format!(
                            "failed to create temp directory: {e}"
                        ))
                    })?;
                }

                let status = StdCommand::new(&self.ffmpeg_path)
                    .arg("-y")
                    .arg("-i")
                    .arg(&target_path)
                    .arg("-c")
                    .arg("copy")
                    .arg("-bsf:a")
                    .arg("aac_adtstoasc")
                    .arg("-movflags")
                    .arg("frag_keyframe+empty_moov+delay_moov+default_base_moof")
                    .arg("-frag_duration")
                    .arg("10000000")
                    .arg(&tmp_output)
                    .status()
                    .map_err(|e| {
                        RecordingError::RepairFailed(format!("ffmpeg failed to start: {e}"))
                    })?;

                if !status.success() {
                    let _ = fs::remove_file(&tmp_output);
                    return Err(RecordingError::RepairFailed(
                        "ffmpeg remux failed".to_string(),
                    ));
                }

                let final_mp4 = target_path.with_extension("mp4");
                if let Some(parent) = final_mp4.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        RecordingError::RepairFailed(format!(
                            "failed to create destination directory: {e}"
                        ))
                    })?;
                }
                fs::rename(&tmp_output, &final_mp4).map_err(|e| {
                    RecordingError::RepairFailed(format!("failed to move repaired file: {e}"))
                })?;
                let _ = fs::remove_file(&target_path);

                self.rebuild_hls_playlist(&channel_login, &final_mp4)
                    .map_err(RecordingError::RepairFailed)?;
                let marker = processing_marker_path_for_recording(&final_mp4);
                let _ = fs::remove_file(marker);

                Ok(self.completed_entry_from_path(&channel_login, &final_mp4))
            }
            _ => Err(RecordingError::RepairFailed(
                "unsupported recording extension".to_string(),
            )),
        }
    }

    pub fn resolve_completed_file_path(
        &self,
        channel_login: &str,
        filename: &str,
    ) -> Result<PathBuf, RecordingError> {
        self.resolve_recording_file_path(RecordingBucket::Completed, channel_login, filename)
    }

    fn validate_merge_sources(
        &self,
        channel_login: &str,
        filenames: &[String],
    ) -> Result<(Vec<MergeSourceItem>, Option<String>), RecordingError> {
        if filenames.len() < 2 {
            return Err(RecordingError::MergeFailed(
                "at least 2 files are required for merging".to_string(),
            ));
        }

        let channel_dir = self.channel_bucket_dir("incomplete", channel_login);
        let mut combined: Vec<MergeSourceItem> = Vec::new();

        for filename in filenames {
            let validated = validate_recording_filename(filename)?;
            let parsed = parse_recording_filename(&validated).map_err(|e| {
                RecordingError::MergeFailed(format!("invalid filename format: {e}"))
            })?;

            if parsed.channel != channel_login {
                return Err(RecordingError::MergeFailed(format!(
                    "filename channel '{}' does not match requested channel '{}'",
                    parsed.channel, channel_login
                )));
            }

            let file_path = channel_dir.join(&validated);
            if !file_path.exists() {
                return Err(RecordingError::MergeFailed(format!(
                    "file not found: {}",
                    file_path.display()
                )));
            }
            combined.push((parsed, file_path));
        }

        let first_date = combined[0].0.date.clone();
        let first_title = combined[0].0.title.as_deref().unwrap_or("").to_string();
        for (parsed, _) in &combined {
            if parsed.date != first_date {
                return Err(RecordingError::MergeFailed(
                    "all files must have the same date (YYYY-MM-DD)".to_string(),
                ));
            }
            if parsed.title.as_deref().unwrap_or("") != first_title {
                return Err(RecordingError::MergeFailed(
                    "all files must have the same title".to_string(),
                ));
            }
        }

        let stream_title = if first_title.is_empty() {
            None
        } else {
            Some(first_title)
        };
        Ok((combined, stream_title))
    }

    fn expected_completed_mp4_filename(
        &self,
        channel_login: &str,
        parsed: &ParsedRecordingFilename,
        stream_title: Option<&str>,
    ) -> String {
        let started_at_unix =
            parse_filename_timestamp_to_unix(&parsed.timestamp).unwrap_or_else(now_unix_secs);
        let quality = parsed.quality.clone();
        let mode = match parsed.mode.as_str() {
            "manual" => RecordingMode::Manual,
            "auto" => RecordingMode::Auto,
            _ => RecordingMode::Manual,
        };
        let expected = build_completed_recording_path(
            &self.channel_bucket_dir("completed", channel_login),
            channel_login,
            &ActiveRecording {
                channel_login: channel_login.to_string(),
                quality,
                started_at_unix,
                output_path: String::new(),
                pid: None,
                mode,
                error: None,
            },
            stream_title,
        );
        expected
            .with_extension("mp4")
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("merged.mp4")
            .to_string()
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
    ) -> Result<PathBuf, RecordingError> {
        let channel_login =
            normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
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

    fn ensure_directories(&self) -> Result<(), RecordingError> {
        fs::create_dir_all(self.tmp_dir()).map_err(|e| {
            RecordingError::DirectoryNotWritable(format!("recordings directory not writable: {e}"))
        })?;
        fs::create_dir_all(self.completed_dir()).map_err(|e| {
            RecordingError::DirectoryNotWritable(format!("recordings directory not writable: {e}"))
        })?;
        fs::create_dir_all(self.incomplete_dir()).map_err(|e| {
            RecordingError::DirectoryNotWritable(format!("recordings directory not writable: {e}"))
        })?;
        Ok(())
    }

    fn cleanup_startup_tmp(&self) -> Result<(), RecordingError> {
        let tmp = self.tmp_dir();
        let incomplete = self.incomplete_dir();
        let entries = fs::read_dir(&tmp).map_err(|e| {
            RecordingError::Io(format!("read recordings tmp directory failed: {e}"))
        })?;
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
        let end_offset = now_unix_secs().saturating_sub(metadata.started_at_unix);
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
        let processing_marker = processing_marker_path_for_recording(&mp4_path);
        let _ = fs::write(&processing_marker, b"processing\n");
        // Generate fragmented MP4 (fMP4) for proper HLS byte-range playback
        // Creates moof+mdat fragments aligned with keyframes, ~10 seconds each
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
            .arg("-bsf:a")
            .arg("aac_adtstoasc")
            .arg("-movflags")
            .arg("frag_keyframe+empty_moov+delay_moov+default_base_moof")
            .arg("-frag_duration")
            .arg("10000000") // 10 seconds in microseconds
            .arg(&mp4_path)
            .status()
            .await
        {
            Ok(status) => status.success(),
            Err(_) => false,
        };

        if !remux_ok {
            tracing::warn!(channel = %channel_login, path = %recording_path.display(), "ffmpeg mp4 remux failed");
            let _ = fs::remove_file(recording_path);
            let _ = fs::remove_file(&chapter_file);
            let _ = fs::remove_file(&processing_marker);
            return;
        }

        let _ = fs::remove_file(recording_path);
        let _ = fs::remove_file(&chapter_file);

        // Generate HLS playlist for byte-range playback using pure Rust (fast!)
        let mp4_filename = mp4_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("recording.mp4");
        match crate::hls_generator::generate_hls_playlist(&mp4_path, channel_login, mp4_filename) {
            Ok(playlist_content) => {
                let playlist_path = mp4_path.with_extension("m3u8");
                if let Err(e) = fs::write(&playlist_path, playlist_content) {
                    tracing::warn!(channel = %channel_login, error = %e, "failed to write hls playlist file");
                } else {
                    tracing::info!(channel = %channel_login, path = %playlist_path.display(), "hls playlist generated");
                    let _ = fs::remove_file(&processing_marker);
                }
            }
            Err(error) => {
                tracing::warn!(channel = %channel_login, path = %mp4_path.display(), error = %error, "failed to generate hls playlist");
                // Non-fatal: MP4 still works for direct playback
                let _ = fs::remove_file(&processing_marker);
            }
        }
    }

    fn rebuild_hls_playlist(&self, channel_login: &str, mp4_path: &Path) -> Result<(), String> {
        let mp4_filename = mp4_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("recording.mp4");
        let playlist_content =
            crate::hls_generator::generate_hls_playlist(mp4_path, channel_login, mp4_filename)
                .map_err(|e| format!("failed to generate hls playlist: {e}"))?;
        let playlist_path = mp4_path.with_extension("m3u8");
        fs::write(&playlist_path, playlist_content)
            .map_err(|e| format!("failed to write hls playlist: {e}"))
    }

    fn completed_entry_from_path(&self, channel_login: &str, path: &Path) -> RecordingFileEntry {
        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown.mp4")
            .to_string();
        RecordingFileEntry {
            channel_login: channel_login.to_string(),
            filename,
            path_display: path.display().to_string(),
            status: "completed".to_string(),
            pinned: is_recording_pinned(path),
            has_hls: path.with_extension("m3u8").exists(),
            processing_state: if processing_marker_path_for_recording(path).exists() {
                RecordingProcessingState::Processing
            } else {
                RecordingProcessingState::Ready
            },
        }
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

#[cfg(test)]
mod tests {
    use super::RecordingService;
    use super::files::*;
    use super::nfo::*;
    use super::types::*;
    use crate::util::channel::normalize_channel_login;
    use std::path::Path;

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

    #[test]
    fn validate_quality_accepts_valid_options() {
        assert_eq!(RecordingService::validate_quality("best").unwrap(), "best");
        assert_eq!(
            RecordingService::validate_quality("1080p60").unwrap(),
            "1080p60"
        );
        assert_eq!(
            RecordingService::validate_quality("  BEST  ").unwrap(),
            "best"
        );
    }

    #[test]
    fn validate_quality_rejects_invalid_options() {
        assert_eq!(
            RecordingService::validate_quality("invalid"),
            Err(RecordingError::InvalidQuality)
        );
        assert_eq!(
            RecordingService::validate_quality(""),
            Err(RecordingError::InvalidQuality)
        );
    }

    #[test]
    fn normalize_channel_login_accepts_valid_input() {
        assert_eq!(normalize_channel_login("shroud").unwrap(), "shroud");
        assert_eq!(normalize_channel_login("  Shroud  ").unwrap(), "shroud");
    }

    #[test]
    fn normalize_channel_login_rejects_empty() {
        assert_eq!(
            normalize_channel_login(""),
            Err("channel login cannot be empty".to_string())
        );
        assert_eq!(
            normalize_channel_login("   "),
            Err("channel login cannot be empty".to_string())
        );
    }

    #[test]
    fn validate_recording_filename_accepts_valid_names() {
        assert_eq!(
            validate_recording_filename("recording.ts").unwrap(),
            "recording.ts"
        );
        assert_eq!(
            validate_recording_filename("  recording.ts  ").unwrap(),
            "recording.ts"
        );
    }

    #[test]
    fn validate_recording_filename_rejects_empty() {
        assert_eq!(
            validate_recording_filename(""),
            Err(RecordingError::EmptyFilename)
        );
        assert_eq!(
            validate_recording_filename("   "),
            Err(RecordingError::EmptyFilename)
        );
    }

    #[test]
    fn validate_recording_filename_rejects_invalid() {
        assert_eq!(
            validate_recording_filename("path/to/recording.ts"),
            Err(RecordingError::InvalidFilename)
        );
        assert_eq!(
            validate_recording_filename("recording\\.ts"),
            Err(RecordingError::InvalidFilename)
        );
        assert_eq!(
            validate_recording_filename("."),
            Err(RecordingError::InvalidFilename)
        );
        assert_eq!(
            validate_recording_filename(".."),
            Err(RecordingError::InvalidFilename)
        );
    }

    #[test]
    fn validate_recording_filename_rejects_path_traversal() {
        // Path traversal patterns
        assert_eq!(
            validate_recording_filename("../foo.mp4"),
            Err(RecordingError::InvalidFilename)
        );
        assert_eq!(
            validate_recording_filename("../recording.ts"),
            Err(RecordingError::InvalidFilename)
        );
        assert_eq!(
            validate_recording_filename("../../etc/passwd"),
            Err(RecordingError::InvalidFilename)
        );
        assert_eq!(
            validate_recording_filename("foo/../bar.ts"),
            Err(RecordingError::InvalidFilename)
        );
    }
}
