use std::{
   collections::HashMap,
   convert::Infallible,
   sync::{
      Arc,
      atomic::AtomicU64,
   },
   time::Duration,
};

use axum::{
   Json,
   extract::{
      Path,
      Query,
      State,
   },
   http::StatusCode,
   response::{
      IntoResponse,
      Response,
      sse::{
         Event,
         KeepAlive,
         Sse,
      },
   },
};
use futures_util::stream::{
   self,
   StreamExt,
};
use serde::Serialize;
use tokio::{
   sync::{
      RwLock,
      broadcast,
      mpsc,
      oneshot,
   },
   time::timeout,
};
use tokio_stream::wrappers::BroadcastStream;

use crate::{
   chat::{
      emotes::{
         CachedEmoteEntry,
         CachedOwnerName,
         EmotePickerItem,
         EmotePickerResponse,
         emotes_for_channel_with_account,
      },
      events::{
         ChatChannelRequest,
         ChatChannelStatus,
         ChatSendRequest,
         ChatStatusQuery,
         ChatStatusResponse,
         ChatStreamEvent,
         EmotesQuery,
      },
      irc::run_chat_manager,
   },
   routes::error::error_response,
   twitch_auth::TwitchAuthService,
   util::channel::normalize_channel_login,
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChatError {
   #[error("chat runtime is not available")]
   RuntimeUnavailable,
   #[error("chat command queue is full")]
   CommandQueueFull,
   #[error("chat runtime did not return status")]
   StatusTimeout,
   #[error("channel not found")]
   ChannelNotFound,
   #[error("message cannot be empty")]
   EmptyMessage,
   #[error("message is too long")]
   MessageTooLong,
   #[error("invalid channel name")]
   InvalidChannelName,
   #[error("chat connection unavailable: {0}")]
   ConnectionUnavailable(String),
   #[error("chat websocket connect failed: {0}")]
   WebSocketConnectFailed(String),
   #[error("chat PASS failed: {0}")]
   PassFailed(String),
   #[error("chat NICK failed: {0}")]
   NickFailed(String),
   #[error("chat writer is not available")]
   WriterUnavailable,
   #[error(
      "missing Twitch emote scope. disconnect and reconnect Twitch account to grant \
       user:read:emotes"
   )]
   MissingEmoteScope,
   #[error("{0}")]
   Other(String),
}

impl From<String> for ChatError {
   fn from(msg: String) -> Self {
      // Try to parse known error patterns
      if msg.contains("user:read:emotes") {
         Self::MissingEmoteScope
      } else {
         Self::Other(msg)
      }
   }
}

#[derive(Debug, Clone)]
pub struct ChatService {
   auth:                             TwitchAuthService,
   command_tx:                       mpsc::Sender<ChatCommand>,
   channels: Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
   emote_cache:                      Arc<RwLock<HashMap<String, CachedEmoteEntry>>>,
   owner_name_cache:                 Arc<RwLock<HashMap<String, CachedOwnerName>>>,
   owner_lookup_cooldown_until_unix: Arc<AtomicU64>,
   metrics:                          ChatMetrics,
}

#[derive(Debug, Clone)]
pub struct ChatState {
   pub service: ChatService,
}

#[derive(Debug, Clone)]
pub struct ChatMetrics {
   pub(crate) connected:           Arc<std::sync::atomic::AtomicBool>,
   pub(crate) connection_attempts: Arc<AtomicU64>,
   pub(crate) connection_failures: Arc<AtomicU64>,
   pub(crate) reconnects:          Arc<AtomicU64>,
   pub(crate) joins:               Arc<AtomicU64>,
   pub(crate) disconnects:         Arc<AtomicU64>,
   pub(crate) command_queue_full:  Arc<AtomicU64>,
   pub(crate) event_queue_full:    Arc<AtomicU64>,
   pub(crate) last_attempt_ms:     Arc<AtomicU64>,
   pub(crate) sse_broadcast_lag:   Arc<AtomicU64>,
}

#[derive(Debug, Serialize)]
pub struct ChatMetricsSnapshot {
   connected:           bool,
   connection_attempts: u64,
   connection_failures: u64,
   reconnects:          u64,
   joins:               u64,
   disconnects:         u64,
   command_queue_full:  u64,
   event_queue_full:    u64,
   last_attempt_ms:     u64,
   sse_broadcast_lag:   u64,
}

impl ChatMetrics {
   fn new() -> Self {
      Self {
         connected:           Arc::new(std::sync::atomic::AtomicBool::new(false)),
         connection_attempts: Arc::new(AtomicU64::new(0)),
         connection_failures: Arc::new(AtomicU64::new(0)),
         reconnects:          Arc::new(AtomicU64::new(0)),
         joins:               Arc::new(AtomicU64::new(0)),
         disconnects:         Arc::new(AtomicU64::new(0)),
         command_queue_full:  Arc::new(AtomicU64::new(0)),
         event_queue_full:    Arc::new(AtomicU64::new(0)),
         last_attempt_ms:     Arc::new(AtomicU64::new(0)),
         sse_broadcast_lag:   Arc::new(AtomicU64::new(0)),
      }
   }

   pub fn snapshot(&self) -> ChatMetricsSnapshot {
      use std::sync::atomic::Ordering;
      ChatMetricsSnapshot {
         connected:           self.connected.load(Ordering::Relaxed),
         connection_attempts: self.connection_attempts.load(Ordering::Relaxed),
         connection_failures: self.connection_failures.load(Ordering::Relaxed),
         reconnects:          self.reconnects.load(Ordering::Relaxed),
         joins:               self.joins.load(Ordering::Relaxed),
         disconnects:         self.disconnects.load(Ordering::Relaxed),
         command_queue_full:  self.command_queue_full.load(Ordering::Relaxed),
         event_queue_full:    self.event_queue_full.load(Ordering::Relaxed),
         last_attempt_ms:     self.last_attempt_ms.load(Ordering::Relaxed),
         sse_broadcast_lag:   self.sse_broadcast_lag.load(Ordering::Relaxed),
      }
   }

   fn prometheus(&self) -> String {
      let snapshot = self.snapshot();
      format!(
         concat!(
            "# HELP twitch_relay_chat_connected Whether the IRC connection completed Twitch \
             welcome.\n",
            "# TYPE twitch_relay_chat_connected gauge\n",
            "twitch_relay_chat_connected {}\n",
            "# HELP twitch_relay_chat_connection_attempts_total IRC connection attempts.\n",
            "# TYPE twitch_relay_chat_connection_attempts_total counter\n",
            "twitch_relay_chat_connection_attempts_total {}\n",
            "# HELP twitch_relay_chat_connection_failures_total Failed IRC connection attempts.\n",
            "# TYPE twitch_relay_chat_connection_failures_total counter\n",
            "twitch_relay_chat_connection_failures_total {}\n",
            "# HELP twitch_relay_chat_reconnects_total IRC reconnect attempts.\n",
            "# TYPE twitch_relay_chat_reconnects_total counter\n",
            "twitch_relay_chat_reconnects_total {}\n",
            "# HELP twitch_relay_chat_joins_total Confirmed Twitch channel joins.\n",
            "# TYPE twitch_relay_chat_joins_total counter\n",
            "twitch_relay_chat_joins_total {}\n",
            "# HELP twitch_relay_chat_disconnects_total IRC disconnects.\n",
            "# TYPE twitch_relay_chat_disconnects_total counter\n",
            "twitch_relay_chat_disconnects_total {}\n",
            "# HELP twitch_relay_chat_command_queue_full_total Rejected commands due to a full \
             queue.\n",
            "# TYPE twitch_relay_chat_command_queue_full_total counter\n",
            "twitch_relay_chat_command_queue_full_total {}\n",
            "# HELP twitch_relay_chat_event_queue_full_total Saturated inbound IRC event queues.\n",
            "# TYPE twitch_relay_chat_event_queue_full_total counter\n",
            "twitch_relay_chat_event_queue_full_total {}\n",
            "# HELP twitch_relay_chat_sse_broadcast_lag_total SSE events skipped by lagging \
             clients.\n",
            "# TYPE twitch_relay_chat_sse_broadcast_lag_total counter\n",
            "twitch_relay_chat_sse_broadcast_lag_total {}\n",
            "# HELP twitch_relay_chat_last_connection_attempt_milliseconds Duration of the latest \
             connection attempt.\n",
            "# TYPE twitch_relay_chat_last_connection_attempt_milliseconds gauge\n",
            "twitch_relay_chat_last_connection_attempt_milliseconds {}\n"
         ),
         u8::from(snapshot.connected),
         snapshot.connection_attempts,
         snapshot.connection_failures,
         snapshot.reconnects,
         snapshot.joins,
         snapshot.disconnects,
         snapshot.command_queue_full,
         snapshot.event_queue_full,
         snapshot.sse_broadcast_lag,
         snapshot.last_attempt_ms,
      )
   }
}

#[derive(Debug)]
pub enum ChatCommand {
   Subscribe {
      channel:  String,
      response: oneshot::Sender<Result<(), ChatError>>,
   },
   Unsubscribe {
      channel:  String,
      response: oneshot::Sender<Result<(), ChatError>>,
   },
   SendMessage {
      channel:  String,
      message:  String,
      response: oneshot::Sender<Result<(), ChatError>>,
   },
   Status {
      channel:  String,
      response: oneshot::Sender<ChatChannelStatus>,
   },
}

impl ChatService {
   pub fn new(auth: TwitchAuthService) -> Self {
      let (command_tx, command_rx) = mpsc::channel(128);
      let channels = Arc::new(RwLock::new(HashMap::new()));
      let emote_cache = Arc::new(RwLock::new(HashMap::new()));
      let third_party_emote_cache = Arc::new(RwLock::new(HashMap::new()));
      let owner_name_cache = Arc::new(RwLock::new(HashMap::new()));
      let owner_lookup_cooldown_until_unix = Arc::new(AtomicU64::new(0));
      let metrics = ChatMetrics::new();

      tokio::spawn(run_chat_manager(
         auth.clone(),
         command_rx,
         channels.clone(),
         emote_cache.clone(),
         third_party_emote_cache,
         metrics.clone(),
      ));

      Self {
         auth,
         command_tx,
         channels,
         emote_cache,
         owner_name_cache,
         owner_lookup_cooldown_until_unix,
         metrics,
      }
   }

   pub async fn subscribe_channel(&self, channel: &str) -> Result<(), ChatError> {
      let (tx, rx) = oneshot::channel();
      let normalized =
         normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
      tracing::info!(channel = %normalized, "chat subscribe requested");
      self
         .command_tx
         .try_send(ChatCommand::Subscribe {
            channel:  normalized.clone(),
            response: tx,
         })
         .map_err(|error| self.command_send_error(&error))?;
      let result = timeout(Duration::from_secs(5), rx)
         .await
         .map_err(|_| ChatError::StatusTimeout)?
         .map_err(|_| ChatError::StatusTimeout)?;
      if result.is_ok() {
         tracing::info!(channel = %normalized, "chat subscribe completed");
      }
      result
   }

   pub async fn unsubscribe_channel(&self, channel: &str) -> Result<(), ChatError> {
      let (tx, rx) = oneshot::channel();
      let normalized =
         normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
      tracing::info!(channel = %normalized, "chat unsubscribe requested");
      self
         .command_tx
         .try_send(ChatCommand::Unsubscribe {
            channel:  normalized.clone(),
            response: tx,
         })
         .map_err(|error| self.command_send_error(&error))?;
      let result = timeout(Duration::from_secs(5), rx)
         .await
         .map_err(|_| ChatError::StatusTimeout)?
         .map_err(|_| ChatError::StatusTimeout)?;
      if result.is_ok() {
         tracing::info!(channel = %normalized, "chat unsubscribe completed");
      }
      result
   }

   pub async fn send_message(&self, channel: &str, message: &str) -> Result<(), ChatError> {
      let trimmed = message.trim();
      if trimmed.is_empty() {
         return Err(ChatError::EmptyMessage);
      }
      if trimmed.chars().count() > 500 {
         return Err(ChatError::MessageTooLong);
      }

      let (tx, rx) = oneshot::channel();
      let normalized =
         normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
      self
         .command_tx
         .try_send(ChatCommand::SendMessage {
            channel:  normalized.clone(),
            message:  trimmed.to_string(),
            response: tx,
         })
         .map_err(|error| self.command_send_error(&error))?;
      timeout(Duration::from_secs(5), rx)
         .await
         .map_err(|_| ChatError::StatusTimeout)?
         .map_err(|_| ChatError::StatusTimeout)?
   }

   pub async fn status(&self, channel: &str) -> Result<ChatChannelStatus, ChatError> {
      let (tx, rx) = oneshot::channel();
      let normalized =
         normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
      self
         .command_tx
         .try_send(ChatCommand::Status {
            channel:  normalized.clone(),
            response: tx,
         })
         .map_err(|error| self.command_send_error(&error))?;
      timeout(Duration::from_secs(2), rx)
         .await
         .map_err(|_| ChatError::StatusTimeout)?
         .map_err(|_| ChatError::StatusTimeout)
   }

   pub async fn receiver_for_channel(
      &self,
      channel: &str,
   ) -> Result<broadcast::Receiver<ChatStreamEvent>, ChatError> {
      let normalized =
         normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
      let sender = {
         let mut guard = self.channels.write().await;
         guard
            .entry(normalized.clone())
            .or_insert_with(|| {
               let (sender, _receiver) = broadcast::channel(256);
               sender
            })
            .clone()
      };

      let receiver = sender.subscribe();
      tracing::info!(
         channel = %normalized,
         receivers = sender.receiver_count(),
         "chat EventSource receiver subscribed"
      );
      Ok(receiver)
   }

   pub async fn emotes_for_channel(
      &self,
      channel: &str,
   ) -> Result<Vec<EmotePickerItem>, ChatError> {
      let normalized_channel =
         normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
      let account = self.auth.ensure_emote_account().await.map_err(|e| {
         if e.contains("user:read:emotes") {
            ChatError::MissingEmoteScope
         } else {
            ChatError::Other(e)
         }
      })?;

      emotes_for_channel_with_account(
         &self.emote_cache,
         &self.owner_name_cache,
         &self.owner_lookup_cooldown_until_unix,
         &self.auth,
         &normalized_channel,
         &account,
      )
      .await
      .map_err(|e| {
         if e.contains("channel not found") {
            ChatError::ChannelNotFound
         } else {
            ChatError::Other(e)
         }
      })
   }

   pub async fn prewarm_emotes_for_channels(&self, channels: &[String]) -> Result<(), ChatError> {
      let account = self.auth.ensure_emote_account().await.map_err(|e| {
         if e.contains("user:read:emotes") {
            ChatError::MissingEmoteScope
         } else {
            ChatError::Other(e)
         }
      })?;

      let targets: Vec<String> = channels
         .iter()
         .map(|channel| channel.trim().to_ascii_lowercase())
         .filter(|channel| !channel.is_empty())
         .collect();

      stream::iter(targets)
            .map(|channel| {
                let account = account.clone();
                let emote_cache = self.emote_cache.clone();
                let owner_name_cache = self.owner_name_cache.clone();
                let cooldown = self.owner_lookup_cooldown_until_unix.clone();
                let auth = self.auth.clone();
                async move {
                    (
                        channel.clone(),
                        emotes_for_channel_with_account(
                            &emote_cache,
                            &owner_name_cache,
                            &cooldown,
                            &auth,
                            &channel,
                            &account,
                        ).await,
                    )
                }
            })
            .buffer_unordered(2)
            .for_each(|(channel, result)| async move {
                if let Err(error) = result {
                    tracing::debug!(error = %error, channel = %channel, "failed prewarming emotes for channel");
                }
            })
            .await;

      Ok(())
   }

   fn command_send_error(&self, error: &mpsc::error::TrySendError<ChatCommand>) -> ChatError {
      match error {
         mpsc::error::TrySendError::Full(_) => {
            self
               .metrics
               .command_queue_full
               .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            ChatError::CommandQueueFull
         },
         mpsc::error::TrySendError::Closed(_) => ChatError::RuntimeUnavailable,
      }
   }
}

pub async fn subscribe(
   State(state): State<ChatState>,
   Json(payload): Json<ChatChannelRequest>,
) -> Response {
   tracing::debug!(channel = %payload.channel_login, "chat subscribe request");
   match state
      .service
      .subscribe_channel(&payload.channel_login)
      .await
   {
      Ok(()) => StatusCode::NO_CONTENT.into_response(),
      Err(e) => {
         tracing::warn!(error = %e, channel = %payload.channel_login, "failed subscribing chat channel");
         error_response(StatusCode::BAD_REQUEST, &e.to_string(), None)
      },
   }
}

pub async fn unsubscribe(State(state): State<ChatState>, Path(channel): Path<String>) -> Response {
   tracing::debug!(channel = %channel, "chat unsubscribe request");
   match state.service.unsubscribe_channel(&channel).await {
      Ok(()) => StatusCode::NO_CONTENT.into_response(),
      Err(e) => {
         tracing::warn!(error = %e, channel = %channel, "failed unsubscribing chat channel");
         error_response(StatusCode::BAD_REQUEST, &e.to_string(), None)
      },
   }
}

pub async fn send(
   State(state): State<ChatState>,
   Json(payload): Json<ChatSendRequest>,
) -> Response {
   match state
      .service
      .send_message(&payload.channel_login, &payload.message)
      .await
   {
      Ok(()) => StatusCode::NO_CONTENT.into_response(),
      Err(e) => {
         tracing::warn!(error = %e, channel = %payload.channel_login, "failed sending chat message");
         error_response(StatusCode::BAD_REQUEST, &e.to_string(), None)
      },
   }
}

pub async fn status(
   State(state): State<ChatState>,
   Query(query): Query<ChatStatusQuery>,
) -> Response {
   match state.service.status(&query.channel_login).await {
      Ok(status_value) => {
         Json(ChatStatusResponse {
            status: status_value,
         })
         .into_response()
      },
      Err(e) => error_response(StatusCode::BAD_REQUEST, &e.to_string(), None),
   }
}

pub async fn metrics(State(state): State<ChatState>) -> Response {
   (
      [(
         axum::http::header::CONTENT_TYPE,
         "text/plain; version=0.0.4; charset=utf-8",
      )],
      state.service.metrics.prometheus(),
   )
      .into_response()
}

pub async fn emotes(State(state): State<ChatState>, Query(query): Query<EmotesQuery>) -> Response {
   match state.service.emotes_for_channel(&query.channel_login).await {
      Ok(items) => Json(EmotePickerResponse { emotes: items }).into_response(),
      Err(e) => {
         tracing::warn!(error = %e, channel = %query.channel_login, "failed loading chat emotes");
         error_response(StatusCode::BAD_REQUEST, &e.to_string(), None)
      },
   }
}

pub async fn events(State(state): State<ChatState>, Path(channel): Path<String>) -> Response {
   tracing::info!(channel = %channel, "chat events SSE request");
   let receiver = match state.service.receiver_for_channel(&channel).await {
      Ok(receiver) => receiver,
      Err(e) => return error_response(StatusCode::BAD_REQUEST, &e.to_string(), None),
   };
   let initial_status = match state.service.status(&channel).await {
      Ok(status) => status,
      Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string(), None),
   };

   let log_guard = ChatSseConnectionLog::new(channel);
   let metrics = state.service.metrics.clone();
   let updates = BroadcastStream::new(receiver).filter_map(move |result| {
      let channel = log_guard.channel();
      let metrics = metrics.clone();
      async move {
         match result {
            Ok(ChatStreamEvent::Chat(event)) => {
               tracing::trace!(channel = %channel, "chat EventSource chat event sent");
               let sse_event = Event::default().event("chat").json_data(event).ok()?;
               Some(Ok::<Event, Infallible>(sse_event))
            },
            Ok(ChatStreamEvent::Status(status)) => {
               tracing::debug!(channel = %channel, connected = status.connected, joined = status.joined, "chat EventSource status event sent");
               let sse_event = Event::default().event("status").json_data(status).ok()?;
               Some(Ok::<Event, Infallible>(sse_event))
            },
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(skipped)) => {
               metrics
                  .sse_broadcast_lag
                  .fetch_add(skipped, std::sync::atomic::Ordering::Relaxed);
               tracing::warn!(channel = %channel, skipped, "chat EventSource receiver lagged");
               None
            },
         }
      }
   });
   let initial = stream::once(async move {
      Event::default()
         .event("status")
         .json_data(initial_status)
         .map(Ok::<Event, Infallible>)
         .ok()
   })
   .filter_map(|event| async move { event });
   let stream = initial.chain(updates);

   Sse::new(stream)
      .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(12)))
      .into_response()
}

struct ChatSseConnectionLog {
   channel: String,
}

impl ChatSseConnectionLog {
   fn new(channel: String) -> Self {
      tracing::info!(channel = %channel, "chat EventSource opened");
      Self { channel }
   }

   fn channel(&self) -> String {
      self.channel.clone()
   }
}

impl Drop for ChatSseConnectionLog {
   fn drop(&mut self) {
      tracing::info!(channel = %self.channel, "chat EventSource closed");
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn prometheus_output_has_stable_names_and_types() {
      let metrics = ChatMetrics::new();
      metrics
         .connection_attempts
         .store(3, std::sync::atomic::Ordering::Relaxed);
      let output = metrics.prometheus();

      assert!(output.contains("# TYPE twitch_relay_chat_connected gauge\n"));
      assert!(output.contains("twitch_relay_chat_connected 0\n"));
      assert!(output.contains("# TYPE twitch_relay_chat_connection_attempts_total counter\n"));
      assert!(output.contains("twitch_relay_chat_connection_attempts_total 3\n"));
      assert!(output.contains("# TYPE twitch_relay_chat_sse_broadcast_lag_total counter\n"));
   }
}
