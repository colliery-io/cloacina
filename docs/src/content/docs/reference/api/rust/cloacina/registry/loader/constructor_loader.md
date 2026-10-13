---
title: "cloacina::registry::loader::constructor_loader"
description: "WASM constructor loader + primitive adapters (CLOACI-I-0132 / T-0823, T-0824)."
editUrl: false
sidebar:
  order: 158
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

WASM constructor loader + primitive adapters (CLOACI-I-0132 / T-0823, T-0824).

Loads a WASM **constructor** package (a `.wasm` component + a sidecar
`constructor.json` manifest) and wraps the configured fidius handle as the
cloacina primitive the manifest's `primitive_kind` names — a
[`crate::task::Task`](../../../../../rust/cloacina/registry/loader/constructor_loader/#enum.ConstructorBinding) (T-0823) or a [`crate::trigger::Trigger`](../../../../../rust/cloacina-workflow/trigger/#trait.Trigger) (T-0824) —
that the existing async executor / scheduler can run unchanged, then
registers it into a [`Runtime`](../../../../../rust/cloacina/runtime/#struct.Runtime) so it participates in workflows/schedules
exactly like a macro-authored one.

Gated behind the default-OFF `constructors-wasm` feature: enabling it turns on
`fidius-host`'s `wasm` feature (→ wasmtime/cranelift) and pulls
`fidius-macro` + `cloacina-constructor-contract`. The default cloacina build
pulls none of this (verified via `cargo tree`).

**Flow (per primitive)**

1. Read the package's `constructor.json` into an `ConstructorManifest`.
2. Require the `primitive_kind` the caller asked for and an
   `interface_version` matching that primitive's contract.
3. `load_wasm_configured(component, &config)` — config binds ONCE at load.
4. Wrap the handle as the matching adapter and run the **async↔sync bridge**:
   serialize the async input → `JSON(...Invocation)` → `spawn_blocking` the
   wasmtime `call_method` → parse `JSON(...Outcome)` → rebuild the async
   result, surfacing a failed outcome as the primitive's normal error.
   - TASK: `Context` → `TaskInvocation` / `TaskOutcome` → `Context`.
   - TRIGGER: `()` → `TriggerInvocation` / `PollOutcome` →
     [`TriggerResult`](../../../../../rust/cloacina-workflow/trigger/#enum.TriggerResult).

**Runtime registration**

[`load_constructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor) reads the manifest and dispatches on `primitive_kind`,
registering the configured primitive into a [`Runtime`](../../../../../rust/cloacina/runtime/#struct.Runtime) (Task →
[`Runtime::register_task`](../../../../../rust/cloacina/runtime/#Runtime.register_task), Trigger → [`Runtime::register_trigger`](../../../../../rust/cloacina/runtime/#Runtime.register_trigger)). The
registered constructor hands out `Arc` clones that share the one configured
fidius handle, so every `get_task`/`get_trigger` call dispatches into the
same sandboxed instance.

**Accumulator + reactor (CLOACI-T-0828)**

These two primitives are NOT plain `Runtime` constructors, so they are wired
differently from task/trigger:

  * ACCUMULATOR — [`WasmAccumulatorConstructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#struct.WasmAccumulatorConstructor) implements
    [`Accumulator`](../../../../../rust/cloacina/computation_graph/accumulator/#trait.Accumulator)
    directly; [`load_accumulator_constructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_accumulator_constructor) returns it for
    `accumulator_runtime` to drive. `Accumulator::process` is SYNC, so the
    bridge calls the blocking wasmtime `ingest` directly (no `spawn_blocking`).
  * REACTOR — the reactor's firing decision is now pluggable via the
    [`ReactorFireDecider`](../../../../../rust/cloacina/computation_graph/reactor/#trait.ReactorFireDecider) seam added to
    [`Reactor`](../../../../../rust/cloacina/computation_graph/reactor/#struct.Reactor)
    (`with_evaluator`). [`WasmReactorConstructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#struct.WasmReactorConstructor) implements it, bridging the
    async decision to a `spawn_blocking` `evaluate`. The bridge + seam are
    proven against `Reactor` directly; threading a reactor constructor through
    the CG SCHEDULER's package-load path is a documented remaining lift.

The `#[constructor]` authoring macro covers all four kinds (task/trigger T-0826,
accumulator/reactor T-0828).

## Structs

### <span id="struct.WasmTaskConstructor"></span><code style="font-size: 0.85em">struct WasmTaskConstructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A loaded, configured WASM task constructor wrapped as a cloacina [`Task`](../../../../../rust/cloacina-workflow/task/#trait.Task).

Holds the configured fidius `fidius_host::PluginHandle` (the
`configure` hook bound the constructor's config once at load, so per-call
`execute` dispatches on the already-configured persistent store). The handle
is held behind an `Arc` so the async bridge can hand a `'static + Send`
clone to `spawn_blocking`; concurrent calls are serialized inside the handle
(the WASM backend guards its store with a mutex).

#### Methods

##### <span id="WasmTaskConstructor.name"></span><code>name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn name(&self) -> &str
```

The constructor's declared name (its task id).

<details>
<summary>Source</summary>

```rust
    pub fn name(&self) -> &str {
        &self.id
    }
```

</details>

### <span id="struct.TriggerBinding"></span><code style="font-size: 0.85em">struct TriggerBinding</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Host-side scheduling metadata bound to a trigger constructor at load.

The WASM guest's `poll` decides *whether* to fire; the *cadence* and the
scheduler-routing metadata (poll interval, cron expression, target workflow,
concurrency) are host concerns the guest never sees. The constructor manifest
has no poll-interval field (it describes the component, not a deployment), so
these are bound here — the trigger analogue of "config binds once at load".

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `poll_interval` | `Duration` | How often the reconciler polls this trigger (ignored for cron triggers). |
| `allow_concurrent` | `bool` | Whether concurrent fires with the same context hash are allowed. |
| `workflow_name` | `String` | The workflow this trigger fires (the `#[trigger(on = "...")]` binding). |
| `cron_expression` | `Option<String>` | Optional cron expression; `Some` routes to the cron scheduler instead of the poll-interval trigger registry. |

### <span id="struct.WasmTriggerConstructor"></span><code style="font-size: 0.85em">struct WasmTriggerConstructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A loaded, configured WASM trigger constructor wrapped as a cloacina [`Trigger`](../../../../../rust/cloacina-workflow/trigger/#trait.Trigger).

Mirrors [`WasmTaskConstructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#struct.WasmTaskConstructor): holds the configured fidius handle behind an
`Arc` so the async `poll` bridge can hand a `'static + Send` clone to
`spawn_blocking`. The host-side [`TriggerBinding`](../../../../../rust/cloacina/registry/loader/constructor_loader/#struct.TriggerBinding) supplies the scheduling
metadata the `Trigger` trait exposes (`poll_interval`, `workflow_name`, …).

### <span id="struct.WasmAccumulatorConstructor"></span><code style="font-size: 0.85em">struct WasmAccumulatorConstructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A loaded, configured WASM accumulator constructor wrapped as a cloacina `Accumulator`.

Holds the configured fidius handle behind an `Arc`; each event's `process`
dispatches the sync `ingest` into the already-configured sandbox. Its
`Output` is the boundary's JSON **bytes** (`Vec<u8>`): the
`BoundarySender` then bincode-serializes that to `bincode(json_bytes)` —
exactly the canonical boundary frame the reactor's FFI bridge /
[`capture_fire_inputs`](../../../../../rust/cloacina/computation_graph/reactor/) decode. Emitting a
`serde_json::Value` would NOT round-trip (bincode is not self-describing), so
the JSON-bytes shape is both correct and downstream-compatible.

#### Methods

##### <span id="WasmAccumulatorConstructor.name"></span><code>name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn name(&self) -> &str
```

The accumulator constructor's declared name.

<details>
<summary>Source</summary>

```rust
    pub fn name(&self) -> &str {
        &self.name
    }
```

</details>

### <span id="struct.ProviderStreamSource"></span><code style="font-size: 0.85em">struct ProviderStreamSource</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

An `EventSource` that drains a native provider's server-streaming `source` (a `fidius_host::ChunkStream` from `fidius_host::PluginHandle::call_streaming`) and pushes each boundary-JSON item onto the accumulator merge channel.

The held `Arc` of `fidius_host::PluginHandle` keeps the dlopen'd
provider (and thus its stream producer) alive for the stream's lifetime.
Dropping this source drops the `ChunkStream`, which tears the producer down —
the fidius host-pull backpressure/teardown contract (REQ-003), matching how an
`EventSource` shutdown tears a Kafka consumer down.

### <span id="struct.WasmReactorConstructor"></span><code style="font-size: 0.85em">struct WasmReactorConstructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A loaded, configured WASM reactor constructor.

Holds the configured fidius handle behind an `Arc`. Exposes the firing
decision two ways:
  * [`WasmReactorConstructor::evaluate`](../../../../../rust/cloacina/registry/loader/constructor_loader/#WasmReactorConstructor.evaluate) — call the bridge directly with a
    pre-serialized boundaries JSON (used by tests / a future scheduler), and
  * the [`ReactorFireDecider`](../../../../../rust/cloacina/computation_graph/reactor/#trait.ReactorFireDecider) impl — what a live [`Reactor`](../../../../../rust/cloacina-computation-graph/#trait.Reactor) consults via
    `with_evaluator` so the WASM guest IS the firing criteria.

#### Methods

##### <span id="WasmReactorConstructor.name"></span><code>name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn name(&self) -> &str
```

The reactor constructor's declared name.

<details>
<summary>Source</summary>

```rust
    pub fn name(&self) -> &str {
        &self.name
    }
```

</details>

##### <span id="WasmReactorConstructor.evaluate"></span><code>evaluate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn evaluate(
    &self,
    boundaries_json: String,
) -> Result<ReactorOutcome, LoaderError>
```

Bridge the firing decision: serialize-free `boundaries_json` in, `ReactorOutcome` out. Hops into the blocking wasmtime `evaluate` on a `spawn_blocking` thread (mirrors the task/trigger bridge).

<details>
<summary>Source</summary>

```rust
    pub async fn evaluate(&self, boundaries_json: String) -> Result<ReactorOutcome, LoaderError> {
        let name = self.name.clone();
        let inv_json =
            serde_json::to_string(&ReactorInvocation { boundaries_json }).map_err(|e| {
                LoaderError::Validation {
                    reason: format!("reactor constructor '{name}': serialize invocation: {e}"),
                }
            })?;

        let handle = self.handle.clone();
        let call_name = name.clone();
        let out_json: String = tokio::task::spawn_blocking(move || {
            handle.call_method::<_, String>(METHOD_EVALUATE, &(inv_json,))
        })
        .await
        .map_err(|e| LoaderError::Validation {
            reason: format!("reactor constructor '{call_name}': evaluate join: {e}"),
        })?
        .map_err(|e| LoaderError::Validation {
            reason: format!("reactor constructor '{call_name}': evaluate FFI call: {e}"),
        })?;

        serde_json::from_str::<ReactorOutcome>(&out_json).map_err(|e| LoaderError::Validation {
            reason: format!("reactor constructor '{name}': parse evaluate outcome: {e}"),
        })
    }
```

</details>

### <span id="struct.ScopedProviderSearch"></span><code style="font-size: 0.85em">struct ScopedProviderSearch</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

RAII guard installing a [`ProviderScope`](../../../../../rust/cloacina/registry/loader/constructor_loader/#enum.ProviderScope) on the current thread for the duration of one package load (CLOACI-T-0925).

Nests: the previous scope is saved and restored on drop, so a load that
transitively triggers another scoped load cannot strand the outer scope.

**Threads, not futures.** Hold it across synchronous work only (a Python
module import, a `spawn_blocking` closure). A guard held across an `.await`
would follow the THREAD, not the task, once the executor moves the future —
async resolution sites take the directory as an explicit argument instead.

#### Methods

##### <span id="ScopedProviderSearch.enter"></span><code>enter</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enter(scope: ProviderScope) -> Self
```

Install `scope` on this thread until the guard drops.

<details>
<summary>Source</summary>

```rust
    pub fn enter(scope: ProviderScope) -> Self {
        Self::enter_opt(Some(scope))
    }
```

</details>

##### <span id="ScopedProviderSearch.enter_opt"></span><code>enter_​opt</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enter_opt(scope: Option<ProviderScope>) -> Self
```

Install `scope` (or uninstall any scope, for `None`) on this thread until the guard drops. `None` is what an unscoped caller propagates.

<details>
<summary>Source</summary>

```rust
    pub fn enter_opt(scope: Option<ProviderScope>) -> Self {
        let previous = CURRENT_PROVIDER_SCOPE.with(|slot| slot.replace(scope));
        Self { previous }
    }
```

</details>

##### <span id="ScopedProviderSearch.for_staged_root"></span><code>for_​staged_​root</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn for_staged_root(root: Option<std::path::PathBuf>) -> Self
```

Install the scope for a package whose bundled providers were staged into `root` (`None` = the package bundles none).

<details>
<summary>Source</summary>

```rust
    pub fn for_staged_root(root: Option<std::path::PathBuf>) -> Self {
        Self::enter(ProviderScope::from_staged_root(root))
    }
```

</details>

### <span id="struct.ConstructorNode"></span><code style="font-size: 0.85em">struct ConstructorNode</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A workflow DAG node backed by a packaged WASM task constructor — the runtime representation of a `constructor!(...)` declaration (CLOACI-T-0829).

Wraps the `Arc<dyn Task>` [`load_task_constructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_task_constructor) returns, overriding its
`id` (the DAG node id the workflow author chose, which other tasks depend on —
distinct from the constructor's own `constructor.json` name) and its
`dependencies` (the workflow-namespaced deps the `constructor!` declared).
Everything else delegates to the loaded constructor.

## Enums

### <span id="enum.ConstructorBinding"></span><code style="font-size: 0.85em">enum ConstructorBinding</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Per-primitive binding the caller supplies to [`load_constructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor). The variant must match the loaded constructor's `primitive_kind` or the load fails closed.

#### Variants

- **`Task`** - Register the loaded constructor as a [`Task`](../../../../../rust/cloacina-workflow/task/#trait.Task) under this namespace.
- **`Trigger`** - Register the loaded constructor as a [`Trigger`](../../../../../rust/cloacina-workflow/trigger/#trait.Trigger) with this scheduling
  metadata. The trigger is keyed in the registry by the manifest `name`.

### <span id="enum.ProviderScope"></span><code style="font-size: 0.85em">enum ProviderScope</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

The provider tree ONE package load resolves `from = ".."` against (CLOACI-T-0925).

#### Variants

- **`Staged`** - The package's OWN bundled providers, unpacked into this directory by the
  reconciler. Highest precedence — it is this package's hermetic world.
- **`Unbundled`** - The package bundles NO providers. Its `from = ".."` refs must not fall
  back to whatever another tenant's load staged, so the process-wide
  override is SKIPPED and resolution falls through to
  [`PROVIDER_PATH_ENV`](../../../../../rust/cloacina/registry/loader/constructor_loader/#const.PROVIDER_PATH_ENV) / [`DEFAULT_PROVIDER_DIR`](../../../../../rust/cloacina/registry/loader/constructor_loader/#const.DEFAULT_PROVIDER_DIR) — the same directory the
  pre-T-0925 `clear_provider_search_path()` call left behind, but scoped to
  this load instead of stomping the process.

#### Methods

##### <span id="ProviderScope.from_staged_root"></span><code>from_​staged_​root</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_staged_root(root: Option<std::path::PathBuf>) -> Self
```

`Some(root)` → [`ProviderScope::Staged`](../../../../../rust/cloacina/registry/loader/constructor_loader/#enum.ProviderScope); `None` → [`ProviderScope::Unbundled`](../../../../../rust/cloacina/registry/loader/constructor_loader/#enum.ProviderScope).

<details>
<summary>Source</summary>

```rust
    pub fn from_staged_root(root: Option<std::path::PathBuf>) -> Self {
        match root {
            Some(root) => ProviderScope::Staged(root),
            None => ProviderScope::Unbundled,
        }
    }
```

</details>

##### <span id="ProviderScope.search_path"></span><code>search_​path</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn search_path(&self) -> std::path::PathBuf
```

The directory this scope resolves in.

<details>
<summary>Source</summary>

```rust
    pub fn search_path(&self) -> std::path::PathBuf {
        match self {
            ProviderScope::Staged(root) => root.clone(),
            ProviderScope::Unbundled => provider_search_path_from_env(),
        }
    }
```

</details>

## Traits

### <span id="trait.TaskConstructor"></span><code style="font-size: 0.85em">trait TaskConstructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Host-side re-declaration of the TASK-constructor interface. This is the SAME trait shape the guest implements; declaring it with `crate = "fidius_core"` makes the fidius macro emit the matching `TaskConstructor_WASM_DESCRIPTOR` (in the companion `__fidius_TaskConstructor` module) that the loader links the component against. The `fidius-interface-hash` export then gates integrity at load (CLOACI-T-0821).

#### Methods

##### <span id="TaskConstructor.execute"></span><code>execute</code>

```rust
fn execute(&self, invocation_json: String) -> String
```

`JSON(TaskInvocation)` in -> `JSON(TaskOutcome)` out. SYNC.

<details>
<summary>Source</summary>

```rust
    fn execute(&self, invocation_json: String) -> String;
```

</details>

### <span id="trait.TriggerConstructor"></span><code style="font-size: 0.85em">trait TriggerConstructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Host-side re-declaration of the TRIGGER-constructor interface (CLOACI-T-0824). Same trait shape the guest implements; the fidius macro emits the matching `TriggerConstructor_WASM_DESCRIPTOR` (in `__fidius_TriggerConstructor`) the loader links the component against. Single SYNC method — the WASM analogue of the async `cloacina_workflow::Trigger::poll`.

#### Methods

##### <span id="TriggerConstructor.poll"></span><code>poll</code>

```rust
fn poll(&self, invocation_json: String) -> String
```

`JSON(TriggerInvocation)` in -> `JSON(PollOutcome)` out. SYNC.

<details>
<summary>Source</summary>

```rust
    fn poll(&self, invocation_json: String) -> String;
```

</details>

### <span id="trait.AccumulatorConstructor"></span><code style="font-size: 0.85em">trait AccumulatorConstructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Host-side re-declaration of the ACCUMULATOR-constructor interface (CLOACI-T-0828). Same trait shape the guest implements; the fidius macro emits the matching `AccumulatorConstructor_WASM_DESCRIPTOR` the loader links against. Single SYNC method — the WASM analogue of `Accumulator::process`.

#### Methods

##### <span id="AccumulatorConstructor.ingest"></span><code>ingest</code>

```rust
fn ingest(&self, invocation_json: String) -> String
```

`JSON(AccumulatorInvocation)` in -> `JSON(AccumulatorOutcome)` out. SYNC.

<details>
<summary>Source</summary>

```rust
    fn ingest(&self, invocation_json: String) -> String;
```

</details>

### <span id="trait.ReactorConstructor"></span><code style="font-size: 0.85em">trait ReactorConstructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Host-side re-declaration of the REACTOR-constructor interface (CLOACI-T-0828). Same trait shape the guest implements; the fidius macro emits the matching `ReactorConstructor_WASM_DESCRIPTOR` the loader links against. Single SYNC method — the WASM analogue of the reactor's firing-criteria evaluation.

#### Methods

##### <span id="ReactorConstructor.evaluate"></span><code>evaluate</code>

```rust
fn evaluate(&self, invocation_json: String) -> String
```

`JSON(ReactorInvocation)` in -> `JSON(ReactorOutcome)` out. SYNC.

<details>
<summary>Source</summary>

```rust
    fn evaluate(&self, invocation_json: String) -> String;
```

</details>

## Functions

### <span id="fn.read_provider_manifest"></span><code style="font-size: 0.85em">fn read_​provider_​manifest</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn read_provider_manifest(
    package_dir: &Path,
) -> Result<ProviderManifest, LoaderError>
```

Read and parse a provider package's `provider.json` into a `ProviderManifest`.

<details>
<summary>Source</summary>

```rust
pub fn read_provider_manifest(package_dir: &Path) -> Result<ProviderManifest, LoaderError> {
    let path = package_dir.join(PROVIDER_MANIFEST_FILE);
    let raw = std::fs::read_to_string(&path).map_err(|e| LoaderError::FileSystem {
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    ProviderManifest::from_json(&raw).map_err(|e| LoaderError::ManifestParse {
        reason: format!("{PROVIDER_MANIFEST_FILE}: {e}"),
    })
}
```

</details>

### <span id="fn.read_constructor_manifest"></span><code style="font-size: 0.85em">fn read_​constructor_​manifest</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn read_constructor_manifest(
    package_dir: &Path,
) -> Result<ConstructorManifest, LoaderError>
```

Read and parse a package's sidecar `constructor.json` manifest.

<details>
<summary>Source</summary>

```rust
pub fn read_constructor_manifest(package_dir: &Path) -> Result<ConstructorManifest, LoaderError> {
    let path = package_dir.join(CONSTRUCTOR_MANIFEST_FILE);
    let raw = std::fs::read_to_string(&path).map_err(|e| LoaderError::FileSystem {
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    ConstructorManifest::from_json(&raw).map_err(|e| LoaderError::ManifestParse {
        reason: format!("{CONSTRUCTOR_MANIFEST_FILE}: {e}"),
    })
}
```

</details>

### <span id="fn.load_task_constructor"></span><code style="font-size: 0.85em">fn load_​task_​constructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_task_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    grants: &ResolvedGrants,
) -> Result<Arc<dyn Task>, LoaderError>
```

Load a WASM **task** constructor and return it as a runnable [`Task`](../../../../../rust/cloacina-workflow/task/#trait.Task).

`search_path` is a directory containing constructor package subdirectories;
`package_name` is the `[package].name` in the package's `package.toml` (what
fidius matches on). `config` binds the constructor's per-instance configuration
once at load (the `configure` hook) — two loads with different configs yield
two independently-configured constructors.

Fails closed (a clear `LoaderError`) if the package is missing, the
manifest is unreadable, the primitive is not a `Task`, or the interface
version doesn't match the contract. (fidius's `fidius-interface-hash` export
additionally gates ABI integrity inside `load_wasm_configured`.)

<details>
<summary>Source</summary>

```rust
pub fn load_task_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    grants: &ResolvedGrants,
) -> Result<Arc<dyn Task>, LoaderError> {
    let search_path = search_path.as_ref();

    // NATIVE provider fast-path (CLOACI-T-0902): a `runtime = "native"` provider
    // loads in-process via `configure_from_loaded`; grants are advisory (no
    // sandbox). Falls through to the WASM path when the package isn't native.
    if let Some((handle, member)) = load_native_member(
        search_path,
        package_name,
        constructor_name,
        config,
        PrimitiveKind::Task,
        TASK_CONSTRUCTOR_INTERFACE_VERSION,
        "__ProviderTask",
    )? {
        return Ok(Arc::new(WasmTaskConstructor {
            id: member.name,
            handle: Arc::new(handle),
            dependencies: Vec::new(),
        }));
    }

    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;

    // Resolve the package directory so we can read its provider manifest + select
    // the requested member constructor.
    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;

    let manifest = read_member_manifest(&dir, constructor_name)?;

    if manifest.primitive_kind != PrimitiveKind::Task {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not Task; trigger/accumulator/reactor loading is CLOACI-T-0824",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != TASK_CONSTRUCTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares task-constructor interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, TASK_CONSTRUCTOR_INTERFACE_VERSION
            ),
        });
    }

    // Config binds ONCE here; the integrity hash is checked inside fidius. The
    // member name travels in the configure payload (name-in-configure) so the suite
    // shell instantiates the right member. The tenant's translated grants override
    // the manifest caps + supply the egress policy (default-closed when deny-all).
    let configure = wrap_member_config(constructor_name, config)?;
    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_TaskConstructor::TaskConstructor_WASM_DESCRIPTOR,
            &configure,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants: {e}"),
        })?;

    Ok(Arc::new(WasmTaskConstructor {
        id: manifest.name,
        handle: Arc::new(handle),
        dependencies: Vec::new(),
    }))
}
```

</details>

### <span id="fn.unpack_provider_archive"></span><code style="font-size: 0.85em">fn unpack_​provider_​archive</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unpack_provider_archive(
    archive: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    verify_keys: &[ed25519_dalek::VerifyingKey],
) -> Result<std::path::PathBuf, LoaderError>
```

Unpack a packed **provider package** archive (a `.cloacina`/`.fid` produced by [`crate::packaging::constructor_provider::package_constructor_provider`](../../../../../rust/cloacina/packaging/constructor_provider/#fn.package_constructor_provider)) into `dest` and, when `verify_keys` is non-empty, verify its Ed25519 `package.sig` against those trusted keys. Returns the unpacked package directory (which contains `package.toml`, the `constructor.json` sidecar, and the `.wasm` component).

Fails closed at every stage:
  * a structurally-hostile archive (path traversal, absolute path, symlink,
    hardlink, size/ratio bomb, or no `package.toml`) is rejected by
    `fidius_core::package::unpack_package`;
  * with `verify_keys` supplied, a missing or non-verifying signature — which
    is exactly what tampering with the component or manifest after signing
    produces (the digest no longer matches) — is a hard error via
    `fidius_host::package::verify_package`.

An empty `verify_keys` skips signature checking (the unsigned/dev flow); the
structural archive checks still apply.

<details>
<summary>Source</summary>

```rust
pub fn unpack_provider_archive(
    archive: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    verify_keys: &[ed25519_dalek::VerifyingKey],
) -> Result<std::path::PathBuf, LoaderError> {
    let archive = archive.as_ref();
    let dest = dest.as_ref();

    let pkg_dir = fidius_core::package::unpack_package(archive, dest).map_err(|e| {
        LoaderError::Validation {
            reason: format!("unpack provider package '{}': {e}", archive.display()),
        }
    })?;

    if !verify_keys.is_empty() {
        fidius_host::package::verify_package(&pkg_dir, verify_keys).map_err(|e| {
            LoaderError::Validation {
                reason: format!("verify provider signature for '{}': {e}", pkg_dir.display()),
            }
        })?;
    }

    Ok(pkg_dir)
}
```

</details>

### <span id="fn.load_task_constructor_from_package"></span><code style="font-size: 0.85em">fn load_​task_​constructor_​from_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_task_constructor_from_package<C: Serialize>(
    archive: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    verify_keys: &[ed25519_dalek::VerifyingKey],
    grants: &ResolvedGrants,
) -> Result<Arc<dyn Task>, LoaderError>
```

Load a WASM **task** constructor from a packed provider archive and return it as a runnable [`Task`](../../../../../rust/cloacina-workflow/task/#trait.Task).

The packaged analogue of [`load_task_constructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_task_constructor): it unpacks (and, with
`verify_keys`, verifies the signature of) the provider archive into `dest`,
then resolves `package_name` out of the unpacked tree exactly as the loose-dir
loader does (the unpacked package dir lives under `dest`, and fidius matches
the package by its `[package].name`, not its directory name). `config` binds
the constructor's per-instance configuration once at load.

Fails closed if the archive is hostile/unsigned-when-required, the package is
missing, the manifest is unreadable or not a `Task`, or the interface version
doesn't match the contract.

<details>
<summary>Source</summary>

```rust
pub fn load_task_constructor_from_package<C: Serialize>(
    archive: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    verify_keys: &[ed25519_dalek::VerifyingKey],
    grants: &ResolvedGrants,
) -> Result<Arc<dyn Task>, LoaderError> {
    let dest = dest.as_ref();
    // Unpack (+ verify) first; this is the seam that rejects a tampered or
    // unsigned-but-required archive before any wasm is loaded.
    let _pkg_dir = unpack_provider_archive(archive, dest, verify_keys)?;
    // `dest` is the fidius search path; the package resolves by name out of the
    // freshly-unpacked `<name>-<version>/` directory it contains.
    load_task_constructor(dest, package_name, constructor_name, config, grants)
}
```

</details>

### <span id="fn.load_trigger_constructor"></span><code style="font-size: 0.85em">fn load_​trigger_​constructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_trigger_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    binding: TriggerBinding,
    grants: &ResolvedGrants,
) -> Result<Arc<dyn Trigger>, LoaderError>
```

Load a WASM **trigger** constructor and return it as a runnable [`Trigger`](../../../../../rust/cloacina-workflow/trigger/#trait.Trigger).

`config` binds the constructor's per-instance WASM configuration once at load
(the guest's `configure` hook); `binding` supplies the host-side scheduling
metadata the `Trigger` trait exposes (poll interval / cron / target
workflow). Fails closed if the package is missing, the manifest is
unreadable, the primitive is not a `Trigger`, or the interface version
doesn't match the trigger contract.

<details>
<summary>Source</summary>

```rust
pub fn load_trigger_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    binding: TriggerBinding,
    grants: &ResolvedGrants,
) -> Result<Arc<dyn Trigger>, LoaderError> {
    let search_path = search_path.as_ref();

    // NATIVE provider fast-path (CLOACI-T-0902): grants advisory; falls through
    // to the WASM path when the package isn't a `runtime = "native"` provider.
    if let Some((handle, member)) = load_native_member(
        search_path,
        package_name,
        constructor_name,
        config,
        PrimitiveKind::Trigger,
        TRIGGER_CONSTRUCTOR_INTERFACE_VERSION,
        "__ProviderTrigger",
    )? {
        return Ok(Arc::new(WasmTriggerConstructor {
            name: member.name,
            handle: Arc::new(handle),
            binding,
        }));
    }

    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;

    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;

    let manifest = read_member_manifest(&dir, constructor_name)?;

    if manifest.primitive_kind != PrimitiveKind::Trigger {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not Trigger",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != TRIGGER_CONSTRUCTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares trigger-constructor interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, TRIGGER_CONSTRUCTOR_INTERFACE_VERSION
            ),
        });
    }

    let configure = wrap_member_config(constructor_name, config)?;
    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_TriggerConstructor::TriggerConstructor_WASM_DESCRIPTOR,
            &configure,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants: {e}"),
        })?;

    Ok(Arc::new(WasmTriggerConstructor {
        name: manifest.name,
        handle: Arc::new(handle),
        binding,
    }))
}
```

</details>

### <span id="fn.load_constructor"></span><code style="font-size: 0.85em">fn load_​constructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_constructor<C: Serialize>(
    runtime: &Runtime,
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    binding: ConstructorBinding,
    grants: &ResolvedGrants,
) -> Result<(), LoaderError>
```

Load a WASM constructor and register the configured primitive into `runtime`, dispatching on the manifest's `primitive_kind`.

This is the seam that makes a loaded constructor a first-class participant in
workflows/schedules: after this returns, `runtime.get_task(ns)` /
`runtime.get_trigger(name)` hand out the configured constructor exactly as they
would a macro-authored one. The registered constructor clones a shared `Arc`
over the single configured fidius handle, so every instantiation dispatches
into the same sandboxed store.

Fails closed if the [`ConstructorBinding`](../../../../../rust/cloacina/registry/loader/constructor_loader/#enum.ConstructorBinding) variant doesn't match the constructor's
declared `primitive_kind`, or on any underlying load error.

<details>
<summary>Source</summary>

```rust
pub fn load_constructor<C: Serialize>(
    runtime: &Runtime,
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    binding: ConstructorBinding,
    grants: &ResolvedGrants,
) -> Result<(), LoaderError> {
    let search_path = search_path.as_ref();

    // Peek the manifest so a mismatched binding fails before we pay for a load.
    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;
    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;
    let manifest = read_member_manifest(&dir, constructor_name)?;

    match (&binding, manifest.primitive_kind) {
        (ConstructorBinding::Task { namespace }, PrimitiveKind::Task) => {
            let task =
                load_task_constructor(search_path, package_name, constructor_name, config, grants)?;
            let namespace = namespace.clone();
            runtime.register_task(namespace, move || task.clone());
            Ok(())
        }
        (ConstructorBinding::Trigger(tb), PrimitiveKind::Trigger) => {
            let name = manifest.name.clone();
            let trigger = load_trigger_constructor(
                search_path,
                package_name,
                constructor_name,
                config,
                tb.clone(),
                grants,
            )?;
            runtime.register_trigger(name, move || trigger.clone());
            Ok(())
        }
        (_, kind) => Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, which does not match the supplied binding \
                 (accumulator/reactor registration is a CLOACI-T-0824 continuation)",
                manifest.name, kind
            ),
        }),
    }
}
```

</details>

### <span id="fn.load_accumulator_constructor"></span><code style="font-size: 0.85em">fn load_​accumulator_​constructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_accumulator_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    grants: &ResolvedGrants,
) -> Result<WasmAccumulatorConstructor, LoaderError>
```

Load a WASM **accumulator** constructor and return it as a runnable `Accumulator`.

The returned value is driven by
[`accumulator_runtime`](../../../../../rust/cloacina/computation_graph/accumulator/#fn.accumulator_runtime):
hand it (plus an `AccumulatorContext` carrying the `BoundarySender` to the
reactor) to that runtime and each socket/source event flows through the
configured WASM `ingest`. `config` binds the constructor's per-instance WASM
configuration once at load (the guest's `configure` hook).

Fails closed if the package is missing, the manifest is unreadable, the
primitive is not an `Accumulator`, or the interface version doesn't match the
accumulator contract.

<details>
<summary>Source</summary>

```rust
pub fn load_accumulator_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    grants: &ResolvedGrants,
) -> Result<WasmAccumulatorConstructor, LoaderError> {
    let search_path = search_path.as_ref();

    // NATIVE provider fast-path (CLOACI-T-0902; also T-0904's stream-accumulator
    // prereq): grants advisory; falls through to WASM when not a native provider.
    if let Some((handle, member)) = load_native_member(
        search_path,
        package_name,
        constructor_name,
        config,
        PrimitiveKind::Accumulator,
        ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION,
        "__ProviderAccumulator",
    )? {
        return Ok(WasmAccumulatorConstructor {
            name: member.name,
            handle: Arc::new(handle),
        });
    }

    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;

    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;

    let manifest = read_member_manifest(&dir, constructor_name)?;

    if manifest.primitive_kind != PrimitiveKind::Accumulator {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not Accumulator",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares accumulator-constructor interface v{}, loader supports v{}",
                manifest.name,
                manifest.interface_version,
                ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION
            ),
        });
    }

    let configure = wrap_member_config(constructor_name, config)?;
    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_AccumulatorConstructor::AccumulatorConstructor_WASM_DESCRIPTOR,
            &configure,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants: {e}"),
        })?;

    Ok(WasmAccumulatorConstructor {
        name: manifest.name,
        handle: Arc::new(handle),
    })
}
```

</details>

### <span id="fn.load_stream_accumulator_source"></span><code style="font-size: 0.85em">fn load_​stream_​accumulator_​source</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn load_stream_accumulator_source<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
) -> Result<ProviderStreamSource, LoaderError>
```

Load a NATIVE **stream accumulator** member and start its server-streaming `source`, returning an `EventSource` that drives boundary items onto an accumulator's merge channel (feed it to `accumulator_runtime_with_source` with a passthrough `Accumulator`).

Native-only (a `runtime = "native"` provider whose member carries
`interface = "stream-accumulator-constructor"`); wasm streaming parity is
T-0906/0907. Fails closed if the member isn't a native stream accumulator.
`config` binds the member's `#[config]` once at load, exactly like the other
constructor loaders.

<details>
<summary>Source</summary>

```rust
pub async fn load_stream_accumulator_source<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
) -> Result<ProviderStreamSource, LoaderError> {
    let search_path = search_path.as_ref();

    let (handle, member) = load_native_member(
        search_path,
        package_name,
        constructor_name,
        config,
        PrimitiveKind::Accumulator,
        STREAM_ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION,
        "__ProviderStreamAccumulator",
    )?
    .ok_or_else(|| LoaderError::Validation {
        reason: format!(
            "provider '{package_name}' member '{constructor_name}' is not a native stream \
             accumulator (needs runtime = \"native\" + a stream-accumulator member; wasm \
             streaming parity is CLOACI-T-0906/0907)"
        ),
    })?;

    let handle = Arc::new(handle);
    // Start the server-streaming `source` (METHOD_SOURCE). The `seed` arg is unused
    // by the shell — a single-arg fidius method takes a `(T,)` tuple. Items decode
    // as boundary-JSON `String`s.
    let stream = handle
        .call_streaming::<(u64,), String>(METHOD_SOURCE, &(0u64,))
        .await
        .map_err(|e| LoaderError::LibraryLoad {
            path: package_name.to_string(),
            error: format!("call_streaming(source) on native stream accumulator: {e}"),
        })?;

    Ok(ProviderStreamSource {
        name: member.name,
        _handle: handle,
        stream,
    })
}
```

</details>

### <span id="fn.load_stream_accumulator_source_from_config"></span><code style="font-size: 0.85em">fn load_​stream_​accumulator_​source_​from_​config</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn load_stream_accumulator_source_from_config(
    provider_name: &str,
    constructor_name: &str,
    config: &std::collections::HashMap<String, String>,
) -> Result<ProviderStreamSource, LoaderError>
```

Load a NATIVE stream-accumulator member declared by a PACKAGED workflow's `[[metadata.accumulators]]` config map (CLOACI-T-0907) and start its source.

The packaged-workflow declaration surface is a name-keyed STRING map
(`[metadata.accumulators.config] broker = ".." topic = ".."`), but the member's
`#[config]` crosses as bincode of its typed struct. This bridges the two the
same way `constructor!(config = { … })` nodes do: read the member's declared
`config_fields` from the provider manifest, bind the map BY NAME
(`bind_config_by_name` — fails closed on unknown/missing keys), and encode in
declaration order. String values coerce per the declared field type (`"5"` for
an `i64` field parses; kafka's fields are all `String`).

The provider resolves from the ambient [`provider_search_path`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.provider_search_path) — the same
`providers/` tree the reconciler unpacks a package's bundled providers into.
A host that knows which package it is loading should call
[`load_stream_accumulator_source_from_config_in`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_stream_accumulator_source_from_config_in) instead: this resolution runs
from a spawned accumulator task, so an ambient read can land long after the
load that staged the providers finished (CLOACI-T-0925).

<details>
<summary>Source</summary>

```rust
pub async fn load_stream_accumulator_source_from_config(
    provider_name: &str,
    constructor_name: &str,
    config: &std::collections::HashMap<String, String>,
) -> Result<ProviderStreamSource, LoaderError> {
    load_stream_accumulator_source_from_config_in(
        &provider_search_path(),
        provider_name,
        constructor_name,
        config,
    )
    .await
}
```

</details>

### <span id="fn.load_stream_accumulator_source_from_config_in"></span><code style="font-size: 0.85em">fn load_​stream_​accumulator_​source_​from_​config_​in</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn load_stream_accumulator_source_from_config_in(
    search_path: &Path,
    provider_name: &str,
    constructor_name: &str,
    config: &std::collections::HashMap<String, String>,
) -> Result<ProviderStreamSource, LoaderError>
```

[`load_stream_accumulator_source_from_config`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_stream_accumulator_source_from_config) against an EXPLICIT provider directory (CLOACI-T-0925) — the root captured when the accumulator's declaration was built, not whatever the process global holds when the spawned stream task happens to run.

<details>
<summary>Source</summary>

```rust
pub async fn load_stream_accumulator_source_from_config_in(
    search_path: &Path,
    provider_name: &str,
    constructor_name: &str,
    config: &std::collections::HashMap<String, String>,
) -> Result<ProviderStreamSource, LoaderError> {
    let search_path = search_path.to_path_buf();

    // Member manifest first — bind_config_by_name needs its config_fields.
    let (_dir, provider) =
        resolve_native_provider(&search_path, provider_name)?.ok_or_else(|| {
            LoaderError::Validation {
                reason: format!(
                    "stream accumulator declares provider '{provider_name}' but no NATIVE provider \
                 by that name exists under '{}' (bundle it with the package, or check the \
                 provider/runtime — wasm stream providers are CLOACI-T-0906/0907 follow-up)",
                    search_path.display()
                ),
            }
        })?;
    let member = provider
        .constructor(constructor_name)
        .ok_or_else(|| LoaderError::Validation {
            reason: format!(
                "provider '{provider_name}' has no constructor '{constructor_name}'; members: [{}]",
                provider
                    .constructors
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })?
        .clone();

    // Map → JSON values (strings that parse as the declared numeric/bool type
    // coerce — package.toml accumulator config is stringly-typed).
    let author: Vec<(String, serde_json::Value)> = config
        .iter()
        .map(|(k, v)| {
            let field_ty = member
                .config_fields
                .iter()
                .find(|f| f.name == *k)
                .map(|f| f.ty.as_str());
            let value = match field_ty {
                Some("String") | Some("str") | None => serde_json::Value::String(v.clone()),
                Some("bool") => v
                    .parse::<bool>()
                    .map(serde_json::Value::Bool)
                    .unwrap_or_else(|_| serde_json::Value::String(v.clone())),
                // Numeric field: parse the string; a non-parsing value falls
                // through as a string so bind_config_by_name names the field in
                // its type error.
                Some(_) => serde_json::from_str::<serde_json::Value>(v)
                    .ok()
                    .filter(serde_json::Value::is_number)
                    .unwrap_or_else(|| serde_json::Value::String(v.clone())),
            };
            (k.clone(), value)
        })
        .collect();

    let ordered = bind_config_by_name(
        // Context strings for the fail-closed error messages.
        &format!("accumulator '{constructor_name}'"),
        provider_name,
        constructor_name,
        &member,
        author,
    )?;

    load_stream_accumulator_source(&search_path, provider_name, constructor_name, &ordered).await
}
```

</details>

### <span id="fn.load_reactor_constructor"></span><code style="font-size: 0.85em">fn load_​reactor_​constructor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_reactor_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    grants: &ResolvedGrants,
) -> Result<WasmReactorConstructor, LoaderError>
```

Load a WASM **reactor** constructor and return its configured firing-decision bridge.

Hand the returned [`WasmReactorConstructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#struct.WasmReactorConstructor) to
[`Reactor::with_evaluator`](../../../../../rust/cloacina/computation_graph/reactor/#Reactor.with_evaluator)
(as an `Arc<dyn ReactorFireDecider>`) so the running reactor consults the WASM
guest's `evaluate` for its firing decision. `config` binds the constructor's
per-instance WASM configuration once at load.

Fails closed if the package is missing, the manifest is unreadable, the
primitive is not a `Reactor`, or the interface version doesn't match the
reactor contract.

<details>
<summary>Source</summary>

```rust
pub fn load_reactor_constructor<C: Serialize>(
    search_path: impl AsRef<Path>,
    package_name: &str,
    constructor_name: &str,
    config: &C,
    grants: &ResolvedGrants,
) -> Result<WasmReactorConstructor, LoaderError> {
    let search_path = search_path.as_ref();

    // NATIVE provider fast-path (CLOACI-T-0902): grants advisory; falls through
    // to the WASM path when the package isn't a `runtime = "native"` provider.
    if let Some((handle, member)) = load_native_member(
        search_path,
        package_name,
        constructor_name,
        config,
        PrimitiveKind::Reactor,
        REACTOR_CONSTRUCTOR_INTERFACE_VERSION,
        "__ProviderReactor",
    )? {
        return Ok(WasmReactorConstructor {
            name: member.name,
            handle: Arc::new(handle),
        });
    }

    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;

    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate wasm constructor package '{package_name}': {e}"),
        })?;

    let manifest = read_member_manifest(&dir, constructor_name)?;

    if manifest.primitive_kind != PrimitiveKind::Reactor {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not Reactor",
                manifest.name, manifest.primitive_kind
            ),
        });
    }

    if manifest.interface_version != REACTOR_CONSTRUCTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares reactor-constructor interface v{}, loader supports v{}",
                manifest.name, manifest.interface_version, REACTOR_CONSTRUCTOR_INTERFACE_VERSION
            ),
        });
    }

    let configure = wrap_member_config(constructor_name, config)?;
    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_ReactorConstructor::ReactorConstructor_WASM_DESCRIPTOR,
            &configure,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants: {e}"),
        })?;

    Ok(WasmReactorConstructor {
        name: manifest.name,
        handle: Arc::new(handle),
    })
}
```

</details>

### <span id="fn.set_provider_search_path"></span><code style="font-size: 0.85em">fn set_​provider_​search_​path</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_provider_search_path(path: impl Into<std::path::PathBuf>)
```

Set the process-wide provider search path used to resolve `constructor!` `from = "..."` references. Takes precedence over [`PROVIDER_PATH_ENV`](../../../../../rust/cloacina/registry/loader/constructor_loader/#const.PROVIDER_PATH_ENV).

Embedded / single-tenant use only. A multi-tenant host (the server's
reconciler) must NOT set this — it scopes each package load instead
([`ScopedProviderSearch`](../../../../../rust/cloacina/registry/loader/constructor_loader/#struct.ScopedProviderSearch) / the `*_in` loader entry points), because one
process-wide directory makes every tenant's staged providers visible to
every other tenant's resolution (CLOACI-T-0925).

<details>
<summary>Source</summary>

```rust
pub fn set_provider_search_path(path: impl Into<std::path::PathBuf>) {
    *PROVIDER_SEARCH_PATH.write().unwrap() = Some(path.into());
}
```

</details>

### <span id="fn.clear_provider_search_path"></span><code style="font-size: 0.85em">fn clear_​provider_​search_​path</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn clear_provider_search_path()
```

Clear the process-wide provider search-path override (falls back to env/default).

<details>
<summary>Source</summary>

```rust
pub fn clear_provider_search_path() {
    *PROVIDER_SEARCH_PATH.write().unwrap() = None;
}
```

</details>

### <span id="fn.current_provider_scope"></span><code style="font-size: 0.85em">fn current_​provider_​scope</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn current_provider_scope() -> Option<ProviderScope>
```

The provider scope installed on this thread (`None` when unscoped).

Capture this before spawning a thread that will resolve constructors on the
caller's behalf (the Python loader does exactly that) and re-install it there
with [`ScopedProviderSearch::enter_opt`](../../../../../rust/cloacina/registry/loader/constructor_loader/#ScopedProviderSearch.enter_opt) — a thread-local does not follow a
`std::thread::spawn` / `spawn_blocking` hop on its own.

<details>
<summary>Source</summary>

```rust
pub fn current_provider_scope() -> Option<ProviderScope> {
    CURRENT_PROVIDER_SCOPE.with(|slot| slot.borrow().clone())
}
```

</details>

### <span id="fn.provider_search_path"></span><code style="font-size: 0.85em">fn provider_​search_​path</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn provider_search_path() -> std::path::PathBuf
```

The directory `constructor!(from = ...)` resolves provider packages in.

Precedence (highest first, CLOACI-T-0925):

  1. the per-load [`ProviderScope`](../../../../../rust/cloacina/registry/loader/constructor_loader/#enum.ProviderScope) installed on this thread
     ([`ScopedProviderSearch`](../../../../../rust/cloacina/registry/loader/constructor_loader/#struct.ScopedProviderSearch)) — the tenant/package-scoped world;
  2. the process-wide [`set_provider_search_path`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.set_provider_search_path) override — the
     embedded/single-tenant convenience knob;
  3. [`PROVIDER_PATH_ENV`](../../../../../rust/cloacina/registry/loader/constructor_loader/#const.PROVIDER_PATH_ENV);
  4. [`DEFAULT_PROVIDER_DIR`](../../../../../rust/cloacina/registry/loader/constructor_loader/#const.DEFAULT_PROVIDER_DIR).

With no scope installed this is byte-for-byte the pre-T-0925 behavior.

<details>
<summary>Source</summary>

```rust
pub fn provider_search_path() -> std::path::PathBuf {
    if let Some(scope) = current_provider_scope() {
        return scope.search_path();
    }
    if let Some(p) = PROVIDER_SEARCH_PATH.read().unwrap().clone() {
        return p;
    }
    provider_search_path_from_env()
}
```

</details>

### <span id="fn.parse_runtime_pin"></span><code style="font-size: 0.85em">fn parse_​runtime_​pin</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn parse_runtime_pin(
    context: &str,
    literal: &str,
) -> Result<ProviderRuntime, LoaderError>
```

Parse a consumer-surface `runtime = "wasm"|"native"` literal (CLOACI-T-0920).

The macros validate this at compile time; this is the fail-closed re-check for
the string-carried paths (`ReactorConstructorRef::runtime`, which crosses a
stringly-typed struct rather than gaining a contract-crate dependency).

<details>
<summary>Source</summary>

```rust
pub fn parse_runtime_pin(context: &str, literal: &str) -> Result<ProviderRuntime, LoaderError> {
    match literal {
        "wasm" => Ok(ProviderRuntime::Wasm),
        "native" => Ok(ProviderRuntime::Native),
        other => Err(LoaderError::Validation {
            reason: format!(
                "{context}: unknown runtime pin '{other}'; valid values are \"wasm\" and \"native\""
            ),
        }),
    }
}
```

</details>

### <span id="fn.load_constructor_node"></span><code style="font-size: 0.85em">fn load_​constructor_​node</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_constructor_node(
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
) -> Result<Arc<dyn Task>, LoaderError>
```

Resolve + load a packaged constructor as a workflow DAG node (the runtime half of the `constructor!(...)` consumer form).

Resolves `from` against the [`provider_search_path`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.provider_search_path), reads the resolved
constructor's manifest to learn its `#[config]` schema, binds the author's
`config = { name = value }` kwargs BY NAME (`bind_config_by_name`), loads the
named `constructor` via the packaged task-constructor loader (binding the
reordered config once), validates that the provider actually carries the
requested constructor, and wraps it as a [`ConstructorNode`](../../../../../rust/cloacina/registry/loader/constructor_loader/#struct.ConstructorNode) carrying the
author-chosen `node_id` + the workflow-namespaced `dependencies`.

`config` is the author's `config = { … }` entries as `(name, value)` pairs in
WRITTEN order. fidius binds config via bincode (positional, NOT self-describing),
so before loading we **reorder** the values into the constructor's `#[config]`
DECLARATION order — read from the manifest's `config_fields` — and coerce each to
its declared type. This gives `config = { name = value }` true kwarg semantics:
the field NAMES bind the values, not their written order.

Fails closed (a clear `LoaderError`) if the provider is missing, the manifest
is unreadable / not a `Task`, the interface version mismatches, a config kwarg is
unknown / duplicated / missing / mistyped, or the resolved constructor name does
not match `constructor_name`.

<details>
<summary>Source</summary>

```rust
pub fn load_constructor_node(
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
) -> Result<Arc<dyn Task>, LoaderError> {
    load_constructor_node_pinned(
        node_id,
        from,
        constructor_name,
        config,
        dependencies,
        grants,
        None,
    )
}
```

</details>

### <span id="fn.load_constructor_node_pinned"></span><code style="font-size: 0.85em">fn load_​constructor_​node_​pinned</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_constructor_node_pinned(
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
    runtime_pin: Option<ProviderRuntime>,
) -> Result<Arc<dyn Task>, LoaderError>
```

[`load_constructor_node`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node) with the author's optional trust-tier pin (`constructor!(.., runtime = "wasm"|"native")` — CLOACI-T-0920).

This is what the `#[workflow]` macro emits. The pin rides the Rust signature
rather than the packaged FFI declaration struct
(`cloacina_workflow_plugin::ConstructorPackageMetadata`, which crosses a
**bincode** wire via `get_constructor_metadata` — appending a field there would
mis-decode every already-built cdylib). Packaged workflows therefore resolve
through the unpinned [`load_constructor_node`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node) and are still protected, because
the grants-present-but-unenforced rule keys off `grants`, which already cross the
wire.

<details>
<summary>Source</summary>

```rust
pub fn load_constructor_node_pinned(
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
    runtime_pin: Option<ProviderRuntime>,
) -> Result<Arc<dyn Task>, LoaderError> {
    load_constructor_node_pinned_in(
        &provider_search_path(),
        node_id,
        from,
        constructor_name,
        config,
        dependencies,
        grants,
        runtime_pin,
    )
}
```

</details>

### <span id="fn.load_constructor_node_in"></span><code style="font-size: 0.85em">fn load_​constructor_​node_​in</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_constructor_node_in(
    search_path: &Path,
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
) -> Result<Arc<dyn Task>, LoaderError>
```

[`load_constructor_node`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node) against an EXPLICIT provider directory (CLOACI-T-0925) — no shared state is read, so a multi-tenant host resolves each package against the tree it staged for that package and nothing else.

This is the primitive; the ambient-path forms above are thin wrappers that
supply [`provider_search_path`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.provider_search_path). Host-side callers that know which package
they are loading (the reconciler, the CG scheduler, provider-backed stream
accumulators) MUST use this form: they resolve from spawned tasks that can
outlive any process-global the load set.

<details>
<summary>Source</summary>

```rust
pub fn load_constructor_node_in(
    search_path: &Path,
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
) -> Result<Arc<dyn Task>, LoaderError> {
    load_constructor_node_pinned_in(
        search_path,
        node_id,
        from,
        constructor_name,
        config,
        dependencies,
        grants,
        None,
    )
}
```

</details>

### <span id="fn.load_constructor_node_pinned_in"></span><code style="font-size: 0.85em">fn load_​constructor_​node_​pinned_​in</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_constructor_node_pinned_in(
    search_path: &Path,
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
    runtime_pin: Option<ProviderRuntime>,
) -> Result<Arc<dyn Task>, LoaderError>
```

[`load_constructor_node_pinned`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node_pinned) against an EXPLICIT provider directory (CLOACI-T-0925). See [`load_constructor_node_in`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node_in).

<details>
<summary>Source</summary>

```rust
pub fn load_constructor_node_pinned_in(
    search_path: &Path,
    node_id: &str,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    dependencies: Vec<TaskNamespace>,
    grants: GrantSpec,
    runtime_pin: Option<ProviderRuntime>,
) -> Result<Arc<dyn Task>, LoaderError> {
    let search_path = search_path.to_path_buf();
    let package_name = provider_package_name(from);

    // CLOACI-T-0920: settle the trust tier BEFORE any load work — both because a
    // grants/runtime mismatch must never get as far as loading code, and because
    // `find_wasm_package` below is wasm-only (fidius gates it on
    // `PackageRuntime::Wasm`), so a native provider would otherwise surface as an
    // opaque "package not found" rather than a runtime mismatch.
    let native_dir = enforce_runtime_pin(
        &format!("constructor node '{node_id}'"),
        from,
        &search_path,
        package_name,
        runtime_pin,
        &grants,
    )?;

    // Translate the tenant's grants into the fidius caps + egress policy. Fail
    // closed: a malformed grant aborts the load (it never silently widens access).
    let resolved = translate(&grants).map_err(|e| LoaderError::Validation {
        reason: format!("constructor node '{node_id}' (from = '{from}'): {e}"),
    })?;

    // Peek the manifest to learn the constructor's #[config] schema, so we can bind
    // `config = { name = value }` by NAME (reorder into declaration order) before
    // the positional bincode load. A NATIVE provider was already located by
    // `enforce_runtime_pin` (fidius's `find_wasm_package` is gated on
    // `PackageRuntime::Wasm` and would never match it).
    let dir = match native_dir {
        Some(dir) => dir,
        None => {
            let host = PluginHost::builder()
                .search_path(&search_path)
                .build()
                .map_err(|e| LoaderError::LibraryLoad {
                    path: search_path.display().to_string(),
                    error: format!("build plugin host: {e}"),
                })?;
            host.find_wasm_package(package_name)
                .map_err(|e| LoaderError::Validation {
                    reason: format!(
                        "resolve constructor node '{node_id}' (from = '{from}', constructor = \
                         '{constructor_name}'): locate provider package '{package_name}' in \
                         provider search path '{}': {e}",
                        search_path.display()
                    ),
                })?
        }
    };
    // Honor the author's `@version` pin against the resolved provider (T-0833).
    enforce_version_pin(&format!("constructor node '{node_id}'"), from, &dir)?;
    // Select the requested member from the provider suite (fails closed, naming the
    // available members, if `constructor` is not in this provider).
    let manifest = read_member_manifest(&dir, constructor_name)?;

    // Load-time capability lint (REQ-1.3.1, advisory): warn when the package
    // declares an intent to use a capability the tenant didn't grant — it will be
    // denied at runtime, so surface it now rather than as a mystery failure later.
    lint_constructor_grants(node_id, from, &dir, &grants);

    let ordered_config = bind_config_by_name(node_id, from, constructor_name, &manifest, config)?;

    let task = load_task_constructor(
        &search_path,
        package_name,
        constructor_name,
        &ordered_config,
        &resolved,
    )
    .map_err(|e| LoaderError::Validation {
        reason: format!(
            "resolve constructor node '{node_id}' (from = '{from}', constructor = \
                 '{constructor_name}') in provider search path '{}': {e}",
            search_path.display()
        ),
    })?;

    Ok(Arc::new(ConstructorNode {
        id: node_id.to_string(),
        inner: task,
        dependencies,
    }))
}
```

</details>

### <span id="fn.load_reactor_constructor_node"></span><code style="font-size: 0.85em">fn load_​reactor_​constructor_​node</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_reactor_constructor_node(
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    grants: GrantSpec,
) -> Result<Arc<dyn ReactorFireDecider>, LoaderError>
```

Resolve + load a packaged WASM **reactor** constructor as a firing decider for the CG scheduler (the runtime half of the `#[reactor(from = .., constructor = .., config = { .. })]` authoring form — CLOACI-T-0830).

Resolves `from` against the [`provider_search_path`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.provider_search_path), reads the resolved
constructor's manifest to learn its `#[config]` schema, binds the author's
`config = { name = value }` kwargs BY NAME (`bind_config_by_name`, reordered
into the guest's declaration order), loads the named reactor constructor via
[`load_reactor_constructor`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_reactor_constructor) (binding the reordered config once), validates the
resolved constructor's name matches `constructor_name`, and returns it as an
`Arc<dyn ReactorFireDecider>` ready for
[`Reactor::with_evaluator`](../../../../../rust/cloacina/computation_graph/reactor/#Reactor.with_evaluator).

`config` is the author's `config = { … }` entries as `(name, value)` pairs in
WRITTEN order; the reorder/coerce is identical to [`load_constructor_node`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node) (the
task/trigger node form) so config binds by NAME, not written order.

Fails closed (a clear `LoaderError`) if the provider is missing, the manifest
is unreadable / not a `Reactor`, the interface version mismatches, a config kwarg
is unknown / duplicated / missing / mistyped, or the resolved constructor name
does not match `constructor_name`.

<details>
<summary>Source</summary>

```rust
pub fn load_reactor_constructor_node(
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    grants: GrantSpec,
) -> Result<Arc<dyn ReactorFireDecider>, LoaderError> {
    load_reactor_constructor_node_pinned(from, constructor_name, config, grants, None)
}
```

</details>

### <span id="fn.load_reactor_constructor_node_pinned"></span><code style="font-size: 0.85em">fn load_​reactor_​constructor_​node_​pinned</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_reactor_constructor_node_pinned(
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    grants: GrantSpec,
    runtime_pin: Option<ProviderRuntime>,
) -> Result<Arc<dyn ReactorFireDecider>, LoaderError>
```

[`load_reactor_constructor_node`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_reactor_constructor_node) with the author's optional trust-tier pin (`#[reactor(.., runtime = "wasm"|"native")]` — CLOACI-T-0920). Same semantics as [`load_constructor_node_pinned`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node_pinned); see that function for why the pin is carried host-side rather than on the packaged FFI declaration.

<details>
<summary>Source</summary>

```rust
pub fn load_reactor_constructor_node_pinned(
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    grants: GrantSpec,
    runtime_pin: Option<ProviderRuntime>,
) -> Result<Arc<dyn ReactorFireDecider>, LoaderError> {
    load_reactor_constructor_node_pinned_in(
        &provider_search_path(),
        from,
        constructor_name,
        config,
        grants,
        runtime_pin,
    )
}
```

</details>

### <span id="fn.load_reactor_constructor_node_pinned_in"></span><code style="font-size: 0.85em">fn load_​reactor_​constructor_​node_​pinned_​in</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn load_reactor_constructor_node_pinned_in(
    search_path: &Path,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    grants: GrantSpec,
    runtime_pin: Option<ProviderRuntime>,
) -> Result<Arc<dyn ReactorFireDecider>, LoaderError>
```

[`load_reactor_constructor_node_pinned`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_reactor_constructor_node_pinned) against an EXPLICIT provider directory (CLOACI-T-0925). The CG scheduler resolves a reactor constructor on a `spawn_blocking` thread, i.e. off the load's task — it must carry the package's staged root rather than read process-global state that another tenant's load may already have re-pointed.

<details>
<summary>Source</summary>

```rust
pub fn load_reactor_constructor_node_pinned_in(
    search_path: &Path,
    from: &str,
    constructor_name: &str,
    config: Vec<(String, serde_json::Value)>,
    grants: GrantSpec,
    runtime_pin: Option<ProviderRuntime>,
) -> Result<Arc<dyn ReactorFireDecider>, LoaderError> {
    let search_path = search_path.to_path_buf();
    let package_name = provider_package_name(from);

    // CLOACI-T-0920: settle the trust tier before any load work (see
    // `load_constructor_node_pinned`).
    let native_dir = enforce_runtime_pin(
        &format!("reactor constructor '{constructor_name}'"),
        from,
        &search_path,
        package_name,
        runtime_pin,
        &grants,
    )?;

    // Translate the tenant's grants (fail closed on a malformed grant) before any
    // load work, so an invalid grant never silently widens access.
    let resolved = translate(&grants).map_err(|e| LoaderError::Validation {
        reason: format!("reactor constructor '{constructor_name}' (from = '{from}'): {e}"),
    })?;

    // Peek the manifest to learn the constructor's #[config] schema, so we can bind
    // `config = { name = value }` by NAME before the positional bincode load. A
    // NATIVE provider was already located by `enforce_runtime_pin` (see
    // `load_constructor_node_pinned`).
    let dir = match native_dir {
        Some(dir) => dir,
        None => {
            let host = PluginHost::builder()
                .search_path(&search_path)
                .build()
                .map_err(|e| LoaderError::LibraryLoad {
                    path: search_path.display().to_string(),
                    error: format!("build plugin host: {e}"),
                })?;
            host.find_wasm_package(package_name)
                .map_err(|e| LoaderError::Validation {
                    reason: format!(
                        "resolve reactor constructor (from = '{from}', constructor = \
                         '{constructor_name}'): locate provider package '{package_name}' in \
                         provider search path '{}': {e}",
                        search_path.display()
                    ),
                })?
        }
    };
    // Honor the author's `@version` pin against the resolved provider (T-0833).
    enforce_version_pin(
        &format!("reactor constructor '{constructor_name}'"),
        from,
        &dir,
    )?;
    // Select the requested member from the provider suite (fails closed if absent).
    let manifest = read_member_manifest(&dir, constructor_name)?;

    // Load-time capability lint (REQ-1.3.1, advisory).
    lint_constructor_grants(constructor_name, from, &dir, &grants);

    // Reuse the T-0829 name-keyed config binding (node_id = the constructor name,
    // since a reactor has no separate DAG node id).
    let ordered_config =
        bind_config_by_name(constructor_name, from, constructor_name, &manifest, config)?;

    let reactor_constructor = load_reactor_constructor(
        &search_path,
        package_name,
        constructor_name,
        &ordered_config,
        &resolved,
    )
    .map_err(|e| LoaderError::Validation {
        reason: format!(
            "resolve reactor constructor (from = '{from}', constructor = \
                 '{constructor_name}') in provider search path '{}': {e}",
            search_path.display()
        ),
    })?;

    Ok(Arc::new(reactor_constructor) as Arc<dyn ReactorFireDecider>)
}
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.CONSTRUCTOR_MANIFEST_FILE"></span>`CONSTRUCTOR_MANIFEST_FILE` | `&str` | `"constructor.json"` | The sidecar manifest filename inside a WASM constructor package. |
| <span id="const.PROVIDER_MANIFEST_FILE"></span>`PROVIDER_MANIFEST_FILE` | `&str` | `"provider.json"` | The provider-index filename inside a WASM provider package (CLOACI-A-0011): the `ProviderManifest` listing every member constructor the suite exposes. |
| <span id="const.PROVIDER_PATH_ENV"></span>`PROVIDER_PATH_ENV` | `&str` | `"CLOACINA_PROVIDER_PATH"` | Environment variable naming the directory `constructor!(from = ...)` resolves provider packages in (overridden by [`set_provider_search_path`](../../../../../rust/cloacina/registry/loader/constructor_loader/#fn.set_provider_search_path)). |
| <span id="const.DEFAULT_PROVIDER_DIR"></span>`DEFAULT_PROVIDER_DIR` | `&str` | `"providers"` | Default provider search-path directory (relative to CWD) when neither the process override nor [`PROVIDER_PATH_ENV`](../../../../../rust/cloacina/registry/loader/constructor_loader/#const.PROVIDER_PATH_ENV) is set. |
