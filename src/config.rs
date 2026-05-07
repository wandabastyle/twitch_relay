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

    #[test]
    fn parse_bool_invalid_rotate_password_returns_config_error() {
        let _lock = env_test_lock().lock().expect("env test lock");
        const VAR_NAME: &str = "TWITCH_RELAY_ROTATE_PASSWORD";

        // Save any existing value
        let previous = env::var(VAR_NAME).ok();
        unsafe { env::set_var(VAR_NAME, "invalid_value") };

        let result = parse_bool(VAR_NAME);
        assert!(result.is_err());
        if let Err(AppError::Config(msg)) = result {
            assert!(msg.contains("TWITCH_RELAY_ROTATE_PASSWORD"));
            assert!(msg.contains("expected boolean"));
        } else {
            panic!("expected AppError::Config for invalid bool value");
        }

        // Restore
        match previous {
            Some(value) => unsafe { env::set_var(VAR_NAME, value) },
            None => unsafe { env::remove_var(VAR_NAME) },
        }
    }

    // ====================================================================
    // Invidious config tests
    // ====================================================================

    #[test]
    fn invidious_config_unset_returns_none() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Save existing values
        let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
        let prev_token = env::var("INVIDIOUS_TOKEN").ok();

        // Clear all Invidious env vars
        unsafe {
            env::remove_var("INVIDIOUS_BASE_URL");
            env::remove_var("INVIDIOUS_TOKEN");
        }

        let result = parse_invidious_config().unwrap();
        assert!(
            result.is_none(),
            "expected None when INVIDIOUS_BASE_URL is unset"
        );

        // Restore
        match prev_base {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
        }
        match prev_token {
            Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
            None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
        }
    }

    #[test]
    fn invidious_config_normal_mode_requires_token() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Save existing values
        let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
        let prev_token = env::var("INVIDIOUS_TOKEN").ok();

        // Set base URL but not token
        unsafe {
            env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
            env::remove_var("INVIDIOUS_TOKEN");
        }

        let result = parse_invidious_config();
        assert!(
            result.is_err(),
            "expected error when base_url set but token missing"
        );
        if let Err(AppError::Config(msg)) = result {
            assert!(
                msg.contains("INVIDIOUS_TOKEN"),
                "error should mention INVIDIOUS_TOKEN"
            );
        } else {
            panic!("expected AppError::Config");
        }

        // Restore
        match prev_base {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
        }
        match prev_token {
            Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
            None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
        }
    }

    #[test]
    fn invidious_config_normal_mode_with_token_ok() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Save existing values
        let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
        let prev_token = env::var("INVIDIOUS_TOKEN").ok();

        unsafe {
            env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
            env::set_var("INVIDIOUS_TOKEN", "test_token_123");
        }

        let result = parse_invidious_config().unwrap();
        assert!(result.is_some(), "expected Some config");
        let config = result.unwrap();
        assert_eq!(config.base_url, "https://invidious.example.com");
        assert_eq!(config.token, "test_token_123");
        assert!(config.sid_cookie.is_none());
        assert!(config.basic_auth_user.is_none());
        assert!(config.basic_auth_password.is_none());

        // Restore
        match prev_base {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
        }
        match prev_token {
            Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
            None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
        }
    }

    #[test]
    fn invidious_config_invalid_url_scheme_fails() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Save existing values
        let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
        let prev_token = env::var("INVIDIOUS_TOKEN").ok();

        unsafe {
            env::set_var("INVIDIOUS_BASE_URL", "ftp://invidious.example.com");
            env::set_var("INVIDIOUS_TOKEN", "test_token_123");
        }

        let result = parse_invidious_config();
        assert!(result.is_err(), "expected error for invalid URL scheme");
        if let Err(AppError::Config(msg)) = result {
            assert!(
                msg.contains("http://") || msg.contains("https://"),
                "error should mention http/https"
            );
        } else {
            panic!("expected AppError::Config");
        }

        // Restore
        match prev_base {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
        }
        match prev_token {
            Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
            None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
        }
    }

    #[test]
    fn invidious_config_basic_auth_requires_both_user_and_password() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Save existing values
        let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
        let prev_token = env::var("INVIDIOUS_TOKEN").ok();
        let prev_user = env::var("INVIDIOUS_BASIC_AUTH_USER").ok();
        let prev_pass = env::var("INVIDIOUS_BASIC_AUTH_PASSWORD").ok();
        let prev_sid = env::var("INVIDIOUS_SID").ok();

        // Test: user only, no password
        unsafe {
            env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
            env::set_var("INVIDIOUS_TOKEN", "test_token_123");
            env::set_var("INVIDIOUS_BASIC_AUTH_USER", "admin");
            env::remove_var("INVIDIOUS_BASIC_AUTH_PASSWORD");
            env::remove_var("INVIDIOUS_SID");
        }

        let result = parse_invidious_config();
        assert!(result.is_err(), "expected error when only user is set");
        if let Err(AppError::Config(msg)) = result {
            assert!(
                msg.contains("BASIC_AUTH_USER") && msg.contains("BASIC_AUTH_PASSWORD"),
                "error should mention both vars"
            );
        } else {
            panic!("expected AppError::Config");
        }

        // Test: password only, no user
        unsafe {
            env::remove_var("INVIDIOUS_BASIC_AUTH_USER");
            env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", "secret");
        }

        let result = parse_invidious_config();
        assert!(result.is_err(), "expected error when only password is set");
        if let Err(AppError::Config(msg)) = result {
            assert!(
                msg.contains("BASIC_AUTH_USER") && msg.contains("BASIC_AUTH_PASSWORD"),
                "error should mention both vars"
            );
        } else {
            panic!("expected AppError::Config");
        }

        // Restore
        match prev_base {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
        }
        match prev_token {
            Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
            None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
        }
        match prev_user {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_USER", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_USER") },
        }
        match prev_pass {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_PASSWORD") },
        }
        match prev_sid {
            Some(v) => unsafe { env::set_var("INVIDIOUS_SID", v) },
            None => unsafe { env::remove_var("INVIDIOUS_SID") },
        }
    }

    #[test]
    fn invidious_config_basic_auth_requires_sid() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Save existing values
        let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
        let prev_token = env::var("INVIDIOUS_TOKEN").ok();
        let prev_user = env::var("INVIDIOUS_BASIC_AUTH_USER").ok();
        let prev_pass = env::var("INVIDIOUS_BASIC_AUTH_PASSWORD").ok();
        let prev_sid = env::var("INVIDIOUS_SID").ok();

        // Set Basic auth but no SID
        unsafe {
            env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
            env::set_var("INVIDIOUS_TOKEN", "test_token_123");
            env::set_var("INVIDIOUS_BASIC_AUTH_USER", "admin");
            env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", "secret");
            env::remove_var("INVIDIOUS_SID");
        }

        let result = parse_invidious_config();
        assert!(
            result.is_err(),
            "expected error when Basic auth set but SID missing"
        );
        if let Err(AppError::Config(msg)) = result {
            assert!(
                msg.contains("INVIDIOUS_SID"),
                "error should mention INVIDIOUS_SID"
            );
        } else {
            panic!("expected AppError::Config");
        }

        // Restore
        match prev_base {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
        }
        match prev_token {
            Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
            None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
        }
        match prev_user {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_USER", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_USER") },
        }
        match prev_pass {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_PASSWORD") },
        }
        match prev_sid {
            Some(v) => unsafe { env::set_var("INVIDIOUS_SID", v) },
            None => unsafe { env::remove_var("INVIDIOUS_SID") },
        }
    }

    #[test]
    fn invidious_config_reverse_proxy_mode_ok() {
        let _lock = env_test_lock().lock().expect("env test lock");

        // Save existing values
        let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
        let prev_token = env::var("INVIDIOUS_TOKEN").ok();
        let prev_user = env::var("INVIDIOUS_BASIC_AUTH_USER").ok();
        let prev_pass = env::var("INVIDIOUS_BASIC_AUTH_PASSWORD").ok();
        let prev_sid = env::var("INVIDIOUS_SID").ok();

        // Set all reverse-proxy mode vars, no token needed
        unsafe {
            env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
            env::remove_var("INVIDIOUS_TOKEN");
            env::set_var("INVIDIOUS_BASIC_AUTH_USER", "admin");
            env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", "secret");
            env::set_var("INVIDIOUS_SID", "sid_session_123");
        }

        let result = parse_invidious_config().unwrap();
        assert!(result.is_some(), "expected Some config");
        let config = result.unwrap();
        assert_eq!(config.base_url, "https://invidious.example.com");
        assert_eq!(config.token, ""); // Token is optional in reverse-proxy mode
        assert_eq!(config.sid_cookie, Some("sid_session_123".to_string()));
        assert_eq!(config.basic_auth_user, Some("admin".to_string()));
        assert_eq!(config.basic_auth_password, Some("secret".to_string()));

        // Restore
        match prev_base {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
        }
        match prev_token {
            Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
            None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
        }
        match prev_user {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_USER", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_USER") },
        }
        match prev_pass {
            Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", v) },
            None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_PASSWORD") },
        }
        match prev_sid {
            Some(v) => unsafe { env::set_var("INVIDIOUS_SID", v) },
            None => unsafe { env::remove_var("INVIDIOUS_SID") },
        }
    }
}
