---
title: "Invoke a computation graph from a workflow task"
description: "Embed a computation graph as a single workflow task using #[task(invokes = computation_graph(\"name\"))]. The graph runs as one task in the workflow's quantum (CLOACI-I-0101)."
weight: 19
---

# Invoke a computation graph from a workflow task

A computation graph (CG) is normally a standalone, reactor-triggered primitive — but per CLOACI-I-0101 a workflow task can **embed a CG inline**, running the compiled graph function as part of the workflow's quantum. The graph executes once per task invocation (no reactor, no standalone trigger surface), the workflow's task context is passed in as the graph's input, and the graph's terminal-node output becomes the task's output context.

Use this when a workflow needs the determinism of a compiled DAG inside a single workflow step (multi-stage scoring, deterministic ETL transforms with branching, validation pipelines) but doesn't need the event-driven reactor surface.

## Background

Per CLOACI-S-0011, **a computation graph is a DAG where the traversal is the quantum of scheduling and execution.** The standalone form has a reactor that owns the input cache + dirty flags. The embedded form swaps that out — the workflow task is the trigger; the graph runs once, deterministically, with the input context the task received.

| Property | Standalone CG (reactor-triggered) | Embedded CG (workflow task) |
|---|---|---|
| Triggered by | A reactor firing on accumulator deliveries | The workflow task that invokes it |
| Input source | Reactor's `InputCache` populated by accumulators | The task's `Context<Value>` |
| Lifecycle | Long-running scheduler-supervised primitive | Subsumed by the workflow — retries, timeouts, completion all flow through workflow machinery |
| Use when | The graph traversal is the quantum of work | The graph is one deterministic step in a larger pipeline |

The embedded form is **subsumed by the workflow's quantum** — it has no separate fast-fire path, no reactor, no standalone health surface. The graph runs synchronously inside the task; if the task is retried, the graph re-executes; if the task times out, the graph is cancelled mid-traversal.

## Prerequisites

- A `#[computation_graph]` declared in your crate (or imported via a packaged CG plugin).
- A workflow with a task that should invoke that graph.
- Both must be in the same registry — embedded-mode graphs in the same crate, packaged graphs loaded by the same daemon/server.

## Steps

### 1. Declare the computation graph as you normally would

```rust
use cloacina::computation_graph;

#[computation_graph(
    name = "decision_graph",
    graph = {
        score(orderbook, pricing) -> normalize,
        normalize -> dispatch,
    },
)]
pub mod decision_graph {
    pub use super::nodes::{score, normalize, dispatch};
}
```

Note: no `trigger = reactor("...")` clause. An embedded CG doesn't need a reactor declaration — the workflow task is its trigger.

### 2. Invoke the graph from a `#[task]`

```rust
use cloacina::{task, workflow, Context, TaskError};
use serde_json::Value;

#[task(
    id = "score_inputs",
    invokes = computation_graph("decision_graph"),
)]
pub async fn score_inputs(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    // The macro's expansion takes care of:
    //   1. Reading the task's input context and translating each key to a
    //      named source the graph expects (the keys must match the graph's
    //      entry-node source names).
    //   2. Calling the compiled graph function with the snapshot.
    //   3. Decoding the terminal node's output back into JSON and inserting
    //      it into the task's context.
    //
    // You typically leave this body empty — the macro provides the
    // invocation wiring. If you need to massage the input before the call,
    // mutate `ctx` here before falling through.
    Ok(())
}

#[workflow(
    name = "score_and_dispatch",
    tasks = [score_inputs, /* downstream tasks consume score_inputs output */],
)]
pub mod score_and_dispatch {
    pub use super::score_inputs;
}
```

### 3. (Optional) Massage the context before / after the invocation

If the task body has prep / cleanup work that should run on the same task invocation:

```rust
#[task(
    id = "score_inputs",
    invokes = computation_graph("decision_graph"),
    post_invocation = enrich_output,
)]
pub async fn score_inputs(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    // Pre-invocation prep: normalize input shape, fetch external state, etc.
    ctx.insert("orderbook", fetch_orderbook_snapshot().await?)?;
    Ok(())
    // After this returns, the macro invokes the graph with the current ctx.
    // After the graph returns, `enrich_output` is called with the output ctx.
}

async fn enrich_output(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    // Post-invocation: stamp metadata, validate output, etc.
    ctx.insert("processed_at", chrono::Utc::now().to_rfc3339())?;
    Ok(())
}
```

The `post_invocation` callback runs after the graph completes, with the merged output context. Use it for cross-cutting concerns (audit fields, validation) that should apply to every embedded-graph invocation.

### 4. (Python equivalent)

The Python surface mirrors the Rust shape. Declare a graph with the `ComputationGraphBuilder` context manager (no `reactor=` kwarg, since this is the embedded form), then bind it from a task with `invokes=`:

```python
import cloaca

# Embedded graph — no reactor= kwarg.
with cloaca.ComputationGraphBuilder("decision_graph", graph={
    "score": {"inputs": ["orderbook", "pricing"], "next": "normalize"},
    "normalize": {"inputs": ["score"], "next": "dispatch"},
    "dispatch": {"inputs": ["normalize"]},
}) as builder:
    builder.add_node("score", score_node)
    builder.add_node("normalize", normalize_node)
    builder.add_node("dispatch", dispatch_node)

@cloaca.task(id="score_inputs", invokes="decision_graph")
def score_inputs(ctx):
    # Same as Rust — the decorator's expansion handles invocation wiring.
    pass
```

See [Python CG tutorial]({{< ref "/python/computation-graphs/tutorials/09-computation-graph" >}}) for the Python authoring side.

## Verification

The embedded CG is invisible to operational surfaces — it doesn't appear in `cloacinactl graph list` (that command lists reactor-triggered graphs only) and doesn't have its own health endpoint. Verify the invocation worked by checking the workflow execution log:

```sh
cloacinactl --profile prod execution status <exec_id>
```

The task that invoked the graph should show `Completed` (or `Failed` if the graph errored). Per-node graph errors are propagated as task errors with the failing node's name in the error message.

## Error handling

| Failure mode | What happens |
|---|---|
| A graph node returns an error | The task fails with the node's error. Workflow retries (if configured) re-run the entire task — the graph re-traverses from entry nodes. |
| The graph's terminal node returns malformed JSON | The macro's output adapter returns a typed error per CLOACI-I-0110. The task fails with `ContextError::Serialization`. |
| The task is cancelled (timeout, claim loss per T-0487) | The graph traversal is cancelled at the next `await` boundary. |
| The graph references a node that doesn't exist | Compile-time error — the macro validates the topology at expansion time. |

The graph itself has no retry semantics — retry is a workflow-task concern. Configure `retry_attempts` / `retry_backoff` on the `#[task]` as usual.

## When to use this vs the standalone reactor-triggered form

| You want | Reach for |
|---|---|
| The graph as one deterministic step inside a larger workflow | This recipe (embedded `invokes = computation_graph(...)`) |
| The graph to fire on every accumulator boundary, in-process minimum-latency | Standalone with `#[computation_graph(trigger = reactor("name"))]` — see [Tutorial 07]({{< ref "/computation-graphs/tutorials/library/07-computation-graph" >}}) |
| A workflow to fire on a reactor firing (durable, not in-process) | [Subscribe a workflow to a reactor]({{< ref "subscribe-workflow-to-reactor" >}}) |

## What this how-to does NOT cover

- **Authoring the graph itself.** See [Tutorial 07 — Your First Computation Graph]({{< ref "/computation-graphs/tutorials/library/07-computation-graph" >}}).
- **Hand-calling the compiled graph function.** That's the library-API path; this recipe is about the macro-driven embedded form.
- **Packaged-CG-from-workflow.** Loading a packaged CG and invoking it from a workflow is the same recipe, but the package must be loaded by the runner before the workflow registers.

## See also

- [Trigger-less Graphs]({{< ref "/engine/explanation/trigger-less-graphs" >}}) — the explanation doc for the embedded-CG form.
- [Computation Graph as Workflow Task]({{< ref "/engine/computation-graphs/how-to/computation-graph-in-workflow" >}}) — sister how-to from the CG side.
- [Macro Reference]({{< ref "/reference/macros" >}}) — full `#[task(invokes = computation_graph(...))]` syntax.
- **CLOACI-I-0101** — Decouple computation graph from reactor; enable embedded CG workflow tasks.
- **CLOACI-S-0011** — primitive nomenclature spec.
