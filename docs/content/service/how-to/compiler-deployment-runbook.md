---
title: "Compiler + Server Deployment Runbook"
description: "Long-form runbook for deploying the cloacina-server and cloacina-compiler pair across bare-metal, Docker Compose, and Kubernetes."
weight: 60
aliases:
  - "/platform/how-to-guides/compiler-deployment-runbook/"

---

# Compiler + Server Deployment Runbook

Cloacina runs as two paired processes: `cloacina-server` (HTTP API +
reconciler) and `cloacina-compiler` (build worker). They coordinate
through the shared database; no sidecar RPC, no leader election.

This runbook covers how to deploy the pair in three environments —
local bare-metal, Docker Compose, and Kubernetes — plus the config
knobs you'll want to know about and the "a build is stuck" playbook.

The short-form how-to is [`running-the-compiler.md`]({{< ref "/service/how-to/running-the-compiler" >}}) — covers the threat model and vendor-curation
posture. This runbook focuses on multi-process deploy mechanics.

## Why two binaries

Historical context is in **ADR-0004** (Compiler Service Architecture).
The short version:

- The reconciler used to shell out to `cargo build` inline. That
  pinned the server to a full Rust toolchain image (~1 GB) and made
  the reconciler's tail latency dependent on compiler availability.
- Splitting the compiler out keeps the server image slim (~50 MB,
  no toolchain) and lets builds scale horizontally — add another
  compiler replica, the atomic claim query (`FOR UPDATE SKIP LOCKED`
  on Postgres, transactional on SQLite) handles coordination.

The build queue lives in `workflow_packages` — the `build_status`
column (`pending` → `building` → `success` | `failed`) is the whole
protocol. Heartbeats refresh `build_claimed_at`; a sweeper resets
rows whose heartbeats go stale.

## Local / bare metal

Two terminal panes, one per binary, both pointing at the same
`DATABASE_URL`:

```bash
# Pane 1 — server
cloacinactl server start \
  --bind 127.0.0.1:8080 \
  --database-url "postgres://cloacina:cloacina@localhost:5432/cloacina"

# Pane 2 — compiler
cloacinactl compiler start \
  --bind 127.0.0.1:9000 \
  --database-url "postgres://cloacina:cloacina@localhost:5432/cloacina"
```

Check the pair is alive:

```bash
cloacinactl status           # daemon + server + compiler side by side
cloacinactl compiler status  # queue depth + last build timestamps
cloacinactl compiler health  # terse up/down
```

The compiler needs `cargo` + `rustc` on `$PATH`. The server does
not — it loads prebuilt `compiled_data` bytes out of the DB.

## Docker Compose

Two templates live under `deploy/docker-compose/`:

- **`cloacina.yml`** — Postgres + server + compiler. Use this for
  any real deployment; it's the production-shape topology.
- **`cloacina-sqlite.yml`** — server + compiler sharing a SQLite DB
  via a named volume. Good for local demos / single-machine dev.
  Don't run this in production.

```bash
# Postgres stack
docker compose -f deploy/docker-compose/cloacina.yml up -d

# Single-machine SQLite stack
docker compose -f deploy/docker-compose/cloacina-sqlite.yml up -d
```

Images are published to `ghcr.io/colliery-io/cloacina-server` and
`ghcr.io/colliery-io/cloacina-compiler` on every release tag (per
I-0111). Pin to a concrete tag in real deployments — `:latest` is
a maintenance hazard, not a contract.

Dockerfiles used by those images live in `deploy/docker/`:

- `server.Dockerfile` — debian-slim runtime, no Rust toolchain.
  Includes the `rdkafka` build deps (T-0609) so Kafka stream
  accumulators work in containerized server deployments.
- `compiler.Dockerfile` — `rust:1.85-bookworm` (toolchain included).

## Kubernetes

The official Helm chart for `cloacina-server` is published as an OCI
artifact at `ghcr.io/colliery-io/charts/cloacina-server` (per T-0610,
which embeds a local Postgres subchart so the chart no longer depends
on the Bitnami `postgresql` chart). See
[Deploying to Kubernetes (Helm)]({{< ref "/service/how-to/deploying-to-kubernetes" >}}) for the chart-driven install path.

The Helm chart currently ships the server only — the compiler is not
yet templated. For a server+compiler topology on K8s, layer your own
`Deployment` for the compiler on top of the chart install. A minimal
sketch:

```yaml
# Server lifecycle is owned by the Helm chart; this is just the
# compiler Deployment to lay on top.
apiVersion: apps/v1
kind: Deployment
metadata: { name: cloacina-compiler }
spec:
  replicas: 2
  selector: { matchLabels: { app: cloacina-compiler } }
  template:
    metadata: { labels: { app: cloacina-compiler } }
    spec:
      containers:
        - name: compiler
          image: ghcr.io/colliery-io/cloacina-compiler:latest
          args: ["--bind", "0.0.0.0:9000", "--database-url", "$(DATABASE_URL)"]
          envFrom: [{ secretRef: { name: cloacina-db } }]
          ports: [{ containerPort: 9000 }]
```

The compiler pods don't need a Service unless you want to scrape
`/v1/status` from outside the cluster — use a single ClusterIP
Service with a round-robin endpoint if so.

## Config knobs

All tunables default to sensible values; adjust only when you have
evidence the defaults are wrong.

### Compiler

| Flag | Default | Notes |
|---|---|---|
| `--poll-interval-ms` | `2000` | How often the build loop polls for pending rows. Lower = snappier pickup, more DB traffic. |
| `--heartbeat-interval-s` | `10` | Heartbeat refresh cadence while a build is in flight. |
| `--stale-threshold-s` | `60` | If `build_claimed_at` is older than this, the sweeper resets the row to `pending`. Should be ≥ 3× `--heartbeat-interval-s`. |
| `--sweep-interval-s` | `30` | How often the sweeper checks for stale rows. |
| `--cargo-flag` | `build --release --lib` | Repeatable. Override for debug builds, custom features, etc. |
| `--bind` | `127.0.0.1:9000` | Local `/health`, `/v1/status`, and `/metrics` endpoint. |
| `tmp_root` (env / config) | `$CLOACINA_HOME/build-tmp` | Where source archives are unpacked during builds. |
| `--log-retention-days` | `7` | Per I-0109; rotates compiler structured logs. |

The compiler hardening flags from I-0104 (`--frozen --offline` cargo
defaults, `setrlimit`-based resource limits, configurable per-build
timeout) are documented in [Running the Compiler]({{< ref "/service/how-to/running-the-compiler" >}}) — they're the security-relevant knobs and live with the
threat model rather than here.

### Server

The server publishes `/metrics` on the same port as the API
(unauthenticated, by design). Key deployment-relevant flags are
documented in [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}}) and the full set is in [CLI Reference]({{< ref "/reference/cli" >}}).

For signature enforcement, the server accepts `--require-signatures`
and `--verification-org-id <UUID>` (per I-0103) to enforce
fail-closed package-signature verification at the canonical load
path. For the operator-side setup, see
[Require Signed Packages]({{< ref "/service/how-to/require-signed-packages" >}}).

For tenant decommissioning, `DELETE /v1/tenants/{name}` performs the
4-step teardown orchestration (revoke keys → evict runner cache →
evict DB cache → drop schema) per I-0106 / T-0581, gated by
`--tenant-deletion-drain-timeout-s`.

## Observability

Both `cloacina-server` and `cloacina-compiler` expose Prometheus
metrics at `GET /metrics`. The full catalog of `cloacina_*` and
`cloacina_compiler_*` metrics — with PromQL examples — lives in
[Metrics Catalog]({{< ref "/reference/metrics-catalog" >}}).

Operationally useful at the compiler tier:

- `cloacina_compiler_builds_total{status}` — build outcome counter.
- `cloacina_compiler_queue_depth{state="queued"|"building"}` — SQL-derived gauge; cannot drift on crash.
- `cloacina_compiler_sweep_resets_total` — stale builds reclaimed.
  Sustained non-zero rate indicates worker crashes or hung builds.

## Playbook: "a build is stuck"

1. **Find the package** — grab its ID from the upload response or
   via `cloacinactl workflow list` (only shows `success` rows) +
   direct DB query if the row is pending/failed.

2. **Inspect** — `cloacinactl package inspect <id>` surfaces
   `build_status` and `build_error`. This hits the tenant route
   `/v1/tenants/{tenant}/workflows/{id}` which bypasses the
   success-only filter for UUIDs.

3. **If `build_status = building`** — a compiler claimed it and
   is working. Tail `cloacina-compiler` logs. If the compiler
   died, the sweeper will reset the row to `pending` after
   `--stale-threshold-s` seconds; a surviving compiler will pick
   it up on the next poll tick.

4. **If `build_status = failed`** — the error is in `build_error`
   (tail 64 KB of stderr). Fix the package, re-upload; the new
   upload supersedes the old row. Retrying in place lands in
   v1.1 as `cloacinactl package retry-build`.

5. **If `build_status = pending` forever** — no compiler is
   running, or all compilers are busy building other packages.
   Check `cloacinactl compiler status` on each replica:
   `pending` + `building` counts tell you queue depth; if
   `pending > 0` and nothing is `building`, you've lost all
   compiler replicas.

## References

- [Running the Compiler]({{< ref "/service/how-to/running-the-compiler" >}}) — short-form how-to with threat model and vendor curation.
- [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}}) — server-specific deployment.
- [Deploying to Kubernetes (Helm)]({{< ref "/service/how-to/deploying-to-kubernetes" >}}) — chart-driven install.
- [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}}) — all `cloacina_*` and `cloacina_compiler_*` metrics.
- **ADR-0004** — Compiler Service Architecture.
- **CLOACI-I-0097** — The initiative that built this.
- **CLOACI-S-0010** — Full spec: schema, state machine, claim protocol, heartbeat, sweeper semantics.
- **CLOACI-I-0104** — Compiler hardening Phase 1 (offline builds, resource limits, timeouts).
- **CLOACI-I-0109** — Compiler `/metrics` endpoint and `--log-retention-days`.
- **CLOACI-I-0111** — Distribution: install script, Docker image, Helm chart.
- **CLOACI-T-0610** — Embedded Postgres subchart in the Helm chart.
