---
title: "cloacina-python::runtime_scope"
description: "Thread-local \"current Runtime\" slot used by Python decorators/loader."
sidebar:
  order: 39
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Thread-local "current Runtime" slot used by Python decorators/loader.

CLOACI-T-0509 step 2: instead of pushing into the process-global
task/workflow/trigger registries, Python registration paths resolve
a scoped [`cloacina::Runtime`] via [`current_runtime`]. The caller
(typically [`crate::runtime_impl::CloacinaPythonRuntime`]) installs a
[`ScopedRuntime`] around the Python import so decorators see it.

## Structs

### <span id="struct.ScopedRuntime"></span>`struct ScopedRuntime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


RAII guard that installs a Runtime into the thread-local slot for the duration of its lifetime. Errors on `new` if a runtime is already installed on this thread — nesting would silently corrupt the caller's registration target.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `_private` | `()` |  |

#### Methods

##### <span id="ScopedRuntime.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (runtime : Arc < Runtime >) -> Result < Self , String >
```

Install `runtime` into the thread-local slot. Errors if another runtime is already installed on this thread.

<details>
<summary>Source</summary>

```rust
    pub fn new(runtime: Arc<Runtime>) -> Result<Self, String> {
        CURRENT_RUNTIME.with(|slot| {
            let mut borrow = slot.borrow_mut();
            if borrow.is_some() {
                return Err(
                    "ScopedRuntime::new called while a Runtime is already installed on this thread"
                        .to_string(),
                );
            }
            *borrow = Some(runtime);
            Ok(())
        })?;
        Ok(Self { _private: () })
    }
```

</details>





## Functions

### <span id="fn.current_runtime"></span>`fn current_runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn current_runtime () -> Option < Arc < Runtime > >
```

Fetch the Runtime currently installed on this thread, if any.

<details>
<summary>Source</summary>

```rust
pub fn current_runtime() -> Option<Arc<Runtime>> {
    CURRENT_RUNTIME.with(|slot| slot.borrow().clone())
}
```

</details>



