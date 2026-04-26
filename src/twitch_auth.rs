use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use rand::{Rng, distributions::Alphanumeric};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    auth::WebAuthConfig,
    config::TwitchOAuthConfig,
    error::AppError,
    prewarm::PrewarmCoordinator,
    secure_store::{SecureStore, twitch_account_store_path},
    twitch_follows::{self, FollowedChannel},
};

const REQUIRED_FOLLOW_SCOPE: &str = "user:read:follows";
const REQUIRED_CHAT_READ_SCOPE: &str = "chat:read";
const REQUIRED_CHAT_EDIT_SCOPE: &str = "chat:edit";
const REQUIRED_USER_EMOTES_SCOPE: &str = "user:read:emotes";

#[derive(Debug, Clone)]
pub struct TwitchAuthService {
    oauth: TwitchOAuthConfig,
    store: SecureStore,
    client: Client,
    account: Arc<RwLock<Option<TwitchAccount>>>,
    pending_states: Arc<RwLock<HashMap<String, PendingState>>>,
}

#[derive(Debug, Clone)]
pub struct TwitchAuthState {
    pub auth: WebAuthConfig,
    pub twitch: TwitchAuthService,
    pub prewarm: Option<PrewarmCoordinator>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TwitchStatusResponse {
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchAccount {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at_unix: u64,
    pub scopes: Vec<String>,
    pub user_id: String,
    pub login: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
struct PendingState {
    state: String,
    expires_at_unix: u64,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    #[serde(default)]
    scope: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TwitchUsersResponse {
    data: Vec<TwitchUser>,
}

#[derive(Debug, Deserialize)]
struct TwitchUser {
    id: String,
    login: String,
    display_name: String,
}

impl TwitchAuthService {
    pub fn new(oauth: TwitchOAuthConfig) -> Result<Self, AppError> {
        let store = SecureStore::new(&oauth.token_encryption_key)?;
        let path = twitch_account_store_path().ok_or_else(|| {
            AppError::Config("unable to resolve twitch account storage path".to_string())
        })?;
        let account = store.load_json::<TwitchAccount>(&path).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "failed to load twitch account from secure store");
            None
        });

        Ok(Self {
            oauth,
            store,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .build()
                .unwrap_or_else(|_| Client::new()),
            account: Arc::new(RwLock::new(account)),
            pending_states: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn status(&self) -> TwitchStatusResponse {
        let guard = self.account.read().await;
        if let Some(account) = guard.as_ref() {
            return TwitchStatusResponse {
                connected: true,
                login: Some(account.login.clone()),
                display_name: Some(account.display_name.clone()),
                scopes: account.scopes.clone(),
            };
        }

        TwitchStatusResponse {
            connected: false,
            login: None,
            display_name: None,
            scopes: Vec::new(),
        }
    }

    pub async fn disconnect(&self) -> Result<(), String> {
        let path = twitch_account_store_path().ok_or("unable to resolve twitch account path")?;
        self.store.delete(&path)?;
        let mut guard = self.account.write().await;
        *guard = None;
        Ok(())
    }

    pub async fn build_connect_url(&self, session_token: &str) -> String {
        let state = generate_state(42);
        let expires_at_unix = now_unix_secs().saturating_add(300);

        {
            let mut pending = self.pending_states.write().await;
            pending.retain(|_, value| value.expires_at_unix > now_unix_secs());
            pending.insert(
                session_token.to_string(),
                PendingState {
                    state: state.clone(),
                    expires_at_unix,
                },
            );
        }

        let scopes = [
            REQUIRED_FOLLOW_SCOPE,
            REQUIRED_CHAT_READ_SCOPE,
            REQUIRED_CHAT_EDIT_SCOPE,
            REQUIRED_USER_EMOTES_SCOPE,
        ]
        .join(" ");

        let mut url = reqwest::Url::parse("https://id.twitch.tv/oauth2/authorize")
            .expect("static oauth url should parse");
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.oauth.client_id)
            .append_pair("redirect_uri", &self.oauth.redirect_uri)
            .append_pair("scope", &scopes)
            .append_pair("state", &state)
            .append_pair("force_verify", "false");
        url.to_string()
    }

    pub async fn complete_callback(
        &self,
        session_token: &str,
        code: &str,
        state: &str,
    ) -> Result<(), String> {
        self.validate_state(session_token, state).await?;

        let token = self.exchange_code(code).await?;
        let user = self.fetch_user(&token.access_token).await?;
        let account = TwitchAccount {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_at_unix: now_unix_secs().saturating_add(token.expires_in),
            scopes: token.scope,
            user_id: user.id,
            login: user.login,
            display_name: user.display_name,
        };

        self.save_account(account).await
    }

    pub async fn ensure_followed_channels(&self) -> Result<Vec<FollowedChannel>, String> {
        let account = self
            .ensure_valid_account_with_scopes(&[REQUIRED_FOLLOW_SCOPE])
            .await?;
        twitch_follows::fetch_followed_channels(
            &self.client,
            &self.oauth.client_id,
            &account.access_token,
            &account.user_id,
        )
        .await
    }

    pub async fn ensure_chat_account(&self) -> Result<TwitchAccount, String> {
        self.ensure_valid_account_with_scopes(&[REQUIRED_CHAT_READ_SCOPE, REQUIRED_CHAT_EDIT_SCOPE])
            .await
    }

    pub async fn ensure_emote_account(&self) -> Result<TwitchAccount, String> {
        self.ensure_valid_account_with_scopes(&[REQUIRED_USER_EMOTES_SCOPE])
            .await
    }

    pub fn api_client(&self) -> Client {
        self.client.clone()
    }

    pub fn client_id(&self) -> String {
        self.oauth.client_id.clone()
    }

    async fn ensure_valid_account_with_scopes(
        &self,
        required_scopes: &[&str],
    ) -> Result<TwitchAccount, String> {
        let current = {
            let guard = self.account.read().await;
            guard.clone()
        }
        .ok_or("twitch account is not connected")?;

        validate_scopes(&current.scopes, required_scopes)?;

        let now = now_unix_secs();
        if current.expires_at_unix > now.saturating_add(60) {
            return Ok(current);
        }

        let refreshed = self.refresh_token(&current.refresh_token).await?;
        let user = self.fetch_user(&refreshed.access_token).await?;
        let updated = TwitchAccount {
            access_token: refreshed.access_token,
            refresh_token: refreshed.refresh_token,
            expires_at_unix: now.saturating_add(refreshed.expires_in),
            scopes: refreshed.scope,
            user_id: user.id,
            login: user.login,
            display_name: user.display_name,
        };

        self.save_account(updated.clone()).await?;
        Ok(updated)
    }

    async fn validate_state(&self, session_token: &str, state: &str) -> Result<(), String> {
        let mut guard = self.pending_states.write().await;
        guard.retain(|_, pending| pending.expires_at_unix > now_unix_secs());
        let Some(expected) = guard.remove(session_token) else {
            return Err("oauth state was not initialized for this session".to_string());
        };
        if expected.state != state {
            return Err("oauth state mismatch".to_string());
        }
        Ok(())
    }

    async fn save_account(&self, account: TwitchAccount) -> Result<(), String> {
        let path = twitch_account_store_path().ok_or("unable to resolve twitch account path")?;
        self.store.save_json(&path, &account)?;
        let mut guard = self.account.write().await;
        *guard = Some(account);
        Ok(())
    }

    async fn exchange_code(&self, code: &str) -> Result<OAuthTokenResponse, String> {
        let response = self
            .client
            .post("https://id.twitch.tv/oauth2/token")
            .form(&[
                ("client_id", self.oauth.client_id.as_str()),
                ("client_secret", self.oauth.client_secret.as_str()),
                ("code", code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", self.oauth.redirect_uri.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("oauth token exchange failed: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("oauth token exchange failed with status {status}: {body}"));
        }

        response
            .json::<OAuthTokenResponse>()
            .await
            .map_err(|e| format!("oauth token decode failed: {e}"))
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokenResponse, String> {
        let response = self
            .client
            .post("https://id.twitch.tv/oauth2/token")
            .form(&[
                ("client_id", self.oauth.client_id.as_str()),
                ("client_secret", self.oauth.client_secret.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await
            .map_err(|e| format!("oauth token refresh failed: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("oauth token refresh failed with status {status}: {body}"));
        }

        response
            .json::<OAuthTokenResponse>()
            .await
            .map_err(|e| format!("oauth refresh decode failed: {e}"))
    }

    async fn fetch_user(&self, access_token: &str) -> Result<TwitchUser, String> {
        let response = self
            .client
            .get("https://api.twitch.tv/helix/users")
            .header("Client-Id", &self.oauth.client_id)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| format!("fetch user request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "fetch user request failed with status {}",
                response.status()
            ));
        }

        let payload: TwitchUsersResponse = response
            .json()
            .await
            .map_err(|e| format!("fetch user decode failed: {e}"))?;

        payload
            .data
            .into_iter()
            .next()
            .ok_or("missing user data in twitch response".to_string())
    }
}

pub async fn get_status(State(state): State<TwitchAuthState>) -> Json<TwitchStatusResponse> {
    Json(state.twitch.status().await)
}

pub async fn connect(State(state): State<TwitchAuthState>, headers: HeaderMap) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    let url = state.twitch.build_connect_url(&session_token).await;
    Redirect::temporary(&url).into_response()
}

pub async fn callback(
    State(state): State<TwitchAuthState>,
    headers: HeaderMap,
    Query(query): Query<OAuthCallbackQuery>,
) -> Response {
    let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "authentication required");
    };

    if let Some(error) = query.error {
        tracing::warn!(error = %error, description = ?query.error_description, "twitch oauth callback returned error");
        return Redirect::temporary("/").into_response();
    }

    let Some(code) = query.code else {
        return error_response(StatusCode::BAD_REQUEST, "missing oauth code");
    };
    let Some(callback_state) = query.state else {
        return error_response(StatusCode::BAD_REQUEST, "missing oauth state");
    };

    match state
        .twitch
        .complete_callback(&session_token, &code, &callback_state)
        .await
    {
        Ok(()) => {
            if let Some(prewarm) = state.prewarm.as_ref() {
                prewarm.trigger_now();
            }
            Redirect::temporary("/").into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed completing twitch oauth callback");
            error_response(StatusCode::BAD_GATEWAY, "failed to complete twitch oauth callback")
        }
    }
}

pub async fn disconnect(State(state): State<TwitchAuthState>) -> Response {
    match state.twitch.disconnect().await {
        Ok(()) => {
            if let Some(prewarm) = state.prewarm.as_ref() {
                prewarm.trigger_now();
            }

            Json(TwitchStatusResponse {
                connected: false,
                login: None,
                display_name: None,
                scopes: Vec::new(),
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to disconnect twitch account");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to disconnect twitch account")
        }
    }
}

fn validate_scopes(scopes: &[String], required_scopes: &[&str]) -> Result<(), String> {
    let available: HashSet<String> = scopes.iter().map(|scope| scope.to_string()).collect();
    for required in required_scopes {
        if !available.contains(*required) {
            return Err(format!("missing required twitch scope: {required}"));
        }
    }
    Ok(())
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn generate_state(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
