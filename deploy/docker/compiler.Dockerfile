# cloacina-compiler — polling build worker.
#
# The image published as ghcr.io/colliery-io/cloacina-compiler at each release
# (CLOACINA-T-1088). It needs the Rust toolchain at runtime: the compiler
# shells out to `cargo build` per uploaded package. It carries no cloacina
# source: packages resolve the cloacina crates from the registry (or from the
# operator's vendor dir). Pair with the server via docker-compose. See
# CLOACI-I-0097 + ADR-0004 for the two-binary rationale.
#
# docker/Dockerfile.compiler is the dev/e2e image: it bakes the workspace for
# `--dev-workspace` and is not published.
#
# Build:  docker build -f deploy/docker/compiler.Dockerfile -t cloacina-compiler .
# Run:    docker run -e DATABASE_URL=postgres://... cloacina-compiler

ARG RUST_VERSION=1.99.0

# ---------------------------------------------------------------------------
# Stage 1: Build
# ---------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-bookworm AS builder

# Same build chain as the cloacina-server builder stage.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        cmake \
        libpq-dev \
        libsasl2-dev \
        libssl-dev \
        pkg-config \
        git \
        ca-certificates \
        python3 \
        python3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release --locked -p cloacina-compiler --bin cloacina-compiler

# ---------------------------------------------------------------------------
# Stage 2: Runtime — full rust image so `cargo build` works on uploaded packages
# ---------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-bookworm

# The build chain is here at runtime too: uploaded packages compile against
# libpq / pyo3 / rdkafka, the same libraries the server and agent images
# carry for the cdylibs they load.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        cmake \
        libpq-dev \
        libsasl2-dev \
        libssl-dev \
        pkg-config \
        git \
        ca-certificates \
        python3 \
        python3-dev \
    && rm -rf /var/lib/apt/lists/*

# CLOACI-T-0836: constructor providers are built to WASM components at package
# build time (`cargo build --target wasm32-wasip2`).
RUN rustup target add wasm32-wasip2

COPY --from=builder /app/target/release/cloacina-compiler /usr/local/bin/

RUN mkdir -p /var/lib/cloacina
ENV CLOACINA_HOME=/var/lib/cloacina

# Local status/health — cloacinactl compiler status talks to this.
EXPOSE 9000

LABEL org.opencontainers.image.source="https://github.com/colliery-io/cloacina"
LABEL org.opencontainers.image.description="cloacina-compiler — build service for uploaded workflow packages"
LABEL org.opencontainers.image.licenses="Apache-2.0"

ENTRYPOINT ["cloacina-compiler"]
CMD ["--bind", "0.0.0.0:9000"]
