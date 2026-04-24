---
id: t-02-workflow-task-cg-invocation
level: task
title: "T-02: Workflow-task CG invocation (`invokes = computation_graph(...)`)"
short_code: "CLOACI-T-0540"
created_at: 2026-04-24T15:08:15.391806+00:00
updated_at: 2026-04-24T15:08:15.391806+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-02: Workflow-task CG invocation

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Add the capability for a workflow task to wrap a computation graph and execute it on demand. Implements the `invokes = computation_graph("name")` clause on `#[task]`, the matching executor path that resolves a graph by name and runs it to completion, and the context ↔ graph-types adapter. This is the embedded-CG-in-workflow payoff from S-0011 — one shot per task execution, no reactor in scope, no streams, graph runs as a deterministic function.

## Acceptance Criteria

- [ ] `#[task]` accepts an `invokes = computation_graph("name")` clause and emits a task whose body is a CG invocation. If the user supplies a function body, it runs before/after the invocation; if omitted, the task is a pure invocation.
- [ ] New task-invocation kind in the dispatcher / `ThreadTaskExecutor`. When a task with `invokes = ...` is dispatched, the executor:
  - Resolves the graph by name via the graph registry.
  - Translates the task's input context into the graph's `entry_type`.
  - Invokes the compiled graph function.
  - Translates terminal-node outputs back into the task's output context.
  - Records completion via the existing task lifecycle (heartbeat, complete).
- [ ] Task retry/timeout apply as for any other task. A graph panic becomes a task error; a graph that exceeds the task's timeout is cancelled per the existing T-0487 cancellation path.
- [ ] Compile-time binding: `invokes = computation_graph("name")` must reference a graph that exists in the build — macro expansion errors at compile time if the name is unresolvable at link time.
- [ ] Integration tests, postgres + sqlite:
  - Trigger-less CG invoked by a workflow task; terminal outputs land in the downstream task's context.
  - Fan-out: two computation graphs both declare `trigger = reactor("R")`; one firing of R invokes both graphs. Unrelated to `invokes`, but this initiative owns proving fan-out works.
  - Graph error → task failure → workflow retry engages.
  - Graph timeout → task cancellation via the claim-loss / timeout path.

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

*To be added during implementation.*
