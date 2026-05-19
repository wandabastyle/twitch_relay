use std::path::Path;

use super::{
   files::processing_marker_path_for_recording,
   metadata::{
      ChapterEvent,
      write_ffmetadata_chapters,
   },
};
use crate::{
   recording::ActiveRecording,
   util::time::now_unix_secs,
};

/// Write playback assets (HLS playlist, MP4 remux) for a completed recording.
pub(super) async fn write_playback_assets(
   channel_login: &str,
   recording_path: &Path,
   metadata: &ActiveRecording,
   chapter_events: &[ChapterEvent],
   recordings_dir: &Path,
   ffmpeg_path: &str,
) {
   let mut chapters = chapter_events.to_vec();
   let end_offset = now_unix_secs().saturating_sub(metadata.started_at_unix);
   chapters.push(ChapterEvent {
      offset_secs: end_offset,
      title:       "Stream End".to_string(),
   });

   let chapter_file = recording_path.with_extension("ffmetadata");
   if let Err(error) = write_ffmetadata_chapters(&chapter_file, &chapters) {
      tracing::warn!(channel = %channel_login, error = %error, "failed to write ffmetadata chapters");
      return;
   }

   let mp4_path = recording_path.with_extension("mp4");
   let processing_marker = processing_marker_path_for_recording(&mp4_path);
   let _ = std::fs::write(&processing_marker, b"processing\n");

    // Generate fragmented MP4 (fMP4) for proper HLS byte-range playback
    // Creates moof+mdat fragments aligned with keyframes, ~10 seconds each
    let remux_ok = tokio::process::Command::new(ffmpeg_path)
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
          .is_ok_and(|status| status.success());

   if !remux_ok {
      tracing::warn!(channel = %channel_login, path = %recording_path.display(), "ffmpeg mp4 remux failed");
      // On remux failure, preserve the source .ts file by moving it to incomplete/
      // rather than deleting it, to prevent data loss.
      let incomplete_ts_path = recordings_dir.join("incomplete").join(
         recording_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("failed_recording.ts"),
      );
      if let Some(parent) = incomplete_ts_path.parent() {
         let _ = std::fs::create_dir_all(parent);
      }
      if let Err(e) = std::fs::rename(recording_path, &incomplete_ts_path) {
         tracing::warn!(
             channel = %channel_login,
             from = %recording_path.display(),
             to = %incomplete_ts_path.display(),
             error = %e,
             "failed to move failed recording to incomplete"
         );
         // If rename fails, keep the file in place - do NOT delete it
      } else {
         tracing::info!(
             channel = %channel_login,
             from = %recording_path.display(),
             to = %incomplete_ts_path.display(),
             "moved failed recording to incomplete"
         );
      }
      let _ = std::fs::remove_file(&chapter_file);
      let _ = std::fs::remove_file(&processing_marker);
      return;
   }

   // Only remove the source .ts file after successful remux
   let _ = std::fs::remove_file(recording_path);
   let _ = std::fs::remove_file(&chapter_file);

   // Generate HLS playlist for byte-range playback using pure Rust (fast!)
   let mp4_filename = mp4_path
      .file_name()
      .and_then(|f| f.to_str())
      .unwrap_or("recording.mp4");
   match crate::hls_generator::generate_hls_playlist(&mp4_path, channel_login, mp4_filename) {
      Ok(playlist_content) => {
         let playlist_path = mp4_path.with_extension("m3u8");
         if let Err(e) = std::fs::write(&playlist_path, playlist_content) {
            tracing::warn!(channel = %channel_login, error = %e, "failed to write hls playlist file");
         } else {
            tracing::info!(channel = %channel_login, path = %playlist_path.display(), "hls playlist generated");
            let _ = std::fs::remove_file(&processing_marker);
         }
      },
      Err(error) => {
         tracing::warn!(channel = %channel_login, path = %mp4_path.display(), error = %error, "failed to generate hls playlist");
         // Non-fatal: MP4 still works for direct playback
         let _ = std::fs::remove_file(&processing_marker);
      },
   }
}

/// Rebuild HLS playlist for an existing MP4 file.
pub(super) fn rebuild_hls_playlist(channel_login: &str, mp4_path: &Path) -> Result<(), String> {
   let mp4_filename = mp4_path
      .file_name()
      .and_then(|f| f.to_str())
      .unwrap_or("recording.mp4");
   let playlist_content =
      crate::hls_generator::generate_hls_playlist(mp4_path, channel_login, mp4_filename)
         .map_err(|e| format!("failed to generate hls playlist: {e}"))?;
   let playlist_path = mp4_path.with_extension("m3u8");
   std::fs::write(&playlist_path, playlist_content)
      .map_err(|e| format!("failed to write hls playlist: {e}"))
}
