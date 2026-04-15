# TODO - twitch-relay MVP (Rust backend + Svelte frontend)

## Goal

Build a self-hosted, login-protected web app where trusted users can:
- see configured Twitch channels with clear LIVE/OFFLINE status
- watch live channels inside the app
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

---

## Phase 0 - Project Setup

- [ ] Create `docs/` and this file.
- [ ] Add frontend workspace at `web/` using SvelteKit + TS.
- [ ] Pin package manager to `pnpm@10.33.0` in `web/package.json`.
- [ ] Add/confirm lockfiles are tracked:
  - [ ] `Cargo.lock`
  - [ ] `web/pnpm-lock.yaml`
- [ ] Update `AGENTS.md` to include web verification commands and cross-cutting checks.

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

- [ ] Add core crates:
  - `axum`, `tokio`, `serde`, `serde_json`, `reqwest`, `tower-http`, `tracing`, `thiserror`, `argon2`, `rand`
- [ ] Create app modules:
  - [ ] `src/main.rs`
  - [ ] `src/config.rs`
  - [ ] `src/app.rs`
  - [ ] `src/error.rs`
- [ ] Add health/readiness routes:
  - [ ] `GET /healthz`
  - [ ] `GET /readyz`
- [ ] Add shared app state struct (config + auth + twitch + channel cache + ticket signer).

### Definition of Done

- [ ] App starts and responds on `/healthz` and `/readyz`.
- [ ] `cargo check` passes.

---

## Phase 2 - Simple Auth (imsa_tui style)

- [ ] Implement auth module:
  - [ ] `POST /auth/login`
  - [ ] `GET /auth/session`
  - [ ] `POST /auth/logout`
  - [ ] `require_session` middleware
- [ ] Implement Argon2 access code verification.
- [ ] Implement in-memory session store: `token -> expires_at`.
- [ ] Set secure cookie attributes:
  - [ ] `HttpOnly`
  - [ ] `SameSite=Lax`
  - [ ] `Secure` configurable (true in TLS/prod)
- [ ] Implement login attempt throttling/temporary block:
  - [ ] `max_login_attempts`
  - [ ] `login_window_secs`
  - [ ] `login_block_secs`
- [ ] Add auth event logging (success/failure/blocked/logout).

### Definition of Done

- [ ] Protected route returns 401 without session.
- [ ] Valid login sets cookie and grants protected access.
- [ ] Logout clears cookie and revokes session.

---

## Phase 3 - Twitch Integration (Live Status)

- [ ] Implement Twitch Helix client module:
  - [ ] app access token fetch/refresh
  - [ ] users lookup (login -> user_id)
  - [ ] streams lookup (by user_id)
- [ ] Load allowlisted channels from config.
- [ ] Implement periodic poller task (default 30s).
- [ ] Maintain in-memory `ChannelState` cache:
  - [ ] `is_live`, title, game, viewers, started_at, thumbnail, updated_at
- [ ] Handle outages with stale-if-error behavior (do not hard-drop state immediately).

### Definition of Done

- [ ] Cache updates on schedule.
- [ ] Offline/online transitions visible in cache/API.

---

## Phase 4 - Protected API for Dashboard

- [ ] Add protected endpoints:
  - [ ] `GET /api/channels` (all allowlisted channels + state)
  - [ ] `GET /api/live` (live-only)
- [ ] Include `stale` + `updated_at` metadata in response.
- [ ] Keep non-auth/public endpoints minimal (`/healthz`, `/readyz` only).

### Definition of Done

- [ ] API returns expected JSON shape.
- [ ] Unauthorized requests are denied.

---

## Phase 5 - Watch Tickets + Playback Page

- [ ] Add short-lived watch ticket service:
  - [ ] signed token (`channel`, `session`, `exp`)
  - [ ] TTL default 60s
- [ ] Add endpoint:
  - [ ] `POST /api/watch-ticket` with `{ channel_login }`
- [ ] Add watch route:
  - [ ] `GET /watch/:ticket`
  - [ ] verify session + ticket validity + channel live state
- [ ] Render official Twitch embed only after validation.

### Definition of Done

- [ ] Watch page only opens for authenticated live sessions.
- [ ] Expired/replayed/foreign-session ticket is rejected.
- [ ] Frontend never receives raw Twitch HLS media URL from backend APIs.

---

## Phase 6 - Frontend (SvelteKit + TS + pnpm)

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

## Phase 7 - Serve Frontend from Rust

- [ ] Add static file serving of `web/build`.
- [ ] Add SPA fallback to `index.html`.
- [ ] Ensure API/auth/watch routes are not shadowed by static routing.

### Definition of Done

- [ ] Single process serves API + frontend assets correctly.

---

## Phase 8 - Security Hardening (MVP level)

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

## Phase 9 - Testing

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

## Phase 10 - Verification Matrix (must pass)

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

## Phase 11 - MVP Acceptance Checklist

- [ ] App is self-hostable with file/env config.
- [ ] Only trusted authenticated users can access dashboard/playback.
- [ ] Channel list clearly indicates LIVE/OFFLINE.
- [ ] Watch only enabled for live channels.
- [ ] Playback is inside app UI via official Twitch embed.
- [ ] Backend uses short-lived watch ticket policy.
- [ ] No raw Twitch media URL is exposed via simple public API.
- [ ] Baseline security controls and checks are in place.
- [ ] All verification commands pass.

---

## Phase 12 - Post-MVP Roadmap (later)

- [ ] Optional SSE channel updates for smoother UI.
- [ ] Optional Redis for shared sessions/cache.
- [ ] Optional TOTP/2FA.
- [ ] Optional admin channel management UI.
- [ ] Optional advanced broker mode behind feature flag + legal review.
