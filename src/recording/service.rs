use std::{
   collections::HashMap,
   fs,
   path::{
      Path,
      PathBuf,
   },
   process::Stdio,
   sync::Arc,
};

use tokio::{
   process::Command,
   sync::RwLock,
};

use super::{
   files::{build_recording_filename, build_completed_recording_path, move_file_if_exists, list_recording_files, pin_marker_path_for_recording, validate_recording_filename, parse_recording_filename, processing_marker_path_for_recording, find_file_by_name_recursive, sanitize_filename, prune_completed_channel_dir, is_recording_pinned},
   merge::{
      expected_completed_mp4_filename as merge_expected_filename,
      finalize_incomplete_recording as finalize_recording_impl,
      merge_incomplete_recordings as merge_recordings_impl,
      validate_merge_sources as validate_merge_impl,
   },
   nfo::write_tv_nfo_files,
   playback::{
      rebuild_hls_playlist,
      write_playback_assets,
   },
   process::{
      ActiveProcess,
      reconcile_exited_recordings,
   },
   types::{RecordingProcessingConfig, RecordingError, QUALITY_OPTIONS, RecordingMode, ActiveRecording, RecordingsOverview, RecordingBucket, RecordingFileEntry, NfoContext, RecordingProcessingState},
};
use crate::{
   config::RecordingNfoStyle,
   recording_rules,
   twitch_auth::TwitchAuthService,
   util::{
      channel::normalize_channel_login,
      time::now_unix_secs,
   },
};

/// Service for managing stream recordings.
#[derive(Debug, Clone)]
pub struct RecordingService {
   streamlink_path:              String,
   recordings_dir:               PathBuf,
   write_nfo:                    bool,
   nfo_style:                    RecordingNfoStyle,
   twitch:                       TwitchAuthService,
   ffmpeg_path:                  String,
   chapter_min_gap_secs:         u64,
   chapter_change_confirmations: u64,
   active:                       Arc<RwLock<HashMap<String, ActiveProcess>>>,
}

impl RecordingService {
   /// Create a new recording service instance.
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

   /// Validate a quality string.
   pub fn validate_quality(quality: &str) -> Result<String, RecordingError> {
      let normalized = quality.trim().to_ascii_lowercase();
      if QUALITY_OPTIONS.contains(&normalized.as_str()) {
         Ok(normalized)
      } else {
         Err(RecordingError::InvalidQuality)
      }
   }

   /// Start a new recording for a channel.
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
            RecordingError::DirectoryNotWritable(format!("recordings directory not writable: {e}"))
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
         active.insert(channel_login.clone(), ActiveProcess {
            metadata: metadata.clone(),
            stream_title: stream_title
               .map(str::trim)
               .filter(|value| !value.is_empty())
               .map(ToOwned::to_owned),
            last_observed_game: None,
            pending_game: None,
            pending_game_confirmations: 0,
            chapter_events: vec![super::metadata::ChapterEvent {
               offset_secs: 0,
               title:       "Stream Start".to_string(),
            }],
            child,
         });
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

   /// Stop an active recording for a channel.
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
         write_playback_assets(
            &channel_login,
            &final_path,
            &process.metadata,
            &process.chapter_events,
            &self.recordings_dir,
            &self.ffmpeg_path,
         )
         .await;
         self
            .write_nfo_if_enabled(
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

   /// Get list of active recordings.
   pub async fn active_recordings(&self) -> Vec<ActiveRecording> {
      self.reconcile_exited_recordings().await;

      let active = self.active.read().await;
      let mut items: Vec<ActiveRecording> = active.values().map(|p| p.metadata.clone()).collect();
      drop(active);
      items.sort_by_key(|item| std::cmp::Reverse(item.started_at_unix));
      items
   }

   /// Get a specific active recording by channel login.
   pub async fn get_active_recording(&self, channel_login: &str) -> Option<ActiveRecording> {
      self.reconcile_exited_recordings().await;

      let active = self.active.read().await;
      active
         .get(&channel_login.trim().to_ascii_lowercase())
         .map(|p| p.metadata.clone())
   }

   /// Get overview of recordings (active, completed, incomplete).
   pub async fn list_overview(&self, limit_per_bucket: usize) -> RecordingsOverview {
      self.reconcile_exited_recordings().await;

      RecordingsOverview {
         active:     self.active_recordings().await,
         completed:  list_recording_files(&self.completed_dir(), "completed", limit_per_bucket),
         incomplete: list_recording_files(&self.incomplete_dir(), "incomplete", limit_per_bucket),
      }
   }

   /// Delete a recording file from a bucket.
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

   /// Pin a recording file to prevent deletion.
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

   /// Unpin a recording file.
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

      fs::remove_file(&pin_path)
         .map_err(|error| RecordingError::UnpinFailed(format!("recording unpin failed: {error}")))
   }

   /// Merge multiple incomplete recordings.
   pub async fn merge_incomplete_recordings(
      &self,
      channel_login: &str,
      filenames: Vec<String>,
      expected_filename: &str,
   ) -> Result<RecordingFileEntry, RecordingError> {
      let channel_login =
         normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
      let nfo_ctx = NfoContext {
         write_nfo: self.write_nfo,
         nfo_style: self.nfo_style,
         twitch:    &self.twitch,
      };
      merge_recordings_impl(
         &channel_login,
         filenames,
         expected_filename,
         &self.recordings_dir,
         &self.ffmpeg_path,
         &nfo_ctx,
      )
      .await
   }

   /// Validate a merge request and return the expected filename.
   pub fn validate_merge_request(
      &self,
      channel_login: &str,
      filenames: &[String],
   ) -> Result<(String, String), RecordingError> {
      let normalized =
         normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
      let (combined, stream_title) =
         validate_merge_impl(&normalized, filenames.to_vec(), &self.recordings_dir)?;
      let expected_name = merge_expected_filename(
         &normalized,
         &combined[0].0,
         stream_title.as_deref(),
         &self.recordings_dir,
      );
      Ok((normalized, expected_name))
   }

   /// Validate a finalize request and return the expected filename.
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

      let expected_name = merge_expected_filename(
         &normalized,
         &parsed,
         parsed.title.as_deref(),
         &self.recordings_dir,
      );
      Ok((normalized, expected_name))
   }

   /// Finalize an incomplete recording.
   pub async fn finalize_incomplete_recording(
      &self,
      channel_login: &str,
      filename: &str,
      expected_filename: &str,
   ) -> Result<RecordingFileEntry, RecordingError> {
      let normalized =
         normalize_channel_login(channel_login).map_err(RecordingError::InvalidChannelLogin)?;
      let nfo_ctx = NfoContext {
         write_nfo: self.write_nfo,
         nfo_style: self.nfo_style,
         twitch:    &self.twitch,
      };
      finalize_recording_impl(
         &normalized,
         filename,
         expected_filename,
         &self.recordings_dir,
         &self.ffmpeg_path,
         &nfo_ctx,
      )
      .await
   }

   /// Repair a completed recording by rebuilding HLS or remuxing.
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
         .map(str::to_ascii_lowercase)
         .unwrap_or_default();

      match extension.as_str() {
         "mp4" => {
            rebuild_hls_playlist(&channel_login, &target_path)
               .map_err(RecordingError::RepairFailed)?;
            let marker = processing_marker_path_for_recording(&target_path);
            let _ = fs::remove_file(marker);
            Ok(completed_entry_from_path(&channel_login, &target_path))
         },
         "ts" => {
            let tmp_output = self
               .tmp_dir()
               .join(format!("repair-{}.mp4", uuid::Uuid::new_v4()));
            if let Some(parent) = tmp_output.parent() {
               fs::create_dir_all(parent).map_err(|e| {
                  RecordingError::RepairFailed(format!("failed to create temp directory: {e}"))
               })?;
            }

            let status = std::process::Command::new(&self.ffmpeg_path)
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
               .map_err(|e| RecordingError::RepairFailed(format!("ffmpeg failed to start: {e}")))?;

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

            rebuild_hls_playlist(&channel_login, &final_mp4)
               .map_err(RecordingError::RepairFailed)?;
            let marker = processing_marker_path_for_recording(&final_mp4);
            let _ = fs::remove_file(marker);

            Ok(completed_entry_from_path(&channel_login, &final_mp4))
         },
         _ => {
            Err(RecordingError::RepairFailed(
               "unsupported recording extension".to_string(),
            ))
         },
      }
   }

   /// Resolve path to a completed recording file.
   pub fn resolve_completed_file_path(
      &self,
      channel_login: &str,
      filename: &str,
   ) -> Result<PathBuf, RecordingError> {
      self.resolve_recording_file_path(RecordingBucket::Completed, channel_login, filename)
   }

   /// Note a game observation for chapter tracking.
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

      let mut process = {
         let mut active = self.active.write().await;
         let Some(p) = active.remove(&normalized) else {
            drop(active);
            return;
         };
         drop(active);
         p
      };

      let candidate = game
         .map(str::trim)
         .filter(|value| !value.is_empty())
         .map(ToOwned::to_owned);

      if process.last_observed_game == candidate {
         process.pending_game = None;
         process.pending_game_confirmations = 0;
         self.active.write().await.insert(normalized, process);
         return;
      }

      if process.pending_game != candidate {
         process.pending_game = candidate;
         process.pending_game_confirmations = 1;
         self.active.write().await.insert(normalized, process);
         return;
      }

      process.pending_game_confirmations = process.pending_game_confirmations.saturating_add(1);
      if process.pending_game_confirmations < self.chapter_change_confirmations {
         self.active.write().await.insert(normalized, process);
         return;
      }

      process.last_observed_game = process.pending_game.clone();
      process.pending_game = None;
      process.pending_game_confirmations = 0;

      let offset_secs = observed_at_unix.saturating_sub(process.metadata.started_at_unix);
      if let Some(last) = process.chapter_events.last()
         && offset_secs.saturating_sub(last.offset_secs) < self.chapter_min_gap_secs
      {
         self.active.write().await.insert(normalized, process);
         return;
      }

      let chapter_title = process.last_observed_game.as_deref().map_or_else(
         || "Game: Unknown".to_string(),
         |name| format!("Game: {name}"),
      );
      process.chapter_events.push(super::metadata::ChapterEvent {
         offset_secs,
         title: chapter_title,
      });
      self.active.write().await.insert(normalized, process);
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
      let nfo_ctx = NfoContext {
         write_nfo: self.write_nfo,
         nfo_style: self.nfo_style,
         twitch:    &self.twitch,
      };
      reconcile_exited_recordings(
         &self.active,
         &self.recordings_dir,
         &nfo_ctx,
         &self.ffmpeg_path,
      )
      .await;
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
      let entries = fs::read_dir(&tmp)
         .map_err(|e| RecordingError::Io(format!("read recordings tmp directory failed: {e}")))?;
      for entry in entries.flatten() {
         let path = entry.path();
         if path.is_file() {
            let target = incomplete.join(
               path
                  .file_name()
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

   fn tmp_dir(&self) -> PathBuf {
      self.recordings_dir.join("tmp")
   }

   fn completed_dir(&self) -> PathBuf {
      self.recordings_dir.join("completed")
   }

   fn incomplete_dir(&self) -> PathBuf {
      self.recordings_dir.join("incomplete")
   }

   fn channel_bucket_dir(&self, bucket: &str, channel_login: &str) -> PathBuf {
      self
         .recordings_dir
         .join(bucket)
         .join(sanitize_filename(channel_login))
   }

   fn prune_completed_for_channel(&self, channel_login: &str) {
      let keep_last = match recording_rules::load_rules() {
         Ok(rules) => {
            rules
               .into_iter()
               .find(|rule| rule.channel_login == channel_login)
               .and_then(|rule| rule.keep_last_videos)
         },
         Err(error) => {
            tracing::warn!(
                channel = %channel_login,
                error = %error,
                "failed to load recording rules for pruning"
            );
            None
         },
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

   async fn write_tv_nfo_files(
      &self,
      channel_login: &str,
      recording_path: &Path,
      metadata: &ActiveRecording,
      stream_title: Option<&str>,
   ) -> Result<(), String> {
      write_tv_nfo_files(
         channel_login,
         recording_path,
         metadata,
         stream_title,
         &self.twitch,
      )
      .await
   }
}

/// Create a `RecordingFileEntry` from a completed file path.
pub fn completed_entry_from_path(channel_login: &str, path: &Path) -> RecordingFileEntry {
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
