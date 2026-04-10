# Multi-stage production build for cloacinactl serve
#
# Build:  docker build -f docker/Dockerfile -t cloacina .
# Run:    docker run -p 8080:8080 -e DATABASE_URL=postgres://... cloacina serve

# ---------------------------------------------------------------------------
# Stage 1: Build
# ---------------------------------------------------------------------------
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y libpq-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p cloacinactl

# ---------------------------------------------------------------------------
# Stage 2: Runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends libpq5 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cloacinactl /usr/local/bin/

# Default home directory for config, logs, and packages
RUN mkdir -p /var/lib/cloacina
ENV CLOACINA_HOME=/var/lib/cloacina

EXPOSE 8080

ENTRYPOINT ["cloacinactl"]
CMD ["serve", "--bind", "0.0.0.0:8080"]
