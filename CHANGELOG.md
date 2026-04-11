# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
