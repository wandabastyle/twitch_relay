use axum::{
   extract::{
      Path,
      Query,
      State,
   },
   http::{
      StatusCode,
      header,
   },
   response::{
      IntoResponse,
      Response,
   },
};

use crate::{
   routes::error::error_response,
   stream_proxy::{
      RelayQuery,
      StreamError,
      StreamProxyState,
   },
};

/// Handler for master playlist (multi-level manifest).
pub async fn proxy_manifest(
   State(state): State<StreamProxyState>,
   Path((stream_id, session_token)): Path<(String, String)>,
   Query(query): Query<RelayQuery>,
) -> Response {
   let force_relay = query.force_relay();
   match state
      .service
      .get_multi_level_manifest(&stream_id, &session_token, force_relay)
      .await
   {
      Ok(body) => {
         (
            StatusCode::OK,
            [
               (
                  axum::http::header::CONTENT_TYPE,
                  axum::http::HeaderValue::from_static("application/vnd.apple.mpegurl"),
               ),
               (
                  axum::http::header::CACHE_CONTROL,
                  axum::http::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
               ),
            ],
            body,
         )
            .into_response()
      },
      Err(StreamError::StreamNotFound) => {
         error_response(StatusCode::NOT_FOUND, "stream not found or has ended", None)
      },
      Err(StreamError::SessionMismatch) => {
         error_response(
            StatusCode::FORBIDDEN,
            "stream belongs to a different session",
            None,
         )
      },
      Err(StreamError::HlsFetchFailed(msg)) => {
         tracing::error!(error = %msg, stream_id = %stream_id, "failed to fetch HLS manifest");
         error_response(
            StatusCode::BAD_GATEWAY,
            "failed to fetch stream manifest",
            None,
         )
      },
   }
}

/// Handler for quality variant manifest.
pub async fn proxy_variant_manifest(
   State(state): State<StreamProxyState>,
   Path((stream_id, session_token, quality)): Path<(String, String, String)>,
   Query(query): Query<RelayQuery>,
) -> Response {
   let force_relay = query.force_relay();
   match state
      .service
      .get_variant_manifest(&stream_id, &session_token, &quality, force_relay)
      .await
   {
      Ok(body) => {
         (
            StatusCode::OK,
            [
               (
                  axum::http::header::CONTENT_TYPE,
                  axum::http::HeaderValue::from_static("application/vnd.apple.mpegurl"),
               ),
               (
                  axum::http::header::CACHE_CONTROL,
                  axum::http::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
               ),
            ],
            body,
         )
            .into_response()
      },
      Err(StreamError::StreamNotFound) => {
         error_response(StatusCode::NOT_FOUND, "quality not found", None)
      },
      Err(StreamError::SessionMismatch) => {
         error_response(
            StatusCode::FORBIDDEN,
            "stream belongs to a different session",
            None,
         )
      },
      Err(StreamError::HlsFetchFailed(msg)) => {
         tracing::error!(error = %msg, stream_id = %stream_id, "failed to fetch variant manifest");
         error_response(
            StatusCode::BAD_GATEWAY,
            "failed to fetch variant manifest",
            None,
         )
      },
   }
}

/// Handler for TS segments.
pub async fn proxy_segment(
   State(state): State<StreamProxyState>,
   Path((stream_id, session_token, quality, segment)): Path<(String, String, String, String)>,
   Query(query): Query<RelayQuery>,
) -> Response {
   let force_relay = query.force_relay();
   match state
      .service
      .resolve_segment(&stream_id, &quality, &segment, &session_token)
      .await
   {
      Ok((cdn_url, resolver)) => {
         if state.service.should_redirect_to_cdn(force_relay) {
            if state
               .service
               .mark_delivery_logged_once(&stream_id, &quality, "cdn_redirect")
               .await
            {
               tracing::info!(
                   delivery = "cdn_redirect",
                   resolver = ?resolver,
                   force_relay = force_relay,
                   stream_id = %stream_id,
                   quality = %quality,
                   "serving stream via CDN redirect"
               );
            }
            return (StatusCode::FOUND, [(header::LOCATION, cdn_url)]).into_response();
         }

         let body = match super::resolver::fetch_bytes(&cdn_url).await {
            Ok(bytes) => bytes,
            Err(msg) => {
               tracing::error!(
                   error = %msg,
                   stream_id = %stream_id,
                   segment = %segment,
                   delivery = "relay_bytes",
                   resolver = ?resolver,
                   "failed to fetch stream segment"
               );
               return error_response(
                  StatusCode::BAD_GATEWAY,
                  "failed to fetch stream segment",
                  None,
               );
            },
         };

         if state
            .service
            .mark_delivery_logged_once(&stream_id, &quality, "relay_bytes")
            .await
         {
            tracing::info!(
                delivery = "relay_bytes",
                resolver = ?resolver,
                force_relay = force_relay,
                fallback_reason = if force_relay { "hls_fatal_retry" } else { "delivery_mode_relay" },
                stream_id = %stream_id,
                quality = %quality,
                "serving stream via relay"
            );
         }

         let ct = if segment.ends_with(".ts") || segment.contains(".ts?") {
            "video/mp2t"
         } else if segment.ends_with(".m4s") {
            "video/mp4"
         } else {
            "application/octet-stream"
         };

         (
            StatusCode::OK,
            [
               (
                  axum::http::header::CONTENT_TYPE,
                  axum::http::HeaderValue::from_str(ct).unwrap_or_else(|_| {
                     axum::http::HeaderValue::from_static("application/octet-stream")
                  }),
               ),
               (
                  axum::http::header::CACHE_CONTROL,
                  axum::http::HeaderValue::from_static("public, max-age=3600"),
               ),
               (
                  axum::http::header::ACCEPT_RANGES,
                  axum::http::HeaderValue::from_static("bytes"),
               ),
            ],
            body,
         )
            .into_response()
      },
      Err(StreamError::StreamNotFound) => {
         error_response(StatusCode::NOT_FOUND, "segment not found", None)
      },
      Err(StreamError::SessionMismatch) => {
         error_response(
            StatusCode::FORBIDDEN,
            "stream belongs to a different session",
            None,
         )
      },
      Err(StreamError::HlsFetchFailed(msg)) => {
         tracing::error!(error = %msg, stream_id = %stream_id, segment = %segment, "failed to resolve stream segment URL");
         error_response(
            StatusCode::BAD_GATEWAY,
            "failed to fetch stream segment",
            None,
         )
      },
   }
}
