# API contracts

This document describes the HTTP API used by the frontend. It is organized by feature area and reflects the current implementation in `src/app.rs` and related route modules.

## Type maintenance

Frontend TypeScript types are maintained manually in `web/src/lib/api-client/types.ts`. When changing backend response structs, update this document and the frontend types in the same PR.

---

## Auth

### `POST /auth/login`

Authenticate with access code.

**Request:**

```json
{
  "access_code": "string",
  "qr_token": "string?"
}
```

**Response:**

```json
{
  "authenticated": true
}
```

Sets session cookie on success.

**Errors:**
- `401 Unauthorized`: Invalid access code
- `429 Too Many Requests`: Rate limited (includes `retry_after_secs` field)

### `POST /auth/logout`

Clear session.

**Response:**

```json
{
  "authenticated": false
}
```

### `GET /auth/session`

Check current session status.

**Response:**

```json
{
  "authenticated": "bool"
}
```

### `GET /auth/qr/create`

Create a QR session for mobile login.

**Response:**

```json
{
  "token": "string",
  "expires_at": "number (unix secs)"
}
```

### `GET /auth/qr/status/{token}`

Poll QR session status.

**Response:**

```json
{
  "status": "pending | authenticated"
}
```

**Errors:**
- `404 Not Found`: Session expired or not found

### `POST /auth/qr/claim/{token}`

Claim a completed QR session.

**Response:**

```json
{
  "status": "authenticated"
}
```

Sets session cookie on success.

**Errors:**
- `404 Not Found`: Session not found
- `400 Bad Request`: QR login not yet completed

---

## App/version

### `GET /healthz`

Liveness probe (unauthenticated).

**Response:**

```json
{
  "status": "ok",
  "service": "twitch-relay"
}
```

### `GET /readyz`

Readiness probe (unauthenticated).

**Response:**

```json
{
  "status": "ready",
  "service": "twitch-relay"
}
```

### `GET /api/version`

Get application version (unauthenticated).

**Response:**

```json
{
  "version": "string"
}
```

---

## Twitch channels

Requires authentication.

### `GET /api/channels`

List configured channels.

**Response:**

```json
{
  "channels": [
    {
      "login": "string",
      "image_url": "string?",
      "display_name": "string?",
      "source": "manual | followed | both",
      "removable": "bool"
    }
  ]
}
```

### `POST /api/channels`

Add a channel.

**Request:**

```json
{
  "login": "string"
}
```

**Response:** `201 Created`

```json
{
  "login": "string"
}
```

**Errors:**
- `400 Bad Request`: Empty channel login
- `409 Conflict`: Channel already exists

### `DELETE /api/channels/{login}`

Remove a channel.

**Response:** `204 No Content`

**Errors:**
- `404 Not Found`: Channel not found

---

## Live status

Requires authentication.

### `GET /api/live-status`

Get live status for all configured channels.

**Response:**

```json
{
  "channels": {
    "channel_login": {
      "live": "bool",
      "viewer_count": "number?",
      "game": "string?",
      "title": "string?",
      "profile_url": "string?",
      "display_name": "string?"
    }
  }
}
```

---

## Twitch OAuth

Requires authentication.

### `GET /api/twitch/status`

Get Twitch account connection status.

**Response:**

```json
{
  "connected": "bool",
  "login": "string?",
  "display_name": "string?",
  "scopes": ["string"]
}
```

### `GET /api/twitch/connect`

Initiate Twitch OAuth flow.

**Response:** `302 Redirect` to Twitch authorization URL

### `GET /api/twitch/callback`

OAuth callback handler (redirected from Twitch).

**Response:** `302 Redirect` to `/`

**Errors:**
- `400 Bad Request`: Missing OAuth code/state
- `502 Bad Gateway`: OAuth callback failure

### `POST /api/twitch/disconnect`

Disconnect Twitch account.

**Response:**

```json
{
  "connected": false
}
```

---

## Recordings

Requires authentication.

### `GET /api/recordings`

List active and completed recordings.

**Response:**

```json
{
  "active": [
    {
      "channel_login": "string",
      "quality": "string",
      "started_at_unix": "number",
      "output_path": "string",
      "pid": "number?",
      "mode": "manual | auto",
      "error": "string?"
    }
  ],
  "completed": [
    {
      "channel_login": "string",
      "filename": "string",
      "path_display": "string",
      "status": "string",
      "pinned": "bool"
    }
  ],
  "incomplete": [
    /* same shape as completed */
  ]
}
```

### `POST /api/recordings/start`

Start a manual recording.

**Request:**

```json
{
  "channel_login": "string",
  "quality": "string?",
  "stream_title": "string?"
}
```

**Response:** Returns `ActiveRecording` shape

**Errors:**
- `400 Bad Request`: Invalid quality or empty channel
- `409 Conflict`: Recording already active
- `502 Bad Gateway`: Streamlink spawn failed

### `POST /api/recordings/stop`

Stop a recording.

**Request:**

```json
{
  "channel_login": "string"
}
```

**Response:** Returns `ActiveRecording` shape (or empty if none)

**Errors:**
- `404 Not Found`: No active recording for channel

### `POST /api/recordings/pin`

Pin a recording file (prevent auto-cleanup).

**Request:**

```json
{
  "bucket": "completed",
  "channel_login": "string",
  "filename": "string"
}
```

**Response:** `204 No Content`

### `POST /api/recordings/unpin`

Unpin a recording file.

**Request:** Same as pin

**Response:** `204 No Content`

### `POST /api/recordings/delete`

Delete a recording file.

**Request:**

```json
{
  "bucket": "completed | incomplete",
  "channel_login": "string",
  "filename": "string"
}
```

**Response:** `204 No Content`

**Errors:**
- `404 Not Found`: File not found
- `400 Bad Request`: Invalid filename

### `GET /api/recordings/playback-file`

Stream a recording file (MP4).

**Query:** `channel_login={login}&filename={name}`

**Response:** Video file stream

### `GET /api/recordings/hls-playlist`

Get HLS playlist for a recording.

**Query:** `channel_login={login}&filename={name}`

**Response:** HLS playlist (M3U8)

---

## Recording rules

Requires authentication.

### `GET /api/recording-rules`

List recording rules.

**Response:**

```json
{
  "rules": [
    {
      "channel_login": "string",
      "enabled": "bool",
      "quality": "string",
      "stop_when_offline": "bool",
      "max_duration_minutes": "number?",
      "keep_last_videos": "number?"
    }
  ]
}
```

### `POST /api/recording-rules`

Create or update a recording rule.

**Request:**

```json
{
  "channel_login": "string",
  "enabled": "bool",
  "quality": "string?",
  "stop_when_offline": "bool?",
  "max_duration_minutes": "number?",
  "keep_last_videos": "number?"
}
```

**Response:** Returns `RecordingRule` shape

**Errors:**
- `400 Bad Request`: Validation errors (empty login, invalid quality, etc.)

### `DELETE /api/recording-rules/{channel_login}`

Delete a recording rule.

**Response:** `204 No Content`

**Errors:**
- `404 Not Found`: Rule not found

---

## YouTube/Invidious

Requires authentication and Invidious configuration.

### `GET /api/youtube/subscriptions`

Get user's YouTube subscriptions.

**Response:**

```json
{
  "channels": [
    {
      "name": "string",
      "channel_id": "string",
      "url": "string",
      "avatar": "string?",
      "description": "string?"
    }
  ]
}
```

**Errors:**
- `502 Bad Gateway`: Invidious API error

### `GET /api/youtube/channel/{channel_id}/videos`

Get videos for a channel.

**Query:** `max_results={number}` (default: 20)

**Response:**

```json
{
  "videos": [
    {
      "title": "string",
      "video_id": "string",
      "author": "string",
      "author_id": "string",
      "published": "number",
      "published_text": "string",
      "duration": "number",
      "thumbnail": "string",
      "view_count": "number",
      "description": "string?"
    }
  ]
}
```

### `GET /api/youtube/channel/{channel_id}/info`

Get channel info with description.

**Response:**

```json
{
  "channel": {
    "name": "string",
    "channel_id": "string",
    "url": "string",
    "description": "string?",
    "description_html": "string?",
    "sub_count": "number",
    "author_verified": "bool",
    "avatar": "string?"
  }
}
```

### `GET /api/youtube/video/{video_id}/meta`

Get video metadata (for watch page).

**Response:**

```json
{
  "video": {
    "title": "string",
    "duration": "number"
  }
}
```

### `GET /api/youtube/video/{video_id}/progress`

Get stored playback progress for a video.

**Response:**

```json
{
  "video_id": "string",
  "position_secs": "number?",
  "duration_secs": "number?",
  "updated_at_unix": "number?",
  "completed": "bool",
  "invidious_sync_attempted": "bool",
  "invidious_sync_ok": "bool?",
  "invidious_sync_action": "mark_watched | mark_unwatched | none"
}
```

### `PUT /api/youtube/video/{video_id}/progress`

Update stored playback progress for a video.

**Request:**

```json
{
  "position_secs": "number",
  "duration_secs": "number?",
  "completed": "bool?"
}
```

**Response:** Same shape as `GET .../progress`

**Notes:**
- Local progress persistence is authoritative for resume position.
- Invidious watch-history sync is best-effort only and never fails this endpoint when local save succeeds.

### `GET /api/youtube/playlists`

Get user's playlists.

**Response:**

```json
{
  "playlists": [
    {
      "title": "string",
      "playlist_id": "string",
      "video_count": "number",
      "updated": "number",
      "thumbnail": "string?"
    }
  ]
}
```

### `GET /api/youtube/playlist/{playlist_id}/videos`

Get videos in a playlist.

**Response:** Same shape as channel videos

**Errors:**
- `404 Not Found`: Playlist has no videos

### `GET /api/youtube/thumbnail/{video_id}`

Proxy video thumbnail (avoids basic auth popup in browser).

**Response:** JPEG image

### `GET /api/youtube/playlist-thumbnail/{playlist_id}`

Proxy playlist thumbnail.

**Response:** JPEG image

### `GET /api/youtube/embed-config`

Get embed configuration for Invidious player.

**Response:**

```json
{
  "invidious_base_url": "string",
  "defaults": {
    "autoplay": "number",
    "quality": "string",
    "quality_dash": "string"
  },
  "referrer_policy": "string"
}
```

**Notes:**
- This endpoint requires session authentication.
- The frontend no longer receives upstream Basic Auth credentials; all
  Invidious authentication is handled server-side through relay proxy routes.

---

## Chat

Requires authentication.

### `GET /api/chat/status`

Get chat connection status for a channel.

**Query:** `channel_login={login}`

**Response:**

```json
{
  "status": {
    "subscribed": "bool",
    "connected": "bool",
    "error": "string?"
  }
}
```

### `GET /api/chat/emotes`

Get emotes for a channel (for emote picker).

**Query:** `channel_login={login}`

**Response:**

```json
{
  "emotes": [
    {
      "id": "string",
      "code": "string",
      "image_url": "string",
      "group_key": "string",
      "group_name": "string"
    }
  ]
}
```

### `POST /api/chat/subscribe`

Subscribe to channel chat.

**Request:**

```json
{
  "channel_login": "string"
}
```

**Response:** `204 No Content`

### `DELETE /api/chat/subscribe/{login}`

Unsubscribe from channel chat.

**Response:** `204 No Content`

### `POST /api/chat/send`

Send a chat message.

**Request:**

```json
{
  "channel_login": "string",
  "message": "string"
}
```

**Response:** `204 No Content`

**Errors:**
- `400 Bad Request`: Message empty, too long, or other validation error

### `GET /api/chat/events/{login}`

Server-sent events stream for chat messages.

**Response:** SSE stream

**Event payload:**

```json
{
  "kind": "message | notice",
  "channel_login": "string",
  "sender_login": "string?",
  "sender_display_name": "string?",
  "sender_color": "string?",
  "text": "string",
  "parts": [
    {
      /* ChatPart: Text or Emote */
    }
  ],
  "sent_at_unix": "number"
}
```

---

## Stream/watch

Requires authentication.

### `POST /api/watch-ticket`

Create a watch ticket for a channel.

**Request:**

```json
{
  "channel_login": "string"
}
```

**Response:**

```json
{
  "watch_url": "string"
}
```

**Errors:**
- `400 Bad Request`: Channel not in channel list

### `GET /watch/{ticket}`

Watch page for a ticket. Returns HTML.

**Query:** `relay=1` to force relay mode

**Errors:**
- `401 Unauthorized`: Invalid/expired ticket or authentication required
- `403 Forbidden`: Ticket belongs to different session

### `GET /stream/{stream_id}/{session_token}/manifest`

HLS master manifest for a stream.

**Response:** HLS playlist

### `GET /stream/{stream_id}/{session_token}/manifest/{quality}`

Quality-specific HLS manifest.

**Response:** HLS playlist

### `GET /stream/{stream_id}/{session_token}/{quality}/{*segment}`

Stream video segment.

**Response:** Video segment (may redirect to CDN)

**Errors:**
- `404 Not Found`: Stream or segment not found
- `403 Forbidden`: Session mismatch
- `302 Found`: CDN redirect (when not forcing relay)
- `502 Bad Gateway`: Failed to fetch manifest/segment

---

## Common error format

JSON error responses follow this shape:

```json
{
  "error": "string"
}
```

Rate limiting (`429`) may include:

```json
{
  "error": "too many login attempts, try again later",
  "retry_after_secs": 300
}
```
