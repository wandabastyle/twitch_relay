use std::collections::HashMap;

use serde::Serialize;

use crate::{channels, twitch_auth::TwitchAuthService};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelSource {
    Manual,
    Followed,
    Both,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogChannel {
    pub login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub source: ChannelSource,
    pub removable: bool,
}

#[derive(Debug, Clone)]
pub struct ChannelCatalogService {
    twitch: TwitchAuthService,
}

impl ChannelCatalogService {
    pub fn new(twitch: TwitchAuthService) -> Self {
        Self { twitch }
    }

    pub async fn list_channels(&self) -> Vec<CatalogChannel> {
        let mut merged: HashMap<String, CatalogChannel> = HashMap::new();

        let manual = channels::load_stored_channels();
        for item in manual {
            let login = item.login.trim().to_ascii_lowercase();
            if login.is_empty() {
                continue;
            }

            let image_url = item
                .image_filename
                .as_ref()
                .map(|file| format!("/static/images/{file}"));

            merged.insert(
                login.clone(),
                CatalogChannel {
                    login,
                    image_url,
                    display_name: None,
                    source: ChannelSource::Manual,
                    removable: true,
                },
            );
        }

        match self.twitch.ensure_followed_channels().await {
            Ok(followed) => {
                for item in followed {
                    let login = item.login.trim().to_ascii_lowercase();
                    if login.is_empty() {
                        continue;
                    }

                    if let Some(existing) = merged.get_mut(&login) {
                        if existing.image_url.is_none() {
                            existing.image_url = item.profile_image_url.clone();
                        }
                        if existing.display_name.is_none() {
                            existing.display_name = item.display_name.clone();
                        }
                        existing.source = ChannelSource::Both;
                        existing.removable = true;
                        continue;
                    }

                    merged.insert(
                        login.clone(),
                        CatalogChannel {
                            login,
                            image_url: item.profile_image_url,
                            display_name: item.display_name,
                            source: ChannelSource::Followed,
                            removable: false,
                        },
                    );
                }
            }
            Err(error) => {
                tracing::debug!(error = %error, "skipping followed channels merge");
            }
        }

        let mut out: Vec<CatalogChannel> = merged.into_values().collect();
        out.sort_by(|a, b| a.login.cmp(&b.login));
        out
    }

    pub async fn has_channel(&self, login: &str) -> bool {
        let normalized = login.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return false;
        }

        self.list_channels()
            .await
            .iter()
            .any(|channel| channel.login == normalized)
    }

    pub async fn channel_logins(&self) -> Vec<String> {
        self.list_channels()
            .await
            .into_iter()
            .map(|channel| channel.login)
            .collect()
    }
}
