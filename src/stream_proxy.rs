use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::{process::Command, sync::RwLock};

#[derive(Debug, Clone)]
pub struct StreamProxyState {
    pub service: StreamSessionService,
}

#[derive(Debug, Clone)]
pub struct StreamSessionService {
    sessions: Arc<RwLock<HashMap<String, StreamSession>>>,
    streamlink_path: String,
}

#[derive(Debug, Clone)]
pub struct QualityVariant {
    pub manifest_url: String,
    pub segment_lookup: HashMap<String, String>,
    pub cdn_base: String,
}

#[derive(Debug, Clone)]
pub struct StreamSession {
    pub session_token: String,
    pub variants: HashMap<String, QualityVariant>,
}

#[derive(Debug)]
pub enum StreamError {
    StreamNotFound,
    SessionMismatch,
    HlsFetchFailed(String),
}

impl StreamProxyState {
    pub fn new(service: StreamSessionService) -> Self {
        Self { service }
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
        let qualities_to_fetch = if quality == "best" {
            vec![
                "source", "1080p60", "720p60", "720p", "480p", "360p", "160p",
            ]
        } else {
            vec![quality, "720p", "480p"]
        };

        let mut variants = HashMap::new();

        for q in &qualities_to_fetch {
            match get_hls_url(channel, &self.streamlink_path, q).await {
                Ok(manifest_url) => match fetch_and_parse_manifest(&manifest_url).await {
                    Ok((lookup, cdn_base)) => {
                        let variant = QualityVariant {
                            manifest_url: manifest_url.clone(),
                            segment_lookup: lookup,
                            cdn_base,
                        };

                        variants.insert(q.to_string(), variant);
                    }
                    Err(e) => {
                        tracing::warn!(channel = %channel, quality = %q, error = %e, "failed to parse manifest for quality");
                    }
                },
                Err(e) => {
                    tracing::debug!(channel = %channel, quality = %q, error = %e, "quality not available");
                }
            }

            if variants.len() >= 4 {
                break;
            }
        }

        if variants.is_empty() {
            return Err(StreamError::HlsFetchFailed(
                "No qualities available for channel".to_string(),
            ));
        }

        let session = StreamSession {
            session_token: session_token.to_string(),
            variants,
        };

        tracing::info!(
            stream_id = %stream_id,
            channel = %channel,
            available_qualities = ?session.variants.keys().collect::<Vec<_>>(),
            "opened stream session"
        );

        let mut guard = self.sessions.write().await;
        guard.insert(stream_id.to_string(), session);
        Ok(())
    }

    pub async fn get_variant_manifest(
        &self,
        stream_id: &str,
        session_token: &str,
        quality: &str,
    ) -> Result<String, StreamError> {
        let session = self.get_session(stream_id, session_token).await?;

        let variant = session
            .variants
            .get(quality)
            .ok_or(StreamError::StreamNotFound)?;

        let manifest_text = fetch_text(&variant.manifest_url)
            .await
            .map_err(StreamError::HlsFetchFailed)?;

        let rewritten = rewrite_manifest_urls(&manifest_text, stream_id, session_token, quality);

        Ok(rewritten)
    }

    pub async fn get_multi_level_manifest(
        &self,
        stream_id: &str,
        session_token: &str,
    ) -> Result<String, StreamError> {
        let session = self.get_session(stream_id, session_token).await?;

        let mut manifest_lines = vec!["#EXTM3U".to_string(), "#EXT-X-VERSION:3".to_string()];

        for quality in session.variants.keys() {
            let (bandwidth, width, height) = quality_info(quality);
            let name = match quality.as_str() {
                "source" => "Auto",
                q => q,
            };

            manifest_lines.push(format!(
                "#EXT-X-STREAM-INF:BANDWIDTH={},RESOLUTION={}x{},NAME=\"{}\"",
                bandwidth, width, height, name
            ));
            manifest_lines.push(format!(
                "/stream/{}/{}/manifest/{}",
                stream_id, session_token, quality
            ));
        }

        Ok(manifest_lines.join("\n"))
    }

    pub async fn proxy_segment(
        &self,
        stream_id: &str,
        quality: &str,
        segment_name: &str,
        session_token: &str,
    ) -> Result<Vec<u8>, StreamError> {
        let session = self.get_session(stream_id, session_token).await?;

        let variant = session
            .variants
            .get(quality)
            .ok_or(StreamError::StreamNotFound)?;

        let cdn_url = if variant.cdn_base.is_empty() {
            variant
                .segment_lookup
                .get(segment_name)
                .cloned()
                .ok_or(StreamError::StreamNotFound)?
        } else {
            format!("{}/segment/{}", variant.cdn_base, segment_name)
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

async fn get_hls_url(
    channel: &str,
    streamlink_path: &str,
    quality: &str,
) -> Result<String, String> {
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
        tracing::debug!(status = ?output.status, stderr = %stderr, channel = %channel, quality = %quality, "streamlink quality not available");
        return Err(format!("streamlink exited with {}", output.status));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        return Err("streamlink returned empty URL".to_string());
    }

    tracing::debug!(url = %url, channel = %channel, quality = %quality, "streamlink returned HLS URL");
    Ok(url)
}

async fn fetch_and_parse_manifest(url: &str) -> Result<(HashMap<String, String>, String), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let text = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    let (lookup, cdn_base) = parse_segment_lookup(&text);

    Ok((lookup, cdn_base))
}

fn quality_info(quality: &str) -> (u32, u32, u32) {
    match quality {
        "source" => (8000000, 1920, 1080),
        "1080p60" => (6000000, 1920, 1080),
        "1080p" => (4500000, 1920, 1080),
        "720p60" => (3000000, 1280, 720),
        "720p" => (2000000, 1280, 720),
        "480p60" => (1500000, 854, 480),
        "480p" => (1000000, 854, 480),
        "360p" => (600000, 640, 360),
        "160p" => (300000, 284, 160),
        _ => (1500000, 1280, 720),
    }
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

fn rewrite_manifest_urls(
    manifest: &str,
    stream_id: &str,
    session_token: &str,
    quality: &str,
) -> String {
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
                format!(
                    "/stream/{}/{}/{}/{}",
                    stream_id, session_token, quality, segment_name
                )
            } else {
                let segment_name = line.split('?').next().unwrap_or(line);
                format!(
                    "/stream/{}/{}/{}/{}",
                    stream_id, session_token, quality, segment_name
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn proxy_manifest(
    State(state): State<StreamProxyState>,
    Path((stream_id, session_token)): Path<(String, String)>,
) -> Response {
    match state
        .service
        .get_multi_level_manifest(&stream_id, &session_token)
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

pub async fn proxy_variant_manifest(
    State(state): State<StreamProxyState>,
    Path((stream_id, session_token, quality)): Path<(String, String, String)>,
) -> Response {
    match state
        .service
        .get_variant_manifest(&stream_id, &session_token, &quality)
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
            error_response(StatusCode::NOT_FOUND, "quality not found")
        }
        Err(StreamError::SessionMismatch) => error_response(
            StatusCode::FORBIDDEN,
            "stream belongs to a different session",
        ),
        Err(StreamError::HlsFetchFailed(msg)) => {
            tracing::error!(error = %msg, stream_id = %stream_id, "failed to fetch variant manifest");
            error_response(StatusCode::BAD_GATEWAY, "failed to fetch variant manifest")
        }
    }
}

pub async fn proxy_segment(
    State(state): State<StreamProxyState>,
    Path((stream_id, session_token, quality, segment)): Path<(String, String, String, String)>,
) -> Response {
    match state
        .service
        .proxy_segment(&stream_id, &quality, &segment, &session_token)
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
            tracing::error!(error = %msg, stream_id = %stream_id, segment = %segment, "failed to fetch stream segment");
            error_response(StatusCode::BAD_GATEWAY, "failed to fetch stream segment")
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
}
