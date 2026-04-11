---
title: "Production Deployment"
weight: 30
---

# How to Deploy Cloacina in Production

This guide covers deploying `cloacinactl serve` behind a TLS-terminating reverse proxy.

## Prerequisites

- PostgreSQL 14+ accessible from the server
- A reverse proxy (nginx, Caddy, or Envoy)
- TLS certificate (Let's Encrypt, or self-signed for internal use)

## Why a Reverse Proxy?

`cloacinactl serve` binds plain HTTP. All traffic — including API keys, tenant credentials, and workflow package uploads — is transmitted in cleartext without a reverse proxy providing TLS termination.

A reverse proxy also provides:
- TLS termination with automatic certificate renewal (Caddy)
- Connection draining and load balancing
- Request logging and access control
- Static file serving for documentation

## Caddy (Recommended)

Caddy provides automatic HTTPS with Let's Encrypt.

```
# Caddyfile
cloacina.example.com {
    reverse_proxy localhost:8080
}
```

Start Caddy:
```bash
caddy run --config Caddyfile
```

## nginx

```nginx
server {
    listen 443 ssl http2;
    server_name cloacina.example.com;

    ssl_certificate /etc/ssl/certs/cloacina.pem;
    ssl_certificate_key /etc/ssl/private/cloacina.key;

    # WebSocket support
    location /v1/ws/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Body size limit (match server's 100MB limit)
        client_max_body_size 100m;
    }
}
```

## Starting the Server

```bash
# Basic start
cloacinactl serve \
  --database-url "postgres://user:pass@localhost:5432/cloacina" \
  --bind 127.0.0.1:8080

# With explicit bootstrap key
cloacinactl serve \
  --database-url "postgres://user:pass@localhost:5432/cloacina" \
  --bind 127.0.0.1:8080 \
  --bootstrap-key "clk_your_bootstrap_key_here"
```

The server binds to `127.0.0.1` (localhost only) when behind a reverse proxy — do not bind to `0.0.0.0` without TLS.

## Health Checks

Configure your load balancer to check:

- **Liveness**: `GET /health` — returns 200 if the process is alive
- **Readiness**: `GET /ready` — returns 200 if the database is connected and migrations are applied

## Docker Compose Example

```yaml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: cloacina
      POSTGRES_PASSWORD: cloacina
      POSTGRES_DB: cloacina
    volumes:
      - pgdata:/var/lib/postgresql/data

  cloacina:
    build: .
    command: ["serve", "--database-url", "postgres://cloacina:cloacina@postgres:5432/cloacina", "--bind", "0.0.0.0:8080"]
    depends_on:
      - postgres
    ports:
      - "8080:8080"

  caddy:
    image: caddy:2
    ports:
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile

volumes:
  pgdata:
```
