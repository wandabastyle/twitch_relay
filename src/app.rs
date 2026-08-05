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
   invidious::InvidiousClient,
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
   pub auth:        WebAuthConfig,
   pub playback:    PlaybackTicketService,
   pub stream:      stream_proxy::StreamSessionService,
   pub catalog:     ChannelCatalogService,
   pub live_status: LiveStatusService,
}

/// Build a recording service with the given configuration.
fn build_recording_service(
   config: &AppConfig,
   streamlink_path: String,
   twitch_auth_service: twitch_auth::TwitchAuthService,
) -> Result<RecordingService, AppError> {
   RecordingService::new(
      streamlink_path,
      config.recording.recordings_dir.clone(),
      config.recording.write_nfo,
      config.recording.nfo_style,
      twitch_auth_service,
      RecordingProcessingConfig {
         ffmpeg_path:                  config.recording.ffmpeg_path.clone(),
         chapter_min_gap_secs:         config.recording.chapter_min_gap_secs,
         chapter_change_confirmations: config.recording.chapter_change_confirmations,
      },
   )
   .map_err(|e| AppError::Config(e.to_string()))
}

/// Context for building route modules.
struct RouteModuleContext<'a> {
   auth_config:         &'a WebAuthConfig,
   config:              &'a AppConfig,
   protected_state:     ProtectedState,
   live_status_service: LiveStatusService,
   catalog_service:     ChannelCatalogService,
   chat_state:          chat::ChatState,
   twitch_auth_service: twitch_auth::TwitchAuthService,
   stream_service:      stream_proxy::StreamSessionService,
   prewarm:             PrewarmCoordinator,
   recording_service:   RecordingService,
   channel_state:       routes::ChannelState,
}

/// Build route modules for the application.
fn build_route_modules(
   ctx: &RouteModuleContext<'_>,
) -> (
   Router,
   Router,
   Router,
   Router,
   Router,
   Router,
   Router,
   Router,
) {
   let twitch_state = twitch_auth::TwitchAuthState {
      auth:    ctx.auth_config.clone(),
      twitch:  ctx.twitch_auth_service.clone(),
      prewarm: Some(ctx.prewarm.clone()),
   };

   let stream_proxy_state = stream_proxy::StreamProxyState::new(ctx.stream_service.clone());

   let recording_state = routes::RecordingState {
      auth:                    ctx.auth_config.clone(),
      service:                 ctx.recording_service.clone(),
      default_quality:         ctx.config.recording.default_quality.clone(),
      progress:                crate::recording_progress::RecordingProgressStore::new(),
      active_processing_guard: Arc::new(RwLock::new(HashSet::new())),
      recording_jobs:          Arc::new(RwLock::new(HashMap::new())),
   };

   let live_status_state = routes::LiveStatusState {
      service: ctx.live_status_service.clone(),
      catalog: ctx.catalog_service.clone(),
   };

   let health_routes = routes::health_routes(recording_state.clone());

   (
      health_routes,
      routes::channel_routes(ctx.channel_state.clone(), ctx.auth_config.clone()),
      routes::live_status_routes(live_status_state, ctx.auth_config.clone()),
      routes::watch_routes(ctx.protected_state.clone(), ctx.auth_config.clone()),
      routes::recording_routes(recording_state, ctx.auth_config.clone()),
      routes::twitch_routes(twitch_state, ctx.auth_config.clone()),
      routes::chat_routes(ctx.chat_state.clone(), ctx.auth_config.clone()),
      routes::stream_routes(stream_proxy_state),
   )
}

/// Attach static asset and frontend fallback services to a router.
fn serve_frontend(router: Router) -> Router {
   let base_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
   let static_path = base_path.join("web").join("build");
   let assets_path = base_path.join("web").join("static");
   let images_path = channels::images_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
   let youtube_images_path =
      crate::youtube_channels::images_dir().unwrap_or_else(|| PathBuf::from("/tmp"));

   router
      .nest_service("/static/images", ServeDir::new(&images_path))
      .nest_service(
         "/static/youtube_images",
         ServeDir::new(&youtube_images_path),
      )
      .nest_service("/static", ServeDir::new(&assets_path))
      .fallback_service(
         ServeDir::new(&static_path).fallback(ServeFile::new(static_path.join("index.html"))),
      )
}

/// Build a router for the application.
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

   let live_status_service = LiveStatusService::new();

   let protected_state = ProtectedState {
      auth: auth_config.clone(),
      playback,
      stream: stream_service.clone(),
      catalog: catalog_service.clone(),
      live_status: live_status_service.clone(),
   };
   let channel_state = routes::ChannelState {
      live_status: live_status_service.clone(),
   };

   let chat_service = chat::ChatService::new(twitch_auth_service.clone());
   let chat_state = chat::ChatState {
      service: chat_service,
   };

   let youtube_client = config.invidious.as_ref().map(InvidiousClient::new);

   let prewarm = PrewarmCoordinator::new(
      catalog_service.clone(),
      live_status_service.clone(),
      chat_state.service.clone(),
      stream_service.clone(),
      youtube_client.clone(),
   );
   prewarm.trigger_now();

   let recording_service =
      build_recording_service(config, streamlink_path, twitch_auth_service.clone())?;
   RecordingScheduler::start(
      config.recording.clone(),
      live_status_service.clone(),
      recording_service.clone(),
   );

   let (
      health_routes,
      channel_routes,
      live_status_routes,
      watch_routes,
      recording_routes,
      twitch_routes,
      chat_routes,
      stream_routes,
   ) = build_route_modules(&RouteModuleContext {
      auth_config: &auth_config,
      config,
      protected_state,
      live_status_service,
      catalog_service,
      chat_state,
      twitch_auth_service,
      stream_service,
      prewarm,
      recording_service,
      channel_state,
   });

   let auth_routes = routes::auth_routes(auth_config.clone());
   let youtube_routes = youtube::build_routes_with_client(auth_config, config, youtube_client);

   let router = serve_frontend(
      Router::new()
         .merge(health_routes)
         .merge(auth_routes)
         .merge(channel_routes)
         .merge(live_status_routes)
         .merge(watch_routes)
         .merge(recording_routes)
         .merge(twitch_routes)
         .merge(chat_routes)
         .merge(stream_routes)
         .merge(youtube_routes),
   );

   Ok(router)
}

#[cfg(test)]
mod tests {
   // Recording error tests moved to src/routes/recordings.rs
}
