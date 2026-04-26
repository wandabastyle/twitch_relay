use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, sse::Event, sse::KeepAlive, sse::Sse},
};
use futures_util::{SinkExt, StreamExt, future::pending, stream};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::twitch_auth::TwitchAuthService;

#[derive(Debug, Clone)]
pub struct ChatService {
    auth: TwitchAuthService,
    command_tx: mpsc::UnboundedSender<ChatCommand>,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
    emote_cache: Arc<RwLock<HashMap<String, CachedEmoteEntry>>>,
}

#[derive(Debug, Clone)]
pub struct ChatState {
    pub service: ChatService,
}

#[derive(Debug)]
enum ChatCommand {
    Subscribe {
        channel: String,
        response: oneshot::Sender<Result<(), String>>,
    },
    Unsubscribe {
        channel: String,
        response: oneshot::Sender<Result<(), String>>,
    },
    SendMessage {
        channel: String,
        message: String,
        response: oneshot::Sender<Result<(), String>>,
    },
    Status {
        channel: String,
        response: oneshot::Sender<ChatChannelStatus>,
    },
}

#[derive(Debug)]
enum ReaderEvent {
    Line(String),
    Disconnected,
}

#[derive(Debug, Clone)]
struct ChatIdentity {
    login: String,
    display_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatEventKind {
    Message,
    Notice,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatEvent {
    pub kind: ChatEventKind,
    pub channel_login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_login: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_display_name: Option<String>,
    pub text: String,
    pub parts: Vec<ChatPart>,
    pub sent_at_unix: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmotePickerResponse {
    emotes: Vec<EmotePickerItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmotePickerItem {
    id: String,
    code: String,
    image_url: String,
    group_key: String,
    group_name: String,
}

#[derive(Debug, Clone)]
struct CachedEmoteEntry {
    expires_at_unix: u64,
    items: Vec<EmotePickerItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatPart {
    Text { text: String },
    Emote { id: String, code: String },
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatChannelStatus {
    pub subscribed: bool,
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChannelRequest {
    channel_login: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatSendRequest {
    channel_login: String,
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatStatusQuery {
    channel_login: String,
}

#[derive(Debug, Deserialize)]
pub struct EmotesQuery {
    channel_login: String,
}

#[derive(Debug, Serialize)]
pub struct ChatStatusResponse {
    pub status: ChatChannelStatus,
}

impl ChatService {
    pub fn new(auth: TwitchAuthService) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let channels = Arc::new(RwLock::new(HashMap::new()));
        let emote_cache = Arc::new(RwLock::new(HashMap::new()));

        tokio::spawn(run_chat_manager(auth.clone(), command_rx, channels.clone()));

        Self {
            auth,
            command_tx,
            channels,
            emote_cache,
        }
    }

    pub async fn subscribe_channel(&self, channel: &str) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ChatCommand::Subscribe {
                channel: normalize_channel(channel)?,
                response: tx,
            })
            .map_err(|_| "chat runtime is not available".to_string())?;
        rx.await
            .map_err(|_| "chat runtime did not return status".to_string())?
    }

    pub async fn unsubscribe_channel(&self, channel: &str) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ChatCommand::Unsubscribe {
                channel: normalize_channel(channel)?,
                response: tx,
            })
            .map_err(|_| "chat runtime is not available".to_string())?;
        rx.await
            .map_err(|_| "chat runtime did not return status".to_string())?
    }

    pub async fn send_message(&self, channel: &str, message: &str) -> Result<(), String> {
        let trimmed = message.trim();
        if trimmed.is_empty() {
            return Err("message cannot be empty".to_string());
        }
        if trimmed.chars().count() > 500 {
            return Err("message is too long".to_string());
        }

        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ChatCommand::SendMessage {
                channel: normalize_channel(channel)?,
                message: trimmed.to_string(),
                response: tx,
            })
            .map_err(|_| "chat runtime is not available".to_string())?;
        rx.await
            .map_err(|_| "chat runtime did not return status".to_string())?
    }

    pub async fn status(&self, channel: &str) -> Result<ChatChannelStatus, String> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ChatCommand::Status {
                channel: normalize_channel(channel)?,
                response: tx,
            })
            .map_err(|_| "chat runtime is not available".to_string())?;
        rx.await
            .map_err(|_| "chat runtime did not return status".to_string())
    }

    async fn receiver_for_channel(
        &self,
        channel: &str,
    ) -> Result<broadcast::Receiver<ChatEvent>, String> {
        let normalized = normalize_channel(channel)?;
        let mut guard = self.channels.write().await;
        let sender = guard
            .entry(normalized)
            .or_insert_with(|| {
                let (sender, _receiver) = broadcast::channel(256);
                sender
            })
            .clone();

        Ok(sender.subscribe())
    }

    pub async fn emotes_for_channel(&self, channel: &str) -> Result<Vec<EmotePickerItem>, String> {
        let normalized_channel = normalize_channel(channel)?;
        let account = self.auth.ensure_emote_account().await.map_err(|e| {
            if e.contains("user:read:emotes") {
                "missing Twitch emote scope. disconnect and reconnect Twitch account to grant user:read:emotes".to_string()
            } else {
                e
            }
        })?;

        self.emotes_for_channel_with_account(&normalized_channel, &account)
            .await
    }

    pub async fn prewarm_emotes_for_channels(&self, channels: &[String]) -> Result<(), String> {
        let account = self.auth.ensure_emote_account().await.map_err(|e| {
            if e.contains("user:read:emotes") {
                "missing Twitch emote scope. disconnect and reconnect Twitch account to grant user:read:emotes".to_string()
            } else {
                e
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
                async move {
                    (
                        channel.clone(),
                        self.emotes_for_channel_with_account(&channel, &account).await,
                    )
                }
            })
            .buffer_unordered(6)
            .for_each(|(channel, result)| async move {
                if let Err(error) = result {
                    tracing::debug!(error = %error, channel = %channel, "failed prewarming emotes for channel");
                }
            })
            .await;

        Ok(())
    }

    async fn emotes_for_channel_with_account(
        &self,
        normalized_channel: &str,
        account: &crate::twitch_auth::TwitchAccount,
    ) -> Result<Vec<EmotePickerItem>, String> {
        let cache_key = format!("{}:{normalized_channel}", account.user_id);
        let now = now_unix_secs();

        {
            let cache = self.emote_cache.read().await;
            if let Some(entry) = cache.get(&cache_key)
                && entry.expires_at_unix > now
            {
                return Ok(entry.items.clone());
            }
        }

        let client = self.auth.api_client();
        let client_id = self.auth.client_id();
        let broadcaster = resolve_user_by_login(
            &client,
            &client_id,
            &account.access_token,
            normalized_channel,
        )
        .await?;

        let channel_emotes =
            fetch_channel_emotes(&client, &client_id, &account.access_token, &broadcaster.id)
                .await?;
        let user_emotes = fetch_user_emotes(
            &client,
            &client_id,
            &account.access_token,
            &account.user_id,
            &broadcaster.id,
        )
        .await?;

        let mut owner_ids = HashSet::new();
        for emote in &user_emotes {
            if let Some(owner_id) = emote.owner_id.as_ref()
                && !owner_id.trim().is_empty()
            {
                owner_ids.insert(owner_id.trim().to_string());
            }
        }

        let owner_names = resolve_user_display_names_by_ids(
            &client,
            &client_id,
            &account.access_token,
            owner_ids.into_iter().collect(),
        )
        .await;

        let mut merged = Vec::new();
        let mut seen = HashSet::new();
        let watched_group_name = broadcaster
            .display_name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| normalized_channel.to_string());
        let watched_group_key = format!("owner:{}", broadcaster.id);

        for emote in channel_emotes {
            if seen.insert(emote.id.clone()) {
                let emote_id = emote.id.clone();
                merged.push(EmotePickerItem {
                    id: emote_id.clone(),
                    code: emote.name,
                    image_url: resolve_emote_url(&emote.images, &emote.format, &emote_id),
                    group_key: watched_group_key.clone(),
                    group_name: watched_group_name.clone(),
                });
            }
        }

        for emote in user_emotes {
            if !seen.insert(emote.id.clone()) {
                continue;
            }
            let owner_id = emote.owner_id.clone().unwrap_or_default();
            let owner_name = if owner_id.trim().is_empty() {
                "Global".to_string()
            } else if owner_id == broadcaster.id {
                watched_group_name.clone()
            } else {
                owner_names
                    .get(owner_id.trim())
                    .cloned()
                    .unwrap_or_else(|| format!("Channel {}", owner_id))
            };
            let emote_id = emote.id.clone();
            let group_key = if owner_id.trim().is_empty() {
                "global".to_string()
            } else {
                format!("owner:{}", owner_id.trim())
            };
            merged.push(EmotePickerItem {
                id: emote_id.clone(),
                code: emote.name,
                image_url: resolve_emote_url(&emote.images, &emote.format, &emote_id),
                group_key,
                group_name: owner_name,
            });
        }

        merged.sort_by(|a, b| {
            let a_priority = if a.group_key.as_str() == watched_group_key.as_str() {
                0
            } else {
                1
            };
            let b_priority = if b.group_key.as_str() == watched_group_key.as_str() {
                0
            } else {
                1
            };

            a_priority
                .cmp(&b_priority)
                .then_with(|| {
                    a.group_name
                        .to_ascii_lowercase()
                        .cmp(&b.group_name.to_ascii_lowercase())
                })
                .then_with(|| {
                    a.code
                        .to_ascii_lowercase()
                        .cmp(&b.code.to_ascii_lowercase())
                })
        });

        let mut cache = self.emote_cache.write().await;
        cache.insert(
            cache_key,
            CachedEmoteEntry {
                expires_at_unix: now.saturating_add(Duration::from_secs(120).as_secs()),
                items: merged.clone(),
            },
        );

        Ok(merged)
    }
}

pub async fn subscribe(
    State(state): State<ChatState>,
    Json(payload): Json<ChatChannelRequest>,
) -> Response {
    match state
        .service
        .subscribe_channel(&payload.channel_login)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::warn!(error = %e, channel = %payload.channel_login, "failed subscribing chat channel");
            error_response(StatusCode::BAD_REQUEST, &e)
        }
    }
}

pub async fn unsubscribe(State(state): State<ChatState>, Path(channel): Path<String>) -> Response {
    match state.service.unsubscribe_channel(&channel).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::warn!(error = %e, channel = %channel, "failed unsubscribing chat channel");
            error_response(StatusCode::BAD_REQUEST, &e)
        }
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
            error_response(StatusCode::BAD_REQUEST, &e)
        }
    }
}

pub async fn status(
    State(state): State<ChatState>,
    Query(query): Query<ChatStatusQuery>,
) -> Response {
    match state.service.status(&query.channel_login).await {
        Ok(status_value) => Json(ChatStatusResponse {
            status: status_value,
        })
        .into_response(),
        Err(e) => error_response(StatusCode::BAD_REQUEST, &e),
    }
}

pub async fn emotes(State(state): State<ChatState>, Query(query): Query<EmotesQuery>) -> Response {
    match state.service.emotes_for_channel(&query.channel_login).await {
        Ok(items) => Json(EmotePickerResponse { emotes: items }).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, channel = %query.channel_login, "failed loading chat emotes");
            error_response(StatusCode::BAD_REQUEST, &e)
        }
    }
}

pub async fn events(State(state): State<ChatState>, Path(channel): Path<String>) -> Response {
    let receiver = match state.service.receiver_for_channel(&channel).await {
        Ok(receiver) => receiver,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &e),
    };

    let stream = BroadcastStream::new(receiver).filter_map(|result| async move {
        match result {
            Ok(event) => {
                let sse_event = Event::default().event("chat").json_data(event).ok()?;
                Some(Ok::<Event, Infallible>(sse_event))
            }
            Err(_) => None,
        }
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(12)))
        .into_response()
}

async fn run_chat_manager(
    auth: TwitchAuthService,
    mut command_rx: mpsc::UnboundedReceiver<ChatCommand>,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
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
                                    text: message.clone(),
                                    parts: vec![ChatPart::Text {
                                        text: message.clone(),
                                    }],
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
                        let _ = response.send(ChatChannelStatus {
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

                        if let Some(event) = parse_chat_event(&line)
                            && !is_duplicate_local_echo(&mut pending_local_echo, &event)
                            && let Some(sender) = get_channel_sender(&channels, &event.channel_login).await
                        {
                            let _ = sender.send(event);
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

fn parse_chat_event(line: &str) -> Option<ChatEvent> {
    let mut rest = line.trim();
    if rest.is_empty() {
        return None;
    }

    let mut tags: HashMap<&str, &str> = HashMap::new();

    if rest.starts_with('@') {
        let (raw_tags, remaining) = rest.split_once(' ')?;
        for pair in raw_tags.trim_start_matches('@').split(';') {
            if let Some((key, value)) = pair.split_once('=') {
                tags.insert(key, value);
            }
        }
        rest = remaining;
    }

    if rest.starts_with(':') {
        let (_, remaining) = rest.split_once(' ')?;
        rest = remaining;
    }

    let (command, tail) = rest.split_once(' ').unwrap_or((rest, ""));
    let trailing = tail.split_once(" :").map(|(_, value)| value).unwrap_or("");

    match command {
        "PRIVMSG" => {
            let mut pieces = tail.split_whitespace();
            let channel = pieces.next()?.trim_start_matches('#').to_ascii_lowercase();
            if channel.is_empty() {
                return None;
            }

            let sender_login = tags.get("login").map(|v| (*v).to_string());
            let sender_display_name = tags.get("display-name").map(|v| (*v).to_string());
            let parts = parse_message_parts(trailing, tags.get("emotes").copied());

            Some(ChatEvent {
                kind: ChatEventKind::Message,
                channel_login: channel,
                sender_login,
                sender_display_name,
                text: trailing.to_string(),
                parts,
                sent_at_unix: now_unix_secs(),
            })
        }
        "NOTICE" => {
            let mut pieces = tail.split_whitespace();
            let channel = pieces.next()?.trim_start_matches('#').to_ascii_lowercase();
            if channel.is_empty() {
                return None;
            }

            Some(ChatEvent {
                kind: ChatEventKind::Notice,
                channel_login: channel,
                sender_login: None,
                sender_display_name: None,
                text: trailing.to_string(),
                parts: vec![ChatPart::Text {
                    text: trailing.to_string(),
                }],
                sent_at_unix: now_unix_secs(),
            })
        }
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct TwitchUsersResponse {
    data: Vec<TwitchUser>,
}

#[derive(Debug, Deserialize)]
struct TwitchUser {
    id: String,
    #[serde(default)]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmotesApiResponse {
    #[serde(default)]
    data: Vec<EmoteApiItem>,
    #[serde(default)]
    template: Option<String>,
    #[serde(default)]
    pagination: EmotesPagination,
}

#[derive(Debug, Deserialize, Default)]
struct EmotesPagination {
    #[serde(default)]
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmoteApiItem {
    id: String,
    name: String,
    #[serde(default)]
    owner_id: Option<String>,
    #[serde(default)]
    format: Vec<String>,
    #[serde(default)]
    images: EmoteApiImages,
}

#[derive(Debug, Deserialize, Default)]
struct EmoteApiImages {
    #[serde(default)]
    url_1x: Option<String>,
    #[serde(default)]
    url_2x: Option<String>,
    #[serde(default)]
    template: Option<String>,
}

async fn resolve_user_by_login(
    client: &reqwest::Client,
    client_id: &str,
    access_token: &str,
    login: &str,
) -> Result<TwitchUser, String> {
    let response = client
        .get("https://api.twitch.tv/helix/users")
        .header("Client-Id", client_id)
        .header("Authorization", format!("Bearer {access_token}"))
        .query(&[("login", login)])
        .send()
        .await
        .map_err(|e| format!("resolve channel user id failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "resolve channel user id failed with status {}",
            response.status()
        ));
    }

    let payload: TwitchUsersResponse = response
        .json()
        .await
        .map_err(|e| format!("decode channel user id response failed: {e}"))?;

    payload
        .data
        .into_iter()
        .next()
        .ok_or("channel not found".to_string())
}

async fn resolve_user_display_names_by_ids(
    client: &reqwest::Client,
    client_id: &str,
    access_token: &str,
    user_ids: Vec<String>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if user_ids.is_empty() {
        return out;
    }

    for chunk in user_ids.chunks(100) {
        let response = match client
            .get("https://api.twitch.tv/helix/users")
            .header("Client-Id", client_id)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&chunk.iter().map(|id| ("id", id)).collect::<Vec<_>>())
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(error = %error, "resolve user names by ids request failed");
                continue;
            }
        };

        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "resolve user names by ids returned non-success status");
            continue;
        }

        let payload: TwitchUsersResponse = match response.json().await {
            Ok(payload) => payload,
            Err(error) => {
                tracing::warn!(error = %error, "decode user names by ids failed");
                continue;
            }
        };

        for user in payload.data {
            out.insert(
                user.id,
                user.display_name
                    .clone()
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| "Unknown channel".to_string()),
            );
        }
    }

    out
}

async fn fetch_channel_emotes(
    client: &reqwest::Client,
    client_id: &str,
    access_token: &str,
    broadcaster_id: &str,
) -> Result<Vec<EmoteApiItem>, String> {
    let response = client
        .get("https://api.twitch.tv/helix/chat/emotes")
        .header("Client-Id", client_id)
        .header("Authorization", format!("Bearer {access_token}"))
        .query(&[("broadcaster_id", broadcaster_id)])
        .send()
        .await
        .map_err(|e| format!("fetch channel emotes failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "fetch channel emotes failed with status {}",
            response.status()
        ));
    }

    let payload: EmotesApiResponse = response
        .json()
        .await
        .map_err(|e| format!("decode channel emotes failed: {e}"))?;

    let template = payload.template;
    Ok(payload
        .data
        .into_iter()
        .map(|mut item| {
            if item.images.template.is_none() {
                item.images.template = template.clone();
            }
            item
        })
        .collect())
}

async fn fetch_user_emotes(
    client: &reqwest::Client,
    client_id: &str,
    access_token: &str,
    user_id: &str,
    broadcaster_id: &str,
) -> Result<Vec<EmoteApiItem>, String> {
    let mut after: Option<String> = None;
    let mut out = Vec::new();

    loop {
        let mut req = client
            .get("https://api.twitch.tv/helix/chat/emotes/user")
            .header("Client-Id", client_id)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[
                ("user_id", user_id),
                ("broadcaster_id", broadcaster_id),
                ("first", "100"),
            ]);

        if let Some(cursor) = after.as_ref() {
            req = req.query(&[("after", cursor)]);
        }

        let response = req
            .send()
            .await
            .map_err(|e| format!("fetch user emotes failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "fetch user emotes failed with status {}",
                response.status()
            ));
        }

        let payload: EmotesApiResponse = response
            .json()
            .await
            .map_err(|e| format!("decode user emotes failed: {e}"))?;

        let template = payload.template;
        for mut item in payload.data {
            if item.images.template.is_none() {
                item.images.template = template.clone();
            }
            out.push(item);
        }

        let Some(cursor) = payload.pagination.cursor else {
            break;
        };
        if cursor.trim().is_empty() {
            break;
        }
        after = Some(cursor);
    }

    Ok(out)
}

fn resolve_emote_url(images: &EmoteApiImages, formats: &[String], emote_id: &str) -> String {
    let supports_animated = formats
        .iter()
        .any(|value| value.trim().eq_ignore_ascii_case("animated"));

    if let Some(tpl) = images.template.as_deref() {
        let format = if supports_animated {
            "animated"
        } else {
            "default"
        };

        return tpl
            .replace("{{id}}", emote_id)
            .replace("{{format}}", format)
            .replace("{{theme_mode}}", "dark")
            .replace("{{scale}}", "2.0");
    }

    if let Some(url) = images.url_2x.as_ref().filter(|v| !v.trim().is_empty()) {
        return url.to_string();
    }
    if let Some(url) = images.url_1x.as_ref().filter(|v| !v.trim().is_empty()) {
        return url.to_string();
    }

    format!(
        "https://static-cdn.jtvnw.net/emoticons/v2/{}/default/dark/2.0",
        emote_id
    )
}

#[derive(Debug, Clone)]
struct EmoteOccurrence {
    id: String,
    start: usize,
    end: usize,
}

fn parse_message_parts(message: &str, emotes_tag: Option<&str>) -> Vec<ChatPart> {
    let chars: Vec<char> = message.chars().collect();
    if chars.is_empty() {
        return vec![ChatPart::Text {
            text: String::new(),
        }];
    }

    let mut occurrences = parse_emote_occurrences(emotes_tag, chars.len());
    if occurrences.is_empty() {
        return vec![ChatPart::Text {
            text: message.to_string(),
        }];
    }

    occurrences.sort_by_key(|occurrence| (occurrence.start, occurrence.end));

    let mut parts = Vec::new();
    let mut cursor = 0_usize;

    for occurrence in occurrences {
        if occurrence.start < cursor {
            continue;
        }

        if occurrence.start > cursor {
            let text = chars[cursor..occurrence.start].iter().collect::<String>();
            if !text.is_empty() {
                parts.push(ChatPart::Text { text });
            }
        }

        let emote_text = chars[occurrence.start..=occurrence.end]
            .iter()
            .collect::<String>();

        parts.push(ChatPart::Emote {
            id: occurrence.id,
            code: emote_text,
        });
        cursor = occurrence.end.saturating_add(1);
    }

    if cursor < chars.len() {
        let text = chars[cursor..].iter().collect::<String>();
        if !text.is_empty() {
            parts.push(ChatPart::Text { text });
        }
    }

    if parts.is_empty() {
        vec![ChatPart::Text {
            text: message.to_string(),
        }]
    } else {
        parts
    }
}

fn parse_emote_occurrences(emotes_tag: Option<&str>, char_len: usize) -> Vec<EmoteOccurrence> {
    let Some(raw) = emotes_tag else {
        return Vec::new();
    };

    if raw.trim().is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for emote_def in raw.split('/') {
        let Some((id, positions)) = emote_def.split_once(':') else {
            continue;
        };

        let emote_id = id.trim();
        if emote_id.is_empty() {
            continue;
        }

        for position in positions.split(',') {
            let Some((start_raw, end_raw)) = position.split_once('-') else {
                continue;
            };

            let Ok(start) = start_raw.parse::<usize>() else {
                continue;
            };
            let Ok(end) = end_raw.parse::<usize>() else {
                continue;
            };

            if start > end || end >= char_len {
                continue;
            }

            out.push(EmoteOccurrence {
                id: emote_id.to_string(),
                start,
                end,
            });
        }
    }

    out
}

fn remember_local_echo(pending_local_echo: &mut HashMap<String, u64>, event: &ChatEvent) {
    prune_local_echo_cache(pending_local_echo);
    if let Some(key) = local_echo_key(event) {
        pending_local_echo.insert(key, now_unix_secs().saturating_add(8));
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

fn normalize_channel(channel: &str) -> Result<String, String> {
    let normalized = channel.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("channel login cannot be empty".to_string());
    }
    Ok(normalized)
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
