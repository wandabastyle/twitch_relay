use std::{
   collections::HashMap,
   sync::Arc,
   time::{
      Duration,
      Instant,
   },
};

use reqwest::header::{
   AUTHORIZATION,
   HeaderMap,
   HeaderValue,
};
use serde::{
   Deserialize,
   Serialize,
};
use tokio::sync::RwLock;

use crate::{
   config::InvidiousConfig,
   error::AppError,
   youtube_channels,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const AVATAR_CACHE_TTL_SECS: u64 = 86400; // 24 hours
const DESCRIPTION_CACHE_TTL_SECS: u64 = 86400; // 24 hours

/// Invidious API client for fetching `YouTube` data
#[derive(Debug, Clone)]
pub struct InvidiousClient {
   base_url:          String,
   pub http:          reqwest::Client,
   avatar_cache:      Arc<RwLock<HashMap<String, (String, Instant)>>>, /* channel_id -> (url,
                                                                        * fetched_at) */
   description_cache: Arc<RwLock<HashMap<String, (String, Instant)>>>, /* channel_id ->
                                                                        * (description,
                                                                        * fetched_at) */
   // (user, password) for reverse proxy Basic auth.
   // When Basic auth is configured, the Authorization header is used for proxy auth,
   // so the Invidious session must be sent via the SID cookie instead.
   basic_auth:        Option<(String, String)>,
}

/// Normalized `YouTube` channel from Invidious subscriptions
#[derive(Debug, Clone, Serialize)]
pub struct YoutubeChannel {
   pub name:        String,
   pub channel_id:  String,
   pub url:         String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub avatar:      Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub description: Option<String>,
}

/// Normalized `YouTube` channel info with description
#[derive(Debug, Clone, Serialize)]
pub struct YoutubeChannelInfo {
   pub name:             String,
   pub channel_id:       String,
   pub url:              String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub description:      Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub description_html: Option<String>,
   pub sub_count:        i64,
   pub author_verified:  bool,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub avatar:           Option<String>,
}

/// Normalized `YouTube` video from Invidious
#[derive(Debug, Clone, Serialize)]
pub struct YoutubeVideo {
   pub title:          String,
   pub video_id:       String,
   pub author:         String,
   pub author_id:      String,
   pub published:      i64,
   pub published_text: String,
   pub duration:       i64,
   pub thumbnail:      String,
   pub view_count:     i64,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub description:    Option<String>,
}

/// `YouTube` video metadata for watch page
#[derive(Debug, Clone, Serialize)]
pub struct YoutubeVideoMeta {
   pub title:    String,
   pub duration: i64,
}

/// Normalized `YouTube` playlist from Invidious
#[derive(Debug, Clone, Serialize)]
pub struct YoutubePlaylist {
   pub title:       String,
   pub playlist_id: String,
   pub video_count: i64,
   pub updated:     i64,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub thumbnail:   Option<String>,
}

/// Raw Invidious subscription response
#[derive(Debug, Deserialize)]
struct InvidiousSubscription {
   author:    String,
   #[serde(rename = "authorId")]
   author_id: String,
}

/// Raw Invidious channel details (for avatar fetching)
#[derive(Debug, Deserialize)]
struct InvidiousChannelDetails {
   #[serde(rename = "authorThumbnails")]
   author_thumbnails: Option<Vec<Thumbnail>>,
}

/// Raw Invidious channel info response (includes description)
#[derive(Debug, Deserialize)]
struct InvidiousChannelInfo {
   author:            String,
   #[serde(rename = "authorId")]
   author_id:         String,
   #[serde(rename = "authorThumbnails")]
   author_thumbnails: Option<Vec<Thumbnail>>,
   #[serde(default)]
   description:       String,
   #[serde(rename = "descriptionHtml", default)]
   description_html:  String,
   #[serde(rename = "subCount", default)]
   sub_count:         i64,
   #[serde(rename = "authorVerified", default)]
   author_verified:   bool,
}

/// Raw Invidious video response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvidiousVideoRaw {
   title:          String,
   video_id:       String,
   author:         String,
   author_id:      String,
   #[serde(default)]
   published:      i64,
   #[serde(default)]
   published_text: String,
   #[serde(default)]
   length_seconds: i64,
   #[serde(default)]
   view_count:     i64,
   #[serde(default)]
   description:    String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvidiousVideoDetails {
   title:          String,
   #[serde(default)]
   length_seconds: i64,
}

/// Raw Invidious playlist response (for auth/playlists endpoint)
#[derive(Debug, Deserialize)]
struct InvidiousPlaylistRaw {
   title:              String,
   #[serde(rename = "playlistId")]
   playlist_id:        String,
   #[serde(rename = "videoCount")]
   video_count:        i64,
   #[serde(default)]
   updated:            i64,
   playlist_thumbnail: Option<String>,
}

/// Raw Invidious playlist details response
#[derive(Debug, Deserialize)]
struct InvidiousPlaylistDetails {
   videos: Vec<InvidiousVideoRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum InvidiousRecentFeedResponse {
   Videos(Vec<InvidiousVideoRaw>),
   Wrapped { videos: Vec<InvidiousVideoRaw> },
}

#[derive(Debug, Clone, Deserialize)]
struct Thumbnail {
   url:   String,
   width: Option<i32>,
}

fn map_reqwest_error(e: reqwest::Error) -> AppError {
   if e.is_timeout() || e.is_connect() {
      AppError::InvidiousUnreachable
   } else {
      AppError::Http(e)
   }
}

impl InvidiousClient {
   /// Create a new Invidious client from config
   pub fn new(config: &InvidiousConfig) -> Self {
      let mut headers = HeaderMap::new();

      // Add Bearer token for Invidious API auth (fallback if SID cookie not
      // available)
      if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", config.token)) {
         headers.insert(AUTHORIZATION, value);
      }

      // Add SID cookie header for Invidious session auth.
      // When reverse-proxy Basic auth is enabled, the outbound request's
      // Authorization header is used for Basic auth. In that mode, the
      // Invidious account/session credential must be sent via the SID cookie
      // instead of the Bearer Authorization header.
      if let Some(ref sid) = config.sid_cookie
         && let Ok(value) = HeaderValue::from_str(&format!("SID={sid}"))
      {
         headers.insert(reqwest::header::COOKIE, value);
      }

      let http = reqwest::Client::builder()
         .timeout(REQUEST_TIMEOUT)
         .default_headers(headers)
         .build()
         .expect("failed to build reqwest client");

      let basic_auth = config
         .basic_auth_user
         .as_ref()
         .zip(config.basic_auth_password.as_ref())
         .map(|(u, p)| (u.clone(), p.clone()));

      Self {
         base_url: config.base_url.clone(),
         http,
         avatar_cache: Arc::new(RwLock::new(HashMap::new())),
         description_cache: Arc::new(RwLock::new(HashMap::new())),
         basic_auth,
      }
   }

   /// Helper to apply basic auth to a request builder if configured
   pub fn with_basic_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
      if let Some((ref user, ref pass)) = self.basic_auth {
         request.basic_auth(user, Some(pass))
      } else {
         request
      }
   }

   /// Get authenticated user's subscriptions
   pub async fn get_subscriptions(&self) -> Result<Vec<YoutubeChannel>, AppError> {
      let url = format!("{}/api/v1/auth/subscriptions", self.base_url);

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      match response.status() {
         status if status.is_success() => {
            let subscriptions: Vec<InvidiousSubscription> = response
               .json()
               .await
               .map_err(|_| AppError::InvidiousBadResponse)?;

            // Eagerly fetch channel details, avatars, and descriptions in parallel
            let avatar_futures: Vec<_> = subscriptions
               .iter()
               .map(|sub| self.fetch_channel_avatar(&sub.author_id))
               .collect();

            let description_futures: Vec<_> = subscriptions
               .iter()
               .map(|sub| self.fetch_channel_description(&sub.author_id))
               .collect();

            let (avatar_results, description_results) = futures_util::future::join(
               futures_util::future::join_all(avatar_futures),
               futures_util::future::join_all(description_futures),
            )
            .await;

            let mut channels: Vec<YoutubeChannel> = subscriptions
               .into_iter()
               .zip(avatar_results)
               .zip(description_results)
               .map(|((sub, avatar_result), description_result)| {
                  // Use cached avatar URL or fallback to external URL
                  let avatar = match avatar_result {
                     Ok(url) => Some(url),
                     Err(_) => {
                        // Check if we have a cached image on disk
                        youtube_channels::get_channel_image_url(&sub.author_id)
                     },
                  };

                  // Use cached description if available
                  let description = description_result.ok().filter(|d| !d.is_empty());

                  YoutubeChannel {
                     name: sub.author,
                     channel_id: sub.author_id.clone(),
                     url: format!("/channel/{}", sub.author_id),
                     avatar,
                     description,
                  }
               })
               .collect();

            // Sort alphabetically by name
            channels.sort_by_key(|a| a.name.to_lowercase());

            Ok(channels)
         },
         status if status.as_u16() == 401 => Err(AppError::InvidiousAuthFailed),
         status if status.as_u16() == 429 => Err(AppError::InvidiousRateLimited),
         _ => Err(AppError::InvidiousBadResponse),
      }
   }

   pub async fn mark_video_watched(&self, video_id: &str) -> Result<(), AppError> {
      if !is_valid_video_id(video_id) {
         return Err(AppError::Config(format!("invalid video_id: {video_id}")));
      }

      let url = format!("{}/api/v1/auth/history/{}", self.base_url, video_id);
      let response = self
         .with_basic_auth(self.http.post(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      if response.status().is_success() {
         Ok(())
      } else {
         Err(AppError::InvidiousBadResponse)
      }
   }

   pub async fn mark_video_unwatched(&self, video_id: &str) -> Result<(), AppError> {
      if !is_valid_video_id(video_id) {
         return Err(AppError::Config(format!("invalid video_id: {video_id}")));
      }

      let url = format!("{}/api/v1/auth/history/{}", self.base_url, video_id);
      let response = self
         .with_basic_auth(self.http.delete(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      if response.status().is_success() {
         Ok(())
      } else {
         Err(AppError::InvidiousBadResponse)
      }
   }

   /// Get latest videos for a channel
   pub async fn get_channel_videos(
      &self,
      channel_id: &str,
      max_results: Option<u32>,
   ) -> Result<Vec<YoutubeVideo>, AppError> {
      // Response structure for channel videos API
      #[derive(Debug, Deserialize)]
      struct ChannelVideosResponse {
         videos: Vec<InvidiousVideoRaw>,
      }

      // Validate channel_id format (should start with UC and be 24 chars)
      if !is_valid_channel_id(channel_id) {
         return Err(AppError::Config(format!(
            "invalid channel_id: {channel_id}"
         )));
      }

      let max = max_results.unwrap_or(20).min(40);
      let url = format!(
         "{}/api/v1/channels/{}/videos?max_results={}",
         self.base_url, channel_id, max
      );

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      if !response.status().is_success() {
         return Err(AppError::InvidiousBadResponse);
      }

      // The response is a channel object with a "videos" field
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

   /// Get authenticated user's recent videos from subscription feed
   pub async fn get_recent_videos(
      &self,
      max_results: Option<u32>,
   ) -> Result<Vec<YoutubeVideo>, AppError> {
      let max = max_results.unwrap_or(25).min(40);
      let url = format!("{}/api/v1/auth/feed?max_results={}", self.base_url, max);

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      match response.status() {
         status if status.is_success() => {
            let feed: InvidiousRecentFeedResponse = response
               .json()
               .await
               .map_err(|_| AppError::InvidiousBadResponse)?;

            let mut videos: Vec<YoutubeVideo> = match feed {
               InvidiousRecentFeedResponse::Videos(videos)
               | InvidiousRecentFeedResponse::Wrapped { videos } => videos,
            }
            .into_iter()
            .map(|v| {
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

            videos.sort_by_key(|video| std::cmp::Reverse(video.published));
            Ok(videos)
         },
         status if status.as_u16() == 401 => Err(AppError::InvidiousAuthFailed),
         status if status.as_u16() == 429 => Err(AppError::InvidiousRateLimited),
         _ => Err(AppError::InvidiousBadResponse),
      }
   }

   /// Get channel info including description
   pub async fn get_channel_info(&self, channel_id: &str) -> Result<YoutubeChannelInfo, AppError> {
      // Validate channel_id format (should start with UC and be 24 chars)
      if !is_valid_channel_id(channel_id) {
         return Err(AppError::Config(format!(
            "invalid channel_id: {channel_id}"
         )));
      }

      let url = format!("{}/api/v1/channels/{}", self.base_url, channel_id);

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      if !response.status().is_success() {
         return Err(AppError::InvidiousBadResponse);
      }

      let info: InvidiousChannelInfo = response
         .json()
         .await
         .map_err(|_| AppError::InvidiousBadResponse)?;

      // Get avatar URL from thumbnails
      let avatar = info
         .author_thumbnails
         .as_ref()
         .and_then(|thumbs| {
            thumbs
               .iter()
               .find(|t| t.width == Some(176))
               .or_else(|| thumbs.last())
               .map(|t| t.url.clone())
         })
         .filter(|url| !url.is_empty());

      Ok(YoutubeChannelInfo {
         name: info.author.clone(),
         channel_id: info.author_id.clone(),
         url: format!("/channel/{}", info.author_id),
         description: Some(info.description).filter(|d| !d.is_empty()),
         description_html: Some(info.description_html).filter(|d| !d.is_empty()),
         sub_count: info.sub_count,
         author_verified: info.author_verified,
         avatar,
      })
   }

   /// Get video metadata (title + duration)
   pub async fn get_video_meta(&self, video_id: &str) -> Result<YoutubeVideoMeta, AppError> {
      if !is_valid_video_id(video_id) {
         return Err(AppError::Config(format!("invalid video_id: {video_id}")));
      }

      let url = format!("{}/api/v1/videos/{}", self.base_url, video_id);

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      if !response.status().is_success() {
         return Err(AppError::InvidiousBadResponse);
      }

      let details: InvidiousVideoDetails = response
         .json()
         .await
         .map_err(|_| AppError::InvidiousBadResponse)?;

      Ok(YoutubeVideoMeta {
         title:    details.title,
         duration: details.length_seconds,
      })
   }

   /// Get authenticated user's playlists
   pub async fn get_playlists(&self) -> Result<Vec<YoutubePlaylist>, AppError> {
      let url = format!("{}/api/v1/auth/playlists", self.base_url);

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      match response.status() {
         status if status.is_success() => {
            let playlists: Vec<InvidiousPlaylistRaw> = response
               .json()
               .await
               .map_err(|_| AppError::InvidiousBadResponse)?;

            let mut result: Vec<YoutubePlaylist> = playlists
               .into_iter()
               .map(|p| {
                  YoutubePlaylist {
                     title:       p.title,
                     playlist_id: p.playlist_id.clone(),
                     video_count: p.video_count,
                     updated:     p.updated,
                     thumbnail:   p.playlist_thumbnail,
                  }
               })
               .collect();

            // Sort by updated timestamp descending (most recent first)
            result.sort_by_key(|a| std::cmp::Reverse(a.updated));

            Ok(result)
         },
         status if status.as_u16() == 401 => Err(AppError::InvidiousAuthFailed),
         status if status.as_u16() == 429 => Err(AppError::InvidiousRateLimited),
         _ => Err(AppError::InvidiousBadResponse),
      }
   }

   /// Get videos from a playlist
   pub async fn get_playlist_videos(
      &self,
      playlist_id: &str,
   ) -> Result<Vec<YoutubeVideo>, AppError> {
      if !is_valid_playlist_id(playlist_id) {
         return Err(AppError::Config(format!(
            "invalid playlist_id: {playlist_id}"
         )));
      }

      let url = format!("{}/api/v1/playlists/{}", self.base_url, playlist_id);

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      if !response.status().is_success() {
         return Err(AppError::InvidiousBadResponse);
      }

      let playlist_data: InvidiousPlaylistDetails = response
         .json()
         .await
         .map_err(|_| AppError::InvidiousBadResponse)?;

      let videos: Vec<YoutubeVideo> = playlist_data
         .videos
         .into_iter()
         .map(|v| {
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

   /// Fetch channel avatar - checks in-memory cache first, then disk cache,
   /// then fetches from Invidious
   async fn fetch_channel_avatar(&self, channel_id: &str) -> Result<String, AppError> {
      // 1. Check in-memory cache (24h TTL)
      {
         let cache = self.avatar_cache.read().await;
         if let Some((url, fetched_at)) = cache.get(channel_id)
            && fetched_at.elapsed().as_secs() < AVATAR_CACHE_TTL_SECS
         {
            // Return cached URL (points to /static/youtube_images/)
            return Ok(url.clone());
         }
      } // Drop read lock

      // 2. Check disk cache
      if let Some(_path) = youtube_channels::get_cached_image_path(channel_id) {
         // Image exists on disk, return local URL
         let local_url = format!("/static/youtube_images/{channel_id}.jpg");

         // Update in-memory cache
         {
            let mut cache = self.avatar_cache.write().await;
            cache.insert(channel_id.to_string(), (local_url.clone(), Instant::now()));
         }

         return Ok(local_url);
      }

      // 3. Fetch from Invidious API
      let url = format!("{}/api/v1/channels/{}", self.base_url, channel_id);

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

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

      // 4. Download the image (external URL, no basic auth needed)
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
      let local_url = format!("/static/youtube_images/{channel_id}.jpg");

      // 7. Update in-memory cache
      {
         let mut cache = self.avatar_cache.write().await;
         cache.insert(channel_id.to_string(), (local_url.clone(), Instant::now()));
      }

      Ok(local_url)
   }

   /// Fetch channel description - checks in-memory cache first, then fetches
   /// from Invidious
   async fn fetch_channel_description(&self, channel_id: &str) -> Result<String, AppError> {
      // 1. Check in-memory cache (24h TTL)
      {
         let cache = self.description_cache.read().await;
         if let Some((description, fetched_at)) = cache.get(channel_id)
            && fetched_at.elapsed().as_secs() < DESCRIPTION_CACHE_TTL_SECS
         {
            return Ok(description.clone());
         }
      } // Drop read lock

      // 2. Fetch from Invidious API
      let url = format!("{}/api/v1/channels/{}", self.base_url, channel_id);

      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      if !response.status().is_success() {
         return Err(AppError::InvidiousBadResponse);
      }

      let info: InvidiousChannelInfo = response
         .json()
         .await
         .map_err(|_| AppError::InvidiousBadResponse)?;

      let description = info.description;

      // 3. Update in-memory cache
      {
         let mut cache = self.description_cache.write().await;
         cache.insert(
            channel_id.to_string(),
            (description.clone(), Instant::now()),
         );
      }

      Ok(description)
   }
}

/// Validate `YouTube` channel ID format
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

/// Validate `YouTube` video ID format
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

/// Validate `YouTube` playlist ID format
pub fn is_valid_playlist_id(playlist_id: &str) -> bool {
   // Valid playlist prefixes (PL = playlist, IV = liked videos, etc.)
   // UC is specifically excluded as it's a channel ID prefix
   const VALID_PREFIXES: &[&str] = &[
      "PL", "IV", "OL", "FL", "WL", "LL", "RD", "UU", "PU", "EN", "MM", "EL",
   ];

   // Playlist IDs must be at least 3 characters
   if playlist_id.len() < 3 {
      return false;
   }

   // Check if starts with a valid playlist prefix
   if !VALID_PREFIXES
      .iter()
      .any(|&prefix| playlist_id.starts_with(prefix))
   {
      return false;
   }

   // All characters must be alphanumeric or underscores/hyphens
   playlist_id
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
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
   fn test_is_valid_channel_id_rejects_invalid_characters() {
      // Valid channel ID format: UC prefix + 22 alphanumeric/underscore/hyphen chars
      let base = "UC_x5XG1OV2P6uZZ5FSM9Tt";
      assert!(!is_valid_channel_id(
         &format!("{base}w").replace('w', "!")
      ));
      assert!(!is_valid_channel_id(
         &format!("{base}w").replace('w', "@")
      ));
      assert!(!is_valid_channel_id(
         &format!("{base}w").replace('w', "#")
      ));
      assert!(!is_valid_channel_id(
         &format!("{base}w").replace('w', "$")
      ));
      assert!(!is_valid_channel_id(
         &format!("{base}w").replace('w', "%")
      ));
      assert!(!is_valid_channel_id(
         &format!("{base}w").replace('w', "+")
      ));
      assert!(!is_valid_channel_id(
         &format!("{base}w").replace('w', "=")
      ));
      assert!(!is_valid_channel_id(
         &format!("{base}w").replace('w', " ")
      ));
   }

   #[test]
   fn test_is_valid_video_id() {
      assert!(is_valid_video_id("dQw4w9WgXcQ"));
      assert!(!is_valid_video_id("tooshort"));
      assert!(!is_valid_video_id("waytoolongforvideoid"));
   }

   #[test]
   fn test_is_valid_video_id_rejects_invalid_characters() {
      // Valid video ID format: 11 alphanumeric/underscore/hyphen chars
      assert!(!is_valid_video_id("dQw4w9WgXc!"));
      assert!(!is_valid_video_id("dQw4w9WgXc@"));
      assert!(!is_valid_video_id("dQw4w9WgXc#"));
      assert!(!is_valid_video_id("dQw4w9WgXc$"));
      assert!(!is_valid_video_id("dQw4w9WgXc%"));
      assert!(!is_valid_video_id("dQw4w9WgXc+"));
      assert!(!is_valid_video_id("dQw4w9WgXc="));
      assert!(!is_valid_video_id("dQw4w9WgXcQ "));
   }

   #[test]
   fn test_is_valid_playlist_id() {
      assert!(is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
      assert!(!is_valid_playlist_id("PL"));
      assert!(!is_valid_playlist_id("invalid"));
      assert!(!is_valid_playlist_id("UC_invalid_playlist"));
   }

   #[test]
   fn test_is_valid_playlist_id_rejects_invalid_characters() {
      // Valid playlist ID format: valid prefix + at least 1
      // alphanumeric/underscore/hyphen
      assert!(!is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxx!x"));
      assert!(!is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxx@x"));
      assert!(!is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxx#x"));
      assert!(!is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxx$x"));
      assert!(!is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxx%x"));
      assert!(!is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxx+x"));
      assert!(!is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxx=x"));
      assert!(!is_valid_playlist_id("PLxxxxxxxxxxxxxxxxxxxxxxxxxx x"));
   }
}
