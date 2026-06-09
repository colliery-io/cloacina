# cloacina::registry::loader::ffi_trigger <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


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

### `cloacina::registry::loader::ffi_trigger::FfiTriggerImpl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
