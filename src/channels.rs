use std::{
   fs,
   path::PathBuf,
};

use serde::{
   Deserialize,
   Serialize,
};

use crate::storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredChannel {
   pub login:          String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub image_filename: Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub profile_url:    Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredChannels {
   channels: Vec<StoredChannel>,
}

pub fn stored_channels_path() -> Option<PathBuf> {
   storage::paths::channels_path()
}

pub fn images_dir() -> Option<PathBuf> {
   storage::paths::images_dir()
}

pub fn load_stored_channels() -> Vec<StoredChannel> {
   let Some(path) = stored_channels_path() else {
      return Vec::new();
   };

   storage::files::load_toml_optional::<StoredChannels>(&path)
      .ok()
      .flatten()
      .map(|stored| stored.channels)
      .unwrap_or_default()
}

pub fn save_stored_channels(channels: &[StoredChannel]) -> Result<(), String> {
   let Some(path) = stored_channels_path() else {
      return Err("unable to resolve config directory".to_string());
   };

   let payload = StoredChannels {
      channels: channels.to_vec(),
   };
   storage::files::write_toml_pretty_atomic(&path, &payload)
      .map_err(|e| format!("save channels config failed: {e}"))
}

pub fn get_channel_image_path(login: &str) -> Option<PathBuf> {
   let channels = load_stored_channels();
   let channel = channels.iter().find(|c| c.login == login)?;
   let filename = channel.image_filename.as_ref()?;
   let dir = images_dir()?;
   Some(dir.join(filename))
}

pub fn save_channel_image(login: &str, image_data: &[u8]) -> Result<String, String> {
   let dir = images_dir().ok_or("unable to resolve images directory")?;
   let filename = format!("{login}.png");
   let path = dir.join(&filename);

   storage::files::write_text(&path, "").map_err(|e| format!("create image file failed: {e}"))?;
   fs::write(&path, image_data).map_err(|e| format!("write image failed: {e}"))?;

   Ok(filename)
}

pub fn update_channel_image(
   login: &str,
   image_filename: &str,
   profile_url: &str,
) -> Result<(), String> {
   let mut channels = load_stored_channels();

   if let Some(channel) = channels.iter_mut().find(|c| c.login == login) {
      channel.image_filename = Some(image_filename.to_string());
      channel.profile_url = Some(profile_url.to_string());
   } else {
      return Err("channel not found".to_string());
   }

   save_stored_channels(&channels)
}

pub fn add_channel(login: String) -> Result<StoredChannel, String> {
   let mut channels = load_stored_channels();

   if channels.iter().any(|c| c.login == login) {
      return Err("channel already exists".to_string());
   }

   let channel = StoredChannel {
      login:          login.clone(),
      image_filename: None,
      profile_url:    None,
   };

   channels.push(channel);
   save_stored_channels(&channels)?;

   Ok(StoredChannel {
      login,
      image_filename: None,
      profile_url: None,
   })
}

pub fn remove_channel(login: &str) -> Result<(), String> {
   let mut channels = load_stored_channels();
   let original_len = channels.len();
   channels.retain(|c| c.login != login);

   if channels.len() == original_len {
      return Err("channel not found".to_string());
   }

   if let Some(image_path) = get_channel_image_path(login) {
      let _ = fs::remove_file(image_path);
   }

   save_stored_channels(&channels)
}

pub fn channel_exists(login: &str) -> bool {
   load_stored_channels().iter().any(|c| c.login == login)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_stored_channel_serialization() {
      let channel = StoredChannel {
         login:          "test_channel".to_string(),
         image_filename: Some("test_channel.png".to_string()),
         profile_url:    Some("https://example.com/image.png".to_string()),
      };

      let json = serde_json::to_string(&channel).unwrap();
      assert!(json.contains("\"login\":\"test_channel\""));
      assert!(json.contains("\"image_filename\":\"test_channel.png\""));
   }

   #[test]
   fn test_stored_channel_no_image() {
      let channel = StoredChannel {
         login:          "test_channel".to_string(),
         image_filename: None,
         profile_url:    None,
      };

      let json = serde_json::to_string(&channel).unwrap();
      assert!(json.contains("\"login\":\"test_channel\""));
      assert!(!json.contains("image_filename"));
   }
}
