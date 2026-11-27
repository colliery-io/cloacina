---
title: "How to Deploy Cloacina in Production"
sidebar:
  order: 30
  label: "Production Deployment"
---

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
- **Readiness**: `GET /ready` — returns 200 when the database connection pool is reachable **and** the embedded Python runtime is usable; otherwise 503 with a fixed `reason` field (`database unreachable` or `python runtime unavailable; restart required`). A crashed computation graph does not fail readiness; see `GET /v1/health/graphs`

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
[When to Use Cloacina](../../../explanation/is-cloacina-for-you/#choosing-a-door)
decision table maps each goal to a mode. This guide assumes the **server** shape;
for the horizontally-scaled fleet see
[Deploy an Execution Agent Fleet](../deploy-an-execution-agent-fleet/).

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
- [ ] Application clients use **tenant-scoped** keys (`cloacinactl key create … --role …`), never the admin key. See [01 - Deploy a Server](../../../tutorials/service/01-deploy-a-server/).
- [ ] CORS configured **only if** a browser UI calls the server cross-origin — via the `CLOACINA_CORS_ALLOWED_ORIGINS` env var (works with `cloacinactl server start`) or the `--cors-allowed-origins` flag on the `cloacina-server` binary. The value is the origin users load the UI from. See [Deploy the Web UI](../deploy-the-web-ui/).

**Data & isolation**
- [ ] PostgreSQL (not SQLite) for any multi-tenant or multi-replica deployment. See [Database Backends](../../../explanation/service/database-backends/).
- [ ] Multi-tenant isolation reviewed — in particular that executions actually run against the tenant's own schema (a misconfigured runner can execute against the wrong schema and break isolation). See [Configure a Multi-Tenant Deployment](../configure-multi-tenant-deployment/).
- [ ] Database backups and a restore drill in place. See [Back Up and Restore](../backup-and-restore/).
- [ ] Run history retention period chosen. It is off by default: run history is kept for ever, and the database grows with each run, until you set a period. See [Run history retention](#run-history-retention).

**Supply chain**
- [ ] In low-trust / multi-tenant deployments, package signing required so unsigned packages are refused — otherwise a compromised or untrusted uploader could get arbitrary code compiled and run by your compiler, which has no process sandbox. See [Require Signed Packages](../require-signed-packages/).

**Operations**
- [ ] Load-balancer health checks wired to `GET /health` (liveness) and `GET /ready` (readiness — confirms the DB pool is reachable and no computation graph has crashed).
- [ ] Metrics scraped from `/metrics` and tracing wired up. See [Observe Execution State](../../embed/observe-execution-state/).
- [ ] Runner sizing tuned for your load — database connection pool size, task concurrency, and execution timeouts. See [Performance Tuning](../performance-tuning/) for the server knobs and recommended values.

## Run history retention

Every workflow run writes rows that stay after it finishes: one
`workflow_executions` row, one `schedule_executions` row when a schedule
started it, and per task one `task_executions` row, one
`task_execution_metadata` row and one `contexts` row with the full context
JSON. Each state change adds an `execution_events` row and a `delivery_outbox`
row. The server can remove them with a background retention job, once you set a period.

| Setting | Env var | Default | What it does |
|---|---|---|---|
| `--run-history-retention-days` | `CLOACINA_RUN_HISTORY_RETENTION_DAYS` | `0` (off) | A run that finished (Completed, Failed or Cancelled) longer ago than this is deleted, with its task executions, task metadata, contexts, execution events, recovery events, schedule executions and any event push still waiting in `delivery_outbox`. `0` = off. |
| `--outbox-acked-retention-s` | `CLOACINA_OUTBOX_ACKED_RETENTION_S` | `3600` | A `delivery_outbox` row is deleted this long after its ack. `0` = off. |
| `--retention-interval-s` | `CLOACINA_RETENTION_INTERVAL_S` | `300` | Seconds between passes. The first pass runs one interval after start. |

What to know:

- **Only finished runs are deleted.** A run that is Pending, Running or Paused is never touched, however old. A context is deleted only when no remaining run or task references it.
- **The delete is final.** After the period, `GET /v1/tenants/{tenant_id}/executions/{exec_id}` and the UI no longer show the run. If you must keep history longer (audit, compliance), raise the period or set it to `0`, or export before it expires.
- **Run history is kept until you set a period.** `--run-history-retention-days` is `0` by default, so no run is deleted. When you set a period, the first passes remove every finished run older than it. Acked `delivery_outbox` rows are queue records, not history, and are cleared after an hour by default.
- **Per tenant.** Each pass covers the admin (`public`) schema and every tenant schema, with the same periods.
- **One replica at a time.** Each pass is gated on a Postgres advisory lock, so replicas do not work in parallel.
- **Bounded work.** A pass removes at most 5,000 runs and 100,000 acked outbox rows per schema; a backlog drains over the next passes. Postgres reuses the freed space after autovacuum; the files on disk do not shrink without `VACUUM FULL`.
- **Watch it.** `cloacina_retention_rows_deleted_total{table}` counts removed rows; `cloacina_retention_errors_total` counts failed passes. See the [metrics catalog](../../../reference/metrics-catalog/).
- **Separate from logs.** `--log-retention-days` limits log files on disk and nothing else.

Storage per run has not been measured yet, so there is no bytes-per-run
figure to size the database from. The row count is: 1 run row, plus per task
1 task row, 1 metadata row, 1 context row and several event rows.

The daemon (`cloacinactl daemon`) and embedded applications do not run this
job. For those, `cloacinactl admin cleanup-events` removes old
`execution_events` rows only; see
[Cleaning Up Events](../../embed/cleaning-up-events/).

## Upgrading the server

For Kubernetes/Helm deployments, follow the rolling-update path in
[Deploying to Kubernetes](../deploying-to-kubernetes/#upgrades).
For a directly-managed server (`cloacinactl server start` or the container image),
upgrade like this:

1. **Back up the database first.** Run a fresh backup ([Back Up and Restore](../backup-and-restore/)) so you can roll back if a migration misbehaves.
2. **Pin the new version.** Deploy a specific image tag (e.g. `ghcr.io/colliery-io/cloacina-server:0.12.0`), never `:latest`, so the rollout is reproducible.
3. **Drain, then swap.** Stop sending new traffic at the reverse proxy, let in-flight requests finish, stop the old process, and start the new one against the **same** `DATABASE_URL`. The server **runs any pending schema migrations automatically on startup** (embedded, transaction-safe), so no manual migration step is needed.
4. **Gate traffic on readiness.** Wait for `GET /ready` to return 200 before re-enabling proxy traffic — it confirms the DB pool is reachable and no loaded graph crashed on boot.

**Downtime note:** a single-replica server has a brief gap between stop and ready.
For zero-downtime, run multiple replicas against one PostgreSQL database (see
[Horizontal Scaling](../../../explanation/service/horizontal-scaling/)) and
roll them one at a time behind the proxy.

**Rollback:** if the new version fails readiness, restart the previous pinned tag
against the same database. If a migration changed the schema incompatibly, restore
the pre-upgrade backup from step 1.
