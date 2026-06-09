# cloacina::python_runtime <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Indirection layer between the reconciler and the Python runtime.

Python-language workflow packages need a pyo3-backed runtime to import
user code and register tasks. That runtime lives behind the
[`PythonRuntime`] trait so it can move into a separate crate
(`cloacina-python`, CLOACI-T-0529) — binaries that don't execute
Python (e.g. `cloacina-compiler`) simply don't link the impl and
therefore don't drag in pyo3 / `Python3.framework`.
A process that needs Python support calls [`register_python_runtime`]
once at startup. The reconciler looks the registration up via
[`python_runtime`]; if nothing is registered, Python packages fail
with a clear `not attached` error at reconcile time.

## Structs

### `cloacina::python_runtime::LoadedPythonWorkflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Result of loading a Python workflow package.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_namespaces` | `Vec < TaskNamespace >` | Tasks registered in the global task registry under their fully-qualified
namespace. The reconciler tracks these so it can unregister on unload. |
| `workflow_name` | `String` | Name of the workflow registered in the global workflow registry. |



## Functions

### `cloacina::python_runtime::register_python_runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_python_runtime (runtime : Arc < dyn PythonRuntime >)
```

Install a [`PythonRuntime`] implementation for this process. Only the first call wins — subsequent calls are silently ignored. Processes with no Python responsibility (e.g. `cloacina-compiler`) simply never call this and Python packages fail at reconcile time with a clear error.

<details>
<summary>Source</summary>

```rust
pub fn register_python_runtime(runtime: Arc<dyn PythonRuntime>) {
    let _ = PYTHON_RUNTIME.set(runtime);
}
```

</details>



### `cloacina::python_runtime::python_runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn python_runtime () -> Option < Arc < dyn PythonRuntime > >
```

Fetch the registered [`PythonRuntime`], if any. Returns `None` when no runtime is attached to this process.

<details>
<summary>Source</summary>

```rust
pub fn python_runtime() -> Option<Arc<dyn PythonRuntime>> {
    PYTHON_RUNTIME.get().cloned()
}
```

</details>
