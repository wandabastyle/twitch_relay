use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use rand::{Rng, distr::Alphanumeric};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PlaybackTicketService {
    ttl_secs: u64,
    tickets: Arc<RwLock<HashMap<String, WatchTicket>>>,
}

#[derive(Debug, Clone)]
pub struct ValidatedWatch {
    pub channel_login: String,
}

#[derive(Debug, Clone)]
struct WatchTicket {
    channel_login: String,
    session_token: String,
    expires_at_unix: u64,
}

#[derive(Debug, Error)]
pub enum PlaybackTicketError {
    #[error("invalid watch ticket")]
    InvalidTicket,
    #[error("expired watch ticket")]
    ExpiredTicket,
    #[error("watch ticket does not belong to this session")]
    SessionMismatch,
}

impl PlaybackTicketService {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            ttl_secs: ttl_secs.max(10),
            tickets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn issue_ticket(
        &self,
        session_token: &str,
        channel_login: &str,
    ) -> Result<String, PlaybackTicketError> {
        let normalized_channel = channel_login.trim().to_ascii_lowercase();
        if normalized_channel.is_empty() {
            return Err(PlaybackTicketError::InvalidTicket);
        }

        let now = now_unix_secs();
        let expires_at_unix = now.saturating_add(self.ttl_secs);
        let ticket_value = generate_ticket(48);

        let ticket = WatchTicket {
            channel_login: normalized_channel,
            session_token: session_token.to_string(),
            expires_at_unix,
        };

        let mut guard = self
            .tickets
            .write()
            .map_err(|_| PlaybackTicketError::InvalidTicket)?;
        guard.retain(|_, ticket| ticket.expires_at_unix > now);
        guard.insert(ticket_value.clone(), ticket);

        Ok(ticket_value)
    }

    pub fn validate_ticket(
        &self,
        ticket_value: &str,
        session_token: &str,
    ) -> Result<ValidatedWatch, PlaybackTicketError> {
        let now = now_unix_secs();
        let mut guard = self
            .tickets
            .write()
            .map_err(|_| PlaybackTicketError::InvalidTicket)?;

        guard.retain(|_, ticket| ticket.expires_at_unix > now);

        let Some(ticket) = guard.get(ticket_value).cloned() else {
            return Err(PlaybackTicketError::InvalidTicket);
        };

        if ticket.expires_at_unix <= now {
            return Err(PlaybackTicketError::ExpiredTicket);
        }

        if ticket.session_token != session_token {
            return Err(PlaybackTicketError::SessionMismatch);
        }

        Ok(ValidatedWatch {
            channel_login: ticket.channel_login,
        })
    }
}

fn generate_ticket(length: usize) -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
