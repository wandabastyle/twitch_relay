use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
};
use futures_util::stream;

use crate::routes::error::error_response;

use super::{RecordingState, types::PlayRecordingAssetQuery};

pub async fn play_recording_asset(
    State(state): State<RecordingState>,
    Query(query): Query<PlayRecordingAssetQuery>,
    headers: HeaderMap,
) -> Response {
    let media_path = match state
        .service
        .resolve_completed_file_path(&query.channel_login, &query.filename)
    {
        Ok(path) => path,
        Err(error) => {
            let (status, message) = super::classify_recording_error(&error);
            return error_response(status, message, None);
        }
    };

    if !media_path.exists() {
        return error_response(
            StatusCode::NOT_FOUND,
            "recording playback asset not found",
            None,
        );
    }

    let file_size = match tokio::fs::metadata(&media_path).await {
        Ok(meta) => meta.len(),
        Err(error) => {
            tracing::error!(error = %error, path = %media_path.display(), "failed to read playback media metadata");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "recording playback failed",
                None,
            );
        }
    };

    if let Some(range_header) = headers.get(header::RANGE) {
        let Ok(range_str) = range_header.to_str() else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header", None);
        };
        let Some(range_spec) = range_str.strip_prefix("bytes=") else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header", None);
        };
        let Some((start_str, end_str)) = range_spec.split_once('-') else {
            return error_response(StatusCode::BAD_REQUEST, "invalid range header", None);
        };

        let start: u64 = match start_str.parse() {
            Ok(v) => v,
            Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid range start", None),
        };

        // HLS uses exact byte ranges from the m3u8 - open-ended ranges not supported
        let end: u64 = if end_str.is_empty() {
            return error_response(
                StatusCode::BAD_REQUEST,
                "open-ended ranges not supported",
                None,
            );
        } else {
            match end_str.parse() {
                Ok(v) => v,
                Err(_) => {
                    return error_response(StatusCode::BAD_REQUEST, "invalid range end", None);
                }
            }
        };

        if start >= file_size || end >= file_size || end < start {
            return error_response(
                StatusCode::RANGE_NOT_SATISFIABLE,
                "range not satisfiable",
                None,
            );
        }

        let length = end - start + 1;
        let media_stream = match stream_file_range(&media_path, start, length).await {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(error = %error, path = %media_path.display(), "failed to read playback range");
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "recording playback failed",
                    None,
                );
            }
        };

        let content_range = format!("bytes {start}-{end}/{file_size}");
        let mut response = Response::new(Body::from_stream(media_stream));
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("video/mp4"));
        response.headers_mut().insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&content_range).unwrap(),
        );
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&length.to_string()).unwrap(),
        );
        response
            .headers_mut()
            .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=2592000, immutable"),
        );
        response
    } else {
        let media_stream = match stream_file_range(&media_path, 0, file_size).await {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(error = %error, path = %media_path.display(), "failed to read playback media");
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "recording playback failed",
                    None,
                );
            }
        };
        let mut response = Response::new(Body::from_stream(media_stream));
        *response.status_mut() = StatusCode::OK;
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("video/mp4"));
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&file_size.to_string()).unwrap(),
        );
        response
            .headers_mut()
            .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=2592000, immutable"),
        );
        response
    }
}

async fn stream_file_range(
    path: &std::path::Path,
    start: u64,
    length: u64,
) -> Result<
    std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<axum::body::Bytes, std::io::Error>> + Send>,
    >,
    String,
> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| format!("failed to open playback media: {error}"))?;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|error| format!("failed to seek playback media: {error}"))?;
    let stream = stream::try_unfold((file, length), |(mut file, remaining)| async move {
        const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4 MiB chunks for HDD optimization

        if remaining == 0 {
            return Ok(None);
        }

        let next_len = usize::try_from(remaining.min(CHUNK_SIZE as u64)).unwrap_or(CHUNK_SIZE);
        let mut chunk = vec![0_u8; next_len];
        let read = file.read(&mut chunk).await?;
        if read == 0 {
            return Ok(None);
        }
        chunk.truncate(read);
        let read_u64 = u64::try_from(read).unwrap_or(0);
        let next_remaining = remaining.saturating_sub(read_u64);
        Ok(Some((
            axum::body::Bytes::from(chunk),
            (file, next_remaining),
        )))
    });

    Ok(Box::pin(stream))
}
