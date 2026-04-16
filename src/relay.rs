use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rand::Rng;
use tokio::{net::TcpListener, process::Command, sync::RwLock};

#[derive(Debug, Clone)]
pub struct RelayService {
    streams: Arc<RwLock<HashMap<String, ActiveStream>>>,
    streamlink_path: String,
}

#[derive(Debug, Clone)]
pub struct ActiveStream {
    pub channel: String,
    pub session_token: String,
    pub port: u16,
    #[allow(dead_code)]
    pub started_at_unix: u64,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub stream_id: String,
    pub channel: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub enum RelayError {
    PortUnavailable,
    #[allow(dead_code)]
    SpawnFailed(String),
    #[allow(dead_code)]
    ChannelOffline,
    StreamNotFound,
    SessionMismatch,
}

impl RelayService {
    pub fn new(streamlink_path: Option<String>) -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            streamlink_path: streamlink_path.unwrap_or_else(|| "streamlink".to_string()),
        }
    }

    pub async fn spawn(
        &self,
        channel: &str,
        session_token: String,
    ) -> Result<StreamInfo, RelayError> {
        let port = find_free_port().await.ok_or(RelayError::PortUnavailable)?;
        let stream_id = generate_stream_id();
        let normalized_channel = channel.trim().to_ascii_lowercase();
        let streamlink_path = self.streamlink_path.clone();
        let ch = normalized_channel.clone();
        let sid = stream_id.clone();
        let streams = self.streams.clone();

        tokio::spawn(async move {
            let result = Command::new(&streamlink_path)
                .args([
                    &format!("https://twitch.tv/{ch}"),
                    "best",
                    "--player-external-http",
                    "--player-external-http-port",
                    &port.to_string(),
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true)
                .spawn();

            match result {
                Ok(mut child) => {
                    tracing::info!(channel = %ch, port = %port, stream_id = %sid, "streamlink started");
                    let mut guard = streams.write().await;
                    guard.insert(
                        sid.clone(),
                        ActiveStream {
                            channel: ch.clone(),
                            session_token: session_token.clone(),
                            port,
                            started_at_unix: now_unix_secs(),
                        },
                    );
                    drop(guard);
                    let status = child.wait().await;
                    tracing::info!(stream_id = %sid, status = ?status, "streamlink exited");
                }
                Err(err) => {
                    tracing::error!(channel = %ch, error = %err, "streamlink spawn failed");
                }
            }

            let mut guard = streams.write().await;
            guard.remove(&sid);
        });

        Ok(StreamInfo {
            stream_id,
            channel: normalized_channel,
            port,
        })
    }

    pub async fn validate(
        &self,
        stream_id: &str,
        session_token: &str,
    ) -> Result<StreamInfo, RelayError> {
        let guard = self.streams.read().await;
        let Some(stream) = guard.get(stream_id) else {
            return Err(RelayError::StreamNotFound);
        };

        if stream.session_token != session_token {
            return Err(RelayError::SessionMismatch);
        }

        Ok(StreamInfo {
            stream_id: stream_id.to_string(),
            channel: stream.channel.clone(),
            port: stream.port,
        })
    }

    #[allow(dead_code)]
    pub async fn cleanup(&self, stream_id: &str) {
        let mut guard = self.streams.write().await;
        if guard.remove(stream_id).is_some() {
            tracing::info!(stream_id = %stream_id, "stream cleanup requested");
        }
    }

    #[allow(dead_code)]
    pub async fn cleanup_all(&self) {
        let mut guard = self.streams.write().await;
        let count = guard.len();
        guard.clear();
        if count > 0 {
            tracing::info!(count = %count, "all streams cleanup requested");
        }
    }

    #[allow(dead_code)]
    pub async fn list(&self) -> Vec<StreamInfo> {
        let guard = self.streams.read().await;
        guard
            .iter()
            .map(|(id, s)| StreamInfo {
                stream_id: id.clone(),
                channel: s.channel.clone(),
                port: s.port,
            })
            .collect()
    }
}

async fn find_free_port() -> Option<u16> {
    let addr: SocketAddr = "127.0.0.1:0".parse().ok()?;
    let listener = TcpListener::bind(addr).await.ok()?;
    let local_addr = listener.local_addr().ok()?;
    drop(listener);
    Some(local_addr.port())
}

fn generate_stream_id() -> String {
    let mut rng = rand::thread_rng();
    (0..24)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            chars[idx] as char
        })
        .collect()
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
