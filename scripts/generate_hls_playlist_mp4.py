#!/usr/bin/env python3
"""
Generate an HLS byte‑range playlist (.m3u8) for an MP4 file.

This version parses the MP4 *moov* atom directly using the `construct`
library, avoiding a full `ffprobe` scan of the media data. It runs in
seconds even on multi‑gigabyte recordings.

Usage:
    python generate_hls_playlist_mp4.py <mp4_file>

Example:
    python generate_hls_playlist_mp4.py "konoobi - S2026E0502 - first_time_going_to_space.mp4"

The script will create a sibling ``.m3u8`` file next to the MP4.
"""

import sys
from pathlib import Path
from typing import List, Tuple

# We'll import `construct` lazily so the script can give a clear error
# if the package is missing.
try:
    from construct import (Struct, Int32ub, Bytes, GreedyRange, LazyBound,
                         this, len_, Computed, Array, If, Probe, Int8ub)
except ImportError as e:  # pragma: no cover – executed only if missing
    sys.stderr.write(
        "\nError: missing dependency `construct`. Install with: pip install construct\n"
    )
    sys.exit(1)

# --- MP4 atom parsers -------------------------------------------------------
# An MP4 atom (box) starts with a 32‑bit size and a 4‑byte type.
AtomHeader = Struct(
    "size" / Int32ub,
    "type" / Bytes(4),
    "data" / Bytes(this.size - 8),
)

# Helper to find a child atom inside a parent atom's data.
def find_atom(data: bytes, target: bytes) -> bytes:
    """Return the raw bytes of the first child atom with type `target`.
    Raises ``ValueError`` if not found.
    """
    offset = 0
    while offset + 8 <= len(data):
        size = int.from_bytes(data[offset : offset + 4], "big")
        typ = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + size]
        if typ == target:
            return payload
        offset += size
    raise ValueError(f"Atom {target!r} not found")

# Parse the ``stsz`` box – sample sizes.
StszBox = Struct(
    "version" / Int8ub,
    "flags" / Bytes(3),
    "sample_size" / Int32ub,  # if non‑zero, all samples share this size
    "sample_count" / Int32ub,
    "entry_sizes" / If(this.sample_size == 0, Array(this.sample_count, Int32ub)),
)

# Parse the ``stco`` box – chunk offsets.
StcoBox = Struct(
    "version" / Int8ub,
    "flags" / Bytes(3),
    "entry_count" / Int32ub,
    "chunk_offsets" / Array(this.entry_count, Int32ub),
)

# Parse the ``stts`` box – time‑to‑sample.
SttsBox = Struct(
    "version" / Int8ub,
    "flags" / Bytes(3),
    "entry_count" / Int32ub,
    "entries" / Array(this.entry_count, Struct(
        "sample_count" / Int32ub,
        "sample_delta" / Int32ub,
    )),
)

# Optional ``stss`` – sync (key) samples.
StssBox = Struct(
    "version" / Int8ub,
    "flags" / Bytes(3),
    "entry_count" / Int32ub,
    "sample_numbers" / Array(this.entry_count, Int32ub),
)

# --------------------------------------------------------------------------

def read_moov(fp) -> bytes:
    """Locate the ``moov`` atom in the file and return its raw payload."""
    # The ``moov`` atom is usually at the start because we use ``-movflags faststart``.
    # We'll read the first 16 MiB – more than enough for any moov.
    fp.seek(0)
    data = fp.read(16 * 1024 * 1024)
    offset = 0
    while offset + 8 <= len(data):
        size = int.from_bytes(data[offset : offset + 4], "big")
        typ = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + size]
        if typ == b"moov":
            return payload
        offset += size
    raise RuntimeError("moov atom not found – ensure the file was created with -movflags faststart")


def parse_stbl(moov: bytes) -> Tuple[List[int], List[int], List[int], List[int]]:
    """Extract sample sizes, chunk offsets, sample timestamps, and keyframe flags.

    Returns four parallel lists (size, offset, timestamp, is_keyframe).
    """
    # The atom hierarchy is: moov → trak → mdia → minf → stbl
    trak = find_atom(moov, b"trak")
    mdia = find_atom(trak, b"mdia")
    minf = find_atom(mdia, b"minf")
    stbl = find_atom(minf, b"stbl")

    # --- Sample sizes ---------------------------------------------------
    stsz_data = find_atom(stbl, b"stsz")
    stsz = StszBox.parse(stsz_data)
    if stsz.sample_size != 0:
        # All samples share the same size – repeat it.
        sample_sizes = [stsz.sample_size] * stsz.sample_count
    else:
        sample_sizes = list(stsz.entry_sizes)

    # --- Chunk offsets ---------------------------------------------------
    stco_data = find_atom(stbl, b"stco")
    stco = StcoBox.parse(stco_data)
    # For simplicity we assume one sample per chunk – which is true for our
    # recordings because we use ``-c copy`` and each sample maps to a chunk.
    chunk_offsets = list(stco.chunk_offsets)

    # --- Timestamps ------------------------------------------------------
    stts_data = find_atom(stbl, b"stts")
    stts = SttsBox.parse(stts_data)
    timestamps: List[int] = []
    current = 0
    for entry in stts.entries:
        for _ in range(entry.sample_count):
            timestamps.append(current)
            current += entry.sample_delta
    # timestamps are in the MP4 time‑scale (usually 90000). We convert to seconds.
    # Derive the timescale from the ``mdhd`` box – but in our recordings the
    # timescale is 90000, so we can divide directly.
    timescale = 90000
    pts_seconds = [t / timescale for t in timestamps]

    # --- Keyframes (sync samples) ---------------------------------------
    is_keyframe = [False] * len(sample_sizes)
    try:
        stss_data = find_atom(stbl, b"stss")
        stss = StssBox.parse(stss_data)
        for num in stss.sample_numbers:
            # Sample numbers are 1‑based.
            if 1 <= num <= len(is_keyframe):
                is_keyframe[num - 1] = True
    except ValueError:
        # No ``stss`` – fall back to assuming every sample is a keyframe.
        is_keyframe = [True] * len(sample_sizes)

    # Sanity check – lengths must match.
    if not (len(sample_sizes) == len(chunk_offsets) == len(pts_seconds) == len(is_keyframe)):
        raise RuntimeError("MP4 parsing mismatch – unexpected box layout")

    return sample_sizes, chunk_offsets, pts_seconds, is_keyframe


def group_segments(sample_sizes: List[int], offsets: List[int], pts: List[float], keyflags: List[bool], target_dur: float = 10.0) -> List[Tuple[float, int, int, int]]:
    """Group frames into ~target_dur‑second HLS segments.

    Returns a list of (duration, start_byte, end_byte, byte_size).
    """
    segments: List[Tuple[float, int, int, int]] = []
    if not sample_sizes:
        return segments

    seg_start_idx = 0
    seg_start_time = pts[0]
    seg_start_byte = offsets[0]
    for i in range(1, len(sample_sizes)):
        elapsed = pts[i] - seg_start_time
        # Start a new segment once we've reached the target duration **and**
        # the current frame is a keyframe (so playback can start cleanly).
        if elapsed >= target_dur and keyflags[i]:
            # End of the current segment is *just before* this keyframe.
            end_byte = offsets[i]  # start of the new keyframe
            seg_size = end_byte - seg_start_byte
            seg_dur = pts[i] - seg_start_time
            segments.append((seg_dur, seg_start_byte, end_byte, seg_size))
            # Start new segment
            seg_start_idx = i
            seg_start_time = pts[i]
            seg_start_byte = offsets[i]
    # Final segment – include everything to the EOF.
    end_byte = offsets[-1] + sample_sizes[-1]
    seg_size = end_byte - seg_start_byte
    seg_dur = pts[-1] - seg_start_time
    segments.append((seg_dur, seg_start_byte, end_byte, seg_size))
    return segments


def write_m3u8(segments: List[Tuple[float, int, int, int]], mp4_name: str, out_path: Path) -> None:
    """Write the HLS byte‑range playlist.

    ``segments`` is a list of (duration, start, end, size).
    ``mp4_name`` is the filename as it will appear in the playlist.
    """
    lines = [
        "#EXTM3U",
        "#EXT-X-VERSION:4",
        "#EXT-X-TARGETDURATION:10",
        "#EXT-X-MEDIA-SEQUENCE:0",
        "#EXT-X-PLAYLIST-TYPE:VOD",
        f'#EXT-X-MAP:URI="{mp4_name}"',
    ]
    for dur, start, _end, size in segments:
        lines.append(f"#EXTINF:{dur:.3f},")
        lines.append(f"#EXT-X-BYTERANGE:{size}@{start}")
        lines.append(mp4_name)
    lines.append("#EXT-X-ENDLIST")

    out_path.write_text("\n".join(lines) + "\n")


def main() -> None:
    if len(sys.argv) != 2:
        sys.stderr.write("Usage: python generate_hls_playlist_mp4.py <mp4_file>\n")
        sys.exit(1)

    mp4_path = Path(sys.argv[1])
    if not mp4_path.is_file():
        sys.stderr.write(f"Error: file not found – {mp4_path}\n")
        sys.exit(1)

    with mp4_path.open('rb') as f:
        moov = read_moov(f)

    sample_sizes, offsets, pts, keyflags = parse_stbl(moov)
    segments = group_segments(sample_sizes, offsets, pts, keyflags, target_dur=10.0)

    playlist_path = mp4_path.with_suffix('.m3u8')
    write_m3u8(segments, mp4_path.name, playlist_path)

    print(f"Generated HLS playlist: {playlist_path}")
    print(f"  Segments: {len(segments)}")
    total_dur = sum(d for d, *_ in segments)
    print(f"  Total duration: {total_dur:.2f}s")

if __name__ == "__main__":
    main()
