use std::{env, net::SocketAddr};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub auth: AuthConfig,
    pub playback: PlaybackConfig,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub access_code: String,
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub session_ttl_secs: u64,
    pub login_window_secs: u64,
    pub max_login_attempts: u32,
    pub login_block_secs: u64,
}

#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    pub channels: Vec<String>,
    pub watch_ticket_ttl_secs: u64,
    pub streamlink_path: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let bind_addr = parse_socket_addr("BIND_ADDR")?
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 8080)));

        let auth = AuthConfig {
            access_code: env::var("AUTH_ACCESS_CODE")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "change-me".to_string()),
            cookie_name: env::var("AUTH_COOKIE_NAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "twitch_relay_session".to_string()),
            cookie_secure: parse_bool("AUTH_COOKIE_SECURE")?.unwrap_or(false),
            session_ttl_secs: parse_u64("AUTH_SESSION_TTL_SECS")?.unwrap_or(60 * 60 * 24 * 30),
            login_window_secs: parse_u64("AUTH_LOGIN_WINDOW_SECS")?.unwrap_or(60),
            max_login_attempts: parse_u32("AUTH_MAX_LOGIN_ATTEMPTS")?.unwrap_or(6),
            login_block_secs: parse_u64("AUTH_LOGIN_BLOCK_SECS")?.unwrap_or(5 * 60),
        };

        if auth.access_code == "change-me" {
            tracing::warn!(
                "AUTH_ACCESS_CODE is using default value; set a strong secret before production use"
            );
        }

        let playback = PlaybackConfig {
            channels: parse_list("TWITCH_CHANNELS")
                .unwrap_or_else(|| vec!["demo_channel".to_string()]),
            watch_ticket_ttl_secs: parse_u64("WATCH_TICKET_TTL_SECS")?.unwrap_or(60),
            streamlink_path: env::var("STREAMLINK_PATH")
                .ok()
                .filter(|v| !v.trim().is_empty()),
        };

        Ok(Self {
            bind_addr,
            auth,
            playback,
        })
    }
}

fn parse_socket_addr(name: &str) -> Result<Option<SocketAddr>, AppError> {
    let Some(raw) = env::var(name).ok() else {
        return Ok(None);
    };

    raw.parse::<SocketAddr>()
        .map(Some)
        .map_err(|err| AppError::Config(format!("invalid {name}: {err}")))
}

fn parse_bool(name: &str) -> Result<Option<bool>, AppError> {
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

fn parse_u32(name: &str) -> Result<Option<u32>, AppError> {
    let Some(raw) = env::var(name).ok() else {
        return Ok(None);
    };

    raw.parse::<u32>()
        .map(Some)
        .map_err(|err| AppError::Config(format!("invalid {name}: {err}")))
}

fn parse_list(name: &str) -> Option<Vec<String>> {
    let raw = env::var(name).ok()?;
    let values = raw
        .split(',')
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}
