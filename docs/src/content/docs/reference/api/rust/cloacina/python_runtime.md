---
title: "cloacina::python_runtime"
description: "Indirection layer between the reconciler and the Python runtime."
editUrl: false
sidebar:
  order: 154
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Indirection layer between the reconciler and the Python runtime.

Python-language workflow packages need a pyo3-backed runtime to import
user code and register tasks. That runtime lives behind the
[`PythonRuntime`](../../../rust/cloacina/python_runtime/#trait.PythonRuntime) trait so it can move into a separate crate
(`cloacina-python`, CLOACI-T-0529) — binaries that don't execute
Python (e.g. `cloacina-compiler`) simply don't link the impl and
therefore don't drag in pyo3 / `Python3.framework`.

A process that needs Python support calls [`register_python_runtime`](../../../rust/cloacina/python_runtime/#fn.register_python_runtime)
once at startup. The reconciler looks the registration up via
[`python_runtime`](../../../rust/cloacina/python_runtime/#fn.python_runtime); if nothing is registered, Python packages fail
with a clear `not attached` error at reconcile time.

## Structs

### <span id="struct.PythonTaskNode"></span><code style="font-size: 0.85em">struct PythonTaskNode</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

One task in a loaded Python workflow, with its dependency edges — captured host-side (from the scoped Runtime, before it is dropped) so the reconciler can persist the task DAG into package metadata and the UI can render it like Rust workflows. (CLOACI-T-0672)

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `String` | Local task id (e.g. `"finish"`). |
| `dependencies` | `Vec<String>` | Local ids of the tasks this task depends on (e.g. `["prepare"]`). |

### <span id="struct.LoadedPythonWorkflow"></span><code style="font-size: 0.85em">struct LoadedPythonWorkflow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Result of loading a Python workflow package.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_namespaces` | `Vec<TaskNamespace>` | Tasks registered in the global task registry under their fully-qualified namespace. The reconciler tracks these so it can unregister on unload. |
| `tasks` | `Vec<PythonTaskNode>` | Per-task dependency edges (local ids), for persisting/rendering the task DAG. Mirrors the Rust path's `PackageMetadata.tasks`. (CLOACI-T-0672) |
| `workflow_name` | `String` | Name of the workflow registered in the global workflow registry. |

## Traits

### <span id="trait.PythonRuntime"></span><code style="font-size: 0.85em">trait PythonRuntime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Runtime backing Python-language package loading.

Implementations run on the calling thread (typically inside a
`spawn_blocking`) — they're responsible for initializing the Python
interpreter as needed. Errors are flattened to `String` because the
reconciler wraps them in `RegistryError::RegistrationFailed` anyway.

#### Methods

##### <span id="PythonRuntime.load_workflow_package"></span><code>load_​workflow_​package</code>

```rust
fn load_workflow_package(
    &self,
    archive_data: &[u8],
    staging_dir: &Path,
    tenant_id: &str,
    runtime: &Arc<Runtime>,
) -> Result<LoadedPythonWorkflow, String>
```

Extract a `.cloacina` archive, import the entry module, and register its tasks + triggers in the global registries for the given tenant.

<details>
<summary>Source</summary>

```rust
    fn load_workflow_package(
```

</details>

##### <span id="PythonRuntime.load_cg_package"></span><code>load_​cg_​package</code>

```rust
fn load_cg_package(
    &self,
    archive_data: &[u8],
    staging_dir: &Path,
    tenant_id: &str,
    graph_name: &str,
    entry_module: &str,
    accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
    runtime: &Arc<Runtime>,
) -> Result<Option<ComputationGraphDeclaration>, String>
```

Extract + import a Python computation graph package, then build the [`ComputationGraphDeclaration`](../../../rust/cloacina/computation_graph/scheduler/#struct.ComputationGraphDeclaration) the `ComputationGraphScheduler` loads. Returns `None` if the imported module registered no executor for `graph_name` — matches the prior behavior where the caller silently moves on.

<details>
<summary>Source</summary>

```rust
    fn load_cg_package(
```

</details>

## Functions

### <span id="fn.register_python_runtime"></span><code style="font-size: 0.85em">fn register_​python_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_python_runtime(runtime: Arc<dyn PythonRuntime>)
```

Install a [`PythonRuntime`](../../../rust/cloacina/python_runtime/#trait.PythonRuntime) implementation for this process. Only the first call wins — subsequent calls are silently ignored. Processes with no Python responsibility (e.g. `cloacina-compiler`) simply never call this and Python packages fail at reconcile time with a clear error.

<details>
<summary>Source</summary>

```rust
pub fn register_python_runtime(runtime: Arc<dyn PythonRuntime>) {
    let _ = PYTHON_RUNTIME.set(runtime);
    init_python_runtime_health_metrics();
}
```

</details>

### <span id="fn.python_runtime"></span><code style="font-size: 0.85em">fn python_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn python_runtime() -> Option<Arc<dyn PythonRuntime>>
```

Fetch the registered [`PythonRuntime`](../../../rust/cloacina/python_runtime/#trait.PythonRuntime), if any. Returns `None` when no runtime is attached to this process.

<details>
<summary>Source</summary>

```rust
pub fn python_runtime() -> Option<Arc<dyn PythonRuntime>> {
    PYTHON_RUNTIME.get().cloned()
}
```

</details>

### <span id="fn.mark_python_runtime_wedged"></span><code style="font-size: 0.85em">fn mark_​python_​runtime_​wedged</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn mark_python_runtime_wedged(reason: impl Into<String>)
```

Latch the process-global "Python runtime is wedged" flag.

Called by `cloacina-python` when an import thread could not be interrupted
and had to be abandoned while (presumably) holding the GIL. Emits the
`cloacina_python_runtime_wedged` gauge and an `error!` naming the cause.
Repeat calls keep the FIRST reason — that's the one that actually wedged
the interpreter; later ones are collateral.

<details>
<summary>Source</summary>

```rust
pub fn mark_python_runtime_wedged(reason: impl Into<String>) {
    let reason = reason.into();
    let first = !PYTHON_RUNTIME_WEDGED.swap(true, Ordering::SeqCst);
    if first {
        if let Ok(mut slot) = PYTHON_RUNTIME_WEDGED_REASON.lock() {
            *slot = Some(reason.clone());
        }
    }
    metrics::gauge!("cloacina_python_runtime_wedged").set(1.0);
    tracing::error!(
        reason = %reason,
        first_wedge = first,
        "PYTHON RUNTIME WEDGED — the embedded interpreter is holding the GIL and \
         cannot be recovered in-process. All further Python package loads in this \
         process will hang or fail; /ready now reports not-ready. Restart the \
         process to recover, and remove/fix the offending package first."
    );
}
```

</details>

### <span id="fn.is_python_runtime_wedged"></span><code style="font-size: 0.85em">fn is_​python_​runtime_​wedged</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_python_runtime_wedged() -> bool
```

Whether the embedded Python interpreter is known-wedged in this process.

<details>
<summary>Source</summary>

```rust
pub fn is_python_runtime_wedged() -> bool {
    PYTHON_RUNTIME_WEDGED.load(Ordering::SeqCst)
}
```

</details>

### <span id="fn.python_runtime_wedged_reason"></span><code style="font-size: 0.85em">fn python_​runtime_​wedged_​reason</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn python_runtime_wedged_reason() -> Option<String>
```

Reason the Python runtime is wedged, or `None` when it is healthy (or when Python is not used in this process at all).

<details>
<summary>Source</summary>

```rust
pub fn python_runtime_wedged_reason() -> Option<String> {
    if !is_python_runtime_wedged() {
        return None;
    }
    PYTHON_RUNTIME_WEDGED_REASON
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .or_else(|| Some("python runtime wedged (reason unavailable)".to_string()))
}
```

</details>

### <span id="fn.record_python_import_interrupted"></span><code style="font-size: 0.85em">fn record_​python_​import_​interrupted</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn record_python_import_interrupted()
```

Record that a hung Python import thread WAS successfully interrupted (`PyThreadState_SetAsyncExc` landed and the thread joined). The package load failed but the interpreter stayed healthy — the good outcome of the ladder.

<details>
<summary>Source</summary>

```rust
pub fn record_python_import_interrupted() {
    metrics::counter!("cloacina_python_import_interrupted_total").increment(1);
}
```

</details>

### <span id="fn.reset_python_runtime_wedged_for_tests"></span><code style="font-size: 0.85em">fn reset_​python_​runtime_​wedged_​for_​tests</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn reset_python_runtime_wedged_for_tests()
```

Clear the wedged flag. **Test support only.**

The flag is latching in production for a reason (a wedged interpreter does
not heal), but it is process-global, so a test that sets it would poison
every later test in the same binary. Downstream crates (`cloacina-server`)
need this to assert the `/ready` effect and then put the process back.

<details>
<summary>Source</summary>

```rust
pub fn reset_python_runtime_wedged_for_tests() {
    PYTHON_RUNTIME_WEDGED.store(false, Ordering::SeqCst);
    if let Ok(mut slot) = PYTHON_RUNTIME_WEDGED_REASON.lock() {
        *slot = None;
    }
    metrics::gauge!("cloacina_python_runtime_wedged").set(0.0);
}
```

</details>

### <span id="fn.init_python_runtime_health_metrics"></span><code style="font-size: 0.85em">fn init_​python_​runtime_​health_​metrics</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn init_python_runtime_health_metrics()
```

Publish the wedged gauge as `0` so the series exists on healthy processes (a gauge that only appears when broken is invisible to alert rules that need a baseline). Called once when a Python runtime is registered.

<details>
<summary>Source</summary>

```rust
pub fn init_python_runtime_health_metrics() {
    if !is_python_runtime_wedged() {
        metrics::gauge!("cloacina_python_runtime_wedged").set(0.0);
    }
}
```

</details>
