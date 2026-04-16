---
id: compiler-service-extract-build
level: initiative
title: "Compiler Service — Extract Build Pipeline, Cache Artifacts, Service Orchestration"
short_code: "CLOACI-I-0097"
created_at: 2026-04-16T15:31:44.386277+00:00
updated_at: 2026-04-16T15:31:44.386277+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: compiler-service-extract-build
---

# Compiler Service — Extract Build Pipeline, Cache Artifacts, Service Orchestration Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

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

## Implementation Plan

To be decomposed into tasks after design review. Rough phases:
1. Schema migration — add compiled_data + build_status columns
2. Compiler service — `cloacinactl compiler` subcommand, DB queue poll loop, build execution
3. Artifact persistence — compiler writes cdylib bytes to DB on success
4. Reconciler refactor — load from `compiled_data` instead of compiling, skip non-ready packages
5. Orchestration — `cloacinactl up` convenience launcher, Docker Compose template, docs
6. Integration tests — upload → compile → load → execute end-to-end
