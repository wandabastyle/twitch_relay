# Architecture - twitch-relay MVP

## Overview

`twitch-relay` is a self-hosted web app for trusted users.

- Rust backend is the control plane for auth, channel state, and watch authorization.
- Svelte frontend is the UI layer for login, channel list, and player view.
- Twitch integration is live-status polling plus official embed playback for MVP.

The system is intentionally small: one backend process, one frontend build, file/env config, and in-memory state.

## Core Components

### 1) Frontend (`web/`)

- SvelteKit + TypeScript + pnpm.
- Pages:
  - `/login`
  - `/` dashboard (channel cards + live/offline state)
  - `/watch/:ticket` player page
- Calls backend auth/API endpoints and never stores sensitive secrets.

### 2) Rust Backend (`src/`)

- `axum` HTTP server, `tokio` runtime.
- Responsibilities:
  - authentication/session management
  - Twitch Helix API polling
  - channel state cache
  - watch ticket issuance/validation
  - static asset serving for frontend build

### 3) Twitch Service

- Uses Helix API with app token.
- Poll interval default: 30 seconds.
- Tracks allowlisted channels only.
- Produces normalized channel state consumed by API/UI.

## Request/Playback Flow

1. User opens `/`.
2. Frontend checks `GET /auth/session`.
3. If unauthenticated, frontend routes to `/login`.
4. User submits access code to `POST /auth/login`.
5. Backend verifies Argon2 hash and sets secure session cookie.
6. Frontend fetches channel state from `GET /api/channels`.
7. User clicks watch on a live channel.
8. Frontend requests `POST /api/watch-ticket` with channel login.
9. Backend validates session + channel live state and returns short-lived watch URL.
10. Frontend navigates to `/watch/:ticket`.
11. Backend validates ticket/session again and renders Twitch embed page.

## Auth and Session Model

- Shared access code (operator-managed) for trusted users.
- Access code stored as Argon2 hash.
- Session tokens stored server-side in memory (`token -> expires_at`).
- Cookie flags:
  - `HttpOnly`
  - `SameSite=Lax`
  - `Secure` in TLS/prod
- Login abuse controls:
  - attempt window
  - max attempts
  - temporary block duration

## Data and Caching

- No database in MVP.
- Config from file/env.
- In-memory caches:
  - Twitch user-id mapping for configured channels
  - latest channel stream state
  - active sessions
  - short-lived watch ticket verifier context
- Stale-if-error strategy for Twitch outages.

## API Surface (MVP)

Public:
- `GET /healthz`
- `GET /readyz`

Auth:
- `GET /auth/session`
- `POST /auth/login`
- `POST /auth/logout`

Protected:
- `GET /api/channels`
- `GET /api/live`
- `POST /api/watch-ticket`
- `GET /watch/:ticket`

Optional protected realtime:
- `GET /api/stream/channels` (SSE)

## Security Posture (MVP)

- All functional routes require auth except health checks.
- Session cookie is HttpOnly and server-validated.
- Watch access requires short-lived ticket and active session.
- No raw Twitch media URL is exposed via simple public API endpoint.
- Rate limiting + timeout + body-size caps on sensitive endpoints.
- CSRF protection on state-changing routes.
- Security headers and embed-compatible CSP.

## Non-Goals in MVP

- Full HLS relay/proxy through backend.
- Multi-tenant accounts and role hierarchy.
- Horizontal scaling and distributed session store.
- Broad public internet access model.

## Deployment Shape

- One Rust binary process.
- `web/build` served by Rust server.
- Reverse proxy/TLS terminator (Caddy/Nginx) in front.
- Config + secrets via environment and/or local config file.

## Verification Standard

Rust:
- `cargo check`
- `cargo clippy --all-targets --all-features`
- `cargo test`

Web (`web/`):
- `pnpm run verify`

Cross-cutting:
- run both Rust and web verification sets.
