// Core service types
pub use self::service::{ChatService, ChatState};
pub use self::service::{emotes, events, send, status, subscribe, unsubscribe};

// Internal modules - keep private
mod emotes;
mod events;
mod irc;
mod service;
