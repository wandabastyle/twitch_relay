use std::{env, net::SocketAddr, str::FromStr};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamResolverMode {
    #[default]
    Auto,
    Native,
    Streamlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamDeliveryMode {
    #[default]
    CdnFirst,
    Relay,
}

impl FromStr for StreamResolverMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "native" => Ok(Self::Native),
            "streamlink" => Ok(Self::Streamlink),
            _ => Err(format!(
                "invalid STREAM_RESOLVER_MODE: '{}'. Must be one of: auto, native, streamlink",
                s
            )),
        }
    }
}

impl FromStr for StreamDeliveryMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "cdn_first" => Ok(Self::CdnFirst),
            "relay" => Ok(Self::Relay),
            _ => Err(format!(
                "invalid STREAM_DELIVERY_MODE: '{}'. Must be one of: cdn_first, relay",
                s
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    pub watch_ticket_ttl_secs: u64,
    pub streamlink_path: Option<String>,
    pub stream_resolver_mode: StreamResolverMode,
    pub stream_delivery_mode: StreamDeliveryMode,
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
            stream_resolver_mode: parse_enum("STREAM_RESOLVER_MODE")?.unwrap_or_default(),
            stream_delivery_mode: parse_enum("STREAM_DELIVERY_MODE")?.unwrap_or_default(),
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

/// Parses Invidious configuration from environment variables.
///
/// This app supports two Invidious auth modes:
///
/// 1. Normal mode: Direct Invidious API auth using `Authorization: Bearer <token>`.
///    Requires: `INVIDIOUS_BASE_URL` + `INVIDIOUS_TOKEN`
///
/// 2. Reverse-proxy Basic auth mode: Invidious is behind a reverse proxy that
///    uses HTTP Basic auth. The `Authorization` header is used for the proxy,
///    so the Invidious session must be sent via the `SID` cookie instead of
///    the Bearer `Authorization` header.
///    Requires: `INVIDIOUS_BASE_URL` + `INVIDIOUS_BASIC_AUTH_USER` +
///    `INVIDIOUS_BASIC_AUTH_PASSWORD` + `INVIDIOUS_SID`
///
/// In reverse-proxy Basic auth mode, `INVIDIOUS_TOKEN` is not required because
/// the Invidious session is represented by `INVIDIOUS_SID`. `INVIDIOUS_TOKEN`
/// may still be set for compatibility, but it is not the active Invidious
/// credential when Basic auth is configured.
fn parse_invidious_config() -> Result<Option<InvidiousConfig>, AppError> {
    let base_url = match env::var("INVIDIOUS_BASE_URL") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return Ok(None),
    };

    // Validate base_url looks like a URL
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err(AppError::Config(
            "INVIDIOUS_BASE_URL must start with http:// or https://".to_string(),
        ));
    }

    // Optional SID cookie for Invidious auth.
    // When reverse-proxy Basic auth is enabled, the outbound request's
    // `Authorization` header is used for Basic auth. In that mode, the
    // Invidious account/session credential must be sent via the `SID` cookie
    // instead of the Bearer `Authorization` header.
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

    // Basic auth user and password must both be set or neither
    let has_basic_auth_user = basic_auth_user.is_some();
    let has_basic_auth_password = basic_auth_password.is_some();

    if has_basic_auth_user != has_basic_auth_password {
        return Err(AppError::Config(
            "INVIDIOUS_BASIC_AUTH_USER and INVIDIOUS_BASIC_AUTH_PASSWORD must both be set or neither".to_string(),
        ));
    }

    let basic_auth_configured = has_basic_auth_user && has_basic_auth_password;

    // When Basic auth is configured, require SID for Invidious session auth
    if basic_auth_configured && sid_cookie.is_none() {
        return Err(AppError::Config(
            "INVIDIOUS_SID is required when INVIDIOUS_BASIC_AUTH_USER and INVIDIOUS_BASIC_AUTH_PASSWORD are set".to_string(),
        ));
    }

    // INVIDIOUS_TOKEN is required for normal mode, optional in Basic auth mode
    let token = match env::var("INVIDIOUS_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            if !basic_auth_configured {
                // If base_url is set but token is not, and we're not in Basic auth mode, that's an error
                return Err(AppError::Config(
                    "INVIDIOUS_BASE_URL is set but INVIDIOUS_TOKEN is missing".to_string(),
                ));
            }
            // In Basic auth mode, token is optional (SID is the active credential)
            String::new()
        }
    };

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

fn parse_enum<T: FromStr>(name: &str) -> Result<Option<T>, AppError>
where
    T::Err: ToString,
{
    let Some(raw) = env::var(name).ok() else {
        return Ok(None);
    };

    if raw.trim().is_empty() {
        return Ok(None);
    }

    raw.parse::<T>()
        .map(Some)
        .map_err(|err| AppError::Config(err.to_string()))
}

#[cfg(test)]
mod tests;
