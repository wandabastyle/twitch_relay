# Twitch Relay Frontend

SvelteKit frontend for Twitch Relay. Provides the web UI for managing channel catalogs, viewing live streams, and controlling recordings.

## Development

This project uses **pnpm** as its package manager.

```bash
# Install dependencies
pnpm install

# Start development server
pnpm run dev

# Build for production
pnpm run build

# Preview production build
pnpm run preview

# Type checking only
pnpm run typecheck

# Run linting and Svelte checks
pnpm run check

# Run full verification (check)
pnpm run verify
```

## Structure

- `src/` — Svelte components, routes, and application logic
- `static/` — Static assets including:
  - `hls.js` — HLS player library
  - `robots.txt`
- `build/` — Production build output (served by the Rust backend)

## Deployment

The frontend is built into `web/build` and served as static files by the Rust backend (or included in the Docker image at `/app/web/build`). The `web/static` directory contains assets referenced directly by the backend's watch player functionality.
