---
id: t-02-workflow-task-cg-invocation
level: task
title: "T-02: Workflow-task CG invocation (`invokes = computation_graph(...)`)"
short_code: "CLOACI-T-0540"
created_at: 2026-04-24T15:08:15.391806+00:00
updated_at: 2026-04-25T15:02:52.600670+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-02: Workflow-task CG invocation

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Add the capability for a workflow task to wrap a computation graph and execute it on demand. Implements the `invokes = computation_graph("name")` clause on `#[task]`, the matching executor path that resolves a graph by name and runs it to completion, and the context ↔ graph-types adapter. This is the embedded-CG-in-workflow payoff from S-0011 — one shot per task execution, no reactor in scope, no streams, graph runs as a deterministic function.

## Acceptance Criteria

**Design decisions (locked in 2026-04-25 via human-in-the-loop):**
1. Graph identity: type-path handle, mirroring `trigger = reactor(T)`. The CG macro emits a unit-struct `GraphHandle` alongside its registration; `invokes = computation_graph(GraphHandle)` references it by type path so the binding is checked at compile time.
2. Only trigger-less CGs are task-invokable. Reactor-triggered CGs are not — the entry-signature divergence (decision 3) makes them incompatible.
3. **Two-flavor CG entry contract.** Trigger-less CGs declare `entry(ctx: &Context<Value>) -> ...` and operate on the workflow context directly; reactor-triggered CGs keep today's accumulator-typed entry signature (`entry(alpha: Option<&Alpha>) -> ...`). The graph macro picks the right shape based on the presence of `trigger = reactor(T)`. Friction is intentional: the entry signature makes the unit-of-work boundary explicit at the call site.
4. Fan-out (multiple graphs sharing one reactor) split into sibling task **CLOACI-T-0544** — orthogonal runtime change.

**Acceptance criteria:**

- [ ] `#[computation_graph]` (trigger-less form): entry node signature changes from accumulator-typed inputs to `fn(ctx: &Context<Value>) -> ...`. The compiled graph fn for the trigger-less form takes `&Context<Value>` instead of `&InputCache`. Reactor-triggered form is unchanged.
- [ ] `#[computation_graph]` macro emits a unit-struct `GraphHandle` (e.g. `pub struct __Graph_<modname>;`) alongside the existing registration, with a `Graph` trait impl carrying `NAME: &'static str`. Mirrors the `Reactor` trait pattern from T-0543.
- [ ] `#[task]` accepts an `invokes = computation_graph(GraphHandle)` clause. Macro expansion type-checks that the referenced handle implements `Graph`. If the user supplies a function body, it runs before/after the invocation; if omitted, the task is a pure invocation. Macro errors with a readable message if the referenced graph is *not* trigger-less (carries a non-empty trigger reactor).
- [ ] New task-invocation kind in the dispatcher / `ThreadTaskExecutor`. When a task with `invokes = ...` is dispatched, the executor:
  - Resolves the graph by name via the trigger-less graph registry.
  - Passes the task's `Context<Value>` to the compiled graph fn directly (no adapter — the graph consumes the context as written).
  - Writes terminal-node outputs back into the task's output context under each terminal's node name (serde-serialized).
  - Records completion via the existing task lifecycle (heartbeat, complete).
- [ ] Task retry/timeout apply as for any other task. A graph panic becomes a task error; a graph that exceeds the task's timeout is cancelled per the existing T-0487 cancellation path.
- [ ] Integration tests, postgres + sqlite:
  - Trigger-less CG declared with `entry(ctx) -> ...`, invoked by a workflow task; terminal outputs land in the downstream task's context.
  - Workflow task with both pre-body and post-body around the CG invocation.
  - Graph error → task failure → workflow retry engages.
  - Graph timeout → task cancellation via the claim-loss / timeout path.
  - Compile-failure test: `invokes = computation_graph(G)` where `G` is reactor-triggered should fail at macro expansion with a readable message.
- [ ] `cargo check --workspace --all-features` green; `angreal test unit`, `test integration --backend sqlite`, `test integration --backend postgres` all green.

## Implementation Notes

### Technical Approach

1. Extend the `#[task]` macro to parse `invokes = computation_graph("name")`. Emit a task whose `execute()` body resolves the graph and runs it. The user's function body, if provided, is inserted around the invocation (pre-work, invocation, post-work).
2. Extend the task dispatcher (see `thread_task_executor.rs`) to recognize the new invocation kind. The graph lookup uses the registry already exposed by T-01a.
3. Build the context adapter: `Context<Value>` → `entry_type` on the way in (serde deserialize from the context's serialized form into the typed entry), `terminal_outputs` → `Context<Value>` on the way out (serde serialize each terminal into the task's output context under the terminal's node name).
4. Hook graph panics into `TaskError`. The existing executor already catches panics from task bodies; extend the same treatment to graph invocations.

### Key Files

- `crates/cloacina-macros/src/task/parser.rs` — add the `invokes` clause.
- `crates/cloacina-macros/src/task/codegen.rs` — emit the CG-invocation body.
- `crates/cloacina/src/executor/thread_task_executor.rs` — handle the new invocation kind.
- `crates/cloacina/src/computation_graph/scheduler.rs` — expose the compiled-function lookup if T-01a didn't already.
- New integration tests under `crates/cloacina/tests/integration/executor/` or alongside the existing CG tests.

### Dependencies

- **T-01a** (graph registry understands trigger-less CGs; `#[computation_graph]` split exists).
- Does *not* depend on T-01b — this task can land while the bundled form still works in T-01a's branch state.

### Risk Considerations

- Context ↔ graph-types adapter is the highest-risk piece. `Context<Value>` is serde-flexible; the graph's `entry_type` may have required fields the context can't satisfy. Return a specific task error on adapter failure, not a generic panic.
- Terminal node naming: if a graph has multiple terminal nodes, each terminal's output is serialized under a distinct context key. Document the convention in the task doc comment.
- Fan-out test requires two CGs declaring the same `trigger = reactor("R")` — confirm with T-01a that multiple subscribers on one reactor are supported at the runtime level before writing the test.

## Status Updates

### 2026-04-25 — Design locked, implementation plan

Design questions 1–4 settled with user (recorded above the AC). Implementation will land in milestones, each a separate commit on `i-0101-cg-reactor-decouple`:

- **M1**: Add `Graph` trait to `cloacina-computation-graph` (consts: `NAME`, `IS_TRIGGERLESS: bool`). `#[computation_graph]` emits a `GraphHandle` unit struct + `impl Graph` for it alongside the existing registration. Outer-scope type alias `__CGHandle_<mod>` for the same FFI-scoping reason as T-0539's `__CGTriggerReactor_<mod>`. No runtime behavior change yet — purely a new compile-time surface so T-02 can reference graphs by type path.
- **M2**: Switch the trigger-less compiled fn from `Fn(InputCache) -> GraphResult` to `Fn(Context<Value>) -> GraphResult`. Macro changes:
  - Trigger-less entry nodes can't declare accumulator inputs (`entry(alpha)` becomes a parse error in trigger-less form); the user writes `fn entry(ctx: &Context<Value>) -> ...` and the topology references the node by bare name (`entry -> next`).
  - `register_triggerless_graph` / `invoke_triggerless_graph` re-typed to use the new fn shape.
  - Existing trigger-less integration test (`test_cloaci_t_0538_triggerless_*`) migrated.
- **M3**: Extend `#[task]` to parse `invokes = computation_graph(GraphHandle)`. Emit a task body that:
  - Const-checks `<H as Graph>::IS_TRIGGERLESS` (compile error otherwise — that satisfies AC's "reject reactor-triggered" requirement).
  - Resolves the graph fn from the registry by `<H as Graph>::NAME`.
  - Calls it with the task's `Context`.
  - Writes terminal outputs back into the context under their node names.
  - User-supplied function body (if present) wraps the invocation: pre-body → invocation → post-body.
- **M4** *(folded into M3 — the macro emits everything; no separate executor change)*.
- **M5**: Integration tests (sqlite + postgres): trigger-less invoked by task, pre/post body wrapping, graph error → task failure → retry, graph timeout → cancellation, compile-failure test for reactor-triggered graph.

Starting M1 now.
