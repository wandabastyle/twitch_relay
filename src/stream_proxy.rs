use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use reqwest::{Client, Url};
use serde::Deserialize;
use tokio::{process::Command, sync::RwLock};

#[derive(Debug, Clone)]
pub struct StreamProxyState {
    pub service: StreamSessionService,
}

#[derive(Debug, Clone)]
pub struct StreamSessionService {
    sessions: Arc<RwLock<HashMap<String, StreamSession>>>,
    prewarmed: Arc<RwLock<HashMap<String, PrewarmedEntry>>>,
    prewarm_inflight: Arc<RwLock<HashSet<String>>>,
    streamlink_path: String,
    resolver_mode: StreamResolverMode,
    delivery_mode: StreamDeliveryMode,
    twitch_client_id: String,
    client: Client,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum StreamResolverMode {
    Auto,
    Native,
    Streamlink,
}

#[derive(Debug, Clone, Copy)]
enum StreamDeliveryMode {
    CdnFirst,
    Relay,
}

impl StreamResolverMode {
    fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "native" => Self::Native,
            "streamlink" => Self::Streamlink,
            _ => Self::Auto,
        }
    }
}

impl StreamDeliveryMode {
    fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "relay" => Self::Relay,
            _ => Self::CdnFirst,
        }
    }
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
    pub resolver: StreamResolverMode,
    pub logged_delivery_modes: HashSet<String>,
}

#[derive(Debug, Clone)]
struct PrewarmedEntry {
    variants: HashMap<String, QualityVariant>,
    resolver: StreamResolverMode,
    warmed_at: Instant,
}

const PREWARM_TTL_SECS: u64 = 90;
const PREWARM_MAX_CHANNELS: usize = 20;
const PREWARM_POOL_QUALITIES: [&str; 5] = ["source", "1080p60", "720p60", "480p", "360p"];
const PREWARM_POOL_CONCURRENCY: usize = 2;

#[derive(Debug)]
pub enum StreamError {
    StreamNotFound,
    SessionMismatch,
    HlsFetchFailed(String),
}

#[derive(Debug, Deserialize, Default)]
pub struct RelayQuery {
    pub relay: Option<String>,
}

impl RelayQuery {
    pub fn force_relay(&self) -> bool {
        self.relay
            .as_deref()
            .map(|v| {
                let normalized = v.trim().to_ascii_lowercase();
                normalized == "1"
                    || normalized == "true"
                    || normalized == "yes"
                    || normalized == "on"
            })
            .unwrap_or(false)
    }
}

impl StreamProxyState {
    pub fn new(service: StreamSessionService) -> Self {
        Self { service }
    }
}

impl StreamSessionService {
    pub fn new(
        streamlink_path: String,
        resolver_mode: String,
        delivery_mode: String,
        twitch_client_id: String,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            prewarmed: Arc::new(RwLock::new(HashMap::new())),
            prewarm_inflight: Arc::new(RwLock::new(HashSet::new())),
            streamlink_path,
            resolver_mode: StreamResolverMode::from_env_value(&resolver_mode),
            delivery_mode: StreamDeliveryMode::from_env_value(&delivery_mode),
            twitch_client_id,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
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

        if let Some(prewarmed) = self.take_prewarmed(channel).await {
            let session = StreamSession {
                session_token: session_token.to_string(),
                variants: prewarmed.variants,
                resolver: prewarmed.resolver,
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

            let mut guard = self.sessions.write().await;
            guard.insert(stream_id.to_string(), session);
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

        let mut guard = self.sessions.write().await;
        guard.insert(stream_id.to_string(), session);
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
                    service
                        .put_prewarmed(
                            &channel,
                            PrewarmedEntry {
                                variants,
                                resolver: StreamResolverMode::Streamlink,
                                warmed_at: Instant::now(),
                            },
                        )
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
                }
                Err(error) => {
                    tracing::debug!(
                        channel = %channel,
                        pooled_qualities = ?PREWARM_POOL_QUALITIES,
                        error = ?error,
                        "stream prewarm skipped"
                    );
                }
            }
            service.clear_prewarm_inflight(&channel).await;
        });
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
                    Ok(manifest_url) => match fetch_and_parse_manifest(&manifest_url).await {
                        Ok((lookup, cdn_base)) => {
                            let variant = QualityVariant {
                                manifest_url,
                                segment_lookup: lookup,
                                cdn_base,
                            };
                            tracing::debug!(channel = %channel, quality = %quality, "prewarm quality resolved");
                            variants.insert(quality, variant);
                        }
                        Err(error) => {
                            tracing::debug!(channel = %channel, quality = %quality, error = %error, "prewarm quality parse failed");
                        }
                    },
                    Err(error) => {
                        tracing::debug!(channel = %channel, quality = %quality, error = %error, "prewarm quality not available");
                    }
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
            StreamResolverMode::Native => self
                .resolve_with_native(channel, quality)
                .await
                .map(|variants| (variants, StreamResolverMode::Native)),
            StreamResolverMode::Streamlink => self
                .resolve_with_streamlink(channel, quality)
                .await
                .map(|variants| (variants, StreamResolverMode::Streamlink)),
            StreamResolverMode::Auto => match self.resolve_with_native(channel, quality).await {
                Ok(variants) => {
                    tracing::info!(channel = %channel, resolver = "native", "resolved stream variants");
                    Ok((variants, StreamResolverMode::Native))
                }
                Err(native_err) => {
                    tracing::warn!(
                        channel = %channel,
                        resolver = "native",
                        error = ?native_err,
                        "native resolver failed, falling back to streamlink"
                    );
                    self.resolve_with_streamlink(channel, quality)
                        .await
                        .map(|variants| (variants, StreamResolverMode::Streamlink))
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
            fetch_native_master_manifest(&self.client, channel, &self.twitch_client_id)
                .await
                .map_err(StreamError::HlsFetchFailed)?;

        let variants = select_native_variants(&master_manifest_url, &master_manifest_text, quality)
            .map_err(StreamError::HlsFetchFailed)?;

        if variants.is_empty() {
            return Err(StreamError::HlsFetchFailed(
                "No qualities available for channel".to_string(),
            ));
        }

        let mut out = HashMap::new();
        for (quality_label, manifest_url) in variants {
            match fetch_and_parse_manifest(&manifest_url).await {
                Ok((lookup, cdn_base)) => {
                    out.insert(
                        quality_label,
                        QualityVariant {
                            manifest_url: manifest_url.to_string(),
                            segment_lookup: lookup,
                            cdn_base,
                        },
                    );
                }
                Err(e) => {
                    tracing::warn!(channel = %channel, error = %e, "failed to parse native variant manifest");
                }
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

        let qualities_to_fetch = if quality == "best" {
            vec!["best", "source"]
        } else {
            vec![quality, "best"]
        };

        let mut variants = HashMap::new();

        for q in &qualities_to_fetch {
            match get_hls_url_streamlink(channel, &self.streamlink_path, q).await {
                Ok(manifest_url) => {
                    let label = if *q == "best" {
                        "source".to_string()
                    } else {
                        q.to_string()
                    };
                    if variants.contains_key(&label) {
                        continue;
                    }

                    match fetch_and_parse_manifest(&manifest_url).await {
                        Ok((lookup, cdn_base)) => {
                            let variant = QualityVariant {
                                manifest_url: manifest_url.clone(),
                                segment_lookup: lookup,
                                cdn_base,
                            };

                            variants.insert(label, variant);
                            if !variants.is_empty() {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(channel = %channel, quality = %q, error = %e, "failed to parse manifest for quality");
                        }
                    }
                }
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

        tracing::info!(channel = %channel, resolver = "streamlink", "resolved stream variants");
        Ok(variants)
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

        let maybe_variant = {
            let guard = self.sessions.read().await;
            let Some(session) = guard.get(stream_id) else {
                return Err(StreamError::StreamNotFound);
            };
            if session.session_token != session_token {
                return Err(StreamError::SessionMismatch);
            }
            session.variants.get(quality).cloned()
        };

        let variant = if let Some(variant) = maybe_variant {
            variant
        } else {
            self.resolve_and_store_quality(stream_id, session_token, quality)
                .await?
        };

        let manifest_text = fetch_text(&variant.manifest_url)
            .await
            .map_err(StreamError::HlsFetchFailed)?;

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
            let (bandwidth, width, height) = quality_info(quality);
            let name = match quality {
                "source" => "Auto",
                q => q,
            };

            manifest_lines.push(format!(
                "#EXT-X-STREAM-INF:BANDWIDTH={},RESOLUTION={}x{},NAME=\"{}\"",
                bandwidth, width, height, name
            ));
            manifest_lines.push(format!(
                "/stream/{}/{}/manifest/{}{}",
                stream_id, session_token, quality, relay_suffix
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

        let cdn_url = if variant.cdn_base.is_empty() {
            variant
                .segment_lookup
                .get(segment_name)
                .cloned()
                .ok_or(StreamError::StreamNotFound)?
        } else {
            format!("{}/segment/{}", variant.cdn_base, segment_name)
        };

        Ok((cdn_url, session.resolver))
    }

    fn should_redirect_to_cdn(&self, force_relay: bool) -> bool {
        matches!(self.delivery_mode, StreamDeliveryMode::CdnFirst) && !force_relay
    }

    fn prewarm_enabled(&self) -> bool {
        matches!(self.resolver_mode, StreamResolverMode::Streamlink)
    }

    async fn has_fresh_prewarm(&self, channel: &str) -> bool {
        let guard = self.prewarmed.read().await;
        guard
            .get(channel)
            .is_some_and(|entry| entry.warmed_at.elapsed() < Duration::from_secs(PREWARM_TTL_SECS))
    }

    async fn take_prewarmed(&self, channel: &str) -> Option<PrewarmedEntry> {
        let guard = self.prewarmed.read().await;
        let entry = guard.get(channel)?;
        if entry.warmed_at.elapsed() >= Duration::from_secs(PREWARM_TTL_SECS) {
            return None;
        }
        Some(entry.clone())
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
        let channel = {
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
            infer_channel_from_manifest_url(&first_variant.manifest_url).ok_or_else(|| {
                StreamError::HlsFetchFailed(
                    "unable to infer channel for quality resolve".to_string(),
                )
            })?
        };

        let manifest_url = get_hls_url_streamlink(&channel, &self.streamlink_path, quality)
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
        };

        let mut guard = self.sessions.write().await;
        let Some(session) = guard.get_mut(stream_id) else {
            return Err(StreamError::StreamNotFound);
        };
        if session.session_token != session_token {
            return Err(StreamError::SessionMismatch);
        }
        session
            .variants
            .insert(quality.to_string(), variant.clone());
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

        Ok(session.clone())
    }

    async fn mark_delivery_logged_once(
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
        session.logged_delivery_modes.insert(key)
    }
}

fn infer_channel_from_manifest_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let path = parsed.path();
    let before_chunked = path.split("/chunked").next()?;
    before_chunked
        .split('/')
        .next_back()
        .map(ToString::to_string)
}

fn sort_qualities<'a>(qualities: impl Iterator<Item = &'a String>) -> Vec<&'a str> {
    let mut out: Vec<&str> = qualities.map(String::as_str).collect();
    out.sort_by_key(|quality| quality_sort_rank(quality));
    out
}

fn is_allowed_quality(quality: &str) -> bool {
    PREWARM_POOL_QUALITIES.contains(&quality)
}

fn quality_sort_rank(quality: &str) -> (u8, std::cmp::Reverse<u32>, &str) {
    let rank = match quality {
        "source" => 0,
        "1080p60" => 1,
        "720p60" => 2,
        "480p" => 3,
        "360p" => 4,
        _ => 100,
    };
    let (_, width, _) = quality_info(quality);
    (rank, std::cmp::Reverse(width), quality)
}

async fn get_hls_url_streamlink(
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

#[derive(Debug, Deserialize)]
struct PlaybackAccessResponse {
    data: Option<PlaybackAccessData>,
}

#[derive(Debug, Deserialize)]
struct PlaybackAccessData {
    #[serde(rename = "streamPlaybackAccessToken")]
    stream_playback_access_token: Option<PlaybackAccessToken>,
}

#[derive(Debug, Deserialize)]
struct PlaybackAccessToken {
    value: String,
    signature: String,
}

#[derive(Debug, Clone)]
struct NativeVariant {
    quality: String,
    manifest_url: String,
    bandwidth: u64,
}

async fn fetch_native_master_manifest(
    client: &Client,
    channel: &str,
    client_id: &str,
) -> Result<(String, String), String> {
    let query = serde_json::json!({
        "query": "query PlaybackAccessToken($login: String!, $isLive: Boolean!, $vodID: ID!, $isVod: Boolean!, $playerType: String!) { streamPlaybackAccessToken(channelName: $login, params: { platform: \"web\", playerBackend: \"mediaplayer\", playerType: $playerType }) @include(if: $isLive) { value signature } videoPlaybackAccessToken(id: $vodID, params: { platform: \"web\", playerBackend: \"mediaplayer\", playerType: $playerType }) @include(if: $isVod) { value signature } }",
        "variables": {
            "isLive": true,
            "login": channel,
            "isVod": false,
            "vodID": "",
            "playerType": "site"
        }
    });

    let response = client
        .post("https://gql.twitch.tv/gql")
        .header("Client-Id", client_id)
        .header("Content-Type", "application/json")
        .json(&query)
        .send()
        .await
        .map_err(|e| format!("native GraphQL request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "native GraphQL request failed with status {}",
            response.status()
        ));
    }

    let payload: PlaybackAccessResponse = response
        .json()
        .await
        .map_err(|e| format!("failed to decode GraphQL response: {e}"))?;

    let token = payload
        .data
        .and_then(|d| d.stream_playback_access_token)
        .ok_or_else(|| "missing playback token in GraphQL response".to_string())?;

    if token.value.trim().is_empty() || token.signature.trim().is_empty() {
        return Err("received empty playback token from GraphQL".to_string());
    }

    let mut usher_url = Url::parse(&format!(
        "https://usher.ttvnw.net/api/channel/hls/{}.m3u8",
        channel
    ))
    .map_err(|e| format!("failed to build usher URL: {e}"))?;

    usher_url
        .query_pairs_mut()
        .append_pair("allow_source", "true")
        .append_pair("allow_audio_only", "true")
        .append_pair("fast_bread", "true")
        .append_pair("player_backend", "mediaplayer")
        .append_pair("playlist_include_framerate", "true")
        .append_pair("reassignments_supported", "true")
        .append_pair("sig", &token.signature)
        .append_pair("supported_codecs", "av1,h265,h264")
        .append_pair("token", &token.value)
        .append_pair("transcode_mode", "cbr_v1")
        .append_pair("cdm", "wv")
        .append_pair("player", "twitchweb");

    let master_manifest_url = usher_url.to_string();
    let master_manifest = client
        .get(master_manifest_url.clone())
        .send()
        .await
        .map_err(|e| format!("native usher request failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed reading usher response: {e}"))?;

    if master_manifest.trim().is_empty() {
        return Err("usher returned empty master playlist".to_string());
    }

    Ok((master_manifest_url, master_manifest))
}

fn select_native_variants(
    master_manifest_url: &str,
    master_manifest: &str,
    requested_quality: &str,
) -> Result<Vec<(String, String)>, String> {
    let parsed = parse_native_variants(master_manifest_url, master_manifest);
    if parsed.is_empty() {
        return Err("native master playlist has no variants".to_string());
    }

    let mut best_by_quality: HashMap<String, NativeVariant> = HashMap::new();
    for candidate in parsed {
        let key = candidate.quality.clone();
        match best_by_quality.get(&key) {
            Some(existing) if existing.bandwidth >= candidate.bandwidth => {}
            _ => {
                best_by_quality.insert(key, candidate);
            }
        }
    }

    let mut selected = Vec::new();
    if requested_quality == "best" {
        let mut entries: Vec<NativeVariant> = best_by_quality.into_values().collect();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.bandwidth));
        for item in entries.into_iter().take(4) {
            selected.push((item.quality, item.manifest_url));
        }
        return Ok(selected);
    }

    let preferred_order = [
        requested_quality,
        "source",
        "1080p60",
        "1080p",
        "720p60",
        "720p",
        "480p60",
        "480p",
        "360p",
        "160p",
        "audio_only",
    ];

    for quality in preferred_order {
        if let Some(item) = best_by_quality.remove(quality) {
            selected.push((item.quality, item.manifest_url));
        }
        if selected.len() >= 4 {
            break;
        }
    }

    if selected.len() < 4 {
        let mut remaining: Vec<NativeVariant> = best_by_quality.into_values().collect();
        remaining.sort_by_key(|entry| std::cmp::Reverse(entry.bandwidth));
        for item in remaining {
            selected.push((item.quality, item.manifest_url));
            if selected.len() >= 4 {
                break;
            }
        }
    }

    Ok(selected)
}

fn parse_native_variants(master_manifest_url: &str, manifest: &str) -> Vec<NativeVariant> {
    let mut variants = Vec::new();
    let base = Url::parse(master_manifest_url).ok();
    let lines: Vec<&str> = manifest.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if let Some(attrs_raw) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let attrs = parse_hls_attrs(attrs_raw);
            let mut next_url = None;
            let mut j = i + 1;
            while j < lines.len() {
                let candidate = lines[j].trim();
                if candidate.is_empty() {
                    j += 1;
                    continue;
                }
                if candidate.starts_with('#') {
                    break;
                }
                next_url = Some(candidate.to_string());
                break;
            }

            if let Some(raw_url) = next_url {
                let manifest_url =
                    if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
                        raw_url
                    } else if let Some(base_url) = &base {
                        base_url
                            .join(&raw_url)
                            .map(|u| u.to_string())
                            .unwrap_or(raw_url)
                    } else {
                        raw_url
                    };

                let quality = normalize_quality_label(
                    attrs.get("NAME").map(String::as_str),
                    attrs.get("VIDEO").map(String::as_str),
                    attrs.get("RESOLUTION").map(String::as_str),
                    attrs.get("FRAME-RATE").map(String::as_str),
                );

                let bandwidth = attrs
                    .get("BANDWIDTH")
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);

                variants.push(NativeVariant {
                    quality,
                    manifest_url,
                    bandwidth,
                });
            }
        }

        i += 1;
    }

    variants
}

fn parse_hls_attrs(attrs: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in attrs.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
            continue;
        }

        if ch == ',' && !in_quotes {
            if let Some((k, v)) = current.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
            }
            current.clear();
            continue;
        }

        current.push(ch);
    }

    if let Some((k, v)) = current.split_once('=') {
        map.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
    }

    map
}

fn normalize_quality_label(
    name: Option<&str>,
    video: Option<&str>,
    resolution: Option<&str>,
    frame_rate: Option<&str>,
) -> String {
    if let Some(name) = name {
        let lowered = name.to_ascii_lowercase();
        if lowered.contains("chunked") || lowered == "source" {
            return "source".to_string();
        }
        if lowered.contains("audio") {
            return "audio_only".to_string();
        }

        let compact: String = lowered
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        if compact.contains('p') {
            return compact;
        }
    }

    if let Some(video) = video {
        let lowered = video.to_ascii_lowercase();
        if lowered.contains("chunked") {
            return "source".to_string();
        }
    }

    let height = resolution
        .and_then(|res| res.split('x').nth(1))
        .and_then(|h| h.parse::<u32>().ok());
    let fps = frame_rate.and_then(|fps| fps.parse::<f32>().ok());

    match (height, fps) {
        (Some(h), Some(fps)) if fps >= 50.0 => format!("{h}p60"),
        (Some(h), _) => format!("{h}p"),
        _ => "source".to_string(),
    }
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
    force_relay: bool,
) -> String {
    let relay_suffix = if force_relay { "?relay=1" } else { "" };
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
                    "/stream/{}/{}/{}/{}{}",
                    stream_id, session_token, quality, segment_name, relay_suffix
                )
            } else {
                let segment_name = line.split('?').next().unwrap_or(line);
                format!(
                    "/stream/{}/{}/{}/{}{}",
                    stream_id, session_token, quality, segment_name, relay_suffix
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn proxy_manifest(
    State(state): State<StreamProxyState>,
    Path((stream_id, session_token)): Path<(String, String)>,
    Query(query): Query<RelayQuery>,
) -> Response {
    let force_relay = query.force_relay();
    match state
        .service
        .get_multi_level_manifest(&stream_id, &session_token, force_relay)
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
    Query(query): Query<RelayQuery>,
) -> Response {
    let force_relay = query.force_relay();
    match state
        .service
        .get_variant_manifest(&stream_id, &session_token, &quality, force_relay)
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
    Query(query): Query<RelayQuery>,
) -> Response {
    let force_relay = query.force_relay();
    match state
        .service
        .resolve_segment(&stream_id, &quality, &segment, &session_token)
        .await
    {
        Ok((cdn_url, resolver)) => {
            if state.service.should_redirect_to_cdn(force_relay) {
                if state
                    .service
                    .mark_delivery_logged_once(&stream_id, &quality, "cdn_redirect")
                    .await
                {
                    tracing::info!(
                        delivery = "cdn_redirect",
                        resolver = ?resolver,
                        force_relay = force_relay,
                        stream_id = %stream_id,
                        quality = %quality,
                        "serving stream via CDN redirect"
                    );
                }
                return (StatusCode::FOUND, [(header::LOCATION, cdn_url)]).into_response();
            }

            let body = match fetch_bytes(&cdn_url).await {
                Ok(bytes) => bytes,
                Err(msg) => {
                    tracing::error!(
                        error = %msg,
                        stream_id = %stream_id,
                        segment = %segment,
                        delivery = "relay_bytes",
                        resolver = ?resolver,
                        "failed to fetch stream segment"
                    );
                    return error_response(
                        StatusCode::BAD_GATEWAY,
                        "failed to fetch stream segment",
                    );
                }
            };

            if state
                .service
                .mark_delivery_logged_once(&stream_id, &quality, "relay_bytes")
                .await
            {
                tracing::info!(
                    delivery = "relay_bytes",
                    resolver = ?resolver,
                    force_relay = force_relay,
                    fallback_reason = if force_relay { "hls_fatal_retry" } else { "delivery_mode_relay" },
                    stream_id = %stream_id,
                    quality = %quality,
                    "serving stream via relay"
                );
            }

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
            tracing::error!(error = %msg, stream_id = %stream_id, segment = %segment, "failed to resolve stream segment URL");
            error_response(StatusCode::BAD_GATEWAY, "failed to fetch stream segment")
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_native_variants_extracts_quality_labels() {
        let master = "#EXTM3U\n\
#EXT-X-STREAM-INF:BANDWIDTH=5800000,RESOLUTION=1920x1080,FRAME-RATE=60.000,VIDEO=chunked,NAME=\"1080p60\"\n\
1080p60/index-dvr.m3u8\n\
#EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720,FRAME-RATE=60.000,NAME=\"720p60\"\n\
https://example.test/720p60/index-dvr.m3u8\n";

        let variants =
            parse_native_variants("https://usher.ttvnw.net/api/channel/hls/test.m3u8", master);
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].quality, "1080p60");
        assert!(
            variants[0]
                .manifest_url
                .starts_with("https://usher.ttvnw.net/")
        );
        assert_eq!(variants[1].quality, "720p60");
    }

    #[test]
    fn select_native_variants_prefers_requested_quality() {
        let master = "#EXTM3U\n\
#EXT-X-STREAM-INF:BANDWIDTH=6500000,RESOLUTION=1920x1080,FRAME-RATE=60.000,NAME=\"1080p60\"\n\
https://example.test/1080.m3u8\n\
#EXT-X-STREAM-INF:BANDWIDTH=2800000,RESOLUTION=1280x720,FRAME-RATE=60.000,NAME=\"720p60\"\n\
https://example.test/720.m3u8\n\
#EXT-X-STREAM-INF:BANDWIDTH=1200000,RESOLUTION=854x480,NAME=\"480p\"\n\
https://example.test/480.m3u8\n";

        let selected = select_native_variants(
            "https://usher.ttvnw.net/api/channel/hls/test.m3u8",
            master,
            "720p60",
        )
        .expect("select variants");

        assert!(!selected.is_empty());
        assert_eq!(selected[0].0, "720p60");
    }
}
