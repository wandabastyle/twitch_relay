// Core service types
pub use self::service::{ChatService, ChatState};
pub use self::service::{subscribe, unsubscribe, send, status, emotes, events};

// Internal modules - keep private
mod events;
mod emotes;
mod irc;
mod service;
