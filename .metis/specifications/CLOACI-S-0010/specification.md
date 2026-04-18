---
id: cloacina-compiler-service-build
level: specification
title: "cloacina-compiler service — build queue, state machine, and lifecycle"
short_code: "CLOACI-S-0010"
created_at: 2026-04-18T01:47:44.527584+00:00
updated_at: 2026-04-18T01:47:44.527584+00:00
parent: CLOACI-I-0097
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# cloacina-compiler Service — Specification

Living spec for the compiler service introduced by I-0097 / ADR-0004. Covers the schema, state machine, claim protocol, build execution, heartbeat + sweeper, and operator surface.

## Schema

Additions to `workflow_packages` (postgres migration 022, sqlite migration 019):

```sql
ALTER TABLE workflow_packages ADD COLUMN compiled_data BYTEA NULL;
ALTER TABLE workflow_packages ADD COLUMN build_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE workflow_packages ADD COLUMN build_error TEXT NULL;
ALTER TABLE workflow_packages ADD COLUMN build_claimed_at TIMESTAMP NULL;
ALTER TABLE workflow_packages ADD COLUMN compiled_at TIMESTAMP NULL;
```

Additional partial index for the queue:

```sql
CREATE INDEX idx_pending_builds
    ON workflow_packages (build_status, build_claimed_at)
    WHERE build_status IN ('pending', 'building') AND NOT superseded;
```

SQLite uses `BLOB` and `TEXT`/`INTEGER` in place of the Postgres native types, same as the other `cloacina` columns.

## State machine

```
                    ┌───────────────┐
                    │    pending    │  ← upload default
                    └──────┬────────┘
                           │ claim (UPDATE ... RETURNING)
                           ▼
                    ┌───────────────┐
              ┌─────│   building    │──────────────┐
              │     └──────┬────────┘              │
              │            │                       │
              ▼            ▼                       ▼
         stale heartbeat  build ok              build fails
              │            │                       │
              │            ▼                       ▼
              │     ┌───────────────┐       ┌───────────────┐
              │     │    success    │       │    failed     │
              │     └───────────────┘       └──────┬────────┘
              │                                    │
              └────────────── sweeper ─────────────┘
                  (reset to pending, clear claimant)
```

Transitions:

| From | To | Trigger |
|---|---|---|
| (none) | `pending` | Package uploaded or new-hash content that couldn't be content-reused |
| `pending` | `success` | Upload-time content-hash reuse (another row with same hash is already `success`) |
| `pending` | `building` | Compiler claims the row |
| `building` | `success` | Build completes; `compiled_data` persisted, `compiled_at` set |
| `building` | `failed` | Build fails; `build_error` set |
| `building` | `pending` | Sweeper detects stale `build_claimed_at`; resets + clears error |
| `failed` | `pending` | Manual `cloacinactl package retry-build <id>` (deferred to v1.1 — T-0517 scope) |

`superseded = true` rows are invisible to the queue but their `compiled_data` stays around to power content-hash reuse.

## Claim protocol

Atomic claim via `UPDATE ... RETURNING` with a small batch size (default 1 per poll tick):

```sql
-- Postgres
UPDATE workflow_packages
SET build_status = 'building',
    build_claimed_at = NOW(),
    build_error = NULL
WHERE id = (
    SELECT id
    FROM workflow_packages
    WHERE build_status = 'pending'
      AND NOT superseded
    ORDER BY uploaded_at
    LIMIT 1
    FOR UPDATE SKIP LOCKED
)
RETURNING id;
```

SQLite (no `SKIP LOCKED`; relies on `WAL` + busy-timeout to serialize claims):

```sql
UPDATE workflow_packages
SET build_status = 'building',
    build_claimed_at = datetime('now'),
    build_error = NULL
WHERE id = (
    SELECT id
    FROM workflow_packages
    WHERE build_status = 'pending'
      AND superseded = 0
    ORDER BY uploaded_at
    LIMIT 1
)
RETURNING id;
```

The compiler polls every `compiler.poll_interval_ms` (default 2000) for pending rows. Multiple compiler instances can run safely — `FOR UPDATE SKIP LOCKED` (Postgres) or per-row busy-lock (SQLite) prevents double-claiming.

## Build execution

For each claimed row:

1. Load source bytes from `workflow_registry.data`.
2. Unpack to a temp dir via `fidius_core::package::unpack_package`.
3. Read `package.toml`.
4. Dispatch on language:
   - **`rust`** (or `mixed`): `cargo build --release --lib` in the unpacked source dir. Read the produced cdylib bytes.
   - **`python`** (pure): no build step; `compiled_data` is set to an empty blob (or repacked source — TBD during T3).
5. On success: `UPDATE workflow_packages SET build_status='success', compiled_data=?, compiled_at=NOW() WHERE id=?`.
6. On failure: `UPDATE workflow_packages SET build_status='failed', build_error=? WHERE id=?`. Error message is stderr tail (max 64KB).
7. In either case, drop the temp dir.

Language is read from `manifest.package.language` (new field; default `"rust"` for back-compat). Cargo build flags are configurable via `compiler.cargo_flags` (default `["build", "--release", "--lib"]`).

## Heartbeat

While step 4 above runs, a background tokio task on the compiler fires every `compiler.heartbeat_interval_s` (default 10) and `UPDATE workflow_packages SET build_claimed_at = NOW() WHERE id = ?`. The background task is cancelled as soon as the build finishes (or fails), so the final `success`/`failed` update beats any further heartbeats.

## Sweeper

Runs in the compiler process (on every compiler instance). Every `compiler.sweep_interval_s` (default 30), it runs:

```sql
UPDATE workflow_packages
SET build_status = 'pending',
    build_claimed_at = NULL,
    build_error = '(reset after stale heartbeat)'
WHERE build_status = 'building'
  AND build_claimed_at < NOW() - INTERVAL '{stale_threshold_s} seconds'
  AND NOT superseded;
```

Default `stale_threshold_s = 60` (6× heartbeat interval). When multiple compilers run, each sweeps independently — a row swept by one is picked up by the next poll.

## Upload-handler flow

When a new row would be inserted with a particular `content_hash`:

1. Look for an existing row with the same `content_hash` and `build_status = 'success'`.
2. If found: insert the new row with `build_status = 'success'`, `compiled_data` copied from the matching row, and `compiled_at = NOW()`. Skip the queue.
3. Else: insert the new row with `build_status = 'pending'`. Compiler picks it up.

The supersede-and-insert transaction from T-0497 wraps both the content-hash lookup and the insert to avoid races.

## Reconciler

Retire the inline `cargo build` from `registry::reconciler::loading`. New behavior:

1. `list_workflows` excludes rows where `build_status != 'success'`.
2. `load_package()` reads `compiled_data` directly and passes the bytes to `fidius_host::open_library_bytes`. No `cargo` subprocess.
3. Rows in `pending` / `building` / `failed` are silently ignored (reconciler retries on the next tick — eventually they reach `success` or the operator sees `failed` via `cloacinactl package inspect`).

`cloacinactl package inspect <ID>` surfaces `build_status` and `build_error` so operators can see why a package isn't running.

## `cloacinactl compiler` noun

Mirrors the `server` noun (ADR-0003 §1, T-0511):

| Verb | Description |
|---|---|
| `start` | `exec`s `cloacina-compiler` with flags passed through. Writes `$home/compiler.pid`. |
| `stop [--force]` | SIGTERM (or SIGKILL with `--force`) via `$home/compiler.pid`. |
| `status` | Rich HTTP status probe against `compiler.local_addr` (default `127.0.0.1:9000`). Reports queue depth, recent builds, heartbeat status. |
| `health` | Terse HTTP `/health` probe. Exit 0 if up, 2 otherwise. |

## Config surface

New `[compiler]` section in `~/.cloacina/config.toml`:

```toml
[compiler]
database_url = "postgres://..."           # may share the server's DB URL
poll_interval_ms = 2000
heartbeat_interval_s = 10
stale_threshold_s = 60
sweep_interval_s = 30
local_addr = "127.0.0.1:9000"             # for the compiler's own /health endpoint
cargo_flags = ["build", "--release", "--lib"]
tmp_root = "~/.cloacina/build-tmp"        # where source is unpacked during builds
```

## `cloacina-compiler` flags

```
cloacina-compiler [--verbose] [--home PATH] [--database-url URL] [--bind ADDR]
                  [--poll-interval-ms N]
                  [--heartbeat-interval-s N] [--stale-threshold-s N]
                  [--sweep-interval-s N]
                  [--cargo-flag <FLAG>]...
```

Flags override the equivalent config keys. `--bind` is the compiler's own health HTTP endpoint.

## Operational runbook (T8)

Docker Compose (local dev, SQLite):
```yaml
services:
  server:
    image: ghcr.io/colliery-software/cloacina-server:0.6.0
    volumes: ["./data:/data"]
    environment:
      DATABASE_URL: "sqlite:///data/cloacina.db"
  compiler:
    image: ghcr.io/colliery-software/cloacina-compiler:0.6.0
    volumes: ["./data:/data", "./target-cache:/cache"]
    environment:
      DATABASE_URL: "sqlite:///data/cloacina.db"
      CLOACINA_COMPILER_TMP_ROOT: "/cache"
```

Kubernetes sketch (Postgres): two deployments, one StatefulSet for Postgres. N compiler replicas scale without coordination.

## Open items

- `package retry-build <id>` verb — useful for re-queueing `failed` rows. Deferred; simple to add under the existing `package` noun.
- Cross-compilation (multi-arch). Out of scope; a compiler builds for its own arch.
- Build artifact size caps. Cdylibs for production workloads can hit tens of MB. Should the DB use `LARGE OBJECT` storage (Postgres `lo_*`) or filesystem offloading? Revisit if artifact size becomes a bottleneck.
- Build log capture — currently only stderr tail (64KB) is persisted in `build_error`. Full log streaming to a future `/v1/builds/{id}/log` endpoint is a follow-up.

---

*Living document. Update as implementation lands.*
