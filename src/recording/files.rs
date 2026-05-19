use std::{
   fs,
   path::{
      Path,
      PathBuf,
   },
   time::SystemTime,
};

use time::{
   format_description,
   format_description::parse as parse_format,
};

use super::{
   nfo::{
      datetime_from_unix,
      next_same_day_suffix_index,
   },
   types::{RecordingMode, RecordingError, ActiveRecording, RecordingFileEntry, RecordingProcessingState},
};

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

pub(super) fn parse_filename_timestamp_to_unix(timestamp_str: &str) -> Option<u64> {
   let Ok(format) = parse_format("[year]-[month]-[day]-[hour][minute]") else {
      return None;
   };

   let Ok(datetime) = time::PrimitiveDateTime::parse(timestamp_str, &format) else {
      return None;
   };

   Some(u64::try_from(datetime.assume_utc().unix_timestamp()).unwrap_or(0))
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
      .map(|(channel_login, path)| {
         let processing_marker = processing_marker_path_for_recording(&path);
         let has_hls = path.with_extension("m3u8").exists();
         let processing_state = if processing_marker.exists() {
            RecordingProcessingState::Processing
         } else {
            RecordingProcessingState::Ready
         };
         RecordingFileEntry {
            channel_login,
            filename: path
               .file_name()
               .and_then(|f| f.to_str())
               .unwrap_or("unknown")
               .to_string(),
            path_display: path.display().to_string(),
            status: status.to_string(),
            pinned: is_recording_pinned(&path),
            has_hls,
            processing_state,
         }
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
   path
      .extension()
      .and_then(|ext| ext.to_str())
      .is_some_and(|ext| {
         matches!(
            ext.to_ascii_lowercase().as_str(),
            "ts" | "mp4" | "mkv" | "m4v" | "mov" | "webm"
         )
      })
}

pub(super) fn pin_marker_path_for_recording(recording_path: &Path) -> PathBuf {
   let file_name = recording_path
      .file_name()
      .and_then(|value| value.to_str())
      .unwrap_or("recording");
   recording_path.with_file_name(format!("{file_name}.pin"))
}

pub(super) fn processing_marker_path_for_recording(recording_path: &Path) -> PathBuf {
   let file_name = recording_path
      .file_name()
      .and_then(|value| value.to_str())
      .unwrap_or("recording");
   recording_path.with_file_name(format!("{file_name}.processing"))
}

pub(super) fn is_recording_pinned(recording_path: &Path) -> bool {
   pin_marker_path_for_recording(recording_path).exists()
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ParsedRecordingFilename {
   pub channel:   String,
   pub timestamp: String,
   pub date:      String,
   pub quality:   String,
   pub mode:      String,
   pub title:     Option<String>,
}

pub(super) fn parse_recording_filename(
   filename: &str,
) -> Result<ParsedRecordingFilename, RecordingError> {
   let trimmed = filename.trim();
   if trimmed.is_empty() {
      return Err(RecordingError::EmptyFilename);
   }

   // Validate extension
   let extension = Path::new(trimmed)
      .extension()
      .and_then(|ext| ext.to_str())
      .map(str::to_ascii_lowercase)
      .unwrap_or_default();

   if extension != "ts" {
      return Err(RecordingError::InvalidFilename);
   }

   // Remove extension for parsing
   let stem = Path::new(trimmed)
      .file_stem()
      .and_then(|stem| stem.to_str())
      .ok_or(RecordingError::InvalidFilename)?;

   // Split by underscore
   let parts: Vec<&str> = stem.split('_').collect();
   if parts.len() < 4 {
      return Err(RecordingError::InvalidFilename);
   }

   let channel = parts[0];
   let timestamp = parts[1];
   let quality = parts[2];
   let mode = parts[3];

   // Validate timestamp format (YYYY-MM-DD-HHMM)
   let date = if timestamp.len() >= 10 {
      &timestamp[0..10] // YYYY-MM-DD
   } else {
      return Err(RecordingError::InvalidFilename);
   };

   // Check if there's a title (parts[4..])
   let title = if parts.len() > 4 {
      let title_parts = &parts[4..];
      let title = title_parts.join("_");
      if title.is_empty() { None } else { Some(title) }
   } else {
      None
   };

   Ok(ParsedRecordingFilename {
      channel:   channel.to_string(),
      timestamp: timestamp.to_string(),
      date:      date.to_string(),
      quality:   quality.to_string(),
      mode:      mode.to_string(),
      title,
   })
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

#[cfg(test)]
mod tests {
   use std::fs;

   use super::{
      super::types::RecordingError,
      *,
   };

   #[test]
   fn parse_recording_filename_with_title() {
      let result =
         parse_recording_filename("forsen_2026-05-11-1430_best_manual_minecraft.ts").unwrap();
      assert_eq!(result.channel, "forsen");
      assert_eq!(result.timestamp, "2026-05-11-1430");
      assert_eq!(result.date, "2026-05-11");
      assert_eq!(result.quality, "best");
      assert_eq!(result.mode, "manual");
      assert_eq!(result.title, Some("minecraft".to_string()));
   }

   #[test]
   fn parse_recording_filename_without_title() {
      let result = parse_recording_filename("forsen_2026-05-11-1430_best_manual.ts").unwrap();
      assert_eq!(result.channel, "forsen");
      assert_eq!(result.timestamp, "2026-05-11-1430");
      assert_eq!(result.date, "2026-05-11");
      assert_eq!(result.quality, "best");
      assert_eq!(result.mode, "manual");
      assert_eq!(result.title, None);
   }

   #[test]
   fn parse_recording_filename_rejects_non_ts_extension() {
      let result = parse_recording_filename("forsen_2026-05-11-1430_best_manual.mp4");
      assert!(matches!(result, Err(RecordingError::InvalidFilename)));
   }

   #[test]
   fn parse_recording_filename_rejects_invalid_format() {
      let result = parse_recording_filename("invalid_filename.ts");
      assert!(matches!(result, Err(RecordingError::InvalidFilename)));
   }

   #[test]
   fn parse_recording_filename_rejects_empty() {
      let result = parse_recording_filename("");
      assert!(matches!(result, Err(RecordingError::EmptyFilename)));
   }

   #[test]
   fn list_recording_files_maps_processing_and_hls_state() {
      let base = std::env::temp_dir().join(format!("tr-files-test-{}", uuid::Uuid::new_v4()));
      let channel_dir = base.join("completed").join("forsen");
      fs::create_dir_all(&channel_dir).unwrap();

      let mp4 = channel_dir.join("a.mp4");
      fs::write(&mp4, b"video").unwrap();
      fs::write(mp4.with_extension("m3u8"), b"#EXTM3U").unwrap();

      let ts = channel_dir.join("b.ts");
      fs::write(&ts, b"video").unwrap();
      let marker = processing_marker_path_for_recording(&ts);
      fs::write(marker, b"processing").unwrap();

      let items = list_recording_files(&base.join("completed"), "completed", 10);
      assert_eq!(items.len(), 2);

      let ready = items.iter().find(|i| i.filename == "a.mp4").unwrap();
      assert!(ready.has_hls);
      assert_eq!(ready.processing_state, RecordingProcessingState::Ready);

      let processing = items.iter().find(|i| i.filename == "b.ts").unwrap();
      assert!(!processing.has_hls);
      assert_eq!(
         processing.processing_state,
         RecordingProcessingState::Processing
      );

      let _ = fs::remove_dir_all(base);
   }
}
