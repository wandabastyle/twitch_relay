use std::{
   collections::{
      HashMap,
      HashSet,
   },
   fmt::Write,
   sync::Arc,
   time::{
      Duration,
      Instant,
   },
};

use tokio::sync::RwLock;

use super::resolver::{
   fetch_and_parse_manifest,
   fetch_text,
   get_hls_url_streamlink,
   is_allowed_quality,
   quality_frame_rate,
   quality_info,
   quality_sort_rank,
   rewrite_manifest_urls,
   sort_qualities,
};
use crate::config::{
   StreamDeliveryMode,
   StreamResolverMode,
};

#[derive(Debug, Clone)]
pub struct StreamProxyState {
   pub service: StreamSessionService,
}

#[derive(Debug, Clone)]
pub struct StreamSessionService {
   sessions:         Arc<RwLock<HashMap<String, StreamSession>>>,
   prewarmed:        Arc<RwLock<HashMap<String, PrewarmedEntry>>>,
   prewarm_inflight: Arc<RwLock<HashSet<String>>>,
   streamlink_path:  String,
   resolver_mode:    StreamResolverMode,
   delivery_mode:    StreamDeliveryMode,
   twitch_client_id: String,
   client:           reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct QualityVariant {
   pub manifest_url:   String,
   pub segment_lookup: HashMap<String, String>,
   pub cdn_base:       String,
   pub bandwidth:      Option<u64>,
   pub width:          Option<u32>,
   pub height:         Option<u32>,
   pub frame_rate:     Option<f32>,
}

#[derive(Debug, Clone)]
pub struct StreamSession {
   pub session_token:         String,
   pub variants:              HashMap<String, QualityVariant>,
   pub resolver:              StreamResolverMode,
   pub logged_delivery_modes: HashSet<String>,
}

#[derive(Debug, Clone)]
struct PrewarmedEntry {
   variants:               HashMap<String, QualityVariant>,
   resolver:               StreamResolverMode,
   warmed_at:              Instant,
   validated_at:           Instant,
   validation_jitter_secs: u64,
}

/// Hard TTL - entries older than this are considered expired
const PREWARM_TTL_SECS: u64 = 90;
/// Validation interval - check validity every 15 minutes + jitter
const PREWARM_VALIDATE_AFTER_SECS: u64 = 900;
/// Maximum jitter in seconds to spread validation load
const PREWARM_JITTER_MAX_SECS: u64 = 120;
const PREWARM_MAX_CHANNELS: usize = 20;
const PREWARM_POOL_QUALITIES: [&str; 5] = ["source", "1080p60", "720p60", "480p", "360p"];
const PREWARM_POOL_CONCURRENCY: usize = 3;

/// Compute deterministic jitter (`0..PREWARM_JITTER_MAX_SECS`) from channel
/// login
fn compute_validation_jitter(channel: &str) -> u64 {
   channel
      .as_bytes()
      .iter()
      .fold(0u64, |a, b| a.wrapping_add(u64::from(*b)))
      .wrapping_rem(PREWARM_JITTER_MAX_SECS)
}

use super::StreamError;

impl StreamProxyState {
   pub const fn new(service: StreamSessionService) -> Self {
      Self { service }
   }
}

impl StreamSessionService {
   pub fn new(
      streamlink_path: String,
      resolver_mode: StreamResolverMode,
      delivery_mode: StreamDeliveryMode,
      twitch_client_id: String,
   ) -> Self {
      Self {
         sessions: Arc::new(RwLock::new(HashMap::new())),
         prewarmed: Arc::new(RwLock::new(HashMap::new())),
         prewarm_inflight: Arc::new(RwLock::new(HashSet::new())),
         streamlink_path,
         resolver_mode,
         delivery_mode,
         twitch_client_id,
         client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new()),
      }
   }

   pub async fn open_session(
      &self,
      stream_id: &str,
      channel: &str,
      session_token: &str,
      quality: &str,
   ) -> Result<(), StreamError> {
      {
         let guard = self.sessions.read().await;
         if let Some(existing) = guard.get(stream_id)
            && existing.session_token == session_token
         {
            return Ok(());
         }
      }

      if let Some(prewarmed) = self.get_prewarmed(channel).await {
         let session = StreamSession {
            session_token:         session_token.to_string(),
            variants:              prewarmed.variants,
            resolver:              prewarmed.resolver,
            logged_delivery_modes: HashSet::new(),
         };

         tracing::info!(
             stream_id = %stream_id,
             channel = %channel,
             resolver = ?session.resolver,
             available_qualities = ?session.variants.keys().collect::<Vec<_>>(),
             warm_cache_hit = true,
             "opened stream session"
         );

         self
            .sessions
            .write()
            .await
            .insert(stream_id.to_string(), session);
         return Ok(());
      }

      let (variants, resolver) = self.resolve_variants(channel, quality).await?;

      if variants.is_empty() {
         return Err(StreamError::HlsFetchFailed(
            "No qualities available for channel".to_string(),
         ));
      }

      let session = StreamSession {
         session_token: session_token.to_string(),
         variants,
         resolver,
         logged_delivery_modes: HashSet::new(),
      };

      tracing::info!(
          stream_id = %stream_id,
          channel = %channel,
          resolver = ?resolver,
          available_qualities = ?session.variants.keys().collect::<Vec<_>>(),
          "opened stream session"
      );

      self
         .sessions
         .write()
         .await
         .insert(stream_id.to_string(), session);
      Ok(())
   }

   pub async fn prewarm_channel_if_needed(&self, channel: &str) {
      if !self.prewarm_enabled() {
         return;
      }

      if self.has_fresh_prewarm(channel).await {
         return;
      }

      if !self.mark_prewarm_inflight(channel).await {
         return;
      }

      let service = self.clone();
      let channel = channel.to_string();
      tokio::spawn(async move {
         tracing::debug!(channel = %channel, "stream prewarm started");
         match service.resolve_prewarm_pool(&channel).await {
            Ok(variants) => {
               let mut discovered_qualities: Vec<String> = variants.keys().cloned().collect();
               discovered_qualities.sort_by(|a, b| {
                  quality_sort_rank(a.as_str()).cmp(&quality_sort_rank(b.as_str()))
               });
               let now = Instant::now();
               service
                  .put_prewarmed(&channel, PrewarmedEntry {
                     variants,
                     resolver: StreamResolverMode::Streamlink,
                     warmed_at: now,
                     validated_at: now,
                     validation_jitter_secs: compute_validation_jitter(&channel),
                  })
                  .await;
               tracing::debug!(
                   channel = %channel,
                   pooled_qualities = ?PREWARM_POOL_QUALITIES,
                   discovered_qualities = ?discovered_qualities,
                   "stream prewarm completed"
               );
               tracing::info!(
                   channel = %channel,
                   discovered_qualities = ?discovered_qualities,
                   discovered_count = discovered_qualities.len(),
                   "stream prewarm completed"
               );
            },
            Err(error) => {
               tracing::debug!(
                   channel = %channel,
                   pooled_qualities = ?PREWARM_POOL_QUALITIES,
                   error = ?error,
                   "stream prewarm skipped"
               );
            },
         }
         service.clear_prewarm_inflight(&channel).await;
      });
   }

   pub async fn get_variant_manifest(
      &self,
      stream_id: &str,
      session_token: &str,
      quality: &str,
      force_relay: bool,
   ) -> Result<String, StreamError> {
      if !is_allowed_quality(quality) {
         tracing::debug!(stream_id = %stream_id, quality = %quality, "rejected disallowed quality request");
         return Err(StreamError::StreamNotFound);
      }

      let guard = self.sessions.read().await;
      let Some(session) = guard.get(stream_id) else {
         return Err(StreamError::StreamNotFound);
      };
      if session.session_token != session_token {
         drop(guard);
         return Err(StreamError::SessionMismatch);
      }
      let maybe_variant = session.variants.get(quality).cloned();
      drop(guard);

      let variant = if let Some(variant) = maybe_variant {
         variant
      } else {
         self
            .resolve_and_store_quality(stream_id, session_token, quality)
            .await?
      };

      let manifest_text = fetch_text(&variant.manifest_url)
         .await
         .map_err(StreamError::HlsFetchFailed)?;

      let (segment_lookup, cdn_base) = parse_segment_lookup(&manifest_text);
      self
         .refresh_variant_lookup(stream_id, session_token, quality, segment_lookup, cdn_base)
         .await;

      let rewritten = rewrite_manifest_urls(
         &manifest_text,
         stream_id,
         session_token,
         quality,
         force_relay,
      );

      Ok(rewritten)
   }

   pub async fn get_multi_level_manifest(
      &self,
      stream_id: &str,
      session_token: &str,
      force_relay: bool,
   ) -> Result<String, StreamError> {
      let session = self.get_session(stream_id, session_token).await?;

      let mut manifest_lines = vec!["#EXTM3U".to_string(), "#EXT-X-VERSION:3".to_string()];
      let relay_suffix = if force_relay { "?relay=1" } else { "" };
      let mut seen_manifest_urls = HashSet::new();

      let ordered_qualities = sort_qualities(session.variants.keys());
      for quality in ordered_qualities {
         if !is_allowed_quality(quality) {
            continue;
         }
         let Some(variant) = session.variants.get(quality) else {
            continue;
         };
         if !seen_manifest_urls.insert(variant.manifest_url.clone()) {
            continue;
         }
         let name = match quality {
            "source" => "Source",
            q => q,
         };

         let bandwidth = variant.bandwidth.unwrap_or_else(|| quality_info(quality).0);
         let width = variant.width.unwrap_or_else(|| quality_info(quality).1);
         let height = variant.height.unwrap_or_else(|| quality_info(quality).2);
         let frame_rate = variant.frame_rate.or_else(|| quality_frame_rate(quality));

         let mut stream_inf = format!(
            "#EXT-X-STREAM-INF:BANDWIDTH={bandwidth},RESOLUTION={width}x{height},NAME=\"{name}\""
         );
         if let Some(frame_rate) = frame_rate {
            let _ = write!(stream_inf, ",FRAME-RATE={frame_rate:.3}");
         }
         manifest_lines.push(stream_inf);
         manifest_lines.push(format!(
            "/stream/{stream_id}/{session_token}/manifest/{quality}{relay_suffix}"
         ));
      }

      Ok(manifest_lines.join("\n"))
   }

   pub async fn resolve_segment(
      &self,
      stream_id: &str,
      quality: &str,
      segment_name: &str,
      session_token: &str,
   ) -> Result<(String, StreamResolverMode), StreamError> {
      let session = self.get_session(stream_id, session_token).await?;

      let variant = session
         .variants
         .get(quality)
         .ok_or(StreamError::StreamNotFound)?;

      let cdn_url = if let Some(exact_url) = variant.segment_lookup.get(segment_name) {
         exact_url.clone()
      } else if !variant.cdn_base.is_empty() {
         format!("{}/segment/{}", variant.cdn_base, segment_name)
      } else {
         return Err(StreamError::StreamNotFound);
      };

      Ok((cdn_url, session.resolver))
   }

   async fn resolve_prewarm_pool(
      &self,
      channel: &str,
   ) -> Result<HashMap<String, QualityVariant>, StreamError> {
      let mut variants = HashMap::new();

      for group in PREWARM_POOL_QUALITIES.chunks(PREWARM_POOL_CONCURRENCY) {
         let mut handles = Vec::with_capacity(group.len());
         for quality in group {
            let quality = (*quality).to_string();
            let channel = channel.to_string();
            let streamlink_path = self.streamlink_path.clone();
            handles.push(tokio::spawn(async move {
               let quality_arg = if quality == "source" {
                  "best"
               } else {
                  quality.as_str()
               };
               let manifest_url =
                  get_hls_url_streamlink(&channel, &streamlink_path, quality_arg).await;
               (quality, manifest_url)
            }));
         }

         for handle in handles {
            let Ok((quality, manifest_result)) = handle.await else {
               continue;
            };

            match manifest_result {
               Ok(manifest_url) => {
                  match fetch_and_parse_manifest(&manifest_url).await {
                     Ok((lookup, cdn_base)) => {
                        let variant = QualityVariant {
                           manifest_url,
                           segment_lookup: lookup,
                           cdn_base,
                           bandwidth: quality_info(quality.as_str()).0.into(),
                           width: Some(quality_info(quality.as_str()).1),
                           height: Some(quality_info(quality.as_str()).2),
                           frame_rate: quality_frame_rate(quality.as_str()),
                        };
                        tracing::debug!(channel = %channel, quality = %quality, "prewarm quality resolved");
                        variants.insert(quality, variant);
                     },
                     Err(error) => {
                        tracing::debug!(channel = %channel, quality = %quality, error = %error, "prewarm quality parse failed");
                     },
                  }
               },
               Err(error) => {
                  tracing::debug!(channel = %channel, quality = %quality, error = %error, "prewarm quality not available");
               },
            }
         }
      }

      if variants.is_empty() {
         return Err(StreamError::HlsFetchFailed(
            "No pooled qualities available for channel".to_string(),
         ));
      }

      Ok(variants)
   }

   async fn resolve_variants(
      &self,
      channel: &str,
      quality: &str,
   ) -> Result<(HashMap<String, QualityVariant>, StreamResolverMode), StreamError> {
      match self.resolver_mode {
         StreamResolverMode::Native => {
            self
               .resolve_with_native(channel, quality)
               .await
               .map(|variants| (variants, StreamResolverMode::Native))
         },
         StreamResolverMode::Streamlink => {
            self
               .resolve_with_streamlink(channel, quality)
               .await
               .map(|variants| (variants, StreamResolverMode::Streamlink))
         },
         StreamResolverMode::Auto => {
            match self.resolve_with_native(channel, quality).await {
               Ok(variants) => {
                  tracing::info!(channel = %channel, resolver = "native", "resolved stream variants");
                  Ok((variants, StreamResolverMode::Native))
               },
               Err(native_err) => {
                  tracing::warn!(
                      channel = %channel,
                      resolver = "native",
                      error = ?native_err,
                      "native resolver failed, falling back to streamlink"
                  );
                  self
                     .resolve_with_streamlink(channel, quality)
                     .await
                     .map(|variants| (variants, StreamResolverMode::Streamlink))
               },
            }
         },
      }
   }

   async fn resolve_with_native(
      &self,
      channel: &str,
      quality: &str,
   ) -> Result<HashMap<String, QualityVariant>, StreamError> {
      let (master_manifest_url, master_manifest_text) =
         super::resolver::fetch_native_master_manifest(
            &self.client,
            channel,
            &self.twitch_client_id,
         )
         .await
         .map_err(StreamError::HlsFetchFailed)?;

      let variants = super::resolver::select_native_variants(
         &master_manifest_url,
         &master_manifest_text,
         quality,
      )
      .map_err(StreamError::HlsFetchFailed)?;

      if variants.is_empty() {
         return Err(StreamError::HlsFetchFailed(
            "No qualities available for channel".to_string(),
         ));
      }

      let mut out = HashMap::new();
      for variant_meta in variants {
         let quality_label = variant_meta.quality.clone();
         let manifest_url = variant_meta.manifest_url.clone();
         match fetch_and_parse_manifest(&manifest_url).await {
            Ok((lookup, cdn_base)) => {
               out.insert(quality_label, QualityVariant {
                  manifest_url,
                  segment_lookup: lookup,
                  cdn_base,
                  bandwidth: Some(variant_meta.bandwidth),
                  width: variant_meta.width,
                  height: variant_meta.height,
                  frame_rate: variant_meta.frame_rate,
               });
            },
            Err(e) => {
               tracing::warn!(channel = %channel, error = %e, "failed to parse native variant manifest");
            },
         }

         if out.len() >= 4 {
            break;
         }
      }

      if out.is_empty() {
         return Err(StreamError::HlsFetchFailed(
            "No qualities available for channel".to_string(),
         ));
      }

      tracing::info!(channel = %channel, resolver = "native", "resolved stream variants");
      Ok(out)
   }

   async fn resolve_with_streamlink(
      &self,
      channel: &str,
      quality: &str,
   ) -> Result<HashMap<String, QualityVariant>, StreamError> {
      if quality != "best" && !is_allowed_quality(quality) {
         return Err(StreamError::HlsFetchFailed(format!(
            "quality not allowed: {quality}"
         )));
      }

      let mut qualities_to_fetch: Vec<&str> = if quality == "best" {
         vec!["source", "1080p60", "720p60", "480p", "360p"]
      } else {
         vec![quality, "source", "1080p60", "720p60", "480p", "360p"]
      };
      qualities_to_fetch.dedup();

      let mut variants = HashMap::new();

      for q in &qualities_to_fetch {
         let streamlink_quality = if *q == "source" { "best" } else { q };
         match get_hls_url_streamlink(channel, &self.streamlink_path, streamlink_quality).await {
            Ok(manifest_url) => {
               let label = q.to_string();
               if variants.contains_key(&label) {
                  continue;
               }

               match fetch_and_parse_manifest(&manifest_url).await {
                  Ok((lookup, cdn_base)) => {
                     let variant = QualityVariant {
                        manifest_url: manifest_url.clone(),
                        segment_lookup: lookup,
                        cdn_base,
                        bandwidth: quality_info(label.as_str()).0.into(),
                        width: Some(quality_info(label.as_str()).1),
                        height: Some(quality_info(label.as_str()).2),
                        frame_rate: quality_frame_rate(label.as_str()),
                     };

                     variants.insert(label, variant);
                     if variants.len() >= 4 {
                        break;
                     }
                  },
                  Err(e) => {
                     tracing::warn!(channel = %channel, quality = %q, error = %e, "failed to parse manifest for quality");
                  },
               }
            },
            Err(e) => {
               tracing::debug!(channel = %channel, quality = %q, error = %e, "quality not available");
            },
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

      tracing::info!(channel = %channel, resolver = "streamlink", "resolved stream variants");
      Ok(variants)
   }

   pub const fn should_redirect_to_cdn(&self, force_relay: bool) -> bool {
      matches!(self.delivery_mode, StreamDeliveryMode::CdnFirst) && !force_relay
   }

   const fn prewarm_enabled(&self) -> bool {
      matches!(self.resolver_mode, StreamResolverMode::Streamlink)
   }

   async fn has_fresh_prewarm(&self, channel: &str) -> bool {
      let guard = self.prewarmed.read().await;
      guard
         .get(channel)
         .is_some_and(|entry| entry.warmed_at.elapsed() < Duration::from_secs(PREWARM_TTL_SECS))
   }

   /// Returns a prewarmed entry if available, regardless of age.
   /// For fast playback path - background maintenance handles validation.
   async fn get_prewarmed(&self, channel: &str) -> Option<PrewarmedEntry> {
      let guard = self.prewarmed.read().await;
      guard.get(channel).cloned()
   }

   async fn put_prewarmed(&self, channel: &str, entry: PrewarmedEntry) {
      let mut guard = self.prewarmed.write().await;
      if guard.len() >= PREWARM_MAX_CHANNELS
         && !guard.contains_key(channel)
         && let Some((oldest_key, _)) = guard
            .iter()
            .min_by_key(|(_, value)| value.warmed_at)
            .map(|(key, value)| (key.clone(), value.warmed_at))
      {
         guard.remove(&oldest_key);
      }
      guard.insert(channel.to_string(), entry);
   }

   /// Background validation of a prewarm entry.
   /// Fetches one representative manifest URL to check validity.
   /// On success: updates `validated_at` timestamp.
   /// On failure: triggers full prewarm refresh.
   async fn validate_prewarm_entry(&self, channel: &str) {
      let entry = {
         let guard = self.prewarmed.read().await;
         guard.get(channel).cloned()
      };

      let Some(entry) = entry else {
         tracing::debug!(channel = %channel, "prewarm validation: entry not found");
         return;
      };

      // Mark validation as in-flight to prevent duplicate validations
      if !self.mark_validation_inflight(channel).await {
         tracing::debug!(channel = %channel, "prewarm validation already in flight");
         return;
      }

      tracing::info!(
         channel = %channel,
         prewarm_age_secs = entry.warmed_at.elapsed().as_secs(),
         validated_age_secs = entry.validated_at.elapsed().as_secs(),
         validation_jitter_secs = entry.validation_jitter_secs,
         "prewarm_validation_started"
      );

      // Get first available variant URL for validation
      let Some((quality, variant)) = entry.variants.iter().next() else {
         tracing::warn!(channel = %channel, "prewarm validation: no variants available");
         self.clear_validation_inflight(channel).await;
         // Remove stale entry and trigger refresh
         self.remove_prewarmed(channel).await;
         self.prewarm_channel_if_needed(channel).await;
         return;
      };

      let manifest_url = variant.manifest_url.clone();
      let service = self.clone();
      let channel = channel.to_string();
      let quality = quality.clone();

      // Spawn non-blocking validation task
      tokio::spawn(async move {
         match service.perform_manifest_validation(&manifest_url).await {
            Ok(()) => {
               // Validation succeeded - update validated_at timestamp
               let mut guard = service.prewarmed.write().await;
               if let Some(entry) = guard.get_mut(&channel) {
                  entry.validated_at = Instant::now();
                  tracing::info!(
                     channel = %channel,
                     quality = %quality,
                     "prewarm_validation_succeeded"
                  );
               }
               drop(guard);
               service.clear_validation_inflight(&channel).await;
            },
            Err(error) => {
               tracing::warn!(
                  channel = %channel,
                  quality = %quality,
                  error = %error,
                  "prewarm_validation_failed"
               );
               service.clear_validation_inflight(&channel).await;
               // Remove invalid entry and trigger full refresh
               service.remove_prewarmed(&channel).await;
               service.prewarm_channel_if_needed(&channel).await;
            },
         }
      });
   }

   /// Performs actual HTTP validation of a manifest URL
   async fn perform_manifest_validation(&self, manifest_url: &str) -> Result<(), StreamError> {
      let response = self
         .client
         .get(manifest_url)
         .send()
         .await
         .map_err(|e| StreamError::HlsFetchFailed(format!("validation request failed: {e}")))?;

      if !response.status().is_success() {
         return Err(StreamError::HlsFetchFailed(format!(
            "validation returned HTTP {}",
            response.status()
         )));
      }

      let body = response
         .text()
         .await
         .map_err(|e| StreamError::HlsFetchFailed(format!("validation read failed: {e}")))?;

      if !body.contains("#EXTM3U") {
         return Err(StreamError::HlsFetchFailed(
            "validation failed: response missing #EXTM3U".to_string(),
         ));
      }

      Ok(())
   }

   /// Maintenance method for live channels.
   /// - If no entry: start prewarm
   /// - If entry age < 240s + jitter: do nothing
   /// - If due for validation: start background validation
   pub async fn maintain_prewarm_for_live_channel(&self, channel: &str) {
      if !self.prewarm_enabled() {
         return;
      }

      // Check if entry exists
      let entry_info = {
         let guard = self.prewarmed.read().await;
         guard.get(channel).map(|e| {
            let warmed_age = e.warmed_at.elapsed();
            let validated_age = e.validated_at.elapsed();
            let interval =
               Duration::from_secs(PREWARM_VALIDATE_AFTER_SECS + e.validation_jitter_secs);
            (
               warmed_age,
               validated_age,
               interval,
               e.validation_jitter_secs,
            )
         })
      };

      match entry_info {
         None => {
            // No entry - start prewarm
            tracing::debug!(channel = %channel, "prewarm maintenance: no entry, starting prewarm");
            self.prewarm_channel_if_needed(channel).await;
         },
         Some((warmed_age, validated_age, interval, jitter_secs)) => {
            // Check if due for validation
            if validated_age >= interval {
               tracing::debug!(
                  channel = %channel,
                  prewarm_age_secs = warmed_age.as_secs(),
                  validated_age_secs = validated_age.as_secs(),
                  validation_jitter_secs = jitter_secs,
                  "prewarm maintenance: validation due"
               );
               self.validate_prewarm_entry(channel).await;
            } else {
               let next_secs = interval
                  .checked_sub(validated_age)
                  .map_or(0, |d| d.as_secs());
               tracing::debug!(
                  channel = %channel,
                  validated_age_secs = validated_age.as_secs(),
                  next_validation_secs = next_secs,
                  "prewarm maintenance: validation not yet due"
               );
            }
         },
      }
   }

   /// Remove prewarm entry for an offline channel
   pub async fn drop_prewarm_for_offline_channel(&self, channel: &str) {
      let removed = self.remove_prewarmed(channel).await;
      if removed {
         tracing::info!(channel = %channel, "prewarm_dropped_offline");
      }
   }

   /// Remove a prewarm entry and return whether it existed
   async fn remove_prewarmed(&self, channel: &str) -> bool {
      let mut guard = self.prewarmed.write().await;
      guard.remove(channel).is_some()
   }

   async fn mark_validation_inflight(&self, channel: &str) -> bool {
      let mut guard = self.prewarm_inflight.write().await;
      guard.insert(format!("validate:{channel}"))
   }

   async fn clear_validation_inflight(&self, channel: &str) {
      self
         .prewarm_inflight
         .write()
         .await
         .remove(&format!("validate:{channel}"));
   }

   async fn mark_prewarm_inflight(&self, channel: &str) -> bool {
      let mut guard = self.prewarm_inflight.write().await;
      guard.insert(channel.to_string())
   }

   async fn clear_prewarm_inflight(&self, channel: &str) {
      let mut guard = self.prewarm_inflight.write().await;
      guard.remove(channel);
   }

   async fn resolve_and_store_quality(
      &self,
      stream_id: &str,
      session_token: &str,
      quality: &str,
   ) -> Result<QualityVariant, StreamError> {
      if !is_allowed_quality(quality) {
         tracing::debug!(stream_id = %stream_id, quality = %quality, "lazy quality resolve rejected");
         return Err(StreamError::StreamNotFound);
      }

      tracing::debug!(stream_id = %stream_id, quality = %quality, "lazy quality resolve started");
      let guard = self.sessions.read().await;
      let Some(session) = guard.get(stream_id) else {
         return Err(StreamError::StreamNotFound);
      };
      if session.session_token != session_token {
         return Err(StreamError::SessionMismatch);
      }
      let Some((_, first_variant)) = session.variants.iter().next() else {
         return Err(StreamError::StreamNotFound);
      };
      let channel = super::resolver::infer_channel_from_manifest_url(&first_variant.manifest_url)
         .ok_or_else(|| {
         StreamError::HlsFetchFailed("unable to infer channel for quality resolve".to_string())
      })?;
      drop(guard);

      let streamlink_quality = if quality == "source" { "best" } else { quality };
      let manifest_url = get_hls_url_streamlink(&channel, &self.streamlink_path, streamlink_quality)
            .await
            .map_err(|error| {
                tracing::debug!(stream_id = %stream_id, channel = %channel, quality = %quality, error = %error, "lazy quality resolve failed");
                StreamError::HlsFetchFailed(error)
            })?;
      let (lookup, cdn_base) = fetch_and_parse_manifest(&manifest_url)
            .await
            .map_err(|error| {
                tracing::debug!(stream_id = %stream_id, channel = %channel, quality = %quality, error = %error, "lazy quality resolve failed");
                StreamError::HlsFetchFailed(error)
            })?;
      let variant = QualityVariant {
         manifest_url,
         segment_lookup: lookup,
         cdn_base,
         bandwidth: quality_info(quality).0.into(),
         width: Some(quality_info(quality).1),
         height: Some(quality_info(quality).2),
         frame_rate: quality_frame_rate(quality),
      };

      {
         let mut guard = self.sessions.write().await;
         let Some(session) = guard.get_mut(stream_id) else {
            drop(guard);
            return Err(StreamError::StreamNotFound);
         };
         if session.session_token != session_token {
            drop(guard);
            return Err(StreamError::SessionMismatch);
         }
         session
            .variants
            .insert(quality.to_string(), variant.clone());
         drop(guard);
      }
      tracing::debug!(stream_id = %stream_id, channel = %channel, quality = %quality, "lazy quality resolve completed");
      Ok(variant)
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

      let result = session.clone();
      drop(guard);
      Ok(result)
   }

   async fn refresh_variant_lookup(
      &self,
      stream_id: &str,
      session_token: &str,
      quality: &str,
      segment_lookup: HashMap<String, String>,
      cdn_base: String,
   ) {
      let mut guard = self.sessions.write().await;
      let Some(session) = guard.get_mut(stream_id) else {
         return;
      };
      if session.session_token != session_token {
         return;
      }
      let Some(variant) = session.variants.get_mut(quality) else {
         return;
      };
      variant.segment_lookup = segment_lookup;
      variant.cdn_base = cdn_base;
      drop(guard);
   }

   pub async fn mark_delivery_logged_once(
      &self,
      stream_id: &str,
      quality: &str,
      delivery: &str,
   ) -> bool {
      let mut guard = self.sessions.write().await;
      let Some(session) = guard.get_mut(stream_id) else {
         return false;
      };

      let key = format!("{quality}:{delivery}");
      let result = session.logged_delivery_modes.insert(key);
      drop(guard);
      result
   }
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

// ====================================================================
// Tests
// ====================================================================

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn compute_validation_jitter_returns_same_value_for_same_channel() {
      let jitter1 = compute_validation_jitter("testchannel");
      let jitter2 = compute_validation_jitter("testchannel");
      assert_eq!(jitter1, jitter2, "jitter should be deterministic");
   }

   #[test]
   fn compute_validation_jitter_returns_different_values_for_different_channels() {
      // Different channel names should generally produce different jitter values
      // (not guaranteed but highly likely given the hash function)
      let channels = ["channel1", "channel2", "channel3", "channel4", "channel5"];
      let jitters: Vec<u64> = channels
         .iter()
         .map(|c| compute_validation_jitter(c))
         .collect();

      // Check all values are within bounds
      for jitter in &jitters {
         assert!(
            *jitter < PREWARM_JITTER_MAX_SECS,
            "jitter should be less than {PREWARM_JITTER_MAX_SECS}"
         );
      }

      // Check we have some variety (at least 3 different values among 5 channels)
      let unique: std::collections::HashSet<_> = jitters.iter().collect();
      assert!(
         unique.len() >= 3,
         "different channels should produce different jitters, got: {jitters:?}"
      );
   }

   #[test]
   fn compute_validation_jitter_bounds_are_correct() {
      let test_channels = [
         "",
         "a",
         "verylongchannelnamethatmightexceedsomehashalgorithm",
         "1234567890",
         "!@#$%^&*()",
      ];

      for channel in &test_channels {
         let jitter = compute_validation_jitter(channel);
         assert!(
            jitter < PREWARM_JITTER_MAX_SECS,
            "jitter for '{channel}' should be less than {PREWARM_JITTER_MAX_SECS}, got {jitter}"
         );
      }
   }

   #[test]
   fn prewarm_constants_are_reasonable() {
      // Validation interval should be longer than TTL
      const _: () = assert!(
         PREWARM_VALIDATE_AFTER_SECS > PREWARM_TTL_SECS,
         "validation interval should be longer than TTL"
      );
      // Jitter should be a reasonable fraction of validation interval
      const _: () = assert!(
         PREWARM_JITTER_MAX_SECS < PREWARM_VALIDATE_AFTER_SECS / 2,
         "jitter should be less than half the validation interval"
      );
   }
}
