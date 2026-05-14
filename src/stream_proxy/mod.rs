use serde::Deserialize;

// Public exports - types other modules need
pub use self::session::{StreamProxyState, StreamSessionService};
pub use self::handlers::{proxy_manifest, proxy_variant_manifest, proxy_segment};

// Error type
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("stream not found")]
    StreamNotFound,
    #[error("session mismatch")]
    SessionMismatch,
    #[error("HLS fetch failed: {0}")]
    HlsFetchFailed(String),
}

// Query parameters
#[derive(Debug, Deserialize)]
pub struct RelayQuery {
    #[serde(default)]
    pub force: bool,
}

impl RelayQuery {
    pub fn force_relay(&self) -> bool {
        self.force
    }
}

// Module declarations
mod session;
mod resolver;
mod handlers;
