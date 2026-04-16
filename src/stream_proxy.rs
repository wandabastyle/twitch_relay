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
    pub cdn_base: String,
    pub quality: String,
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
        quality: &str,
    ) -> Result<(), StreamError> {
        let (manifest_url, resolved_quality) = get_hls_url_with_fallback(channel, &self.streamlink_path, quality)
            .await
            .map_err(StreamError::HlsFetchFailed)?;

        let manifest_text = fetch_text(&manifest_url)
            .await
            .map_err(StreamError::HlsFetchFailed)?;

        let (segment_lookup, cdn_base) = parse_segment_lookup(&manifest_text);

        let manifest_preview = if manifest_text.len() > 500 {
            format!("{}...", &manifest_text[..500])
        } else {
            manifest_text.clone()
        };
        
        tracing::info!(
            stream_id = %stream_id,
            channel = %channel,
            requested_quality = %quality,
            resolved_quality = %resolved_quality,
            cdn_base = %cdn_base,
            initial_segment_count = %segment_lookup.len(),
            manifest_preview = %manifest_preview,
            "opened stream session"
        );

        let session = StreamSession {
            session_token: session_token.to_string(),
            manifest_url,
            segment_lookup,
            cdn_base,
            quality: resolved_quality,
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

        let rewritten = rewrite_manifest_urls(&manifest_text, stream_id, session_token);
        
        tracing::debug!(
            stream_id = %stream_id,
            quality = %session.quality,
            manifest_size = %manifest_text.len(),
            rewritten_size = %rewritten.len(),
            "serving manifest"
        );

        Ok(rewritten)
    }

    pub async fn proxy_segment(
        &self,
        stream_id: &str,
        segment_name: &str,
        session_token: &str,
    ) -> Result<Vec<u8>, StreamError> {
        let session = self.get_session(stream_id, session_token).await?;

        let cdn_url = if session.cdn_base.is_empty() {
            session
                .segment_lookup
                .get(segment_name)
                .cloned()
                .ok_or(StreamError::StreamNotFound)?
        } else {
            format!("{}/segment/{}", session.cdn_base, segment_name)
        };

        fetch_bytes(&cdn_url)
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

async fn get_hls_url(channel: &str, streamlink_path: &str, quality: &str) -> Result<String, String> {
    let output = Command::new(streamlink_path)
        .args([
            &format!("https://twitch.tv/{channel}"),
            quality,
            "--stream-url",
        ])
        .output()
        .await
        .map_err(|e| format!("streamlink spawn failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!(status = ?output.status, stderr = %stderr, channel = %channel, quality = %quality, "streamlink failed");
        return Err(format!(
            "streamlink exited with {}: {}",
            output.status, stderr
        ));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        tracing::error!(channel = %channel, quality = %quality, "streamlink returned empty URL");
        return Err("streamlink returned empty URL".to_string());
    }

    tracing::info!(url = %url, channel = %channel, quality = %quality, "streamlink returned HLS URL");
    Ok(url)
}

pub async fn get_hls_url_with_fallback(
    channel: &str,
    streamlink_path: &str,
    requested_quality: &str,
) -> Result<(String, String), String> {
    let qualities: Vec<&str> = if requested_quality == "auto" {
        vec!["1080p60", "720p"]
    } else {
        vec![requested_quality]
    };

    let quality_names: Vec<String> = qualities.iter().map(|s| s.to_string()).collect();

    for quality in &qualities {
        tracing::info!(channel = %channel, quality = %quality, "trying quality");
        match get_hls_url(channel, streamlink_path, quality).await {
            Ok(url) => return Ok((url, quality.to_string())),
            Err(e) => {
                tracing::warn!(channel = %channel, quality = %quality, error = %e, "quality failed, trying next");
            }
        }
    }

    Err(format!(
        "Failed to get HLS URL for channel {} with qualities {:?}",
        channel, quality_names
    ))
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))
        .map(|b| b.to_vec())
}

async fn fetch_text(url: &str) -> Result<String, String> {
    let bytes = fetch_bytes(url).await?;
    String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {}", e))
}

fn parse_segment_lookup(manifest: &str) -> (HashMap<String, String>, String) {
    let mut cdn_base = String::new();
    let lookup: HashMap<String, String> = manifest
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
                if cdn_base.is_empty() {
                    if let Some(segment_idx) = url.find("/segment/") {
                        cdn_base = url[..segment_idx].to_string();
                    } else if let Some(vod_idx) = url.find("/vod/") {
                        cdn_base = url[..vod_idx].to_string();
                    }
                }
                Some((name, url.to_string()))
            } else {
                None
            }
        })
        .collect();
    (lookup, cdn_base)
}

fn rewrite_manifest_urls(manifest: &str, stream_id: &str, session_token: &str) -> String {
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
                format!("/stream/{stream_id}/{session_token}/segment/{segment_name}")
            } else {
                let segment_name = line.split('?').next().unwrap_or(line);
                format!("/stream/{stream_id}/{session_token}/segment/{segment_name}")
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
            [
                (
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("application/vnd.apple.mpegurl"),
                ),
                (
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
                ),
            ],
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
    Path((stream_id, session_token, segment)): Path<(String, String, String)>,
) -> Response {
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
                [
                    (
                        axum::http::header::CONTENT_TYPE,
                        axum::http::HeaderValue::from_str(ct).unwrap_or_else(|_| {
                            axum::http::HeaderValue::from_static("application/octet-stream")
                        }),
                    ),
                    (
                        axum::http::header::CACHE_CONTROL,
                        axum::http::HeaderValue::from_static("public, max-age=3600"),
                    ),
                    (
                        axum::http::header::ACCEPT_RANGES,
                        axum::http::HeaderValue::from_static("bytes"),
                    ),
                ],
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
