use std::path::Path;

/// Chapter event for recording segmentation.
#[derive(Debug, Clone)]
pub struct ChapterEvent {
   pub offset_secs: u64,
   pub title:       String,
}

/// Write `FFMetadata` chapter file for ffmpeg.
pub fn write_ffmetadata_chapters(path: &Path, events: &[ChapterEvent]) -> Result<(), String> {
   let mut content = String::from(";FFMETADATA1\n");
   for (index, event) in events.iter().enumerate() {
      let start_ms = event.offset_secs.saturating_mul(1000);
      let end_ms = events
         .get(index + 1)
         .map_or(start_ms.saturating_add(1000), |next| next.offset_secs.saturating_mul(1000));
      if end_ms <= start_ms {
         continue;
      }
      content.push_str("[CHAPTER]\nTIMEBASE=1/1000\n");
      content.push_str(&format!("START={start_ms}\nEND={end_ms}\n"));
      content.push_str(&format!("title={}\n", event.title.replace('\n', " ")));
   }
   std::fs::write(path, content).map_err(|error| {
      format!(
         "failed to write chapter metadata {}: {error}",
         path.display()
      )
   })
}
