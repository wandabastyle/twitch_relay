use axum::{Router, routing::get};

use crate::stream_proxy::{
    StreamProxyState, proxy_manifest, proxy_segment, proxy_variant_manifest,
};

/// Build stream proxy routes.
pub fn stream_routes(state: StreamProxyState) -> Router {
    Router::new()
        .route(
            "/stream/{stream_id}/{session_token}/manifest",
            get(proxy_manifest),
        )
        .route(
            "/stream/{stream_id}/{session_token}/manifest/{quality}",
            get(proxy_variant_manifest),
        )
        .route(
            "/stream/{stream_id}/{session_token}/{quality}/{*segment}",
            get(proxy_segment),
        )
        .with_state(state)
}
