# syntax=docker/dockerfile:1.7

FROM node:22-alpine AS web-build
WORKDIR /build/web

RUN corepack enable

COPY web/package.json web/pnpm-lock.yaml ./
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile

COPY web/ ./
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm run build

FROM rust:1.88-alpine AS rust-base
WORKDIR /build

RUN apk add --no-cache musl-dev pkgconfig \
    && cargo install cargo-chef --locked

FROM rust-base AS rust-planner

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM rust-base AS rust-cook

COPY --from=rust-planner /build/recipe.json recipe.json
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/build/target \
    cargo chef cook --release --locked --recipe-path recipe.json

FROM rust-base AS rust-build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/build/target \
    cargo build --release --locked \
    && cp /build/target/release/twitch-relay /build/twitch-relay

FROM alpine:3.22 AS runtime
WORKDIR /app

RUN apk add --no-cache ca-certificates python3 py3-pip \
    && pip3 install --no-cache-dir --break-system-packages "streamlink==8.3.0" \
    && addgroup -S app \
    && adduser -S -G app -u 10001 app \
    && mkdir -p /app/web/build /app/web/static /app/recordings /data \
    && chown -R app:app /app /data

COPY --from=rust-build /build/twitch-relay /app/twitch-relay
COPY --from=web-build /build/web/build /app/web/build
COPY --from=web-build /build/web/static /app/web/static

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

EXPOSE 8080

USER app

ENTRYPOINT ["/app/twitch-relay"]
