# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.1] - 2026-05-09

### Fixed

- **Cron `schedule_executions` never marked complete → cron_recovery infinite loop** (CLOACI-T-0572) — `process_cron_schedule` created the audit row with `started_at` populated but never called `.complete()`. `cron_recovery::find_lost_executions` selects rows where `completed_at IS NULL` and `started_at < threshold`, so every successful firing was rescheduled on every recovery tick. Workloads with `cron_enable_recovery(true)` (the default) saw ~37x execution amplification (e.g. 906 firings in 6 hours instead of 24). Both the success and failure branches now call `schedule_execution().complete(audit_record_id, Utc::now())`, mirroring the existing trigger-failure pattern. Workaround for older versions: `.cron_enable_recovery(false)` on the runner config.

## [0.6.0] - 2026-05-07

### Added

- **Unified `cloacina::package!()` plugin shell** (CLOACI-I-0102) — single fidius plugin per cdylib emitted by one shell macro for any combination of declared primitives (tasks, workflows, reactors, triggers, computation graphs). Replaces per-macro plugin emission; primitives self-declare via `#[workflow]`, `#[reactor]`, `#[trigger]`, `#[computation_graph]` etc.
- **Trigger FFI bridge for packaged cdylibs** — workflow→trigger subscriptions cross the ABI; cron registration runs through the reconciler at load/unload, closing the gap where packaged cron triggers never fired under `cloacina-server`.
- **Trigger-less computation graphs over FFI** — packaged cdylibs can declare CGs that bind to externally-owned reactors (cross-package fan-out) and run through the unified pipeline.
- **Server-side opt-in signature verification** (CLOACI-I-0103) — `cloacina-server --require-signatures --verification-org-id <UUID>` (or `CLOACINA_VERIFICATION_ORG_ID` env) gates uploads against `package_signatures` matched to a trusted org. Misconfiguration is rejected at boot, not at first upload. Verified + rejected uploads emit structured audit events via `audit::log_package_load_*`. New ADR-0005 codifies the deployment-mode trust model: daemon = high-trust hobbyist, server = enterprise multi-tenant, server is Linux-only.
- **`org_id` column on `package_signatures`** (T-0566) — additive migrations on Postgres + SQLite, prerequisite for scoped signature trust.
- **Reverse-order package unload pipeline** (T-0554 Phase 2) — Python paths route through the unified pipeline; subscribers are unbound before owners are torn down.
- **fidius 0.2.0** across the workspace (T-0546) — adopts `#[optional(since)]` for ABI evolvability.

### Changed

- **Per-macro plugin emission stripped** (T-0549) — the unified shell macro is now the only path. In-tree packaged crates migrated.
- **Manifest cleanup** (T-0551) — `[[triggers]]` and `package_type` removed from `package.toml`. `#[serde(deny_unknown_fields)]` on `CloacinaMetadata` produces friendly migration hints.
- **Reconciler `load_package` precedence-pipeline restructure** (T-0554) — single canonical lifecycle for tasks, workflows, triggers, reactors, computation graphs.
- **Documentation sweep** — tutorials, how-tos, and reference brought in line with the unified package shell; CLI and HTTP API reference rewritten; post-I-0096 ctor references refreshed; I-0102 / pipeline / vtable docs authored.
- **Internal API surface narrowed** (T-0565) — selected `pub` → `pub(crate)` to reduce accidental cross-crate coupling.

### Fixed

- **Server signature verification at upload** (T-0557 Bug 2) — verifies via `package_signer` against the configured trusted org; structured 403 codes (`package_tampered`, `untrusted_signer`, `invalid_signature`, `signature_not_found`).
- **API key escalation guard** (T-0557 Bug 3) — non-admins can no longer create admin keys.
- **Workflow `build_status` accuracy** (T-0557 Bug 4) — package status now reflects compile outcome, not just upload acceptance.
- **Server tests + startup banner + banned-phrase scrubbing** (T-0557 Bugs 1, 6, 7).
- **Python TriggerResult API unified** (T-0557 Bug 5).
- **Reconciler reactor-unload arm + canonical method-index constants** (T-0564).
- **Build/test config drift** (T-0561) — workspace lints, feature flag flags, and angreal harness reorganization aligned.
- **Stale comments + dead daemon manifest plumbing** (T-0562) — runtime strings and code references brought current with closed work.

### Removed

- **Dead code uncovered by post-I-0102 audit** (T-0555, T-0556, T-0563) — orphan modules, dead branches in reconciler `load_package` and scheduler, dead back-compat shims, register_package_triggers shim, unused `PackageLoadView.tasks`, six `todo!()` placeholder signing tests (T-0569; multi-org SaaS scenarios are deferred per ADR-0005 — re-introduce when CLOACI-I-0106 ships).

### Migration notes

- **`package_signatures.org_id`** is added as a nullable column on both backends. Existing rows have `org_id IS NULL` and will not pass verification once `--require-signatures` is enabled — operators upgrading to the verification gate must re-sign packages with `org_id` populated.
- **`package_type` and `[[triggers]]` removed from manifest** — packages still using either fail to load with a friendly migration hint pointing at the unified macro shell.

## [0.5.1] - 2026-04-14

### Changed

- **Pipeline-to-workflow terminology migration** — complete rename across code, DB schema (Diesel migrations for Postgres + SQLite), DAL, models, error messages, metrics, and all tests. `pipeline_executions` table renamed to `workflow_executions`; all `pipeline_*` columns and fields renamed to `workflow_*`. `ExecutionEventType` variants renamed (`PipelineStarted` → `WorkflowStarted`, etc.) with backward-compatible `from_str` parsing.
- **Config builder returns Result** — `DefaultRunnerConfigBuilder::build()` now returns `Result<DefaultRunnerConfig, ConfigError>` with validation for `scheduler_poll_interval >= 10ms` and `cron_max_catchup_executions <= 1000`. Default `cron_max_catchup_executions` capped at 100.
- **Python runner DRY refactor** — 40% reduction in `runner.rs` via extracted `run_event_loop()`, `spawn_runtime()`, `send_and_recv()`, and dict conversion helpers. Fixed `with_schema()` double-construction bug.

### Added

- **Daemon health observability** — Unix domain socket (`daemon.sock`) serves JSON health on connect. Structured log pulse every 60s. `cloacinactl status` command for querying daemon health.
- **Claim ownership guard** — `mark_completed`/`mark_failed` now check `claimed_by` column before updating, preventing race conditions between concurrent runners.
- **TOML config validation** — `deny_unknown_fields` on `CloacinaConfig`, `DaemonSection`, `WatchSection` for early typo detection.

### Fixed

- Integration test `.build()` calls updated for `Result` return type.

## [0.5.0] - 2026-04-10

### Added

- **Computation Graph System** — reactive, event-driven data processing primitive alongside the existing workflow system:
  - `#[computation_graph]` proc macro with compile-time topology validation, cycle detection, and code generation
  - Accumulator trait and built-in types: passthrough, polling, batch, stream (Kafka), and state accumulators
  - Reactor with `WhenAny`/`WhenAll` reaction criteria and `Latest`/`Sequential` input strategies
  - Reactive Scheduler for spawning, supervising, and restarting accumulator/reactor task trees
  - Checkpoint-based crash recovery for accumulators and reactor input cache via DAL
  - Health state machines: `Starting` → `Warming` → `Live` → `Degraded` for both accumulators and reactors
  - Supervisor with exponential backoff (max 5 retries, 1-60s backoff, 60s success reset)
  - Reactor manual commands: `ForceFire`, `FireWith`, `Pause`, `Resume`, `GetState`
- **WebSocket integration** for computation graphs:
  - Accumulator endpoints for pushing events from external producers
  - Reactor endpoints for manual commands and state queries
  - Single-use ticket authentication (`POST /auth/ws-ticket`)
  - Per-endpoint authorization policies scoped to tenant
- **Computation graph packaging** for both Rust and Python:
  - `cdylib` shared library packages with FFI plugin interface via fidius
  - Python computation graph loading via `import_python_computation_graph`
  - Reconciler routing: detects `has_computation_graph()` and routes to reactive scheduler
  - `package.toml` metadata for graph declarations, accumulator config, and reaction mode
- **Kafka stream backend** — `StreamBackend` trait with `KafkaStreamBackend` implementation (KRaft mode, no ZooKeeper)
- **Python computation graph bindings** — `@node`, `@passthrough_accumulator`, `@stream_accumulator`, `@polling_accumulator`, `@batch_accumulator` decorators and `ComputationGraphBuilder`
- **Variable registry** — `CLOACINA_VAR_{NAME}` environment variable convention with `var()`, `var_or()`, and `resolve_template()` for runtime configuration
- **Routing graphs in soak tests** — market maker scenario with enum dispatch routing
- **7 new documentation pages** following Diataxis framework:
  - Tutorial: Python packaged triggers
  - Reference: package manifest schema
  - How-to guides: packaging Python workflows, custom task routing, migrating library to service mode, variable registry
  - Explanation: reactive scheduling architecture
- **REST health endpoints** for computation graphs: `/v1/health/accumulators`, `/v1/health/reactors`, `/v1/health/reactors/{name}`

### Changed

- Documentation site restructured by feature area (workflows, computation graphs, Python, platform)
- Reconciler now routes Python packages through workflow or computation graph paths based on package metadata

### Fixed

- Quick start guide referenced stale version number (0.1.0 → 0.5.0)
- Stale API references in examples and tutorials updated
- CI: shared build cache, libpq-dev installation, retry logic for flaky tutorial tests
- Release pipeline: Python wheel build restored, crate publish ordering fixed, macOS x86_64 wheel dropped

## [0.4.0] - 2026-03-15

Initial public release with workflow orchestration, cron scheduling, multi-tenancy, packaging, Python bindings, and HTTP API server.
