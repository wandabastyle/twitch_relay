mod app;
mod auth;
mod config;
mod error;
mod playback;
mod relay;
mod stream_proxy;

use std::process::ExitCode;

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
    let config = AppConfig::from_env()?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

    tracing::info!(addr = %listener.local_addr()?, "listening for requests");

    let app = app::build_router(&config)?;
    axum::serve(listener, app).await?;

    Ok(())
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
