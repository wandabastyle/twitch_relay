use std::{env, net::SocketAddr};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub startup: StartupConfig,
    pub auth: AuthConfig,
    pub playback: PlaybackConfig,
    pub recording: RecordingConfig,
    pub twitch_oauth: TwitchOAuthConfig,
    pub invidious: Option<InvidiousConfig>,
}

#[derive(Debug, Clone)]
pub struct StartupConfig {
    pub rotate_password: bool,
}

#[derive(Debug, Clone)]
pub struct InvidiousConfig {
    pub base_url: String,
    pub token: String,
    pub sid_cookie: Option<String>,
    pub basic_auth_user: Option<String>,
    pub basic_auth_password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub cookie_name: String,
    pub cookie_secure: bool,
}

#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    pub watch_ticket_ttl_secs: u64,
    pub streamlink_path: Option<String>,
    pub stream_resolver_mode: String,
    pub stream_delivery_mode: String,
    pub twitch_client_id: String,
}

#[derive(Debug, Clone)]
pub struct TwitchOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub token_encryption_key: String,
}

#[derive(Debug, Clone)]
pub struct RecordingConfig {
    pub recordings_dir: String,
    pub default_quality: String,
    pub poll_interval_secs: u64,
    pub start_live_confirmations: u64,
    pub stop_offline_confirmations: u64,
    pub write_nfo: bool,
    pub nfo_style: RecordingNfoStyle,
    pub ffmpeg_path: String,
    pub chapter_min_gap_secs: u64,
    pub chapter_change_confirmations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingNfoStyle {
    Tv,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let bind_addr = parse_socket_addr("BIND_ADDR")?
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 8080)));

        let startup = StartupConfig {
            rotate_password: parse_bool("TWITCH_RELAY_ROTATE_PASSWORD")?.unwrap_or(false),
        };

        let auth = AuthConfig {
            cookie_name: env::var("AUTH_COOKIE_NAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "twitch_relay_session".to_string()),
            cookie_secure: parse_bool("AUTH_COOKIE_SECURE")?.unwrap_or(false),
        };

        let playback = PlaybackConfig {
            watch_ticket_ttl_secs: parse_u64("WATCH_TICKET_TTL_SECS")?.unwrap_or(60),
            streamlink_path: env::var("STREAMLINK_PATH")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            stream_resolver_mode: env::var("STREAM_RESOLVER_MODE")
                .ok()
                .map(|v| v.trim().to_ascii_lowercase())
                .filter(|v| matches!(v.as_str(), "auto" | "native" | "streamlink"))
                .unwrap_or_else(|| "auto".to_string()),
            stream_delivery_mode: env::var("STREAM_DELIVERY_MODE")
                .ok()
                .map(|v| v.trim().to_ascii_lowercase())
                .filter(|v| matches!(v.as_str(), "cdn_first" | "relay"))
                .unwrap_or_else(|| "cdn_first".to_string()),
            twitch_client_id: env::var("TWITCH_CLIENT_ID")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "kimne78kx3ncx6brgo4mv6wki5h1ko".to_string()),
        };

        let twitch_oauth = TwitchOAuthConfig {
            client_id: parse_required_string("TWITCH_OAUTH_CLIENT_ID")?,
            client_secret: parse_required_string("TWITCH_OAUTH_CLIENT_SECRET")?,
            redirect_uri: parse_required_string("TWITCH_OAUTH_REDIRECT_URI")?,
            token_encryption_key: parse_required_string("TWITCH_TOKEN_ENCRYPTION_KEY")?,
        };

        let recording = RecordingConfig {
            recordings_dir: env::var("RECORDINGS_DIR")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "./recordings".to_string()),
            default_quality: env::var("RECORDING_DEFAULT_QUALITY")
                .ok()
                .map(|v| v.trim().to_ascii_lowercase())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "best".to_string()),
            poll_interval_secs: parse_u64("RECORDING_POLL_INTERVAL_SECS")?.unwrap_or(45),
            start_live_confirmations: parse_u64("RECORDING_START_LIVE_CONFIRMATIONS")?.unwrap_or(2),
            stop_offline_confirmations: parse_u64("RECORDING_STOP_OFFLINE_CONFIRMATIONS")?
                .unwrap_or(3),
            write_nfo: parse_bool("RECORDING_WRITE_NFO")?.unwrap_or(true),
            nfo_style: RecordingNfoStyle::Tv,
            ffmpeg_path: env::var("FFMPEG_PATH")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "ffmpeg".to_string()),
            chapter_min_gap_secs: parse_u64("RECORDING_CHAPTER_MIN_GAP_SECS")?.unwrap_or(180),
            chapter_change_confirmations: parse_u64("RECORDING_CHAPTER_CHANGE_CONFIRMATIONS")?
                .unwrap_or(2),
        };

        if recording.poll_interval_secs == 0 {
            return Err(AppError::Config(
                "invalid RECORDING_POLL_INTERVAL_SECS: must be >= 1".to_string(),
            ));
        }
        if recording.start_live_confirmations == 0 {
            return Err(AppError::Config(
                "invalid RECORDING_START_LIVE_CONFIRMATIONS: must be >= 1".to_string(),
            ));
        }
        if recording.stop_offline_confirmations == 0 {
            return Err(AppError::Config(
                "invalid RECORDING_STOP_OFFLINE_CONFIRMATIONS: must be >= 1".to_string(),
            ));
        }
        if recording.chapter_change_confirmations == 0 {
            return Err(AppError::Config(
                "invalid RECORDING_CHAPTER_CHANGE_CONFIRMATIONS: must be >= 1".to_string(),
            ));
        }

        // Invidious config is optional - YouTube mode will be disabled if not configured
        let invidious = match parse_invidious_config() {
            Ok(Some(config)) => Some(config),
            Ok(None) => None,
            Err(e) => return Err(e),
        };

        Ok(Self {
            bind_addr,
            startup,
            auth,
            playback,
            recording,
            twitch_oauth,
            invidious,
        })
    }
}

fn parse_invidious_config() -> Result<Option<InvidiousConfig>, AppError> {
    let base_url = match env::var("INVIDIOUS_BASE_URL") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return Ok(None),
    };

    let token = match env::var("INVIDIOUS_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            // If base_url is set but token is not, that's an error
            return Err(AppError::Config(
                "INVIDIOUS_BASE_URL is set but INVIDIOUS_TOKEN is missing".to_string(),
            ));
        }
    };

    // Validate base_url looks like a URL
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err(AppError::Config(
            "INVIDIOUS_BASE_URL must start with http:// or https://".to_string(),
        ));
    }

    // Optional SID cookie for Invidious auth (preferred over Bearer token when using basic auth)
    let sid_cookie = env::var("INVIDIOUS_SID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    // Optional basic auth credentials for reverse proxy
    let basic_auth_user = env::var("INVIDIOUS_BASIC_AUTH_USER")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let basic_auth_password = env::var("INVIDIOUS_BASIC_AUTH_PASSWORD")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    Ok(Some(InvidiousConfig {
        base_url,
        token,
        sid_cookie,
        basic_auth_user,
        basic_auth_password,
    }))
}

fn parse_required_string(name: &str) -> Result<String, AppError> {
    let value = env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::Config(format!("missing required env var {name}")))?;
    Ok(value)
}

fn parse_socket_addr(name: &str) -> Result<Option<SocketAddr>, AppError> {
    let Some(raw) = env::var(name).ok() else {
        return Ok(None);
    };

    raw.parse::<SocketAddr>()
        .map(Some)
        .map_err(|err| AppError::Config(format!("invalid {name}: {err}")))
}

pub(crate) fn parse_bool(name: &str) -> Result<Option<bool>, AppError> {
    let Some(raw) = env::var(name).ok() else {
        return Ok(None);
    };

    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(Some(true)),
        "0" | "false" | "off" | "no" => Ok(Some(false)),
        _ => Err(AppError::Config(format!(
            "invalid {name}: expected boolean"
        ))),
    }
}

fn parse_u64(name: &str) -> Result<Option<u64>, AppError> {
    let Some(raw) = env::var(name).ok() else {
        return Ok(None);
    };

    raw.parse::<u64>()
        .map(Some)
        .map_err(|err| AppError::Config(format!("invalid {name}: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    // Use a lock to prevent parallel test execution for env var tests
    fn env_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    const TEST_VAR: &str = "TEST_BOOL_VAR";

    #[test]
    fn parse_bool_unset_defaults_to_none() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Store any existing value
        let previous = env::var(TEST_VAR).ok();
        unsafe { env::remove_var(TEST_VAR) };

        let result = parse_bool(TEST_VAR).unwrap();
        assert_eq!(result, None);

        // Restore
        match previous {
            Some(value) => unsafe { env::set_var(TEST_VAR, value) },
            None => unsafe { env::remove_var(TEST_VAR) },
        }
    }

    #[test]
    fn parse_bool_truthy_values_parse_as_true() {
        let _lock = env_test_lock().lock().expect("env test lock");
        let truthy = ["true", "1", "yes", "on", "TRUE", "True", "YES", "ON"];

        for value in truthy {
            unsafe { env::set_var(TEST_VAR, value) };
            let result = parse_bool(TEST_VAR).unwrap();
            assert_eq!(result, Some(true), "expected '{}' to parse as true", value);
        }

        unsafe { env::remove_var(TEST_VAR) };
    }

    #[test]
    fn parse_bool_falsy_values_parse_as_false() {
        let _lock = env_test_lock().lock().expect("env test lock");
        let falsy = ["false", "0", "no", "off", "FALSE", "False", "NO", "OFF"];

        for value in falsy {
            unsafe { env::set_var(TEST_VAR, value) };
            let result = parse_bool(TEST_VAR).unwrap();
            assert_eq!(
                result,
                Some(false),
                "expected '{}' to parse as false",
                value
            );
        }

        unsafe { env::remove_var(TEST_VAR) };
    }

    #[test]
    fn parse_bool_invalid_returns_error() {
        let _lock = env_test_lock().lock().expect("env test lock");
        let invalid = ["maybe", "invalid", "2", ""];

        for value in invalid {
            unsafe { env::set_var(TEST_VAR, value) };
            let result = parse_bool(TEST_VAR);
            assert!(result.is_err(), "expected '{}' to return an error", value);
        }

        unsafe { env::remove_var(TEST_VAR) };
    }

    #[test]
    fn startup_config_from_env_defaults_rotate_password_false() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Save any existing value
        let previous = env::var("TWITCH_RELAY_ROTATE_PASSWORD").ok();
        unsafe { env::remove_var("TWITCH_RELAY_ROTATE_PASSWORD") };

        let result = parse_bool("TWITCH_RELAY_ROTATE_PASSWORD").unwrap();
        assert_eq!(result, None);

        // Restore
        match previous {
            Some(value) => unsafe { env::set_var("TWITCH_RELAY_ROTATE_PASSWORD", value) },
            None => unsafe { env::remove_var("TWITCH_RELAY_ROTATE_PASSWORD") },
        }
    }
}
