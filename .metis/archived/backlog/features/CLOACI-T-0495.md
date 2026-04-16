---
id: extract-compiler-into-standalone
level: task
title: "Extract compiler into standalone service"
short_code: "CLOACI-T-0495"
created_at: 2026-04-16T12:38:20.875039+00:00
updated_at: 2026-04-16T12:38:20.875039+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Extract compiler into standalone service

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Extract the package compilation step from the reconciler into a standalone service. Today compilation happens **not at upload time but at load time** — the reconciler compiles from source every time it loads a package, and compiled artifacts are never persisted. This means every server restart recompiles everything, and horizontal scaling means N instances independently compiling the same packages.

## Current Chain of Custody

1. **User** runs `cloacinactl package` — creates `.cloacina` bzip2 archive of **source code** + `package.toml`
2. **Upload handler** (`POST /tenants/:id/workflows`, `workflows.rs:36`) — validates archive, reads manifest for name/version, checks for duplicate, **stores raw source archive** in DB. No compilation.
3. **Reconciler** (background, ~30s interval) — diffs DB packages vs in-memory loaded set, calls `load_package()` for new ones
4. **`load_package()`** (`loading.rs:38`) — retrieves source archive from DB, unpacks to temp dir, reads manifest
5. **Compilation** (`compile_source_package()`) — for Rust: runs `cargo build --lib` in subprocess, produces `.dylib`/`.so` in temp dir. For Python: extracts wheel, imports via PyO3 (no compilation).
6. **Registration** — reads compiled cdylib bytes from temp dir, loads via fidius FFI, extracts task/workflow/CG metadata, registers in global registries and ReactiveScheduler
7. **Temp dir dropped** — compiled artifact is gone. Next restart = recompile from source.

### Problems

- **DB only stores source** — compiled artifacts are never persisted. Every restart recompiles.
- **Compilation in reconciler** — the reconciler runs on every server instance. N instances = N independent compilations of the same package.
- **Rust toolchain required on server** — every server instance needs `cargo`, `rustc`, and all build dependencies.
- **Blocking reconcile loop** — `cargo build` can take minutes for complex packages, blocking the reconciler from loading other packages.
- **No build status tracking** — if compilation fails, it fails silently in the reconciler logs. No API to check build status.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (blocks horizontal scaling, causes redundant compilation)

### Business Justification
- **User Value**: Decouples build infrastructure from serving infrastructure. Server instances no longer need Rust toolchains. Compilation happens once, not N times. Restarts are fast (load compiled artifact, not recompile).
- **Effort Estimate**: L

## Acceptance Criteria

## Acceptance Criteria

- [ ] Compiled artifacts (cdylib bytes) are persisted to DB alongside source archive
- [ ] Reconciler loads compiled artifacts directly — no compilation on load
- [ ] Compiler runs as a separate service/process with its own Docker image
- [ ] Compiler polls DB for packages with source but no compiled artifact, builds them
- [ ] Build status tracked in DB (pending, building, success, failed + error message)
- [ ] Upload API returns build status or build ID for async tracking
- [ ] Single-node mode still works (compiler embedded in server for simple deployments)
- [ ] Server Docker image is slim (no Rust toolchain)
- [ ] Compiler Docker image has full Rust toolchain

## Implementation Notes

### Proposed flow
1. Upload stores source archive in DB, sets `build_status = pending`
2. Compiler service polls for `build_status = pending`, claims the build (similar to task claiming)
3. Compiler unpacks, compiles, reads cdylib bytes
4. Compiler stores compiled artifact in DB, sets `build_status = success`
5. Reconciler on any server instance sees the compiled artifact, loads it directly via fidius FFI — no `cargo build`
6. On build failure: `build_status = failed`, error message stored. Reconciler skips failed packages.

### Schema changes
- `workflow_packages` needs: `compiled_data BYTEA NULL`, `build_status TEXT DEFAULT 'pending'`, `build_error TEXT NULL`, `compiled_at TIMESTAMP NULL`
- Or a separate `package_builds` table if we want build history

### Daemon parity
The daemon (`cloacina-daemon`) has the same problem — it runs its own reconciler and compiles from source on every load/restart. The compiler service must work for both:
- **Server mode** (`cloacinactl serve` + `cloacinactl compiler`): both connect to Postgres, compiler polls for pending builds, server reconciler loads compiled artifacts
- **Daemon mode** (`cloacinactl daemon` + `cloacinactl compiler`): both connect to SQLite, same flow but single-machine. The daemon doesn't accept uploads via API — packages are placed on disk and detected by the filesystem watcher — but the compile→persist→load flow is identical.

### Service orchestration
The compiler is not optional — it's a required companion to serve/daemon. The system is a **service pair** that must be co-launched:

| Deployment | Runtime | Compiler | Coordination |
|------------|---------|----------|-------------|
| Local dev (single machine) | `cloacinactl daemon` | `cloacinactl compiler` | SQLite file, both processes on same host |
| Single-node server | `cloacinactl serve` | `cloacinactl compiler` | Postgres, both processes on same host |
| Kubernetes | N x server pods | 1 x compiler pod | Postgres, compiler claims builds via DB |

Even on a single machine, the compiler should be a separate process using the DB for coordination — not embedded in the runtime. This keeps the architecture consistent across deployment modes and ensures the runtime binary never needs a Rust toolchain.

The compiler must be **idempotent and horizontally scalable** — multiple compiler instances can run concurrently. Simple DB queue: poll for `build_status = pending`, atomically claim via `UPDATE ... WHERE build_status = 'pending'`, compile, write artifact + set `build_status = success`. If a compiler dies mid-build, the row stays in `building` — a simple timeout or manual reset handles it. No heartbeat/sweeper machinery needed initially. **Open question**: distinguishing "long build" from "dead build" is hard without additional signal — a 10+ minute Rust build is plausible, so a simple timeout risks killing legitimate builds. Start with the simple queue and manual reset; add compiler heartbeats later if stale builds become an operational problem.

Orchestration options for single-machine:
- `cloacinactl up` — launches both daemon/serve + compiler as child processes (like docker-compose but built-in)
- systemd unit files / launchd plist that start both
- Docker Compose for the server case (already natural)
- For the daemon: a simple process supervisor or just two terminal commands

### Key Questions
- Should the compiler be a long-running service (DB polling) or triggered (message queue / webhook)?
- Cross-compilation: should the compiler target a specific arch, or build for the server's arch?
- Python packages don't need compilation — compiler should detect language and skip
- Should `cloacinactl up` exist as a convenience launcher for the service pair?
- For daemon mode: does the filesystem watcher insert into SQLite (triggering the compiler), or does the compiler also watch the filesystem?

## Status Updates

*To be added during implementation*
