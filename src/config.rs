use std::{env, net::SocketAddr};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let bind_addr = env::var("BIND_ADDR")
            .ok()
            .and_then(|value| value.parse::<SocketAddr>().ok())
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 8080)));

        Ok(Self { bind_addr })
    }
}
