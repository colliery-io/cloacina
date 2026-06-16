---
title: "Workflow"
description: "A durable, database-backed DAG of tasks — the unit you execute and recover."
weight: 11
---

# Workflow

A **Workflow** is a durable, database-backed DAG (directed acyclic graph) of
[Tasks]({{< ref "/engine/workflows/task" >}}) with explicit dependencies. It is
the thing you *execute*: its task states are persisted, claimed atomically, and
recovered after a restart. The **task** is the unit of scheduling; the workflow is
the unit you name, version, and run.

## Mental model

- A workflow **contains tasks**; the dependencies between tasks form the DAG.
- A workflow has a **name** (how you execute it) and is **content-versioned** —
  its version derives from its tasks' code and structure, so changes are explicit.
- A [Runner]({{< ref "/engine/workflows/runner" >}}) executes a workflow against a
  database; a [Context]({{< ref "/engine/workflows/context" >}}) carries data
  between its tasks.
- Workflows are tenant-scoped (one tenant by default; many under the server).

## Interfaces

The same workflow, defined in each interface. Tasks are declared with the task
decorator/macro and assembled into a named workflow.

{{< tabs "workflow-define" >}}
{{< tab "Rust" >}}
In Rust, the `#[workflow]` **module attribute** names the workflow; the `#[task]`
functions inside the module are its tasks (registered in a global registry the
runner reads):

```rust
use cloacina::{task, workflow, Context, TaskError};

#[workflow(name = "greeting", description = "A one-task workflow")]
pub mod greeting {
    use super::*;

    #[task(id = "hello", dependencies = [])]
    pub async fn hello(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("message", serde_json::json!("Hello World!"))?;
        Ok(())
    }
}
```
{{< /tab >}}
{{< tab "Python" >}}
In Python, the `WorkflowBuilder` context manager assembles tasks declared with
`@cloaca.task` inside its scope (auto-registered on exit):

```python
import cloaca

with cloaca.WorkflowBuilder("greeting") as builder:
    builder.description("A one-task workflow")

    @cloaca.task(id="hello")
    def hello(context):
        context.set("message", "Hello World!")
        return context
```
{{< /tab >}}
{{< /tabs >}}

## Key facts

- **Naming:** the name you register is the name you pass to `run`/`execute`.
- **Versioning:** content-derived; redefining a workflow's tasks changes its
  version. See [Workflow Versioning]({{< ref "/engine/explanation/workflow-versioning" >}}).
- **Validation:** the DAG is validated when built — missing tasks, cycles, and
  unresolvable dependencies are rejected.
- **Execution semantics:** at-least-once with recovery; tasks must be idempotent
  under redelivery.

## Build one

The *concept* lives here; learning to build and run a workflow lives in the doors:

- **Embed it in your app** → [Embed · Tutorials]({{< ref "/embed/tutorials" >}})
- **Ship it to a server** → [Run the Service · Tutorials]({{< ref "/service/tutorials" >}})

## See also

- [Task]({{< ref "/engine/workflows/task" >}}) · [Context]({{< ref "/engine/workflows/context" >}}) · [Runner]({{< ref "/engine/workflows/runner" >}})
- Full API: [Reference]({{< ref "/reference" >}})
