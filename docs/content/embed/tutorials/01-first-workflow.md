---
title: "01 — Your First Workflow"
description: "Define a task, assemble it into a workflow, and run it in-process — in Rust or Python."
weight: 11
aliases:
  - "/python/workflows/tutorials/00-basic-workflow/"
  - "/python/workflows/tutorials/01-first-python-workflow/"
  - "/workflows/tutorials/library/01-first-workflow/"

---

# 01 — Your First Workflow

Build and run a single-task workflow embedded in your own process. The engine is
identical in both languages; pick your tab.

## What you'll build

A workflow named `greeting` with one task that writes a message into the
[Context]({{< ref "/engine/workflows/context" >}}), executed by a
[Runner]({{< ref "/engine/workflows/runner" >}}) against SQLite.

## Step 1 — Add Cloacina

{{< tabs "t01-install" >}}
{{< tab "Rust" >}}
```toml
[dependencies]
cloacina = "0.7"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```
The backend is chosen at runtime from the database URL — no feature flags.
{{< /tab >}}
{{< tab "Python" >}}
```bash
pip install cloaca          # SQLite + PostgreSQL
```
Pre-built wheels for Linux and macOS on Python 3.9–3.12.
{{< /tab >}}
{{< /tabs >}}

## Step 2 — Define a task and a workflow

A [Task]({{< ref "/engine/workflows/task" >}}) is an `async` function with an `id`;
a [Workflow]({{< ref "/engine/workflows/workflow" >}}) names a set of tasks.

{{< tabs "t01-define" >}}
{{< tab "Rust" >}}
The `#[workflow]` module attribute names the workflow; `#[task]` functions inside
it are its tasks:

```rust
use cloacina::{task, workflow, Context, TaskError};

#[workflow(name = "greeting", description = "Say hello")]
pub mod greeting {
    use super::*;

    #[task(id = "hello", dependencies = [])]
    pub async fn hello(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        ctx.insert("message", serde_json::json!("Hello World!"))?;
        Ok(())
    }
}
```
{{< /tab >}}
{{< tab "Python" >}}
The `WorkflowBuilder` context manager assembles `@cloaca.task` functions declared
inside it (registered automatically on exit):

```python
import cloaca

with cloaca.WorkflowBuilder("greeting") as builder:
    builder.description("Say hello")

    @cloaca.task(id="hello")
    def hello(context):
        context.set("message", "Hello World!")
        return context
```
{{< /tab >}}
{{< /tabs >}}

## Step 3 — Run it

Create a [Runner]({{< ref "/engine/workflows/runner" >}}) against SQLite and execute
the workflow by name.

{{< tabs "t01-run" >}}
{{< tab "Rust" >}}
```rust
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = DefaultRunner::with_config(
        "sqlite://app.db?mode=rwc",
        DefaultRunnerConfig::default(),
    ).await?;

    let result = runner.execute("greeting", Context::new()).await?;
    println!("status: {:?}", result.status);
    println!("context: {:?}", result.final_context);

    runner.shutdown().await?;
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
runner = cloaca.DefaultRunner("sqlite:///app.db")
result = runner.execute("greeting", cloaca.Context())
print("status:", result.status)
print("context:", result.final_context)
runner.shutdown()
```
{{< /tab >}}
{{< /tabs >}}

You'll see a completed status and the `message` value in the final context.

## What you learned

- A **task** is an async function with an `id`; a **workflow** names a set of tasks.
- A **runner** executes a workflow against a database (SQLite here) and persists
  state — so execution survives restarts.

## Next

- **[02 — Passing data with Context]({{< ref "/embed/tutorials" >}})**
- Reference: [Workflow]({{< ref "/engine/workflows/workflow" >}}) · [Task]({{< ref "/engine/workflows/task" >}}) · [Context]({{< ref "/engine/workflows/context" >}}) · [Runner]({{< ref "/engine/workflows/runner" >}})
