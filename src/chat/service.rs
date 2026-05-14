use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, sse::Event, sse::KeepAlive, sse::Sse},
};
use futures_util::stream::{self, StreamExt};
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};
use tokio_stream::wrappers::BroadcastStream;

use crate::chat::emotes::{CachedEmoteEntry, CachedOwnerName, emotes_for_channel_with_account};
use crate::chat::emotes::{EmotePickerItem, EmotePickerResponse};
use crate::chat::events::{
    ChatChannelRequest, ChatChannelStatus, ChatEvent, ChatSendRequest, ChatStatusQuery,
    ChatStatusResponse, EmotesQuery,
};
use crate::chat::irc::run_chat_manager;
use crate::routes::error::error_response;
use crate::twitch_auth::TwitchAuthService;
use crate::util::channel::normalize_channel_login;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChatError {
    #[error("chat runtime is not available")]
    RuntimeUnavailable,
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
        "missing Twitch emote scope. disconnect and reconnect Twitch account to grant user:read:emotes"
    )]
    MissingEmoteScope,
    #[error("{0}")]
    Other(String),
}

impl From<String> for ChatError {
    fn from(msg: String) -> Self {
        // Try to parse known error patterns
        if msg.contains("user:read:emotes") {
            ChatError::MissingEmoteScope
        } else {
            ChatError::Other(msg)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatService {
    auth: TwitchAuthService,
    command_tx: mpsc::UnboundedSender<ChatCommand>,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<ChatEvent>>>>,
    emote_cache: Arc<RwLock<HashMap<String, CachedEmoteEntry>>>,
    owner_name_cache: Arc<RwLock<HashMap<String, CachedOwnerName>>>,
    owner_lookup_cooldown_until_unix: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
pub struct ChatState {
    pub service: ChatService,
}

#[derive(Debug)]
pub enum ChatCommand {
    Subscribe {
        channel: String,
        response: oneshot::Sender<Result<(), ChatError>>,
    },
    Unsubscribe {
        channel: String,
        response: oneshot::Sender<Result<(), ChatError>>,
    },
    SendMessage {
        channel: String,
        message: String,
        response: oneshot::Sender<Result<(), ChatError>>,
    },
    Status {
        channel: String,
        response: oneshot::Sender<ChatChannelStatus>,
    },
}

impl ChatService {
    pub fn new(auth: TwitchAuthService) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let channels = Arc::new(RwLock::new(HashMap::new()));
        let emote_cache = Arc::new(RwLock::new(HashMap::new()));
        let third_party_emote_cache = Arc::new(RwLock::new(HashMap::new()));
        let owner_name_cache = Arc::new(RwLock::new(HashMap::new()));
        let owner_lookup_cooldown_until_unix = Arc::new(AtomicU64::new(0));

        tokio::spawn(run_chat_manager(
            auth.clone(),
            command_rx,
            channels.clone(),
            emote_cache.clone(),
            third_party_emote_cache.clone(),
        ));

        Self {
            auth,
            command_tx,
            channels,
            emote_cache,
            owner_name_cache,
            owner_lookup_cooldown_until_unix,
        }
    }

    pub async fn subscribe_channel(&self, channel: &str) -> Result<(), ChatError> {
        let (tx, rx) = oneshot::channel();
        let normalized =
            normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
        self.command_tx
            .send(ChatCommand::Subscribe {
                channel: normalized,
                response: tx,
            })
            .map_err(|_| ChatError::RuntimeUnavailable)?;
        rx.await.map_err(|_| ChatError::StatusTimeout)?
    }

    pub async fn unsubscribe_channel(&self, channel: &str) -> Result<(), ChatError> {
        let (tx, rx) = oneshot::channel();
        let normalized =
            normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
        self.command_tx
            .send(ChatCommand::Unsubscribe {
                channel: normalized,
                response: tx,
            })
            .map_err(|_| ChatError::RuntimeUnavailable)?;
        rx.await.map_err(|_| ChatError::StatusTimeout)?
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
        self.command_tx
            .send(ChatCommand::SendMessage {
                channel: normalized,
                message: trimmed.to_string(),
                response: tx,
            })
            .map_err(|_| ChatError::RuntimeUnavailable)?;
        rx.await.map_err(|_| ChatError::StatusTimeout)?
    }

    pub async fn status(&self, channel: &str) -> Result<ChatChannelStatus, ChatError> {
        let (tx, rx) = oneshot::channel();
        let normalized =
            normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
        self.command_tx
            .send(ChatCommand::Status {
                channel: normalized,
                response: tx,
            })
            .map_err(|_| ChatError::RuntimeUnavailable)?;
        rx.await.map_err(|_| ChatError::StatusTimeout)
    }

    pub async fn receiver_for_channel(
        &self,
        channel: &str,
    ) -> Result<broadcast::Receiver<ChatEvent>, ChatError> {
        let normalized =
            normalize_channel_login(channel).map_err(|_| ChatError::InvalidChannelName)?;
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
            error_response(StatusCode::BAD_REQUEST, &e.to_string(), None)
        }
    }
}

pub async fn unsubscribe(State(state): State<ChatState>, Path(channel): Path<String>) -> Response {
    match state.service.unsubscribe_channel(&channel).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::warn!(error = %e, channel = %channel, "failed unsubscribing chat channel");
            error_response(StatusCode::BAD_REQUEST, &e.to_string(), None)
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
            error_response(StatusCode::BAD_REQUEST, &e.to_string(), None)
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
        Err(e) => error_response(StatusCode::BAD_REQUEST, &e.to_string(), None),
    }
}

pub async fn emotes(State(state): State<ChatState>, Query(query): Query<EmotesQuery>) -> Response {
    match state.service.emotes_for_channel(&query.channel_login).await {
        Ok(items) => Json(EmotePickerResponse { emotes: items }).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, channel = %query.channel_login, "failed loading chat emotes");
            error_response(StatusCode::BAD_REQUEST, &e.to_string(), None)
        }
    }
}

pub async fn events(State(state): State<ChatState>, Path(channel): Path<String>) -> Response {
    let receiver = match state.service.receiver_for_channel(&channel).await {
        Ok(receiver) => receiver,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &e.to_string(), None),
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
