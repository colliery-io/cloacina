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

    #[task]
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

    @cloaca.task()
    def hello(context):
        context.set("message", "Hello World!")
        return context
```
{{< /tab >}}
{{< /tabs >}}

## Declared inputs (params)

A workflow can declare **typed inputs** so callers know what to pass and bad input
is caught before the run starts (rather than surfacing as a task failure mid-DAG).
Declared params derive a JSON Schema, surface on `WorkflowDetail.declared_params`,
and are validated at execute time — a missing required param or a top-level type
mismatch is rejected with `400 workflow_input_invalid`. A workflow that declares
no params keeps free-form, unvalidated context.

{{< tabs "workflow-params" >}}
{{< tab "Rust" >}}
A `params(...)` clause inside `#[workflow]`; each entry is `name: Type` (required)
or `name: Type = default` (optional). The type must be
`serde::Serialize + schemars::JsonSchema` (the schema is derived automatically):

```rust
#[workflow(
    name = "report",
    params(
        account_id: String,
        window_days: u32 = 30,
    ),
)]
pub mod report { /* … */ }
```
{{< /tab >}}
{{< tab "Python" >}}
The `@cloaca.workflow_params(...)` decorator; required is `name=type`, optional is
`name=(type, default)`. It is parsed from source at build time (a runtime no-op),
so it must be present in the packaged source:

```python
@cloaca.workflow_params(account_id=str, window_days=(int, 30))
with cloaca.WorkflowBuilder("report") as builder:
    ...
```
{{< /tab >}}
{{< /tabs >}}

See [Declare Workflow Inputs]({{< ref "/embed/how-to/declare-workflow-inputs" >}})
for the full guide. (Validation is currently required-presence + top-level type;
nested-schema validation is a planned follow-up.)

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
