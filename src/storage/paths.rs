//! Path resolution helpers for application directories.
//!
//! All paths are based on ProjectDirs for "twitch-relay".

use std::path::PathBuf;

use directories::ProjectDirs;

const APP_QUALIFIER: &str = "";
const APP_ORGANIZATION: &str = "";
const APP_NAME: &str = "twitch-relay";

/// Returns the data local directory for the application.
///
/// Returns `None` if the platform-specific directories cannot be determined.
pub fn data_dir() -> Option<PathBuf> {
    let dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)?;
    Some(dirs.data_local_dir().to_path_buf())
}

/// Returns the path to a file in the data directory.
///
/// Returns `None` if the data directory cannot be determined.
pub fn data_file_path(filename: &str) -> Option<PathBuf> {
    let dir = data_dir()?;
    Some(dir.join(filename))
}

/// Path to the channels.toml file.
pub fn channels_path() -> Option<PathBuf> {
    data_file_path("channels.toml")
}

/// Path to the recording_rules.json file.
pub fn recording_rules_path() -> Option<PathBuf> {
    data_file_path("recording_rules.json")
}

/// Path to the youtube_channels.toml file.
pub fn youtube_channels_path() -> Option<PathBuf> {
    data_file_path("youtube_channels.toml")
}

/// Path to the auth.toml file.
pub fn auth_path() -> Option<PathBuf> {
    data_file_path("auth.toml")
}

/// Path to the sessions.toml file.
pub fn sessions_path() -> Option<PathBuf> {
    data_file_path("sessions.toml")
}

/// Path to the twitch-account.toml file.
pub fn twitch_account_path() -> Option<PathBuf> {
    data_file_path("twitch-account.toml")
}

/// Path to the images directory for Twitch channels.
pub fn images_dir() -> Option<PathBuf> {
    data_file_path("images")
}

/// Path to the youtube_images directory for YouTube channel images.
pub fn youtube_images_dir() -> Option<PathBuf> {
    data_file_path("youtube_images")
}

/// Path to the youtube_watch_progress.json file.
pub fn youtube_watch_progress_path() -> Option<PathBuf> {
    data_file_path("youtube_watch_progress.json")
}
