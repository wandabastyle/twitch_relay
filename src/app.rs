use std::{
   collections::{
      HashMap,
      HashSet,
   },
   path::PathBuf,
   sync::Arc,
};

use axum::Router;
use tokio::sync::RwLock;
use tower_http::services::{
   ServeDir,
   ServeFile,
};

use crate::{
   auth::WebAuthConfig,
   channel_catalog::ChannelCatalogService,
   channels,
   chat,
   config::AppConfig,
   error::AppError,
   live_status::LiveStatusService,
   playback::PlaybackTicketService,
   prewarm::PrewarmCoordinator,
   recording::{
      RecordingProcessingConfig,
      RecordingService,
   },
   recording_scheduler::RecordingScheduler,
   routes,
   stream_proxy,
   twitch_auth,
   youtube,
};

/// State shared between channel list and watch routes.
#[derive(Debug, Clone)]
pub struct ProtectedState {
   pub auth:     WebAuthConfig,
   pub playback: PlaybackTicketService,
   pub stream:   stream_proxy::StreamSessionService,
   pub catalog:  ChannelCatalogService,
}

pub fn build_router(config: &AppConfig, access_code_hash: String) -> Result<Router, AppError> {
   let auth_config = WebAuthConfig::new(
      access_code_hash,
      config.auth.cookie_name.clone(),
      config.auth.cookie_secure,
   );

   let twitch_auth_service = twitch_auth::TwitchAuthService::new(config.twitch_oauth.clone())?;
   let catalog_service = ChannelCatalogService::new(twitch_auth_service.clone());
   let playback = PlaybackTicketService::new(config.playback.watch_ticket_ttl_secs);
   let streamlink_path = config
      .playback
      .streamlink_path
      .clone()
      .unwrap_or_else(|| "streamlink".to_string());

   let stream_service = stream_proxy::StreamSessionService::new(
      streamlink_path.clone(),
      config.playback.stream_resolver_mode,
      config.playback.stream_delivery_mode,
      config.playback.twitch_client_id.clone(),
   );

   let protected_state = ProtectedState {
      auth: auth_config.clone(),
      playback,
      stream: stream_service.clone(),
      catalog: catalog_service.clone(),
   };

   let live_status_service = LiveStatusService::new();
   let channel_state = routes::ChannelState {
      live_status: live_status_service.clone(),
   };

   let live_status_state = routes::LiveStatusState {
      service: live_status_service,
      catalog: catalog_service.clone(),
   };

   let chat_service = chat::ChatService::new(twitch_auth_service.clone());
   let chat_state = chat::ChatState {
      service: chat_service,
   };

   let prewarm = PrewarmCoordinator::new(
      catalog_service,
      live_status_state.service.clone(),
      chat_state.service.clone(),
      stream_service.clone(),
   );
   prewarm.trigger_now();

   let recording_service = RecordingService::new(
      streamlink_path,
      config.recording.recordings_dir.clone(),
      config.recording.write_nfo,
      config.recording.nfo_style,
      twitch_auth_service.clone(),
      RecordingProcessingConfig {
         ffmpeg_path:                  config.recording.ffmpeg_path.clone(),
         chapter_min_gap_secs:         config.recording.chapter_min_gap_secs,
         chapter_change_confirmations: config.recording.chapter_change_confirmations,
      },
   )
   .map_err(|e| AppError::Config(e.to_string()))?;
   RecordingScheduler::start(
      config.recording.clone(),
      live_status_state.service.clone(),
      recording_service.clone(),
   );

   let twitch_state = twitch_auth::TwitchAuthState {
      auth:    auth_config.clone(),
      twitch:  twitch_auth_service,
      prewarm: Some(prewarm),
   };

   let stream_proxy_state = stream_proxy::StreamProxyState::new(stream_service);

   let recording_state = routes::RecordingState {
      auth:                    auth_config.clone(),
      service:                 recording_service,
      default_quality:         config.recording.default_quality.clone(),
      progress:                crate::recording_progress::RecordingProgressStore::new(),
      active_processing_guard: Arc::new(RwLock::new(HashSet::new())),
      recording_jobs:          Arc::new(RwLock::new(HashMap::new())),
   };

   // Build route modules
   let channel_routes = routes::channel_routes(channel_state, auth_config.clone());
   let live_status_routes = routes::live_status_routes(live_status_state, auth_config.clone());
   let watch_routes = routes::watch_routes(protected_state, auth_config.clone());
   let recording_routes = routes::recording_routes(recording_state, auth_config.clone());
   let twitch_routes = routes::twitch_routes(twitch_state, auth_config.clone());
   let chat_routes = routes::chat_routes(chat_state, auth_config.clone());
   let stream_routes = routes::stream_routes(stream_proxy_state);
   let auth_routes = routes::auth_routes(auth_config.clone());

   let base_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

   let static_path = base_path.join("web").join("build");
   let assets_path = base_path.join("web").join("static");

   let images_path = channels::images_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
   let youtube_images_path =
      crate::youtube_channels::images_dir().unwrap_or_else(|| PathBuf::from("/tmp"));

   let youtube_routes = youtube::build_routes(auth_config, config);

   let router = Router::new()
      .merge(routes::health_routes())
      .merge(auth_routes)
      .merge(channel_routes)
      .merge(live_status_routes)
      .merge(watch_routes)
      .merge(recording_routes)
      .merge(twitch_routes)
      .merge(chat_routes)
      .merge(stream_routes)
      .merge(youtube_routes)
      .nest_service("/static/images", ServeDir::new(&images_path))
      .nest_service(
         "/static/youtube_images",
         ServeDir::new(&youtube_images_path),
      )
      .nest_service("/static", ServeDir::new(&assets_path))
      .fallback_service(
         ServeDir::new(&static_path).fallback(ServeFile::new(static_path.join("index.html"))),
      );

   Ok(router)
}

#[cfg(test)]
mod tests {
   // Recording error tests moved to src/routes/recordings.rs
}
