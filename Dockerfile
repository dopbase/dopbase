# syntax=docker/dockerfile:1

FROM oven/bun:1 AS admin-ui

WORKDIR /workspace

COPY package.json bun.lock ./
RUN --mount=type=cache,target=/root/.bun/install/cache \
  bun install --frozen-lockfile

COPY index.html vite.config.ts tsconfig*.json eslint.config.js ./
COPY config ./config
COPY public ./public
COPY src ./src

ENV NODE_ENV=production
RUN bun run build:ui:docker

FROM rust:1.98-bookworm AS chef

RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  cargo install cargo-chef --locked

WORKDIR /workspace/app

COPY app/Cargo.toml app/Cargo.lock app/rustfmt.toml ./

FROM chef AS planner

COPY app/src ./src
COPY app/migrations ./migrations
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /workspace/app/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  --mount=type=cache,target=/workspace/app/target \
  cargo chef cook --profile docker --recipe-path recipe.json --locked

COPY app/src ./src
COPY app/migrations ./migrations
COPY --from=admin-ui /workspace/dist /workspace/dist

RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  --mount=type=cache,target=/workspace/app/target \
  cargo build --profile docker --locked --bin dopbase && \
  install -Dm755 /workspace/app/target/docker/dopbase /workspace/dopbase

FROM debian:bookworm-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/dopbase /usr/local/bin/dopbase
RUN chmod 755 /usr/local/bin/dopbase

WORKDIR /data

ENTRYPOINT ["/usr/local/bin/dopbase"]
CMD ["server", "start", "--config", "/etc/dopbase/server.toml"]
