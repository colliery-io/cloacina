---
title: "Repository Structure"
description: "Overview of the Cloacina repository organization, crate architecture, and the relationship between workspace members."
weight: 10
aliases:
  - "/platform/reference/repository-structure/"

---

# Repository Structure

The Cloacina workspace ships fifteen Rust crates plus a Python wheel built on top of one of them. This page is a map of what lives where and which crate to look at for a given concern.

## Directory layout

```
cloacina/
  crates/                          # Rust workspace members (15)
    cloacina/                      # Core engine: tasks, workflows, runner, DAL, executor, registry
    cloacina-agent/                # DB-less fleet execution agent binary (cloacina-agent)
    cloacina-api-types/            # Shared HTTP/WS wire types (list envelopes, health, delivery, reactor)
    cloacina-build/                # build-script helper for binaries linking cloacina/PyO3 (rpath fixups)
    cloacina-client/               # Published Rust client SDK (REST + WS follow)
    cloacina-compiler/             # Standalone build worker (cargo build for .cloacina packages)
    cloacina-computation-graph/    # Computation-graph runtime: reactors, accumulators, scheduler
    cloacina-constructor-contract/ # Provider/constructor contract types (host + wasm32-wasip2)
    cloacina-macros/               # Proc macros: #[task], #[workflow], #[trigger], #[reactor], #[computation_graph], accumulator + constructor macros
    cloacina-python/               # PyO3 wheel runtime (the `cloaca` Python module)
    cloacina-server/               # HTTP API binary (cloacina-server)
    cloacina-testing/              # Test utilities: TestRunner, BoundaryEmitter
    cloacina-workflow/             # Slim workflow-author crate (Trigger trait, RetryPolicy, types)
    cloacina-workflow-plugin/      # FFI vtable for .cloacina cdylib plugins (CloacinaPlugin trait + package!() shell)
    cloacinactl/                   # CLI binary (cloacinactl)

  providers/                       # Constructor provider crates (own workspaces, excluded from the root workspace)
    cloacina-provider-fs/          # WASM task provider (read_file / write_file)
    cloacina-provider-kafka/       # Native stream-accumulator provider (kafka_source)
    COMPAT.toml                    # Generated provider↔core compatibility table
    PROVIDERS.md                   # Provider conventions + roster

  clients/                         # Generated SDKs
    python/                        # Python client SDK (cloacina-client)
    typescript/                    # TypeScript client SDK (@cloacina/client)

  ui/                              # Leptos/WASM SPA (embedded in the server behind `embedded-ui`)
  charts/                          # Helm charts: cloacina-server (local Postgres subchart), cloacina-agent, cloacina-ui
  deploy/                          # Deploy templates (docker-compose, k8s)
  docker/                          # Dockerfiles + demo compose stack
  docs/                            # Hugo documentation site
  examples/                        # Runnable examples (see below)
  scripts/                         # Maintenance scripts (provider_wave.py, SDK checks, install.sh — served at get.cloacina.dev)
  tests/                           # Integration tests
    python/                        # Python pytest scenarios

  Cargo.toml                       # Workspace manifest
  Dockerfile                       # cloacina production image
  plissken.toml                    # api-reference generation config
  .angreal/                        # Task automation (check, ci, demos, docs, helm, lint, performance, providers, release, services, test, ui)
  .github/                         # CI workflows
  .metis/                          # Metis planning artifacts (vision, initiatives, ADRs, specs, tasks)
```

(The Python wheel manifest lives at `crates/cloacina-python/pyproject.toml`, not the repo root.)

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

**Cargo features:** `postgres` (default), `sqlite` (default), `macros` (default), `auth`, `constructor-packaging`, `constructors-wasm` (off by default — pulls wasmtime). There is no `kafka` feature (event sources ship as provider crates); the PyO3 `extension-module` switch lives on `cloacina-python`, not here.

#### `cloacina-workflow`

The thin author-facing surface — types and traits that workflow authors need without pulling the full runtime. Houses the `Trigger` trait, `TriggerResult`, `RetryPolicy`, error types, and a feature gate (`packaged`) for cdylib-mode plugin authoring.

#### `cloacina-computation-graph`

Computation-graph runtime: `Reactor`, `Accumulator` (with `passthrough`, `stream`, `polling`, `batch`, `state` variants), `ComputationGraphScheduler`, `InputCache`, boundary types, channel-backed firing path.

#### `cloacina-macros`

Procedural macros for declarative authoring:

- `#[task]` — declare a task function (supports `invokes = computation_graph("name")` for embedded CG-in-workflow per I-0101).
- `#[workflow]` — declare a workflow module (supports `triggers = [...]`, `params(...)`, `secrets(...)`).
- `#[trigger]` — declare a custom poll or cron trigger bound to a workflow via `on = "..."`.
- `#[reactor]` — declare a reactor as a first-class primitive (I-0101 split from `#[computation_graph]`).
- `#[computation_graph]` — declare a graph with `trigger = reactor("name")` (reactor-bound) or no trigger (trigger-less, invoked via `invokes = computation_graph(...)` from a task).
- `#[passthrough_accumulator]`, `#[stream_accumulator]`, `#[polling_accumulator]`, `#[batch_accumulator]`, `#[state_accumulator]` — accumulator-kind macros.
- `#[constructor]` / `constructor_provider!` / `constructor!` — provider-crate authoring and in-workflow constructor nodes.

### Packaging & distribution

#### `cloacina-workflow-plugin`

The FFI vtable trait crate. Compiled `.cloacina` packages are cdylibs that expose a fixed eleven-method vtable (indices 0–10, `CloacinaPlugin` interface version 5) dispatched at runtime by `fidius`. The `cloacina_workflow_plugin::package!()` shell macro emits the whole implementation; the host (`cloacina`) loads packages via this trait without symbol-name knowledge of the user's code. See the [FFI Vtable Reference]({{< ref "ffi-vtable" >}}).

#### `cloacina-build`

A small build-script helper (`cloacina_build::configure()`) for **binaries that link cloacina with PyO3** — applies `pyo3_build_config` cfgs and the libpython rpath link-arg (with macOS `.framework` handling). It is *not* a packaging tool, and packaged workflows no longer carry a `build.rs` at all — the compiler injects the build wiring (T-0737).

#### `cloacina-compiler`

Standalone build worker. Polls the database for pending package builds (`workflow_packages.build_status = 'pending'`), runs `cargo build --frozen --offline` against a curated vendored registry (CLOACI-I-0104 hardening), and persists the resulting `compiled_data` bytes. Coordinated with the server via atomic claim queries (`FOR UPDATE SKIP LOCKED` on Postgres). Exposes its own `/health`, `/v1/status`, and `/metrics` endpoints (I-0109).

#### `cloacina-constructor-contract`

The provider/constructor contract crate — serde-only types (`ProviderManifest`, `ConstructorManifest`, `PrimitiveKind`, `ProviderRuntime`, the object-safe member traits and wire types) deliberately buildable for both the host and `wasm32-wasip2`. Provider crates and the loader both depend on it.

### Shared types & clients

#### `cloacina-api-types`

Shared HTTP/WS wire types used by the server, CLI, and SDKs: list envelopes (`ListResponse` / `TenantListResponse`), health types (`GraphStatus`, `ReactorStatus`, `AccumulatorStatus`), delivery-protocol frames, reactor command types, `InputSlot`, and the error body.

#### `cloacina-client`

The published Rust client SDK — REST client plus the WebSocket follow path used by `cloacinactl execution events --follow` (T-0646) and downstream consumers.

### Service surface

#### `cloacina-server`

The HTTP API binary. Backed by PostgreSQL. Multi-tenant by default with schema isolation. Exposes the REST API documented in [HTTP API Reference]({{< ref "http-api" >}}) under `/v1/*` plus public `/health`, `/ready`, `/metrics`, `/openapi.json`. The server image (`ghcr.io/colliery-software/cloacina-server`) ships per I-0111 / T-0610.

#### `cloacina-agent`

The DB-less fleet execution agent binary (I-0114). Registers with a server over REST, opens the delivery WebSocket, fetches compiled cdylibs by digest, executes work in-process, and reports results — no database connection of its own.

#### `cloacinactl`

The CLI binary. Noun-verb command structure (per CLOACI-I-0098 / T-0538): `cloacinactl <noun> <verb>` with nouns `accumulator`, `compiler`, `constructor`, `daemon`, `execution`, `graph`, `key`, `package`, `reactor`, `secret`, `server`, `tenant`, `trigger`, `workflow`; singletons `status`, `config`, `admin`, `completions`. Profile model via `~/.cloacina/config.toml` (ADR-0003). See [CLI Reference]({{< ref "cli" >}}).

### Python

#### `cloacina-python`

The PyO3 wheel runtime — isolated as its own crate per CLOACI-T-0529 / T-0532 so binaries that don't execute Python don't transitively link `pyo3`. The `cloaca` Python module is the user-facing surface; it wraps the `cloacina` core engine with Pythonic ergonomics:

- `@cloaca.task`, `@cloaca.trigger`, `@cloaca.reactor`, `@cloaca.node`, `@cloaca.passthrough_accumulator`, `@cloaca.stream_accumulator`, `@cloaca.polling_accumulator`, `@cloaca.batch_accumulator`, `@cloaca.state_accumulator`, `@cloaca.boundary_schema` decorators.
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

Run any tutorial via `angreal demos tutorials rust NN` or `angreal demos tutorials python NN` (angreal task nesting is space-separated).

### `examples/features/`

Feature showcases — each demonstrates one capability end-to-end. Run via `angreal demos features <name>`. (New example directories auto-register into the harness and CI matrix; `package.toml` presence selects the packaged gold path over embedded `cargo run`.)

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
| `parameterized-workflow/` | Declared workflow params (`params(...)`) |
| `per-tenant-credentials/` | Per-tenant DB credentials (defense in depth) |
| `python-conditional/` | Packaged Python trigger_rules / gated Skips |
| `python-cron/` | Packaged Python cron trigger |
| `python-multi-tenant/` | Same Python workflow in two tenants |
| `python-packaged/` | Packaged Python workflow |
| `python-parameterized/` | Parameterized Python workflow |
| `python-retries/` | Python retry policy |
| `python-secrets/` | Python workflow secrets |
| `python-triggers/` | Packaged Python poll trigger |
| `python-workflow/` | Python wheel demo (venv + `run_pipeline.py`) |
| `registry-execution/` | Registry-driven dynamic loading |
| `simple-packaged/` | Minimal packaged workflow (smallest reproducer) |
| `validation-failures/` | Macro-validation failure shapes (negative fixture; excluded from demos/CI) |
| `workflow-secrets/` | Workflow secrets (Rust) |

**Computation-graph features (`examples/features/computation-graphs/`):**

| Directory | Feature |
|---|---|
| `cg-feature-tour/` | Computation-graph feature tour (incl. live Kafka stream surface) |
| `filtered-reactor/` | CEL predicate filtering on reactor → workflow subscriptions (T-0602) |
| `packaged-graph/` | Distributable `.cloacina` computation-graph package |
| `python-batch-graph/` | Python batch accumulator |
| `python-packaged-graph/` | Python-authored packaged computation graph |
| `python-polling-graph/` | Python polling accumulator |
| `python-stateful-graph/` | Python state accumulator (bounded rolling window) |

The repo also carries `examples/constructor-contract/` (constructor/provider fixtures and seed providers), `examples/fixtures/` (test-harness package fixtures, including pre-built `.cloacina` archives under `dist/`), and `examples/wasm-operator-spike/`.

### `examples/performance/`

Performance benchmarks. Run via `angreal performance <name>` (`simple`, `parallel`, `pipeline`, `computation-graph-bench`, `quick`, `all`).

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
| `Cargo.toml` | Workspace manifest (lists all 15 crates; excludes `examples/*` and `providers/*`) |
| `crates/cloacina-python/pyproject.toml` | Python wheel manifest; maturin reads `[tool.maturin]` |
| `plissken.toml` | api-reference generation config |
| `rustfmt.toml` | Rust formatting rules |
| `.pre-commit-config.yaml` | Pre-commit hooks (trailing whitespace, end-of-file, codespell, clippy, fmt) |
| `Dockerfile` | `cloacina-server` runtime image |
| `scripts/install.sh` | One-liner install script, served at get.cloacina.dev (per I-0111) |
| `charts/cloacina-server/Chart.yaml` | Helm chart manifest |

## Development

Use `angreal` for every build/test/demo workflow — the angreal tasks encode the correct flags, feature sets, and ordering. Common entry points:

```bash
angreal tree                       # Discover every task
angreal check crate <path>         # Targeted cargo check (all-crates exists but builds every example — disk-heavy)
angreal test unit                  # Unit tests
angreal test all                   # Unit + macros + integration
angreal test coverage              # Merged llvm-cov across the workspace
angreal lint all                   # fmt + clippy + credential-logging guard
angreal services up                # Bring up local Postgres (host port 15432) + Kafka + dex (Docker)
angreal docs build                 # Build the Hugo docs site
angreal demos features <name>      # Run a feature example
angreal demos tutorials rust NN    # Run Rust tutorial NN
angreal demos tutorials python NN  # Run Python tutorial NN
angreal helm test                  # End-to-end Helm install on kind + /health curl
```

Manual cargo invocations work too but bypass the angreal task definitions; prefer the angreal route in CI scripts.

## See also

- [CLI Reference]({{< ref "cli" >}}) — `cloacinactl` command surface.
- [HTTP API Reference]({{< ref "http-api" >}}) — `cloacina-server` REST endpoints.
- [Compiler + Server Deployment Runbook]({{< ref "/service/how-to/compiler-deployment-runbook" >}}) — long-form runbook for the server + compiler pair.
- [Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture" >}}) — how the cdylib + FFI vtable pieces fit together.
