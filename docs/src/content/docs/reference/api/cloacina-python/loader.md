---
title: "cloacina-python.loader"
description: "Python workflow package loader."
sidebar:
  order: 9
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


Python workflow package loader.

Imports a Python workflow module via PyO3, triggering `@task` decorator
registration, then collects the registered tasks and builds the workflow.
This is the bridge between extracted `.cloacina` packages and the
cloacina task execution engine.

## Functions

### <span id="fn.py_var"></span>`py_var`

```python
def py_var(name:  str) -> PyResult < String >
```

> **Rust Implementation**: [cloacina-python::loader::py_var](../../rust/cloacina-python/loader/#fn.py_var)

Python binding: `cloaca.var(name)` — resolve a `CLOACINA_VAR_{NAME}` env var.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | ` str` |  |


<details>
<summary>Source</summary>

```python
pub(crate) fn py_var(name: &str) -> PyResult<String> {
    cloacina::var(name).map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
}
```

</details>



### <span id="fn.py_var_or"></span>`py_var_or`

```python
def py_var_or(name:  str, default:  str) -> str
```

> **Rust Implementation**: [cloacina-python::loader::py_var_or](../../rust/cloacina-python/loader/#fn.py_var_or)

Python binding: `cloaca.var_or(name, default)` — resolve with a fallback.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | ` str` |  |
| `default` | ` str` |  |


<details>
<summary>Source</summary>

```python
pub(crate) fn py_var_or(name: &str, default: &str) -> String {
    cloacina::var_or(name, default)
}
```

</details>



