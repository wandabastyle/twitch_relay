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
  - `watch.js` / `watch.css` — Watch player assets used by the backend
  - `hls.js` — HLS player library
  - `robots.txt`
- `build/` — Production build output (served by the Rust backend)

## Static watch player assets

The backend-rendered watch page uses files from `static/`, including:

- `watch.js` — Player controls, chat, and HLS handling
- `watch.css` — Watch page styling

These are served as static assets by the Rust backend.

At the moment, `watch.js` is hand-maintained; there is no build step generating it from source. When editing it:

- Keep playback controls behavior unchanged
- Keep chat behavior unchanged
- Test live and relayed playback
- Run frontend checks/build (`pnpm run verify`)

## Deployment

The frontend is built into `web/build` and served as static files by the Rust backend (or included in the Docker image at `/app/web/build`). The `web/static` directory contains assets referenced directly by the backend's watch player functionality.
