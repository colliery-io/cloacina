---
title: "cloacina::registry::loader::ffi_trigger"
description: "Host-side `Trigger` adapter that dispatches `poll()` through a packaged cdylib via fidius FFI."
sidebar:
  order: 177
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Host-side `Trigger` adapter that dispatches `poll()` through a packaged cdylib via fidius FFI.

Background: `Runtime::seed_from_inventory` only sees `inventory::iter`
entries that were submitted by code linked against the SAME
compilation of `cloacina-workflow-plugin`. Independently-compiled
cdylibs (every fixture under `examples/fixtures/*` and most user
workflows) are separate workspaces with their own `cloacina-workflow-
plugin` build, so their inventory submissions land in a private
linker section the host can't enumerate. Instead of giving up on
workflow-trigger subscriptions for those packages, the reconciler
builds an `FfiTriggerImpl` per FFI-declared trigger: the host-side
impl looks like a normal `cloacina_workflow::Trigger`, but its
`poll()` calls method index 6 (`invoke_trigger_poll`) on the
cdylib's plugin handle, which walks the cdylib's local inventory
and runs the user's `poll()` body on the cdylib's own tokio
runtime.

## Structs

### <span id="struct.FfiTriggerImpl"></span>`struct FfiTriggerImpl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Host-side `Trigger` impl that proxies to a packaged cdylib through fidius. Cached metadata (`name`, `poll_interval`, `allow_concurrent`, `cron_expression`) comes from `get_trigger_metadata` at registration time, so the synchronous accessors don't cross the FFI boundary — only `poll()` does.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `handle` | `Arc < fidius_host :: PluginHandle >` |  |
| `name` | `String` |  |
| `poll_interval` | `Duration` |  |
| `allow_concurrent` | `bool` |  |
| `cron_expression` | `Option < String >` |  |

#### Methods

##### <span id="FfiTriggerImpl.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (handle : Arc < fidius_host :: PluginHandle > , name : String , poll_interval : Duration , allow_concurrent : bool , cron_expression : Option < String > ,) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(
        handle: Arc<fidius_host::PluginHandle>,
        name: String,
        poll_interval: Duration,
        allow_concurrent: bool,
        cron_expression: Option<String>,
    ) -> Self {
        Self {
            handle,
            name,
            poll_interval,
            allow_concurrent,
            cron_expression,
        }
    }
```

</details>





