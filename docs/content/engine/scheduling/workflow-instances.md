---
title: "Workflow Instances"
description: "Define a workflow once, stamp out named, parameterized, scheduled copies of it."
weight: 30
---

# Workflow Instances

A workflow that declares `params(...)` advertises its configurable surface. A
**workflow instance** binds values to those params and gives the result a
human name and a schedule — so one workflow template becomes many independent,
operable schedules:

```
sync_file  (template, declares: source, dst, mode = "copy")
 ├── sync_prod     source=/prod     dst=/backup/prod     0 * * * *
 ├── sync_staging  source=/staging  dst=/backup/staging  0 3 * * *
 └── sync_archive  source=/archive  dst=/cold            0 4 * * 0
```

An instance is **data, not code**: a serializable value of
`(workflow name + fully-resolved params)`. Defaults are snapshotted when the
instance is built — a registered instance never silently changes behavior when
the workflow's defaults change; re-register to adopt new defaults.

{{< hint type=note title="Prerequisite: declare the params first" >}}
Instances only make sense for a workflow that declares its inputs. Before you can
bind params, the `sync_file` workflow must declare them with
`#[workflow(params(...))]` (Python: `@cloaca.workflow_params(...)`) — see
[Declare and validate workflow inputs]({{< ref "/embed/how-to/declare-workflow-inputs" >}}).
That declaration is what produces the `declared` slots the builder validates
against below.
{{< /hint >}}

## Rust

```rust
use cloacina::workflow_instance::WorkflowInstance;
use cloacina::input_interface::{schema_for, InputSlot};

// `declared` is the workflow's declared input slots — the `Vec<InputSlot>` the
// `#[workflow(params(...))]` macro emits for `sync_file`. It's the same schema
// the execute API validates against. Shown inline here so this example is
// self-contained; in practice you read it from the registered workflow rather
// than hand-writing it:
let declared = vec![
    InputSlot::required("source", schema_for::<String>()),
    InputSlot::required("dst", schema_for::<String>()),
    InputSlot::optional("mode", schema_for::<String>(), Some(serde_json::json!("copy"))),
];

let instance = WorkflowInstance::builder("sync_file")
    .param("source", "/prod")?
    .param("dst", "/backup/prod")?
    .build(&declared)?;          // validates: unknown / missing-required / reserved names
                                 // and snapshots defaults (mode = "copy")

// Register under a human name, on its own cron schedule:
runner
    .register_cron_workflow_instance(&instance, "sync_prod", "0 * * * *", "UTC")
    .await?;

// Lifecycle by name (resolves to the schedule row underneath):
let row = runner.get_workflow_instance("sync_file", "sync_prod").await?;
runner.unregister_workflow_instance("sync_file", "sync_prod").await?;
```

Instance names are unique per workflow (a second `sync_prod` registration
fails); different names stamp out independent copies.

## Python

```python
params = cloaca.Context({"source": "/prod", "dst": "/backup/prod"})
runner.register_workflow_instance(
    "sync_file", "sync_prod", "0 * * * *", "UTC", params
)
```

## How params reach the workflow

At every fire (cron or trigger), the instance's stored params are merged into
the run's context as **flat top-level keys** — exactly the shape a manual
`execute` with a validated context produces, so tasks read them identically
in both cases:

- The scheduler's **reserved keys always win**: `scheduled_time`,
  `schedule_id`, `schedule_timezone`, `schedule_expression`, `trigger_name`,
  `triggered_at` cannot be overridden (or spoofed) by a binding.
- For **trigger** fires, bound instance params override same-named keys in
  the trigger-produced payload.

Anonymous schedules (registered via `register_cron_workflow`) are unaffected —
they carry no params and behave exactly as before.

## What an instance is *not*

- **Not a closure.** You can bind a path, a mode, an ID — serializable data
  only. The same workflow may run in-process, from a packaged `.so`, or on a
  remote fleet agent; bound params travel with the run via the context.
- **Not a workflow version.** Params are instance data; they don't change the
  workflow's content hash.
