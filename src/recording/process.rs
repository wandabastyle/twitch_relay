use std::path::PathBuf;

use super::{
   files::{
      build_completed_recording_path,
      move_file_if_exists,
   },
   metadata::ChapterEvent,
   types::{
      ActiveRecording,
      NfoContext,
   },
};

/// Active recording process with runtime state.
#[derive(Debug)]
pub(super) struct ActiveProcess {
   pub(super) metadata:                   ActiveRecording,
   pub(super) stream_title:               Option<String>,
   pub(super) last_observed_game:         Option<String>,
   pub(super) pending_game:               Option<String>,
   pub(super) pending_game_confirmations: u64,
   pub(super) chapter_events:             Vec<ChapterEvent>,
   pub(super) child:                      tokio::process::Child,
}

/// Reconciles exited recordings by checking process status and finalizing them.
pub(super) async fn reconcile_exited_recordings(
   active: &tokio::sync::RwLock<std::collections::HashMap<String, ActiveProcess>>,
   recordings_dir: &std::path::Path,
   nfo_ctx: &NfoContext<'_>,
   ffmpeg_path: &str,
) -> Vec<(String, ActiveRecording)> {
   let mut finished: Vec<(String, ActiveProcess, std::process::ExitStatus)> = Vec::new();

   {
      let mut active_write = active.write().await;
      let keys: Vec<String> = active_write.keys().cloned().collect();
      for key in keys {
         let status = match active_write.get_mut(&key) {
            Some(process) => {
               match process.child.try_wait() {
                  Ok(status) => status,
                  Err(error) => {
                     tracing::error!(channel = %key, error = %error, "failed to poll recording process status");
                     None
                  },
               }
            },
            None => None,
         };

         if let Some(status) = status
            && let Some(process) = active_write.remove(&key)
         {
            finished.push((key, process, status));
         }
      }
   }

   let mut result = Vec::new();
   for (channel_login, process, exit) in finished {
      finalize_exited_process(
         &channel_login,
         &process,
         exit,
         recordings_dir,
         nfo_ctx,
         ffmpeg_path,
      )
      .await;
      result.push((channel_login, process.metadata));
   }
   result
}

/// Finalizes a process that has exited.
async fn finalize_exited_process(
   channel_login: &str,
   process: &ActiveProcess,
   exit: std::process::ExitStatus,
   recordings_dir: &std::path::Path,
   nfo_ctx: &NfoContext<'_>,
   ffmpeg_path: &str,
) {
   use super::{
      nfo::write_tv_nfo_files,
      playback::write_playback_assets,
   };

   let output_path = PathBuf::from(&process.metadata.output_path);
   if !output_path.exists() {
      tracing::info!(channel = %channel_login, status = ?exit, "recording process exited with no output file present");
      return;
   }

   if exit.success() {
      let completed_dir = recordings_dir.join("completed");
      let final_path = build_completed_recording_path(
         &channel_completed_dir(&completed_dir, channel_login),
         channel_login,
         &process.metadata,
         process.stream_title.as_deref(),
      );
      move_file_if_exists(&output_path, &final_path);

      // Write playback assets (HLS playlist, etc.)
      write_playback_assets(
         channel_login,
         &final_path,
         &process.metadata,
         &process.chapter_events,
         recordings_dir,
         ffmpeg_path,
      )
      .await;

      // Write NFO if enabled
      if nfo_ctx.write_nfo
         && nfo_ctx.nfo_style == crate::config::RecordingNfoStyle::Tv
         && let Err(error) = write_tv_nfo_files(
            channel_login,
            &final_path,
            &process.metadata,
            process.stream_title.as_deref(),
            nfo_ctx.twitch,
         )
         .await
      {
         tracing::warn!(
             channel = %channel_login,
             path = %final_path.display(),
             error = %error,
             "failed to write recording nfo"
         );
      }

      // Prune old recordings
      super::files::prune_completed_channel_dir(
         &channel_completed_dir(&completed_dir, channel_login),
         get_keep_last_for_channel(channel_login),
      );

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

   let incomplete_dir = recordings_dir.join("incomplete");
   let final_path = channel_completed_dir(&incomplete_dir, channel_login).join(filename);
   move_file_if_exists(&output_path, &final_path);
   tracing::warn!(
       channel = %channel_login,
       status = ?exit,
       from = %output_path.display(),
       to = %final_path.display(),
       "recording exited abnormally"
   );
}

/// Get the number of recordings to keep for a channel.
fn get_keep_last_for_channel(channel_login: &str) -> usize {
   use crate::recording_rules;

   recording_rules::load_rules().map_or(0, |rules| {
      rules
         .into_iter()
         .find(|rule| rule.channel_login == channel_login)
         .and_then(|rule| rule.keep_last_videos)
         .map_or(0, |v| usize::try_from(v).unwrap_or(usize::MAX))
   })
}

/// Helper to get channel-specific directory within a bucket.
fn channel_completed_dir(base: &std::path::Path, channel_login: &str) -> std::path::PathBuf {
   base.join(super::files::sanitize_filename(channel_login))
}
