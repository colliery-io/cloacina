# syntax=docker/dockerfile:1.7
#
# cloacina-server — production HTTP API image (CLOACI-I-0111 / T-0604)
#
# Multi-stage build:
#   Stage 1 (builder): rust:1.85-slim — compiles cloacina-server in release.
#   Stage 2 (runtime): debian:bookworm-slim — minimal Debian + libpq5.
#
# Why debian:bookworm-slim instead of distroless?
#   The cloacina crate links against libpq dynamically (diesel/postgres
#   feature). Distroless cc-debian12 lacks libpq, and copying libpq + its
#   transitive deps (libgssapi-krb5, libldap, libssl, libsasl2, ...) from
#   the builder layer is fragile across base-image updates. Bookworm-slim
#   is ~80MB and `apt install libpq5` lands in ~6MB — net image ~150MB
#   compressed. Switching to distroless is a future size optimisation.
#
# Build:   docker build -t cloacina-server:dev .
# Run:     docker run --rm -e DATABASE_URL=postgres://... cloacina-server:dev
# Verify:  docker run --rm cloacina-server:dev --version

ARG RUST_VERSION=1.93

# ---------------------------------------------------------------------------
# Stage 1: builder
# ---------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-slim-bookworm AS builder

# Build deps: libpq for diesel/postgres, pkg-config for libpq discovery,
# git for any vendored deps that resolve git refs at build time.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        libpq-dev \
        pkg-config \
        git \
        ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy the entire workspace. cargo's incremental + workspace resolver makes
# this faster than the per-Cargo.toml dance for our 11-crate layout.
COPY . .

# Release build, locked to ensure reproducibility against the committed
# Cargo.lock. `--bin cloacina-server` keeps us from accidentally compiling
# every binary in the workspace.
RUN cargo build --release --locked --bin cloacina-server

# ---------------------------------------------------------------------------
# Stage 2: runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Runtime deps: libpq5 for diesel, ca-certificates for any HTTPS calls
# (e.g. compiler service callbacks, signature verification).
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        libpq5 \
        ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Non-root user (uid 10001 stays clear of host /etc/passwd).
RUN groupadd --system --gid 10001 cloacina \
 && useradd --system --uid 10001 --gid cloacina --create-home --home-dir /home/cloacina cloacina

COPY --from=builder /build/target/release/cloacina-server /usr/local/bin/cloacina-server
RUN chmod +x /usr/local/bin/cloacina-server

USER cloacina
WORKDIR /home/cloacina

# DATABASE_URL is required at runtime; bind/home/etc. all have sensible
# CLI defaults exposed via clap.
ENV CLOACINA_HOME=/home/cloacina/.cloacina

EXPOSE 8080

# Image labels — drives the GitHub package linkage and makes
# `docker inspect` carry the source/license/description fields.
LABEL org.opencontainers.image.source="https://github.com/colliery-software/cloacina"
LABEL org.opencontainers.image.description="cloacina-server — workflow orchestration HTTP API"
LABEL org.opencontainers.image.licenses="Apache-2.0"

# No HEALTHCHECK directive — distroless-style images can't reliably curl
# /v1/health. Orchestrators (Kubernetes via the Helm chart in T-0605,
# docker-compose via the user's own healthcheck stanza) own probes for
# this image.

ENTRYPOINT ["/usr/local/bin/cloacina-server"]
