# Multi-stage Dockerfile for cloacinactl
# Build: docker build -f deploy/Dockerfile -t cloacina .
# Run:   docker run -p 8080:8080 cloacina serve --mode=all

# --- Stage 1: Builder ---
FROM rust:bookworm AS builder

WORKDIR /build

# Install system dependencies for diesel (postgres + sqlite)
RUN apt-get update && apt-get install -y \
    libpq-dev \
    libsqlite3-dev \
    pkg-config \
    python3-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace
COPY . .

# Build release binary
RUN cargo build --release -p cloacinactl \
    && strip target/release/cloacinactl

# --- Stage 2: Runtime ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libpq5 \
    libsqlite3-0 \
    libpython3.11 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/cloacinactl /usr/local/bin/cloacinactl

# Default config location
RUN mkdir -p /etc/cloacina
COPY deploy/cloacina.toml /etc/cloacina/cloacina.toml

EXPOSE 8080

ENTRYPOINT ["cloacinactl"]
CMD ["serve", "--config", "/etc/cloacina/cloacina.toml"]
