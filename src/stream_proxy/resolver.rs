use std::collections::HashMap;

use reqwest::{
   Client,
   Url,
};
use serde::Deserialize;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct NativeVariant {
   pub quality:      String,
   pub manifest_url: String,
   pub bandwidth:    u64,
   pub width:        Option<u32>,
   pub height:       Option<u32>,
   pub frame_rate:   Option<f32>,
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
   value:     String,
   signature: String,
}

pub fn infer_channel_from_manifest_url(url: &str) -> Option<String> {
   let parsed = Url::parse(url).ok()?;
   let path = parsed.path();
   let before_chunked = path.split("/chunked").next()?;
   before_chunked
      .split('/')
      .next_back()
      .map(ToString::to_string)
}

pub fn sort_qualities<'a>(qualities: impl Iterator<Item = &'a String>) -> Vec<&'a str> {
   let mut out: Vec<&str> = qualities.map(String::as_str).collect();
   out.sort_by_key(|quality| quality_sort_rank(quality));
   out
}

pub fn is_allowed_quality(quality: &str) -> bool {
   let allowed: [&str; 5] = ["source", "1080p60", "720p60", "480p", "360p"];
   allowed.contains(&quality)
}

pub fn quality_sort_rank(quality: &str) -> (u8, std::cmp::Reverse<u32>, &str) {
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

pub async fn get_hls_url_streamlink(
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

pub async fn fetch_native_master_manifest(
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
      "https://usher.ttvnw.net/api/channel/hls/{channel}.m3u8"
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

pub fn select_native_variants(
   master_manifest_url: &str,
   master_manifest: &str,
   requested_quality: &str,
) -> Result<Vec<NativeVariant>, String> {
   let parsed = parse_native_variants(master_manifest_url, master_manifest);
   if parsed.is_empty() {
      return Err("native master playlist has no variants".to_string());
   }

   let mut best_by_quality: HashMap<String, NativeVariant> = HashMap::new();
   for candidate in parsed {
      let key = candidate.quality.clone();
      match best_by_quality.get(&key) {
         Some(existing) if existing.bandwidth >= candidate.bandwidth => {},
         _ => {
            best_by_quality.insert(key, candidate);
         },
      }
   }

   let mut selected = Vec::new();
   if requested_quality == "best" {
      let mut entries: Vec<NativeVariant> = best_by_quality.into_values().collect();
      entries.sort_by_key(|entry| std::cmp::Reverse(entry.bandwidth));
      selected.extend(entries.into_iter().take(4));
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
         selected.push(item);
      }
      if selected.len() >= 4 {
         break;
      }
   }

   if selected.len() < 4 {
      let mut remaining: Vec<NativeVariant> = best_by_quality.into_values().collect();
      remaining.sort_by_key(|entry| std::cmp::Reverse(entry.bandwidth));
      for item in remaining {
         selected.push(item);
         if selected.len() >= 4 {
            break;
         }
      }
   }

   Ok(selected)
}

pub fn parse_native_variants(master_manifest_url: &str, manifest: &str) -> Vec<NativeVariant> {
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
            let manifest_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://")
            {
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

            let (width, height) = attrs
               .get("RESOLUTION")
               .and_then(|v| v.split_once('x'))
               .map_or((None, None), |(w, h)| (w.parse::<u32>().ok(), h.parse::<u32>().ok()));

            let frame_rate = attrs.get("FRAME-RATE").and_then(|v| v.parse::<f32>().ok());

            variants.push(NativeVariant {
               quality,
               manifest_url,
               bandwidth,
               width,
               height,
               frame_rate,
            });
         }
      }

      i += 1;
   }

   variants
}

pub fn parse_hls_attrs(attrs: &str) -> HashMap<String, String> {
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

pub fn normalize_quality_label(
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
         .filter(char::is_ascii_alphanumeric)
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

pub async fn fetch_and_parse_manifest(
   url: &str,
) -> Result<(HashMap<String, String>, String), String> {
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

pub fn quality_info(quality: &str) -> (u64, u32, u32) {
   match quality {
      "source" => (8_000_000, 1920, 1080),
      "1080p60" => (6_000_000, 1920, 1080),
      "1080p" => (4_500_000, 1920, 1080),
      "720p60" => (3_000_000, 1280, 720),
      "720p" => (2_000_000, 1280, 720),
      "480p60" => (1_500_000, 854, 480),
      "480p" => (1_000_000, 854, 480),
      "360p" => (600_000, 640, 360),
      "160p" => (300_000, 284, 160),
      _ => (1_500_000, 1280, 720),
   }
}

pub fn quality_frame_rate(quality: &str) -> Option<f32> {
   if quality.ends_with("p60") {
      return Some(60.0);
   }
   None
}

pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
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

pub async fn fetch_text(url: &str) -> Result<String, String> {
   let bytes = fetch_bytes(url).await?;
   String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {e}"))
}

pub fn parse_segment_lookup(manifest: &str) -> (HashMap<String, String>, String) {
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

pub fn rewrite_manifest_urls(
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
               "/stream/{stream_id}/{session_token}/{quality}/{segment_name}"
            )
         } else {
            let segment_name = line.split('?').next().unwrap_or(line);
            format!(
               "/stream/{stream_id}/{session_token}/{quality}/{segment_name}"
            )
         }
      })
      .map(|line| {
         if !line.starts_with('#') && !line.is_empty() && !line.starts_with("http") {
            format!("{line}{relay_suffix}")
         } else {
            line
         }
      })
      .collect::<Vec<_>>()
      .join("\n")
}
