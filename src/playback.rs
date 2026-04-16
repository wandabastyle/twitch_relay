use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use rand::{distributions::Alphanumeric, Rng};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PlaybackTicketService {
    channels: Arc<RwLock<Vec<String>>>,
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
    #[error("unknown channel")]
    UnknownChannel,
    #[error("invalid watch ticket")]
    InvalidTicket,
    #[error("expired watch ticket")]
    ExpiredTicket,
    #[error("watch ticket does not belong to this session")]
    SessionMismatch,
}

impl PlaybackTicketService {
    pub fn new(channels: Vec<String>, ttl_secs: u64) -> Self {
        let channels = channels
            .into_iter()
            .map(|channel| channel.trim().to_ascii_lowercase())
            .filter(|channel| !channel.is_empty())
            .collect::<Vec<_>>();

        Self {
            channels: Arc::new(RwLock::new(channels)),
            ttl_secs: ttl_secs.max(10),
            tickets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn channel_list(&self) -> Vec<String> {
        self.channels
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn add_channel(&self, login: &str) -> bool {
        let normalized = login.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return false;
        }

        if let Ok(mut guard) = self.channels.write()
            && !guard.contains(&normalized)
        {
            guard.push(normalized);
            return true;
        }
        false
    }

    pub fn remove_channel(&self, login: &str) -> bool {
        let normalized = login.trim().to_ascii_lowercase();
        if let Ok(mut guard) = self.channels.write() {
            let len_before = guard.len();
            guard.retain(|c| c != &normalized);
            return guard.len() < len_before;
        }
        false
    }

    pub fn issue_ticket(
        &self,
        session_token: &str,
        channel_login: &str,
    ) -> Result<String, PlaybackTicketError> {
        let normalized_channel = channel_login.trim().to_ascii_lowercase();
        let has_channel = self
            .channels
            .read()
            .map(|guard| guard.contains(&normalized_channel))
            .unwrap_or(false);

        if !has_channel {
            return Err(PlaybackTicketError::UnknownChannel);
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
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
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
