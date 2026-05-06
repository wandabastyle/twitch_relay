use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

const YOUTUBE_IMAGES_SUBDIR: &str = "youtube_images";

/// Stored YouTube channel data including cached avatar info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredYoutubeChannel {
    pub channel_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YoutubeChannelsData {
    pub channels: Vec<StoredYoutubeChannel>,
}

/// Get the directory for storing YouTube channel images
pub fn images_dir() -> Option<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "twitch-relay")?;
    Some(dirs.data_local_dir().join(YOUTUBE_IMAGES_SUBDIR))
}

/// Get the path for the YouTube channels metadata file
fn stored_channels_path() -> Option<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "twitch-relay")?;
    Some(dirs.data_local_dir().join("youtube_channels.toml"))
}

/// Load the YouTube channels data from disk
fn load_channels_data() -> Result<YoutubeChannelsData, AppError> {
    let path = match stored_channels_path() {
        Some(p) => p,
        None => return Ok(YoutubeChannelsData::default()),
    };

    if !path.exists() {
        return Ok(YoutubeChannelsData::default());
    }

    let content = std::fs::read_to_string(&path)?;
    let data: YoutubeChannelsData = toml::from_str(&content).unwrap_or_default();

    Ok(data)
}

/// Save the YouTube channels data to disk
fn save_channels_data(data: &YoutubeChannelsData) -> Result<(), AppError> {
    let path = match stored_channels_path() {
        Some(p) => p,
        None => return Ok(()),
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(data)?;
    std::fs::write(&path, content)?;

    Ok(())
}

/// Get the stored channel info for a given channel ID
pub fn get_channel_info(channel_id: &str) -> Option<StoredYoutubeChannel> {
    let data = match load_channels_data() {
        Ok(d) => d,
        Err(_) => return None,
    };

    data.channels
        .iter()
        .find(|c| c.channel_id == channel_id)
        .cloned()
}

/// Check if a cached image exists and is still valid (not expired)
pub fn get_cached_image_path(channel_id: &str) -> Option<std::path::PathBuf> {
    let info = get_channel_info(channel_id)?;

    // Check if cache is still valid (24 hours)
    if let Some(cached_at) = info.cached_at {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 24 hours = 86400 seconds
        if now - cached_at > 86400 {
            return None; // Cache expired
        }
    } else {
        return None;
    }

    let images_dir = images_dir()?;
    let filename = info.image_filename?;
    let path = images_dir.join(&filename);

    if path.exists() { Some(path) } else { None }
}

/// Get the URL for a channel's image (either cached or external)
pub fn get_channel_image_url(channel_id: &str) -> Option<String> {
    // First check if we have a cached image
    if get_cached_image_path(channel_id).is_some() {
        return Some(format!("/static/youtube_images/{}", channel_id));
    }

    // Fall back to stored external URL
    let info = get_channel_info(channel_id)?;
    info.image_url
}

/// Save a channel image to disk
pub fn save_channel_image(channel_id: &str, image_data: &[u8]) -> Result<String, AppError> {
    let images_dir = images_dir()
        .ok_or_else(|| AppError::Config("Failed to get youtube images directory".to_string()))?;

    std::fs::create_dir_all(&images_dir)?;

    // Use jpg extension for smaller file size
    let filename = format!("{}.jpg", channel_id);
    let path = images_dir.join(&filename);

    let mut file = std::fs::File::create(&path)?;
    file.write_all(image_data)?;

    Ok(filename)
}

/// Update or insert channel image metadata
pub fn update_channel_image(
    channel_id: &str,
    filename: &str,
    image_url: &str,
) -> Result<(), AppError> {
    let mut data = load_channels_data()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Find existing channel or create new one
    let mut found = false;
    for channel in &mut data.channels {
        if channel.channel_id == channel_id {
            channel.image_filename = Some(filename.to_string());
            channel.image_url = Some(image_url.to_string());
            channel.cached_at = Some(now);
            found = true;
            break;
        }
    }

    // Add new channel if not found
    if !found {
        data.channels.push(StoredYoutubeChannel {
            channel_id: channel_id.to_string(),
            image_filename: Some(filename.to_string()),
            image_url: Some(image_url.to_string()),
            cached_at: Some(now),
        });
    }

    save_channels_data(&data)?;
    Ok(())
}

/// Get the MIME type for an image file based on extension
pub fn get_image_mime_type(filename: &str) -> &'static str {
    if filename.ends_with(".png") {
        "image/png"
    } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_image_mime_type() {
        assert_eq!(get_image_mime_type("test.png"), "image/png");
        assert_eq!(get_image_mime_type("test.jpg"), "image/jpeg");
        assert_eq!(get_image_mime_type("test.jpeg"), "image/jpeg");
        assert_eq!(get_image_mime_type("test.txt"), "application/octet-stream");
    }
}
