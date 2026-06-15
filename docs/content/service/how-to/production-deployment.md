---
title: "Production Deployment"
weight: 30
---

# How to Deploy Cloacina in Production

This guide covers deploying `cloacinactl server start` behind a TLS-terminating reverse proxy.

## Prerequisites

- PostgreSQL 14+ accessible from the server
- A reverse proxy (nginx, Caddy, or Envoy)
- TLS certificate (Let's Encrypt, or self-signed for internal use)

## Why a Reverse Proxy?

`cloacinactl server start` binds plain HTTP. All traffic — including API keys, tenant credentials, and workflow package uploads — is transmitted in cleartext without a reverse proxy providing TLS termination.

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
cloacinactl server start \
  --database-url "postgres://user:pass@localhost:5432/cloacina" \
  --bind 127.0.0.1:8080

# With explicit bootstrap key
cloacinactl server start \
  --database-url "postgres://user:pass@localhost:5432/cloacina" \
  --bind 127.0.0.1:8080 \
  --bootstrap-key "clk_your_bootstrap_key_here"
```

The server binds to `127.0.0.1` (localhost only) when behind a reverse proxy — do not bind to `0.0.0.0` without TLS.

## Health Checks

Configure your load balancer to check:

- **Liveness**: `GET /health` — returns 200 if the process is alive
- **Readiness**: `GET /ready` — returns 200 when the database connection pool is reachable **and** no loaded computation graph has crashed; otherwise 503 with a `reason` field (`database unreachable` or `crashed computation graphs`)

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
    # cloacina-server takes its flags directly (no subcommand). Binding
    # 0.0.0.0 is safe here because the port is NOT published to the host —
    # only Caddy (443) is reachable from outside the compose network.
    command: ["--database-url", "postgres://cloacina:cloacina@postgres:5432/cloacina", "--bind", "0.0.0.0:8080"]
    depends_on:
      - postgres
    expose:
      - "8080"

  caddy:
    image: caddy:2
    ports:
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile

volumes:
  pgdata:
```

## Choosing a deployment shape first

Before productionizing, confirm you're deploying the right shape — embedded
library, local daemon, single server, or server + compiler + agent fleet. The
[When to Use Cloacina]({{< ref "/quick-start/when-to-use" >}}#choosing-a-mode)
decision table maps each goal to a mode. This guide assumes the **server** shape;
for the horizontally-scaled fleet see
[Deploy an Execution Agent Fleet]({{< ref "/service/how-to/deploy-an-execution-agent-fleet" >}}).

## Production readiness checklist

Work through this before exposing a server to real traffic. Each item links to
the guide that covers it in depth.

**Network & transport**
- [ ] TLS terminated at a reverse proxy (Caddy/nginx above); the server itself binds plain HTTP.
- [ ] Server bound to `127.0.0.1` (or a private interface) — never `0.0.0.0` reachable from the internet without the proxy in front. Don't publish the raw `8080` port.
- [ ] Reverse-proxy `client_max_body_size` matches the server's 100 MB package-upload limit.
- [ ] WebSocket upgrade headers proxied (needed for live execution streams and event ingestion).

**Authentication & access**
- [ ] Bootstrap admin key captured into a secret manager on first start, then the `~/.cloacina/bootstrap-key` file removed or locked down. It is shown only once.
- [ ] Application clients use **tenant-scoped** keys (`cloacinactl key create … --role …`), never the admin key. See [01 - Deploy a Server]({{< ref "/service/tutorials/01-deploy-a-server" >}}).
- [ ] CORS configured **only if** a browser UI calls the server cross-origin — via the `CLOACINA_CORS_ALLOWED_ORIGINS` env var (works with `cloacinactl server start`) or the `--cors-allowed-origins` flag on the `cloacina-server` binary. The value is the origin users load the UI from. See [Deploy the Web UI]({{< ref "/service/how-to/deploy-the-web-ui" >}}).

**Data & isolation**
- [ ] PostgreSQL (not SQLite) for any multi-tenant or multi-replica deployment. See [Database Backends]({{< ref "/service/explanation/database-backends" >}}).
- [ ] Multi-tenant isolation reviewed — in particular that executions actually run against the tenant's own schema (a misconfigured runner can execute against the wrong schema and break isolation). See [Configure a Multi-Tenant Deployment]({{< ref "/service/how-to/configure-multi-tenant-deployment" >}}).
- [ ] Database backups and a restore drill in place.

**Supply chain**
- [ ] In low-trust / multi-tenant deployments, package signing required so unsigned packages are refused — otherwise a compromised or untrusted uploader could get arbitrary code compiled and run in your build sandbox. See [Require Signed Packages]({{< ref "/service/how-to/require-signed-packages" >}}).

**Operations**
- [ ] Load-balancer health checks wired to `GET /health` (liveness) and `GET /ready` (readiness — confirms the DB pool is reachable and no computation graph has crashed).
- [ ] Metrics scraped from `/metrics` and tracing wired up. See [Observe Execution State]({{< ref "/workflows/how-to-guides/observe-execution-state" >}}).
- [ ] Runner sizing tuned for your load — database connection pool size, task concurrency, and execution timeouts. See [Performance Tuning]({{< ref "/service/how-to/performance-tuning" >}}) for the server knobs and recommended values.
