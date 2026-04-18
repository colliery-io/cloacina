# cloacina-compiler — polling build worker.
#
# Needs the Rust toolchain at runtime (it shells out to `cargo build` per
# package). Image is deliberately larger than the server; pair the two via
# docker-compose. See CLOACI-I-0097 + ADR-0004 for the two-binary rationale.
#
# Build:  docker build -f deploy/docker/compiler.Dockerfile -t cloacina-compiler .
# Run:    docker run -e DATABASE_URL=postgres://... cloacina-compiler

# ---------------------------------------------------------------------------
# Stage 1: Build
# ---------------------------------------------------------------------------
FROM rust:1.85-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p cloacina-compiler

# ---------------------------------------------------------------------------
# Stage 2: Runtime — full rust image so `cargo build` works on uploaded packages
# ---------------------------------------------------------------------------
FROM rust:1.85-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends libpq5 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cloacina-compiler /usr/local/bin/

RUN mkdir -p /var/lib/cloacina
ENV CLOACINA_HOME=/var/lib/cloacina

# Local status/health — cloacinactl compiler status talks to this.
EXPOSE 9000

ENTRYPOINT ["cloacina-compiler"]
CMD ["--bind", "0.0.0.0:9000"]
