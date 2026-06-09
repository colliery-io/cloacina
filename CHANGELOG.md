# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-06-09

### Added

- **Execution-agent fleet** (CLOACI-I-0114 / CLOACI-I-0115) — a horizontally scalable, database-less execution tier. Remote `cloacina-agent` workers pull ready work from the server, run it, and report results back over HTTP, so task execution scales out independently of the scheduler and without granting workers direct database access. Ships with a `cloacina-agent` Helm chart (server + compiler + N agents), in-flight reclaim on agent loss, and a fleet soak harness (roster/outbox/drift). See [Execution-Agent Fleet](https://cloacina.colliery.io/platform/explanation/execution-agent-fleet/) and [Deploy an Execution-Agent Fleet](https://cloacina.colliery.io/platform/how-to-guides/deploy-an-execution-agent-fleet/).
- **Server-level default executor** (CLOACI-T-0640) — execution topology is now a single deployment knob (Airflow `[core] executor` style). The preferred surface is `[server].default_executor` in `~/.cloacina/config.toml`; override it ad-hoc with `--default-executor` or the `CLOACINA_DEFAULT_EXECUTOR` env var (precedence: explicit CLI/env > `config.toml` > built-in `default`). Set it to `fleet` to route all work to the execution-agent fleet. The configured key is **hard-matched** against the registered executors at startup — an unknown key fails fast with an error listing the valid keys, never a silent fallback.

### Security

- **`cloacina-compiler` Phase 1 hardening** (CLOACI-I-0104) — bounded-cost mitigations for malicious `build.rs` source. `cargo build` now runs with `--frozen --offline` by default against an operator-curated `CARGO_HOME` (`--vendor-dir` / `CLOACINA_COMPILER_VENDOR_DIR`); packages whose deps aren't vendored fail fast with a structured rejection naming the missing crates. Builds are bounded by a wall-clock timeout (`--build-timeout-s`, default 600s) — overruns are SIGKILL'd and reclaimed by the existing stale-build sweeper. On Linux, four kernel-enforced rlimits are applied to the cargo subprocess via `setrlimit` in a `pre_exec` hook: CPU-seconds (tracks `--build-timeout-s`), virtual address space (`--build-rlimit-mem`, default `4G`, accepts `K`/`M`/`G` suffixes), open file descriptors (`--build-rlimit-files`, default 1024), and user processes (`--build-rlimit-procs`, default 256). Every build emits a `compiler.build.started` and `compiler.build.finished` structured audit event via `tracing`, including build-claim id, `Cargo.toml` / `Cargo.lock` SHA-256, outcome, cargo exit code/signal, wall-clock duration, and a process-wide `compiler_instance_id`. See [Run cloacina-compiler in Production](https://cloacina.colliery.io/platform/how-to-guides/running-the-compiler/) for the deployment runbook. Phase 2 (CLOACI-I-0105) adds a bubblewrap + landlock sandbox.

  **Operator action required** for existing deployments:

  - Run `cargo vendor` against your known-good source tree and point `--vendor-dir` at the result, or pre-populate `~/.cargo` on the compiler host with the deps your authors submit. Packages that previously succeeded by fetching deps online will now fail until vendored.
  - Verify the compiler runs under an unprivileged UID with a database role limited to `workflow_packages` (`SELECT`/`UPDATE`) — a shared admin DB role would be a privilege-escalation path for any malicious `build.rs`.
  - Tune `--build-rlimit-mem` if your release builds peak above 4 GiB (large generic-heavy crates can).

### Changed (breaking)

- **Computation graphs and reactors are now declared separately** (CLOACI-I-0101) — the bundled `#[computation_graph(react = when_any(...), graph = { ... })]` form is removed. A reactor is now its own top-level primitive declared with `#[reactor(...)]`, and computation graphs reference it by string name via `trigger = reactor("name")`. Trigger-less graphs (no `trigger =` clause) are first-class and can be invoked from a workflow task with `#[task(invokes = computation_graph("name"))]`. Multiple computation graphs may now subscribe to the same reactor. Python mirrors the split: `@cloaca.reactor(...)` declares the reactor class; `ComputationGraphBuilder(..., reactor=ReactorClass, ...)` binds the graph; `@cloaca.task(invokes=graph_builder)` wraps a trigger-less graph as a workflow task. No deprecation window; rewrite required.

  **Before (Rust):**

  ```rust
  #[cloacina_macros::computation_graph(
      react = when_any(orderbook),
      graph = { ingest(orderbook) -> output },
  )]
  pub mod pricing_pipeline { /* ... */ }
  ```

  **After (Rust):**

  ```rust
  #[cloacina_macros::reactor(
      name = "pricing_pipeline_reactor",
      accumulators = [orderbook],
      criteria = when_any(orderbook),
  )]
  pub struct PricingPipelineReactor;

  #[cloacina_macros::computation_graph(
      trigger = reactor("pricing_pipeline_reactor"),
      graph = { ingest(orderbook) -> output },
  )]
  pub mod pricing_pipeline { /* ... */ }
  ```

  **Before (Python):**

  ```python
  with cloaca.ComputationGraphBuilder(
      "pricing_pipeline",
      react={"mode": "when_any", "accumulators": ["orderbook"]},
      graph={...},
  ) as builder:
      ...
  ```

  **After (Python):**

  ```python
  @cloaca.reactor(
      name="pricing_pipeline_reactor",
      accumulators=["orderbook"],
      mode="when_any",
  )
  class PricingPipelineReactor:
      pass

  with cloaca.ComputationGraphBuilder(
      "pricing_pipeline",
      reactor=PricingPipelineReactor,
      graph={...},
  ) as builder:
      ...
  ```

  See [CLOACI-S-0011](https://github.com/colliery-io/cloacina/blob/main/.metis/specs/CLOACI-S-0011.md) for the primitive nomenclature and the [Computation Graph in a Workflow Task](https://cloacina.colliery.io/computation-graphs/how-to-guides/computation-graph-in-workflow/) how-to for the new embedded-CG pattern.

- **Glob-based task routing removed** (CLOACI-T-0640) — the per-task routing surface is gone. `Router`, `RoutingConfig`, and `RoutingRule` are removed from the public prelude (`cloacina::dispatcher`), and `cloacina-server` no longer accepts `--route` / `CLOACINA_FLEET_ROUTES`. The dispatcher now sends every task to the single configured default executor (see **Added → Server-level default executor**); choosing which node or compute a task lands on is an executor-internal concern, not a scheduler/dispatcher one. **Migration:** replace any `--route "**=fleet"` / `CLOACINA_FLEET_ROUTES` usage with `--default-executor fleet` (or `[server].default_executor = "fleet"`); library consumers that referenced `RoutingConfig`/`RoutingRule`/`Router` should remove them and configure the default executor via `DefaultRunnerConfig::default_executor`.

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
