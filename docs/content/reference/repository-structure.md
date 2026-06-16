---
title: "Repository Structure"
description: "Overview of the Cloacina repository organization, crate architecture, and the relationship between workspace members."
weight: 10
aliases:
  - "/platform/reference/repository-structure/"

---

# Repository Structure

The Cloacina workspace ships eleven Rust crates plus a Python wheel built on top of one of them. This page is a map of what lives where and which crate to look at for a given concern.

## Directory layout

```
cloacina/
  crates/                          # Rust workspace members (11)
    cloacina/                      # Core engine: tasks, workflows, runner, DAL, executor, registry
    cloacina-build/                # build-script helper (linker flags, rpath fixups)
    cloacina-compiler/             # Standalone build worker (cargo build for .cloacina packages)
    cloacina-computation-graph/    # Computation-graph runtime: reactors, accumulators, scheduler
    cloacina-macros/               # Proc macros: #[task], #[workflow], #[trigger], #[reactor], #[computation_graph], #[accumulator]
    cloacina-python/               # PyO3 wheel runtime (the `cloaca` Python module)
    cloacina-server/               # HTTP API binary (cloacina-server)
    cloacina-testing/              # Test utilities: TestRunner, BoundaryEmitter
    cloacina-workflow/             # Slim workflow-author crate (Trigger trait, RetryPolicy, types)
    cloacina-workflow-plugin/      # FFI vtable for .cloacina cdylib plugins (host-side trait + magic-byte ABI)
    cloacinactl/                   # CLI binary (cloacinactl)

  charts/                          # Helm chart for cloacina-server (T-0610 — local Postgres subchart embedded)
  deploy/                          # Deploy templates (docker-compose, k8s)
  docker/                          # Dockerfiles
  docs/                            # Hugo documentation site
  examples/                        # Runnable examples (see below)
  scripts/                         # Maintenance scripts
  tests/                           # Integration tests
    python/                        # Python pytest scenarios

  Cargo.toml                       # Workspace manifest
  Dockerfile                       # cloacina-server runtime image (rdkafka deps included per T-0609)
  install.sh                       # One-liner install script (I-0111)
  pyproject.toml                   # Python wheel manifest (maturin-driven)
  .angreal/                        # Task automation (build, test, demos, lint, helm, services, ...)
  .github/                         # CI workflows
  .metis/                          # Metis planning artifacts (vision, initiatives, ADRs, specs, tasks)
```

## Crates

### Core runtime

#### `cloacina`

The core workflow orchestration library. Provides:

- **Task system:** `Task` trait, retry policies, deferred-task primitives (`TaskHandle::defer_until`).
- **Workflow engine:** DAG construction, validation, execution sequencing.
- **Context management:** type-safe data passing between tasks with atomic finalize (post-CLOACI-I-0110 `complete_task_transaction`).
- **Persistence:** PostgreSQL and SQLite backends via Diesel; runtime-detected (no compile-time backend switch).
- **Runner:** `DefaultRunner` — high-level orchestration; `DefaultRunnerBuilder` for configured construction.
- **Multi-tenancy:** schema-isolated execution with fail-closed `SET search_path` enforcement (I-0106).
- **Cron scheduling:** at-least-once cron firings with recovery via the heartbeat sweeper (T-0502: `RecoveryManager` removed).
- **Registry:** inventory-driven workflow registration (post-I-0096; replaces the pre-T-0509 `#[ctor]` path).
- **Computation-graph integration:** re-exports from `cloacina-computation-graph` and `cloacina-workflow-plugin` so users have a single dependency.

**Key modules:** `src/{task,workflow,context,dal,executor,runner,registry,computation_graph,cron_trigger_scheduler,database,trigger,security}/`

**Cargo features:** `postgres` (default), `sqlite` (default), `macros` (default), `kafka` (default), `auth`, `telemetry`, `extension-module` (for the PyO3 wheel build).

#### `cloacina-workflow`

The thin author-facing surface — types and traits that workflow authors need without pulling the full runtime. Houses the `Trigger` trait, `TriggerResult`, `RetryPolicy`, error types, and a feature gate (`packaged`) for cdylib-mode plugin authoring.

#### `cloacina-computation-graph`

Computation-graph runtime: `Reactor`, `Accumulator` (with `passthrough`, `stream`, `polling`, `batch`, `state` variants), `ComputationGraphScheduler`, `InputCache`, boundary types, channel-backed firing path.

#### `cloacina-macros`

Procedural macros for declarative authoring:

- `#[task]` — declare a task function (supports `invokes = computation_graph("name")` for embedded CG-in-workflow per I-0101).
- `#[workflow]` — declare a workflow module (supports `triggers = [...]` per I-0102 unified shell).
- `#[trigger]` — declare a custom poll/event trigger (supports `upstream = reactor("name")` per I-0100).
- `#[reactor]` — declare a reactor as a first-class primitive (I-0101 split from `#[computation_graph]`).
- `#[computation_graph]` — declare a graph with `trigger = reactor("name")` (standalone) or used via `invokes = computation_graph(...)` from a task (embedded).
- `#[passthrough_accumulator]`, `#[stream_accumulator]`, `#[polling_accumulator]`, `#[batch_accumulator]`, `#[state_accumulator]` — accumulator-kind macros.

### Packaging & distribution

#### `cloacina-workflow-plugin`

The FFI vtable trait crate. `.cloacina` packages are cdylibs that expose a fixed nine-method vtable (per CLOACI-I-0102) dispatched at runtime by `fidius`. The host (`cloacina`) loads them via this trait without symbol-name knowledge of the user's code.

#### `cloacina-build`

Build-script helper for cdylib packaging — sets correct linker flags and rpath for the macOS Python wheel link order. Used as a `[build-dependencies]` entry by packaged workflows.

#### `cloacina-compiler`

Standalone build worker. Polls the database for pending package builds (`workflow_packages.build_status = 'pending'`), runs `cargo build --frozen --offline` against a curated vendored registry (CLOACI-I-0104 hardening), and persists the resulting `compiled_data` bytes. Coordinated with the server via atomic claim queries (`FOR UPDATE SKIP LOCKED` on Postgres). Exposes its own `/health`, `/v1/status`, and `/metrics` endpoints (I-0109).

### Service surface

#### `cloacina-server`

The HTTP API binary. Backed by PostgreSQL. Multi-tenant by default with schema isolation. Exposes the REST API documented in [HTTP API Reference]({{< ref "http-api" >}}) under `/v1/*` plus public `/health`, `/ready`, `/metrics`. The server image (`ghcr.io/colliery-io/cloacina-server`) ships per I-0111 / T-0610.

#### `cloacinactl`

The CLI binary. Noun-verb command structure (per CLOACI-I-0098 / T-0538): `cloacinactl <noun> <verb>` with nouns `compiler`, `daemon`, `execution`, `graph`, `key`, `package`, `server`, `tenant`, `trigger`, `workflow`; singletons `status`, `config`, `admin`, `completions`. Profile model via `~/.cloacina/config.toml` (ADR-0003). See [CLI Reference]({{< ref "cli" >}}).

### Python

#### `cloacina-python`

The PyO3 wheel runtime — isolated as its own crate per CLOACI-T-0529 / T-0532 so binaries that don't execute Python don't transitively link `pyo3`. The `cloaca` Python module is the user-facing surface; it wraps the `cloacina` core engine with Pythonic ergonomics:

- `@cloaca.task`, `@cloaca.trigger`, `@cloaca.reactor`, `@cloaca.node`, `@cloaca.passthrough_accumulator`, `@cloaca.stream_accumulator`, `@cloaca.polling_accumulator`, `@cloaca.batch_accumulator` decorators.
- `cloaca.WorkflowBuilder`, `cloaca.ComputationGraphBuilder` context managers.
- `cloaca.DefaultRunner`, `cloaca.DatabaseAdmin`.
- `cloaca.var()` / `cloaca.var_or()` for the `CLOACINA_VAR_*` runtime-variable surface.

The wheel is built with `maturin develop --features "postgres,sqlite,macros,extension-module"` (development) or `maturin build --release ...` (publishing).

### Test utilities

#### `cloacina-testing`

Test fixtures: `TestRunner` for no-DB workflow testing, `BoundaryEmitter` for computation-graph simulation. Used by integration tests in `cloacina/tests/`, `cloacina-computation-graph/tests/`, and downstream workflow projects via `[dev-dependencies]`.

## Examples

### `examples/tutorials/`

Progressive learning path. Each track mirrors a documented tutorial under `docs/content/`.

- **`workflows/library/`** — Rust embedded-mode tutorials (numbered 01-06 against the canonical docs).
- **`computation-graphs/library/`** — Rust CG tutorials (07-10).
- **`python/{workflows,computation-graphs}/`** — Python tutorials (covers 01-08 for workflows and 09-11 for CG).

Run any tutorial via `angreal demos:tutorials:rust:NN` or `angreal demos:tutorials:python:NN`.

### `examples/features/`

Feature showcases — each demonstrates one capability end-to-end. Run via `angreal demos:features:<name>`.

**Workflow features (`examples/features/workflows/`):**

| Directory | Feature |
|---|---|
| `complex-dag/` | Complex DAG topologies |
| `conditional-retries/` | `retry_condition` matchers (transient, all, never, custom) |
| `cron-scheduling/` | Scheduled workflow execution |
| `deferred-tasks/` | `TaskHandle::defer_until` patterns |
| `event-triggers/` | Custom `Trigger` trait implementations |
| `multi-tenant/` | Tenant isolation |
| `packaged-triggers/` | Trigger declarations inside packaged workflows |
| `packaged-workflows/` | Distributable `.cloacina` workflow packages |
| `per-tenant-credentials/` | Per-tenant DB credentials (defense in depth) |
| `python-workflow/` | Python workflow end-to-end |
| `registry-execution/` | Registry-driven dynamic loading |
| `simple-packaged/` | Minimal packaged workflow (smallest reproducer) |
| `validation-failures/` | Macro-validation failure shapes |

**Computation-graph features (`examples/features/computation-graphs/`):**

| Directory | Feature |
|---|---|
| `filtered-reactor/` | CEL predicate filtering on reactor → workflow subscriptions (T-0602) |
| `packaged-graph/` | Distributable `.cloacina` computation-graph package |
| `python-packaged-graph/` | Python-authored packaged computation graph |

### `examples/performance/`

Performance benchmarks. Run via `angreal performance:<name>`.

| Directory | Benchmark |
|---|---|
| `simple/` | Single task baseline |
| `parallel/` | Parallel task execution |
| `pipeline/` | Sequential pipeline |
| `computation-graph/` | CG fire latency + throughput (Apple M3 reference machine) |

## Tests

### `tests/python/`

Integration scenarios in `pytest`. Each scenario number maps to a documented behavior (validated end-to-end against a real `cloaca` import). Cover basic API, workflow execution patterns, context propagation, error handling, multi-tenancy, computation-graph wiring, and packaging.

### Per-crate `tests/`

Integration tests live alongside each crate (`crates/<name>/tests/`) — Diesel DAL tests, executor/scheduler/reconciler integration tests, end-to-end packaging tests.

## Configuration files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace manifest (lists all 11 crates) |
| `pyproject.toml` | Python wheel manifest; maturin reads `[tool.maturin]` |
| `rustfmt.toml` | Rust formatting rules |
| `.pre-commit-config.yaml` | Pre-commit hooks (trailing whitespace, end-of-file, codespell, clippy, fmt) |
| `Dockerfile` | `cloacina-server` runtime image |
| `install.sh` | One-liner install script (per I-0111) |
| `charts/cloacina-server/Chart.yaml` | Helm chart manifest |

## Development

Use `angreal` for every build/test/demo workflow — the angreal tasks encode the correct flags, feature sets, and ordering. Common entry points:

```bash
angreal tree                       # Discover every task
angreal check all-crates           # cargo check + build across all crates in parallel
angreal test unit                  # Unit tests
angreal test all                   # Unit + macros + integration
angreal test coverage              # Merged llvm-cov across the workspace
angreal lint all                   # fmt + clippy + credential-logging guard
angreal services up                # Bring up local Postgres + Kafka (Docker)
angreal docs build                 # Build the Hugo docs site
angreal demos:features:<name>      # Run a feature example
angreal demos:tutorials:rust:NN    # Run Rust tutorial NN
angreal demos:tutorials:python:NN  # Run Python tutorial NN
angreal helm test                  # End-to-end Helm install on kind + /health curl
```

Manual cargo invocations work too but bypass the angreal task definitions; prefer the angreal route in CI scripts.

## See also

- [CLI Reference]({{< ref "cli" >}}) — `cloacinactl` command surface.
- [HTTP API Reference]({{< ref "http-api" >}}) — `cloacina-server` REST endpoints.
- [Compiler + Server Deployment Runbook]({{< ref "/service/how-to/compiler-deployment-runbook" >}}) — long-form runbook for the server + compiler pair.
- [Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture" >}}) — how the cdylib + FFI vtable pieces fit together.
