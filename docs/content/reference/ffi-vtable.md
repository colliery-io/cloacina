---
title: "FFI Vtable Reference"
description: "Method-by-method specification of the CloacinaPlugin FFI vtable: indices 0-10, interface version 5, optional-since semantics, and wire types."
weight: 31
aliases:
  - "/platform/reference/ffi-vtable/"

---

# FFI Vtable Reference

Cloacina plugins (`.cloacina` packages) export a fixed FFI vtable that
the host calls by **positional index**. The vtable is declared by the
`CloacinaPlugin` trait in `crates/cloacina-workflow-plugin/src/lib.rs`
and is dispatched at runtime by fidius — the plugin framework Cloacina
uses to load shared libraries and call into them by index (published on
crates.io as the `fidius` / `fidius-core` / `fidius-host` family). The
host-side fidius API is provided by the `fidius-host` crate; the
plugin-side helpers come from `fidius-core`. The fidius crates are the
ABI authority: the descriptor layout (`PluginDescriptor` /
`PluginRegistry`, the `FIDIUS\0\0` magic, the FNV-1a `interface_hash`
drift detector) is defined there, and the wire format is **bincode**.

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
pub const METHOD_GET_INPUT_INTERFACE: usize = 9;
pub const METHOD_GET_CONSTRUCTOR_METADATA: usize = 10;
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
this at load time (step 6 of the [reconciler pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}})) to register a `DynamicLibraryTask` constructor in the host
`Runtime` per declared task.

## Method Index 1 — `execute_task`

| | |
|---|---|
| Wire input | `TaskExecutionRequest { task_name: String, context_json: String, resolved_secrets: BTreeMap<String, BTreeMap<String, String>> }` |
| Wire output | `Result<TaskExecutionResult, PluginError>` (with `success: bool`, `context_json: Option<String>`, `error: Option<String>`) |
| Optional since | — |

Executes a named task with a JSON-serialized context. The host calls
this on the executor's blocking thread; the cdylib runs the task on
its own tokio runtime. The result's `context_json` carries the updated
context back across the boundary.

`resolved_secrets` (added in interface version 5, CLOACI-T-0895) carries
the values of every `{"$secret"}`-referenced secret, keyed by concrete
secret name → `{field: value}`: a resolver object cannot cross the
plugin boundary, so the host resolves secrets up front and the plugin
shell re-attaches them via a `MapSecretResolver` so `context.secret(...)`
works identically inside the package. The map is empty when the task
references no secrets; the struct's hand-written `Debug` impl prints
secret names only, never values.

## Method Index 2 — `get_graph_metadata`

| | |
|---|---|
| Wire input | `()` |
| Wire output | `Result<GraphPackageMetadata, PluginError>` |
| Optional since | — |

Returns the package's primary reactor-bound computation graph metadata
— a holdover slot from the pre-CLOACI-I-0101 1:1 reactor-per-graph model.
As of I-0101, reactors are declared standalone via `#[reactor(...)]` and
graphs bind to them via `trigger = reactor("name")` (see [Reactor
Lifecycle]({{< ref "/engine/explanation/reactor-lifecycle" >}})),
so the "synthesized-reactor" form this method historically described no
longer exists. The metadata still carries name, reaction mode (`when_any`
/ `when_all`), input strategy (`latest` / `sequential`), and accumulator
declarations for whichever reactor-bound graph the package nominates here.
Packages without a CG return `PluginError`; the reconciler treats that as
"no primary CG" and skips step 5.

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
via `#[task(invokes = computation_graph("graph_name"))]`. The metadata entry carries the
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

## Method Index 9 — `get_input_interface`

| | |
|---|---|
| Wire input | `()` |
| Wire output | `Result<InputInterfaceDescriptor, PluginError>` (with `entries: Vec<InputInterfaceEntry>`) |
| Optional since | **v3** — pre-v3 plugins return `CallError::NotImplemented` |

Returns the package's declared **input interface** — the typed,
injectable surfaces a package exposes (CLOACI-I-0128 / T-0756). The
workflow-surface entry carries the workflow's declared params; later
surface kinds (`accumulator`, `reactor`) carry their boundary slots.
Each `InputInterfaceEntry` is `{ surface_kind, surface_name, slots_json }`,
where `slots_json` is a JSON array of `cloacina_api_types::InputSlot`
kept as a string so the fidius wire stays simple — the host parses it.

This is a **dedicated FFI descriptor entrypoint**, deliberately kept
separate from `get_task_metadata`'s `TaskMetadataEntry` wire struct:
piggy-backing the interface on the per-task metadata ABI is drift-prone
(every field add reshapes a struct every package serializes), so the
interface rides its own optional method instead. The per-task metadata
ABI is left untouched.

The host (`package_loader`) calls this at load time and treats
`CallError::NotImplemented` — or any other call error — as **"no
declared interface"** (an empty descriptor): `surface_kind == "workflow"`
entries flow into the package's declared params, and other kinds become
declared surfaces. To declare params or expose typed interfaces, a
package must be compiled against interface version 3 or later — the
unified `cloacina_workflow_plugin::package!()` shell emits this method
and walks `inventory::iter::<WorkflowDescriptorEntry>` to populate it.

## Method Index 10 — `get_constructor_metadata`

| | |
|---|---|
| Wire input | `()` |
| Wire output | `Result<Vec<ConstructorPackageMetadata>, PluginError>` |
| Optional since | **v4** — pre-v4 plugins return `CallError::NotImplemented` |

Returns the package's declared `constructor!(...)` DAG nodes
(CLOACI-T-0832). A packaged cdylib cannot link the WASM constructor
loader, so it *declares* each node here; the host — which links
wasmtime — resolves the provider via
`load_constructor_node(.., GrantSpec::from_pairs(grants))` and injects
the resulting task into the rebuilt workflow DAG. Each
`ConstructorPackageMetadata` carries `{ workflow, id, from, constructor,
config: Vec<(String, String)>, grants: Vec<(String, Vec<String>)>,
dependencies }`; `config` values are JSON-encoded *strings* (not
`serde_json::Value` — `deserialize_any` is unsupported on the fidius
bincode wire), and the host parses each value back with
`serde_json::from_str` before binding. The host treats `NotImplemented`
as "package declares no constructor nodes". The unified
`cloacina_workflow_plugin::package!()` shell walks
`inventory::iter::<ConstructorEntry>` to populate it.

## Python Plugins and Host-Build Requirements

Python `.cloacina` packages are loaded via PyO3 rather than the FFI
vtable — they do not implement `CloacinaPlugin` directly. However,
the host must be compiled **with Python support** to load them. A
host built without the Python feature will reject Python packages at
load time with:

```text
RegistryError::RegistrationFailed(
    "Python package <name> received but no PythonRuntime is attached"
)
```

If you operate a multi-language deployment, ensure your host build
includes Python support (or run separate hosts per language). Rust-
only packages have no such requirement; the FFI vtable is
language-neutral.

## ABI Stability and Versioning

- The trait is annotated `#[fidius::plugin_interface(version = 5,
  buffer = PluginAllocated)]`. fidius-host computes an
  `INTERFACE_HASH` from the trait shape; mismatched hashes are
  rejected at load time, preventing silent ABI drift.
- Version history:
  - **2 → 3** (CLOACI-I-0128 / T-0756): added `get_input_interface`
    at method index 9, `#[optional(since = 3)]`.
  - **3 → 4** (CLOACI-T-0832): added `get_constructor_metadata` at
    method index 10, `#[optional(since = 4)]`.
  - **4 → 5** (CLOACI-T-0895): no new method — `TaskExecutionRequest`
    gained the `resolved_secrets` wire field, a **bincode layout
    change**. Unlike the additive bumps, this one is a hard gate:
    stale pre-v5 artifacts must fail the version check at load rather
    than mis-decode the request.
- Method-additive bumps are backward-compatible: methods 4–8 are
  `#[optional(since = 2)]`, 9 is `#[optional(since = 3)]`, 10 is
  `#[optional(since = 4)]`; older plugins return
  `CallError::NotImplemented` for methods they don't emit and the host
  treats that as "declares none".
- Adding a method requires bumping the version, marking the new
  method `#[optional(since = N)]`, and adding the canonical method-index
  constant in the same edit. The unified [`cloacina_workflow_plugin::package!()`]({{< ref "/reference/package-shell-macro" >}})
  shell macro emits the new method automatically.
- Deleting or reordering a method is a hard breaking change. Don't.

## Related

- [`package!()` macro reference]({{< ref "/reference/package-shell-macro" >}}) — what emits this vtable.
- [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}}) — how the host consumes the vtable across the six load steps.
- [Inventory and Runtime Seeding]({{< ref "/engine/explanation/inventory-and-runtime-seeding" >}}) — why these methods exist.
