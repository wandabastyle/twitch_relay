use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::channels;

use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

const GQL_ENDPOINT: &str = "https://gql.twitch.tv/gql";
const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const CACHE_TTL_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatus {
    pub live: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveStatusResponse {
    pub channels: HashMap<String, ChannelStatus>,
}

#[derive(Debug, Clone)]
struct CachedStatus {
    status: ChannelStatus,
    fetched_at: Instant,
}

#[derive(Debug, Clone)]
pub struct LiveStatusService {
    client: Client,
    cache: Arc<RwLock<HashMap<String, CachedStatus>>>,
}

impl LiveStatusService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check_multiple(&self, channels: &[String]) -> LiveStatusResponse {
        let manual_channels = channels::load_stored_channels();
        let mut manual_profile_urls: HashMap<String, Option<String>> = HashMap::new();
        for channel in manual_channels {
            manual_profile_urls.insert(channel.login.to_ascii_lowercase(), channel.profile_url);
        }

        let mut result = HashMap::new();
        let mut missing = Vec::new();

        for channel in channels {
            let normalized = channel.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                continue;
            }

            if let Some(cached) = self.get_cached(&normalized).await {
                result.insert(normalized, cached);
            } else {
                missing.push(normalized);
            }
        }

        let fetches = stream::iter(missing)
            .map(|login| async move {
                let status = self.fetch_status(&login).await.unwrap_or(ChannelStatus {
                    live: false,
                    viewer_count: None,
                    game: None,
                    title: None,
                    profile_url: None,
                    display_name: None,
                });
                (login, status)
            })
            .buffer_unordered(16);

        tokio::pin!(fetches);

        while let Some((login, status)) = fetches.next().await {
            self.set_cached(&login, status.clone()).await;

            if let Some(profile_url) = status.profile_url.as_ref()
                && let Some(stored_url) = manual_profile_urls.get(&login)
                && stored_url.as_deref() != Some(profile_url)
            {
                let _ = self.refresh_channel_image(&login, profile_url).await;
            }

            result.insert(login, status);
        }

        LiveStatusResponse { channels: result }
    }

    async fn refresh_channel_image(&self, login: &str, image_url: &str) -> Option<()> {
        let response = self.client.get(image_url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }
        let bytes = response.bytes().await.ok()?;
        match channels::save_channel_image(login, &bytes) {
            Ok(filename) => {
                if let Err(e) = channels::update_channel_image(login, &filename, image_url) {
                    tracing::warn!(error = %e, login = %login, "failed to update channel image");
                }
                Some(())
            }
            Err(e) => {
                tracing::warn!(error = %e, login = %login, "failed to save channel image");
                None
            }
        }
    }

    async fn get_cached(&self, channel: &str) -> Option<ChannelStatus> {
        let cache = self.cache.read().await;
        cache.get(channel).and_then(|cached| {
            if cached.fetched_at.elapsed() < Duration::from_secs(CACHE_TTL_SECS) {
                Some(cached.status.clone())
            } else {
                None
            }
        })
    }

    async fn set_cached(&self, channel: &str, status: ChannelStatus) {
        let mut cache = self.cache.write().await;
        cache.insert(
            channel.to_string(),
            CachedStatus {
                status,
                fetched_at: Instant::now(),
            },
        );
    }

    async fn fetch_status(&self, channel: &str) -> Option<ChannelStatus> {
        let query = serde_json::json!({
            "query": "query($login: String!) { user(login: $login) { id displayName profileImageURL(width: 300) stream { id title viewersCount game { name } } } }",
            "variables": { "login": channel }
        });

        let response = self
            .client
            .post(GQL_ENDPOINT)
            .header("client-id", CLIENT_ID)
            .header("Content-Type", "application/json")
            .json(&query)
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            tracing::warn!(
                status = %response.status(),
                channel = %channel,
                "GraphQL request failed"
            );
            return None;
        }

        let gql_response: GqlResponse = response.json().await.ok()?;

        Some(gql_response.into_channel_status())
    }

    pub async fn fetch_profile_image(&self, channel: &str) -> Option<(String, String)> {
        let query = serde_json::json!({
            "query": "query($login: String!) { user(login: $login) { profileImageURL(width: 300) } }",
            "variables": { "login": channel }
        });

        let response = self
            .client
            .post(GQL_ENDPOINT)
            .header("client-id", CLIENT_ID)
            .header("Content-Type", "application/json")
            .json(&query)
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            tracing::warn!(
                status = %response.status(),
                channel = %channel,
                "GraphQL profile image request failed"
            );
            return None;
        }

        let gql_response: GqlImageResponse = response.json().await.ok()?;
        let profile_url = gql_response.into_profile_url()?;

        let image_response = self.client.get(&profile_url).send().await.ok()?;

        if !image_response.status().is_success() {
            tracing::warn!(
                status = %image_response.status(),
                channel = %channel,
                "Profile image download failed"
            );
            return None;
        }

        let image_data = image_response.bytes().await.ok()?;
        match channels::save_channel_image(channel, &image_data) {
            Ok(filename) => {
                if let Err(e) = channels::update_channel_image(channel, &filename, &profile_url) {
                    tracing::warn!(error = %e, channel = %channel, "failed to update channel image filename");
                }
                Some((filename, profile_url))
            }
            Err(e) => {
                tracing::warn!(error = %e, channel = %channel, "failed to save channel image");
                None
            }
        }
    }
}

impl Default for LiveStatusService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct GqlResponse {
    data: Option<GqlData>,
}

#[derive(Debug, Deserialize)]
struct GqlData {
    user: Option<GqlUser>,
}

#[derive(Debug, Deserialize)]
struct GqlUser {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
    stream: Option<GqlStream>,
}

#[derive(Debug, Deserialize)]
struct GqlStream {
    id: Option<String>,
    title: Option<String>,
    #[serde(rename = "viewersCount")]
    viewer_count: Option<u64>,
    game: Option<GqlGame>,
}

#[derive(Debug, Deserialize)]
struct GqlGame {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GqlImageResponse {
    data: Option<GqlImageData>,
}

#[derive(Debug, Deserialize)]
struct GqlImageData {
    user: Option<GqlImageUser>,
}

#[derive(Debug, Deserialize)]
struct GqlImageUser {
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
}

impl GqlImageResponse {
    fn into_profile_url(self) -> Option<String> {
        self.data?.user?.profile_image_url
    }
}

impl GqlResponse {
    fn into_channel_status(self) -> ChannelStatus {
        match self.data {
            Some(GqlData { user: Some(user) }) => {
                let stream = user.stream;
                ChannelStatus {
                    live: stream.as_ref().is_some_and(|s| s.id.is_some()),
                    viewer_count: stream.as_ref().and_then(|s| s.viewer_count),
                    game: stream
                        .as_ref()
                        .and_then(|s| s.game.as_ref().and_then(|g| g.name.clone())),
                    title: stream.as_ref().and_then(|s| s.title.clone()),
                    profile_url: user.profile_image_url,
                    display_name: user.display_name,
                }
            }
            _ => ChannelStatus {
                live: false,
                viewer_count: None,
                game: None,
                title: None,
                profile_url: None,
                display_name: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_status_serialization() {
        let live_status = ChannelStatus {
            live: true,
            viewer_count: Some(1234),
            game: Some("Just Chatting".to_string()),
            title: Some("Hello world!".to_string()),
            profile_url: Some("https://example.com/image.png".to_string()),
            display_name: Some("Grimmi".to_string()),
        };

        let json = serde_json::to_string(&live_status).unwrap();
        assert!(json.contains("\"live\":true"));
        assert!(json.contains("\"viewer_count\":1234"));
        assert!(json.contains("\"game\":\"Just Chatting\""));
        assert!(json.contains("\"profile_url\""));
        assert!(json.contains("\"display_name\""));
    }

    #[test]
    fn test_offline_status_serialization() {
        let offline_status = ChannelStatus {
            live: false,
            viewer_count: None,
            game: None,
            title: None,
            profile_url: None,
            display_name: None,
        };

        let json = serde_json::to_string(&offline_status).unwrap();
        assert!(json.contains("\"live\":false"));
        assert!(!json.contains("viewer_count"));
    }

    #[test]
    fn test_gql_response_parsing() {
        let json = r#"{
            "data": {
                "user": {
                    "id": "12345",
                    "displayName": "Grimmi",
                    "profileImageURL": "https://example.com/image.png",
                    "stream": {
                        "id": "123",
                        "title": "Test Stream",
                        "viewersCount": 500,
                        "game": { "name": "Minecraft" }
                    }
                }
            }
        }"#;

        let response: GqlResponse = serde_json::from_str(json).unwrap();
        let status = response.into_channel_status();

        assert!(status.live);
        assert_eq!(status.viewer_count, Some(500));
        assert_eq!(status.game, Some("Minecraft".to_string()));
        assert_eq!(status.title, Some("Test Stream".to_string()));
        assert_eq!(status.display_name, Some("Grimmi".to_string()));
        assert!(status.profile_url.is_some());
    }

    #[test]
    fn test_gql_response_offline() {
        let json = r#"{
            "data": {
                "user": {
                    "id": "12345",
                    "displayName": "Grimmi",
                    "profileImageURL": "https://example.com/image.png",
                    "stream": null
                }
            }
        }"#;

        let response: GqlResponse = serde_json::from_str(json).unwrap();
        let status = response.into_channel_status();

        assert!(!status.live);
        assert_eq!(status.viewer_count, None);
        assert_eq!(status.display_name, Some("Grimmi".to_string()));
        assert!(status.profile_url.is_some());
    }
}
