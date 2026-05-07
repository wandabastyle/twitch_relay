# Twitch Relay Frontend

SvelteKit frontend for Twitch Relay.

## Setup

Uses `pnpm` as package manager.

```bash
pnpm install
```

## Available Scripts

```bash
# Start dev server
pnpm run dev

# Build for production (outputs to build/)
pnpm run build

# Preview production build
pnpm run preview

# Type check only
pnpm run typecheck

# Run checks (lint + svelte-check)
pnpm run check

# Run verification (same as check)
pnpm run verify
```

## Build Output

The frontend is built into `web/build/` and served by the Rust backend. The Docker image includes these static files at `/app/web/build/`.

## Static Assets

The `web/static/` directory contains static assets used by the backend and watch player:

- `watch.js` — HLS player logic
- `hls.js` — HLS.js library
- `watch.css` — Watch page styles
- `robots.txt` — Search engine directives
