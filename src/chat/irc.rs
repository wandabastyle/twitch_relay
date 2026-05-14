use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, future::pending};
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::twitch_auth::TwitchAuthService;
use crate::util::time::now_unix_secs;
use crate::chat::events::{ChatEvent, ChatEventKind, ChatPart, parse_chat_event, fallback_sender_color};
use crate::chat::emotes::{third_party_emotes_for_channel, local_echo_parts_for_channel, CachedThirdPartyEmotes};
use crate::chat::service::ChatCommand;

const LOCAL_ECHO_TTL_SECS: u64 = 8;

#[derive(Debug)]
pub enum ReaderEvent {
    Line(String),
    Disconnected,
}

#[derive(Debug, Clone)]
struct ChatIdentity {
    login: String,
    display_name: String,
}

pub async fn run_chat_manager(
    auth: TwitchAuthService,
    mut command_rx: mpsc::UnboundedReceiver<ChatCommand>,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
    emote_cache: Arc<RwLock<HashMap<String, crate::chat::emotes::CachedEmoteEntry>>>,
    third_party_emote_cache: Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
) {
    let mut subscribed_counts: HashMap<String, usize> = HashMap::new();
    let mut joined_channels: HashSet<String> = HashSet::new();
    let mut connected = false;
    let mut last_error: Option<String> = None;

    let mut writer_tx: Option<mpsc::UnboundedSender<String>> = None;
    let mut reader_rx: Option<mpsc::UnboundedReceiver<ReaderEvent>> = None;
    let mut chat_identity: Option<ChatIdentity> = None;
    let mut pending_local_echo: HashMap<String, u64> = HashMap::new();

    loop {
        if !connected && !subscribed_counts.is_empty() {
            match connect_chat(&auth).await {
                Ok((tx, rx, identity)) => {
                    writer_tx = Some(tx);
                    reader_rx = Some(rx);
                    chat_identity = Some(identity.clone());
                    connected = true;
                    last_error = None;

                    if let Some(writer) = writer_tx.as_ref() {
                        let _ = writer.send(
                            "CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership"
                                .to_string(),
                        );
                        for channel in subscribed_counts.keys() {
                            let _ = writer.send(format!("JOIN #{channel}"));
                            joined_channels.insert(channel.clone());
                        }
                    }

                    tracing::info!(login = %identity.login, joined = joined_channels.len(), "chat IRC connected");
                }
                Err(e) => {
                    connected = false;
                    last_error = Some(e);
                }
            }
        }

        let read_event = async {
            if let Some(rx) = reader_rx.as_mut() {
                rx.recv().await
            } else {
                pending().await
            }
        };

        tokio::select! {
            maybe_cmd = command_rx.recv() => {
                let Some(command) = maybe_cmd else {
                    break;
                };

                match command {
                    ChatCommand::Subscribe { channel, response } => {
                        let entry = subscribed_counts.entry(channel.clone()).or_insert(0);
                        *entry = entry.saturating_add(1);
                        ensure_channel_sender(&channels, &channel).await;

                        if connected && !joined_channels.contains(&channel)
                            && let Some(writer) = writer_tx.as_ref()
                        {
                            let _ = writer.send(format!("JOIN #{channel}"));
                            joined_channels.insert(channel.clone());
                        }

                        let _ = response.send(Ok(()));
                    }
                    ChatCommand::Unsubscribe { channel, response } => {
                        if let Some(entry) = subscribed_counts.get_mut(&channel) {
                            if *entry > 1 {
                                *entry -= 1;
                            } else {
                                subscribed_counts.remove(&channel);
                                if connected
                                    && joined_channels.remove(&channel)
                                    && let Some(writer) = writer_tx.as_ref()
                                {
                                    let _ = writer.send(format!("PART #{channel}"));
                                }
                            }
                        }
                        let _ = response.send(Ok(()));
                    }
                    ChatCommand::SendMessage { channel, message, response } => {
                        if !connected {
                            let _ = response.send(Err(last_error.clone().unwrap_or_else(|| "chat connection unavailable".to_string())));
                            continue;
                        }

                        if !joined_channels.contains(&channel)
                            && let Some(writer) = writer_tx.as_ref()
                        {
                            let _ = writer.send(format!("JOIN #{channel}"));
                            joined_channels.insert(channel.clone());
                        }

                        if let Some(writer) = writer_tx.as_ref() {
                            let _ = writer.send(format!("PRIVMSG #{channel} :{message}"));

                            if let Some(identity) = chat_identity.as_ref()
                                && let Some(sender) = get_channel_sender(&channels, &channel).await
                            {
                                let echo_event = ChatEvent {
                                    kind: ChatEventKind::Message,
                                    channel_login: channel.clone(),
                                    sender_login: Some(identity.login.clone()),
                                    sender_display_name: Some(identity.display_name.clone()),
                                    sender_color: Some(fallback_sender_color(&identity.login)),
                                    text: message.clone(),
                                    parts: local_echo_parts_for_channel(
                                        &emote_cache,
                                        &third_party_emote_cache,
                                        &auth,
                                        &channel,
                                        &message,
                                    )
                                    .await,
                                    sent_at_unix: now_unix_secs(),
                                };
                                remember_local_echo(&mut pending_local_echo, &echo_event);
                                let _ = sender.send(echo_event);
                            }

                            let _ = response.send(Ok(()));
                        } else {
                            let _ = response.send(Err("chat writer is not available".to_string()));
                        }
                    }
                    ChatCommand::Status { channel, response } => {
                        let subscribed = subscribed_counts.get(&channel).copied().unwrap_or(0) > 0;
                        let _ = response.send(crate::chat::events::ChatChannelStatus {
                            subscribed,
                            connected,
                            error: last_error.clone(),
                        });
                    }
                }
            }
            maybe_event = read_event => {
                match maybe_event {
                    Some(ReaderEvent::Line(line)) => {
                        if let Some(writer) = writer_tx.as_ref()
                            && line.starts_with("PING ")
                        {
                            let payload = line.trim_start_matches("PING ").trim();
                            let _ = writer.send(format!("PONG {payload}"));
                            continue;
                        }

                        if let Some(mut event) = parse_chat_event(&line) {
                            enrich_chat_event_with_third_party_emotes(
                                &mut event,
                                &third_party_emote_cache,
                                &auth,
                            )
                            .await;

                            if !is_duplicate_local_echo(&mut pending_local_echo, &event)
                                && let Some(sender) = get_channel_sender(&channels, &event.channel_login).await
                            {
                                let _ = sender.send(event);
                            }
                        }
                    }
                    Some(ReaderEvent::Disconnected) => {
                        connected = false;
                        writer_tx = None;
                        reader_rx = None;
                        chat_identity = None;
                        joined_channels.clear();
                        last_error = Some("chat connection lost; retrying".to_string());
                    }
                    None => {}
                }
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
    String,
> {
    let account = auth.ensure_chat_account().await?;

    let (ws_stream, _response) = connect_async("wss://irc-ws.chat.twitch.tv:443")
        .await
        .map_err(|e| format!("chat websocket connect failed: {e}"))?;

    let (mut ws_writer, mut ws_reader) = ws_stream.split();

    ws_writer
        .send(Message::Text(
            format!("PASS oauth:{}", account.access_token).into(),
        ))
        .await
        .map_err(|e| format!("chat PASS failed: {e}"))?;
    ws_writer
        .send(Message::Text(format!("NICK {}", account.login).into()))
        .await
        .map_err(|e| format!("chat NICK failed: {e}"))?;

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
                }
                Ok(Message::Ping(_)) => {}
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }

        let _ = reader_tx.send(ReaderEvent::Disconnected);
    });

    Ok((
        writer_tx,
        reader_rx,
        ChatIdentity {
            login: account.login,
            display_name: account.display_name,
        },
    ))
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
        }
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
                            id: segment.clone(),
                            code: segment,
                            image_url: Some(image_url.clone()),
                        });
                    } else {
                        out.push(ChatPart::Text { text: segment });
                    }
                }
            }
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
            }
            Some(value) if value == is_whitespace => current.push(ch),
            Some(_) => {
                if !current.is_empty() {
                    out.push(current.clone());
                    current.clear();
                }
                current_whitespace = Some(is_whitespace);
                current.push(ch);
            }
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
