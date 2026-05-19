use std::sync::OnceLock;

use axum::{
   Json,
   extract::{
      Path,
      State,
   },
   http::{
      HeaderMap,
      HeaderValue,
      StatusCode,
   },
   response::{
      IntoResponse,
      Response,
   },
};
use quick_xml::{
   Reader,
   Writer,
   events::Event as XmlEvent,
};
use regex::{
   Captures,
   Regex,
};
use serde::{
   Deserialize,
   Serialize,
};

use crate::{
   error::AppError,
   invidious::is_valid_video_id,
   youtube::YoutubeState,
};

/// Frontend embed configuration
#[derive(Debug, Serialize)]
pub struct EmbedConfigResponse {
   pub invidious_base_url: String,
   pub defaults:           EmbedDefaults,
   pub referrer_policy:    String,
}

#[derive(Debug, Serialize)]
pub struct EmbedDefaults {
   pub autoplay:     u8,
   pub quality:      String,
   pub quality_dash: String,
}

/// Get frontend embed configuration.
pub async fn get_embed_config(State(state): State<YoutubeState>) -> Response {
   if let Err(e) = state.require_client() {
      return e.into_response();
   }

   let Some(base_url) = state.invidious_base_url() else {
      return AppError::InvidiousNotConfigured.into_response();
   };

   (
      StatusCode::OK,
      Json(EmbedConfigResponse {
         invidious_base_url: base_url.to_string(),
         defaults:           EmbedDefaults {
            autoplay:     1,
            quality:      "dash".to_string(),
            quality_dash: "auto".to_string(),
         },
         referrer_policy:    "no-referrer".to_string(),
      }),
   )
      .into_response()
}

/// Query parameters for embed proxy
#[derive(Debug, Deserialize)]
pub struct EmbedQuery {
   autoplay:     Option<String>,
   quality:      Option<String>,
   quality_dash: Option<String>,
   start:        Option<String>,
   t:            Option<String>,
}

static HTML_ATTR_DOUBLE_RE: OnceLock<Regex> = OnceLock::new();
static HTML_ATTR_SINGLE_RE: OnceLock<Regex> = OnceLock::new();
static STATIC_ROOT_DOUBLE_RE: OnceLock<Regex> = OnceLock::new();
static STATIC_ROOT_SINGLE_RE: OnceLock<Regex> = OnceLock::new();

const COMPANION_PROXY_PREFIX: &str = "/api/youtube/companion/api/";
const VIDEO_PROXY_PREFIX: &str = "/api/youtube/proxy/";
const LATEST_VERSION_PROXY_PATH: &str = "/api/youtube/latest_version";
const STATIC_PROXY_PREFIX: &str = "/api/youtube/static/";

/// Rewrite root-relative URLs in HTML to absolute URLs.
/// Matches src="/...", href="/...", poster="/..." and prepends the base URL.
/// Excludes protocol-relative URLs (starting with //) and fragment-only URLs.
fn rewrite_html_urls(html: &str, base_url: &str) -> String {
   let base = base_url.trim_end_matches('/');

   let re_double = HTML_ATTR_DOUBLE_RE.get_or_init(|| {
      Regex::new(r#"(src|href|poster)="(/[^"/][^"]*?)""#)
         .expect("valid double-quoted HTML attr regex")
   });

   let _re_single = HTML_ATTR_SINGLE_RE.get_or_init(|| {
      // Match '/path' where path is not empty and not starting with //
      Regex::new(r"(src|href|poster)='(/[^'/][^']*?)'")
         .expect("valid single-quoted HTML attr regex")
   });

   let re_single = HTML_ATTR_SINGLE_RE.get_or_init(|| {
      // Match '/path' where path is not empty and not starting with //
      Regex::new(r"(src|href|poster)='(/[^'/][^']*?)'")
         .expect("valid single-quoted HTML attr regex")
   });

   let html = re_double.replace_all(html, |caps: &Captures| {
      let attr = &caps[1];
      let path = &caps[2];
      format!(r#"{attr}="{base}{path}""#)
   });

   let html = re_single
      .replace_all(&html, |caps: &Captures| {
         let attr = &caps[1];
         let path = &caps[2];
         format!("{attr}='{base}{path}'")
      })
      .into_owned();

   rewrite_companion_api_urls(&html, base)
}

fn rewrite_companion_api_urls(html: &str, base_url: &str) -> String {
   let mut rewritten = html.to_string();

   let absolute_prefix = format!("{base_url}/companion/api/");
   rewritten = rewritten.replace(&absolute_prefix, COMPANION_PROXY_PREFIX);
   let absolute_latest_version = format!("{base_url}/companion/latest_version");
   rewritten = rewritten.replace(&absolute_latest_version, LATEST_VERSION_PROXY_PATH);
   rewritten = rewritten.replace("/companion/latest_version", LATEST_VERSION_PROXY_PATH);
   let absolute_videojs = format!("{base_url}/videojs/");
   rewritten = rewritten.replace(&absolute_videojs, &format!("{STATIC_PROXY_PREFIX}videojs/"));
   let absolute_css = format!("{base_url}/css/");
   rewritten = rewritten.replace(&absolute_css, &format!("{STATIC_PROXY_PREFIX}css/"));
   let absolute_js = format!("{base_url}/js/");
   rewritten = rewritten.replace(&absolute_js, &format!("{STATIC_PROXY_PREFIX}js/"));
   let absolute_vi = format!("{base_url}/vi/");
   rewritten = rewritten.replace(&absolute_vi, &format!("{STATIC_PROXY_PREFIX}vi/"));

   // Replace double-quoted companion API paths
   rewritten = rewritten.replace("\"/companion/api/", &format!("\"{COMPANION_PROXY_PREFIX}"));

   let rewritten = STATIC_ROOT_DOUBLE_RE
      .get_or_init(|| {
         Regex::new(r#"\"/(videojs|css|js|vi)/"#).expect("valid double-quoted static root regex")
      })
      .replace_all(&rewritten, format!("\"{STATIC_PROXY_PREFIX}$1/"))
      .into_owned();

   // Replace single-quoted companion API paths
   let rewritten = rewritten.replace("'/companion/api/", &format!("'{COMPANION_PROXY_PREFIX}"));

   STATIC_ROOT_SINGLE_RE
      .get_or_init(|| {
         Regex::new(r"'/(videojs|css|js|vi)/").expect("valid single-quoted static root regex")
      })
      .replace_all(&rewritten, format!("'{STATIC_PROXY_PREFIX}$1/"))
      .into_owned()
}

pub fn rewrite_dash_manifest(manifest_xml: &str) -> Result<String, AppError> {
   let mut reader = Reader::from_str(manifest_xml);
   reader.config_mut().trim_text(false);
   let mut writer = Writer::new(Vec::with_capacity(manifest_xml.len()));
   let mut in_base_url = false;

   loop {
      match reader.read_event() {
         Ok(XmlEvent::Start(e)) => {
            in_base_url = e.name().as_ref() == b"BaseURL";
            writer
               .write_event(XmlEvent::Start(e.into_owned()))
               .map_err(|_| AppError::InvidiousBadResponse)?;
         },
         Ok(XmlEvent::End(e)) => {
            if e.name().as_ref() == b"BaseURL" {
               in_base_url = false;
            }
            writer
               .write_event(XmlEvent::End(e.into_owned()))
               .map_err(|_| AppError::InvidiousBadResponse)?;
         },
         Ok(XmlEvent::Text(e)) => {
            if in_base_url {
               let original = e
                  .decode()
                  .map_err(|_| AppError::InvidiousBadResponse)?
                  .into_owned();
               let rewritten = original
                  .strip_prefix("/companion/")
                  .map(|rest| format!("{VIDEO_PROXY_PREFIX}{rest}"))
                  .unwrap_or(original);
               writer
                  .write_event(XmlEvent::Text(quick_xml::events::BytesText::new(
                     &rewritten,
                  )))
                  .map_err(|_| AppError::InvidiousBadResponse)?;
            } else {
               writer
                  .write_event(XmlEvent::Text(e.into_owned()))
                  .map_err(|_| AppError::InvidiousBadResponse)?;
            }
         },
         Ok(XmlEvent::Eof) => break,
         Ok(e) => {
            writer
               .write_event(e.into_owned())
               .map_err(|_| AppError::InvidiousBadResponse)?;
         },
         Err(_) => return Err(AppError::InvidiousBadResponse),
      }
   }

   String::from_utf8(writer.into_inner()).map_err(|_| AppError::InvidiousBadResponse)
}

fn inject_quality_indicator_script(html: &str, video_id: &str) -> String {
   let mut script = r"<script>(function(){const videoId='__VIDEO_ID__';const observedEndpoint=`/api/youtube/video/${encodeURIComponent(videoId)}/quality-observed`;const streamEndpoint=`/api/youtube/video/${encodeURIComponent(videoId)}/quality-stream`;const badgeId='relay-quality-indicator';let pollTimer=null;let pollDelayMs=2000;let eventSource=null;let sseRetryTimer=null;function ensureBadge(){const controlBar=document.querySelector('.vjs-control-bar');if(!controlBar)return null;let badge=document.getElementById(badgeId);if(!badge){badge=document.createElement('div');badge.id=badgeId;badge.style.marginLeft='auto';badge.style.padding='0 0.65rem';badge.style.display='flex';badge.style.alignItems='center';badge.style.color='#fff';badge.style.fontWeight='normal';badge.style.fontStyle='normal';badge.style.fontFamily='Arial, Helvetica, sans-serif';badge.style.wordBreak='initial';badge.style.cursor='none';badge.style.visibility='visible';badge.style.wordWrap='break-word';badge.style.textAlign='center';badge.style.fontSize='1em';badge.style.lineHeight='3em';badge.style.boxSizing='inherit';badge.style.whiteSpace='nowrap';controlBar.appendChild(badge);}return badge;}function setBadgeText(data){const badge=ensureBadge();if(!badge)return;if(data&&data.current_quality_label){badge.textContent=`Quality: ${data.current_quality_label}`;}else{badge.textContent='Quality: detecting...';}}async function refreshObserved(){try{const res=await fetch(observedEndpoint,{credentials:'same-origin'});if(!res.ok){setBadgeText(null);return;}const data=await res.json();setBadgeText(data);}catch(_){setBadgeText(null);}}function stopPolling(){if(pollTimer!==null){window.clearTimeout(pollTimer);pollTimer=null;}}function schedulePolling(){stopPolling();pollTimer=window.setTimeout(async()=>{await refreshObserved();pollDelayMs=Math.min(Math.round(pollDelayMs*1.5),10000);schedulePolling();},pollDelayMs);}function startPollingFallback(){if(pollTimer!==null)return;pollDelayMs=2000;schedulePolling();}function stopSseRetry(){if(sseRetryTimer!==null){window.clearTimeout(sseRetryTimer);sseRetryTimer=null;}}function scheduleSseReconnect(){if(sseRetryTimer!==null)return;sseRetryTimer=window.setTimeout(()=>{sseRetryTimer=null;startSse();},5000);}function startSse(){if(eventSource!==null)return;try{eventSource=new EventSource(streamEndpoint);}catch(_){startPollingFallback();scheduleSseReconnect();return;}eventSource.onmessage=(event)=>{if(!event||typeof event.data!=='string')return;try{const data=JSON.parse(event.data);setBadgeText(data);stopPolling();pollDelayMs=2000;}catch(_){}};eventSource.onerror=()=>{if(eventSource!==null){eventSource.close();eventSource=null;}startPollingFallback();scheduleSseReconnect();};}function shutdown(){stopPolling();stopSseRetry();if(eventSource!==null){eventSource.close();eventSource=null;}}function boot(){refreshObserved();startSse();window.addEventListener('beforeunload',shutdown,{once:true});}if(document.readyState==='loading'){document.addEventListener('DOMContentLoaded',boot,{once:true});}else{boot();}})();</script>".to_string();
   script = script.replace("__VIDEO_ID__", video_id);

   if let Some(idx) = html.rfind("</body>") {
      let mut out = String::with_capacity(html.len() + script.len());
      out.push_str(&html[..idx]);
      out.push_str(&script);
      out.push_str(&html[idx..]);
      return out;
   }

   format!("{html}{script}")
}

/// Proxy embed requests to avoid basic auth popup in browser.
/// Fetches the Invidious embed page with backend authentication.
pub async fn get_embed(
   State(state): State<YoutubeState>,
   Path(video_id): Path<String>,
   query: axum::extract::Query<EmbedQuery>,
) -> Response {
   // Validate video_id format
   if !is_valid_video_id(&video_id) {
      return (StatusCode::BAD_REQUEST, "invalid video_id format").into_response();
   }

   let Some(base_url) = state.invidious_base_url() else {
      return AppError::InvidiousNotConfigured.into_response();
   };

   let Some(client) = state.invidious_client() else {
      return AppError::InvidiousNotConfigured.into_response();
   };

   // Build upstream Invidious embed URL with whitelisted query parameters
   let mut upstream_url = format!("{base_url}/embed/{video_id}");
   let mut params = Vec::new();

   if let Some(autoplay) = &query.autoplay {
      params.push(format!("autoplay={}", urlencoding::encode(autoplay)));
   }
   if let Some(quality) = &query.quality {
      params.push(format!("quality={}", urlencoding::encode(quality)));
   }
   if let Some(quality_dash) = &query.quality_dash {
      params.push(format!(
         "quality_dash={}",
         urlencoding::encode(quality_dash)
      ));
   }
   if let Some(start) = &query.start {
      params.push(format!("start={}", urlencoding::encode(start)));
   }
   if let Some(t) = &query.t {
      params.push(format!("t={}", urlencoding::encode(t)));
   }

   if !params.is_empty() {
      upstream_url.push('?');
      upstream_url.push_str(&params.join("&"));
   }

   // Log request (without credentials)
   tracing::debug!(video_id = %video_id, "Proxying embed request to Invidious");

   // Fetch embed page through InvidiousClient (handles Basic auth + SID cookie)
   let response = match client
      .with_basic_auth(client.http.get(&upstream_url))
      .send()
      .await
   {
      Ok(r) => r,
      Err(e) => {
         tracing::error!(error = %e, video_id = %video_id, "Failed to fetch embed from Invidious");
         return (
            StatusCode::BAD_GATEWAY,
            "Failed to fetch embed from Invidious",
         )
            .into_response();
      },
   };

   // Handle upstream response status codes
   let status = response.status();

   if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
      tracing::warn!(
          status = %status,
          video_id = %video_id,
          "Invidious embed upstream authentication failed"
      );
      return (
         StatusCode::BAD_GATEWAY,
         "Invidious embed upstream authentication failed",
      )
         .into_response();
   }

   if status == StatusCode::NOT_FOUND {
      return (StatusCode::NOT_FOUND, "Embed not found").into_response();
   }

   if !status.is_success() {
      tracing::warn!(
          status = %status,
          video_id = %video_id,
          "Invidious returned error for embed"
      );
      return (
         StatusCode::BAD_GATEWAY,
         "Failed to fetch embed from Invidious",
      )
         .into_response();
   }

   // Get content type from response, default to text/html
   let content_type = response
      .headers()
      .get("content-type")
      .and_then(|v| v.to_str().ok()).map_or_else(|| "text/html; charset=utf-8".to_string(), std::string::ToString::to_string);

   // Get response body as string for URL rewriting
   let body_bytes = match response.bytes().await {
      Ok(b) => b,
      Err(e) => {
         tracing::error!(error = %e, video_id = %video_id, "Failed to read embed response");
         return (StatusCode::BAD_GATEWAY, "Failed to read embed response").into_response();
      },
   };

   // Convert to string for URL rewriting
   let html = match String::from_utf8(body_bytes.to_vec()) {
      Ok(s) => s,
      Err(e) => {
         tracing::error!(error = %e, video_id = %video_id, "Embed response is not valid UTF-8");
         // Return original bytes if we can't parse as UTF-8
         let mut headers = HeaderMap::new();
         headers.insert(
            "content-type",
            HeaderValue::from_static("text/html; charset=utf-8"),
         );
         headers.insert(
            "cache-control",
            HeaderValue::from_static("no-store, no-cache, must-revalidate"),
         );
         return (headers, body_bytes).into_response();
      },
   };

   // Rewrite root-relative URLs to absolute URLs
   let rewritten_html = rewrite_html_urls(&html, base_url);
   let rewritten_html = inject_quality_indicator_script(&rewritten_html, &video_id);
   let rewritten_bytes = rewritten_html.into_bytes();

   // Build response with appropriate headers
   let mut headers = HeaderMap::new();
   headers.insert(
      "content-type",
      HeaderValue::from_str(&content_type)
         .unwrap_or_else(|_| HeaderValue::from_static("text/html; charset=utf-8")),
   );
   // No cache headers since embed may depend on session/auth state
   headers.insert(
      "cache-control",
      HeaderValue::from_static("no-store, no-cache, must-revalidate"),
   );

   (headers, rewritten_bytes).into_response()
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn rewrite_html_urls_double_quoted_root_relative() {
      let input = r#"<script src="/player.js"></script>"#;
      let expected = r#"<script src="https://inv.example.com/player.js"></script>"#;
      assert_eq!(
         rewrite_html_urls(input, "https://inv.example.com"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_single_quoted_root_relative() {
      let input = "<script src='/embed.js'></script>";
      let expected = "<script src='https://inv.example.com/embed.js'></script>";
      assert_eq!(
         rewrite_html_urls(input, "https://inv.example.com"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_preserves_query_strings() {
      let input = r#"<script src="/player.js?v=123"></script>"#;
      let expected = r#"<script src="https://inv.example.com/player.js?v=123"></script>"#;
      assert_eq!(
         rewrite_html_urls(input, "https://inv.example.com"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_does_not_touch_absolute_urls() {
      let input = r#"<script src="https://cdn.example.com/player.js"></script>"#;
      assert_eq!(rewrite_html_urls(input, "https://inv.example.com"), input);
   }

   #[test]
   fn rewrite_html_urls_does_not_touch_protocol_relative_urls() {
      let input = r#"<script src="//cdn.example.com/player.js"></script>"#;
      assert_eq!(rewrite_html_urls(input, "https://inv.example.com"), input);
   }

   #[test]
   fn rewrite_html_urls_does_not_touch_fragment_only_urls() {
      let input = "<a href=\"#settings\">Settings</a>";
      assert_eq!(rewrite_html_urls(input, "https://inv.example.com"), input);
   }

   #[test]
   fn rewrite_html_urls_does_not_touch_data_urls() {
      let input = r#"<img src="data:image/png;base64,abc">"#;
      assert_eq!(rewrite_html_urls(input, "https://inv.example.com"), input);
   }

   #[test]
   fn rewrite_html_urls_handles_multiple_attributes() {
      let input =
         r#"<link href="/css/default.css" rel="stylesheet"><script src="/player.js"></script>"#;
      let expected = r#"<link href="/api/youtube/static/css/default.css" rel="stylesheet"><script src="https://inv.example.com/player.js"></script>"#;
      assert_eq!(
         rewrite_html_urls(input, "https://inv.example.com"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_handles_poster_attribute() {
      let input = r#"<video poster="/vi/example/maxres.jpg"></video>"#;
      let expected = r#"<video poster="/api/youtube/static/vi/example/maxres.jpg"></video>"#;
      assert_eq!(
         rewrite_html_urls(input, "https://inv.example.com"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_handles_trailing_slash_in_base() {
      let input = r#"<script src="/player.js"></script>"#;
      let expected = r#"<script src="https://inv.example.com/player.js"></script>"#;
      assert_eq!(
         rewrite_html_urls(input, "https://inv.example.com/"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_rewrites_absolute_companion_api_url() {
      let input = "https://inv.wandabanet.de/companion/api/manifest/dash/id/VIDEO_ID?local=true";
      let expected = "/api/youtube/companion/api/manifest/dash/id/VIDEO_ID?local=true";
      assert_eq!(
         rewrite_html_urls(input, "https://inv.wandabanet.de"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_rewrites_root_relative_companion_api_url() {
      let input = "\"/companion/api/manifest/dash/id/VIDEO_ID?local=true\"";
      let expected = "\"/api/youtube/companion/api/manifest/dash/id/VIDEO_ID?local=true\"";
      assert_eq!(
         rewrite_html_urls(input, "https://inv.wandabanet.de"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_static_root_relative_asset_still_rewrites_to_absolute() {
      let input = r#"<script src="/player.js"></script>"#;
      let expected = r#"<script src="https://inv.wandabanet.de/player.js"></script>"#;
      assert_eq!(
         rewrite_html_urls(input, "https://inv.wandabanet.de"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_unrelated_absolute_url_unchanged() {
      let input = r#"<script src="https://cdn.example.com/player.js"></script>"#;
      assert_eq!(rewrite_html_urls(input, "https://inv.wandabanet.de"), input);
   }

   #[test]
   fn rewrite_html_urls_rewrites_static_asset_absolute_urls() {
      let input = r#"<script src="https://inv.wandabanet.de/videojs/player.js"></script><link href="https://inv.wandabanet.de/css/default.css" rel="stylesheet"><script src="https://inv.wandabanet.de/js/embed.js"></script><img src="https://inv.wandabanet.de/vi/vYy4em2fQ8Q/maxres.jpg">"#;
      let expected = r#"<script src="/api/youtube/static/videojs/player.js"></script><link href="/api/youtube/static/css/default.css" rel="stylesheet"><script src="/api/youtube/static/js/embed.js"></script><img src="/api/youtube/static/vi/vYy4em2fQ8Q/maxres.jpg">"#;
      assert_eq!(
         rewrite_html_urls(input, "https://inv.wandabanet.de"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_rewrites_static_asset_root_relative_urls() {
      let input =
         r#""/videojs/player.js" '/css/default.css' "/js/embed.js" "/vi/vYy4em2fQ8Q/maxres.jpg""#;
      let expected = r#""/api/youtube/static/videojs/player.js" '/api/youtube/static/css/default.css' "/api/youtube/static/js/embed.js" "/api/youtube/static/vi/vYy4em2fQ8Q/maxres.jpg""#;
      assert_eq!(
         rewrite_html_urls(input, "https://inv.wandabanet.de"),
         expected
      );
   }

   #[test]
   fn rewrite_dash_manifest_rewrites_base_url_companion_path() {
      let input = r#"<?xml version="1.0"?><MPD><Period><BaseURL>/companion/videoplayback?foo=bar&amp;x=1</BaseURL></Period></MPD>"#;
      let output = rewrite_dash_manifest(input).expect("manifest rewrite should succeed");
      assert!(
         output.contains("<BaseURL>/api/youtube/proxy/videoplayback?foo=bar&amp;x=1</BaseURL>")
      );
   }

   #[test]
   fn rewrite_dash_manifest_keeps_non_companion_base_url_unchanged() {
      let input = r#"<?xml version="1.0"?><MPD><Period><BaseURL>https://cdn.example.com/file.mp4</BaseURL></Period></MPD>"#;
      let output = rewrite_dash_manifest(input).expect("manifest rewrite should succeed");
      assert!(output.contains("<BaseURL>https://cdn.example.com/file.mp4</BaseURL>"));
   }

   #[test]
   fn rewrite_html_urls_rewrites_absolute_latest_version_url() {
      let input = "https://inv.wandabanet.de/companion/latest_version?id=vYy4em2fQ8Q&itag=18";
      let expected = "/api/youtube/latest_version?id=vYy4em2fQ8Q&itag=18";
      assert_eq!(
         rewrite_html_urls(input, "https://inv.wandabanet.de"),
         expected
      );
   }

   #[test]
   fn rewrite_html_urls_rewrites_root_relative_latest_version_url() {
      let input = "\"/companion/latest_version?id=vYy4em2fQ8Q&itag=18\"";
      let expected = "\"/api/youtube/latest_version?id=vYy4em2fQ8Q&itag=18\"";
      assert_eq!(
         rewrite_html_urls(input, "https://inv.wandabanet.de"),
         expected
      );
   }

   #[test]
   fn inject_quality_indicator_script_inserts_before_body_end() {
      let input = "<html><body><div>player</div></body></html>";
      let output = inject_quality_indicator_script(input, "vYy4em2fQ8Q");
      assert!(output.contains("relay-quality-indicator"));
      assert!(
         output.contains("/api/youtube/video/${encodeURIComponent(videoId)}/quality-observed")
      );
      assert!(output.contains("</body></html>"));
   }
}
