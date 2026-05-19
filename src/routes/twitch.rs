use axum::{
   Router,
   middleware,
   routing::get,
};

use crate::{
   auth::{
      self,
      WebAuthConfig,
   },
   twitch_auth::{
      TwitchAuthState,
      callback,
      connect,
      disconnect,
      get_status,
   },
};

/// Build Twitch OAuth routes.
pub fn twitch_routes(state: TwitchAuthState, auth_config: WebAuthConfig) -> Router {
   Router::new()
      .route("/api/twitch/status", get(get_status))
      .route("/api/twitch/connect", get(connect))
      .route("/api/twitch/callback", get(callback))
      .route("/api/twitch/disconnect", post(disconnect))
      .with_state(state)
      .layer(middleware::from_fn_with_state(
         auth_config,
         auth::require_session_middleware,
      ))
}

use axum::routing::post;
