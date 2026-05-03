#!/usr/bin/env python3
"""
Generate HLS playlist (.m3u8) from frame data extracted by ffprobe.

Usage:
    python generate_hls_playlist.py <frame_data.txt> <mp4_filename>

Example:
    python generate_hls_playlist.py frame_data.txt "recording.mp4"
"""

import sys
import csv
from pathlib import Path


def parse_frame_data(filepath: str) -> list:
    """Parse ffprobe CSV output into list of frame data."""
    frames = []
    with open(filepath, 'r') as f:
        reader = csv.reader(f)
        for row in reader:
            if len(row) >= 4:
                try:
                    pts_time = float(row[0])
                    size = int(row[1])
                    pos = int(row[2])
                    pict_type = row[3] if len(row) > 3 else 'P'
                    frames.append({
                        'pts_time': pts_time,
                        'size': size,
                        'pos': pos,
                        'pict_type': pict_type
                    })
                except (ValueError, IndexError):
                    continue
    return frames


def group_into_segments(frames: list, target_duration: float = 10.0) -> list:
    """
    Group frames into segments of approximately target_duration seconds.
    Segments start on I-frames (keyframes) for proper seeking.
    
    Returns list of segments: [(duration, start_byte, end_byte, size), ...]
    """
    if not frames:
        return []
    
    segments = []
    seg_start_time = frames[0]['pts_time']
    seg_start_byte = frames[0]['pos']
    last_keyframe_byte = frames[0]['pos'] + frames[0]['size']
    
    for i, frame in enumerate(frames):
        time_in_segment = frame['pts_time'] - seg_start_time
        
        # Start new segment if:
        # 1. We're at a keyframe (I-frame)
        # 2. We've accumulated >= target_duration seconds
        # 3. Not the first frame
        if frame['pict_type'] == 'I' and time_in_segment >= target_duration and i > 0:
            # Close previous segment
            seg_duration = frame['pts_time'] - seg_start_time
            seg_size = last_keyframe_byte - seg_start_byte
            segments.append((seg_duration, seg_start_byte, last_keyframe_byte, seg_size))
            
            # Start new segment
            seg_start_time = frame['pts_time']
            seg_start_byte = frame['pos']
        
        # Track position after this frame
        last_keyframe_byte = frame['pos'] + frame['size']
        
        # Handle final segment
        if i == len(frames) - 1:
            seg_duration = frame['pts_time'] - seg_start_time
            seg_size = (frame['pos'] + frame['size']) - seg_start_byte
            segments.append((seg_duration, seg_start_byte, frame['pos'] + frame['size'], seg_size))
    
    return segments


def generate_m3u8(segments: list, mp4_filename: str, target_duration: int = 10) -> str:
    """Generate HLS playlist content."""
    lines = [
        "#EXTM3U",
        "#EXT-X-VERSION:4",
        f"#EXT-X-TARGETDURATION:{target_duration}",
        "#EXT-X-MEDIA-SEQUENCE:0",
        "#EXT-X-PLAYLIST-TYPE:VOD",
        f'#EXT-X-MAP:URI="{mp4_filename}"',
        ""
    ]
    
    for duration, start_byte, _end_byte, size in segments:
        lines.append(f"#EXTINF:{duration:.3f},")
        lines.append(f"#EXT-X-BYTERANGE:{size}@{start_byte}")
        lines.append(mp4_filename)
        lines.append("")
    
    lines.append("#EXT-X-ENDLIST")
    
    return "\n".join(lines)


def main():
    if len(sys.argv) < 3:
        print("Usage: python generate_hls_playlist.py <frame_data.txt> <mp4_filename>")
        print("Example: python generate_hls_playlist.py frame_data.txt 'recording.mp4'")
        sys.exit(1)
    
    frame_data_path = sys.argv[1]
    mp4_filename = sys.argv[2]
    
    # Validate inputs
    frame_data_file = Path(frame_data_path)
    if not frame_data_file.exists():
        print(f"Error: Frame data file not found: {frame_data_path}")
        sys.exit(1)
    
    mp4_file = Path(mp4_filename)
    if not mp4_file.exists():
        print(f"Warning: MP4 file not found: {mp4_filename}")
        print("Continuing anyway, but playback may not work.")
    
    # Parse frame data
    print(f"Parsing frame data from {frame_data_path}...")
    frames = parse_frame_data(frame_data_path)
    
    if not frames:
        print("Error: No valid frames found in frame data")
        sys.exit(1)
    
    print(f"Found {len(frames)} frames")
    print(f"Duration: {frames[-1]['pts_time'] - frames[0]['pts_time']:.2f}s")
    
    # Group into segments
    print("Grouping frames into segments...")
    segments = group_into_segments(frames)
    
    if not segments:
        print("Error: Could not create segments from frames")
        sys.exit(1)
    
    print(f"Created {len(segments)} segments")
    
    # Generate playlist
    mp4_basename = mp4_file.name
    playlist_content = generate_m3u8(segments, mp4_basename)
    
    # Write playlist file
    output_path = mp4_file.with_suffix('.m3u8')
    print(f"Writing playlist to {output_path}...")
    
    with open(output_path, 'w') as f:
        f.write(playlist_content)
    
    print(f"\nSuccess! Generated HLS playlist:")
    print(f"  - MP4 file: {mp4_basename}")
    print(f"  - Playlist: {output_path.name}")
    print(f"  - Segments: {len(segments)}")
    print(f"  - Avg segment duration: {sum(s[0] for s in segments) / len(segments):.2f}s")
    print(f"\nThe recording should now play with HLS (fast startup)!")


if __name__ == "__main__":
    main()
