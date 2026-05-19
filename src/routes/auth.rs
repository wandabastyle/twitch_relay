use axum::{
   Router,
   routing::get,
};

use crate::auth::{
   self,
   WebAuthConfig,
};

/// Build auth routes.
pub fn auth_routes(auth_config: WebAuthConfig) -> Router {
   Router::new()
      .route("/auth/login", post(auth::login))
      .route("/auth/logout", post(auth::logout))
      .route("/auth/session", get(auth::session_status))
      .route("/auth/qr/create", get(auth::create_qr_session))
      .route("/auth/qr/status/{token}", get(auth::qr_status))
      .route("/auth/qr/claim/{token}", post(auth::qr_claim))
      .with_state(auth_config)
}

use axum::routing::post;
