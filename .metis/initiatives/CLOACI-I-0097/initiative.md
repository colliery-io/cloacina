---
id: compiler-service-extract-build
level: initiative
title: "Compiler Service — Extract Build Pipeline, Cache Artifacts, Service Orchestration"
short_code: "CLOACI-I-0097"
created_at: 2026-04-16T15:31:44.386277+00:00
updated_at: 2026-04-19T00:23:22.736081+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: compiler-service-extract-build
---

# Compiler Service — Extract Build Pipeline, Cache Artifacts, Service Orchestration Initiative

## Context

Today the compilation step lives inside the reconciler — every server/daemon instance compiles packages from source on every load. Compiled artifacts are never persisted. This means:
- Every restart recompiles all packages
- N server instances = N redundant compilations of the same package
- Every runtime instance requires a full Rust toolchain
- The reconcile loop blocks on `cargo build` (minutes for complex packages)
- No build status visibility — failures are silent log lines

This blocks horizontal scaling (S-0008), inflates Docker images, and creates a poor operator experience. The compiler needs to be extracted as a standalone service that builds once, persists artifacts, and lets all runtime instances load pre-compiled binaries.

### Current chain of custody
1. User runs `cloacinactl package` — creates `.cloacina` bzip2 archive of **source code**
2. Upload handler stores **raw source** in DB. No compilation.
3. Reconciler (background, ~30s) detects new package, calls `load_package()`
4. `load_package()` unpacks source to temp dir, runs `cargo build --lib`, reads cdylib bytes
5. Loads cdylib via fidius FFI, registers tasks/workflows/CGs
6. Temp dir dropped — compiled artifact gone. Next restart = full recompile.

## Goals & Non-Goals

**Goals:**
- Extract compilation into a standalone `cloacinactl compiler` service
- Persist compiled artifacts (cdylib bytes) in the DB
- Reconciler loads pre-compiled artifacts — no `cargo build` on runtime instances
- Compiler is idempotent and horizontally scalable (simple DB queue, multiple instances OK)
- Works for both server mode (Postgres) and daemon mode (SQLite)
- Runtime binaries (server/daemon) no longer require Rust toolchain
- Define the service pair model: runtime + compiler always co-launched

**Non-Goals:**
- Cross-compilation (compiler builds for its own arch; multi-arch is a future concern)
- Distributed build cache (sccache, etc.) — simple DB persistence is sufficient
- Compiler heartbeats or sophisticated stale-build detection (start simple, add if needed)

## Architecture

### Service model

The system becomes a **service pair** — runtime + compiler — coordinated through the DB:

| Deployment | Runtime | Compiler | DB |
|------------|---------|----------|-----|
| Local dev | `cloacinactl daemon` | `cloacinactl compiler` | SQLite |
| Single-node server | `cloacinactl serve` | `cloacinactl compiler` | Postgres |
| Kubernetes | N x server pods | M x compiler pods | Postgres |

Even on a single machine, the compiler runs as a separate process. Same architecture everywhere — only the DB backend and deployment wrapper change.

### Build queue

Simple DB-backed queue, no message broker:
1. Upload stores source archive, sets `build_status = pending`
2. Compiler polls for `pending`, atomically claims via `UPDATE ... WHERE build_status = 'pending'`
3. Compiler unpacks, builds, reads cdylib bytes
4. Compiler writes compiled artifact to DB, sets `build_status = success`
5. On failure: sets `build_status = failed` + error message
6. Reconciler on any runtime instance sees `build_status = success`, loads compiled artifact directly — no `cargo build`

Multiple compiler instances can run concurrently — DB atomicity prevents double-claiming. If a compiler dies mid-build, the row stays in `building`. Start with manual reset; add a timeout-based reset or heartbeats later if stale builds become an operational problem. (Distinguishing "long build" from "dead build" is hard — a 10+ minute Rust build is plausible, so a simple timeout risks killing legitimate builds. Observe real patterns first.)

Python packages skip compilation — compiler detects language from manifest and marks them `success` immediately.

### Schema changes

```sql
ALTER TABLE workflow_packages ADD COLUMN compiled_data BYTEA NULL;
ALTER TABLE workflow_packages ADD COLUMN build_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE workflow_packages ADD COLUMN build_error TEXT NULL;
ALTER TABLE workflow_packages ADD COLUMN build_claimed_at TIMESTAMP NULL;
ALTER TABLE workflow_packages ADD COLUMN compiled_at TIMESTAMP NULL;
```

### Reconciler changes

Current: `load_package()` → unpack → compile → load FFI
After: `load_package()` → check `build_status = success` → read `compiled_data` → load FFI. Skip packages where `build_status != success`.

### Docker images

- **`cloacina-server`**: slim — no Rust toolchain. Contains `cloacinactl` binary only.
- **`cloacina-compiler`**: full Rust toolchain + cargo + build deps. Contains `cloacinactl` binary.
- **`cloacina-daemon`**: same as server (slim). Paired with compiler on same host.

## Alternatives Considered

**Embed compiler in runtime (status quo)**: Simpler deployment but blocks horizontal scaling, wastes compute on redundant builds, requires toolchain on every instance. Rejected.

**Message queue for build triggers (RabbitMQ, Redis streams)**: More responsive than polling but adds an infrastructure dependency. The DB polling approach is sufficient — build latency of ~30s (one reconcile cycle) is acceptable since packages are uploaded infrequently. Rejected for now.

**Upload-time compilation (compile in the HTTP handler)**: Blocks the upload request for minutes. Bad UX. Also doesn't solve the "multiple instances" problem. Rejected.

## Design decisions

Locked in ADR-0004.

1. **Separate binary `cloacina-compiler`** (mirrors `cloacina-server` from I-0098). `cloacinactl compiler` is a nested noun with start/stop/status/health verbs that `exec`s the binary.
2. **Heartbeat-based dead-build reset** (not a timeout). The compiler updates `build_claimed_at` every `compiler.heartbeat_interval_s` (default 10) while a build is in flight; a sweeper resets rows whose heartbeat is older than `compiler.stale_threshold_s` (default 60). Both configurable in `~/.cloacina/config.toml`.
3. **Polyglot packages flow through the compiler.** A package may contain Rust tasks, Python tasks, or both. The compiler runs whatever build steps the manifest declares (Rust → `cargo build`; pure-Python → no-op, mark success). No upload-handler special-case.
4. **Content-hash dedup reuses artifacts.** The supersede-and-insert flow from T-0497 already guarantees identical byte uploads short-circuit to the existing row. On a different-hash upload, the compiler may still reuse an earlier `compiled_data` entry if a row with matching content_hash already has `build_status = success`.
5. **Local `cloacinactl package build` stays.** Keeps "does this compile?" smoke-testing on the workstation, independent of having a server.
6. **Service-pair orchestration by documentation, not tooling.** Docker Compose template + "run both binaries" runbook. No `cloacinactl up` launcher in v1.

## Implementation Plan

Nine tasks, dep-stacked (1 → 2 → 3 → 4/5 parallel → 6 → 7 → 8 → 9):

1. **Schema + DAL** — migration adding `compiled_data`, `build_status`, `build_error`, `build_claimed_at`, `compiled_at`. Filter helpers for the queue (`pending`, `NOT superseded`, claim-by-UPDATE ... RETURNING).
2. **Extract `cloacina-compiler` binary** — new crate with its own `main` + lib; shares Diesel types with `cloacina`.
3. **Compiler build loop** — claim → unpack → `cargo build` (language-dispatch per manifest) → persist cdylib → mark success/fail.
4. **Heartbeat + stale-build sweeper** — compiler updates `build_claimed_at` while building; sweeper resets rows whose heartbeat is stale. Config-driven thresholds.
5. **Upload handler — enqueue on upload** — new rows default to `pending`. Short-circuit: if another row has the same `content_hash` and `build_status = success`, copy its `compiled_data` into the new row and skip the queue.
6. **Reconciler refactor** — load from `compiled_data`; skip rows where `build_status != success`. Retire inline `cargo build` from the reconciler.
7. **`cloacinactl compiler` noun** — start/stop/status/health verbs mirroring `server`; execs `cloacina-compiler`.
8. **Operational — Docker Compose template + docs** — two-process layout (server + compiler), local SQLite and Postgres variants, K8s example.
9. **Integration tests** — upload → queue pending → compiler picks up → build succeeds → reconciler loads → execution runs. Also exercises the failed-build path and the stale-build reset path.
