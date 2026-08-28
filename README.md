# Twitch Relay

A self-hosted Twitch stream proxy with recording capabilities and optional YouTube support via Invidious.

Twitch Relay acts as a middleman between you and Twitch's streaming infrastructure. It can proxy live streams (useful for circumventing ad block detection), record streams to disk with automatic chapter marking for game/category changes, and provides a unified interface for managing both Twitch and YouTube channel catalogs.

## Project Structure

```
├── src/            # Rust backend (Axum web framework)
├── web/            # SvelteKit + TypeScript frontend
│   ├── src/        # Svelte components and routes
│   ├── static/     # Static assets (watch player JS/CSS, HLS.js)
│   └── build/      # Production build output (served by backend)
├── docker-compose.yml
└── Dockerfile
```

## Local Development

### Backend (Rust)

Requires Rust toolchain (1.88+) and optionally `streamlink` + `ffmpeg` for full functionality.

```bash
# Build the project
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linting
cargo clippy

# Run in dev mode (loads .env file)
cargo run dev
```

### Frontend (SvelteKit)

Requires Node.js 22+ and pnpm.

```bash
cd web

# Install dependencies
pnpm install

# Start development server
pnpm run dev

# Type checking
pnpm run typecheck

# Run linting and checks
pnpm run check

# Build for production
pnpm run build

# Preview production build
pnpm run preview
```

## Docker Usage

The simplest way to run Twitch Relay is via Docker Compose using the pre-built image from GitHub Container Registry.

### Quick Start

```bash
# Create your env file from the example
cp .env.example .env

# Edit .env with your Twitch OAuth credentials
# Then start the container
docker compose pull
docker compose up -d
docker compose logs -f twitch-relay
```

The container exposes port 8080 internally. The compose file maps host port 18081 to container port 8080.

### Setup Notes

- **Environment file**: Create `.env` from `.env.example` and fill in your Twitch OAuth credentials. See `.env.example` for all available options.
- **Recordings directory**: Ensure `./recordings` exists and is writable by the configured user (default `1000:1000`). Recordings will be owned by this UID/GID.
- **Image source**: The compose file uses the published GHCR image (`ghcr.io/wandabastyle/twitch_relay:latest`).
- **Networking**: Compose uses the default project network unless you customize networking in your override file.

### Data Paths

Inside the container:
- `/data` — Application data (database, session files) via `XDG_DATA_HOME`
- `/app/recordings` — Stream recordings output directory

The compose file mounts:
- A Docker volume `twitch-relay-data` to `/data` for persistent app data
- `./recordings:/app/recordings` for host-accessible recordings

### Permissions

The compose file sets `user: "1000:1000"` to ensure recordings written to the host-mounted `./recordings` directory are owned by your local user (UID/GID 1000). Adjust this to match your host user if needed.

## Configuration

Required environment variables for Twitch OAuth integration:
- `TWITCH_OAUTH_CLIENT_ID`
- `TWITCH_OAUTH_CLIENT_SECRET`
- `TWITCH_OAUTH_REDIRECT_URI`
- `TWITCH_TOKEN_ENCRYPTION_KEY`

Optional YouTube/Invidious support:
- **Normal mode:** Set `INVIDIOUS_BASE_URL` and `INVIDIOUS_TOKEN` for direct
  Invidious API auth (Bearer token).
- **Reverse-proxy Basic auth mode:** Set `INVIDIOUS_BASE_URL`,
  `INVIDIOUS_BASIC_AUTH_USER`, `INVIDIOUS_BASIC_AUTH_PASSWORD`, and
  `INVIDIOUS_SID` when Invidious is behind a reverse proxy that uses Basic auth.
  The `SID` cookie is used for Invidious session auth because the `Authorization`
  header is consumed by the proxy.
- If not configured, YouTube features will be disabled and only Twitch will be available.

See `.env.example` for the complete list of configuration options.

## License

This project is licensed under [AGPL-3.0-or-later](LICENSE). Operators of modified network-hosted versions must offer corresponding source to remote users.
