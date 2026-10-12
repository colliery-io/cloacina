---
title: "cloacina::executor::slot_token"
description: "Concurrency slot token for executor resource management."
sidebar:
  order: 139
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Concurrency slot token for executor resource management.

`SlotToken` wraps a semaphore permit and provides a clean interface for
releasing and reclaiming concurrency slots. This abstraction decouples
the TaskHandle from tokio's specific permit types, allowing future
extensions like weighted slots or cross-executor management.

## Structs

### <span id="struct.SlotToken"></span>`struct SlotToken`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


A token representing a held concurrency slot in the executor.

When a task is executing, it holds a `SlotToken` that reserves one of the
executor's concurrency slots. The token can be temporarily released (e.g.,
during deferred polling) and later reclaimed.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `permit` | `Option < OwnedSemaphorePermit >` |  |
| `semaphore` | `Arc < Semaphore >` |  |

#### Methods

##### <span id="SlotToken.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


```rust
fn new (permit : OwnedSemaphorePermit , semaphore : Arc < Semaphore >) -> Self
```

Creates a new SlotToken from an already-acquired permit.

<details>
<summary>Source</summary>

```rust
    pub(crate) fn new(permit: OwnedSemaphorePermit, semaphore: Arc<Semaphore>) -> Self {
        Self {
            permit: Some(permit),
            semaphore,
        }
    }
```

</details>



##### <span id="SlotToken.release"></span>`release`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn release (& mut self) -> bool
```

Release the concurrency slot, freeing it for other tasks.

This drops the semaphore permit, making the slot available immediately.
After calling `release()`, the token is in a released state and
`reclaim()` must be called before the task resumes real work.
Returns `true` if a permit was released, `false` if already released.

<details>
<summary>Source</summary>

```rust
    pub fn release(&mut self) -> bool {
        self.permit.take().is_some()
    }
```

</details>



##### <span id="SlotToken.reclaim"></span>`reclaim`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn reclaim (& mut self) -> Result < () , ExecutorError >
```

Reclaim a concurrency slot after it was released.

This acquires a new semaphore permit. If all slots are occupied,
this will wait until one becomes available.

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns an error if the semaphore is closed (executor shutting down). |


<details>
<summary>Source</summary>

```rust
    pub async fn reclaim(&mut self) -> Result<(), ExecutorError> {
        if self.permit.is_some() {
            // Already holding a permit, nothing to do
            return Ok(());
        }

        let permit = self.semaphore.clone().acquire_owned().await.map_err(|_| {
            ExecutorError::TaskExecution(crate::TaskError::ExecutionFailed {
                message: "semaphore closed during slot reclaim".into(),
                task_id: String::new(),
                timestamp: chrono::Utc::now(),
            })
        })?;

        self.permit = Some(permit);
        Ok(())
    }
```

</details>



##### <span id="SlotToken.is_held"></span>`is_held`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_held (& self) -> bool
```

Returns whether the token currently holds a concurrency slot.

<details>
<summary>Source</summary>

```rust
    pub fn is_held(&self) -> bool {
        self.permit.is_some()
    }
```

</details>





