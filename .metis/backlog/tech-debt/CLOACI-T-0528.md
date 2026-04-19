---
id: audit-reactor-vs-computation-graph
level: task
title: "Audit reactor vs computation_graph naming drift in core + server"
short_code: "CLOACI-T-0528"
created_at: 2026-04-18T16:32:39.189020+00:00
updated_at: 2026-04-19T02:58:09.450466+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Audit reactor vs computation_graph naming drift in core + server

## Objective

The codebase uses two distinct terms for what is conceptually a two-layer
model, and the two are not consistently applied. Sweep the internals for
drift and align names to the intended semantics.

**The model:**
- **Computation graph** — the spec. A typed DAG: nodes, edges,
  accumulator definitions, trigger rules. Pure data / structure.
- **Reactor** — the runtime that instantiates and runs a computation
  graph. Has health, pause state, current accumulator values,
  scheduler state.

The CLI already uses this distinction correctly (`cloacinactl reactor
list` = runtime observability). Parts of core and the server do too
(`Reactor`, `ReactiveScheduler`, `/v1/health/reactors`). Other parts
use `graph` / `computation_graph` where they actually mean reactor
state, and vice versa.

## Technical Debt Impact

- **Current problems**: Operators and contributors can't tell at a
  glance whether a symbol refers to spec or runtime. New code picks
  whichever term the author saw most recently. API responses mix both
  names in one payload.
- **Benefits of fixing**: One canonical term per layer. Easier to
  onboard, easier to grep, future endpoints pick the right word by
  default.
- **Risk if deferred**: Every new endpoint / DAL method / config knob
  compounds the drift. Renames get more expensive as more external
  consumers depend on the ambiguous names.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Grep sweep: every `graph` / `Graph` / `computation_graph` symbol
  in `crates/cloacina/src` + `crates/cloacina-server/src` is
  classified as either spec or runtime, and renamed if wrong.
- [ ] Same sweep for public HTTP routes, response field names, and
  config keys.
- [ ] DAL tables / migrations audited — some are correctly
  `computation_graph_state_*` (spec-level state), some may be
  mislabeled. Don't rename migrations already applied; document
  any drift instead.
- [ ] Doc pass on `docs/operations/` and `docs/content/` to use the
  terms consistently.
- [ ] Short README / CLAUDE.md note explaining the two-layer model so
  future contributors don't re-introduce drift.

## Implementation Notes

### Scope

Read-only audit first: produce a table of every identifier, its
current name, and the proposed correct name. Review the table before
starting rename PRs — some cases will be genuinely ambiguous and worth
discussing.

### Suggested rename heuristic

- Returns / touches live `ReactiveScheduler` state → `reactor`.
- Describes / parses / validates a DAG definition → `computation_graph`
  or `graph` (short form).
- Persists state that belongs to the running instance (accumulator
  buffers, dirty flags, last-tick timestamp) → `reactor` in the type
  name; DB columns can keep `computation_graph_state_*` if the schema
  intent is "state for a loaded graph".

### Don't rename

- Existing migration directories (breaks replay history).
- `computation_graph/` module path in `cloacina` core — the module
  owns both the spec types and the reactor. Renaming the module would
  be a bigger initiative.

### Dependencies

None. This is a standalone cleanup; no initiative ties.

## Status Updates

### 2026-04-19 — Audit (read-only pass)

Scanned `crates/cloacina/src`, `crates/cloacina-server/src`,
`crates/cloacinactl/src`, DAL + migrations, and docs. Confirmed the
two-layer intent is already followed in most of the codebase; drift
clusters in the scheduler + Python CG subsystems.

#### Already correct (no changes)

| Subsystem | What it uses |
|---|---|
| Server HTTP routes | `/v1/health/reactors`, `/v1/health/reactors/{name}`, `/v1/health/accumulators` |
| CLI | `cloacinactl reactor {list,status,accumulators}` (post I-0097) |
| Core runtime types | `Reactor`, `ReactorHandle`, `ReactorHealth`, `ReactorCommand` |
| Core spec types | `WorkflowGraph`, `PyComputationGraphBuilder`, `register_computation_graph` |
| DAL runtime tables | `reactor_state`, `save_reactor_state`, `load_reactor_state` |

#### Drift inventory — rename targets

| File:Line | Identifier | Layer | Proposed | Notes |
|---|---|---|---|---|
| `cloacina/src/computation_graph/scheduler.rs:101` | `pub struct GraphStatus` | runtime | `ReactorStatus` | Fields `reactor_paused`, `running`, `health`, `accumulators` — all runtime state. |
| `cloacina/src/computation_graph/scheduler.rs:175` | `load_graph(decl)` | runtime | `load_reactor` | Spawns a running reactor from a declaration. |
| `cloacina/src/computation_graph/scheduler.rs:308` | `unload_graph(name)` | runtime | `unload_reactor` | Stops a running reactor by name. |
| `cloacina/src/computation_graph/scheduler.rs:337` | `list_graphs()` | runtime | `list_reactors` | Returns currently-loaded reactor instances. |
| `cloacina/src/python/computation_graph.rs:481` | `PythonGraphExecutor` | runtime | `PythonReactorExecutor` | Runtime executor struct (mirrors Rust `Reactor`). |
| `cloacina/src/python/computation_graph.rs:476` | `get_graph_executor` | runtime | `get_reactor_executor` | Fetches the executor for a running reactor. |
| `cloacina/src/python/computation_graph.rs:601` | `build_python_graph_declaration` | mixed | `build_python_reactor_declaration` | Builds a `ComputationGraphDeclaration` — arg is the reactor spec, not a graph. |
| `cloacina/src/python/computation_graph.rs:463` | `GRAPH_EXECUTORS` static | runtime | `REACTOR_EXECUTORS` | Map of running reactor executors. |
| `cloacina-server/src/lib.rs:512` | `state.reactive_scheduler.list_graphs()` | call site | (follows `list_reactors` rename) | Reactor endpoints already named correctly, but they call `list_graphs()` internally. |
| `cloacina-server/src/routes/health_reactive.rs:54,83` | `let graphs = ...list_graphs()` | call site | rename local to `reactors` | Two sites in reactor endpoint handlers. |
| `cloacina/src/registry/reconciler/loading.rs:303,400,509` | `scheduler.load_graph` / `unload_graph` | call site | (follows renames) | Rust-CG + Python-CG both affected. |
| `cloacina/src/computation_graph/scheduler.rs:681` | `self.unload_graph(&name)` | internal | (follows rename) | Shutdown path. |
| `cloacina/src/python/loader.rs:434` | `get_graph_executor(&graph_name)` | call site | (follows rename) | Python CG load path. |
| `cloacina/tests/integration/computation_graph.rs:411,429,473,1867` | `scheduler.load_graph` / `list_graphs` / `unload_graph` | test | (follows renames) | Four test call sites. |
| `cloacina/src/computation_graph/scheduler.rs:752,802` | `test_load_graph_*`, `test_unload_graph_*` | test fn names | `test_load_reactor_*` / `test_unload_reactor_*` | Match renamed methods. |
| `cloacina/src/python/computation_graph_tests.rs:90,136,284,359` | `get_graph_executor(...)` | test | (follows rename) | Four test call sites. |

Total: ~6 symbol renames, ~25 call-site updates, ~10 test-name updates.

#### Deliberately NOT renamed

| Thing | Reason |
|---|---|
| `computation_graph/` module path | Owns both spec types (WorkflowGraph, accumulators) and runtime types (Reactor, scheduler). Splitting is a bigger initiative. |
| Migration dirs `017_create_computation_graph_state_tables` + `015_…sqlite` | Applied migrations — renaming breaks replay history. The dir name is cosmetic; the schema inside already uses `reactor_state` correctly. |
| `ComputationGraphDeclaration` struct | Genuinely mixed (contains both accumulator spec *and* reactor config). Splitting it is a design change, not a rename. Left for human call. |
| `register_computation_graph`, `PyComputationGraphBuilder`, `WorkflowGraph` | Correctly spec-level — they accept DAG definitions, not runtime state. |
| DAL tables `accumulator_checkpoints`, `accumulator_boundaries`, `accumulator_state` | Correctly spec-level persisted state. |

#### Top 5 ambiguous cases flagged for human judgement

1. **`ComputationGraphDeclaration`** — has both spec fields (`accumulators[]`) and runtime fields (`reactor {}`). Rename vs split is a design call; leaving as-is until someone wants to separate concerns.
2. **`GraphStatus`** — high-confusion name (holds runtime state). `ReactorStatus` is the obvious rename but the struct is public API on `ReactiveScheduler::list_graphs`, so this is the rename with the biggest ripple.
3. **`load_graph` / `unload_graph`** — the verb+noun read naturally as "load a graph spec," but what they do is spawn/teardown a running reactor. Rename is semantically correct; just touches a lot of call sites.
4. **`build_python_graph_declaration`** — builds a declaration struct that is the input to the reactor. Arguably `build_python_reactor_declaration` since the output is consumed by `load_reactor`.
5. **Migration dir `…computation_graph_state_tables`** — the name suggests spec state but the dir contains `reactor_state`. Cannot rename the dir (replay history). Leave with an inline SQL comment pointing to the naming convention.

### Next step

**Paused pending a nomenclature conversation.** Dylan's position (2026-04-19): computation graph and reactor are separate concepts, not a two-layer spec/runtime split. The rename proposals above assume the latter and need to be re-evaluated against the actual model before any code changes land.

Task stays open with the audit intact — the inventory of where each term is used is still useful input regardless of how the naming discussion resolves. No rename PR until the conceptual split is settled.
