mod app;
mod auth;
mod channels;
mod config;
mod error;
mod live_status;
mod playback;
mod stream_proxy;

use std::process::ExitCode;

use crate::auth::PasswordState;
use crate::config::AppConfig;

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = %err, "application failed");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), error::AppError> {
    let rotate = std::env::var("TWITCH_RELAY_ROTATE_PASSWORD")
        .map(|v| v.trim().to_ascii_lowercase())
        .map(|v| v == "1" || v == "true" || v == "yes" || v == "on")
        .unwrap_or(false);

    let resolved = auth::load_or_initialize_access_code(rotate);
    let config = AppConfig::from_env()?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

    let local_addr = listener.local_addr()?;
    let app = app::build_router(&config, resolved.access_code_hash.clone())?;

    print_startup_info(local_addr, &resolved);

    tracing::info!(addr = %local_addr, "listening for requests");

    axum::serve(listener, app).await?;

    Ok(())
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
