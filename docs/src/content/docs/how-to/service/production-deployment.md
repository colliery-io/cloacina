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
row. The server can remove them with a background retention job, once you set a period. Both periods are off by default; with both off the job does not start.

| Setting | Env var | Default | What it does |
|---|---|---|---|
| `--run-history-retention-days` | `CLOACINA_RUN_HISTORY_RETENTION_DAYS` | `0` (off) | A run that finished (Completed, Failed or Cancelled) longer ago than this is deleted, with its task executions, task metadata, contexts, execution events, recovery events, schedule executions and any event push still waiting in `delivery_outbox`. `0` = off. |
| `--outbox-acked-retention-s` | `CLOACINA_OUTBOX_ACKED_RETENTION_S` | `0` (off) | A `delivery_outbox` row is deleted this long after its ack. `0` = off: acked rows are kept for ever. |
| `--retention-interval-s` | `CLOACINA_RETENTION_INTERVAL_S` | `300` | Seconds between passes. The first pass runs one interval after start. |

Where to set them:

- **Helm chart `cloacina-server`:** the values `retention.runHistoryDays`,
  `retention.outboxAckedSeconds` and `retention.intervalSeconds`. The chart
  defaults are the server's (`0`, `3600`, `300`), so an install keeps run
  history for ever until you set `retention.runHistoryDays`.

  ```sh
  helm upgrade cloacina oci://ghcr.io/colliery-io/charts/cloacina-server \
    --reuse-values --set retention.runHistoryDays=30
  ```

- **`cloacinactl server start`:** the same flags, the env vars, or the keys
  `run_history_retention_days`, `outbox_acked_retention_s` and
  `retention_interval_s` of `[server]` in `config.toml`.

What to know:

- **Only finished runs are deleted.** A run that is Pending, Running or Paused is never touched, however old. A context is deleted only when no remaining run or task references it.
- **The delete is final.** After the period, `GET /v1/tenants/{tenant_id}/executions/{exec_id}` and the UI no longer show the run. If you must keep history longer (audit, compliance), raise the period or set it to `0`, or export before it expires.
- **Run history is kept until you set a period.** `--run-history-retention-days` is `0` by default, so no run is deleted. When you set a period, the first passes remove every finished run older than it.
- **Acked outbox rows are kept until you set a period.** `--outbox-acked-retention-s` is `0` by default, so no acked `delivery_outbox` row is deleted and the table grows by one row for each message that its recipient acked: an execution event pushed to a subscriber, an alert, and on an agent fleet each task dispatch, whose row holds the task's context. These rows are queue records, not history. `3600` deletes each one an hour after its ack.
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
3. **Drain, then swap.** Stop sending new traffic at the reverse proxy, let in-flight requests finish, stop the old process, and start the new one against the **same** `DATABASE_URL`. The server **runs any pending schema migrations automatically on startup** (embedded, transaction-safe), so no manual migration step is needed. The start migrates the `public` schema and then the schema of each tenant, one after the other. A new tenant's schema gets its migrations when the tenant is created. The pending migrations of one schema run in one transaction under a PostgreSQL advisory lock: replicas that start together run them once (the others wait, then find nothing to do), and a migration that fails rolls back all of that schema's pending migrations and leaves the schema as it was. If a tenant's schema fails, the server goes on to the other tenants, logs each failure with the tenant's name, and then exits with an error that names every tenant that failed; the schemas that did migrate stay migrated. Fix the named schema and start the server again. A tenant schema with nothing pending costs one short transaction at start.
4. **Gate traffic on readiness.** Wait for `GET /ready` to return 200 before re-enabling proxy traffic — it confirms the DB pool is reachable and no loaded graph crashed on boot.

**Downtime note:** a single-replica server has a brief gap between stop and ready.
For zero-downtime, run multiple replicas against one PostgreSQL database (see
[Horizontal Scaling](../../../explanation/service/horizontal-scaling/)) and
roll them one at a time behind the proxy.

**Upgrade note, cron schedules:** migration 060 adds a unique index on cron schedules: one schedule per workflow, cron expression and timezone (named instances are not in it). Where a schema holds the same schedule more than once, the migration keeps the row with the latest `last_run_at`, moves the fire records (`schedule_executions`) of the other rows to it with one record per fire time, and deletes the other rows. Run history (`workflow_executions`) is not changed.

**Upgrade note, secrets:** a task reads only the secrets its package declares (`secrets(...)` in `#[workflow]`, `@cloaca.workflow_secrets(...)`), through the run's `$secret` bindings. Any other name answers `secret not granted`, on agents and in the server process. A package that reads a secret it does not declare, or one built before `secrets(...)`, stops reading secrets until a new version declares them. A package with no metadata document reads no secret until its compiler describes it. Each refusal by the server writes an audit row with the action `secret.refused`, the secret name and the package. See [Use Secrets in a Workflow](../../workflows/use-secrets/).

**Upgrade note, stored secrets bound to tenant and name:** each stored secret is now bound to its tenant and its name, and each tenant's data key to its tenant (see [Secrets](../../../explanation/service/secrets/)). At its first start with `CLOACINA_SECRET_KEK` set, the new server rewrites the secrets of `public` and of each tenant schema once, one transaction per schema, under the migration lock; later starts find nothing to rewrite. No step is needed from the operator. What to expect:

- With several replicas, a replica of the old version fails every secret read and write of a tenant that the new version has rewritten, with a decryption error, until it stops. A task that runs there and reads a secret fails. To avoid that window, scale the old replicas to zero before the new one starts.
- A secret created or rotated on an old replica during the roll may be written in the old form. A new server reads it only after its next start rewrites it; a rotate that lands after the rewrite of its tenant fails on the old replica.
- A schema whose data key does not unwrap under the KEK is left as it is, with a warning at start that names the tenant. A secret row that does not decrypt is left and named the same way.
- A database error during the rewrite stops the start and names each schema; the schemas already rewritten stay rewritten.
- Rollback: the previous version cannot read rewritten secrets. Restore the pre-upgrade backup from step 1, or create the secrets again.
- An embedded program that stores secrets with `SecretStore` calls `SecretStore::bind_stored_rows(org_id, kek)` once for each `org_id` it uses before it reads them.

**Rollback:** if the new version fails readiness, restart the previous pinned tag
against the same database. If a migration changed the schema incompatibly, restore
the pre-upgrade backup from step 1.
