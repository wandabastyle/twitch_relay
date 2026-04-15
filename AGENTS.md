# AGENTS

## Current Scope
- This repo is a single Cargo binary crate (`twitch-relay`), not a workspace.
- Runtime entrypoint is `src/main.rs`.
- Frontend lives in `web/` (SvelteKit + TypeScript, managed with pnpm).

## Commands
- `cargo run` — run the binary locally.
- `cargo check` — fast compile verification.
- `cargo clippy --all-targets --all-features` — lint checks (preferred before handing off).
- `cargo test` — run all tests.
- `cargo test <test_name>` — run one test by name.
- `pnpm install` (in `web/`) — install frontend dependencies.
- `pnpm run dev` (in `web/`) — run frontend dev server.
- `pnpm run build` (in `web/`) — build frontend static assets.
- `pnpm run verify` (in `web/`) — frontend type/lint checks.

## Verification Order
- For Rust code changes, use: `cargo check` -> `cargo clippy --all-targets --all-features` -> `cargo test`.
- If formatting is introduced/changed, run `cargo fmt -- --check`.
- For web-only changes (in `web/`), run: `pnpm run verify`.
- For cross-cutting changes (Rust + web), run both Rust checks and `pnpm run verify`.

## Lockfile
- This is a binary crate; keep `Cargo.lock` tracked in git.
- If dependencies change and `Cargo.lock` is missing/stale, regenerate with `cargo generate-lockfile` (or any Cargo build/check command) and commit it with the change.
- Keep `web/pnpm-lock.yaml` tracked in git for frontend dependency changes.

## Verified Constraints
- No CI workflows are configured (`.github/workflows/` is absent).
- No repo-local agent instruction/config files were found (`CLAUDE.md`, `.cursor/rules`, `.cursorrules`, `.github/copilot-instructions.md`, `opencode.json`).
- No extra formatter/linter config is present (`rustfmt.toml`, `clippy.toml` absent); use Cargo defaults unless new config is added.
