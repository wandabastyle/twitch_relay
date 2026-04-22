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

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let bind_addr = parse_socket_addr("BIND_ADDR")?
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 8080)));

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
