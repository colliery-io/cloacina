# cloacina-server — HTTP API runtime.
#
# Slim runtime image — the reconciler loads pre-compiled cdylib bytes from
# workflow_packages.compiled_data, so no Rust toolchain is required at
# runtime (see ADR-0004 / CLOACI-I-0097). Pair this with the compiler image
# via docker-compose so uploaded packages actually get built.
#
# Build:  docker build -f deploy/docker/server.Dockerfile -t cloacina-server .
# Run:    docker run -p 8080:8080 -e DATABASE_URL=postgres://... cloacina-server

# ---------------------------------------------------------------------------
# Stage 1: Build
# ---------------------------------------------------------------------------
FROM rust:1.85-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p cloacina-server

# ---------------------------------------------------------------------------
# Stage 2: Runtime — debian-slim, no cargo, no rustc
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends libpq5 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cloacina-server /usr/local/bin/

RUN mkdir -p /var/lib/cloacina
ENV CLOACINA_HOME=/var/lib/cloacina

EXPOSE 8080

ENTRYPOINT ["cloacina-server"]
CMD ["--bind", "0.0.0.0:8080"]
