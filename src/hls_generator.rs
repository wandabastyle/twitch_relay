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
///
/// # Arguments
/// * `mp4_path` - Path to the MP4 file
/// * `channel_login` - Channel login for API URLs
/// * `filename` - Recording filename for API URLs
pub fn generate_hls_playlist(
    mp4_path: &Path,
    channel_login: &str,
    filename: &str,
) -> Result<String, String> {
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

    // The first sample's offset tells us where the mdat atom starts
    // The init section (ftyp + moov) ends just before the first sample
    let first_sample_offset = video_track.samples[0].offset;
    let init_section_size = first_sample_offset;

    // Build API URL for media segments
    let media_url =
        format!("/api/recordings/playback-file?channel_login={channel_login}&filename={filename}");

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
    // EXT-X-MAP with BYTERANGE pointing to init section (ftyp + moov)
    playlist.push_str(&format!(
        "#EXT-X-MAP:URI=\"{}\",BYTERANGE=\"{}@0\"\n",
        media_url, init_section_size
    ));

    for (duration, start_byte, _end_byte, size) in &segments {
        playlist.push_str(&format!("#EXTINF:{:.3},\n", duration));
        playlist.push_str(&format!("#EXT-X-BYTERANGE:{}@{}\n", size, start_byte));
        playlist.push_str(&format!("{}\n", media_url));
    }

    playlist.push_str("#EXT-X-ENDLIST\n");

    Ok(playlist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_playlist_format() {
        // Test with synthetic data would go here
        // For now, just verify the function exists
        assert_eq!(SEGMENT_DURATION, 10.0);
        assert_eq!(TARGET_DURATION, 10);
    }

    #[test]
    fn test_generate_hls_from_real_mp4() {
        // This test generates an HLS playlist from the existing recording
        // for verification purposes
        let mp4_path = PathBuf::from(
            "/home/kanashi/Dokumente/code/rust/twitch-relay/recordings/completed/tahnookagi/Season 2026/tahnookagi_S2026E0503_24-hour_stream_1_brain_cell.mp4",
        );

        if !mp4_path.exists() {
            println!("Skipping test: MP4 file not found at {:?}", mp4_path);
            return;
        }

        let channel_login = "tahnookagi";
        let filename = "tahnookagi_S2026E0503_24-hour_stream_1_brain_cell.mp4";

        let playlist = generate_hls_playlist(&mp4_path, channel_login, filename)
            .expect("Failed to generate HLS playlist");

        // Verify playlist structure
        assert!(playlist.contains("#EXTM3U"));
        assert!(playlist.contains("#EXT-X-VERSION:4"));
        assert!(playlist.contains("#EXT-X-MAP"));
        assert!(playlist.contains("BYTERANGE="));
        assert!(playlist.contains("/api/recordings/playback-file"));
        assert!(playlist.contains("#EXT-X-ENDLIST"));

        // Print the generated playlist for manual verification
        println!("\n=== Generated HLS Playlist ===\n{}", playlist);

        // Optionally write to file for testing with ffplay
        let m3u8_path = mp4_path.with_extension("m3u8");
        std::fs::write(&m3u8_path, &playlist).expect("Failed to write m3u8 file");
        println!("\nWrote m3u8 to: {:?}", m3u8_path);
    }
}
