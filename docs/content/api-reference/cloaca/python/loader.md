# cloaca.python.loader <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


Python workflow package loader.

Imports a Python workflow module via PyO3, triggering `@task` decorator
registration, then collects the registered tasks and builds the workflow.
This is the bridge between extracted `.cloacina` packages and the
cloacina task execution engine.

## Functions

### `cloaca.python.loader.py_var`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">py_var</span>(name: str) -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::loader::py_var](../../rust/cloacina/python/loader.md#fn-py_var)

Python binding: `cloaca.var(name)` — resolve a `CLOACINA_VAR_{NAME}` env var.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |


<details>
<summary>Source</summary>

```python
fn py_var(name: &str) -> PyResult<String> {
    crate::var(name).map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
}
```

</details>



### `cloaca.python.loader.py_var_or`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">py_var_or</span>(name: str, default: str) -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::loader::py_var_or](../../rust/cloacina/python/loader.md#fn-py_var_or)

Python binding: `cloaca.var_or(name, default)` — resolve with a fallback.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `default` | `str` |  |


<details>
<summary>Source</summary>

```python
fn py_var_or(name: &str, default: &str) -> String {
    crate::var_or(name, default)
}
```

</details>
