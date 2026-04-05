---
title: "Deploying the API Server"
description: "How to deploy the Cloacina HTTP API server with PostgreSQL, authentication, and production configuration"
weight: 55
---

# Deploying the API Server

This guide covers starting the Cloacina API server, configuring authentication, and deploying to production. The API server provides a multi-tenant HTTP interface backed by PostgreSQL.

## Prerequisites

- `cloacinactl` binary installed
- PostgreSQL 16+ running and accessible
- A database created for Cloacina (migrations run automatically on startup)

## Starting the Server

### Step 1: Start PostgreSQL

If you do not have a PostgreSQL instance, use the project's Docker Compose file:

```bash
docker compose -f .angreal/docker-compose.yaml up -d
```

This starts PostgreSQL 16 on port 5432 with credentials `cloacina:cloacina` and database `cloacina`.

### Step 2: Start the API Server

```bash
cloacinactl serve --database-url postgresql://cloacina:cloacina@localhost:5432/cloacina
```

The server binds to `0.0.0.0:8080` by default. To change the bind address:

```bash
cloacinactl serve \
  --database-url postgresql://cloacina:cloacina@localhost:5432/cloacina \
  --bind 127.0.0.1:9090
```

On startup, the server connects to PostgreSQL, applies any pending migrations, and prints the available endpoints:

```
API server is running on http://0.0.0.0:8080
  GET  /health     -- liveness check
  GET  /ready      -- readiness check
  GET  /metrics    -- Prometheus metrics
  POST /auth/keys  -- create API key (auth required)
  GET  /auth/keys  -- list API keys (auth required)
  DEL  /auth/keys/:id -- revoke key (auth required)
```

### Step 3: Retrieve the Bootstrap Key

On first startup, the server auto-generates an admin API key and writes it to `~/.cloacina/bootstrap-key` with `0600` permissions. Read it once:

```bash
cat ~/.cloacina/bootstrap-key
```

Store this key securely. It is the only way to authenticate until you create additional keys.

## Bootstrap Key Options

The bootstrap key is created only when no API keys exist in the database. There are three ways to control it:

### Auto-Generated (Default)

The server generates a random key and writes it to `~/.cloacina/bootstrap-key`. No flags needed.

### Explicit Key via Flag

Provide a specific key on first startup:

```bash
cloacinactl serve \
  --database-url postgresql://... \
  --bootstrap-key "my-secret-admin-key-here"
```

### Explicit Key via Environment Variable

```bash
export CLOACINA_BOOTSTRAP_KEY="my-secret-admin-key-here"
cloacinactl serve --database-url postgresql://...
```

In all cases, the plaintext key is written to `~/.cloacina/bootstrap-key` (mode `0600`). On subsequent startups, the bootstrap step is skipped because keys already exist.

## First-Time Setup: Create an API Key

Use the bootstrap key to create a named API key for regular use:

```bash
curl -s -X POST http://localhost:8080/auth/keys \
  -H "Authorization: Bearer $(cat ~/.cloacina/bootstrap-key)" \
  -H "Content-Type: application/json" \
  -d '{"name": "ci-deploy"}' | jq
```

Response:

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "name": "ci-deploy",
  "key": "clk_abc123...",
  "permissions": "admin",
  "created_at": "2026-04-02T12:00:00+00:00"
}
```

The `key` field is returned exactly once. Store it in your secrets manager. All authenticated endpoints require an `Authorization: Bearer <key>` header.

## Health Checks

The server exposes two unauthenticated health endpoints:

### Liveness: GET /health

Returns 200 if the process is alive. Does not check database connectivity.

```bash
curl -s http://localhost:8080/health | jq
```

```json
{"status": "ok"}
```

### Readiness: GET /ready

Returns 200 if the server can acquire a database connection from the pool. Returns 503 if the database is unreachable.

```bash
curl -s http://localhost:8080/ready | jq
```

```json
{"status": "ready"}
```

Use `/health` for container liveness probes and `/ready` for load balancer readiness checks.

## Production Configuration

### Database URL

The database URL can be provided through three sources (highest priority first):

1. `--database-url` CLI flag
2. `DATABASE_URL` environment variable
3. `database_url` in `~/.cloacina/config.toml`

Set it in the config file for convenience:

```bash
cloacinactl config set database_url "postgresql://cloacina:secret@db.example.com:5432/cloacina"
```

### Bind Address

For production, bind to all interfaces on a specific port:

```bash
cloacinactl serve --bind 0.0.0.0:8080
```

For local-only access (behind a reverse proxy on the same host):

```bash
cloacinactl serve --bind 127.0.0.1:8080
```

### Logging

The server writes logs to both stderr and `~/.cloacina/logs/cloacina-server.log` (daily rotation, JSON format). Control verbosity with `RUST_LOG`:

```bash
RUST_LOG=info cloacinactl serve --database-url postgresql://...
```

Use `--verbose` for debug-level output during troubleshooting.

## TLS Termination

The API server does not handle TLS directly. Place a reverse proxy in front of it for HTTPS.

### Caddy Example

```
api.example.com {
    reverse_proxy localhost:8080
}
```

Caddy handles automatic certificate provisioning and renewal.

### Nginx Example

```nginx
server {
    listen 443 ssl;
    server_name api.example.com;

    ssl_certificate     /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Docker Deployment

### docker-compose.yaml

```yaml
services:
  postgres:
    image: postgres:16
    container_name: cloacina-postgres
    environment:
      POSTGRES_USER: cloacina
      POSTGRES_PASSWORD: cloacina
      POSTGRES_DB: cloacina
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    command: postgres -c max_connections=500
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U cloacina"]
      interval: 5s
      timeout: 5s
      retries: 5

  api:
    image: your-registry/cloacinactl:latest
    command:
      - serve
      - --bind=0.0.0.0:8080
      - --database-url=postgresql://cloacina:cloacina@postgres:5432/cloacina
    ports:
      - "8080:8080"
    environment:
      RUST_LOG: "info"
    volumes:
      - cloacina_home:/root/.cloacina
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  postgres_data:
    name: cloacina_postgres_data
  cloacina_home:
    name: cloacina_home
```

Start with:

```bash
docker compose up -d
```

Retrieve the bootstrap key from the container volume:

```bash
docker compose exec api cat /root/.cloacina/bootstrap-key
```

## Graceful Shutdown

The server handles `SIGINT` (Ctrl+C) and `SIGTERM` for graceful shutdown, draining in-flight HTTP requests before exiting. Container orchestrators like Kubernetes send SIGTERM by default, which the server handles correctly.

## API Key Management

### List Keys

```bash
curl -s http://localhost:8080/auth/keys \
  -H "Authorization: Bearer $API_KEY" | jq
```

### Revoke a Key

```bash
curl -s -X DELETE http://localhost:8080/auth/keys/a1b2c3d4-e5f6-7890-abcd-ef1234567890 \
  -H "Authorization: Bearer $API_KEY" | jq
```

Revoked keys are rejected immediately (the server clears its LRU auth cache on revocation).

For the full list of API endpoints including tenant management, workflow upload, and execution, see the [HTTP API Reference]({{< ref "reference/http-api" >}}).
