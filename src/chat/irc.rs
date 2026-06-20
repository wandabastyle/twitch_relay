use std::{
   collections::{
      HashMap,
      HashSet,
   },
   sync::Arc,
   time::Duration,
};

use futures_util::{
   SinkExt,
   StreamExt,
   future::pending,
};
use tokio::{
   sync::{
      RwLock,
      broadcast,
      mpsc,
   },
   time::{
      Instant,
      sleep_until,
   },
};
use tokio_tungstenite::{
   connect_async,
   tungstenite::Message,
};

use crate::{
   chat::{
      emotes::{
         CachedThirdPartyEmotes,
         local_echo_parts_for_channel,
         third_party_emotes_for_channel,
      },
      events::{
         ChatEvent,
         ChatEventKind,
         ChatPart,
         fallback_sender_color,
         parse_chat_event,
      },
      service::{
         ChatCommand,
         ChatError,
      },
   },
   twitch_auth::TwitchAuthService,
   util::time::now_unix_secs,
};

const LOCAL_ECHO_TTL_SECS: u64 = 8;
const PART_GRACE_PERIOD: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub enum ReaderEvent {
   Line(String),
   Disconnected,
}

#[derive(Debug, Clone)]
struct ChatIdentity {
   login:        String,
   display_name: String,
}

/// State for the chat manager loop.
struct ChatManagerState {
   subscribed_counts:  HashMap<String, usize>,
   joined_channels:    HashSet<String>,
   connected:          bool,
   last_error:         Option<String>,
   writer_tx:          Option<mpsc::UnboundedSender<String>>,
   reader_rx:          Option<mpsc::UnboundedReceiver<ReaderEvent>>,
   chat_identity:      Option<ChatIdentity>,
   pending_local_echo: HashMap<String, u64>,
   pending_parts:      HashMap<String, Instant>,
}

impl ChatManagerState {
   fn new() -> Self {
      Self {
         subscribed_counts:  HashMap::new(),
         joined_channels:    HashSet::new(),
         connected:          false,
         last_error:         None,
         writer_tx:          None,
         reader_rx:          None,
         chat_identity:      None,
         pending_local_echo: HashMap::new(),
         pending_parts:      HashMap::new(),
      }
   }
}

async fn handle_subscribe_command(
   channel: String,
   response: tokio::sync::oneshot::Sender<Result<(), ChatError>>,
   state: &mut ChatManagerState,
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
) {
   let canceled_part = state.pending_parts.remove(&channel).is_some();
   let entry = state.subscribed_counts.entry(channel.clone()).or_insert(0);
   *entry = entry.saturating_add(1);
   tracing::info!(
      channel = %channel,
      count = *entry,
      canceled_part,
      connected = state.connected,
      joined = state.joined_channels.contains(&channel),
      "chat channel subscribed"
   );
   ensure_channel_sender(channels, &channel).await;

   if state.connected
      && !state.joined_channels.contains(&channel)
      && let Some(writer) = state.writer_tx.as_ref()
   {
      tracing::info!(channel = %channel, count = *entry, "sending chat JOIN");
      let _ = writer.send(format!("JOIN #{channel}"));
      state.joined_channels.insert(channel.clone());
   }

   let _ = response.send(Ok(()));
}

fn handle_unsubscribe_command(
   channel: &str,
   response: tokio::sync::oneshot::Sender<Result<(), ChatError>>,
   state: &mut ChatManagerState,
) {
   if let Some(entry) = state.subscribed_counts.get_mut(channel) {
      if *entry > 1 {
         *entry -= 1;
         tracing::debug!(channel = %channel, count = *entry, "chat channel unsubscribed");
      } else {
         state.subscribed_counts.remove(channel);
         let deadline = Instant::now() + PART_GRACE_PERIOD;
         state.pending_parts.insert(channel.to_string(), deadline);
         tracing::debug!(
            channel = %channel,
            grace_ms = PART_GRACE_PERIOD.as_millis(),
            "chat channel unsubscribe reached zero; scheduling PART"
         );
      }
   }
   let _ = response.send(Ok(()));
}

/// Handle a single chat command.
async fn handle_command(
   command: ChatCommand,
   state: &mut ChatManagerState,
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
   emote_cache: &Arc<RwLock<HashMap<String, crate::chat::emotes::CachedEmoteEntry>>>,
   third_party_emote_cache: &Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
   auth: &TwitchAuthService,
) -> bool {
   match command {
      ChatCommand::Subscribe { channel, response } => {
         handle_subscribe_command(channel, response, state, channels).await;
      },
      ChatCommand::Unsubscribe { channel, response } => {
         handle_unsubscribe_command(&channel, response, state);
      },
      ChatCommand::SendMessage {
         channel,
         message,
         response,
      } => {
         if !state.connected {
            let error = state.last_error.clone().map_or_else(
               || ChatError::ConnectionUnavailable("unknown".to_string()),
               ChatError::ConnectionUnavailable,
            );
            let _ = response.send(Err(error));
            return false;
         }

         if !state.joined_channels.contains(&channel)
            && let Some(writer) = state.writer_tx.as_ref()
         {
            tracing::info!(channel = %channel, "sending chat JOIN before message");
            let _ = writer.send(format!("JOIN #{channel}"));
            state.joined_channels.insert(channel.clone());
         }

         if let Some(writer) = state.writer_tx.as_ref() {
            let _ = writer.send(format!("PRIVMSG #{channel} :{message}"));

            if let Some(identity) = state.chat_identity.as_ref()
               && let Some(sender) = get_channel_sender(channels, &channel).await
            {
               let echo_event = ChatEvent {
                  kind:                ChatEventKind::Message,
                  channel_login:       channel.clone(),
                  sender_login:        Some(identity.login.clone()),
                  sender_display_name: Some(identity.display_name.clone()),
                  sender_color:        Some(fallback_sender_color(&identity.login)),
                  text:                message.clone(),
                  parts:               local_echo_parts_for_channel(
                     emote_cache,
                     third_party_emote_cache,
                     auth,
                     &channel,
                     &message,
                  )
                  .await,
                  sent_at_unix:        now_unix_secs(),
               };
               remember_local_echo(&mut state.pending_local_echo, &echo_event);
               let _ = sender.send(echo_event);
            }

            let _ = response.send(Ok(()));
         } else {
            let _ = response.send(Err(ChatError::WriterUnavailable));
         }
      },
      ChatCommand::Status { channel, response } => {
         let subscribed_count = state.subscribed_counts.get(&channel).copied().unwrap_or(0);
         let subscribed = subscribed_count > 0;
         tracing::debug!(
            channel = %channel,
            subscribed_count,
            connected = state.connected,
            joined = state.joined_channels.contains(&channel),
            pending_part = state.pending_parts.contains_key(&channel),
            "chat status reported"
         );
         let _ = response.send(crate::chat::events::ChatChannelStatus {
            subscribed,
            connected: state.connected,
            joined: state.joined_channels.contains(&channel),
            error: state.last_error.clone(),
         });
      },
   }
   true
}

/// Handle a read event from the IRC connection.
async fn handle_read_event(
   event: ReaderEvent,
   state: &mut ChatManagerState,
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
   third_party_emote_cache: &Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
   auth: &TwitchAuthService,
) {
   match event {
      ReaderEvent::Line(line) => {
         if let Some(writer) = state.writer_tx.as_ref()
            && line.starts_with("PING ")
         {
            let payload = line.trim_start_matches("PING ").trim();
            let _ = writer.send(format!("PONG {payload}"));
            return;
         }

         if let Some(mut event) = parse_chat_event(&line) {
            enrich_chat_event_with_third_party_emotes(&mut event, third_party_emote_cache, auth)
               .await;

            if !is_duplicate_local_echo(&mut state.pending_local_echo, &event)
               && let Some(sender) = get_channel_sender(channels, &event.channel_login).await
            {
               let _ = sender.send(event);
            }
         }
      },
      ReaderEvent::Disconnected => {
         state.connected = false;
         state.writer_tx = None;
         state.reader_rx = None;
         state.chat_identity = None;
         state.joined_channels.clear();
         state.last_error = Some("chat connection lost; retrying".to_string());
      },
   }
}

/// Attempt to connect to IRC if not connected.
async fn maybe_connect(state: &mut ChatManagerState, auth: &TwitchAuthService) {
   if !state.connected && !state.subscribed_counts.is_empty() {
      match connect_chat(auth).await {
         Ok((tx, rx, identity)) => {
            state.writer_tx = Some(tx);
            state.reader_rx = Some(rx);
            state.chat_identity = Some(identity.clone());
            state.connected = true;
            state.last_error = None;

            if let Some(writer) = state.writer_tx.as_ref() {
               let _ = writer.send(
                  "CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership".to_string(),
               );
               for channel in state.subscribed_counts.keys() {
                  tracing::debug!(channel = %channel, "sending chat JOIN after connect");
                  let _ = writer.send(format!("JOIN #{channel}"));
                  state.joined_channels.insert(channel.clone());
               }
            }

            tracing::info!(login = %identity.login, joined = state.joined_channels.len(), "chat IRC connected");
         },
         Err(e) => {
            state.connected = false;
            state.last_error = Some(e.to_string());
         },
      }
   }
}

fn part_deadline(state: &ChatManagerState) -> Option<Instant> {
   state.pending_parts.values().copied().min()
}

fn process_due_parts(state: &mut ChatManagerState) {
   let now = Instant::now();
   let due_channels: Vec<String> = state
      .pending_parts
      .iter()
      .filter(|(_, deadline)| **deadline <= now)
      .map(|(channel, _)| channel.clone())
      .collect();

   for channel in due_channels {
      state.pending_parts.remove(&channel);
      if state.subscribed_counts.contains_key(&channel) {
         tracing::debug!(channel = %channel, "skipping scheduled chat PART because channel was resubscribed");
         continue;
      }

      if state.connected
         && state.joined_channels.remove(&channel)
         && let Some(writer) = state.writer_tx.as_ref()
      {
         tracing::debug!(channel = %channel, "sending chat PART");
         let _ = writer.send(format!("PART #{channel}"));
      }
   }
}

pub async fn run_chat_manager(
   auth: TwitchAuthService,
   mut command_rx: mpsc::UnboundedReceiver<ChatCommand>,
   channels: Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
   emote_cache: Arc<RwLock<HashMap<String, crate::chat::emotes::CachedEmoteEntry>>>,
   third_party_emote_cache: Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
) {
   let mut state = ChatManagerState::new();

   loop {
      maybe_connect(&mut state, &auth).await;
      process_due_parts(&mut state);

      let next_part_deadline = part_deadline(&state);

      let read_event = async {
         if let Some(rx) = state.reader_rx.as_mut() {
            rx.recv().await
         } else {
            pending().await
         }
      };

      let part_timer = async {
         if let Some(deadline) = next_part_deadline {
            sleep_until(deadline).await;
         } else {
            pending::<()>().await;
         }
      };

      tokio::select! {
          maybe_cmd = command_rx.recv() => {
              let Some(command) = maybe_cmd else {
                  break;
              };

              // Skip to next loop iteration if command handler returns false
              let _continue = !handle_command(
                  command,
                  &mut state,
                  &channels,
                  &emote_cache,
                  &third_party_emote_cache,
                  &auth,
              ).await;
          }
          maybe_event = read_event => {
              if let Some(event) = maybe_event {
                  handle_read_event(
                      event,
                      &mut state,
                      &channels,
                      &third_party_emote_cache,
                      &auth,
                  ).await;
              }
          }
          () = part_timer => {
              process_due_parts(&mut state);
          }
      }
   }
}

async fn connect_chat(
   auth: &TwitchAuthService,
) -> Result<
   (
      mpsc::UnboundedSender<String>,
      mpsc::UnboundedReceiver<ReaderEvent>,
      ChatIdentity,
   ),
   ChatError,
> {
   let account = auth.ensure_chat_account().await.map_err(ChatError::Other)?;

   let (ws_stream, _response) = connect_async("wss://irc-ws.chat.twitch.tv:443")
      .await
      .map_err(|e| ChatError::WebSocketConnectFailed(e.to_string()))?;

   let (mut ws_writer, mut ws_reader) = ws_stream.split();

   ws_writer
      .send(Message::Text(
         format!("PASS oauth:{}", account.access_token).into(),
      ))
      .await
      .map_err(|e| ChatError::PassFailed(e.to_string()))?;
   ws_writer
      .send(Message::Text(format!("NICK {}", account.login).into()))
      .await
      .map_err(|e| ChatError::NickFailed(e.to_string()))?;

   let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<String>();
   let (reader_tx, reader_rx) = mpsc::unbounded_channel::<ReaderEvent>();

   tokio::spawn(async move {
      while let Some(outbound) = writer_rx.recv().await {
         if ws_writer
            .send(Message::Text(outbound.into()))
            .await
            .is_err()
         {
            break;
         }
      }
   });

   tokio::spawn(async move {
      while let Some(result) = ws_reader.next().await {
         match result {
            Ok(Message::Text(text)) => {
               for line in text.lines() {
                  let _ = reader_tx.send(ReaderEvent::Line(line.to_string()));
               }
            },
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(_) => {},
         }
      }

      let _ = reader_tx.send(ReaderEvent::Disconnected);
   });

   Ok((writer_tx, reader_rx, ChatIdentity {
      login:        account.login,
      display_name: account.display_name,
   }))
}

async fn ensure_channel_sender(
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
   channel: &str,
) {
   let mut guard = channels.write().await;
   guard.entry(channel.to_string()).or_insert_with(|| {
      let (sender, _receiver) = broadcast::channel(256);
      sender
   });
}

async fn get_channel_sender(
   channels: &Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
   channel: &str,
) -> Option<broadcast::Sender<ChatEvent>> {
   let guard = channels.read().await;
   guard.get(channel).cloned()
}

async fn enrich_chat_event_with_third_party_emotes(
   event: &mut ChatEvent,
   third_party_emote_cache: &Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
   auth: &TwitchAuthService,
) {
   if !matches!(event.kind, ChatEventKind::Message) {
      return;
   }

   let emotes_by_code = match third_party_emotes_for_channel(
      auth,
      third_party_emote_cache,
      &event.channel_login,
   )
   .await
   {
      Ok(emotes_by_code) => emotes_by_code,
      Err(error) => {
         tracing::debug!(error = %error, channel = %event.channel_login, "failed loading third-party emotes");
         return;
      },
   };

   if emotes_by_code.is_empty() {
      return;
   }

   event.parts = apply_third_party_emotes_to_parts(&event.parts, &emotes_by_code);
}

fn apply_third_party_emotes_to_parts(
   parts: &[ChatPart],
   emotes_by_code: &HashMap<String, String>,
) -> Vec<ChatPart> {
   let mut out = Vec::new();
   for part in parts {
      match part {
         ChatPart::Text { text } => {
            for segment in split_preserving_whitespace(text) {
               if segment.trim().is_empty() {
                  out.push(ChatPart::Text { text: segment });
               } else if let Some(image_url) = emotes_by_code.get(&segment) {
                  out.push(ChatPart::Emote {
                     id:        segment.clone(),
                     code:      segment,
                     image_url: Some(image_url.clone()),
                  });
               } else {
                  out.push(ChatPart::Text { text: segment });
               }
            }
         },
         ChatPart::Emote { .. } => out.push(part.clone()),
      }
   }

   if out.is_empty() {
      vec![ChatPart::Text {
         text: String::new(),
      }]
   } else {
      out
   }
}

fn split_preserving_whitespace(input: &str) -> Vec<String> {
   let mut out = Vec::new();
   let mut current = String::new();
   let mut current_whitespace: Option<bool> = None;

   for ch in input.chars() {
      let is_whitespace = ch.is_whitespace();
      match current_whitespace {
         None => {
            current_whitespace = Some(is_whitespace);
            current.push(ch);
         },
         Some(value) if value == is_whitespace => current.push(ch),
         Some(_) => {
            if !current.is_empty() {
               out.push(current.clone());
               current.clear();
            }
            current_whitespace = Some(is_whitespace);
            current.push(ch);
         },
      }
   }

   if !current.is_empty() {
      out.push(current);
   }

   out
}

fn remember_local_echo(pending_local_echo: &mut HashMap<String, u64>, event: &ChatEvent) {
   prune_local_echo_cache(pending_local_echo);
   if let Some(key) = local_echo_key(event) {
      pending_local_echo.insert(key, now_unix_secs().saturating_add(LOCAL_ECHO_TTL_SECS));
   }
}

fn is_duplicate_local_echo(
   pending_local_echo: &mut HashMap<String, u64>,
   event: &ChatEvent,
) -> bool {
   prune_local_echo_cache(pending_local_echo);
   let Some(key) = local_echo_key(event) else {
      return false;
   };

   if let Some(expires_at) = pending_local_echo.get(&key)
      && *expires_at > now_unix_secs()
   {
      pending_local_echo.remove(&key);
      return true;
   }

   false
}

fn prune_local_echo_cache(pending_local_echo: &mut HashMap<String, u64>) {
   let now = now_unix_secs();
   pending_local_echo.retain(|_, expires_at| *expires_at > now);
}

fn local_echo_key(event: &ChatEvent) -> Option<String> {
   if !matches!(event.kind, ChatEventKind::Message) {
      return None;
   }

   let sender = event.sender_login.as_ref()?.trim().to_ascii_lowercase();
   if sender.is_empty() {
      return None;
   }

   let channel = event.channel_login.trim().to_ascii_lowercase();
   if channel.is_empty() {
      return None;
   }

   let text = event.text.trim();
   if text.is_empty() {
      return None;
   }

   Some(format!("{channel}|{sender}|{text}"))
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn due_part_is_delayed_until_grace_deadline() {
      let (writer_tx, mut writer_rx) = mpsc::unbounded_channel();
      let mut state = ChatManagerState::new();
      state.connected = true;
      state.writer_tx = Some(writer_tx);
      state.joined_channels.insert("example".to_string());
      state.pending_parts.insert(
         "example".to_string(),
         Instant::now() + Duration::from_mins(1),
      );

      process_due_parts(&mut state);

      assert!(state.joined_channels.contains("example"));
      assert!(writer_rx.try_recv().is_err());
   }

   #[test]
   fn due_part_sends_part_and_removes_joined_channel() {
      let (writer_tx, mut writer_rx) = mpsc::unbounded_channel();
      let mut state = ChatManagerState::new();
      state.connected = true;
      state.writer_tx = Some(writer_tx);
      state.joined_channels.insert("example".to_string());
      state.pending_parts.insert(
         "example".to_string(),
         Instant::now() - Duration::from_secs(1),
      );

      process_due_parts(&mut state);

      assert!(!state.joined_channels.contains("example"));
      assert_eq!(writer_rx.try_recv().expect("PART command"), "PART #example");
   }

   #[test]
   fn due_part_is_skipped_after_resubscribe() {
      let (writer_tx, mut writer_rx) = mpsc::unbounded_channel();
      let mut state = ChatManagerState::new();
      state.connected = true;
      state.writer_tx = Some(writer_tx);
      state.joined_channels.insert("example".to_string());
      state.subscribed_counts.insert("example".to_string(), 1);
      state.pending_parts.insert(
         "example".to_string(),
         Instant::now() - Duration::from_secs(1),
      );

      process_due_parts(&mut state);

      assert!(state.joined_channels.contains("example"));
      assert!(writer_rx.try_recv().is_err());
      assert!(!state.pending_parts.contains_key("example"));
   }
}
