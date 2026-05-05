---
title: "FFI Vtable Reference"
description: "Method-by-method specification of the CloacinaPlugin FFI vtable: indices 0-8, optional-since-v2 semantics, and wire types."
weight: 31
---

# FFI Vtable Reference

Cloacina plugins (`.cloacina` packages) export a fixed FFI vtable that
the host calls by **positional index**. The vtable is declared by the
`CloacinaPlugin` trait in `crates/cloacina-workflow-plugin/src/lib.rs`
and is dispatched at runtime by [fidius](https://github.com/colliery-software/fidius).

The canonical method indices are exported as constants from
`cloacina-workflow-plugin`:

```rust
pub const METHOD_GET_TASK_METADATA: usize = 0;
pub const METHOD_EXECUTE_TASK: usize = 1;
pub const METHOD_GET_GRAPH_METADATA: usize = 2;
pub const METHOD_EXECUTE_GRAPH: usize = 3;
pub const METHOD_GET_REACTOR_METADATA: usize = 4;
pub const METHOD_GET_TRIGGER_METADATA: usize = 5;
pub const METHOD_INVOKE_TRIGGER_POLL: usize = 6;
pub const METHOD_GET_TRIGGERLESS_GRAPH_METADATA: usize = 7;
pub const METHOD_INVOKE_TRIGGERLESS_GRAPH: usize = 8;
```

Both the trait declaration and the constants live in the same file, so
any reorder or addition forces a re-numbering in the same diff. The
host re-exports these constants from
`cloacina::computation_graph::packaging_bridge` so existing call sites
don't have to change their import path.

## Method Index 0 — `get_task_metadata`

| | |
|---|---|
| Wire input | `()` |
| Wire output | `Result<PackageTasksMetadata, PluginError>` |
| Optional since | — (always present) |

Returns the package's full task catalog — for each task, its namespace,
dependency list, description, and source location. The reconciler calls
this at load time (step 6 of the [reconciler pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}})) to register a `DynamicLibraryTask` constructor in the host
`Runtime` per declared task.

## Method Index 1 — `execute_task`

| | |
|---|---|
| Wire input | `TaskExecutionRequest { task_name: String, context_json: String }` |
| Wire output | `Result<TaskExecutionResult, PluginError>` (with `success: bool`, `context_json: Option<String>`, `error: Option<String>`) |
| Optional since | — |

Executes a named task with a JSON-serialized context. The host calls
this on the executor's blocking thread; the cdylib runs the task on
its own tokio runtime. The result's `context_json` carries the updated
context back across the boundary.

## Method Index 2 — `get_graph_metadata`

| | |
|---|---|
| Wire input | `()` |
| Wire output | `Result<GraphPackageMetadata, PluginError>` |
| Optional since | — |

Returns the package's *bundled-form* computation graph metadata: name,
reaction mode (`when_any` / `when_all`), input strategy
(`latest` / `sequential`), and accumulator declarations. Packages
without a CG return `PluginError`; the reconciler treats that as "no
bundled CG" and skips step 5.

## Method Index 3 — `execute_graph`

| | |
|---|---|
| Wire input | `GraphExecutionRequest { cache: HashMap<String, String> }` |
| Wire output | `Result<GraphExecutionResult, PluginError>` (with `terminal_outputs_json: Option<Vec<String>>`) |
| Optional since | — |

Fires the bundled CG with a snapshot of accumulator boundary values.
The reactor calls this on every fire; the result's
`terminal_outputs_json` is the per-terminal-node serialized output.

## Method Index 4 — `get_reactor_metadata`

| | |
|---|---|
| Wire input | `()` |
| Wire output | `Result<Vec<ReactorPackageMetadata>, PluginError>` |
| Optional since | **v2** — pre-v2 plugins return `CallError::NotImplemented` |

Returns the package's reactor declarations (split-form CG support).
The reconciler treats `NotImplemented` and `Ok(Vec::new())`
identically — both mean "package declares no reactors" — and skips
the reactor load step for that package.

## Method Index 5 — `get_trigger_metadata`

| | |
|---|---|
| Wire input | `()` |
| Wire output | `Result<Vec<TriggerPackageMetadata>, PluginError>` |
| Optional since | **v2** |

Returns the package's trigger declarations. The reconciler routes
cron-shaped entries (where `cron_expression: Some(...)`) to the cron
scheduler via `CronWorkflowRegistrar`; non-cron entries get a host-side
`FfiTriggerImpl` adapter that proxies `Trigger::poll()` back into the
plugin via method 6.

## Method Index 6 — `invoke_trigger_poll`

| | |
|---|---|
| Wire input | `TriggerInvokeRequest { trigger_name: String }` |
| Wire output | `Result<TriggerInvokeResult, PluginError>` (with `fire: bool`, optional `context_json`) |
| Optional since | **v2** |

Polls a named trigger across the FFI boundary. Why this exists:
`inventory` entries do not span shared-library linker boundaries, so
the host cannot build a host-side `Arc<dyn Trigger>` directly from the
plugin's inventory section. The `FfiTriggerImpl` adapter caches the
trigger's metadata (poll interval, cron expression, allow-concurrent
flag) at registration time, so only the actual `poll()` call crosses
the boundary on each tick.

The host calls this on a `tokio::task::spawn_blocking` so the cdylib's
synchronous fidius dispatch doesn't block the host's async runtime
while user `poll()` code runs.

## Method Index 7 — `get_triggerless_graph_metadata`

| | |
|---|---|
| Wire input | `()` |
| Wire output | `Result<Vec<TriggerlessGraphMetadataEntry>, PluginError>` |
| Optional since | **v2** |

Returns trigger-less computation graphs declared by the package.
Trigger-less CGs are *not* bound to a reactor and don't consume
accumulator boundaries; they're invoked directly by workflow tasks
via `#[task(invokes = "graph_name")]`. The metadata entry carries the
graph name and its terminal-node-output names; the reconciler builds
host-side `TriggerlessGraphRegistration` adapters that dispatch
invocation through method 8.

## Method Index 8 — `invoke_triggerless_graph`

| | |
|---|---|
| Wire input | `TriggerlessGraphInvokeRequest { graph_name: String, context_json: String }` |
| Wire output | `Result<TriggerlessGraphInvokeResult, PluginError>` (with `terminal_outputs_json: Option<String>`) |
| Optional since | **v2** |

Invokes a named trigger-less CG with a workflow context. Same blocking
+ cross-runtime pattern as method 6: the cdylib's tokio runtime drives
the graph execution, the host receives the terminal outputs.

## ABI Stability and Versioning

- The trait is annotated `#[fidius::plugin_interface(version = 2,
  buffer = PluginAllocated)]`. fidius-host computes an
  `INTERFACE_HASH` from the trait shape; mismatched hashes are
  rejected at load time, preventing silent ABI drift.
- Adding a method requires bumping the version, marking the new
  method `#[optional(since = N)]`, and adding the canonical method-index
  constant in the same edit. The unified [`cloacina::package!()`]({{< ref "/platform/reference/package-shell-macro" >}})
  shell macro emits the new method automatically.
- Deleting or reordering a method is a hard breaking change. Don't.

## Related

- [`package!()` macro reference]({{< ref "/platform/reference/package-shell-macro" >}}) — what emits this vtable.
- [Reconciler Pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}}) — how the host consumes the vtable across the six load steps.
- [Inventory and Runtime Seeding]({{< ref "/platform/explanation/inventory-and-runtime-seeding" >}}) — why these methods exist.
