use serde::Deserialize;

// Public exports - types other modules need
pub use self::handlers::{
   proxy_manifest,
   proxy_segment,
   proxy_variant_manifest,
};
pub use self::session::{
   StreamProxyState,
   StreamSessionService,
};

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
   pub const fn force_relay(&self) -> bool {
      self.force
   }
}

// Module declarations
mod handlers;
mod resolver;
mod session;
