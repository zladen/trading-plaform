# syntax=docker/dockerfile:1

# Backend image.
#
# The build context is the repository root: the workspace has several crates and
# the migrations are embedded at compile time through `sqlx::migrate!`, so the
# builder needs the manifests, the crates, the application and `migrations/`.
#
# Base images are pinned to patch releases; `latest` is never used.

FROM rust:1.97.1-slim-trixie AS builder

WORKDIR /build

# Build arguments only; no secrets are passed into the image.
ENV CARGO_TERM_COLOR=never \
    CARGO_INCREMENTAL=0 \
    SQLX_OFFLINE=true

# Manifests and sources first. Dependency compilation is cached in BuildKit
# mounts, so a source change does not re-download or re-compile the registry.
COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml ./
COPY crates ./crates
COPY apps/backend ./apps/backend
COPY migrations ./migrations

# The toolchain file selects 1.97.1; the rust image already ships it, and
# rustup would otherwise try to download a redundant copy.
ENV RUSTUP_TOOLCHAIN=1.97.1

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked \
        --package platform-backend \
        --bin platform-backend \
        --bin platform-migrate \
    && mkdir -p /out \
    && cp target/release/platform-backend target/release/platform-migrate /out/

FROM debian:trixie-slim AS runtime

LABEL org.opencontainers.image.title="platform-backend" \
      org.opencontainers.image.description="Platform HTTP and WebSocket API"

# `ca-certificates` for TLS to future external services, `curl` for the compose
# healthcheck. No build toolchain and no package manager cache are kept.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# The container never needs write access to the filesystem, so the process runs
# as an unprivileged user with no home directory.
RUN groupadd --system --gid 10001 platform \
    && useradd --system --uid 10001 --gid platform --no-create-home --shell /usr/sbin/nologin platform

WORKDIR /app

COPY --from=builder --chown=root:root /out/platform-backend /out/platform-migrate /usr/local/bin/

USER 10001:10001

ENV APP_ENV=production \
    APP_HOST=0.0.0.0 \
    APP_PORT=8080

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=20s --retries=5 \
    CMD ["curl", "--fail", "--silent", "--show-error", "--max-time", "4", "http://127.0.0.1:8080/api/v1/health"]

ENTRYPOINT ["/usr/local/bin/platform-backend"]
