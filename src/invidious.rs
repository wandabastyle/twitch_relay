use std::{
   collections::{
      HashMap,
      HashSet,
   },
   path::PathBuf,
   sync::Arc,
   time::{
      Duration,
      Instant,
   },
};

use futures_util::stream::{
   self,
   StreamExt,
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
   storage::{
      files,
      paths,
   },
   util::time::now_unix_secs,
   youtube_channels,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const RECENT_VIDEOS_MAX_RESULTS: u32 = 40;
const AVATAR_CACHE_TTL_SECS: u64 = 86400; // 24 hours
const DESCRIPTION_CACHE_TTL_SECS: u64 = 86400; // 24 hours
const FALLBACK_FETCH_CONCURRENCY: usize = 3;
const FALLBACK_FETCH_TIMEOUT_SECS: u64 = 4;
const FALLBACK_POSITIVE_TTL_SECS: u64 = 30 * 24 * 60 * 60;
const FALLBACK_NEGATIVE_TTL_SECS: u64 = 3 * 60 * 60;
const FALLBACK_CACHE_MAX_ENTRIES: usize = 5000;

const fn is_negative_fallback_cache_fresh(elapsed_secs: u64) -> bool {
   elapsed_secs < FALLBACK_NEGATIVE_TTL_SECS
}

fn extract_channel_playlist_fallback_id(value: &serde_json::Value) -> Option<String> {
   let item_type = value
      .get("type")
      .and_then(serde_json::Value::as_str)
      .unwrap_or_default();
   let maybe_playlist_id = value
      .get("playlistId")
      .and_then(serde_json::Value::as_str)
      .map(str::to_string);

   if item_type == "playlist"
      && let Some(video_id) = maybe_playlist_id
      && is_valid_video_id(&video_id)
   {
      return Some(video_id);
   }

   None
}

fn build_fallback_duration_map(
   fallback_results: Vec<(String, Option<YoutubeVideo>)>,
) -> HashMap<String, i64> {
   fallback_results
      .into_iter()
      .filter_map(|(video_id, video)| {
         video
            .filter(|resolved| resolved.duration > 0)
            .map(|resolved| (video_id, resolved.duration))
      })
      .collect()
}

fn apply_fallback_durations(
   videos: &mut [YoutubeVideo],
   fallback_durations: &HashMap<String, i64>,
) {
   for video in videos {
      if video.duration <= 0
         && let Some(duration) = fallback_durations.get(&video.video_id)
      {
         video.duration = *duration;
      }
   }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedCachedVideoLookup {
   video:              Option<YoutubeVideo>,
   fetched_at_unix:    u64,
   last_accessed_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedFallbackVideoCache {
   entries: HashMap<String, PersistedCachedVideoLookup>,
}

#[derive(Debug, Clone)]
struct CachedVideoLookup {
   video:              Option<YoutubeVideo>,
   fetched_at_unix:    u64,
   last_accessed_unix: u64,
}

/// Invidious API client for fetching `YouTube` data
#[derive(Debug, Clone)]
pub struct InvidiousClient {
   base_url:             String,
   pub http:             reqwest::Client,
   avatar_cache:         Arc<RwLock<HashMap<String, (String, Instant)>>>, /* channel_id -> (url,
                                                                           * fetched_at) */
   description_cache:    Arc<RwLock<HashMap<String, (String, Instant)>>>, /* channel_id ->
                                                                           * (description,
                                                                           * fetched_at) */
   fallback_video_cache: Arc<RwLock<HashMap<String, CachedVideoLookup>>>,
   fallback_inflight:    Arc<RwLock<HashSet<String>>>,
   fallback_cache_path:  Option<PathBuf>,
   // (user, password) for reverse proxy Basic auth.
   // When Basic auth is configured, the Authorization header is used for proxy auth,
   // so the Invidious session must be sent via the SID cookie instead.
   basic_auth:           Option<(String, String)>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

const fn is_positive_fallback_cache_fresh(elapsed_secs: u64) -> bool {
   elapsed_secs < FALLBACK_POSITIVE_TTL_SECS
}

const fn is_fallback_entry_fresh(video: Option<&YoutubeVideo>, elapsed_secs: u64) -> bool {
   if video.is_some() {
      return is_positive_fallback_cache_fresh(elapsed_secs);
   }
   is_negative_fallback_cache_fresh(elapsed_secs)
}

fn prune_fallback_cache_entries(cache: &mut HashMap<String, CachedVideoLookup>, now_unix: u64) {
   cache.retain(|_, entry| {
      let elapsed = now_unix.saturating_sub(entry.fetched_at_unix);
      is_fallback_entry_fresh(entry.video.as_ref(), elapsed)
   });

   if cache.len() <= FALLBACK_CACHE_MAX_ENTRIES {
      return;
   }

   let mut keys_by_last_access: Vec<(String, u64)> = cache
      .iter()
      .map(|(key, value)| (key.clone(), value.last_accessed_unix))
      .collect();
   keys_by_last_access.sort_by_key(|(_, last_accessed_unix)| *last_accessed_unix);

   let overflow = cache.len() - FALLBACK_CACHE_MAX_ENTRIES;
   for (key, _) in keys_by_last_access.into_iter().take(overflow) {
      cache.remove(&key);
   }
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvidiousVideoFullDetails {
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
struct InvidiousChannelVideosResponseRaw {
   videos: Vec<serde_json::Value>,
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
         fallback_video_cache: Arc::new(RwLock::new(Self::load_fallback_video_cache())),
         fallback_inflight: Arc::new(RwLock::new(HashSet::new())),
         fallback_cache_path: paths::youtube_fallback_video_cache_path(),
         basic_auth,
      }
   }

   fn load_fallback_video_cache() -> HashMap<String, CachedVideoLookup> {
      let Some(path) = paths::youtube_fallback_video_cache_path() else {
         return HashMap::new();
      };

      let loaded = match files::load_json_optional::<PersistedFallbackVideoCache>(&path) {
         Ok(Some(cache)) => cache,
         Ok(None) => PersistedFallbackVideoCache::default(),
         Err(error) => {
            tracing::warn!(error = %error, path = %path.display(), "failed to load youtube fallback cache");
            PersistedFallbackVideoCache::default()
         },
      };

      let now_unix = now_unix_secs();
      let mut entries: HashMap<String, CachedVideoLookup> = loaded
         .entries
         .into_iter()
         .map(|(video_id, entry)| {
            (video_id, CachedVideoLookup {
               video:              entry.video,
               fetched_at_unix:    entry.fetched_at_unix,
               last_accessed_unix: entry.last_accessed_unix,
            })
         })
         .collect();

      prune_fallback_cache_entries(&mut entries, now_unix);
      entries
   }

   async fn save_fallback_video_cache(&self) {
      let Some(path) = self.fallback_cache_path.as_ref() else {
         return;
      };

      let entries = {
         let cache = self.fallback_video_cache.read().await;
         cache
            .iter()
            .map(|(video_id, entry)| {
               (video_id.clone(), PersistedCachedVideoLookup {
                  video:              entry.video.clone(),
                  fetched_at_unix:    entry.fetched_at_unix,
                  last_accessed_unix: entry.last_accessed_unix,
               })
            })
            .collect()
      };

      let payload = PersistedFallbackVideoCache { entries };
      if let Err(error) = files::write_json_pretty_atomic(path, &payload) {
         tracing::warn!(error = %error, path = %path.display(), "failed to save youtube fallback cache");
      }
   }

   async fn get_cached_fallback_video(&self, video_id: &str) -> Option<Option<YoutubeVideo>> {
      let now_unix = now_unix_secs();
      let mut cache = self.fallback_video_cache.write().await;
      let entry = cache.get_mut(video_id)?;
      let elapsed_secs = now_unix.saturating_sub(entry.fetched_at_unix);
      if is_fallback_entry_fresh(entry.video.as_ref(), elapsed_secs) {
         entry.last_accessed_unix = now_unix;
         return Some(entry.video.clone());
      }
      cache.remove(video_id);
      drop(cache);
      self.save_fallback_video_cache().await;

      None
   }

   async fn set_cached_fallback_video(&self, video_id: &str, video: Option<YoutubeVideo>) {
      let now_unix = now_unix_secs();
      let mut cache = self.fallback_video_cache.write().await;
      cache.insert(video_id.to_string(), CachedVideoLookup {
         video,
         fetched_at_unix: now_unix,
         last_accessed_unix: now_unix,
      });
      prune_fallback_cache_entries(&mut cache, now_unix);
      drop(cache);
      self.save_fallback_video_cache().await;
   }

   async fn mark_inflight(&self, video_id: &str) -> bool {
      let mut inflight = self.fallback_inflight.write().await;
      inflight.insert(video_id.to_string())
   }

   async fn clear_inflight(&self, video_id: &str) {
      let mut inflight = self.fallback_inflight.write().await;
      inflight.remove(video_id);
   }

   async fn fetch_video_fallback(&self, video_id: &str) -> Option<YoutubeVideo> {
      if !is_valid_video_id(video_id) {
         return None;
      }

      if let Some(cached) = self.get_cached_fallback_video(video_id).await {
         return cached;
      }

      if !self.mark_inflight(video_id).await {
         if let Some(cached) = self.get_cached_fallback_video(video_id).await {
            return cached;
         }
         return None;
      }

      let result = tokio::time::timeout(
         Duration::from_secs(FALLBACK_FETCH_TIMEOUT_SECS),
         self.fetch_video_fallback_uncached(video_id),
      )
      .await
      .ok()
      .and_then(Result::ok);

      self
         .set_cached_fallback_video(video_id, result.clone())
         .await;
      self.clear_inflight(video_id).await;
      result
   }

   async fn fetch_video_fallback_uncached(&self, video_id: &str) -> Result<YoutubeVideo, AppError> {
      let url = format!("{}/api/v1/videos/{}", self.base_url, video_id);
      let response = self
         .with_basic_auth(self.http.get(&url))
         .send()
         .await
         .map_err(map_reqwest_error)?;

      if !response.status().is_success() {
         return Err(AppError::InvidiousBadResponse);
      }

      let details: InvidiousVideoFullDetails = response
         .json()
         .await
         .map_err(|_| AppError::InvidiousBadResponse)?;

      Ok(YoutubeVideo {
         title:          details.title,
         video_id:       details.video_id.clone(),
         author:         details.author,
         author_id:      details.author_id,
         published:      details.published,
         published_text: details.published_text,
         duration:       details.length_seconds,
         thumbnail:      format!("{}/vi/{}/hqdefault.jpg", self.base_url, details.video_id),
         view_count:     details.view_count,
         description:    Some(details.description).filter(|d| !d.is_empty()),
      })
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

      let channel_data: InvidiousChannelVideosResponseRaw = response
         .json()
         .await
         .map_err(|_| AppError::InvidiousBadResponse)?;

      let mut resolved = Vec::new();
      let mut fallback_ids = Vec::new();
      for value in channel_data.videos {
         if let Ok(video) = serde_json::from_value::<InvidiousVideoRaw>(value.clone()) {
            resolved.push(YoutubeVideo {
               title:          video.title,
               video_id:       video.video_id.clone(),
               author:         video.author,
               author_id:      video.author_id,
               published:      video.published,
               published_text: video.published_text,
               duration:       video.length_seconds,
               thumbnail:      format!("{}/vi/{}/hqdefault.jpg", self.base_url, video.video_id),
               view_count:     video.view_count,
               description:    Some(video.description).filter(|d| !d.is_empty()),
            });
            continue;
         }

         if let Some(video_id) = extract_channel_playlist_fallback_id(&value) {
            fallback_ids.push(video_id);
         }
      }

      let fallback_videos = stream::iter(fallback_ids)
         .map(|video_id| async move { self.fetch_video_fallback(&video_id).await })
         .buffer_unordered(FALLBACK_FETCH_CONCURRENCY)
         .collect::<Vec<_>>()
         .await;
      resolved.extend(fallback_videos.into_iter().flatten());

      Ok(resolved)
   }

   /// Get authenticated user's recent videos from subscription feed
   pub async fn get_recent_videos(
      &self,
      max_results: Option<u32>,
   ) -> Result<Vec<YoutubeVideo>, AppError> {
      let max = max_results.unwrap_or(25).min(RECENT_VIDEOS_MAX_RESULTS);
      let url = format!(
         "{}/api/v1/auth/feed?max_results={}",
         self.base_url, RECENT_VIDEOS_MAX_RESULTS
      );

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

            let fallback_ids: Vec<String> = videos
               .iter()
               .filter(|video| video.duration <= 0)
               .map(|video| video.video_id.clone())
               .collect();

            let fallback_results = stream::iter(fallback_ids)
               .map(|video_id| {
                  async move {
                     let resolved = self.fetch_video_fallback(&video_id).await;
                     (video_id, resolved)
                  }
               })
               .buffer_unordered(FALLBACK_FETCH_CONCURRENCY)
               .collect::<Vec<_>>()
               .await;

            let fallback_durations = build_fallback_duration_map(fallback_results);
            apply_fallback_durations(&mut videos, &fallback_durations);

            videos.sort_by(|left, right| {
               right
                  .published
                  .cmp(&left.published)
                  .then_with(|| left.video_id.cmp(&right.video_id))
            });

            let mut seen_video_ids = HashSet::new();
            videos.retain(|video| seen_video_ids.insert(video.video_id.clone()));

            videos.truncate(max as usize);
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

   fn fallback_test_video(id: &str) -> YoutubeVideo {
      YoutubeVideo {
         title:          id.to_string(),
         video_id:       id.to_string(),
         author:         "author".to_string(),
         author_id:      "author-id".to_string(),
         published:      1,
         published_text: "now".to_string(),
         duration:       60,
         thumbnail:      "thumb".to_string(),
         view_count:     1,
         description:    None,
      }
   }

   fn cached_fallback_entry(
      video: Option<YoutubeVideo>,
      fetched_at_unix: u64,
      last_accessed_unix: u64,
   ) -> CachedVideoLookup {
      CachedVideoLookup {
         video,
         fetched_at_unix,
         last_accessed_unix,
      }
   }

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
      assert!(!is_valid_channel_id(&format!("{base}w").replace('w', "!")));
      assert!(!is_valid_channel_id(&format!("{base}w").replace('w', "@")));
      assert!(!is_valid_channel_id(&format!("{base}w").replace('w', "#")));
      assert!(!is_valid_channel_id(&format!("{base}w").replace('w', "$")));
      assert!(!is_valid_channel_id(&format!("{base}w").replace('w', "%")));
      assert!(!is_valid_channel_id(&format!("{base}w").replace('w', "+")));
      assert!(!is_valid_channel_id(&format!("{base}w").replace('w', "=")));
      assert!(!is_valid_channel_id(&format!("{base}w").replace('w', " ")));
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

   #[test]
   fn test_extract_channel_playlist_fallback_id_accepts_only_11_char_playlist_id() {
      let value = serde_json::json!({"type": "playlist", "playlistId": "dQw4w9WgXcQ"});
      assert_eq!(
         extract_channel_playlist_fallback_id(&value),
         Some("dQw4w9WgXcQ".to_string())
      );

      let too_long =
         serde_json::json!({"type": "playlist", "playlistId": "PLxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"});
      assert_eq!(extract_channel_playlist_fallback_id(&too_long), None);
   }

   #[test]
   fn test_extract_channel_playlist_fallback_id_rejects_real_playlist_id_as_video() {
      let value =
         serde_json::json!({"type": "playlist", "playlistId": "PLynG1u0lH0xC3M7e2J7uRk2XwQ1zT9a"});
      assert_eq!(extract_channel_playlist_fallback_id(&value), None);
   }

   #[test]
   fn test_apply_fallback_durations_updates_only_zero_or_negative_entries() {
      let mut videos = vec![
         YoutubeVideo {
            title:          "a".to_string(),
            video_id:       "zero0000001".to_string(),
            author:         "author".to_string(),
            author_id:      "author-id".to_string(),
            published:      1,
            published_text: "now".to_string(),
            duration:       0,
            thumbnail:      "thumb".to_string(),
            view_count:     1,
            description:    None,
         },
         YoutubeVideo {
            title:          "b".to_string(),
            video_id:       "positive001".to_string(),
            author:         "author".to_string(),
            author_id:      "author-id".to_string(),
            published:      1,
            published_text: "now".to_string(),
            duration:       120,
            thumbnail:      "thumb".to_string(),
            view_count:     1,
            description:    None,
         },
         YoutubeVideo {
            title:          "c".to_string(),
            video_id:       "negdur00001".to_string(),
            author:         "author".to_string(),
            author_id:      "author-id".to_string(),
            published:      1,
            published_text: "now".to_string(),
            duration:       -1,
            thumbnail:      "thumb".to_string(),
            view_count:     1,
            description:    None,
         },
      ];

      let fallback = HashMap::from([
         ("zero0000001".to_string(), 45),
         ("positive001".to_string(), 777),
         ("negdur00001".to_string(), 90),
      ]);

      apply_fallback_durations(&mut videos, &fallback);

      assert_eq!(videos[0].duration, 45);
      assert_eq!(videos[1].duration, 120);
      assert_eq!(videos[2].duration, 90);
   }

   #[test]
   fn test_build_fallback_duration_map_ignores_missing_or_non_positive_durations() {
      let good = YoutubeVideo {
         title:          "a".to_string(),
         video_id:       "goodvideo01".to_string(),
         author:         "author".to_string(),
         author_id:      "author-id".to_string(),
         published:      1,
         published_text: "now".to_string(),
         duration:       60,
         thumbnail:      "thumb".to_string(),
         view_count:     1,
         description:    None,
      };
      let zero = YoutubeVideo {
         duration: 0,
         ..good.clone()
      };

      let map = build_fallback_duration_map(vec![
         ("goodvideo01".to_string(), Some(good)),
         ("zero0000001".to_string(), Some(zero)),
         ("none0000001".to_string(), None),
      ]);

      assert_eq!(map.get("goodvideo01"), Some(&60));
      assert!(!map.contains_key("zero0000001"));
      assert!(!map.contains_key("none0000001"));
   }

   #[test]
   fn test_negative_fallback_cache_ttl_boundaries() {
      assert!(is_negative_fallback_cache_fresh(
         FALLBACK_NEGATIVE_TTL_SECS - 1
      ));
      assert!(!is_negative_fallback_cache_fresh(
         FALLBACK_NEGATIVE_TTL_SECS
      ));
   }

   #[test]
   fn test_positive_fallback_cache_ttl_boundaries() {
      assert!(is_positive_fallback_cache_fresh(
         FALLBACK_POSITIVE_TTL_SECS - 1
      ));
      assert!(!is_positive_fallback_cache_fresh(
         FALLBACK_POSITIVE_TTL_SECS
      ));
   }

   #[test]
   fn test_prune_fallback_cache_entries_removes_expired_before_lru() {
      let now_unix = 10_000_000;
      let mut cache = HashMap::new();
      cache.insert(
         "expired_pos".to_string(),
         cached_fallback_entry(
            Some(fallback_test_video("expiredPos1")),
            now_unix - FALLBACK_POSITIVE_TTL_SECS,
            2,
         ),
      );
      cache.insert(
         "expired_neg".to_string(),
         cached_fallback_entry(None, now_unix - FALLBACK_NEGATIVE_TTL_SECS, 3),
      );

      for index in 0..=FALLBACK_CACHE_MAX_ENTRIES {
         let key = format!("fresh_{index:04}");
         let last_accessed_unix =
            u64::try_from(index).expect("fallback cache index should fit in u64");
         cache.insert(
            key.clone(),
            cached_fallback_entry(
               Some(fallback_test_video(&key)),
               now_unix - 60,
               last_accessed_unix,
            ),
         );
      }

      prune_fallback_cache_entries(&mut cache, now_unix);

      assert!(!cache.contains_key("expired_pos"));
      assert!(!cache.contains_key("expired_neg"));
      assert_eq!(cache.len(), FALLBACK_CACHE_MAX_ENTRIES);
      assert!(!cache.contains_key("fresh_0000"));
      assert!(cache.contains_key("fresh_0001"));
   }
}
