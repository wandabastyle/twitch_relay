use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event as SseEvent, KeepAlive, Sse},
    },
};
use futures_util::StreamExt;
use regex::Regex;
use serde::Serialize;
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::{invidious::is_valid_video_id, youtube::YoutubeState};

/// Single observation entry for a video quality stream
#[derive(Debug, Clone)]
pub struct QualityObservation {
    pub current_itag: String,
    pub current_quality_label: String,
    pub counts: HashMap<String, u64>,
    pub last_updated_unix_secs: u64,
}

/// Quality observation response payload
#[derive(Debug, Clone, Serialize)]
pub struct QualityObservedResponse {
    pub current_quality_label: Option<String>,
    pub seen_itags: Vec<QualityObservedItag>,
    pub last_updated_unix_secs: Option<u64>,
}

/// Single itag observation count
#[derive(Debug, Clone, Serialize)]
pub struct QualityObservedItag {
    pub quality_label: String,
    pub count: u64,
}

/// Get current quality observation for a video
pub async fn get_video_quality_observed(
    State(state): State<YoutubeState>,
    Path(video_id): Path<String>,
) -> Response {
    if !is_valid_video_id(&video_id) {
        return (StatusCode::BAD_REQUEST, "invalid video_id format").into_response();
    }

    let observations = match state.quality_observations.lock() {
        Ok(o) => o,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "quality observation store unavailable",
            )
                .into_response();
        }
    };

    let Some(obs) = observations.get(&video_id) else {
        return (
            StatusCode::OK,
            Json(QualityObservedResponse {
                current_quality_label: None,
                seen_itags: Vec::new(),
                last_updated_unix_secs: None,
            }),
        )
            .into_response();
    };

    (StatusCode::OK, Json(build_quality_observed_response(obs))).into_response()
}

/// SSE stream for quality observations
pub async fn get_video_quality_stream(
    State(state): State<YoutubeState>,
    Path(video_id): Path<String>,
) -> Response {
    if !is_valid_video_id(&video_id) {
        return (StatusCode::BAD_REQUEST, "invalid video_id format").into_response();
    }

    let sender = get_or_create_quality_sender(&state, &video_id);
    let receiver = sender.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|msg| async move {
        match msg {
            Ok(update) => serde_json::to_string(&update).ok().map(|json| {
                Ok::<SseEvent, std::convert::Infallible>(SseEvent::default().data(json))
            }),
            Err(_) => None,
        }
    });

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(25))
                .text("hb"),
        )
        .into_response()
}

/// Build response from observation state
fn build_quality_observed_response(obs: &QualityObservation) -> QualityObservedResponse {
    let mut seen_itags: Vec<QualityObservedItag> = obs
        .counts
        .iter()
        .map(|(itag, count)| QualityObservedItag {
            quality_label: itag_quality_label(itag).to_string(),
            count: *count,
        })
        .collect();
    seen_itags.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.quality_label.cmp(&b.quality_label))
    });

    QualityObservedResponse {
        current_quality_label: Some(obs.current_quality_label.clone()),
        seen_itags,
        last_updated_unix_secs: Some(obs.last_updated_unix_secs),
    }
}

/// Check if itag is a video stream (not audio)
pub fn is_video_itag(itag: &str) -> bool {
    matches!(
        itag,
        "160"
            | "278"
            | "394"
            | "133"
            | "242"
            | "395"
            | "134"
            | "243"
            | "396"
            | "18"
            | "135"
            | "244"
            | "397"
            | "136"
            | "247"
            | "398"
            | "22"
            | "298"
            | "137"
            | "248"
            | "399"
            | "299"
    )
}

/// Get or create broadcast sender for a video's quality stream
fn get_or_create_quality_sender(
    state: &YoutubeState,
    video_id: &str,
) -> broadcast::Sender<QualityObservedResponse> {
    let mut streams = match state.quality_streams.lock() {
        Ok(s) => s,
        Err(poisoned) => poisoned.into_inner(),
    };

    streams
        .entry(video_id.to_string())
        .or_insert_with(|| {
            let (tx, _) = broadcast::channel(128);
            tx
        })
        .clone()
}

/// Current unix timestamp
fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Decode URL-encoded query value
fn decode_query_value(value: &str) -> Option<String> {
    urlencoding::decode(value).ok().map(|v| v.into_owned())
}

/// Extract query parameter from raw query string
fn query_param(raw_query: Option<&str>, key: &str) -> Option<String> {
    let query = raw_query?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let k = parts.next().unwrap_or_default();
        if k != key {
            continue;
        }
        let value = parts.next().unwrap_or_default();
        return decode_query_value(value).or_else(|| Some(value.to_string()));
    }
    None
}

/// Extract video ID from embed referer header
fn video_id_from_embed_referer(request_headers: &axum::http::HeaderMap) -> Option<String> {
    use axum::http::header;
    use std::sync::OnceLock;

    static EMBED_REFERER_VIDEO_ID_RE: OnceLock<Regex> = OnceLock::new();

    let referer = request_headers.get(header::REFERER)?.to_str().ok()?;
    let re = EMBED_REFERER_VIDEO_ID_RE.get_or_init(|| {
        Regex::new(r"/api/youtube/embed/([A-Za-z0-9_-]{11})(?:[/?#]|$)")
            .expect("valid embed referer video_id regex")
    });
    let captures = re.captures(referer)?;
    let video_id = captures.get(1)?.as_str();
    if is_valid_video_id(video_id) {
        Some(video_id.to_string())
    } else {
        None
    }
}

/// Map itag to human-readable quality label
fn itag_quality_label(itag: &str) -> &'static str {
    match itag {
        "137" => "1080p (avc1)",
        "248" => "1080p (vp9)",
        "399" => "1080p (av1)",
        "299" => "1080p60 (avc1)",
        "136" => "720p (avc1)",
        "247" => "720p (vp9)",
        "398" => "720p (av1)",
        "22" => "720p (avc1)",
        "298" => "720p60",
        "135" => "480p (avc1)",
        "244" => "480p (vp9)",
        "397" => "480p (av1)",
        "18" => "360p (avc1)",
        "134" => "360p (avc1)",
        "243" => "360p (vp9)",
        "396" => "360p (av1)",
        "133" => "240p (avc1)",
        "242" => "240p (vp9)",
        "395" => "240p (av1)",
        "160" => "144p (avc1)",
        "278" => "144p (vp9)",
        "394" => "144p (av1)",
        _ => "unknown",
    }
}

/// Observe quality from video segment request
pub fn observe_quality_from_request(
    state: &YoutubeState,
    raw_query: Option<&str>,
    request_headers: &axum::http::HeaderMap,
) {
    let Some(itag) = query_param(raw_query, "itag") else {
        return;
    };
    if !is_video_itag(&itag) {
        return;
    }
    let video_id = if let Some(id) = video_id_from_embed_referer(request_headers) {
        id
    } else if let Some(raw_video_id) = query_param(raw_query, "id") {
        let base = raw_video_id.split('.').next().unwrap_or(&raw_video_id);
        if is_valid_video_id(base) {
            base.to_string()
        } else {
            return;
        }
    } else {
        return;
    };

    let mut observations = match state.quality_observations.lock() {
        Ok(o) => o,
        Err(_) => return,
    };

    let now = now_unix_secs();
    observations.retain(|_, v| now.saturating_sub(v.last_updated_unix_secs) <= 3600);

    let quality_label = itag_quality_label(&itag).to_string();
    let obs = observations
        .entry(video_id.clone())
        .or_insert_with(|| QualityObservation {
            current_itag: itag.clone(),
            current_quality_label: quality_label.clone(),
            counts: HashMap::new(),
            last_updated_unix_secs: now,
        });

    obs.current_itag = itag.clone();
    obs.current_quality_label = quality_label;
    obs.last_updated_unix_secs = now;
    *obs.counts.entry(itag).or_insert(0) += 1;

    let snapshot = build_quality_observed_response(obs);
    drop(observations);

    let sender = get_or_create_quality_sender(state, &video_id);
    let _ = sender.send(snapshot);
}
