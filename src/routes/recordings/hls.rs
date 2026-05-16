use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
};

use crate::routes::error::error_response;

use super::{RecordingState, types::ServeHlsPlaylistQuery};

pub async fn serve_hls_playlist(
    State(state): State<RecordingState>,
    Query(query): Query<ServeHlsPlaylistQuery>,
) -> Response {
    // Resolve the MP4 path
    let mp4_path = match state
        .service
        .resolve_completed_file_path(&query.channel_login, &query.filename)
    {
        Ok(path) => path,
        Err(error) => {
            let (status, message) = super::classify_recording_error(&error);
            return error_response(status, message, None);
        }
    };

    if !mp4_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "recording not found", None);
    }

    // Look for the .m3u8 playlist file
    let playlist_path = mp4_path.with_extension("m3u8");
    if !playlist_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "hls playlist not found", None);
    }

    // Read and serve the playlist
    let playlist_content = match tokio::fs::read_to_string(&playlist_path).await {
        Ok(content) => content,
        Err(error) => {
            tracing::error!(error = %error, path = %playlist_path.display(), "failed to read hls playlist");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read playlist",
                None,
            );
        }
    };

    let mut response = Response::new(Body::from(playlist_content));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.apple.mpegurl"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, must-revalidate"),
    );
    response
}
