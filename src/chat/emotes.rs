use crate::chat::events::ChatPart;
use crate::twitch_auth::{TwitchAccount, TwitchAuthService};
use crate::util::time::now_unix_secs;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

const EMOTE_CACHE_TTL_SECS: u64 = 900;
const THIRD_PARTY_EMOTE_CACHE_TTL_SECS: u64 = 300;
const OWNER_NAME_CACHE_TTL_SECS: u64 = 24 * 60 * 60;
const OWNER_NAME_MISS_CACHE_TTL_SECS: u64 = 15 * 60;
const OWNER_LOOKUP_429_FALLBACK_COOLDOWN_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize)]
pub struct EmotePickerResponse {
    pub emotes: Vec<EmotePickerItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmotePickerItem {
    pub id: String,
    pub code: String,
    pub image_url: String,
    pub group_key: String,
    pub group_name: String,
}

#[derive(Debug, Clone)]
pub struct CachedEmoteEntry {
    pub expires_at_unix: u64,
    pub items: Vec<EmotePickerItem>,
}

#[derive(Debug, Clone)]
pub struct CachedThirdPartyEmotes {
    pub expires_at_unix: u64,
    pub by_code: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CachedOwnerName {
    pub expires_at_unix: u64,
    pub display_name: Option<String>,
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

#[derive(Debug, Deserialize)]
struct SevenTvUserResponse {
    #[serde(default)]
    emote_set: Option<SevenTvEmoteSet>,
}

#[derive(Debug, Deserialize)]
struct SevenTvEmoteSet {
    #[serde(default)]
    emotes: Vec<SevenTvEmoteItem>,
}

#[derive(Debug, Deserialize)]
struct SevenTvGlobalResponse {
    #[serde(default)]
    emotes: Vec<SevenTvEmoteItem>,
}

#[derive(Debug, Deserialize)]
struct SevenTvEmoteItem {
    name: String,
    data: SevenTvEmoteData,
}

#[derive(Debug, Deserialize)]
struct SevenTvEmoteData {
    id: String,
}

#[derive(Debug, Deserialize)]
struct BttvEmoteItem {
    id: String,
    code: String,
}

#[derive(Debug, Deserialize)]
struct BttvUserResponse {
    #[serde(default)]
    channel_emotes: Vec<BttvEmoteItem>,
    #[serde(default)]
    shared_emotes: Vec<BttvEmoteItem>,
}

pub async fn emotes_for_channel_with_account(
    emote_cache: &Arc<RwLock<HashMap<String, CachedEmoteEntry>>>,
    owner_name_cache: &Arc<RwLock<HashMap<String, CachedOwnerName>>>,
    owner_lookup_cooldown_until_unix: &Arc<AtomicU64>,
    auth: &TwitchAuthService,
    normalized_channel: &str,
    account: &TwitchAccount,
) -> Result<Vec<EmotePickerItem>, String> {
    let cache_key = format!("{}:{normalized_channel}", account.user_id);
    let now = now_unix_secs();

    {
        let cache = emote_cache.read().await;
        if let Some(entry) = cache.get(&cache_key)
            && entry.expires_at_unix > now
        {
            return Ok(entry.items.clone());
        }
    }

    let client = auth.api_client();
    let client_id = auth.client_id();
    let broadcaster = resolve_user_by_login(
        &client,
        &client_id,
        &account.access_token,
        normalized_channel,
    )
    .await?;

    let channel_emotes =
        fetch_channel_emotes(&client, &client_id, &account.access_token, &broadcaster.id).await?;
    let user_emotes = fetch_user_emotes(
        &client,
        &client_id,
        &account.access_token,
        &account.user_id,
        &broadcaster.id,
    )
    .await?;
    let allowed_user_emote_ids: HashSet<String> =
        user_emotes.iter().map(|emote| emote.id.clone()).collect();

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
        owner_name_cache,
        owner_lookup_cooldown_until_unix,
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
        if !allowed_user_emote_ids.contains(&emote.id) {
            continue;
        }
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

    let mut cache = emote_cache.write().await;
    cache.insert(
        cache_key,
        CachedEmoteEntry {
            expires_at_unix: now.saturating_add(EMOTE_CACHE_TTL_SECS),
            items: merged.clone(),
        },
    );

    Ok(merged)
}

pub async fn third_party_emotes_for_channel(
    auth: &TwitchAuthService,
    cache: &Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
    channel: &str,
) -> Result<HashMap<String, String>, String> {
    use crate::util::channel::normalize_channel_login;

    let normalized_channel = normalize_channel_login(channel)?;
    let now = now_unix_secs();
    {
        let guard = cache.read().await;
        if let Some(entry) = guard.get(&normalized_channel)
            && entry.expires_at_unix > now
        {
            return Ok(entry.by_code.clone());
        }
    }

    let client = auth.api_client();
    let mut out = HashMap::new();

    merge_third_party_emote_map(
        &mut out,
        fetch_7tv_channel_emotes(&client, &normalized_channel).await,
    );
    merge_third_party_emote_map(
        &mut out,
        fetch_bttv_channel_emotes(&client, &normalized_channel).await,
    );
    merge_third_party_emote_map(&mut out, fetch_7tv_global_emotes(&client).await);
    merge_third_party_emote_map(&mut out, fetch_bttv_global_emotes(&client).await);

    let expires_at_unix = now_unix_secs().saturating_add(THIRD_PARTY_EMOTE_CACHE_TTL_SECS);
    let mut guard = cache.write().await;
    guard.insert(
        normalized_channel,
        CachedThirdPartyEmotes {
            expires_at_unix,
            by_code: out.clone(),
        },
    );

    Ok(out)
}

pub async fn local_echo_parts_for_channel(
    emote_cache: &Arc<RwLock<HashMap<String, CachedEmoteEntry>>>,
    third_party_emote_cache: &Arc<RwLock<HashMap<String, CachedThirdPartyEmotes>>>,
    auth: &TwitchAuthService,
    channel: &str,
    message: &str,
) -> Vec<ChatPart> {
    let now = now_unix_secs();
    let key_suffix = format!(":{channel}");
    let mut emotes_by_code: HashMap<String, String> = HashMap::new();

    {
        let cache = emote_cache.read().await;
        for (key, entry) in cache.iter() {
            if !key.ends_with(&key_suffix) || entry.expires_at_unix <= now {
                continue;
            }

            for emote in &entry.items {
                emotes_by_code
                    .entry(emote.code.clone())
                    .or_insert_with(|| emote.id.clone());
            }
        }
    }

    let third_party_emotes = third_party_emotes_for_channel(auth, third_party_emote_cache, channel)
        .await
        .unwrap_or_default();

    parse_local_message_parts(message, &emotes_by_code, &third_party_emotes)
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
    owner_name_cache: &Arc<RwLock<HashMap<String, CachedOwnerName>>>,
    owner_lookup_cooldown_until_unix: &Arc<AtomicU64>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if user_ids.is_empty() {
        return out;
    }

    let filtered_ids: Vec<String> = user_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .filter(|id| id.bytes().all(|byte| byte.is_ascii_digit()))
        .collect();

    if filtered_ids.is_empty() {
        return out;
    }

    let now = now_unix_secs();
    let mut missing_ids = Vec::new();
    let mut seen_ids = HashSet::new();
    {
        let cache = owner_name_cache.read().await;
        for id in filtered_ids {
            if !seen_ids.insert(id.clone()) {
                continue;
            }
            if let Some(entry) = cache.get(&id)
                && entry.expires_at_unix > now
            {
                if let Some(display_name) = entry.display_name.as_ref() {
                    out.insert(id, display_name.clone());
                }
            } else {
                missing_ids.push(id);
            }
        }
    }

    if missing_ids.is_empty() {
        return out;
    }

    let cooldown_until = owner_lookup_cooldown_until_unix.load(Ordering::Relaxed);
    if now < cooldown_until {
        tracing::debug!(
            cooldown_until_unix = cooldown_until,
            pending_owner_ids = missing_ids.len(),
            "skipping user name lookup while rate-limited"
        );
        return out;
    }

    for chunk in missing_ids.chunks(100) {
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
            let status = response.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let reset_header = response
                    .headers()
                    .get("Ratelimit-Reset")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok());
                let fallback_cooldown_until =
                    now_unix_secs().saturating_add(OWNER_LOOKUP_429_FALLBACK_COOLDOWN_SECS);
                let cooldown_until = match reset_header {
                    Some(value) if value > now_unix_secs() => value,
                    _ => fallback_cooldown_until,
                };
                owner_lookup_cooldown_until_unix.store(cooldown_until, Ordering::Relaxed);

                let body = response.text().await.unwrap_or_default();
                let sample_ids: Vec<&str> = chunk.iter().map(String::as_str).take(5).collect();
                tracing::warn!(
                    status = %status,
                    sample_ids = ?sample_ids,
                    cooldown_until_unix = cooldown_until,
                    body = %body,
                    "resolve user names by ids rate-limited"
                );
                break;
            }

            let body = response.text().await.unwrap_or_default();
            let sample_ids: Vec<&str> = chunk.iter().map(String::as_str).take(5).collect();
            tracing::warn!(
                status = %status,
                sample_ids = ?sample_ids,
                body = %body,
                "resolve user names by ids returned non-success status"
            );
            continue;
        }

        let payload: TwitchUsersResponse = match response.json().await {
            Ok(payload) => payload,
            Err(error) => {
                tracing::warn!(error = %error, "decode user names by ids failed");
                continue;
            }
        };

        let mut fresh_names: HashMap<String, String> = HashMap::with_capacity(payload.data.len());
        for user in payload.data {
            let display_name = user
                .display_name
                .clone()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| "Unknown channel".to_string());

            out.insert(user.id.clone(), display_name.clone());
            fresh_names.insert(user.id, display_name);
        }

        let hit_expires_at_unix = now_unix_secs().saturating_add(OWNER_NAME_CACHE_TTL_SECS);
        let miss_expires_at_unix = now_unix_secs().saturating_add(OWNER_NAME_MISS_CACHE_TTL_SECS);
        let mut cache = owner_name_cache.write().await;
        for user_id in chunk {
            if let Some(display_name) = fresh_names.get(user_id) {
                cache.insert(
                    user_id.clone(),
                    CachedOwnerName {
                        expires_at_unix: hit_expires_at_unix,
                        display_name: Some(display_name.clone()),
                    },
                );
            } else {
                cache.insert(
                    user_id.clone(),
                    CachedOwnerName {
                        expires_at_unix: miss_expires_at_unix,
                        display_name: None,
                    },
                );
            }
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

fn parse_local_message_parts(
    message: &str,
    emotes_by_code: &HashMap<String, String>,
    third_party_emotes_by_code: &HashMap<String, String>,
) -> Vec<ChatPart> {
    if message.is_empty() {
        return vec![ChatPart::Text {
            text: message.to_string(),
        }];
    }

    let mut parts = Vec::new();
    let mut segment = String::new();
    let mut segment_is_whitespace: Option<bool> = None;

    let flush_segment = |value: &mut String, is_whitespace: bool, out: &mut Vec<ChatPart>| {
        if value.is_empty() {
            return;
        }

        if is_whitespace {
            out.push(ChatPart::Text {
                text: value.clone(),
            });
            value.clear();
            return;
        }

        if let Some(id) = emotes_by_code.get(value) {
            out.push(ChatPart::Emote {
                id: id.clone(),
                code: value.clone(),
                image_url: None,
            });
        } else if let Some(image_url) = third_party_emotes_by_code.get(value) {
            out.push(ChatPart::Emote {
                id: value.clone(),
                code: value.clone(),
                image_url: Some(image_url.clone()),
            });
        } else {
            out.push(ChatPart::Text {
                text: value.clone(),
            });
        }

        value.clear();
    };

    for ch in message.chars() {
        let is_whitespace = ch.is_whitespace();
        match segment_is_whitespace {
            None => {
                segment_is_whitespace = Some(is_whitespace);
                segment.push(ch);
            }
            Some(current) if current == is_whitespace => {
                segment.push(ch);
            }
            Some(current) => {
                flush_segment(&mut segment, current, &mut parts);
                segment_is_whitespace = Some(is_whitespace);
                segment.push(ch);
            }
        }
    }

    if let Some(is_whitespace) = segment_is_whitespace {
        flush_segment(&mut segment, is_whitespace, &mut parts);
    }

    if parts.is_empty() {
        vec![ChatPart::Text {
            text: message.to_string(),
        }]
    } else {
        parts
    }
}

async fn fetch_7tv_channel_emotes(
    client: &reqwest::Client,
    channel: &str,
) -> Result<Vec<(String, String)>, String> {
    let response = client
        .get(format!("https://7tv.io/v3/users/twitch/{channel}"))
        .send()
        .await
        .map_err(|e| format!("fetch 7tv channel emotes failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "fetch 7tv channel emotes failed with status {}",
            response.status()
        ));
    }

    let payload: SevenTvUserResponse = response
        .json()
        .await
        .map_err(|e| format!("decode 7tv channel emotes failed: {e}"))?;

    Ok(payload
        .emote_set
        .map(|set| set.emotes)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let code = item.name.trim().to_string();
            if code.is_empty() {
                return None;
            }
            let image_url = format!("https://cdn.7tv.app/emote/{}/2x.webp", item.data.id);
            Some((code, image_url))
        })
        .collect())
}

async fn fetch_7tv_global_emotes(
    client: &reqwest::Client,
) -> Result<Vec<(String, String)>, String> {
    let response = client
        .get("https://7tv.io/v3/emote-sets/global")
        .send()
        .await
        .map_err(|e| format!("fetch 7tv global emotes failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "fetch 7tv global emotes failed with status {}",
            response.status()
        ));
    }

    let payload: SevenTvGlobalResponse = response
        .json()
        .await
        .map_err(|e| format!("decode 7tv global emotes failed: {e}"))?;

    Ok(payload
        .emotes
        .into_iter()
        .filter_map(|item| {
            let code = item.name.trim().to_string();
            if code.is_empty() {
                return None;
            }
            let image_url = format!("https://cdn.7tv.app/emote/{}/2x.webp", item.data.id);
            Some((code, image_url))
        })
        .collect())
}

async fn fetch_bttv_channel_emotes(
    client: &reqwest::Client,
    channel: &str,
) -> Result<Vec<(String, String)>, String> {
    let response = client
        .get(format!(
            "https://api.betterttv.net/3/cached/users/twitch/{channel}"
        ))
        .send()
        .await
        .map_err(|e| format!("fetch bttv channel emotes failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "fetch bttv channel emotes failed with status {}",
            response.status()
        ));
    }

    let payload: BttvUserResponse = response
        .json()
        .await
        .map_err(|e| format!("decode bttv channel emotes failed: {e}"))?;

    let mut out = Vec::new();
    for item in payload
        .channel_emotes
        .into_iter()
        .chain(payload.shared_emotes)
    {
        let code = item.code.trim().to_string();
        if code.is_empty() {
            continue;
        }
        out.push((
            code,
            format!("https://cdn.betterttv.net/emote/{}/2x.webp", item.id),
        ));
    }

    Ok(out)
}

async fn fetch_bttv_global_emotes(
    client: &reqwest::Client,
) -> Result<Vec<(String, String)>, String> {
    let response = client
        .get("https://api.betterttv.net/3/cached/emotes/global")
        .send()
        .await
        .map_err(|e| format!("fetch bttv global emotes failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "fetch bttv global emotes failed with status {}",
            response.status()
        ));
    }

    let payload: Vec<BttvEmoteItem> = response
        .json()
        .await
        .map_err(|e| format!("decode bttv global emotes failed: {e}"))?;

    Ok(payload
        .into_iter()
        .filter_map(|item| {
            let code = item.code.trim().to_string();
            if code.is_empty() {
                return None;
            }
            Some((
                code,
                format!("https://cdn.betterttv.net/emote/{}/2x.webp", item.id),
            ))
        })
        .collect())
}

fn merge_third_party_emote_map(
    out: &mut HashMap<String, String>,
    incoming: Result<Vec<(String, String)>, String>,
) {
    match incoming {
        Ok(items) => {
            for (code, image_url) in items {
                out.entry(code).or_insert(image_url);
            }
        }
        Err(error) => {
            tracing::debug!(error = %error, "third-party emote fetch failed");
        }
    }
}
