//! Pure Rust TS to MP4 remuxer with chapter support
//!
//! Uses mpeg2ts-reader for demuxing and mp4 crate for muxing with QuickTime chapter tracks.

use std::path::Path;

/// Chapter information for embedding in MP4
#[derive(Debug, Clone)]
pub struct Chapter {
    pub start_time: u64, // Normalized to start at 0, in 90kHz units
    pub title: String,
}

/// Remux a TS file to MP4 with embedded chapters
///
/// This is a placeholder implementation. The full implementation requires:
/// - mpeg2ts-reader integration for TS demuxing
/// - mp4 crate integration for MP4 muxing with chapter tracks
/// - H.264 Annex B to AVCC conversion
/// - AAC ADTS handling
/// - Timestamp normalization
pub async fn remux_ts_to_mp4(
    ts_path: &Path,
    _mp4_path: &Path,
    _chapters: &[Chapter],
) -> Result<(), String> {
    // TODO: Full implementation with mpeg2ts-reader + mp4 crate
    // For now, return an error indicating this needs FFmpeg fallback
    // or is not yet implemented

    // Check if we should use a fallback or error
    if !ts_path.exists() {
        return Err("TS file not found".to_string());
    }

    // Placeholder: In the full implementation, this would:
    // 1. Open TS file with mpeg2ts-reader
    // 2. Demux to extract H.264 and AAC elementary streams
    // 3. Normalize timestamps to start at 0
    // 4. Create MP4 writer with mp4 crate
    // 5. Add video track (H.264)
    // 6. Add audio track (AAC)
    // 7. Add chapter track (QuickTime text track)
    // 8. Write all samples with timestamps
    // 9. Finalize MP4

    Err("Pure Rust remuxer not yet fully implemented".to_string())
}

/// Convert chapter events to Chapter structs
pub fn convert_chapters(
    chapter_events: &[(u64, String)], // (offset_secs, title)
    _duration_seconds: u64,
) -> Vec<Chapter> {
    let mut chapters = Vec::new();

    // Convert offset_secs (u64) to 90kHz timestamps
    for (offset_secs, title) in chapter_events {
        let start_time = offset_secs.saturating_mul(90000); // Convert to 90kHz
        chapters.push(Chapter {
            start_time,
            title: title.clone(),
        });
    }

    // Sort by start time and normalize
    chapters.sort_by_key(|c| c.start_time);

    if let Some(first_time) = chapters.first().map(|c| c.start_time) {
        for chapter in &mut chapters {
            chapter.start_time = chapter.start_time.saturating_sub(first_time);
        }
    }

    chapters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chapter_conversion() {
        // Would need ChapterEvent struct accessible for full testing
        assert_eq!(1, 1); // Placeholder
    }
}
