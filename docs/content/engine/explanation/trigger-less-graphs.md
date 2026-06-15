---
title: "Trigger-less Computation Graphs"
description: "Computation graphs invoked directly by workflow tasks instead of being driven by reactors and accumulator boundaries."
weight: 26
---

# Trigger-less Computation Graphs

Most computation graphs are *driven*: a [reactor]({{< ref "/reference/glossary#reactor" >}})
watches accumulators, fires the graph when reaction criteria are
met, and pipes the InputCache snapshot into the graph's entry nodes.
The graph runs on the reactor's schedule, not the workflow's.

A **trigger-less computation graph** inverts this. There is no
reactor; no accumulator boundaries; no input cache. The graph is
invoked by a workflow task with a workflow context as input, runs
to completion, and writes its terminal-node outputs back into the
post-invocation context by name.

## When to use one

Use a trigger-less CG when:

- You have a complex multi-stage transformation that benefits from
  the CG model (parallel nodes, terminal-node output collection,
  routing) but the *triggering* of that work is the workflow's job,
  not an event stream's.
- You want a workflow task to delegate to a graph defined in a
  *separate* package or module, and you want to ship the graph as a
  reusable component.
- You want graph results threaded into the workflow context so
  downstream tasks can read them as ordinary context keys.

Use a regular reactor-bound CG when:

- The graph should fire continuously based on event arrival
  (accumulator updates).
- Multiple subscribers should bind to the same upstream reactor.
- The graph is the *sink* of a continuous data pipeline, not a
  step in a discrete workflow.

## How they're declared

Trigger-less CGs use the same `#[computation_graph]` macro, but with
no `trigger` argument at all:

```rust
#[computation_graph(
    graph = {
        score: { inputs: ["context"], next: "decide" },
        decide: { next: "publish" },
        publish: {},
    },
)]
mod decision_graph {
    use super::*;

    pub async fn score(ctx: &Context<Value>) -> ScoreOutput { ... }
    pub async fn decide(scored: ScoreOutput) -> Decision { ... }
    pub async fn publish(decision: Decision) -> Value { ... }
}
```

Omitting the `trigger` clause is what marks the graph trigger-less.
The macro emits a `TriggerlessGraphEntry` inventory entry instead of
a reactor-bound `ComputationGraphEntry`.

A workflow task then invokes the graph by name:

```rust
#[task(id = "score_inputs", invokes = "decision_graph")]
async fn score_inputs(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    // Task body. The graph runs after the body returns; terminal
    // outputs are written to the context as keys named after the
    // terminal nodes (here, "publish").
    Ok(())
}
```

After `score_inputs` returns, the host invokes `decision_graph` with
the post-task context, then merges the graph's terminal outputs back
into the context. The `publish` node's output becomes
`ctx.get::<Value>("publish")` for downstream tasks.

## How they cross the FFI boundary

For embedded packages, the graph runs in-process: the
`#[computation_graph]` macro's emitted constructor is registered in
the host `Runtime` via inventory, and the host calls it directly.

For packaged cdylibs, the graph runs *inside* the cdylib but is
invoked from the host. The bridge is two FFI vtable methods:

- **Method 7** (`get_triggerless_graph_metadata`) — at load time,
  the reconciler reads the package's trigger-less graph declarations
  (name + terminal-node-output names) and builds host-side
  `TriggerlessGraphRegistration` adapters per graph.
- **Method 8** (`invoke_triggerless_graph`) — when a workflow task
  with `invokes = "graph_name"` finishes, the host adapter calls
  this method on the cdylib. The plugin runs the graph on its own
  tokio runtime, serializes the terminal outputs, and returns them.
  The host deserializes and writes them into the workflow context.

This pattern preserves the "plugins own their own tokio runtime"
invariant — the host never tries to drive a future across the FFI
boundary.

## Error handling at the FFI boundary

When a packaged trigger-less graph fails inside the cdylib (a node
panicked, deserialization broke, the user code returned `Err`), the
plugin reports failure via the wire format:

```rust
TriggerlessGraphInvokeResult {
    success: false,
    terminal_outputs_json: None,
    error: Some("...some message..."),
}
```

The host adapter (`build_ffi_triggerless_graph_fn` in
`registry/loader/ffi_triggerless_graph.rs`) translates this into a
`GraphResult::Error(GraphError::Execution(...))` for the workflow
task that invoked the graph. The workflow task then surfaces the
error like any other task failure: the task itself is marked
failed, `on_failure` callbacks fire, retry policies apply.

**Two failure modes to be aware of:**

1. **Plugin returns `success: false` but no `error` message.** This
   happens if the cdylib's panic handler caught a panic but couldn't
   extract a useful message. The host inserts a generic placeholder
   (`"trigger-less graph invocation failed (plugin returned no
   error message)"`) so the workflow task still sees a failure
   string. Diagnose by checking the plugin's stderr/log output —
   the panic backtrace will be there.

2. **FFI dispatch itself fails** (the cdylib was unloaded, fidius
   serialization broke, the host couldn't reach the plugin). The
   host wraps this as `GraphError::Execution("FFI dispatch failed:
   ...")` with the underlying fidius error. Treat these as
   infrastructure failures; the plugin probably needs to be
   reloaded.

For embedded trigger-less graphs (no FFI), errors flow directly
from the user's `Result<...>` into the same `GraphError::Execution`
shape, with the message preserved verbatim.

## Loading and unloading

Trigger-less graphs load in **step 4** of the [reconciler
pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}})
— after triggers and reactors, before reactor-bound CGs and
workflows. Step 4 builds a `TriggerlessGraphRegistration` per graph
and registers it in the host `Runtime`. Step 6 then validates that
every workflow's `#[task(invokes = ...)]` reference resolves.

Unload runs step 4 in reverse: the registration is dropped from the
host `Runtime` via `Runtime::unregister_triggerless_graph()`. There
is no scheduler-side teardown for trigger-less graphs (they have no
running task; they only exist as constructors).

## Why this exists

The split between reactor-bound and trigger-less CGs lets you reuse
the same graph machinery (parallel node execution, terminal-output
collection, routing) for two different invocation patterns:

- **Continuous, event-driven** — reactor-bound. The graph fires when
  upstream events arrive; the workflow doesn't know it's running.
- **Discrete, workflow-driven** — trigger-less. The workflow invokes
  the graph at a specific point and consumes its output.

You don't have to choose one model for your whole system. A package
can declare both reactor-bound CGs (steady-state event processing)
and trigger-less CGs (per-workflow-step transformations) side by
side, and the reconciler loads them from the same package metadata.

## Related

- [Computation Graph Reference]({{< ref "/reference/computation-graphs" >}}) — the `#[computation_graph]` macro surface.
- [Reactor Lifecycle]({{< ref "/engine/explanation/reactor-lifecycle" >}}) — what trigger-less graphs *don't* have.
- [FFI Vtable Reference]({{< ref "/reference/ffi-vtable" >}}) — methods 7 and 8.
- [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}}) — step 4.
