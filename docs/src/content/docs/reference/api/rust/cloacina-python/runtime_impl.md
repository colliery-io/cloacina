---
title: "cloacina-python::runtime_impl"
description: "`cloacina-python`'s implementation of `cloacina::python_runtime::PythonRuntime`."
editUrl: false
sidebar:
  order: 22
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

`cloacina-python`'s implementation of `cloacina::python_runtime::PythonRuntime`.

A process that wants Python support (the server today, any future
Python-enabled service) calls [`install`](../../../rust/cloacina-python/runtime_impl/#fn.install) at startup to register this
runtime into cloacina core's dispatch slot. After that, uploaded
Python packages are loaded through the runtime just like Rust ones
are loaded through fidius.

## Structs

### <span id="struct.CloacinaPythonRuntime"></span><code style="font-size: 0.85em">struct CloacinaPythonRuntime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Zero-state runtime — every method thin-wraps the crate-local helpers.

## Functions

### <span id="fn.install"></span><code style="font-size: 0.85em">fn install</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn install()
```

Install [`CloacinaPythonRuntime`](../../../rust/cloacina-python/runtime_impl/#struct.CloacinaPythonRuntime) as this process's Python runtime. Idempotent — subsequent calls are no-ops. Call once at startup from any binary that wants to load Python packages.

<details>
<summary>Source</summary>

```rust
pub fn install() {
    register_python_runtime(Arc::new(CloacinaPythonRuntime));
}
```

</details>
