use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use tokio::{process::Command, sync::RwLock};

use crate::auth::WebAuthConfig;

#[derive(Debug, Clone)]
pub struct StreamProxyState {
    pub auth: WebAuthConfig,
    pub service: StreamSessionService,
}

#[derive(Debug, Clone)]
pub struct StreamSessionService {
    sessions: Arc<RwLock<HashMap<String, StreamSession>>>,
    streamlink_path: String,
}

#[derive(Debug, Clone)]
pub struct StreamSession {
    pub session_token: String,
    pub manifest_url: String,
    pub segment_lookup: HashMap<String, String>,
}

#[derive(Debug)]
pub enum StreamError {
    StreamNotFound,
    SessionMismatch,
    HlsFetchFailed(String),
}

impl StreamProxyState {
    pub fn new(auth: WebAuthConfig, service: StreamSessionService) -> Self {
        Self { auth, service }
    }
}

impl StreamSessionService {
    pub fn new(streamlink_path: String) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            streamlink_path,
        }
    }

    pub async fn open_session(
        &self,
        stream_id: &str,
        channel: &str,
        session_token: &str,
    ) -> Result<(), StreamError> {
        let manifest_url = get_hls_url(channel, &self.streamlink_path)
            .await
            .map_err(StreamError::HlsFetchFailed)?;

        let manifest_text = fetch_text(&manifest_url)
            .await
            .map_err(StreamError::HlsFetchFailed)?;

        let segment_lookup = parse_segment_lookup(&manifest_text);

        let session = StreamSession {
            session_token: session_token.to_string(),
            manifest_url,
            segment_lookup,
        };

        let mut guard = self.sessions.write().await;
        guard.insert(stream_id.to_string(), session);
        Ok(())
    }

    pub async fn proxy_manifest(
        &self,
        stream_id: &str,
        session_token: &str,
    ) -> Result<String, StreamError> {
        let session = self.get_session(stream_id, session_token).await?;

        let manifest_text = fetch_text(&session.manifest_url)
            .await
            .map_err(StreamError::HlsFetchFailed)?;

        Ok(rewrite_manifest_urls(&manifest_text, stream_id))
    }

    pub async fn proxy_segment(
        &self,
        stream_id: &str,
        segment_name: &str,
        session_token: &str,
    ) -> Result<String, StreamError> {
        let session = self.get_session(stream_id, session_token).await?;

        let Some(cdn_url) = session.segment_lookup.get(segment_name) else {
            return Err(StreamError::StreamNotFound);
        };

        fetch_text(cdn_url)
            .await
            .map_err(StreamError::HlsFetchFailed)
    }

    async fn get_session(
        &self,
        stream_id: &str,
        session_token: &str,
    ) -> Result<StreamSession, StreamError> {
        let guard = self.sessions.read().await;
        let Some(session) = guard.get(stream_id) else {
            return Err(StreamError::StreamNotFound);
        };

        if session.session_token != session_token {
            return Err(StreamError::SessionMismatch);
        }

        Ok(session.clone())
    }
}

async fn get_hls_url(channel: &str, streamlink_path: &str) -> Result<String, String> {
    let output = Command::new(streamlink_path)
        .args([
            &format!("https://twitch.tv/{channel}"),
            "best",
            "--stream-url",
        ])
        .output()
        .await
        .map_err(|e| format!("streamlink spawn failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "streamlink exited with {}: {}",
            output.status, stderr
        ));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        return Err("streamlink returned empty URL".to_string());
    }

    Ok(url)
}

async fn fetch_text(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))
}

fn parse_segment_lookup(manifest: &str) -> HashMap<String, String> {
    manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .filter_map(|line| {
            let url = line.trim();
            if url.starts_with("http://") || url.starts_with("https://") {
                let name = url
                    .rsplit('/')
                    .next()
                    .unwrap_or(url)
                    .split('?')
                    .next()
                    .unwrap_or(url)
                    .to_string();
                Some((name, url.to_string()))
            } else {
                None
            }
        })
        .collect()
}

fn rewrite_manifest_urls(manifest: &str, stream_id: &str) -> String {
    manifest
        .lines()
        .map(|line| {
            if line.starts_with('#') || line.is_empty() {
                line.to_string()
            } else if line.starts_with("http://") || line.starts_with("https://") {
                let segment_name = line
                    .rsplit('/')
                    .next()
                    .unwrap_or(line)
                    .split('?')
                    .next()
                    .unwrap_or(line);
                format!("/stream/{stream_id}/segment/{segment_name}")
            } else {
                let segment_name = line.split('?').next().unwrap_or(line);
                format!("/stream/{stream_id}/segment/{segment_name}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn proxy_manifest(
    State(state): State<StreamProxyState>,
    Path(stream_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    match state
        .service
        .proxy_manifest(&stream_id, &session_token)
        .await
    {
        Ok(body) => (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/vnd.apple.mpegurl"),
            )],
            body,
        )
            .into_response(),
        Err(StreamError::StreamNotFound) => {
            error_response(StatusCode::NOT_FOUND, "stream not found or has ended")
        }
        Err(StreamError::SessionMismatch) => error_response(
            StatusCode::FORBIDDEN,
            "stream belongs to a different session",
        ),
        Err(StreamError::HlsFetchFailed(msg)) => {
            tracing::error!(error = %msg, stream_id = %stream_id, "failed to fetch HLS manifest");
            error_response(StatusCode::BAD_GATEWAY, "failed to fetch stream manifest")
        }
    }
}

pub async fn proxy_segment(
    State(state): State<StreamProxyState>,
    Path((stream_id, segment)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    match state
        .service
        .proxy_segment(&stream_id, &segment, &session_token)
        .await
    {
        Ok(body) => {
            let ct = if segment.ends_with(".ts") || segment.contains(".ts?") {
                "video/mp2t"
            } else if segment.ends_with(".m4s") {
                "video/mp4"
            } else {
                "application/octet-stream"
            };

            (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_str(ct).unwrap_or_else(|_| {
                        axum::http::HeaderValue::from_static("application/octet-stream")
                    }),
                )],
                body,
            )
                .into_response()
        }
        Err(StreamError::StreamNotFound) => {
            error_response(StatusCode::NOT_FOUND, "segment not found")
        }
        Err(StreamError::SessionMismatch) => error_response(
            StatusCode::FORBIDDEN,
            "stream belongs to a different session",
        ),
        Err(StreamError::HlsFetchFailed(msg)) => {
            tracing::error!(error = %msg, stream_id = %stream_id, segment = %segment, "failed to fetch segment");
            error_response(StatusCode::BAD_GATEWAY, "failed to fetch stream segment")
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
}
