use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::InvidiousConfig;
use crate::error::AppError;
use crate::youtube_channels;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const AVATAR_CACHE_TTL_SECS: u64 = 86400; // 24 hours

fn extract_expiration_from_url(url: &str) -> Option<i64> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key == "expire" {
            value.parse::<i64>().ok()
        } else {
            None
        }
    })
}

/// Invidious API client for fetching YouTube data
#[derive(Debug, Clone)]
pub struct InvidiousClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
    avatar_cache: Arc<RwLock<HashMap<String, (String, Instant)>>>, // channel_id -> (url, fetched_at)
}

/// Cached avatar URL with timestamp
struct CachedAvatar {
    url: String,
    fetched_at: Instant,
}

impl CachedAvatar {
    fn is_valid(&self) -> bool {
        self.fetched_at.elapsed().as_secs() < AVATAR_CACHE_TTL_SECS
    }
}

/// Normalized YouTube channel from Invidious subscriptions
#[derive(Debug, Clone, Serialize)]
pub struct YoutubeChannel {
    pub name: String,
    pub channel_id: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// Normalized YouTube video from Invidious
#[derive(Debug, Clone, Serialize)]
pub struct YoutubeVideo {
    pub title: String,
    pub video_id: String,
    pub author: String,
    pub author_id: String,
    pub published: i64,
    pub published_text: String,
    pub duration: i64,
    pub thumbnail: String,
    pub view_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Resolved video stream info
#[derive(Debug, Clone, Serialize)]
pub struct VideoStream {
    pub title: String,
    pub duration: i64,
    pub stream_url: String,
    pub mime_type: String,
    pub is_hls: bool,
    pub expires_at: Option<i64>,
}

/// Raw Invidious subscription response
#[derive(Debug, Deserialize)]
struct InvidiousSubscription {
    author: String,
    #[serde(rename = "authorId")]
    author_id: String,
}

/// Raw Invidious channel details (for avatar fetching)
#[derive(Debug, Deserialize)]
struct InvidiousChannelDetails {
    author: String,
    #[serde(rename = "authorId")]
    author_id: String,
    #[serde(rename = "authorThumbnails")]
    author_thumbnails: Option<Vec<Thumbnail>>,
}

/// Raw Invidious video response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvidiousVideoRaw {
    title: String,
    video_id: String,
    author: String,
    author_id: String,
    #[serde(default)]
    published: i64,
    #[serde(default)]
    published_text: String,
    #[serde(default)]
    length_seconds: i64,
    #[serde(default)]
    video_thumbnails: Vec<Thumbnail>,
    #[serde(default)]
    view_count: i64,
    #[serde(default)]
    description: String,
    #[serde(default)]
    description_html: String,
}

/// Raw Invidious video details (for resolution)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvidiousVideoDetails {
    title: String,
    video_id: String,
    #[serde(default)]
    length_seconds: i64,
    #[serde(default)]
    format_streams: Vec<FormatStream>,
    #[serde(default)]
    adaptive_formats: Vec<AdaptiveFormat>,
    #[serde(default)]
    hls_url: Option<String>,
    #[serde(default)]
    dash_manifest: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Thumbnail {
    url: String,
    width: Option<i32>,
    height: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct FormatStream {
    #[serde(rename = "itag")]
    itag: Option<String>,
    url: String,
    #[serde(default, rename = "type")]
    mime_type: String,
    #[serde(default)]
    quality_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdaptiveFormat {
    #[serde(rename = "itag")]
    itag: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default, rename = "type")]
    mime_type: String,
    #[serde(default)]
    quality_label: Option<String>,
    #[serde(default)]
    audio_quality: Option<String>,
    #[serde(default)]
    projection_type: Option<String>,
}

impl InvidiousClient {
    /// Create a new Invidious client from config
    pub fn new(config: &InvidiousConfig) -> Self {
        let mut headers = HeaderMap::new();
        if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", config.token)) {
            headers.insert(AUTHORIZATION, value);
        }

        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .default_headers(headers)
            .build()
            .expect("failed to build reqwest client");

        Self {
            base_url: config.base_url.clone(),
            token: config.token.clone(),
            http,
            avatar_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if Invidious is configured
    pub fn is_configured(&self) -> bool {
        !self.base_url.is_empty() && !self.token.is_empty()
    }

    /// Get authenticated user's subscriptions
    pub async fn get_subscriptions(&self) -> Result<Vec<YoutubeChannel>, AppError> {
        let url = format!("{}/api/v1/auth/subscriptions", self.base_url);

        let response = self.http.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                AppError::InvidiousUnreachable
            } else if e.is_connect() {
                AppError::InvidiousUnreachable
            } else {
                AppError::Http(e)
            }
        })?;

        match response.status() {
            status if status.is_success() => {
                let subscriptions: Vec<InvidiousSubscription> = response
                    .json()
                    .await
                    .map_err(|_| AppError::InvidiousBadResponse)?;

                // Eagerly fetch channel details and avatars in parallel
                let channel_futures: Vec<_> = subscriptions
                    .iter()
                    .map(|sub| self.fetch_channel_avatar(&sub.author_id))
                    .collect();

                let avatar_results = futures_util::future::join_all(channel_futures).await;

                let mut channels: Vec<YoutubeChannel> = subscriptions
                    .into_iter()
                    .zip(avatar_results)
                    .map(|(sub, avatar_result)| {
                        // Use cached avatar URL or fallback to external URL
                        let avatar = match avatar_result {
                            Ok(url) => Some(url),
                            Err(_) => {
                                // Check if we have a cached image on disk
                                youtube_channels::get_channel_image_url(&sub.author_id)
                            }
                        };

                        YoutubeChannel {
                            name: sub.author,
                            channel_id: sub.author_id.clone(),
                            url: format!("/channel/{}", sub.author_id),
                            avatar,
                        }
                    })
                    .collect();

                // Sort alphabetically by name
                channels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                Ok(channels)
            }
            status if status.as_u16() == 401 => Err(AppError::InvidiousAuthFailed),
            status if status.as_u16() == 429 => Err(AppError::InvidiousRateLimited),
            _ => Err(AppError::InvidiousBadResponse),
        }
    }

    /// Get latest videos for a channel
    pub async fn get_channel_videos(
        &self,
        channel_id: &str,
        max_results: Option<u32>,
    ) -> Result<Vec<YoutubeVideo>, AppError> {
        // Validate channel_id format (should start with UC and be 24 chars)
        if !is_valid_channel_id(channel_id) {
            return Err(AppError::Config(format!(
                "invalid channel_id: {}",
                channel_id
            )));
        }

        let max = max_results.unwrap_or(20).min(40);
        let url = format!(
            "{}/api/v1/channels/{}/videos?max_results={}",
            self.base_url, channel_id, max
        );

        let response = self.http.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                AppError::InvidiousUnreachable
            } else if e.is_connect() {
                AppError::InvidiousUnreachable
            } else {
                AppError::Http(e)
            }
        })?;

        if !response.status().is_success() {
            return Err(AppError::InvidiousBadResponse);
        }

        // The response is a channel object with a "videos" field
        #[derive(Debug, Deserialize)]
        struct ChannelVideosResponse {
            videos: Vec<InvidiousVideoRaw>,
        }

        let channel_data: ChannelVideosResponse = response
            .json()
            .await
            .map_err(|_| AppError::InvidiousBadResponse)?;

        let videos: Vec<YoutubeVideo> = channel_data
            .videos
            .into_iter()
            .map(|v| {
                // Use hqdefault (480x360) - high quality but more reliably available
                // Invidious automatically falls back to lower qualities if not available
                let thumbnail = format!("{}/vi/{}/hqdefault.jpg", self.base_url, v.video_id);

                YoutubeVideo {
                    title: v.title,
                    video_id: v.video_id.clone(),
                    author: v.author,
                    author_id: v.author_id,
                    published: v.published,
                    published_text: v.published_text,
                    duration: v.length_seconds,
                    thumbnail,
                    view_count: v.view_count,
                    description: Some(v.description).filter(|d| !d.is_empty()),
                }
            })
            .collect();

        Ok(videos)
    }

    /// Resolve a YouTube video to a playable stream
    pub async fn resolve_video(&self, video_id: &str) -> Result<VideoStream, AppError> {
        let url = format!("{}/api/v1/videos/{}", self.base_url, video_id);

        let response = self.http.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                AppError::InvidiousUnreachable
            } else if e.is_connect() {
                AppError::InvidiousUnreachable
            } else {
                AppError::Http(e)
            }
        })?;

        if !response.status().is_success() {
            if response.status().as_u16() == 404 {
                return Err(AppError::ResolveFailed);
            }
            return Err(AppError::InvidiousBadResponse);
        }

        let details: InvidiousVideoDetails = response
            .json()
            .await
            .map_err(|_| AppError::InvidiousBadResponse)?;

        // Priority: HLS > DASH HLS manifest > Combined MP4 stream
        // 1. Check for HLS URL
        if let Some(hls_url) = details.hls_url {
            if !hls_url.is_empty() {
                let expires_at = extract_expiration_from_url(&hls_url);
                return Ok(VideoStream {
                    title: details.title,
                    duration: details.length_seconds,
                    stream_url: hls_url,
                    mime_type: "application/vnd.apple.mpegurl".to_string(),
                    is_hls: true,
                    expires_at,
                });
            }
        }

        // 2. Check for DASH manifest (can be proxied as HLS)
        if let Some(dash_url) = details.dash_manifest {
            if !dash_url.is_empty() {
                // DASH manifest URL - browser can't play directly but we'll try to find an HLS variant
                // For now, fall through to find a combined stream
                tracing::debug!(dash_url = %dash_url, "found DASH manifest");
            }
        }

        // 3. Look for combined video+audio streams (prefer higher quality)
        // itag 18: 360p MP4 with audio
        // itag 22: 720p MP4 with audio (if available)
        let preferred_itags: Vec<&str> = vec!["22", "18"];

        for itag in &preferred_itags {
            if let Some(stream) = details
                .format_streams
                .iter()
                .find(|s| s.itag.as_deref() == Some(*itag))
            {
                if !stream.url.is_empty() {
                    let expires_at = extract_expiration_from_url(&stream.url);
                    return Ok(VideoStream {
                        title: details.title,
                        duration: details.length_seconds,
                        stream_url: stream.url.clone(),
                        mime_type: stream.mime_type.clone(),
                        is_hls: false,
                        expires_at,
                    });
                }
            }
        }

        // 4. Fall back to any format stream with a URL
        if let Some(stream) = details.format_streams.first() {
            if !stream.url.is_empty() {
                let expires_at = extract_expiration_from_url(&stream.url);
                return Ok(VideoStream {
                    title: details.title,
                    duration: details.length_seconds,
                    stream_url: stream.url.clone(),
                    mime_type: stream.mime_type.clone(),
                    is_hls: false,
                    expires_at,
                });
            }
        }

        Err(AppError::NoCompatibleFormat)
    }

    /// Fetch channel avatar - checks in-memory cache first, then disk cache, then fetches from Invidious
    async fn fetch_channel_avatar(&self, channel_id: &str) -> Result<String, AppError> {
        // 1. Check in-memory cache (24h TTL)
        {
            let cache = self.avatar_cache.read().await;
            if let Some((url, fetched_at)) = cache.get(channel_id) {
                if fetched_at.elapsed().as_secs() < AVATAR_CACHE_TTL_SECS {
                    // Return cached URL (points to /static/youtube_images/)
                    return Ok(url.clone());
                }
            }
        } // Drop read lock

        // 2. Check disk cache
        if let Some(_path) = youtube_channels::get_cached_image_path(channel_id) {
            // Image exists on disk, return local URL
            let local_url = format!("/static/youtube_images/{}.jpg", channel_id);

            // Update in-memory cache
            let mut cache = self.avatar_cache.write().await;
            cache.insert(channel_id.to_string(), (local_url.clone(), Instant::now()));

            return Ok(local_url);
        }

        // 3. Fetch from Invidious API
        let url = format!("{}/api/v1/channels/{}", self.base_url, channel_id);

        let response = self.http.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                AppError::InvidiousUnreachable
            } else if e.is_connect() {
                AppError::InvidiousUnreachable
            } else {
                AppError::Http(e)
            }
        })?;

        if !response.status().is_success() {
            return Err(AppError::InvidiousBadResponse);
        }

        let details: InvidiousChannelDetails = response
            .json()
            .await
            .map_err(|_| AppError::InvidiousBadResponse)?;

        // Get the best avatar URL (prefer 176px, fall back to first available)
        let avatar_url = details
            .author_thumbnails
            .as_ref()
            .and_then(|thumbs| {
                // Try to find 176px thumbnail first
                thumbs
                    .iter()
                    .find(|t| t.width == Some(176))
                    .or_else(|| thumbs.last())
                    .map(|t| t.url.clone())
            })
            .ok_or_else(|| AppError::InvidiousBadResponse)?;

        // 4. Download the image
        let image_response = self.http.get(&avatar_url).send().await.map_err(|e| {
            tracing::error!(error = %e, channel_id = %channel_id, "Failed to download channel avatar");
            AppError::Http(e)
        })?;

        let image_bytes = image_response
            .bytes()
            .await
            .map_err(|_| AppError::InvidiousBadResponse)?;

        // 5. Save to disk
        let filename = youtube_channels::save_channel_image(channel_id, &image_bytes)?;
        youtube_channels::update_channel_image(channel_id, &filename, &avatar_url)?;

        // 6. Return local URL with extension
        let local_url = format!("/static/youtube_images/{}.jpg", channel_id);

        // 7. Update in-memory cache
        let mut cache = self.avatar_cache.write().await;
        cache.insert(channel_id.to_string(), (local_url.clone(), Instant::now()));

        Ok(local_url)
    }
}

/// Validate YouTube channel ID format
fn is_valid_channel_id(channel_id: &str) -> bool {
    // Channel IDs start with "UC" and are 24 characters long
    if channel_id.len() != 24 {
        return false;
    }
    if !channel_id.starts_with("UC") {
        return false;
    }
    // Check all characters are alphanumeric or underscores/hyphens
    channel_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Validate YouTube video ID format
pub fn is_valid_video_id(video_id: &str) -> bool {
    // Video IDs are 11 characters
    if video_id.len() != 11 {
        return false;
    }
    // Check all characters are alphanumeric or underscores/hyphens
    video_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Parse a YouTube URL to extract video ID
pub fn parse_youtube_url(url: &str) -> Option<String> {
    let url = url.trim();

    // youtube.com/watch?v=VIDEO_ID
    if let Some(idx) = url.find("youtube.com/watch?v=") {
        let start = idx + "youtube.com/watch?v=".len();
        let rest = &url[start..];
        let end = rest.find('&').unwrap_or(rest.len());
        let video_id = &rest[..end];
        if is_valid_video_id(video_id) {
            return Some(video_id.to_string());
        }
    }

    // youtu.be/VIDEO_ID
    if let Some(idx) = url.find("youtu.be/") {
        let start = idx + "youtu.be/".len();
        let rest = &url[start..];
        let end = rest.find('?').unwrap_or(rest.len());
        let video_id = &rest[..end];
        if is_valid_video_id(video_id) {
            return Some(video_id.to_string());
        }
    }

    // youtube.com/live/VIDEO_ID
    if let Some(idx) = url.find("youtube.com/live/") {
        let start = idx + "youtube.com/live/".len();
        let rest = &url[start..];
        let end = rest.find('/').unwrap_or(rest.len().min(start + 11));
        let video_id = &rest[..end.min(11)];
        if is_valid_video_id(video_id) {
            return Some(video_id.to_string());
        }
    }

    // youtube.com/shorts/VIDEO_ID
    if let Some(idx) = url.find("youtube.com/shorts/") {
        let start = idx + "youtube.com/shorts/".len();
        let rest = &url[start..];
        let end = rest.find('/').unwrap_or(rest.len().min(start + 11));
        let video_id = &rest[..end.min(11)];
        if is_valid_video_id(video_id) {
            return Some(video_id.to_string());
        }
    }

    // Check if the input is just a raw video ID
    if is_valid_video_id(url) {
        return Some(url.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_channel_id() {
        assert!(is_valid_channel_id("UC_x5XG1OV2P6uZZ5FSM9Ttw"));
        assert!(!is_valid_channel_id("invalid"));
        assert!(!is_valid_channel_id("UC_short"));
        assert!(!is_valid_channel_id("NOTSTARTINGWITHUC12345678901"));
    }

    #[test]
    fn test_is_valid_video_id() {
        assert!(is_valid_video_id("dQw4w9WgXcQ"));
        assert!(!is_valid_video_id("tooshort"));
        assert!(!is_valid_video_id("waytoolongforvideoid"));
    }

    #[test]
    fn test_parse_youtube_url() {
        assert_eq!(
            parse_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            parse_youtube_url("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            parse_youtube_url("https://www.youtube.com/live/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            parse_youtube_url("https://youtube.com/shorts/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            parse_youtube_url("dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(parse_youtube_url("https://example.com/video"), None);
    }
}
