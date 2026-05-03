//! HLS playlist generation from fragmented MP4 (fMP4) files
//!
//! Uses streaming parser to handle large files efficiently (8GB+) without
//! loading entire file into memory. Parses moof atoms to generate byte-range
//! HLS playlists for proper playback.
//!
//! Also extracts total duration from end-of-file moov atom (fast seek).

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

const TARGET_DURATION: u64 = 10;
const BUFFER_SIZE: usize = 256 * 1024; // 256KB buffer for faster scanning

/// Parse a box header from reader and return (box_type, box_size, header_size)
fn read_box_header<R: Read>(reader: &mut R) -> Option<(&'static str, u64, u8)> {
    let mut header = [0u8; 8];
    reader.read_exact(&mut header).ok()?;

    let size_32 = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as u64;
    let box_type = match std::str::from_utf8(&header[4..8]) {
        Ok(s) => Box::leak(s.to_string().into_boxed_str()),
        Err(_) => return None,
    };

    let (size, header_size) = if size_32 == 1 {
        // Extended size (64-bit)
        let mut ext_size = [0u8; 8];
        reader.read_exact(&mut ext_size).ok()?;
        let size_64 = u64::from_be_bytes(ext_size);
        (size_64, 16u8)
    } else {
        (size_32, 8u8)
    };

    Some((box_type, size, header_size))
}

/// Parse a box header from a byte slice
fn parse_box_header_bytes(data: &[u8], offset: usize) -> Option<(&str, u64, u8)> {
    if data.len() < offset + 8 {
        return None;
    }

    let size_32 = u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]) as u64;
    let box_type = std::str::from_utf8(&data[offset + 4..offset + 8]).ok()?;

    let (size, header_size) = if size_32 == 1 {
        // Extended size
        if data.len() < offset + 16 {
            return None;
        }
        let size_64 = u64::from_be_bytes(data[offset + 8..offset + 16].try_into().unwrap());
        (size_64, 16u8)
    } else {
        (size_32, 8u8)
    };

    Some((box_type, size, header_size))
}

/// Timescale info for all tracks
#[derive(Debug, Clone)]
struct TimescaleInfo {
    mvhd_timescale: u32, // Movie timescale (for mvhd duration)
    track_timescales: std::collections::HashMap<u32, u32>, // track_id -> mdhd timescale
}

impl TimescaleInfo {
    fn new() -> Self {
        let mut track_timescales = std::collections::HashMap::new();
        track_timescales.insert(1, 1000); // Default
        Self {
            mvhd_timescale: 1000,
            track_timescales,
        }
    }

    fn get_for_track(&self, track_id: u32) -> u32 {
        *self
            .track_timescales
            .get(&track_id)
            .unwrap_or(&self.mvhd_timescale)
    }
}

/// Scan forward to find where moov ends (init section size) and extract timescales
fn find_moov_end_and_timescales(file: &mut File) -> Result<(u64, TimescaleInfo), String> {
    let file_size = file
        .metadata()
        .map_err(|e| format!("Failed to get file metadata: {e}"))?
        .len();

    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek to start: {e}"))?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);

    let mut offset: u64 = 0;

    while offset < file_size {
        let Some((box_type, size, header_size)) = read_box_header(&mut reader) else {
            break;
        };

        if box_type == "moov" {
            // Read moov content to extract timescales
            let moov_content_size = size.saturating_sub(header_size as u64) as usize;
            if moov_content_size > 0 && moov_content_size <= 10 * 1024 * 1024 {
                // Max 10MB for moov
                let mut moov_content = vec![0u8; moov_content_size];
                if reader.read_exact(&mut moov_content).is_ok() {
                    let timescales = extract_timescales_from_moov(&moov_content);
                    return Ok((offset + size, timescales));
                }
            }
            return Ok((offset + size, TimescaleInfo::new()));
        }

        // Seek past this box
        let box_content_size = size.saturating_sub(header_size as u64);
        if box_content_size > 0 {
            reader
                .seek_relative(box_content_size as i64)
                .map_err(|e| format!("Failed to seek: {e}"))?;
        }

        offset += size;
    }

    Err("Could not find moov atom".to_string())
}

/// Extract all timescales from moov content (mvhd and all mdhd boxes)
fn extract_timescales_from_moov(moov_content: &[u8]) -> TimescaleInfo {
    let mut info = TimescaleInfo::new();
    let mut offset = 0;
    let mut current_track_id: Option<u32>;

    while offset + 8 < moov_content.len() {
        let Some((box_type, size, header_size)) = parse_box_header_bytes(moov_content, offset)
        else {
            break;
        };

        if box_type == "mvhd" {
            // mvhd structure: version(1) + flags(3) + creation(4/8) + modification(4/8) + timescale(4)
            let content_offset = offset + header_size as usize;
            if moov_content.len() >= content_offset + 12 {
                let version = moov_content[content_offset];
                let timescale_offset = if version == 1 { 16 + 8 + 8 } else { 4 + 4 + 4 };
                let ts_idx = content_offset + timescale_offset;
                if ts_idx + 4 <= moov_content.len() {
                    info.mvhd_timescale = u32::from_be_bytes([
                        moov_content[ts_idx],
                        moov_content[ts_idx + 1],
                        moov_content[ts_idx + 2],
                        moov_content[ts_idx + 3],
                    ]);
                }
            }
        } else if box_type == "trak" {
            // Parse trak to find tkhd (track_id) and mdhd (timescale)
            let trak_start = offset + header_size as usize;
            let trak_end = offset + size as usize;
            let mut trak_offset = trak_start;
            current_track_id = None;

            while trak_offset + 8 < trak_end {
                let Some((inner_type, inner_size, inner_header)) =
                    parse_box_header_bytes(moov_content, trak_offset)
                else {
                    break;
                };

                if inner_type == "tkhd" {
                    // tkhd: version(1) + flags(3) + creation(4/8) + modification(4/8) + track_id(4)
                    let content_offset = trak_offset + inner_header as usize;
                    if moov_content.len() >= content_offset + 12 {
                        let version = moov_content[content_offset];
                        let track_id_offset = if version == 1 { 4 + 8 + 8 } else { 4 + 4 + 4 };
                        let tid_idx = content_offset + track_id_offset;
                        if tid_idx + 4 <= moov_content.len() {
                            current_track_id = Some(u32::from_be_bytes([
                                moov_content[tid_idx],
                                moov_content[tid_idx + 1],
                                moov_content[tid_idx + 2],
                                moov_content[tid_idx + 3],
                            ]));
                        }
                    }
                } else if inner_type == "mdia" {
                    // Look for mdhd inside mdia
                    let mdia_start = trak_offset + inner_header as usize;
                    let mdia_end = trak_offset + inner_size as usize;
                    let mut mdia_offset = mdia_start;

                    while mdia_offset + 8 < mdia_end {
                        let Some((media_type, media_size, media_header)) =
                            parse_box_header_bytes(moov_content, mdia_offset)
                        else {
                            break;
                        };

                        if media_type == "mdhd" {
                            // mdhd: version(1) + flags(3) + creation(4/8) + modification(4/8) + timescale(4)
                            let content_offset = mdia_offset + media_header as usize;
                            if moov_content.len() >= content_offset + 12 {
                                let version = moov_content[content_offset];
                                let timescale_offset =
                                    if version == 1 { 4 + 8 + 8 } else { 4 + 4 + 4 };
                                let ts_idx = content_offset + timescale_offset;
                                if ts_idx + 4 <= moov_content.len() {
                                    let mdhd_timescale = u32::from_be_bytes([
                                        moov_content[ts_idx],
                                        moov_content[ts_idx + 1],
                                        moov_content[ts_idx + 2],
                                        moov_content[ts_idx + 3],
                                    ]);
                                    if let Some(tid) = current_track_id {
                                        info.track_timescales.insert(tid, mdhd_timescale);
                                    }
                                }
                            }
                        }

                        mdia_offset += media_size as usize;
                    }
                }

                trak_offset += inner_size as usize;
            }
        }

        offset += size as usize;
    }

    info
}

/// Fragment info: (duration_seconds, start_byte, size_bytes)
#[derive(Debug)]
struct Fragment {
    duration: f64,
    start_byte: u64,
    size: u64,
}

/// Scan file for moof+mdat pairs using streaming approach
fn parse_fragments_streaming(
    file: &mut File,
    moov_end: u64,
    timescales: &TimescaleInfo,
) -> Result<Vec<Fragment>, String> {
    let file_size = file
        .metadata()
        .map_err(|e| format!("Failed to get file metadata: {e}"))?
        .len();

    file.seek(SeekFrom::Start(moov_end))
        .map_err(|e| format!("Failed to seek to moov end: {e}"))?;

    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);

    let mut fragments = Vec::new();
    let mut offset: u64 = moov_end;

    while offset < file_size {
        // Try to read box header
        let mut header_buf = [0u8; 16]; // Max header size
        let bytes_read = match reader.read(&mut header_buf[..8]) {
            Ok(0) => break, // EOF
            Ok(n) => n,
            Err(e) => return Err(format!("Read error: {e}")),
        };

        if bytes_read < 8 {
            break; // Not enough data for header
        }

        let size_32 =
            u32::from_be_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]) as u64;
        let box_type = match std::str::from_utf8(&header_buf[4..8]) {
            Ok(s) => s,
            Err(_) => {
                // Skip invalid box
                offset += 1;
                reader
                    .seek_relative(-7)
                    .map_err(|e| format!("Seek error: {e}"))?;
                continue;
            }
        };

        let (box_size, header_size) = if size_32 == 1 {
            // Extended size - read into separate buffer
            let mut ext_buf = [0u8; 8];
            reader
                .read_exact(&mut ext_buf)
                .map_err(|e| format!("Failed to read extended size: {e}"))?;
            let size_64 = u64::from_be_bytes(ext_buf);
            (size_64, 16u64)
        } else {
            (size_32, 8u64)
        };

        match box_type {
            "moof" => {
                // Found a fragment header
                let moof_start = offset;
                let moof_size = box_size;

                // Read moof content to extract duration
                let mut moof_content = vec![0u8; (moof_size - header_size) as usize];
                reader
                    .read_exact(&mut moof_content)
                    .map_err(|e| format!("Failed to read moof content: {e}"))?;

                let duration = parse_moof_duration(&moof_content, timescales);

                // Next should be mdat
                let mdat_bytes = reader
                    .read(&mut header_buf[..8])
                    .map_err(|e| format!("Failed to read mdat header: {e}"))?;

                if mdat_bytes >= 8 {
                    let mdat_size = u32::from_be_bytes([
                        header_buf[0],
                        header_buf[1],
                        header_buf[2],
                        header_buf[3],
                    ]) as u64;
                    let mdat_type = std::str::from_utf8(&header_buf[4..8]).unwrap_or("");

                    if mdat_type == "mdat" {
                        // Valid fragment: moof + mdat
                        let fragment_size = moof_size + mdat_size;
                        fragments.push(Fragment {
                            duration,
                            start_byte: moof_start,
                            size: fragment_size,
                        });

                        // Skip mdat content
                        let mdat_content_size = mdat_size.saturating_sub(8);
                        if mdat_content_size > 0 {
                            reader
                                .seek_relative(mdat_content_size as i64)
                                .map_err(|e| format!("Failed to skip mdat: {e}"))?;
                        }

                        offset = moof_start + fragment_size;
                        continue;
                    }
                }

                // Not a valid fragment, skip moof
                offset = moof_start + moof_size;
            }
            _ => {
                // Skip other boxes
                let content_size = box_size.saturating_sub(header_size);
                if content_size > 0 {
                    reader
                        .seek_relative(content_size as i64)
                        .map_err(|e| format!("Failed to skip box: {e}"))?;
                }
                offset += box_size;
            }
        }
    }

    Ok(fragments)
}

/// Parse duration from moof content (traf > trun)
/// Only processes the first track (track_id=1) to avoid summing durations
/// from multiple tracks (video+audio+metadata)
fn parse_moof_duration(moof_content: &[u8], timescales: &TimescaleInfo) -> f64 {
    let mut offset = 0;
    let moof_end = moof_content.len();
    let mut total_sample_duration: u64 = 0;
    let mut has_duration = false;

    while offset + 8 < moof_end {
        let size = u32::from_be_bytes([
            moof_content[offset],
            moof_content[offset + 1],
            moof_content[offset + 2],
            moof_content[offset + 3],
        ]) as usize;
        let box_type = std::str::from_utf8(&moof_content[offset + 4..offset + 8]).unwrap_or("");

        if box_type == "traf" {
            // Parse traf content for trun
            let traf_end = offset + size;
            let mut traf_offset = offset + 8;
            let mut track_id: Option<u32> = None;

            while traf_offset + 8 < traf_end {
                let traf_size = u32::from_be_bytes([
                    moof_content[traf_offset],
                    moof_content[traf_offset + 1],
                    moof_content[traf_offset + 2],
                    moof_content[traf_offset + 3],
                ]) as usize;
                let traf_type =
                    std::str::from_utf8(&moof_content[traf_offset + 4..traf_offset + 8])
                        .unwrap_or("");

                // Parse tfhd to get track_id
                if traf_type == "tfhd" && traf_offset + 16 <= moof_content.len() {
                    // tfhd: size(4) + type(4) + version(1) + flags(3) + track_id(4)
                    track_id = Some(u32::from_be_bytes([
                        moof_content[traf_offset + 12],
                        moof_content[traf_offset + 13],
                        moof_content[traf_offset + 14],
                        moof_content[traf_offset + 15],
                    ]));
                }

                if traf_type == "trun" && traf_offset + 16 <= moof_content.len() {
                    // Only process track_id=1 (typically video track)
                    // Other tracks may have different timing that would inflate total duration
                    if track_id != Some(1) {
                        traf_offset += traf_size;
                        continue;
                    }

                    let sample_count = u32::from_be_bytes([
                        moof_content[traf_offset + 12],
                        moof_content[traf_offset + 13],
                        moof_content[traf_offset + 14],
                        moof_content[traf_offset + 15],
                    ]);

                    let flags = u32::from_be_bytes([
                        moof_content[traf_offset + 8],
                        moof_content[traf_offset + 9],
                        moof_content[traf_offset + 10],
                        moof_content[traf_offset + 11],
                    ]);

                    // If flag 5 (sample duration present) is set
                    if (flags & 0x000100) != 0 && sample_count > 0 {
                        // Calculate entry size based on flags
                        let mut entry_size = 0usize;
                        if (flags & 0x000100) != 0 {
                            entry_size += 4;
                        } // duration
                        if (flags & 0x000200) != 0 {
                            entry_size += 4;
                        } // size
                        if (flags & 0x000400) != 0 {
                            entry_size += 4;
                        } // flags
                        if (flags & 0x000800) != 0 {
                            entry_size += 4;
                        } // cto

                        if entry_size > 0 {
                            // Calculate correct offset to sample entries
                            // trun header: size(4) + type(4) + version(1) + flags(3) + sample_count(4) = 16 bytes
                            // Optional fields may follow based on flags
                            let mut data_offset = traf_offset + 16;
                            if (flags & 0x000001) != 0 {
                                data_offset += 4;
                            } // Skip data_offset field
                            if (flags & 0x000004) != 0 {
                                data_offset += 4;
                            } // Skip first_sample_flags field
                            total_sample_duration = (0..sample_count.min(10000))
                                .map(|i| {
                                    let idx = data_offset + i as usize * entry_size;
                                    if idx + 4 <= moof_content.len() {
                                        u32::from_be_bytes([
                                            moof_content[idx],
                                            moof_content[idx + 1],
                                            moof_content[idx + 2],
                                            moof_content[idx + 3],
                                        ]) as u64
                                    } else {
                                        0
                                    }
                                })
                                .sum();
                            has_duration = true;
                            // Found track 1 trun with duration, stop processing
                            break;
                        }
                    }
                }

                traf_offset += traf_size;
            }
        }

        offset += size;
    }

    // Use track-specific timescale (track_id=1 is video)
    if has_duration {
        let track_timescale = timescales.get_for_track(1);
        total_sample_duration as f64 / track_timescale as f64
    } else {
        10.0 // Default fallback
    }
}

/// Generate an HLS playlist from an fMP4 file using streaming parser
///
/// Returns the playlist content as a string
///
/// # Arguments
/// * `mp4_path` - Path to the fMP4 file
/// * `channel_login` - Channel login for API URLs
/// * `filename` - Recording filename for API URLs
pub fn generate_hls_playlist(
    mp4_path: &Path,
    channel_login: &str,
    filename: &str,
) -> Result<String, String> {
    let mut file = File::open(mp4_path).map_err(|e| format!("Failed to open MP4: {e}"))?;

    // Phase 1: Find init section size (moov end) and extract timescales
    let (init_section_size, timescales) = find_moov_end_and_timescales(&mut file)?;

    // Phase 2: Scan for fragments (streaming, minimal memory)
    let fragments = parse_fragments_streaming(&mut file, init_section_size, &timescales)?;

    if fragments.is_empty() {
        return Err("No fMP4 fragments found".to_string());
    }

    // Calculate total duration by summing fragment durations (fastest & most accurate)
    let total_duration: f64 = fragments.iter().map(|f| f.duration).sum();

    // Build API URL for media segments
    let media_url =
        format!("/api/recordings/playback-file?channel_login={channel_login}&filename={filename}");

    // Build playlist
    let mut playlist = String::new();
    playlist.push_str("#EXTM3U\n");
    playlist.push_str("#EXT-X-VERSION:6\n");
    playlist.push_str(&format!("#EXT-X-TARGETDURATION:{}\n", TARGET_DURATION));
    playlist.push_str("#EXT-X-MEDIA-SEQUENCE:0\n");
    playlist.push_str("#EXT-X-PLAYLIST-TYPE:VOD\n");

    // Include total duration for player
    if total_duration > 0.0 {
        playlist.push_str(&format!("#EXT-X-DURATION:{:.3}\n", total_duration));
    }

    // EXT-X-MAP with BYTERANGE pointing to init section (ftyp + moov)
    playlist.push_str(&format!(
        "#EXT-X-MAP:URI=\"{}\",BYTERANGE=\"{}@0\"\n",
        media_url, init_section_size
    ));

    for frag in &fragments {
        playlist.push_str(&format!("#EXTINF:{:.3},\n", frag.duration));
        playlist.push_str(&format!(
            "#EXT-X-BYTERANGE:{}@{}\n",
            frag.size, frag.start_byte
        ));
        playlist.push_str(&format!("{}\n", media_url));
    }

    playlist.push_str("#EXT-X-ENDLIST\n");

    Ok(playlist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playlist_format() {
        assert_eq!(TARGET_DURATION, 10);
    }
}
