use axum::{
   Json,
   Router,
   extract::{
      Path,
      Query,
      State,
   },
   http::{
      HeaderMap,
      StatusCode,
   },
   middleware,
   response::{
      IntoResponse,
      Response,
   },
   routing::{
      get,
      post,
   },
};
use serde::{
   Deserialize,
   Serialize,
};

/// State for watch routes (shared with channels).
/// Imported from `crate::app` since `ProtectedState` is defined there.
pub use crate::app::ProtectedState;
use crate::{
   auth::{
      self,
      WebAuthConfig,
   },
   channel_catalog::CatalogChannel,
   config::{
      StreamDeliveryMode,
      StreamResolverMode,
   },
   playback::PlaybackTicketError,
   routes::{
      APP_VERSION,
      error::error_response,
   },
   stream_proxy::{
      self,
      RelayQuery,
   },
};

/// Response DTO for channel list.
#[derive(Debug, Serialize)]
pub struct ChannelsResponse {
   pub channels: Vec<ChannelItem>,
}

/// Individual channel item in the response.
#[derive(Debug, Serialize)]
pub struct ChannelItem {
   pub login:        String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub image_url:    Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub display_name: Option<String>,
   pub source:       String,
   pub removable:    bool,
}

/// Request DTO for creating a watch ticket.
#[derive(Debug, Deserialize)]
pub struct WatchTicketRequest {
   pub channel_login: String,
}

/// Response DTO for watch ticket creation.
#[derive(Debug, Serialize)]
pub struct WatchTicketResponse {
   pub watch_url: String,
}

/// Response DTO for watch session bootstrap data.
#[derive(Debug, Serialize)]
pub struct WatchSessionResponse {
   pub channel:       String,
   pub manifest_url:  String,
   pub relay:         bool,
   pub app_version:   &'static str,
   pub display_name:  Option<String>,
   pub profile_url:   Option<String>,
   pub title:         Option<String>,
   pub game:          Option<String>,
   pub viewer_count:  Option<u64>,
   pub live:          bool,
   pub resolver:      String,
   pub delivery_mode: String,
}

/// Build watch routes.
pub fn watch_routes(state: ProtectedState, auth_config: WebAuthConfig) -> Router {
   Router::new()
      .route("/api/channels", get(list_channels))
      .route("/api/watch-ticket", post(create_watch_ticket))
      .route("/api/watch-session/{ticket}", get(watch_session_handler))
      .with_state(state)
      .layer(middleware::from_fn_with_state(
         auth_config,
         auth::require_session_middleware,
      ))
}

async fn list_channels(State(state): State<ProtectedState>) -> Json<ChannelsResponse> {
   let mut channels_list: Vec<ChannelItem> = state
      .catalog
      .list_channels()
      .await
      .into_iter()
      .map(channel_item_from_catalog)
      .collect();

   channels_list.sort_by_key(|c| c.login.to_lowercase());

   Json(ChannelsResponse {
      channels: channels_list,
   })
}

fn channel_item_from_catalog(item: CatalogChannel) -> ChannelItem {
   let source = match item.source {
      crate::channel_catalog::ChannelSource::Manual => "manual",
      crate::channel_catalog::ChannelSource::Followed => "followed",
      crate::channel_catalog::ChannelSource::Both => "both",
   };

   ChannelItem {
      login:        item.login,
      image_url:    item.image_url,
      display_name: item.display_name,
      source:       source.to_string(),
      removable:    item.removable,
   }
}

async fn create_watch_ticket(
   State(state): State<ProtectedState>,
   headers: HeaderMap,
   Json(payload): Json<WatchTicketRequest>,
) -> Response {
   if !state.catalog.has_channel(&payload.channel_login).await {
      return error_response(
         StatusCode::BAD_REQUEST,
         "channel is not in channel list",
         None,
      );
   }

   let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
      return error_response(StatusCode::UNAUTHORIZED, "authentication required", None);
   };

   state
      .playback
      .issue_ticket(&session_token, &payload.channel_login)
      .map_or_else(
         |_| {
            error_response(
               StatusCode::INTERNAL_SERVER_ERROR,
               "failed to issue watch ticket",
               None,
            )
         },
         |ticket| {
            let response = WatchTicketResponse {
               watch_url: format!("/watch/{ticket}"),
            };
            (StatusCode::OK, Json(response)).into_response()
         },
      )
}

async fn watch_session_handler(
   State(state): State<ProtectedState>,
   headers: HeaderMap,
   Path(ticket): Path<String>,
   Query(query): Query<RelayQuery>,
) -> Response {
   let Some(session_token) = state.auth.session_token_from_headers(&headers) else {
      return error_response(StatusCode::UNAUTHORIZED, "authentication required", None);
   };

   let validated = match state.playback.validate_ticket(&ticket, &session_token) {
      Ok(v) => v,
      Err(PlaybackTicketError::InvalidTicket | PlaybackTicketError::ExpiredTicket) => {
         return error_response(
            StatusCode::UNAUTHORIZED,
            "invalid or expired watch ticket",
            None,
         );
      },
      Err(PlaybackTicketError::SessionMismatch) => {
         return error_response(
            StatusCode::FORBIDDEN,
            "watch ticket belongs to a different session",
            None,
         );
      },
   };

   if let Err(e) = state
      .stream
      .open_session(&ticket, &validated.channel_login, &session_token, "best")
      .await
   {
      return match e {
         stream_proxy::StreamError::HlsFetchFailed(msg) => {
            tracing::error!(error = %msg, channel = %validated.channel_login, "failed to open stream session");
            error_response(
               StatusCode::BAD_GATEWAY,
               "stream unavailable. the channel may be offline or not accessible",
               None,
            )
         },
         _ => {
            error_response(
               StatusCode::INTERNAL_SERVER_ERROR,
               "failed to open stream session",
               None,
            )
         },
      };
   }

    let force_relay = query.force_relay();
    let relay_suffix = if force_relay { "?relay=1" } else { "" };
    let channel_login = validated.channel_login;
    let status = state.live_status.check_multiple(&[channel_login.clone()]).await;
    let channel_status = status
       .channels
       .get(&channel_login.to_ascii_lowercase())
       .cloned()
       .unwrap_or(crate::live_status::ChannelStatus {
          live:         false,
          viewer_count: None,
          game:         None,
          title:        None,
          profile_url:  None,
          display_name: None,
       });
    let response = WatchSessionResponse {
       channel:      channel_login,
       manifest_url: format!("/stream/{ticket}/{session_token}/manifest{relay_suffix}"),
       relay:        force_relay,
       app_version:  APP_VERSION,
       display_name: channel_status.display_name,
       profile_url:  channel_status.profile_url,
       title:        channel_status.title,
       game:         channel_status.game,
       viewer_count: channel_status.viewer_count,
       live:         channel_status.live,
       resolver:     resolver_label(state.stream.resolver_mode()),
       delivery_mode: delivery_label(state.stream.delivery_mode()),
    };

    (StatusCode::OK, Json(response)).into_response()
}

fn resolver_label(mode: StreamResolverMode) -> String {
   match mode {
      StreamResolverMode::Auto => "auto".to_string(),
      StreamResolverMode::Native => "native".to_string(),
      StreamResolverMode::Streamlink => "streamlink".to_string(),
   }
}

fn delivery_label(mode: StreamDeliveryMode) -> String {
   match mode {
      StreamDeliveryMode::CdnFirst => "cdn_first".to_string(),
      StreamDeliveryMode::Relay => "relay".to_string(),
   }
}
