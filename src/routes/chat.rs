use axum::{
   Router,
   middleware,
   routing::{
      delete,
      get,
      post,
   },
};

use crate::{
   auth::{
      self,
      WebAuthConfig,
   },
   chat::ChatState,
};

/// Build chat routes.
pub fn chat_routes(state: ChatState, auth_config: WebAuthConfig) -> Router {
   let metrics = Router::new()
      .route("/api/chat/metrics", get(crate::chat::metrics))
      .with_state(state.clone());
   let protected = Router::new()
      .route("/api/chat/status", get(crate::chat::status))
      .route("/api/chat/emotes", get(crate::chat::emotes))
      .route("/api/chat/subscribe", post(crate::chat::subscribe))
      .route(
         "/api/chat/subscribe/{login}",
         delete(crate::chat::unsubscribe),
      )
      .route("/api/chat/events/{login}", get(crate::chat::events))
      .route("/api/chat/send", post(crate::chat::send))
      .with_state(state)
      .layer(middleware::from_fn_with_state(
         auth_config,
         auth::require_session_middleware,
      ));

   metrics.merge(protected)
}
