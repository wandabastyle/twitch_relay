use std::net::SocketAddr;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::auth::WebAuthConfig;

#[derive(Debug, Clone)]
pub struct StreamProxyState {
    pub auth: WebAuthConfig,
    pub relay: crate::relay::RelayService,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SegmentPath {
    pub stream_id: String,
    pub segment: String,
}

pub async fn proxy_manifest(
    State(state): State<StreamProxyState>,
    Path(stream_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    let stream = match state.relay.validate(&stream_id, &session_token).await {
        Ok(s) => s,
        Err(crate::relay::RelayError::StreamNotFound) => {
            return error_response(StatusCode::NOT_FOUND, "stream not found or has ended");
        }
        Err(crate::relay::RelayError::SessionMismatch) => {
            return error_response(
                StatusCode::FORBIDDEN,
                "stream belongs to a different session",
            );
        }
        Err(_) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "stream error");
        }
    };

    let manifest = fetch_from_streamlit(&stream.channel, stream.port, "/stream.m3u8").await;

    match manifest {
        Ok(body) => {
            let rewritten = rewrite_manifest_urls(&body, &stream_id, stream.port);
            (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("application/vnd.apple.mpegurl"),
                )],
                rewritten,
            )
                .into_response()
        }
        Err(_) => error_response(StatusCode::BAD_GATEWAY, "failed to fetch stream manifest"),
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

    let stream = match state.relay.validate(&stream_id, &session_token).await {
        Ok(s) => s,
        Err(crate::relay::RelayError::StreamNotFound) => {
            return error_response(StatusCode::NOT_FOUND, "stream not found or has ended");
        }
        Err(crate::relay::RelayError::SessionMismatch) => {
            return error_response(
                StatusCode::FORBIDDEN,
                "stream belongs to a different session",
            );
        }
        Err(_) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "stream error");
        }
    };

    let segment_path = format!("/segment/{segment}");
    match fetch_from_streamlit(&stream.channel, stream.port, &segment_path).await {
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
        Err(_) => error_response(StatusCode::BAD_GATEWAY, "failed to fetch stream segment"),
    }
}

fn rewrite_manifest_urls(manifest: &str, stream_id: &str, _port: u16) -> String {
    let _base = format!("http://127.0.0.1:{_port}");
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

async fn fetch_from_streamlit(_channel: &str, port: u16, path: &str) -> Result<String, ()> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().map_err(|_| ())?;

    let mut stream = TcpStream::connect(addr).await.map_err(|_| ())?;
    let request = format!(
        "GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n",
        path = path
    );

    stream.write_all(request.as_bytes()).await.map_err(|_| ())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.map_err(|_| ())?;

    let body = extract_http_body(&buf).ok_or(())?;
    Ok(String::from_utf8_lossy(body).to_string())
}

fn extract_http_body(buf: &[u8]) -> Option<&[u8]> {
    let s = String::from_utf8_lossy(buf);
    if let Some(pos) = s.find("\r\n\r\n") {
        Some(&buf[pos + 4..])
    } else {
        None
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
}
