# syntax=docker/dockerfile:1.7

# Stage: web-build
# Purpose: Build the SvelteKit frontend and prepare static assets.
# Outputs: /build/web/build (SvelteKit production build) and /build/web/static (watch player assets)
FROM node:22-alpine AS web-build
WORKDIR /build/web

RUN corepack enable

COPY web/package.json web/pnpm-lock.yaml ./
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile

COPY web/ ./
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm run build

# Stage: rust-base
# Purpose: Base image with Rust toolchain and cargo-chef for dependency caching.
FROM rust:1.88-alpine AS rust-base
WORKDIR /build

RUN apk add --no-cache musl-dev pkgconfig \
    && cargo install cargo-chef --locked

# Stage: rust-planner
# Purpose: Analyze Cargo.toml/Cargo.lock and src to generate a recipe.json for dependency caching.
FROM rust-base AS rust-planner

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Stage: rust-cook
# Purpose: Build and cache dependencies only (no application code yet).
# This layer is cached and reused if dependencies haven't changed.
FROM rust-base AS rust-cook

COPY --from=rust-planner /build/recipe.json recipe.json
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/build/target \
    cargo chef cook --release --locked --recipe-path recipe.json

# Stage: rust-build
# Purpose: Build the Rust binary. Dependencies are pre-cooked from the previous stage.
FROM rust-base AS rust-build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/build/target \
    cargo build --release --locked \
    && cp /build/target/release/twitch-relay /build/twitch-relay

# Stage: runtime
# Purpose: Minimal production image with only runtime dependencies.
FROM alpine:3.22 AS runtime
WORKDIR /app

# Install runtime dependencies and create a non-root user for security.
# The app user (UID 10001) runs the container to limit potential damage from security issues.
RUN apk add --no-cache ca-certificates python3 py3-pip ffmpeg \
    && pip3 install --no-cache-dir --break-system-packages "streamlink==8.3.0" \
    && addgroup -S app \
    && adduser -S -G app -u 10001 app \
    && mkdir -p /app/web/build /app/web/static /app/recordings /data \
    && chown -R app:app /app /data

# Copy the compiled Rust binary into the runtime image.
COPY --from=rust-build /build/twitch-relay /app/twitch-relay

# Copy the built frontend assets into the runtime image.
# /app/web/build: SvelteKit production build (served by the backend at root routes).
# /app/web/static: Static watch player assets (watch.js, watch.css) served at /static.
COPY --from=web-build /build/web/build /app/web/build
COPY --from=web-build /build/web/static /app/web/static

# Default runtime configuration values.
# These can be overridden at runtime via environment variables or docker-compose.yml.
ENV BIND_ADDR=0.0.0.0:8080
ENV STREAMLINK_PATH=streamlink
ENV STREAM_RESOLVER_MODE=auto
ENV STREAM_DELIVERY_MODE=cdn_first
ENV TWITCH_CLIENT_ID=kimne78kx3ncx6brgo4mv6wki5h1ko
ENV XDG_DATA_HOME=/data
ENV RECORDINGS_DIR=/app/recordings
ENV RECORDING_DEFAULT_QUALITY=best
ENV RECORDING_POLL_INTERVAL_SECS=45
ENV RECORDING_START_LIVE_CONFIRMATIONS=2
ENV RECORDING_STOP_OFFLINE_CONFIRMATIONS=3
ENV FFMPEG_PATH=ffmpeg
ENV RECORDING_CHAPTER_MIN_GAP_SECS=180
ENV RECORDING_CHAPTER_CHANGE_CONFIRMATIONS=2

EXPOSE 8080

# Run as non-root user for security.
USER app

ENTRYPOINT ["/app/twitch-relay"]
