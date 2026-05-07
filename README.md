# Twitch Relay

A self-hosted stream relay and recording service for Twitch (and optionally YouTube via Invidious). Provides a web interface to watch streams, manage recordings, and browse followed channels.

## Structure

- **`src/`** — Rust backend (Axum-based HTTP server, stream proxy, recording scheduler)
- **`web/`** — SvelteKit frontend (TypeScript, built with Vite)
- **`web/static/`** — Static assets (HLS player, watch page CSS/JS)

## Local Development

### Backend

```bash
# Build
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy

# Run locally (requires .env with TWITCH_OAUTH_* and TWITCH_TOKEN_ENCRYPTION_KEY)
cargo run -- dev
```

### Frontend

```bash
cd web

# Install dependencies
pnpm install

# Dev server
pnpm run dev

# Type check
pnpm run typecheck

# Lint and check
pnpm run check

# Build for production
pnpm run build

# Preview production build
pnpm run preview
```

## Docker

```bash
# Start with docker-compose (uses GHCR image)
docker compose up -d
```

The compose file uses the published image `ghcr.io/wandabastyle/twitch_relay:latest`.

### Data Paths

- `/data` — Application data (via `XDG_DATA_HOME`)
- `/app/recordings` — Recording output directory inside the container

The compose file mounts `./recordings:/app/recordings` for host access to recordings.

### Permissions

The compose file sets `user: "1000:1000"` so host-mounted recordings are written with your host UID/GID. Adjust if your user differs.

### Networking

The compose file expects an external Docker network named `media`. Create it first:

```bash
docker network create media
```

Or adjust `docker-compose.yml` to use a different network.

## Configuration

Copy `.env.example` to `.env` and fill in required values:

- `TWITCH_OAUTH_CLIENT_ID`
- `TWITCH_OAUTH_CLIENT_SECRET`
- `TWITCH_OAUTH_REDIRECT_URI`
- `TWITCH_TOKEN_ENCRYPTION_KEY`

See `.env.example` for all available options.
