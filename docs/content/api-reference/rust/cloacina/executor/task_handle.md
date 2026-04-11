# cloacina::executor::task_handle <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task execution control handle.

`TaskHandle` provides execution control capabilities to tasks that opt in
by accepting it as a second parameter. The primary feature is `defer_until`,
which allows a task to release its concurrency slot while polling an
external condition.

**Examples:**

```rust,ignore
#[task(id = "wait_for_file")]
async fn wait_for_file(
    context: &mut Context<Value>,
    handle: &TaskHandle,
) -> Result<(), TaskError> {
    handle.defer_until(
        || async { std::path::Path::new("/data/input.csv").exists() },
        Duration::from_secs(5),
    ).await?;

    // File exists — slot has been reclaimed, proceed with work
    process_file(context).await
}
```

## Structs

### `cloacina::executor::task_handle::TaskHandle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Execution control handle passed to tasks that need concurrency management.

Tasks receive a `TaskHandle` as an optional second parameter. It provides
methods for controlling the task's relationship with the executor's
concurrency slots.
The handle is created by the executor for each task execution and is not
reusable across executions.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `slot_token` | `SlotToken` |  |
| `task_execution_id` | `UniversalUuid` |  |
| `dal` | `Option < DAL >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn new (slot_token : SlotToken , task_execution_id : UniversalUuid) -> Self
```

Creates a new TaskHandle.

This is called internally by the executor — tasks receive it as a parameter.

<details>
<summary>Source</summary>

```rust
    pub(crate) fn new(slot_token: SlotToken, task_execution_id: UniversalUuid) -> Self {
        Self {
            slot_token,
            task_execution_id,
            dal: None,
        }
    }
```

</details>



##### `with_dal` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn with_dal (slot_token : SlotToken , task_execution_id : UniversalUuid , dal : DAL ,) -> Self
```

Creates a new TaskHandle with DAL for sub_status persistence.

<details>
<summary>Source</summary>

```rust
    pub(crate) fn with_dal(
        slot_token: SlotToken,
        task_execution_id: UniversalUuid,
        dal: DAL,
    ) -> Self {
        Self {
            slot_token,
            task_execution_id,
            dal: Some(dal),
        }
    }
```

</details>



##### `defer_until` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn defer_until < F , Fut > (& mut self , condition : F , poll_interval : Duration ,) -> Result < () , ExecutorError > where F : Fn () -> Fut , Fut : Future < Output = bool > ,
```

Release the concurrency slot while polling an external condition.

This method:
1. Releases the executor concurrency slot (freeing it for other tasks)
2. Polls the condition function at the given interval
3. Reclaims a slot when the condition returns `true`
4. Returns control to the task with the slot re-held
While deferred, the task's async future remains parked in the tokio
runtime but does not consume a concurrency slot. Other tasks can use
the freed slot.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `condition` | `-` | Async function that returns `true` when the task should resume |
| `poll_interval` | `-` | How often to check the condition |


**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns an error if the semaphore is closed (executor shutting down) or if the slot cannot be reclaimed. |


<details>
<summary>Source</summary>

```rust
    pub async fn defer_until<F, Fut>(
        &mut self,
        condition: F,
        poll_interval: Duration,
    ) -> Result<(), ExecutorError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = bool>,
    {
        debug!(
            task_execution_id = %self.task_execution_id,
            poll_interval_ms = poll_interval.as_millis(),
            "Task entering deferred state — releasing concurrency slot"
        );

        // Update sub_status to Deferred in the database
        if let Some(ref dal) = self.dal {
            if let Err(e) = dal
                .task_execution()
                .set_sub_status(self.task_execution_id, Some("Deferred"))
                .await
            {
                warn!(
                    task_execution_id = %self.task_execution_id,
                    error = %e,
                    "Failed to set sub_status to Deferred"
                );
            }
        }

        // Release the concurrency slot
        self.slot_token.release();

        // Poll until condition is met
        loop {
            tokio::time::sleep(poll_interval).await;
            if condition().await {
                break;
            }
        }

        // Reclaim a concurrency slot (may wait if at capacity)
        self.slot_token.reclaim().await?;

        // Update sub_status back to Active
        if let Some(ref dal) = self.dal {
            if let Err(e) = dal
                .task_execution()
                .set_sub_status(self.task_execution_id, Some("Active"))
                .await
            {
                warn!(
                    task_execution_id = %self.task_execution_id,
                    error = %e,
                    "Failed to set sub_status back to Active"
                );
            }
        }

        debug!(
            task_execution_id = %self.task_execution_id,
            "Task resumed — concurrency slot reclaimed"
        );

        Ok(())
    }
```

</details>



##### `task_execution_id` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_execution_id (& self) -> UniversalUuid
```

Returns the task execution ID associated with this handle.

<details>
<summary>Source</summary>

```rust
    pub fn task_execution_id(&self) -> UniversalUuid {
        self.task_execution_id
    }
```

</details>



##### `is_slot_held` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_slot_held (& self) -> bool
```

Returns whether the handle currently holds a concurrency slot.

<details>
<summary>Source</summary>

```rust
    pub fn is_slot_held(&self) -> bool {
        self.slot_token.is_held()
    }
```

</details>



##### `into_slot_token` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn into_slot_token (self) -> SlotToken
```

Consumes the handle, returning the inner SlotToken.

Used by the executor to reclaim ownership of the permit after
task execution completes.

<details>
<summary>Source</summary>

```rust
    pub(crate) fn into_slot_token(self) -> SlotToken {
        self.slot_token
    }
```

</details>





## Functions

### `cloacina::executor::task_handle::take_task_handle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn take_task_handle () -> TaskHandle
```

Takes the current task's `TaskHandle` out of task-local storage.

Called by macro-generated code inside `Task::execute()`. Panics if no
handle was set (indicates an executor bug).

<details>
<summary>Source</summary>

```rust
pub fn take_task_handle() -> TaskHandle {
    TASK_HANDLE_SLOT.with(|cell| {
        cell.borrow_mut()
            .take()
            .expect("TaskHandle not set in task-local storage — executor bug")
    })
}
```

</details>



### `cloacina::executor::task_handle::return_task_handle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn return_task_handle (handle : TaskHandle)
```

Returns a `TaskHandle` to task-local storage after the user function completes.

Called by macro-generated code to restore the handle so the executor can
reclaim the slot token.

<details>
<summary>Source</summary>

```rust
pub fn return_task_handle(handle: TaskHandle) {
    TASK_HANDLE_SLOT.with(|cell| {
        *cell.borrow_mut() = Some(handle);
    })
}
```

</details>



### `cloacina::executor::task_handle::with_task_handle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn with_task_handle < F , T > (handle : TaskHandle , f : F) -> (T , Option < TaskHandle >) where F : Future < Output = T > ,
```

Runs an async future with a `TaskHandle` available in task-local storage.

The executor calls this to wrap `task.execute()` so that macro-generated
code can retrieve the handle via [`take_task_handle`].

<details>
<summary>Source</summary>

```rust
pub async fn with_task_handle<F, T>(handle: TaskHandle, f: F) -> (T, Option<TaskHandle>)
where
    F: Future<Output = T>,
{
    TASK_HANDLE_SLOT
        .scope(RefCell::new(Some(handle)), async {
            let result = f.await;
            let returned_handle = TASK_HANDLE_SLOT.with(|cell| cell.borrow_mut().take());
            (result, returned_handle)
        })
        .await
}
```

</details>
