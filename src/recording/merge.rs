use std::{path::Path, process::Command as StdCommand};

use crate::util::time::now_unix_secs;

use super::{files::*, playback::rebuild_hls_playlist, types::*};

/// Merge source item with parsed filename and path.
type MergeSourceItem = (ParsedRecordingFilename, std::path::PathBuf);

/// Merge multiple incomplete recordings into a single completed recording.
pub(super) async fn merge_incomplete_recordings(
    channel_login: &str,
    filenames: Vec<String>,
    expected_filename: &str,
    recordings_dir: &Path,
    ffmpeg_path: &str,
    write_nfo: bool,
    nfo_style: crate::config::RecordingNfoStyle,
    twitch: &crate::twitch_auth::TwitchAuthService,
) -> Result<RecordingFileEntry, RecordingError> {
    let filename = validate_recording_filename(expected_filename)?;
    let (mut combined, stream_title) =
        validate_merge_sources(channel_login, filenames, recordings_dir)?;
    combined.sort_by(|a, b| a.0.timestamp.cmp(&b.0.timestamp));

    // Create temporary directory
    let tmp_dir = recordings_dir.join(format!("tmp/merge-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| {
        RecordingError::MergeFailed(format!("failed to create temp directory: {e}"))
    })?;

    // Create concat file
    let concat_path = tmp_dir.join("concat.txt");
    let mut concat_content = String::new();

    for (_, file_path) in &combined {
        // ffmpeg concat demuxer requires absolute paths
        let abs_path = std::fs::canonicalize(file_path).map_err(|e| {
            RecordingError::MergeFailed(format!("failed to resolve file path: {e}"))
        })?;
        concat_content.push_str(&format!("file '{}\n'", abs_path.display()));
    }

    std::fs::write(&concat_path, concat_content)
        .map_err(|e| RecordingError::MergeFailed(format!("failed to write concat file: {e}")))?;

    // Merge directly to fragmented MP4 in temporary storage
    let merged_path = tmp_dir.join("merged.mp4");
    let output = StdCommand::new(ffmpeg_path)
        .arg("-y")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(&concat_path)
        .arg("-c")
        .arg("copy")
        .arg("-bsf:a")
        .arg("aac_adtstoasc")
        .arg("-movflags")
        .arg("frag_keyframe+empty_moov+delay_moov+default_base_moof")
        .arg("-frag_duration")
        .arg("10000000")
        .arg(&merged_path)
        .output()
        .map_err(|e| RecordingError::MergeFailed(format!("ffmpeg failed to start: {e}")))?;

    if !output.status.success() {
        let stderr =
            String::from_utf8(output.stderr).unwrap_or_else(|_| "unknown error".to_string());
        return Err(RecordingError::MergeFailed(format!(
            "ffmpeg merge failed: {stderr}"
        )));
    }

    // Determine metadata from first file
    let first_file = &combined[0];
    let started_at_unix =
        parse_filename_timestamp_to_unix(&first_file.0.timestamp).unwrap_or_else(now_unix_secs);
    let quality = first_file.0.quality.clone();
    let mode = match first_file.0.mode.as_str() {
        "manual" => RecordingMode::Manual,
        "auto" => RecordingMode::Auto,
        _ => RecordingMode::Manual,
    };
    let stream_title = stream_title.as_deref();

    let final_name = std::path::Path::new(&filename)
        .with_extension("mp4")
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("merged.mp4")
        .to_string();
    let completed_dir = recordings_dir.join("completed");
    let final_path = channel_bucket_dir(&completed_dir, channel_login).join(final_name);
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            RecordingError::MergeFailed(format!("failed to create destination directory: {e}"))
        })?;
    }
    std::fs::rename(&merged_path, &final_path).map_err(|e| {
        RecordingError::MergeFailed(format!("failed to finalize merged output: {e}"))
    })?;
    let processing_marker = processing_marker_path_for_recording(&final_path);
    let _ = std::fs::write(&processing_marker, b"processing\n");

    if let Err(error) = rebuild_hls_playlist(channel_login, &final_path) {
        let _ = std::fs::remove_file(&processing_marker);
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(RecordingError::MergeFailed(error));
    }
    let _ = std::fs::remove_file(&processing_marker);

    // Write NFO if enabled
    if write_nfo && nfo_style == crate::config::RecordingNfoStyle::Tv {
        use super::nfo::write_tv_nfo_files;
        let _ = write_tv_nfo_files(
            channel_login,
            &final_path,
            &ActiveRecording {
                channel_login: channel_login.to_string(),
                quality: quality.clone(),
                started_at_unix,
                output_path: final_path.display().to_string(),
                pid: None,
                mode,
                error: None,
            },
            stream_title,
            twitch,
        )
        .await;
    }

    // Source files are kept intentionally (user will delete manually)

    // Cleanup temp directory
    if let Err(e) = std::fs::remove_dir_all(&tmp_dir) {
        tracing::warn!(path = %tmp_dir.display(), error = %e, "failed to cleanup temp directory");
    }

    Ok(RecordingFileEntry {
        channel_login: channel_login.to_string(),
        filename: final_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("merged.mp4")
            .to_string(),
        path_display: final_path.display().to_string(),
        status: "completed".to_string(),
        pinned: false,
        has_hls: final_path.with_extension("m3u8").exists(),
        processing_state: RecordingProcessingState::Ready,
    })
}

/// Finalize an incomplete recording by remuxing it to MP4.
pub(super) async fn finalize_incomplete_recording(
    channel_login: &str,
    filename: &str,
    expected_filename: &str,
    recordings_dir: &Path,
    ffmpeg_path: &str,
    write_nfo: bool,
    nfo_style: crate::config::RecordingNfoStyle,
    twitch: &crate::twitch_auth::TwitchAuthService,
) -> Result<RecordingFileEntry, RecordingError> {
    let validated = validate_recording_filename(filename)?;
    let parsed = parse_recording_filename(&validated)
        .map_err(|e| RecordingError::MergeFailed(format!("invalid filename format: {e}")))?;
    if parsed.channel != channel_login {
        return Err(RecordingError::MergeFailed(format!(
            "filename channel '{}' does not match requested channel '{}'",
            parsed.channel, channel_login
        )));
    }

    let incomplete_dir = recordings_dir.join("incomplete");
    let source_path = channel_bucket_dir(&incomplete_dir, channel_login).join(&validated);
    if !source_path.exists() {
        return Err(RecordingError::MergeFailed(format!(
            "file not found: {}",
            source_path.display()
        )));
    }

    let final_name = std::path::Path::new(&validate_recording_filename(expected_filename)?)
        .with_extension("mp4")
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("recording.mp4")
        .to_string();

    let tmp_dir = recordings_dir.join(format!("tmp/finalize-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| {
        RecordingError::MergeFailed(format!("failed to create temp directory: {e}"))
    })?;
    let tmp_output = tmp_dir.join("finalized.mp4");

    let output = StdCommand::new(ffmpeg_path)
        .arg("-y")
        .arg("-i")
        .arg(&source_path)
        .arg("-c")
        .arg("copy")
        .arg("-bsf:a")
        .arg("aac_adtstoasc")
        .arg("-movflags")
        .arg("frag_keyframe+empty_moov+delay_moov+default_base_moof")
        .arg("-frag_duration")
        .arg("10000000")
        .arg(&tmp_output)
        .output()
        .map_err(|e| RecordingError::MergeFailed(format!("ffmpeg failed to start: {e}")))?;
    if !output.status.success() {
        let stderr =
            String::from_utf8(output.stderr).unwrap_or_else(|_| "unknown error".to_string());
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(RecordingError::MergeFailed(format!(
            "ffmpeg finalize failed: {stderr}"
        )));
    }

    let completed_dir = recordings_dir.join("completed");
    let final_path = channel_bucket_dir(&completed_dir, channel_login).join(final_name);
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            RecordingError::MergeFailed(format!("failed to create destination directory: {e}"))
        })?;
    }

    let processing_marker = processing_marker_path_for_recording(&final_path);
    let _ = std::fs::write(&processing_marker, b"processing\n");

    if let Err(error) = std::fs::rename(&tmp_output, &final_path) {
        let _ = std::fs::remove_file(&processing_marker);
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(RecordingError::MergeFailed(format!(
            "failed to finalize output: {error}"
        )));
    }

    if let Err(error) = rebuild_hls_playlist(channel_login, &final_path) {
        let _ = std::fs::remove_file(&processing_marker);
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(RecordingError::MergeFailed(error));
    }

    let started_at_unix =
        parse_filename_timestamp_to_unix(&parsed.timestamp).unwrap_or_else(now_unix_secs);
    let quality = parsed.quality.clone();
    let mode = match parsed.mode.as_str() {
        "manual" => RecordingMode::Manual,
        "auto" => RecordingMode::Auto,
        _ => RecordingMode::Manual,
    };

    if write_nfo && nfo_style == crate::config::RecordingNfoStyle::Tv {
        use super::nfo::write_tv_nfo_files;
        let _ = write_tv_nfo_files(
            channel_login,
            &final_path,
            &ActiveRecording {
                channel_login: channel_login.to_string(),
                quality,
                started_at_unix,
                output_path: final_path.display().to_string(),
                pid: None,
                mode,
                error: None,
            },
            parsed.title.as_deref(),
            twitch,
        )
        .await;
    }

    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_file(&processing_marker);
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(super::service::completed_entry_from_path(
        channel_login,
        &final_path,
    ))
}

/// Validate merge sources and return combined list with stream title.
pub(super) fn validate_merge_sources(
    channel_login: &str,
    filenames: Vec<String>,
    recordings_dir: &Path,
) -> Result<(Vec<MergeSourceItem>, Option<String>), RecordingError> {
    if filenames.len() < 2 {
        return Err(RecordingError::MergeFailed(
            "at least 2 files are required for merging".to_string(),
        ));
    }

    let incomplete_dir = recordings_dir.join("incomplete");
    let channel_dir = channel_bucket_dir(&incomplete_dir, channel_login);
    let mut combined: Vec<MergeSourceItem> = Vec::new();

    for filename in filenames {
        let validated = validate_recording_filename(&filename)?;
        let parsed = parse_recording_filename(&validated)
            .map_err(|e| RecordingError::MergeFailed(format!("invalid filename format: {e}")))?;

        if parsed.channel != channel_login {
            return Err(RecordingError::MergeFailed(format!(
                "filename channel '{}' does not match requested channel '{}'",
                parsed.channel, channel_login
            )));
        }

        let file_path = channel_dir.join(&validated);
        if !file_path.exists() {
            return Err(RecordingError::MergeFailed(format!(
                "file not found: {}",
                file_path.display()
            )));
        }
        combined.push((parsed, file_path));
    }

    let first_date = combined[0].0.date.clone();
    let first_title = combined[0].0.title.as_deref().unwrap_or("").to_string();
    for (parsed, _) in &combined {
        if parsed.date != first_date {
            return Err(RecordingError::MergeFailed(
                "all files must have the same date (YYYY-MM-DD)".to_string(),
            ));
        }
        if parsed.title.as_deref().unwrap_or("") != first_title {
            return Err(RecordingError::MergeFailed(
                "all files must have the same title".to_string(),
            ));
        }
    }

    let stream_title = if first_title.is_empty() {
        None
    } else {
        Some(first_title)
    };
    Ok((combined, stream_title))
}

/// Get expected completed MP4 filename from parsed recording filename.
pub(super) fn expected_completed_mp4_filename(
    channel_login: &str,
    parsed: &ParsedRecordingFilename,
    stream_title: Option<&str>,
    recordings_dir: &Path,
) -> String {
    let started_at_unix =
        parse_filename_timestamp_to_unix(&parsed.timestamp).unwrap_or_else(now_unix_secs);
    let quality = parsed.quality.clone();
    let mode = match parsed.mode.as_str() {
        "manual" => RecordingMode::Manual,
        "auto" => RecordingMode::Auto,
        _ => RecordingMode::Manual,
    };

    let completed_dir = recordings_dir.join("completed");
    let expected = build_completed_recording_path(
        &channel_bucket_dir(&completed_dir, channel_login),
        channel_login,
        &ActiveRecording {
            channel_login: channel_login.to_string(),
            quality,
            started_at_unix,
            output_path: String::new(),
            pid: None,
            mode,
            error: None,
        },
        stream_title,
    );
    expected
        .with_extension("mp4")
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("merged.mp4")
        .to_string()
}

/// Helper to get channel-specific directory within a bucket.
fn channel_bucket_dir(base: &Path, channel_login: &str) -> std::path::PathBuf {
    base.join(super::files::sanitize_filename(channel_login))
}
