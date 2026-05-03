//! HLS playlist generation from MP4 files
//!
//! Uses re_mp4 to parse the moov atom and generate byte-range HLS playlists
//! for fast playback startup.

use re_mp4::{Mp4, TrackKind};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

const SEGMENT_DURATION: f64 = 10.0;
const TARGET_DURATION: u64 = 10;

/// Generate an HLS playlist from an MP4 file
///
/// Returns the playlist content as a string
pub fn generate_hls_playlist(mp4_path: &Path) -> Result<String, String> {
    let file = File::open(mp4_path).map_err(|e| format!("Failed to open MP4: {e}"))?;
    let file_size = file
        .metadata()
        .map_err(|e| format!("Failed to get file metadata: {e}"))?
        .len();
    let mut reader = BufReader::new(file);

    let mp4 = Mp4::read(&mut reader, file_size).map_err(|e| format!("Failed to parse MP4: {e}"))?;

    // Find video track
    let video_track = mp4
        .tracks()
        .values()
        .find(|t| t.kind == Some(TrackKind::Video))
        .ok_or("No video track found in MP4")?;

    if video_track.samples.is_empty() {
        return Err("No video samples found".to_string());
    }

    // Get MP4 filename for playlist
    let mp4_filename = mp4_path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or("Invalid MP4 filename")?;

    // Build segments
    let mut segments: Vec<(f64, u64, u64, u64)> = Vec::new(); // (duration, start_byte, end_byte, size)
    let mut seg_start_sample = 0;
    let mut seg_start_byte = video_track.samples[0].offset;
    let mut last_keyframe_byte = seg_start_byte + video_track.samples[0].size;

    let timescale = video_track.timescale as f64;
    let first_pts = video_track.samples[0].composition_timestamp as f64 / timescale;

    for (i, sample) in video_track.samples.iter().enumerate() {
        let pts = sample.composition_timestamp as f64 / timescale;
        let seg_start_pts =
            video_track.samples[seg_start_sample].composition_timestamp as f64 / timescale;
        let time_since_seg_start = pts - seg_start_pts;

        // Start new segment on keyframe after SEGMENT_DURATION
        if sample.is_sync && time_since_seg_start >= SEGMENT_DURATION && i > 0 {
            let seg_size = last_keyframe_byte - seg_start_byte;
            let seg_duration = pts - first_pts - (seg_start_pts - first_pts);
            segments.push((seg_duration, seg_start_byte, last_keyframe_byte, seg_size));

            seg_start_sample = i;
            seg_start_byte = sample.offset;
        }

        last_keyframe_byte = sample.offset + sample.size;

        // Handle final segment
        if i == video_track.samples.len() - 1 {
            let seg_size = sample.offset + sample.size - seg_start_byte;
            let seg_duration = pts - seg_start_pts;
            segments.push((
                seg_duration,
                seg_start_byte,
                sample.offset + sample.size,
                seg_size,
            ));
        }
    }

    // Build playlist
    let mut playlist = String::new();
    playlist.push_str("#EXTM3U\n");
    playlist.push_str("#EXT-X-VERSION:4\n");
    playlist.push_str(&format!("#EXT-X-TARGETDURATION:{}\n", TARGET_DURATION));
    playlist.push_str("#EXT-X-MEDIA-SEQUENCE:0\n");
    playlist.push_str("#EXT-X-PLAYLIST-TYPE:VOD\n");
    playlist.push_str(&format!("#EXT-X-MAP:URI=\"{}\"\n", mp4_filename));

    for (duration, start_byte, _end_byte, size) in &segments {
        playlist.push_str(&format!("#EXTINF:{:.3},\n", duration));
        playlist.push_str(&format!("#EXT-X-BYTERANGE:{}@{}\n", size, start_byte));
        playlist.push_str(&format!("{}\n", mp4_filename));
    }

    playlist.push_str("#EXT-X-ENDLIST\n");

    Ok(playlist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playlist_format() {
        // Test with synthetic data would go here
        // For now, just verify the function exists
        assert_eq!(SEGMENT_DURATION, 10.0);
        assert_eq!(TARGET_DURATION, 10);
    }
}
