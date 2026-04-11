# cloacina::executor::slot_token <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Concurrency slot token for executor resource management.

`SlotToken` wraps a semaphore permit and provides a clean interface for
releasing and reclaiming concurrency slots. This abstraction decouples
the TaskHandle from tokio's specific permit types, allowing future
extensions like weighted slots or cross-executor management.

## Structs

### `cloacina::executor::slot_token::SlotToken`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


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



##### `release` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



##### `reclaim` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


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



##### `is_held` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
