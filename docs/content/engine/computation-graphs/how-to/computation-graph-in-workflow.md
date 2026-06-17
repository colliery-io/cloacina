---
title: "Computation Graph in a Workflow Task"
description: "Wrap a trigger-less computation graph as a workflow task with #[task(invokes = computation_graph(...))]"
weight: 30
aliases:
  - "/computation-graphs/how-to-guides/computation-graph-in-workflow/"

---

## Problem

You have a workflow whose tasks run on cron or in response to other triggers, and one step is a multi-node computation graph: a deterministic, compile-time-validated DAG that reads some input, fans out, and produces one or more terminal outputs. You want the graph to run as a node *inside* the workflow — input from the task's context, terminal outputs back into the task's context — without standing up an accumulator and reactor.

This is the **embedded computation graph** pattern. The graph is declared trigger-less (no `trigger = reactor(...)` clause) and invoked from a workflow task via `invokes = computation_graph("name")`. There is no reactor, no accumulator, no event loop — the graph is a deterministic function the task calls and waits on.

## When to use this vs. a standalone reactor-bound graph

| | Embedded CG in workflow task | Standalone CG bound to reactor |
|---|---|---|
| **Triggered by** | The workflow that owns the task (cron, manual, upstream task) | A reactor firing on accumulator deliveries |
| **Input source** | The task's `Context<Value>` (or `dict` in Python) | An `InputCache` populated by accumulators |
| **Lifecycle** | Subsumed by the workflow — retries, timeouts, completion all flow through workflow machinery | Long-running scheduler-supervised primitive |
| **Use when** | The graph is one deterministic step in a larger pipeline | The graph traversal is the quantum of execution |

If you find yourself reaching for "I want this graph to run once when an upstream task completes," you want this pattern. If you find yourself reaching for "I want this graph to run every time three accumulator inputs all see new data within a window," you want the standalone reactor-bound form from [Tutorial 09]({{< ref "/embed/tutorials/12-full-pipeline" >}}).

## Declaration shape

### Rust

A trigger-less graph omits the `trigger =` clause entirely:

```rust
use cloacina::Context;
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Score { pub value: f64 }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Confirmation { pub published: bool, pub value: f64 }

#[cloacina_macros::computation_graph(graph = {
    entry -> output,
})]
pub mod scoring_graph {
    use super::*;

    /// Entry node of a trigger-less graph: receives the workflow task's
    /// Context directly. Pull whatever inputs you need by key.
    pub async fn entry(ctx: &Context<Value>) -> Score {
        let raw = ctx
            .get("input_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        Score { value: raw * 1.5 }
    }

    pub async fn output(input: &Score) -> Confirmation {
        Confirmation { published: true, value: input.value }
    }
}
```

The workflow task names the graph by string and gets the invocation generated for it:

```rust
#[cloacina_macros::task(
    invokes = computation_graph("scoring_graph"),
)]
async fn run_scoring(
    context: &mut cloacina_workflow::Context<serde_json::Value>,
) -> Result<(), cloacina_workflow::TaskError> {
    // Pre-work runs *before* the graph fires. Optional.
    assert!(context.get("input_value").is_some());
    Ok(())
}
```

After this task runs:

1. The user body executes (pre-work). It can read or insert into `context`.
2. The macro-generated invocation runs the compiled graph with `context` as the entry node's input.
3. Each terminal node's output is serialized and written back into `context` under that node's name — here, `context.get("output")` returns the `Confirmation` struct as JSON.

If the graph has multiple terminal nodes, each lands under its own name. Downstream tasks in the workflow consume them by key.

### Optional post-invocation hook

If you need a post-step that runs *after* the graph but *before* `on_success`, pass `post_invocation`:

```rust
async fn after_scoring(
    ctx: &mut cloacina_workflow::Context<serde_json::Value>,
) -> Result<(), cloacina_workflow::TaskError> {
    // ctx already contains the graph's terminal outputs.
    let _ = ctx.insert("post_hook_ran", serde_json::json!(true));
    Ok(())
}

#[cloacina_macros::task(
    invokes = computation_graph("scoring_graph"),
    post_invocation = after_scoring,
)]
async fn run_scoring_with_post(
    ctx: &mut cloacina_workflow::Context<serde_json::Value>,
) -> Result<(), cloacina_workflow::TaskError> {
    Ok(())
}
```

Execution order: user body → graph invocation → `post_invocation` hook → `on_success`.

### Python

The Python surface mirrors the Rust shape. Declare a trigger-less graph (no `reactor=` kwarg on the builder), bind it from a task with `invokes=`:

```python
import cloaca

with cloaca.WorkflowBuilder("scoring_workflow") as wf:
    wf.description("Score then publish")

    # Trigger-less graph: no reactor= kwarg on the builder.
    scoring_graph = cloaca.ComputationGraphBuilder(
        "scoring_graph",
        graph={"score": {}},
    )
    with scoring_graph:

        @cloaca.node
        def score(ctx):
            raw = ctx.get("input_value", 0.0)
            return {"value": raw * 1.5}

    @cloaca.task(invokes=scoring_graph)
    def run_scoring(ctx):
        # Pre-work runs before the graph fires.
        return ctx
```

After `run_scoring` executes, `final_context["score"]` holds the terminal `score` node's return dict. Multiple terminals each land under their own name.

`@cloaca.task` also accepts `post_invocation=callable` for the post-graph hook.

## How input and output are adapted

The context-to-graph and graph-to-context adapter is a JSON round-trip. The entry node receives the task's context directly (`&Context<Value>` in Rust, `ctx` dict in Python); the macro does not unpack the context for you. Pull the keys you need.

Terminal outputs are serialized with `serde_json::to_value` (Rust) or the standard Python JSON path. **A terminal that cannot be serialized panics the task**: the macro generates `to_value(&result).unwrap_or_else(|_| panic!("..."))` for trigger-less terminals. Workflow executor machinery turns the panic into `TaskError::ExecutionFailed`, which engages the workflow's retry policy.

A consequence: terminal types must implement `Serialize`. If you need to pass a non-serializable value to a downstream task, serialize it explicitly inside the terminal node and have the downstream task deserialize.

## What the macro will reject

The trigger-less graph contract is enforced both at compile time (Rust) and at task-registration time (Python):

- **`invokes = computation_graph("R")`** where `R` is a *reactor-triggered* graph: rejected. Reactor-triggered graphs implement a different trait surface and don't expose the compiled-function handle the task adapter needs.
- **`invokes = computation_graph("missing")`**: registration error — the graph name must resolve.
- **Graph entry node with the wrong signature**: a trigger-less graph's entry node must take `&Context<Value>` (Rust) / a single `ctx` argument (Python). The macro will not compile / register otherwise.

## Worked example

Putting it together — a workflow that fetches a batch, scores it via an embedded CG, then publishes:

```rust
#[cloacina_macros::computation_graph(graph = { entry -> output })]
pub mod batch_scoring {
    use super::*;
    use cloacina::Context;
    use serde_json::Value;

    pub async fn entry(ctx: &Context<Value>) -> Score {
        let items = ctx
            .get("batch")
            .and_then(|v| v.as_array())
            .map(|a| a.len() as f64)
            .unwrap_or(0.0);
        Score { value: items * 10.0 }
    }

    pub async fn output(score: &Score) -> Confirmation {
        Confirmation { published: true, value: score.value }
    }
}

#[cloacina_macros::workflow(name = "nightly_report")]
mod nightly_report {
    use super::*;

    #[cloacina_macros::task]
    async fn fetch_batch(ctx: &mut Context<Value>) -> Result<(), TaskError> {
        ctx.insert("batch", serde_json::json!([1, 2, 3, 4, 5]))?;
        Ok(())
    }

    #[cloacina_macros::task(
        depends_on = ["fetch_batch"],
        invokes = computation_graph("batch_scoring"),
    )]
    async fn score_batch(_ctx: &mut Context<Value>) -> Result<(), TaskError> {
        Ok(())
    }

    #[cloacina_macros::task(depends_on = ["score_batch"])]
    async fn publish(ctx: &mut Context<Value>) -> Result<(), TaskError> {
        let confirmation = ctx.get("output").expect("graph terminal must land");
        tracing::info!(?confirmation, "publishing score");
        Ok(())
    }
}
```

`fetch_batch` populates the context; `score_batch` invokes the graph (its terminal `output` lands in the context as the JSON-ified `Confirmation`); `publish` reads it. Workflow retry, timeout, and heartbeat semantics apply to `score_batch` exactly as they would to any other task — if the graph panics or its terminal cannot serialize, the task fails and the workflow's retry policy engages.

## Related

- [Tutorial 07 — Your First Computation Graph]({{< ref "/embed/tutorials/10-computation-graph" >}}) — the standalone, reactor-bound form.
- [Tutorial 09 — Full Multi-Source Pipeline]({{< ref "/embed/tutorials/12-full-pipeline" >}}) — when the graph traversal is the quantum.
- [Trigger-less Graphs (explanation)]({{< ref "/engine/explanation/trigger-less-graphs" >}}) — design notes on why trigger-less graphs are a first-class primitive.
- [CLOACI-S-0011 — Cloacina primitive nomenclature](https://github.com/colliery-io/cloacina/blob/main/.metis/specs/CLOACI-S-0011.md) — the two execution quanta and where this pattern fits.
