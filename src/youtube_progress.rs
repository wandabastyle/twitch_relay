// Re-export for backward compatibility
pub use crate::progress_store::{
   ProgressEntry as YoutubeWatchProgressEntry,
   ProgressUpsertResult as YoutubeProgressUpsertResult,
};
use crate::{
   progress_store::ProgressStore,
   storage::paths,
};

#[derive(Debug, Clone)]
pub struct YoutubeProgressStore {
   inner: ProgressStore,
}

impl YoutubeProgressStore {
   pub fn new() -> Self {
      Self {
         inner: ProgressStore::new(paths::youtube_watch_progress_path),
      }
   }

   pub fn get(&self, session_token: &str, video_id: &str) -> Option<YoutubeWatchProgressEntry> {
      self.inner.get(session_token, video_id)
   }

   pub fn upsert(
      &self,
      session_token: &str,
      video_id: &str,
      position_secs: f64,
      duration_secs: Option<f64>,
      completed: Option<bool>,
   ) -> Option<YoutubeProgressUpsertResult> {
      self.inner.upsert(
         session_token,
         video_id,
         position_secs,
         duration_secs,
         completed,
      )
   }
}

impl Default for YoutubeProgressStore {
   fn default() -> Self {
      Self::new()
   }
}
