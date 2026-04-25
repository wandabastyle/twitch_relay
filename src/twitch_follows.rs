use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct FollowedChannel {
    pub login: String,
    pub display_name: Option<String>,
    pub profile_image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FollowedChannelsResponse {
    data: Vec<FollowedChannelItem>,
    #[serde(default)]
    pagination: Pagination,
}

#[derive(Debug, Deserialize, Default)]
struct Pagination {
    #[serde(default)]
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FollowedChannelItem {
    broadcaster_login: String,
    broadcaster_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsersResponse {
    data: Vec<UserItem>,
}

#[derive(Debug, Deserialize)]
struct UserItem {
    login: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    profile_image_url: Option<String>,
}

pub async fn fetch_followed_channels(
    client: &Client,
    client_id: &str,
    access_token: &str,
    user_id: &str,
) -> Result<Vec<FollowedChannel>, String> {
    let mut cursor: Option<String> = None;
    let mut base_items = Vec::new();

    loop {
        let mut req = client
            .get("https://api.twitch.tv/helix/channels/followed")
            .header("Client-Id", client_id)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[("user_id", user_id), ("first", "100")]);

        if let Some(cursor_value) = cursor.as_ref() {
            req = req.query(&[("after", cursor_value)]);
        }

        let response = req
            .send()
            .await
            .map_err(|e| format!("followed channels request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "followed channels request failed with status {}",
                response.status()
            ));
        }

        let payload: FollowedChannelsResponse = response
            .json()
            .await
            .map_err(|e| format!("followed channels decode failed: {e}"))?;

        base_items.extend(payload.data);

        let Some(next) = payload.pagination.cursor else {
            break;
        };
        if next.trim().is_empty() {
            break;
        }
        cursor = Some(next);
    }

    if base_items.is_empty() {
        return Ok(Vec::new());
    }

    let logins: Vec<String> = base_items
        .iter()
        .map(|item| item.broadcaster_login.trim().to_ascii_lowercase())
        .filter(|login| !login.is_empty())
        .collect();

    let users_response = client
        .get("https://api.twitch.tv/helix/users")
        .header("Client-Id", client_id)
        .header("Authorization", format!("Bearer {access_token}"))
        .query(&logins.iter().map(|login| ("login", login)).collect::<Vec<_>>())
        .send()
        .await
        .map_err(|e| format!("users lookup failed: {e}"))?;

    if !users_response.status().is_success() {
        return Err(format!(
            "users lookup failed with status {}",
            users_response.status()
        ));
    }

    let users_payload: UsersResponse = users_response
        .json()
        .await
        .map_err(|e| format!("users lookup decode failed: {e}"))?;

    let mut users_by_login = std::collections::HashMap::new();
    for user in users_payload.data {
        users_by_login.insert(user.login.to_ascii_lowercase(), user);
    }

    let followed = base_items
        .into_iter()
        .filter_map(|item| {
            let login = item.broadcaster_login.trim().to_ascii_lowercase();
            if login.is_empty() {
                return None;
            }

            let detail = users_by_login.get(&login);
            let display_name = detail
                .and_then(|user| user.display_name.clone())
                .or(item.broadcaster_name);
            let profile_image_url = detail.and_then(|user| user.profile_image_url.clone());

            Some(FollowedChannel {
                login,
                display_name,
                profile_image_url,
            })
        })
        .collect();

    Ok(followed)
}
