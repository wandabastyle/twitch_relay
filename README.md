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

```bash
# Create your env file from the example
cp .env.example .env

# Edit .env with your Twitch OAuth credentials
# Then start the container
docker compose up -d
```

The container exposes port 8080 internally. The compose file maps host port 18081 to container port 8080.

### Data Paths

Inside the container:
- `/data` — Application data (database, session files) via `XDG_DATA_HOME`
- `/app/recordings` — Stream recordings output directory

The compose file mounts:
- A Docker volume `twitch-relay-data` to `/data` for persistent app data
- `./recordings:/app/recordings` for host-accessible recordings

### Permissions

The compose file sets `user: "1000:1000"` to ensure recordings written to the host-mounted `./recordings` directory are owned by your local user (UID/GID 1000). Adjust this to match your host user if needed.

### Environment

Copy `.env.example` to `.env` and configure required values. See `.env.example` for all available options.

## Configuration

Required environment variables for Twitch OAuth integration:
- `TWITCH_OAUTH_CLIENT_ID`
- `TWITCH_OAUTH_CLIENT_SECRET`
- `TWITCH_OAUTH_REDIRECT_URI`
- `TWITCH_TOKEN_ENCRYPTION_KEY`

Optional YouTube/Invidious support:
- Set `INVIDIOUS_BASE_URL` and `INVIDIOUS_TOKEN` to enable YouTube channel features
- If not configured, YouTube features will be disabled and only Twitch will be available

See `.env.example` for the complete list of configuration options.
