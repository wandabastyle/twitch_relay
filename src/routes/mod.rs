pub mod auth;
pub mod channels;
pub mod chat;
pub mod error;
pub mod health;
pub mod recordings;
pub mod stream;
pub mod twitch;
pub mod watch;

// Explicit exports - no wildcards
pub use auth::auth_routes;
pub use channels::{
   ChannelState,
   LiveStatusState,
   channel_routes,
   live_status_routes,
};
pub use chat::chat_routes;
pub use health::{
   APP_VERSION,
   health_routes,
};
pub use recordings::{
   RecordingState,
   recording_routes,
};
pub use stream::stream_routes;
pub use twitch::twitch_routes;
pub use watch::watch_routes;
