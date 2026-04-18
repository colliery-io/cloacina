# Compiler + Server Deployment Runbook

Cloacina runs as two paired processes: `cloacina-server` (HTTP API +
reconciler) and `cloacina-compiler` (build worker). They coordinate
through the shared database; no sidecar RPC, no leader election.

This runbook covers how to deploy the pair in three environments —
local bare-metal, Docker Compose, and Kubernetes — plus the config
knobs you'll want to know about and the "a build is stuck" playbook.

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

Image tags pin to `:latest` until the T-0501 release pipeline starts
publishing versioned images to `ghcr.io/colliery-io`. Pin to a
concrete tag in real deployments — `:latest` is a maintenance
hazard, not a contract.

Dockerfiles used by those images live in `deploy/docker/`:

- `server.Dockerfile` — debian-slim runtime, no Rust toolchain.
- `compiler.Dockerfile` — `rust:1.85-bookworm` (toolchain included).

## Kubernetes sketch

Not a Helm chart — that ships with T-0501. The topology is plain
and the YAML is small:

- **`StatefulSet` postgres** — one replica, PVC-backed.
- **`Deployment` cloacina-server** — N replicas, HPA-friendly.
  Stateless from the pod's perspective; all state lives in Postgres.
- **`Deployment` cloacina-compiler** — N replicas. No leader
  election — the atomic claim query handles coordination. Scale
  with queue depth (pending + building).

```yaml
# Server — trimmed, omit probes/resources for brevity
apiVersion: apps/v1
kind: Deployment
metadata: { name: cloacina-server }
spec:
  replicas: 2
  selector: { matchLabels: { app: cloacina-server } }
  template:
    metadata: { labels: { app: cloacina-server } }
    spec:
      containers:
        - name: server
          image: ghcr.io/colliery-io/cloacina-server:latest
          args: ["--bind", "0.0.0.0:8080", "--database-url", "$(DATABASE_URL)"]
          envFrom: [{ secretRef: { name: cloacina-db } }]
          ports: [{ containerPort: 8080 }]
---
# Compiler — structurally identical, no leader election needed
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

| Flag | Default | Notes |
|---|---|---|
| `--poll-interval-ms` | `2000` | How often the build loop polls for pending rows. Lower = snappier pickup, more DB traffic. |
| `--heartbeat-interval-s` | `10` | Heartbeat refresh cadence while a build is in flight. |
| `--stale-threshold-s` | `60` | If `build_claimed_at` is older than this, the sweeper resets the row to `pending`. Should be ≥ 3× `--heartbeat-interval-s`. |
| `--sweep-interval-s` | `30` | How often the sweeper checks for stale rows. |
| `--cargo-flag` | `build --release --lib` | Repeatable. Override for debug builds, custom features, etc. |
| `--bind` | `127.0.0.1:9000` | Local `/health` + `/v1/status` endpoint. |
| `tmp_root` (env / config) | `$CLOACINA_HOME/build-tmp` | Where source archives are unpacked during builds. |

## Playbook: "a build is stuck"

1. **Find the package** — grab its ID from the upload response or
   via `cloacinactl workflow list` (only shows `success` rows) +
   direct DB query if the row is pending/failed.

2. **Inspect** — `cloacinactl package inspect <id>` surfaces
   `build_status` and `build_error`. This hits the tenant route
   `/tenants/{tenant}/workflows/{id}` which bypasses the
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

- **ADR-0004** — Compiler Service Architecture.
- **CLOACI-I-0097** — The initiative that built this.
- **CLOACI-S-0010** — Full spec: schema, state machine, claim
  protocol, heartbeat, sweeper semantics.
