---
id: 004-cloacina-compiler-service-separate
level: adr
title: "cloacina-compiler service — separate binary, DB queue, heartbeat sweeper"
number: 4
short_code: "CLOACI-A-0004"
created_at: 2026-04-18T01:47:43.344919+00:00
updated_at: 2026-04-18T01:49:43.169389+00:00
decision_date:
decision_maker: Dylan Storey
parent:
archived: false

tags:
  - "#adr"
  - "#phase/discussion"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-0004: cloacina-compiler service — separate binary, DB queue, heartbeat sweeper

## Context

Compilation today lives in the reconciler loop: every runtime instance runs `cargo build --lib` on the unpacked source of every package it loads, on every restart. This blocks horizontal scaling (N runtime pods = N redundant builds), forces the Rust toolchain into every runtime container (inflating `cloacina-server` far past a minimal HTTP API image), and makes reconcile latency unpredictable when packages change.

Initiative I-0097 extracts compilation as a standalone service. This ADR locks the architectural calls.

## Decision

### 1. Separate binary: `cloacina-compiler`

Same pattern as `cloacina-server` (I-0098). New crate `crates/cloacina-compiler` with its own `main.rs` plus a library that can also be linked into tests. `cloacinactl compiler` is a nested noun with `start`/`stop`/`status`/`health` verbs; `start` `exec`s `cloacina-compiler`.

The `cloacina-server` Docker image sheds the Rust toolchain. The `cloacina-compiler` image keeps it. This is the real reason the split matters: ship a slim API image + a dedicated build image, scale each independently.

### 2. DB-backed queue, no broker

The existing `workflow_packages` table gains queue columns (`build_status`, `build_error`, `compiled_data`, `build_claimed_at`, `compiled_at`). Upload sets `pending`. Compiler claims via `UPDATE ... WHERE build_status = 'pending' AND NOT superseded ... RETURNING` (atomic across concurrent compilers). No RabbitMQ / Redis dependency.

Reconcile latency (~30s) is acceptable because uploads are operator-initiated events, not high-throughput traffic.

### 3. Heartbeat-based dead-build recovery (not timeout)

A plain timeout is a bad fit because legitimate Rust builds can legitimately take 10+ minutes and a naive reset would kill them. Instead:

- While a row is in `building`, the compiler updates `build_claimed_at` every `compiler.heartbeat_interval_s` (default **10s**).
- A sweeper (runs on any `cloacina-compiler` instance, or can be colocated with the server) resets rows whose `build_claimed_at` is older than `compiler.stale_threshold_s` (default **60s**) back to `pending`.

Both knobs live in `~/.cloacina/config.toml` under a new `[compiler]` section. Rationale: the threshold is ~6× the heartbeat, giving plenty of headroom for legitimate GC pauses or momentary DB contention, and it stays honest about what "dead" means.

### 4. Polyglot packages pass through the compiler

A `.cloacina` may contain Rust tasks, Python tasks, or both. The compiler runs whatever the manifest declares — `cargo build` for Rust code, no-op for pure-Python. There is no upload-handler shortcut for Python-only packages; the compiler is always the single chokepoint. This keeps the state machine uniform and makes mixed-language packages work naturally.

### 5. Content-hash artifact reuse

The supersede-and-insert flow from T-0497 already guarantees byte-identical re-uploads return the existing row unchanged. On *different*-hash uploads where another row happens to have the same content hash with `build_status = success`, the upload handler copies that row's `compiled_data` into the new row and marks it `success` directly, skipping the queue. Rebuilds only run when a package genuinely needs building.

### 6. Local `cloacinactl package build` stays

The CLI's `package build <DIR>` command continues to run a local `cargo build` for smoke testing ("does this compile on my machine?"). It is **not** a prerequisite for `package publish` once I-0097 lands — the server will build. Keeping the local command preserves the developer iteration loop.

### 7. Orchestration by documentation, not tooling

No `cloacinactl up` convenience launcher in v1. We ship a Docker Compose template and a "run both processes" runbook. If the two-binary pattern proves friction-worthy, revisit post-1.0.

## Consequences

**Positive:**
- `cloacina-server` image no longer bundles the Rust toolchain. Ship a slim HTTP image alongside a build-heavy compiler image.
- N server/daemon replicas no longer each re-compile on restart. Compile once, load everywhere.
- Build failures surface structurally (`build_status = failed` + `build_error`) rather than as log lines in the reconcile loop.
- Compiler horizontally scales trivially — atomic claim via UPDATE RETURNING.

**Negative:**
- Two binaries to deploy instead of one (mitigated by Docker Compose / K8s manifests in T8).
- First-upload latency gains a build-queue hop (~30s reconcile + build time before the package is executable). Acceptable trade-off vs. blocking the HTTP upload request on a multi-minute `cargo build`.
- Stuck builds (compiler crashes mid-build) recover after `stale_threshold_s`, not instantly. This is the cost of avoiding false positives.

**Neutral:**
- Python-only packages now technically require a running compiler to transition to `success`, but there was never a "Python runtime without a compiler" deployment model — packages ride mixed — so no real regression.

## Alternatives Considered

- **Status quo — compile in reconciler.** Rejected: blocks scaling, forces toolchain everywhere.
- **Upload-time synchronous compile.** Blocks HTTP request for minutes. Bad UX, doesn't scale.
- **Message broker (RabbitMQ / Redis streams).** Adds infrastructure dep for a problem polling solves. Build latency of ~30s is acceptable for operator-driven uploads.
- **Simple-timeout dead-build reset.** Kills legitimate long builds. Heartbeat-based sweeper is only marginally more complex and produces correct behavior.
- **Separate per-language upload paths.** Complicates the state machine for no benefit — polyglot packages exist and the compiler is the right place to handle them.
- **`cloacinactl up` launcher.** Feature creep for v1. Docker Compose does the same job for the platforms most operators use.

## Status

Draft — pending human decision.
