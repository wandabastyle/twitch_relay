use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::{Rng, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};

use crate::{config::AppConfig, error::AppError};

#[derive(Debug, Clone)]
pub struct WebAuthConfig {
    access_code_hash: String,
    cookie_name: String,
    cookie_secure: bool,
    session_ttl_secs: u64,
    login_window_secs: u64,
    max_login_attempts: u32,
    login_block_secs: u64,
    sessions: Arc<RwLock<HashMap<String, u64>>>,
    login_attempts: Arc<RwLock<HashMap<String, LoginAttemptState>>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct LoginAttemptState {
    window_start: u64,
    attempts: u32,
    blocked_until: u64,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub access_code: String,
}

#[derive(Debug, Serialize)]
pub struct SessionStateResponse {
    pub authenticated: bool,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_secs: Option<u64>,
}

impl WebAuthConfig {
    pub fn from_app_config(config: &AppConfig) -> Result<Self, AppError> {
        let access_code_hash = hash_access_code(&config.auth.access_code)?;

        Ok(Self {
            access_code_hash,
            cookie_name: config.auth.cookie_name.clone(),
            cookie_secure: config.auth.cookie_secure,
            session_ttl_secs: config.auth.session_ttl_secs,
            login_window_secs: config.auth.login_window_secs,
            max_login_attempts: config.auth.max_login_attempts,
            login_block_secs: config.auth.login_block_secs,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            login_attempts: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn create_session(&self) -> Option<String> {
        let expires_at = now_unix_secs().saturating_add(self.session_ttl_secs);
        let token = generate_token(48);
        let mut guard = self.sessions.write().ok()?;
        guard.insert(token.clone(), expires_at);
        Some(token)
    }

    fn validate_headers(&self, headers: &HeaderMap) -> bool {
        let Some(token) = cookie_value(headers, &self.cookie_name) else {
            return false;
        };

        let now = now_unix_secs();
        let mut guard = match self.sessions.write() {
            Ok(guard) => guard,
            Err(_) => return false,
        };

        guard.retain(|_, expires| *expires > now);

        matches!(guard.get(token), Some(expires) if *expires > now)
    }

    fn revoke_from_headers(&self, headers: &HeaderMap) -> bool {
        let Some(token) = cookie_value(headers, &self.cookie_name) else {
            return false;
        };

        if let Ok(mut guard) = self.sessions.write() {
            return guard.remove(token).is_some();
        }

        false
    }

    fn check_login_allowed(&self, key: &str) -> Result<(), u64> {
        let now = now_unix_secs();
        let mut guard = match self.login_attempts.write() {
            Ok(guard) => guard,
            Err(_) => return Err(self.login_block_secs),
        };

        guard.retain(|_, state| {
            state.blocked_until > now
                || now.saturating_sub(state.window_start) <= self.login_window_secs
        });

        let state = guard.entry(key.to_string()).or_default();

        if state.blocked_until > now {
            return Err(state.blocked_until.saturating_sub(now));
        }

        if now.saturating_sub(state.window_start) > self.login_window_secs {
            state.window_start = now;
            state.attempts = 0;
            state.blocked_until = 0;
        }

        if state.attempts >= self.max_login_attempts {
            state.blocked_until = now.saturating_add(self.login_block_secs);
            return Err(self.login_block_secs);
        }

        Ok(())
    }

    fn record_login_failure(&self, key: &str) {
        let now = now_unix_secs();
        if let Ok(mut guard) = self.login_attempts.write() {
            let state = guard.entry(key.to_string()).or_default();

            if state.window_start == 0
                || now.saturating_sub(state.window_start) > self.login_window_secs
            {
                state.window_start = now;
                state.attempts = 0;
                state.blocked_until = 0;
            }

            state.attempts = state.attempts.saturating_add(1);
            if state.attempts >= self.max_login_attempts {
                state.blocked_until = now.saturating_add(self.login_block_secs);
            }
        }
    }

    fn record_login_success(&self, key: &str) {
        if let Ok(mut guard) = self.login_attempts.write() {
            guard.remove(key);
        }
    }

    fn build_cookie(&self, name: &str, value: &str, max_age: Option<u64>) -> String {
        let mut cookie = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax");

        if let Some(max_age) = max_age {
            cookie.push_str(&format!("; Max-Age={max_age}"));
        }

        if self.cookie_secure {
            cookie.push_str("; Secure");
        }

        cookie
    }
}

pub async fn login(
    State(config): State<WebAuthConfig>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Response {
    let login_key = login_attempt_key(&headers);

    if let Err(retry_after_secs) = config.check_login_allowed(&login_key) {
        tracing::warn!(client = %login_key, retry_after_secs, "auth login blocked");
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "too many login attempts, try again later",
            Some(retry_after_secs),
        );
    }

    let valid = verify_access_code(&payload.access_code, &config.access_code_hash).unwrap_or(false);
    if !valid {
        config.record_login_failure(&login_key);
        tracing::warn!(client = %login_key, "auth login failed");
        return error_response(StatusCode::UNAUTHORIZED, "invalid access code", None);
    }

    config.record_login_success(&login_key);

    let Some(token) = config.create_session() else {
        tracing::error!("failed to create session");
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create session",
            None,
        );
    };

    tracing::info!(client = %login_key, "auth login succeeded");

    let cookie = config.build_cookie(&config.cookie_name, &token, None);
    let mut response = (
        StatusCode::OK,
        Json(SessionStateResponse {
            authenticated: true,
        }),
    )
        .into_response();

    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }

    response
}

pub async fn logout(State(config): State<WebAuthConfig>, request: Request) -> Response {
    let had_session = config.revoke_from_headers(request.headers());
    let login_key = login_attempt_key(request.headers());

    if had_session {
        tracing::info!(client = %login_key, "auth logout succeeded");
    } else {
        tracing::info!(client = %login_key, "auth logout without active session");
    }

    let clear_cookie = config.build_cookie(&config.cookie_name, "", Some(0));
    let mut response = (
        StatusCode::OK,
        Json(SessionStateResponse {
            authenticated: false,
        }),
    )
        .into_response();

    if let Ok(value) = HeaderValue::from_str(&clear_cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }

    response
}

pub async fn session_status(State(config): State<WebAuthConfig>, request: Request) -> Response {
    let authenticated = config.validate_headers(request.headers());
    (StatusCode::OK, Json(SessionStateResponse { authenticated })).into_response()
}

pub async fn require_session_middleware(
    State(config): State<WebAuthConfig>,
    request: Request,
    next: Next,
) -> Response {
    if !config.validate_headers(request.headers()) {
        let login_key = login_attempt_key(request.headers());
        tracing::warn!(client = %login_key, "auth guard denied request");
        return error_response(StatusCode::UNAUTHORIZED, "authentication required", None);
    }

    next.run(request).await
}

fn hash_access_code(access_code: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);

    Argon2::default()
        .hash_password(access_code.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::Config(format!("access code hash failed: {err}")))
}

fn verify_access_code(access_code: &str, access_code_hash: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(access_code_hash)
        .map_err(|err| AppError::Config(format!("access code hash parse failed: {err}")))?;

    Ok(Argon2::default()
        .verify_password(access_code.as_bytes(), &parsed)
        .is_ok())
}

fn cookie_value<'a>(headers: &'a HeaderMap, cookie_name: &str) -> Option<&'a str> {
    let raw_cookie = headers.get(header::COOKIE)?.to_str().ok()?;

    raw_cookie.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if name == cookie_name {
            Some(value)
        } else {
            None
        }
    })
}

fn login_attempt_key(headers: &HeaderMap) -> String {
    if let Some(forwarded) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(',').next())
    {
        let key = forwarded.trim();
        if !key.is_empty() {
            return key.to_string();
        }
    }

    if let Some(real_ip) = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
    {
        let key = real_ip.trim();
        if !key.is_empty() {
            return key.to_string();
        }
    }

    "unknown-client".to_string()
}

fn generate_token(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn error_response(status: StatusCode, message: &str, retry_after_secs: Option<u64>) -> Response {
    let mut response = (
        status,
        Json(ErrorResponse {
            error: message.to_string(),
            retry_after_secs,
        }),
    )
        .into_response();

    if let Some(seconds) = retry_after_secs
        && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
    {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }

    response
}
