use std::{
   fmt::Write,
   fs,
   path::Path,
};

use time::OffsetDateTime;

use super::{
   files::sanitize_filename,
   types::{
      ActiveRecording,
      ChannelMetadataCache,
      RecordingMode,
   },
};
use crate::twitch_auth::HelixChannelMetadata;

pub(super) fn datetime_from_unix(unix_secs: u64) -> OffsetDateTime {
   i64::try_from(unix_secs)
      .ok()
      .and_then(|unix| OffsetDateTime::from_unix_timestamp(unix).ok())
      .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

pub(super) fn xml_escape(value: &str) -> String {
   value
      .replace('&', "&amp;")
      .replace('<', "&lt;")
      .replace('>', "&gt;")
      .replace('"', "&quot;")
      .replace('\'', "&apos;")
}

pub(super) fn xml_tag_value(content: &str, tag: &str) -> Option<String> {
   let open = format!("<{tag}>");
   let close = format!("</{tag}>");
   let start = content.find(&open)? + open.len();
   let end_rel = content[start..].find(&close)?;
   Some(content[start..start + end_rel].trim().to_string())
}

pub(super) fn write_episode_nfo_file(
   channel_login: &str,
   recording_path: &Path,
   metadata: &ActiveRecording,
   stream_title: Option<&str>,
   genres: &[String],
) -> Result<(), String> {
   let Some(stem) = recording_path.file_stem().and_then(|value| value.to_str()) else {
      return Err("failed to derive recording basename".to_string());
   };

   let started = datetime_from_unix(metadata.started_at_unix);
   let season = started.year();
   let month = started.month() as u8;
   let day = started.day();
   let base_episode: u16 = u16::from(month) * 100 + u16::from(day);
   let aired = format!("{season:04}-{month:02}-{day:02}");

   let channel_dir = recording_path
      .parent()
      .ok_or_else(|| "recording file has no parent directory".to_string())?;
   let suffix_index = next_same_day_suffix_index(channel_dir, &aired, base_episode);
   let episode_number = base_episode.saturating_add(suffix_index);
   let display_episode = if suffix_index == 0 {
      base_episode.to_string()
   } else {
      format!("{base_episode}-{suffix_index}")
   };

   let chosen_title = stream_title
      .map(str::trim)
      .filter(|value| !value.is_empty())
      .map_or_else(
         || format!("{channel_login} stream {aired}"),
         ToOwned::to_owned,
      );
   let title = if suffix_index == 0 {
      chosen_title.clone()
   } else {
      format!("{chosen_title} ({display_episode})")
   };
   let mode = match metadata.mode {
      RecordingMode::Manual => "manual",
      RecordingMode::Auto => "auto",
   };
   let plot = format!(
      "Twitch recording for {channel_login}. Title: {chosen_title}. Quality: {}. Mode: {mode}.",
      metadata.quality
   );
   let uniqueid = format!(
      "{}-{}",
      sanitize_filename(channel_login),
      metadata.started_at_unix
   );
   let mut genre_xml = String::new();
   for genre in genres {
      let _ = writeln!(genre_xml, "  <genre>{}</genre>", xml_escape(genre));
   }

   let xml = format!(
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<episodedetails>\n  <title>{}</title>\n  \
       <showtitle>{}</showtitle>\n  <season>{season}</season>\n  \
       <episode>{episode_number}</episode>\n  <displayepisode>{}</displayepisode>\n  \
       <aired>{aired}</aired>\n{genre_xml}  <plot>{}</plot>\n  <uniqueid type=\"twitch\" \
       default=\"true\">{}</uniqueid>\n</episodedetails>\n",
      xml_escape(&title),
      xml_escape(channel_login),
      xml_escape(&display_episode),
      xml_escape(&plot),
      xml_escape(&uniqueid)
   );

   let nfo_path = recording_path.with_file_name(format!("{stem}.nfo"));
   fs::write(&nfo_path, xml)
      .map_err(|error| format!("failed to write nfo file {}: {error}", nfo_path.display()))
}

pub(super) fn next_same_day_suffix_index(channel_dir: &Path, aired: &str, episode: u16) -> u16 {
   let Ok(entries) = fs::read_dir(channel_dir) else {
      return 0;
   };

   let mut max_suffix: i32 = -1;
   for entry in entries.flatten() {
      let path = entry.path();
      let is_nfo = path
         .extension()
         .and_then(|ext| ext.to_str())
         .is_some_and(|ext| ext.eq_ignore_ascii_case("nfo"));
      if !is_nfo {
         continue;
      }

      let Ok(content) = fs::read_to_string(&path) else {
         continue;
      };
      if xml_tag_value(&content, "aired").as_deref() != Some(aired) {
         continue;
      }
      let Some(episode_value) = xml_tag_value(&content, "episode") else {
         continue;
      };
      if episode_value.trim().parse::<u16>().ok() != Some(episode) {
         continue;
      }

      let display =
         xml_tag_value(&content, "displayepisode").unwrap_or_else(|| episode_value.clone());
      let parsed = parse_display_episode_suffix(&display, episode);
      if parsed > max_suffix {
         max_suffix = parsed;
      }
   }

   u16::try_from((max_suffix + 1).max(0)).unwrap_or(0)
}

pub(super) fn parse_display_episode_suffix(display_episode: &str, episode: u16) -> i32 {
   let trimmed = display_episode.trim();
   let base = episode.to_string();
   if trimmed == base {
      return 0;
   }

   let Some(suffix) = trimmed.strip_prefix(&format!("{base}-")) else {
      return 0;
   };
   suffix.parse::<i32>().unwrap_or(0)
}

pub(super) fn write_tvshow_nfo_file(
   channel_login: &str,
   channel_dir: &Path,
   metadata: &HelixChannelMetadata,
   genres: &[String],
) -> Result<(), String> {
   use std::fmt::Write;

   let title = metadata.display_name.trim();
   let plot = metadata
      .description
      .as_deref()
      .filter(|v| !v.trim().is_empty())
      .unwrap_or("Twitch channel recordings.");
   let mut genre_xml = String::new();
   for genre in genres {
      let _ = writeln!(genre_xml, "  <genre>{}</genre>", xml_escape(genre));
   }
   let xml = format!(
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<tvshow>\n  <title>{}</title>\n  \
       <plot>{}</plot>\n  <status>Continuing</status>\n  <studio>{}</studio>\n{genre_xml}  \
       <thumb>poster.jpg</thumb>\n  <uniqueid type=\"twitch\" \
       default=\"true\">twitch_{}</uniqueid>\n</tvshow>\n",
      xml_escape(title),
      xml_escape(plot),
      xml_escape(title),
      xml_escape(channel_login)
   );
   let path = channel_dir.join("tvshow.nfo");
   fs::write(&path, xml)
      .map_err(|error| format!("failed to write tvshow.nfo {}: {error}", path.display()))
}

pub(super) fn select_show_tags(
   metadata: Option<&HelixChannelMetadata>,
   cache: &ChannelMetadataCache,
) -> Vec<String> {
   let mut out: Vec<String> = Vec::new();
   if let Some(meta) = metadata {
      for tag in &meta.tags {
         append_unique_tag(&mut out, tag);
      }
      if out.is_empty() {
         for tag in &cache.tags {
            append_unique_tag(&mut out, tag);
         }
      }
      if out.is_empty()
         && let Some(game) = meta.game.as_deref()
      {
         append_unique_tag(&mut out, game);
      }
   } else {
      for tag in &cache.tags {
         append_unique_tag(&mut out, tag);
      }
   }
   out.truncate(10);
   out
}

fn append_unique_tag(tags: &mut Vec<String>, raw: &str) {
   let normalized = raw.trim();
   if normalized.is_empty() {
      return;
   }
   if tags.iter().any(|tag| tag.eq_ignore_ascii_case(normalized)) {
      return;
   }
   tags.push(normalized.to_string());
}

pub(super) fn read_channel_metadata_cache(channel_dir: &Path) -> ChannelMetadataCache {
   let path = channel_dir.join(".metadata-cache.json");
   let Ok(text) = fs::read_to_string(path) else {
      return ChannelMetadataCache::default();
   };
   serde_json::from_str::<ChannelMetadataCache>(&text).unwrap_or_default()
}

pub(super) fn write_channel_metadata_cache(
   channel_dir: &Path,
   cache: &ChannelMetadataCache,
) -> Result<(), String> {
   let path = channel_dir.join(".metadata-cache.json");
   let payload = serde_json::to_string(cache)
      .map_err(|error| format!("failed to encode channel metadata cache: {error}"))?;
   fs::write(&path, payload).map_err(|error| {
      format!(
         "failed to write channel metadata cache {}: {error}",
         path.display()
      )
   })
}

pub(super) async fn update_channel_poster(
   channel_dir: &Path,
   http: &reqwest::Client,
   metadata: &HelixChannelMetadata,
   cache: &mut ChannelMetadataCache,
) {
   let Some(url) = metadata.profile_image_url.as_deref() else {
      return;
   };
   if cache.poster_url.as_deref() == Some(url) {
      return;
   }
   let Ok(response) = http.get(url).send().await else {
      return;
   };
   if !response.status().is_success() {
      return;
   }
   let Ok(bytes) = response.bytes().await else {
      return;
   };
   let _ = fs::create_dir_all(channel_dir);
   let poster_path = channel_dir.join("poster.jpg");
   if fs::write(&poster_path, &bytes).is_ok() {
      cache.poster_url = Some(url.to_string());
   }
}

/// Write TV show and episode NFO files for a completed recording.
///
/// This is an async wrapper that handles fetching channel metadata and writing
/// both the tvshow.nfo (series metadata) and episode.nfo (recording metadata).
pub async fn write_tv_nfo_files(
   channel_login: &str,
   recording_path: &Path,
   metadata: &ActiveRecording,
   stream_title: Option<&str>,
   twitch: &crate::twitch_auth::TwitchAuthService,
) -> Result<(), String> {
   let channel_dir = recording_path
      .parent()
      .and_then(|p| p.parent())
      .ok_or_else(|| "recording file has no season parent".to_string())?;

   let fetched = fetch_twitch_channel_metadata(channel_login, twitch).await;
   let mut cache = read_channel_metadata_cache(channel_dir);
   let tags = select_show_tags(fetched.as_ref(), &cache);

   if let Some(meta) = fetched.as_ref() {
      let http = twitch.api_client();
      update_channel_poster(channel_dir, &http, meta, &mut cache).await;
      write_tvshow_nfo_file(channel_login, channel_dir, meta, &tags)?;
   }
   cache.tags = tags.clone();
   let _ = write_channel_metadata_cache(channel_dir, &cache);

   write_episode_nfo_file(channel_login, recording_path, metadata, stream_title, &tags)
}

async fn fetch_twitch_channel_metadata(
   channel_login: &str,
   twitch: &crate::twitch_auth::TwitchAuthService,
) -> Option<HelixChannelMetadata> {
   match twitch.fetch_channel_metadata(channel_login).await {
      Ok(value) => value,
      Err(error) => {
         tracing::warn!(channel = %channel_login, error = %error, "helix metadata lookup failed");
         None
      },
   }
}
