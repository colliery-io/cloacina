---
title: "cloacina::executor::deferral_registry"
description: "Slot access for packaged tasks that defer (CLOACI-T-0897)."
sidebar:
  order: 136
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Slot access for packaged tasks that defer (CLOACI-T-0897).

An embedded task reaches its concurrency slot through the [`TaskHandle`]
the executor installs in a task-local. A PACKAGED task cannot: its
`defer_until` runs inside a cdylib and comes back through the
`CloacinaHost` callback channel, which lands on a tokio blocking-pool
thread (the host invokes packaged tasks via `spawn_blocking`) where that
task-local is invisible.
So the executor also registers the handle's slot here, keyed by the
task-execution UUID the callback carries. It is the SAME `Arc`, not a copy
— there is exactly one `SlotToken` per running task, whichever door reaches
it.
Entries are removed when the task finishes. A packaged task that never
defers simply never looks its entry up.

## Structs

### <span id="struct.DeferralEntry"></span>`struct DeferralEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


**Derives:** `Clone`

What a host callback needs to service a deferral for one running task.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `slot` | `Arc < AsyncMutex < SlotToken > >` | The running task's one and only slot, shared with its `TaskHandle`. |
| `dal` | `DAL` | For `sub_status` writes, so an operator can see a task is deferred
rather than wedged. |



## Functions

### <span id="fn.registry"></span>`fn registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn registry () -> & 'static Mutex < HashMap < UniversalUuid , DeferralEntry > >
```

Process-wide, because the binding fidius installs is per loaded library and the callback carries only a task id — there is no ambient executor to ask. Keyed by task-execution UUID, which is unique across tenants.

<details>
<summary>Source</summary>

```rust
fn registry() -> &'static Mutex<HashMap<UniversalUuid, DeferralEntry>> {
    static REGISTRY: OnceLock<Mutex<HashMap<UniversalUuid, DeferralEntry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}
```

</details>



### <span id="fn.register"></span>`fn register`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


```rust
fn register (task_execution_id : UniversalUuid , slot : Arc < AsyncMutex < SlotToken > > , dal : DAL ,)
```

Publish a running task's slot so host callbacks can find it.

<details>
<summary>Source</summary>

```rust
pub(crate) fn register(
    task_execution_id: UniversalUuid,
    slot: Arc<AsyncMutex<SlotToken>>,
    dal: DAL,
) {
    registry()
        .lock()
        .expect("deferral registry poisoned")
        .insert(task_execution_id, DeferralEntry { slot, dal });
}
```

</details>



### <span id="fn.deregister"></span>`fn deregister`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


```rust
fn deregister (task_execution_id : & UniversalUuid)
```

Remove a finished task's entry. Idempotent — a task that never deferred is removed the same way.

<details>
<summary>Source</summary>

```rust
pub(crate) fn deregister(task_execution_id: &UniversalUuid) {
    registry()
        .lock()
        .expect("deferral registry poisoned")
        .remove(task_execution_id);
}
```

</details>



### <span id="fn.lookup"></span>`fn lookup`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


```rust
fn lookup (task_execution_id : & UniversalUuid) -> Option < DeferralEntry >
```

Look up a running task's slot. `None` once the task has finished, which is what turns a late callback into a typed error rather than a panic.

<details>
<summary>Source</summary>

```rust
pub(crate) fn lookup(task_execution_id: &UniversalUuid) -> Option<DeferralEntry> {
    registry()
        .lock()
        .expect("deferral registry poisoned")
        .get(task_execution_id)
        .cloned()
}
```

</details>



### <span id="fn.len"></span>`fn len`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


```rust
fn len () -> usize
```

How many tasks are currently registered. Test-facing: lets a test prove entries are cleaned up rather than leaked.

<details>
<summary>Source</summary>

```rust
pub(crate) fn len() -> usize {
    registry().lock().expect("poisoned").len()
}
```

</details>



