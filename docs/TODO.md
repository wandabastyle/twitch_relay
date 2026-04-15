# TODO - twitch-relay MVP (Rust backend + Svelte frontend)

## Goal

Build a self-hosted, login-protected web app where trusted users can:
- see configured Twitch channels from an allowlist
- watch channels inside the app using backend-issued watch routes
- keep Twitch-facing details controlled by backend policy (no raw media URL API exposure)

---

## Decisions Locked In

- Backend: Rust (axum + tokio)
- Frontend: SvelteKit + TypeScript
- Package manager: pnpm
- Auth: simple shared access code (imsa_tui style), cookie session
- Playback MVP: official Twitch embed only
- Access control: short-lived backend-issued watch ticket
- Config: file/env-based, no DB required for MVP
- Live status polling is deferred until after playback-first MVP

---

## Phase 0 - Project Setup

- [x] Create `docs/` and this file.
- [x] Add frontend workspace at `web/` using SvelteKit + TS.
- [x] Pin package manager to `pnpm@10.33.0` in `web/package.json`.
- [x] Add/confirm lockfiles are tracked:
  - [x] `Cargo.lock`
  - [x] `web/pnpm-lock.yaml`
- [x] Update `AGENTS.md` to include web verification commands and cross-cutting checks.

### Commands

- Rust baseline:
  - `cargo check`
- Web scaffold:
  - `pnpm create svelte@latest web`
  - choose: TypeScript, ESLint, minimal extras
- Web deps:
  - `cd web`
  - `pnpm install`

---

## Phase 1 - Backend Skeleton (Rust)

- [x] Add core crates:
  - `axum`, `tokio`, `serde`, `serde_json`, `reqwest`, `tower-http`, `tracing`, `thiserror`, `argon2`, `rand`
- [x] Create app modules:
  - [x] `src/main.rs`
  - [x] `src/config.rs`
  - [x] `src/app.rs`
  - [x] `src/error.rs`
- [x] Add health/readiness routes:
  - [x] `GET /healthz`
  - [x] `GET /readyz`
- [x] Add shared app state bootstrap for auth + playback services.

### Definition of Done

- [x] App starts and responds on `/healthz` and `/readyz`.
- [x] `cargo check` passes.

---

## Phase 2 - Simple Auth (imsa_tui style)

- [x] Implement auth module:
  - [x] `POST /auth/login`
  - [x] `GET /auth/session`
  - [x] `POST /auth/logout`
  - [x] `require_session` middleware
- [x] Implement Argon2 access code verification.
- [x] Implement in-memory session store: `token -> expires_at`.
- [x] Set secure cookie attributes:
  - [x] `HttpOnly`
  - [x] `SameSite=Lax`
  - [x] `Secure` configurable (true in TLS/prod)
- [x] Implement login attempt throttling/temporary block:
  - [x] `max_login_attempts`
  - [x] `login_window_secs`
  - [x] `login_block_secs`
- [x] Add auth event logging (success/failure/blocked/logout).

### Definition of Done

- [x] Protected route returns 401 without session.
- [x] Valid login sets cookie and grants protected access.
- [x] Logout clears cookie and revokes session.

---

## Phase 3 - Playback-First Channel Access

- [x] Load allowlisted channels from config (`TWITCH_CHANNELS`).
- [x] Add protected endpoint `GET /api/channels` (allowlist only, no live metadata yet).
- [x] Add short-lived watch ticket service (session-bound, TTL from `WATCH_TICKET_TTL_SECS`).
- [x] Add protected endpoint `POST /api/watch-ticket` with `{ channel_login }`.
- [x] Add protected route `GET /watch/:ticket` and render Twitch embed page.

### Definition of Done

- [x] Authenticated user can request watch ticket for allowlisted channel.
- [x] Watch route rejects invalid, expired, or session-mismatched tickets.
- [x] No raw Twitch media URL is returned by backend API.

---

## Phase 4 - Deferred: Twitch Live Status

- [ ] Implement Twitch Helix client module (token/users/streams).
- [ ] Add periodic live-status polling cache.
- [ ] Add `GET /api/live` live-only endpoint.
- [ ] Enrich `GET /api/channels` with online/offline metadata.

### Definition of Done

- [ ] Live/offline transitions are visible in API.
- [ ] Stale-if-error behavior works during Twitch API outages.

---

## Phase 5 - Frontend Dashboard + Player Flow

- [ ] Add protected endpoints:
  - [x] `GET /api/channels` (allowlisted channels)
  - [x] `POST /api/watch-ticket`
  - [x] `GET /watch/:ticket`
- [ ] Build frontend login/dashboard/watch flow around those endpoints.

### Definition of Done

- [x] Backend API returns expected playback-first shapes.
- [x] Unauthorized requests are denied.
- [ ] Frontend can open the ticket-based watch page from channel selection.

---

## Phase 6 - Frontend (SvelteKit + TS + pnpm)

- [x] Add short-lived watch ticket service:
  - [x] token (`channel`, `session`, `exp`)
  - [x] TTL default 60s
- [x] Add endpoint:
  - [x] `POST /api/watch-ticket` with `{ channel_login }`
- [x] Add watch route:
  - [x] `GET /watch/:ticket`
  - [x] verify session + ticket validity
- [x] Render official Twitch embed only after validation.

### Definition of Done

- [x] Watch page only opens for authenticated sessions with valid ticket.
- [ ] Expired/replayed/foreign-session ticket is rejected.
- [x] Frontend never receives raw Twitch HLS media URL from backend APIs.

---

## Phase 7 - Serve Frontend from Rust

- [ ] Configure SvelteKit static output (imsa_tui pattern):
  - [ ] `@sveltejs/adapter-static`
  - [ ] output to `web/build`
  - [ ] fallback `index.html`
- [ ] Add scripts in `web/package.json`:
  - [ ] `dev`
  - [ ] `build`
  - [ ] `preview`
  - [ ] `typecheck`
  - [ ] `check`
  - [ ] `lint`
  - [ ] `verify` = typecheck + check + lint
- [ ] Build pages:
  - [ ] login page
  - [ ] dashboard page (channel cards, LIVE/OFFLINE)
  - [ ] watch page
- [ ] Add frontend API client types for strict request/response checking.

### Definition of Done

- [ ] Login flow works end-to-end with cookie session.
- [ ] Dashboard reflects live/offline state.
- [ ] Clicking Watch for live channel opens validated player route.

---

## Phase 8 - Security Hardening (MVP level)

- [ ] Add static file serving of `web/build`.
- [ ] Add SPA fallback to `index.html`.
- [ ] Ensure API/auth/watch routes are not shadowed by static routing.

### Definition of Done

- [ ] Single process serves API + frontend assets correctly.

---

## Phase 9 - Testing

- [ ] Add request timeout and body size limits.
- [ ] Add route-level rate limiting:
  - [ ] stricter on `/auth/login`
  - [ ] moderate on `/api/watch-ticket`
- [ ] Add CSRF protection for state-changing routes (`POST`).
- [ ] Add security headers with Twitch-embed-compatible CSP.
- [ ] Ensure secrets/tokens are never logged in plaintext.

### Definition of Done

- [ ] Basic abuse paths are constrained.
- [ ] Embed still works with CSP policy.

---

## Phase 10 - Verification Matrix (must pass)

- [ ] Add integration tests (auth lifecycle):
  - [ ] unauthenticated denied
  - [ ] bad login rejected
  - [ ] good login accepted
  - [ ] logout invalidates session
- [ ] Add ticket tests:
  - [ ] valid ticket works
  - [ ] expired ticket fails
  - [ ] wrong-session ticket fails
- [ ] Add Twitch client tests with mocked responses.
- [ ] Add core handler tests for channel/live endpoints.

### Definition of Done

- [ ] Critical auth + ticket + endpoint flows are covered.

---

## Phase 11 - MVP Acceptance Checklist

### Rust-only changes

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features`
- [ ] `cargo test`
- [ ] If formatting changed: `cargo fmt -- --check`

### Web-only changes (run in `web/`)

- [ ] `pnpm run verify`

### Cross-cutting changes (Rust + Web)

- [ ] All Rust checks above
- [ ] `pnpm run verify`

---

## Phase 12 - Post-MVP Roadmap (later)

- [ ] App is self-hostable with file/env config.
- [ ] Only trusted authenticated users can access dashboard/playback.
- [x] Channel list is restricted to configured allowlist.
- [x] Watch access requires short-lived backend ticket.
- [ ] Playback is inside app UI via official Twitch embed.
- [ ] Backend uses short-lived watch ticket policy.
- [ ] No raw Twitch media URL is exposed via simple public API.
- [ ] Baseline security controls and checks are in place.
- [ ] All verification commands pass.

---

## Phase 13 - Live Status Roadmap (later)

- [ ] Add Twitch credentials and Helix polling.
- [ ] Add live/offline channel state and live-only filters.
- [ ] Optional SSE channel updates for smoother UI.
- [ ] Optional Redis for shared sessions/cache.
- [ ] Optional TOTP/2FA.
- [ ] Optional admin channel management UI.
- [ ] Optional advanced broker mode behind feature flag + legal review.
