use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, sse::Event, sse::KeepAlive, sse::Sse},
};
use futures_util::{SinkExt, StreamExt, future::pending};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::twitch_auth::TwitchAuthService;

#[derive(Debug, Clone)]
pub struct ChatService {
    command_tx: mpsc::UnboundedSender<ChatCommand>,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
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
    pub sent_at_unix: u64,
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

#[derive(Debug, Serialize)]
pub struct ChatStatusResponse {
    pub status: ChatChannelStatus,
}

impl ChatService {
    pub fn new(auth: TwitchAuthService) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let channels = Arc::new(RwLock::new(HashMap::new()));

        tokio::spawn(run_chat_manager(auth, command_rx, channels.clone()));

        Self {
            command_tx,
            channels,
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

    async fn receiver_for_channel(&self, channel: &str) -> Result<broadcast::Receiver<ChatEvent>, String> {
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
}

pub async fn subscribe(State(state): State<ChatState>, Json(payload): Json<ChatChannelRequest>) -> Response {
    match state.service.subscribe_channel(&payload.channel_login).await {
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

pub async fn send(State(state): State<ChatState>, Json(payload): Json<ChatSendRequest>) -> Response {
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

pub async fn events(
    State(state): State<ChatState>,
    Path(channel): Path<String>,
) -> Response {
    let receiver = match state.service.receiver_for_channel(&channel).await {
        Ok(receiver) => receiver,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &e),
    };

    let stream = BroadcastStream::new(receiver).filter_map(|result| async move {
        match result {
            Ok(event) => {
                let sse_event = Event::default()
                    .event("chat")
                    .json_data(event)
                    .ok()?;
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

    loop {
        if !connected && !subscribed_counts.is_empty() {
            match connect_chat(&auth).await {
                Ok((tx, rx, login)) => {
                    writer_tx = Some(tx);
                    reader_rx = Some(rx);
                    connected = true;
                    last_error = None;

                    if let Some(writer) = writer_tx.as_ref() {
                        let _ = writer.send("CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership".to_string());
                        for channel in subscribed_counts.keys() {
                            let _ = writer.send(format!("JOIN #{channel}"));
                            joined_channels.insert(channel.clone());
                        }
                    }

                    tracing::info!(login = %login, joined = joined_channels.len(), "chat IRC connected");
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
                            && let Some(sender) = get_channel_sender(&channels, &event.channel_login).await
                        {
                            let _ = sender.send(event);
                        }
                    }
                    Some(ReaderEvent::Disconnected) => {
                        connected = false;
                        writer_tx = None;
                        reader_rx = None;
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
        String,
    ),
    String,
> {
    let account = auth.ensure_chat_account().await?;

    let (ws_stream, _response) = connect_async("wss://irc-ws.chat.twitch.tv:443")
        .await
        .map_err(|e| format!("chat websocket connect failed: {e}"))?;

    let (mut ws_writer, mut ws_reader) = ws_stream.split();

    ws_writer
        .send(Message::Text(format!("PASS oauth:{}", account.access_token).into()))
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
            if ws_writer.send(Message::Text(outbound.into())).await.is_err() {
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

    Ok((writer_tx, reader_rx, account.login))
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

            Some(ChatEvent {
                kind: ChatEventKind::Message,
                channel_login: channel,
                sender_login,
                sender_display_name,
                text: trailing.to_string(),
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
                sent_at_unix: now_unix_secs(),
            })
        }
        _ => None,
    }
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
