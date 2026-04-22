FROM node:22-alpine AS web-build
WORKDIR /build/web

COPY web/package.json web/pnpm-lock.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile

COPY web/ ./
RUN pnpm run build

FROM rust:1.88-alpine AS rust-build
WORKDIR /build

RUN apk add --no-cache musl-dev pkgconfig

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:3.22 AS runtime
WORKDIR /app

RUN apk add --no-cache ca-certificates streamlink \
    && addgroup -S app \
    && adduser -S -G app -u 10001 app \
    && mkdir -p /app/web/build /app/web/static /data \
    && chown -R app:app /app /data

COPY --from=rust-build /build/target/release/twitch-relay /app/twitch-relay
COPY --from=web-build /build/web/build /app/web/build
COPY --from=web-build /build/web/static /app/web/static

ENV BIND_ADDR=0.0.0.0:8080
ENV STREAMLINK_PATH=streamlink
ENV STREAM_RESOLVER_MODE=auto
ENV TWITCH_CLIENT_ID=kimne78kx3ncx6brgo4mv6wki5h1ko
ENV XDG_DATA_HOME=/data

EXPOSE 8080

USER app

ENTRYPOINT ["/app/twitch-relay"]
