mod app;
mod auth;
mod channel_catalog;
mod chat;
mod channels;
mod config;
mod error;
mod live_status;
mod playback;
mod secure_store;
mod stream_proxy;
mod twitch_auth;
mod twitch_follows;

use std::process::ExitCode;

use crate::auth::PasswordState;
use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Standard,
    Dev,
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    let mode = parse_run_mode();

    match run(mode).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if mode == RunMode::Dev {
                let message = err.to_string();
                if message.contains("missing required env var") {
                    tracing::error!(
                        error = %err,
                        "application failed (dev mode tip: ensure .env contains all TWITCH_OAUTH_* and TWITCH_TOKEN_ENCRYPTION_KEY values)"
                    );
                } else {
                    tracing::error!(error = %err, "application failed");
                }
            } else {
                tracing::error!(error = %err, "application failed");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run(mode: RunMode) -> Result<(), error::AppError> {
    if mode == RunMode::Dev {
        let _ = dotenvy::dotenv();
    }

    let rotate = std::env::var("TWITCH_RELAY_ROTATE_PASSWORD")
        .map(|v| v.trim().to_ascii_lowercase())
        .map(|v| v == "1" || v == "true" || v == "yes" || v == "on")
        .unwrap_or(false);

    let resolved = auth::load_or_initialize_access_code(rotate);
    let config = AppConfig::from_env()?;

    if mode == RunMode::Dev {
        print_dev_info(&config);
    }

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

    let local_addr = listener.local_addr()?;
    let app = app::build_router(&config, resolved.access_code_hash.clone())?;

    print_startup_info(local_addr, &resolved);

    tracing::info!(addr = %local_addr, "listening for requests");

    axum::serve(listener, app).await?;

    Ok(())
}

fn parse_run_mode() -> RunMode {
    match std::env::args().nth(1).as_deref() {
        Some("dev") => RunMode::Dev,
        _ => RunMode::Standard,
    }
}

fn print_dev_info(config: &AppConfig) {
    println!("dev mode enabled (.env loaded if present)");
    println!("twitch oauth redirect: {}", config.twitch_oauth.redirect_uri);

    if let Some(path) = auth::stored_auth_path()
        && let Some(parent) = path.parent()
    {
        println!("data directory: {}", parent.display());
    }
}

fn print_startup_info(local_addr: std::net::SocketAddr, resolved: &auth::ResolvedAccessCode) {
    println!("twitch-relay listening on {local_addr}");

    match resolved.state {
        PasswordState::Loaded => println!("auth enabled (loaded saved access code)"),
        PasswordState::GeneratedPersisted => {
            println!("auth enabled (generated and saved new access code)")
        }
        PasswordState::GeneratedEphemeral => {
            println!("auth enabled (generated access code but could not save)")
        }
    }

    if let Some(access_code) = &resolved.one_time_access_code {
        println!("access code: {access_code}");
    }

    if let Some(path) = auth::stored_auth_path() {
        println!("auth file: {}", path.display());
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
