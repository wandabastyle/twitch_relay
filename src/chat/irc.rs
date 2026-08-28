use std::{
   collections::{
      HashMap,
      HashSet,
      VecDeque,
   },
   sync::{
      Arc,
      atomic::Ordering,
   },
   time::Duration,
};

use futures_util::{
   SinkExt,
   StreamExt,
   future::pending,
};
use rand::Rng;
use tokio::{
   sync::{
      RwLock,
      Semaphore,
      broadcast,
      mpsc,
      watch,
   },
   task::JoinHandle,
   time::{
      Instant,
      sleep_until,
      timeout,
   },
};
use tokio_tungstenite::{
   connect_async,
   tungstenite::Message,
};

use crate::{
   chat::{
      emotes::{
         CachedEmoteEntry,
         CachedThirdPartyEmotes,
         cached_third_party_emotes_for_channel,
         local_echo_parts_for_channel,
         third_party_emotes_for_channel,
      },
      events::{
         ChatChannelStatus,
         ChatEvent,
         ChatEventKind,
         ChatPart,
         ChatStreamEvent,
         fallback_sender_color,
         parse_chat_event,
      },
      service::{
         ChatCommand,
         ChatError,
         ChatMetrics,
      },
   },
   twitch_auth::TwitchAuthService,
   util::time::now_unix_secs,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
const AUTH_TIMEOUT: Duration = Duration::from_secs(10);
const IDLE_TIMEOUT: Duration = Duration::from_mins(3);
const PART_GRACE_PERIOD: Duration = Duration::from_secs(3);
const RECONNECT_CAP: Duration = Duration::from_mins(1);
const OUTBOUND_CAPACITY: usize = 256;
const INBOUND_CAPACITY: usize = 512;
const EMOTE_REFRESH_CAPACITY: usize = 4;
const LOCAL_ECHO_TTL: Duration = Duration::from_secs(8);
const MAX_LOCAL_ECHOES: usize = 256;
const JOIN_RETRY_DELAY: Duration = Duration::from_millis(250);
const MAX_PENDING_JOIN_RETRIES: usize = 256;

#[derive(Debug)]
enum ReaderEvent {
   Line(String),
}

#[derive(Debug, Clone)]
struct ChatIdentity {
   login:        String,
   display_name: String,
}

struct ConnectedSocket {
   writer_tx:     mpsc::Sender<String>,
   reader_rx:     mpsc::Receiver<ReaderEvent>,
   disconnect_rx: watch::Receiver<Option<String>>,
   identity:      ChatIdentity,
}

struct LocalEcho {
   key:        String,
   expires_at: Instant,
}

struct ChatManagerState {
   subscribed_counts:    HashMap<String, usize>,
   joined_channels:      HashSet<String>,
   joining_channels:     HashSet<String>,
   connected:            bool,
   last_error:           Option<String>,
   writer_tx:            Option<mpsc::Sender<String>>,
   reader_rx:            Option<mpsc::Receiver<ReaderEvent>>,
   disconnect_rx:        Option<watch::Receiver<Option<String>>>,
   chat_identity:        Option<ChatIdentity>,
   pending_parts:        HashMap<String, Instant>,
   pending_joins:        HashMap<String, Instant>,
   pending_local_echoes: VecDeque<LocalEcho>,
   reconnect_after:      Option<Instant>,
   failures:             u32,
}

impl ChatManagerState {
   fn new() -> Self {
      Self {
         subscribed_counts:    HashMap::new(),
         joined_channels:      HashSet::new(),
         joining_channels:     HashSet::new(),
         connected:            false,
         last_error:           None,
         writer_tx:            None,
         reader_rx:            None,
         disconnect_rx:        None,
         chat_identity:        None,
         pending_parts:        HashMap::new(),
         pending_joins:        HashMap::new(),
         pending_local_echoes: VecDeque::new(),
         reconnect_after:      None,
         failures:             0,
      }
   }

   fn disconnect(&mut self, reason: String, metrics: &ChatMetrics) {
      self.connected = false;
      self.writer_tx = None;
      self.reader_rx = None;
      self.disconnect_rx = None;
      self.chat_identity = None;
      self.joined_channels.clear();
      self.joining_channels.clear();
      self.pending_joins.clear();
      self.pending_local_echoes.clear();
      tracing::warn!(error = %reason, failures = self.failures.saturating_add(1), "chat connection disconnected; scheduling reconnect");
      self.last_error = Some(reason);
      self.failures = self.failures.saturating_add(1);
      self.reconnect_after = Some(Instant::now() + reconnect_delay(self.failures));
      metrics.connected.store(false, Ordering::Relaxed);
      metrics.disconnects.fetch_add(1, Ordering::Relaxed);
   }
}

pub async fn run_chat_manager(
   auth: TwitchAuthService,
   mut command_rx: mpsc::Receiver<ChatCommand>,
   channels: Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
   emote_cache: Arc<RwLock<HashMap<String, CachedEmoteEntry>>>,
   third_party_cache: Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
   metrics: ChatMetrics,
) {
   let mut state = ChatManagerState::new();
   let mut attempt: Option<JoinHandle<Result<ConnectedSocket, ChatError>>> = None;
   let mut attempt_started = None;
   let (emote_refresh_tx, mut emote_refresh_rx) = mpsc::channel(EMOTE_REFRESH_CAPACITY);
   let emote_refresh_slots = Arc::new(Semaphore::new(EMOTE_REFRESH_CAPACITY));

   loop {
      if state.subscribed_counts.is_empty()
         && !state.connected
         && let Some(handle) = attempt.take()
      {
         handle.abort();
      }
      if should_start_attempt(&state, attempt.is_some()) {
         metrics.connection_attempts.fetch_add(1, Ordering::Relaxed);
         if state.failures > 0 {
            metrics.reconnects.fetch_add(1, Ordering::Relaxed);
         }
         let auth = auth.clone();
         let attempt_metrics = metrics.clone();
         attempt = Some(tokio::spawn(async move {
            connect_chat(&auth, &attempt_metrics).await
         }));
         attempt_started = Some(Instant::now());
      }

      let deadline = next_timer_deadline(&state);
      let reader = async {
         match state.reader_rx.as_mut() {
            Some(rx) => rx.recv().await,
            None => pending().await,
         }
      };
      let timer = async {
         match deadline {
            Some(value) => sleep_until(value).await,
            None => pending().await,
         }
      };
      let attempt_result = async {
         match attempt.as_mut() {
            Some(handle) => Some(handle.await),
            None => pending().await,
         }
      };
      let disconnect = wait_for_disconnect(state.disconnect_rx.as_mut());

      tokio::select! {
         command = command_rx.recv() => match command {
            Some(command) => handle_command(command, &mut state, &channels, &emote_cache, &third_party_cache, &emote_refresh_tx).await,
            None => break,
         },
         event = reader => match event {
            Some(event) => handle_reader_event(event, &mut state, &channels, &third_party_cache, &emote_refresh_tx, &metrics).await,
            None if state.connected => {
               state.disconnect("chat reader task ended".to_string(), &metrics);
               publish_all_statuses(&channels, &state).await;
            },
            None => {},
         },
         Some(()) = disconnect => {
            let reason = state
               .disconnect_rx
               .as_mut()
               .and_then(|rx| rx.borrow_and_update().clone())
               .unwrap_or_else(|| "chat connection task ended".to_string());
            state.disconnect(reason, &metrics);
            publish_all_statuses(&channels, &state).await;
         },
         result = attempt_result => {
            attempt = None;
            if let Some(started) = attempt_started.take() {
               metrics.last_attempt_ms.store(started.elapsed().as_millis().try_into().unwrap_or(u64::MAX), Ordering::Relaxed);
            }
            match result {
               Some(Ok(Ok(socket))) => install_connection(socket, &mut state, &metrics, &channels).await,
               Some(Ok(Err(error))) => { metrics.connection_failures.fetch_add(1, Ordering::Relaxed); state.disconnect(error.to_string(), &metrics); publish_all_statuses(&channels, &state).await; },
               Some(Err(error)) => { metrics.connection_failures.fetch_add(1, Ordering::Relaxed); state.disconnect(format!("chat connection task failed: {error}"), &metrics); publish_all_statuses(&channels, &state).await; },
               None => {},
            }
         },
         Some(channel) = emote_refresh_rx.recv() => {
            let auth = auth.clone();
            let cache = third_party_cache.clone();
            if let Ok(permit) = emote_refresh_slots.clone().try_acquire_owned() {
               tokio::spawn(async move {
                  let _permit = permit;
                  if let Err(error) = third_party_emotes_for_channel(&auth, &cache, &channel).await {
                     tracing::debug!(%channel, %error, "background third-party emote refresh failed");
                  }
               });
            }
         },
          () = timer => process_due_control(&mut state, &channels).await,
      }
   }
   if let Some(handle) = attempt {
      handle.abort();
   }
   metrics.connected.store(false, Ordering::Relaxed);
}

fn should_start_attempt(state: &ChatManagerState, attempting: bool) -> bool {
   !attempting
      && !state.connected
      && !state.subscribed_counts.is_empty()
      && state.reconnect_after.is_none_or(|at| at <= Instant::now())
}

async fn wait_for_disconnect(rx: Option<&mut watch::Receiver<Option<String>>>) -> Option<()> {
   match rx {
      Some(rx) => rx.changed().await.ok(),
      None => pending().await,
   }
}

async fn install_connection(
   socket: ConnectedSocket,
   state: &mut ChatManagerState,
   metrics: &ChatMetrics,
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
) {
   state.writer_tx = Some(socket.writer_tx);
   state.reader_rx = Some(socket.reader_rx);
   state.disconnect_rx = Some(socket.disconnect_rx);
   state.chat_identity = Some(socket.identity.clone());
   state.connected = true;
   state.last_error = None;
   state.reconnect_after = None;
   state.failures = 0;
   metrics.connected.store(true, Ordering::Relaxed);
   tracing::info!(login = %socket.identity.login, "chat IRC welcome confirmed");
   send_raw(
      state,
      "CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership".to_string(),
   );
   let targets: Vec<String> = state.subscribed_counts.keys().cloned().collect();
   for channel in targets {
      request_join(state, &channel);
   }
   publish_all_statuses(channels, state).await;
}

async fn handle_command(
   command: ChatCommand,
   state: &mut ChatManagerState,
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
   emote_cache: &Arc<RwLock<HashMap<String, CachedEmoteEntry>>>,
   third_party_cache: &Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
   emote_refresh_tx: &mpsc::Sender<String>,
) {
   match command {
      ChatCommand::Subscribe { channel, response } => {
         state.pending_parts.remove(&channel);
         *state.subscribed_counts.entry(channel.clone()).or_insert(0) += 1;
         ensure_channel_sender(channels, &channel).await;
         request_join(state, &channel);
         publish_status(channels, state, &channel).await;
         let _ = response.send(Ok(()));
      },
      ChatCommand::Unsubscribe { channel, response } => {
         if let Some(count) = state.subscribed_counts.get_mut(&channel) {
            if *count > 1 {
               *count -= 1;
            } else {
               state.subscribed_counts.remove(&channel);
               state.pending_joins.remove(&channel);
               state
                  .pending_parts
                  .insert(channel.clone(), Instant::now() + PART_GRACE_PERIOD);
            }
         }
         publish_status(channels, state, &channel).await;
         let _ = response.send(Ok(()));
      },
      ChatCommand::SendMessage {
         channel,
         message,
         response,
      } => {
         if !state.connected || !state.joined_channels.contains(&channel) {
            request_join(state, &channel);
            let error = state
               .last_error
               .clone()
               .unwrap_or_else(|| "channel is not joined yet".to_string());
            let _ = response.send(Err(ChatError::ConnectionUnavailable(error)));
            return;
         }
         if send_raw(state, format!("PRIVMSG #{channel} :{message}")) {
            if let (Some(identity), Some(sender)) = (
               state.chat_identity.as_ref(),
               get_channel_sender(channels, &channel).await,
            ) {
               let event = ChatEvent {
                  kind:                ChatEventKind::Message,
                  channel_login:       channel.clone(),
                  sender_login:        Some(identity.login.clone()),
                  sender_display_name: Some(identity.display_name.clone()),
                  sender_color:        Some(fallback_sender_color(&identity.login)),
                  text:                message.clone(),
                  parts:               local_echo_parts_for_channel(
                     emote_cache,
                     third_party_cache,
                     &channel,
                     &message,
                  )
                  .await,
                  sent_at_unix:        now_unix_secs(),
               };
               remember_local_echo(state, &event);
               let _ = sender.send(ChatStreamEvent::Chat(event));
               let _ = emote_refresh_tx.try_send(channel);
            }
            let _ = response.send(Ok(()));
         } else {
            let _ = response.send(Err(ChatError::WriterUnavailable));
         }
      },
      ChatCommand::Status { channel, response } => {
         let _ = response.send(crate::chat::events::ChatChannelStatus {
            subscribed: state.subscribed_counts.contains_key(&channel),
            connected:  state.connected,
            joined:     state.joined_channels.contains(&channel),
            error:      state.last_error.clone(),
         });
      },
   }
}

async fn handle_reader_event(
   event: ReaderEvent,
   state: &mut ChatManagerState,
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
   third_party_cache: &Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
   emote_refresh_tx: &mpsc::Sender<String>,
   metrics: &ChatMetrics,
) {
   match event {
      ReaderEvent::Line(line) => {
         if let Some(pong) = pong_for_ping(&line) {
            let _ = send_raw(state, pong);
            return;
         }
         match parse_server_signal(
            &line,
            state
               .chat_identity
               .as_ref()
               .map(|identity| identity.login.as_str()),
         ) {
            ServerSignal::Join(channel) => {
               state.joining_channels.remove(&channel);
               if state.subscribed_counts.contains_key(&channel) {
                  state.joined_channels.insert(channel.clone());
                  metrics.joins.fetch_add(1, Ordering::Relaxed);
                  tracing::info!(%channel, "chat JOIN confirmed");
                  publish_status(channels, state, &channel).await;
               }
            },
            ServerSignal::Part(channel) => {
               state.joined_channels.remove(&channel);
               state.joining_channels.remove(&channel);
               publish_status(channels, state, &channel).await;
            },
            ServerSignal::Reconnect => {
               state.disconnect("Twitch requested reconnect".to_string(), metrics);
               publish_all_statuses(channels, state).await;
            },
            ServerSignal::AuthenticationFailed(reason) => {
               state.disconnect(reason, metrics);
               publish_all_statuses(channels, state).await;
            },
            ServerSignal::Welcome | ServerSignal::None => {},
         }
         if let Some(mut chat_event) = parse_chat_event(&line) {
            if is_duplicate_local_echo(state, &chat_event) {
               return;
            }
            let cached =
               cached_third_party_emotes_for_channel(third_party_cache, &chat_event.channel_login)
                  .await;
            if !cached.is_empty() {
               chat_event.parts = apply_third_party_emotes(&chat_event.parts, &cached);
            }
            let _ = emote_refresh_tx.try_send(chat_event.channel_login.clone());
            if let Some(sender) = get_channel_sender(channels, &chat_event.channel_login).await {
               let _ = sender.send(ChatStreamEvent::Chat(chat_event));
            }
         }
      },
   }
}

fn request_join(state: &mut ChatManagerState, channel: &str) {
   if state.connected
      && !state.joined_channels.contains(channel)
      && !state.joining_channels.contains(channel)
   {
      if send_raw(state, format!("JOIN #{channel}")) {
         state.joining_channels.insert(channel.to_string());
         state.pending_joins.remove(channel);
      } else {
         schedule_join_retry(state, channel);
      }
   }
}

fn schedule_join_retry(state: &mut ChatManagerState, channel: &str) {
   if state.pending_joins.contains_key(channel) {
      return;
   }
   if state.pending_joins.len() >= MAX_PENDING_JOIN_RETRIES {
      tracing::warn!(channel, "chat JOIN retry queue is full");
      return;
   }
   state
      .pending_joins
      .insert(channel.to_string(), Instant::now() + JOIN_RETRY_DELAY);
}

fn send_raw(state: &ChatManagerState, line: String) -> bool {
   state
      .writer_tx
      .as_ref()
      .is_some_and(|tx| tx.try_send(line).is_ok())
}

async fn process_due_control(
   state: &mut ChatManagerState,
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
) {
   let now = Instant::now();
   let due: Vec<String> = state
      .pending_parts
      .iter()
      .filter(|(_, at)| **at <= now)
      .map(|(channel, _)| channel.clone())
      .collect();
   for channel in due {
      state.pending_parts.remove(&channel);
      if !state.subscribed_counts.contains_key(&channel) && state.joined_channels.remove(&channel) {
         let _ = send_raw(state, format!("PART #{channel}"));
         publish_status(channels, state, &channel).await;
      }
   }

   let now = Instant::now();
   let due_joins: Vec<String> = state
      .pending_joins
      .iter()
      .filter(|(_, at)| **at <= now)
      .map(|(channel, _)| channel.clone())
      .collect();
   for channel in due_joins {
      state.pending_joins.remove(&channel);
      if state.subscribed_counts.contains_key(&channel) {
         request_join(state, &channel);
      }
   }
}

fn next_timer_deadline(state: &ChatManagerState) -> Option<Instant> {
   state
      .pending_parts
      .values()
      .copied()
      .chain(state.reconnect_after)
      .chain(state.pending_joins.values().copied())
      .min()
}

fn reconnect_delay(failures: u32) -> Duration {
   let exponent = failures.saturating_sub(1).min(6);
   let base = 1_u64 << exponent;
   let jitter = rand::rng().random_range(0..=base / 2);
   Duration::from_secs((base + jitter).min(RECONNECT_CAP.as_secs()))
}

async fn connect_chat(
   auth: &TwitchAuthService,
   metrics: &ChatMetrics,
) -> Result<ConnectedSocket, ChatError> {
   let account = timeout(AUTH_TIMEOUT, auth.ensure_chat_account())
      .await
      .map_err(|_| ChatError::Other("chat OAuth preparation timed out".to_string()))?
      .map_err(ChatError::Other)?;
   let (stream, _) = timeout(
      CONNECT_TIMEOUT,
      connect_async("wss://irc-ws.chat.twitch.tv:443"),
   )
   .await
   .map_err(|_| ChatError::WebSocketConnectFailed("connection timed out".to_string()))?
   .map_err(|error| ChatError::WebSocketConnectFailed(error.to_string()))?;
   let (mut writer, mut reader) = stream.split();
   writer
      .send(Message::Text(
         format!("PASS oauth:{}", account.access_token).into(),
      ))
      .await
      .map_err(|error| ChatError::PassFailed(error.to_string()))?;
   writer
      .send(Message::Text(format!("NICK {}", account.login).into()))
      .await
      .map_err(|error| ChatError::NickFailed(error.to_string()))?;
   wait_for_welcome(&mut reader).await?;
   let (writer_tx, mut writer_rx) = mpsc::channel::<String>(OUTBOUND_CAPACITY);
   let (reader_tx, reader_rx) = mpsc::channel(INBOUND_CAPACITY);
   let (disconnect_tx, disconnect_rx) = watch::channel(None);
   let writer_disconnect = disconnect_tx.clone();
   let reader_metrics = metrics.clone();
   tokio::spawn(async move {
      while let Some(line) = writer_rx.recv().await {
         if writer.send(Message::Text(line.into())).await.is_err() {
            let _ = writer_disconnect.send(Some("chat writer task failed".to_string()));
            return;
         }
      }
   });
   tokio::spawn(async move {
      loop {
         let message = match timeout(IDLE_TIMEOUT, reader.next()).await {
            Ok(Some(Ok(message))) => message,
            Ok(Some(Err(error))) => {
               let _ = disconnect_tx.send(Some(format!("chat reader error: {error}")));
               return;
            },
            Ok(None) => {
               let _ = disconnect_tx.send(Some("chat socket closed".to_string()));
               return;
            },
            Err(_) => {
               let _ = disconnect_tx.send(Some("chat socket idle timeout".to_string()));
               return;
            },
         };
         if let Message::Text(text) = &message {
            for line in text.lines() {
               if reader_tx
                  .try_send(ReaderEvent::Line(line.to_string()))
                  .is_err()
               {
                  reader_metrics
                     .event_queue_full
                     .fetch_add(1, Ordering::Relaxed);
                  let _ = disconnect_tx.send(Some("chat inbound queue saturated".to_string()));
                  return;
               }
            }
         }
         if matches!(message, Message::Close(_)) {
            let _ = disconnect_tx.send(Some("chat socket closed".to_string()));
            return;
         }
      }
   });
   Ok(ConnectedSocket {
      writer_tx,
      reader_rx,
      disconnect_rx,
      identity: ChatIdentity {
         login:        account.login,
         display_name: account.display_name,
      },
   })
}

async fn wait_for_welcome(
   reader: &mut (
           impl futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
           + Unpin
        ),
) -> Result<(), ChatError> {
   timeout(AUTH_TIMEOUT, async {
      while let Some(message) = reader.next().await {
         let message = message.map_err(|error| {
            ChatError::Other(format!("chat authentication read failed: {error}"))
         })?;
         if let Message::Text(text) = message {
            for line in text.lines() {
               match parse_server_signal(line, None) {
                  ServerSignal::Welcome => return Ok(()),
                  ServerSignal::AuthenticationFailed(reason) => {
                     return Err(ChatError::Other(reason));
                  },
                  _ => {},
               }
            }
         }
      }
      Err(ChatError::Other(
         "chat socket closed during authentication".to_string(),
      ))
   })
   .await
   .map_err(|_| ChatError::Other("chat welcome timed out".to_string()))?
}

#[derive(Debug, PartialEq, Eq)]
enum ServerSignal {
   Welcome,
   AuthenticationFailed(String),
   Join(String),
   Part(String),
   Reconnect,
   None,
}

fn parse_server_signal(line: &str, self_login: Option<&str>) -> ServerSignal {
   let upper = line.to_ascii_uppercase();
   if upper.contains(" 001 ") {
      return ServerSignal::Welcome;
   }
   if upper.contains(" RECONNECT") {
      return ServerSignal::Reconnect;
   }
   if upper.contains("LOGIN AUTHENTICATION FAILED") || upper.contains("IMPROPERLY FORMATTED AUTH") {
      return ServerSignal::AuthenticationFailed(line.to_string());
   }
   let Some(login) = self_login else {
      return ServerSignal::None;
   };
   let prefix = format!(":{}!", login.to_ascii_lowercase());
   if !line.to_ascii_lowercase().starts_with(&prefix) {
      return ServerSignal::None;
   }
   let words: Vec<&str> = line.split_whitespace().collect();
   match words.as_slice() {
      [_, "JOIN", channel] => {
         ServerSignal::Join(channel.trim_start_matches('#').to_ascii_lowercase())
      },
      [_, "PART", channel, ..] => {
         ServerSignal::Part(channel.trim_start_matches('#').to_ascii_lowercase())
      },
      _ => ServerSignal::None,
   }
}

fn pong_for_ping(line: &str) -> Option<String> {
   line
      .strip_prefix("PING ")
      .map(|payload| format!("PONG {payload}"))
}

async fn ensure_channel_sender(
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
   channel: &str,
) {
   let mut guard = channels.write().await;
   guard
      .entry(channel.to_string())
      .or_insert_with(|| broadcast::channel(256).0);
}
async fn get_channel_sender(
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
   channel: &str,
) -> Option<broadcast::Sender<ChatStreamEvent>> {
   channels.read().await.get(channel).cloned()
}

fn channel_status(state: &ChatManagerState, channel: &str) -> ChatChannelStatus {
   ChatChannelStatus {
      subscribed: state.subscribed_counts.contains_key(channel),
      connected:  state.connected,
      joined:     state.joined_channels.contains(channel),
      error:      state.last_error.clone(),
   }
}

async fn publish_status(
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
   state: &ChatManagerState,
   channel: &str,
) {
   if let Some(sender) = get_channel_sender(channels, channel).await {
      let _ = sender.send(ChatStreamEvent::Status(channel_status(state, channel)));
   }
}

async fn publish_all_statuses(
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatStreamEvent>>>>,
   state: &ChatManagerState,
) {
   let targets: Vec<String> = {
      let guard = channels.read().await;
      guard.keys().cloned().collect()
   };
   for channel in targets {
      publish_status(channels, state, &channel).await;
   }
}

fn remember_local_echo(state: &mut ChatManagerState, event: &ChatEvent) {
   prune_local_echoes(&mut state.pending_local_echoes);
   let Some(key) = local_echo_key(event) else {
      return;
   };
   if state.pending_local_echoes.len() == MAX_LOCAL_ECHOES {
      state.pending_local_echoes.pop_front();
   }
   state.pending_local_echoes.push_back(LocalEcho {
      key,
      expires_at: Instant::now() + LOCAL_ECHO_TTL,
   });
}

fn is_duplicate_local_echo(state: &mut ChatManagerState, event: &ChatEvent) -> bool {
   prune_local_echoes(&mut state.pending_local_echoes);
   let Some(identity) = state.chat_identity.as_ref() else {
      return false;
   };
   let Some(sender) = event.sender_login.as_deref() else {
      return false;
   };
   if !sender.eq_ignore_ascii_case(&identity.login) {
      return false;
   }
   let Some(key) = local_echo_key(event) else {
      return false;
   };
   let Some(index) = state
      .pending_local_echoes
      .iter()
      .position(|candidate| candidate.key == key)
   else {
      return false;
   };
   state.pending_local_echoes.remove(index);
   true
}

fn prune_local_echoes(echoes: &mut VecDeque<LocalEcho>) {
   let now = Instant::now();
   while echoes.front().is_some_and(|echo| echo.expires_at <= now) {
      echoes.pop_front();
   }
}

fn local_echo_key(event: &ChatEvent) -> Option<String> {
   if !matches!(event.kind, ChatEventKind::Message) {
      return None;
   }
   let channel = event.channel_login.trim();
   let sender = event.sender_login.as_deref()?.trim();
   let text = event.text.trim();
   if channel.is_empty() || sender.is_empty() || text.is_empty() {
      return None;
   }
   Some(format!(
      "{}|{}|{text}",
      channel.to_ascii_lowercase(),
      sender.to_ascii_lowercase()
   ))
}

fn apply_third_party_emotes(parts: &[ChatPart], emotes: &HashMap<String, String>) -> Vec<ChatPart> {
   parts
      .iter()
      .flat_map(|part| {
         match part {
            ChatPart::Text { text } => {
               split_preserving_whitespace(text)
                  .into_iter()
                  .map(|segment| {
                     match emotes.get(&segment) {
                        Some(url) => {
                           ChatPart::Emote {
                              id:        segment.clone(),
                              code:      segment,
                              image_url: Some(url.clone()),
                           }
                        },
                        None => ChatPart::Text { text: segment },
                     }
                  })
                  .collect::<Vec<_>>()
            },
            ChatPart::Emote { .. } => vec![part.clone()],
         }
      })
      .collect()
}
fn split_preserving_whitespace(input: &str) -> Vec<String> {
   let mut out = Vec::new();
   let mut current = String::new();
   let mut whitespace = None;
   for character in input.chars() {
      let value = character.is_whitespace();
      if whitespace.is_some_and(|current_value| current_value != value) {
         out.push(std::mem::take(&mut current));
      }
      whitespace = Some(value);
      current.push(character);
   }
   if !current.is_empty() {
      out.push(current);
   }
   out
}

#[cfg(test)]
mod tests {
   use super::*;

   fn local_message(channel: &str, sender: &str, text: &str) -> ChatEvent {
      ChatEvent {
         kind:                ChatEventKind::Message,
         channel_login:       channel.to_string(),
         sender_login:        Some(sender.to_string()),
         sender_display_name: Some(sender.to_string()),
         sender_color:        None,
         text:                text.to_string(),
         parts:               vec![ChatPart::Text {
            text: text.to_string(),
         }],
         sent_at_unix:        0,
      }
   }
   #[test]
   fn parser_recognizes_welcome_auth_join_and_reconnect() {
      assert_eq!(
         parse_server_signal(":tmi.twitch.tv 001 alice :Welcome", None),
         ServerSignal::Welcome
      );
      assert!(matches!(
         parse_server_signal(":tmi.twitch.tv NOTICE * :Login authentication failed", None),
         ServerSignal::AuthenticationFailed(_)
      ));
      assert_eq!(
         parse_server_signal(":alice!alice@alice.tmi.twitch.tv JOIN #rust", Some("alice")),
         ServerSignal::Join("rust".to_string())
      );
      assert_eq!(
         parse_server_signal(":tmi.twitch.tv RECONNECT", Some("alice")),
         ServerSignal::Reconnect
      );
   }
   #[test]
   fn ping_generates_pong() {
      assert_eq!(
         pong_for_ping("PING :tmi.twitch.tv"),
         Some("PONG :tmi.twitch.tv".to_string())
      );
   }
   #[test]
   fn reconnect_backoff_is_capped() {
      assert!(reconnect_delay(100) <= RECONNECT_CAP);
   }
   #[test]
   fn timer_includes_reconnect_deadline() {
      let mut state = ChatManagerState::new();
      let deadline = Instant::now() + Duration::from_secs(1);
      state.reconnect_after = Some(deadline);
      assert_eq!(next_timer_deadline(&state), Some(deadline));
   }

   #[test]
   fn channel_status_tracks_confirmed_connection_and_join() {
      let mut state = ChatManagerState::new();
      state.subscribed_counts.insert("rust".to_string(), 1);
      state.connected = true;
      state.joined_channels.insert("rust".to_string());

      let status = channel_status(&state, "rust");

      assert!(status.subscribed && status.connected && status.joined);
   }

   #[test]
   fn local_echo_marker_suppresses_only_the_matching_twitch_echo() {
      let mut state = ChatManagerState::new();
      state.chat_identity = Some(ChatIdentity {
         login:        "alice".to_string(),
         display_name: "Alice".to_string(),
      });
      let local = local_message("rust", "alice", "hello");
      remember_local_echo(&mut state, &local);

      assert!(is_duplicate_local_echo(&mut state, &local));
      assert!(!is_duplicate_local_echo(&mut state, &local));
      assert!(!is_duplicate_local_echo(
         &mut state,
         &local_message("rust", "bob", "hello")
      ));
   }

   #[tokio::test]
   async fn saturated_writer_retries_join_after_timer() {
      let (writer_tx, mut writer_rx) = mpsc::channel(1);
      let mut state = ChatManagerState::new();
      state.connected = true;
      state.writer_tx = Some(writer_tx);
      state.subscribed_counts.insert("rust".to_string(), 1);
      let filled = send_raw(&state, "CAP REQ :test".to_string());
      assert!(filled);

      request_join(&mut state, "rust");
      assert!(state.pending_joins.contains_key("rust"));
      assert!(writer_rx.try_recv().is_ok());
      state
         .pending_joins
         .insert("rust".to_string(), Instant::now());

      let channels = Arc::new(RwLock::new(HashMap::new()));
      process_due_control(&mut state, &channels).await;

      assert_eq!(writer_rx.try_recv().ok().as_deref(), Some("JOIN #rust"));
   }

   #[tokio::test]
   async fn saturation_notification_reaches_manager_when_event_queue_is_full() {
      let (event_tx, mut event_rx) = mpsc::channel(1);
      let (disconnect_tx, mut disconnect_rx) = watch::channel(None);
      assert!(
         event_tx
            .try_send(ReaderEvent::Line("full".to_string()))
            .is_ok()
      );
      assert!(
         event_tx
            .try_send(ReaderEvent::Line("overflow".to_string()))
            .is_err()
      );

      let _ = disconnect_tx.send(Some("chat inbound queue saturated".to_string()));
      assert!(disconnect_rx.changed().await.is_ok());

      assert_eq!(
         disconnect_rx.borrow_and_update().clone(),
         Some("chat inbound queue saturated".to_string())
      );
      assert!(event_rx.try_recv().is_ok());
   }
}
