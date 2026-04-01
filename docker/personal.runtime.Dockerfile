FROM node:22.22.0-alpine3.23 AS frontend-builder

WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --legacy-peer-deps

COPY frontend ./
RUN npm run build

FROM rust:1.93.1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services
COPY vendor ./vendor
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

RUN cargo build --release -p control-plane --no-default-features --features sqlite-backend --bin codex-pool-personal

FROM debian:12.13-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/codex-pool-personal /usr/local/bin/codex-pool-personal

ENV RUST_LOG=info

EXPOSE 8090

CMD ["codex-pool-personal"]
