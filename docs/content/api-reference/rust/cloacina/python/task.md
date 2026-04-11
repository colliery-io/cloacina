# cloacina::python::task <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::python::task::TaskHandle`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.task.TaskHandle](../../../cloaca/python/task.md#class-taskhandle)

Python wrapper for TaskHandle providing defer_until capability.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `Option < crate :: TaskHandle >` |  |

#### Methods

##### `defer_until`

```rust
fn defer_until (& mut self , py : Python , condition : PyObject , poll_interval_ms : u64 ,) -> PyResult < () >
```

> **Python API**: [cloaca.python.task.TaskHandle.defer_until](../../../cloaca/python/task.md#defer_until)

Release the concurrency slot while polling an external condition.

<details>
<summary>Source</summary>

```rust
    pub fn defer_until(
        &mut self,
        py: Python,
        condition: PyObject,
        poll_interval_ms: u64,
    ) -> PyResult<()> {
        let handle = self
            .inner
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("TaskHandle has already been consumed"))?;

        let poll_interval = Duration::from_millis(poll_interval_ms);
        let rt_handle = tokio::runtime::Handle::current();

        py.allow_threads(|| {
            rt_handle.block_on(async {
                handle
                    .defer_until(
                        move || {
                            let result = Python::with_gil(|py| match condition.call0(py) {
                                Ok(r) => r.extract::<bool>(py).unwrap_or(false),
                                Err(e) => {
                                    eprintln!("[cloaca] defer_until condition error: {}", e);
                                    false
                                }
                            });
                            async move { result }
                        },
                        poll_interval,
                    )
                    .await
            })
        })
        .map_err(|e| PyValueError::new_err(format!("defer_until failed: {}", e)))
    }
```

</details>



##### `is_slot_held`

```rust
fn is_slot_held (& self) -> PyResult < bool >
```

> **Python API**: [cloaca.python.task.TaskHandle.is_slot_held](../../../cloaca/python/task.md#is_slot_held)

Returns whether the handle currently holds a concurrency slot.

<details>
<summary>Source</summary>

```rust
    pub fn is_slot_held(&self) -> PyResult<bool> {
        let handle = self
            .inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("TaskHandle has already been consumed"))?;
        Ok(handle.is_slot_held())
    }
```

</details>





### `cloacina::python::task::WorkflowBuilderRef`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Workflow builder reference for automatic task registration

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `context` | `PyWorkflowContext` |  |



### `cloacina::python::task::PythonTaskWrapper`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Python task wrapper implementing Rust Task trait

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `String` |  |
| `dependencies` | `Vec < crate :: TaskNamespace >` |  |
| `retry_policy` | `crate :: retry :: RetryPolicy` |  |
| `python_function` | `PyObject` |  |
| `on_success_callback` | `Option < PyObject >` |  |
| `on_failure_callback` | `Option < PyObject >` |  |
| `requires_handle` | `bool` |  |



### `cloacina::python::task::TaskDecorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.task.TaskDecorator](../../../cloaca/python/task.md#class-taskdecorator)

Decorator class that holds task configuration

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Option < String >` |  |
| `dependencies` | `Vec < PyObject >` |  |
| `retry_policy` | `crate :: retry :: RetryPolicy` |  |
| `on_success` | `Option < PyObject >` |  |
| `on_failure` | `Option < PyObject >` |  |

#### Methods

##### `__call__`

```rust
fn __call__ (& self , py : Python , func : PyObject) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.task.TaskDecorator.__call__](../../../cloaca/python/task.md#__call__)

<details>
<summary>Source</summary>

```rust
    pub fn __call__(&self, py: Python, func: PyObject) -> PyResult<PyObject> {
        let context = current_workflow_context()?;

        let task_id = if let Some(id) = &self.id {
            id.clone()
        } else {
            func.getattr(py, "__name__")?.extract::<String>(py)?
        };

        let has_handle = {
            let code = func.getattr(py, "__code__")?;
            let argcount: usize = code.getattr(py, "co_argcount")?.extract(py)?;
            if argcount >= 2 {
                let varnames: Vec<String> = code.getattr(py, "co_varnames")?.extract(py)?;
                matches!(
                    varnames.get(1).map(|s| s.as_str()),
                    Some("handle" | "task_handle")
                )
            } else {
                false
            }
        };

        let deps = match self.convert_dependencies_to_namespaces(py, &context) {
            Ok(deps) => deps,
            Err(e) => {
                eprintln!("Error converting dependencies: {}", e);
                return Err(e);
            }
        };
        let policy = self.retry_policy.clone();
        let function = func.clone_ref(py);
        let on_success_cb = self.on_success.as_ref().map(|f| f.clone_ref(py));
        let on_failure_cb = self.on_failure.as_ref().map(|f| f.clone_ref(py));

        let shared_function = Arc::new(function);
        let shared_on_success = on_success_cb.map(Arc::new);
        let shared_on_failure = on_failure_cb.map(Arc::new);
        let (tenant_id, package_name, workflow_id) = context.as_components();
        let namespace = crate::TaskNamespace::new(tenant_id, package_name, workflow_id, &task_id);

        py.allow_threads(|| {
            crate::register_task_constructor(namespace.clone(), {
                let task_id_clone = task_id.clone();
                let deps_clone = deps.clone();
                let policy_clone = policy.clone();
                let function_arc = shared_function.clone();
                let on_success_arc = shared_on_success.clone();
                let on_failure_arc = shared_on_failure.clone();
                move || {
                    let function_clone = Python::with_gil(|py| function_arc.clone_ref(py));
                    let on_success_clone =
                        Python::with_gil(|py| on_success_arc.as_ref().map(|f| f.clone_ref(py)));
                    let on_failure_clone =
                        Python::with_gil(|py| on_failure_arc.as_ref().map(|f| f.clone_ref(py)));
                    Arc::new(PythonTaskWrapper {
                        id: task_id_clone.clone(),
                        dependencies: deps_clone.clone(),
                        retry_policy: policy_clone.clone(),
                        python_function: function_clone,
                        on_success_callback: on_success_clone,
                        on_failure_callback: on_failure_clone,
                        requires_handle: has_handle,
                    }) as Arc<dyn crate::Task>
                }
            });
        });

        Ok(func)
    }
```

</details>



##### `convert_dependencies_to_namespaces` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn convert_dependencies_to_namespaces (& self , py : Python , context : & PyWorkflowContext ,) -> PyResult < Vec < crate :: TaskNamespace > >
```

Convert mixed dependencies (strings and function objects) to TaskNamespace objects

<details>
<summary>Source</summary>

```rust
    fn convert_dependencies_to_namespaces(
        &self,
        py: Python,
        context: &PyWorkflowContext,
    ) -> PyResult<Vec<crate::TaskNamespace>> {
        let mut namespace_deps = Vec::new();

        for (i, dep) in self.dependencies.iter().enumerate() {
            let task_name = if let Ok(string_dep) = dep.extract::<String>(py) {
                string_dep
            } else {
                match dep.bind(py).hasattr("__name__") {
                    Ok(true) => match dep.getattr(py, "__name__") {
                        Ok(name_obj) => match name_obj.extract::<String>(py) {
                            Ok(func_name) => func_name,
                            Err(e) => {
                                return Err(PyValueError::new_err(format!(
                                    "Dependency {} has __name__ but it's not a string: {}",
                                    i, e
                                )));
                            }
                        },
                        Err(e) => {
                            return Err(PyValueError::new_err(format!(
                                "Failed to get __name__ from dependency {}: {}",
                                i, e
                            )));
                        }
                    },
                    Ok(false) => {
                        return Err(PyValueError::new_err(format!(
                            "Dependency {} must be either a string or a function object with __name__ attribute",
                            i
                        )));
                    }
                    Err(e) => {
                        return Err(PyValueError::new_err(format!(
                            "Failed to check if dependency {} has __name__ attribute: {}",
                            i, e
                        )));
                    }
                }
            };

            let (tenant_id, package_name, workflow_id) = context.as_components();
            namespace_deps.push(crate::TaskNamespace::new(
                tenant_id,
                package_name,
                workflow_id,
                &task_name,
            ));
        }

        Ok(namespace_deps)
    }
```

</details>





## Functions

### `cloacina::python::task::push_workflow_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn push_workflow_context (context : PyWorkflowContext)
```

Push a workflow context onto the stack (called when entering workflow scope)

<details>
<summary>Source</summary>

```rust
pub fn push_workflow_context(context: PyWorkflowContext) {
    WORKFLOW_CONTEXT_STACK
        .lock()
        .push(WorkflowBuilderRef { context });
}
```

</details>



### `cloacina::python::task::pop_workflow_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn pop_workflow_context () -> Option < WorkflowBuilderRef >
```

Pop a workflow context from the stack (called when exiting workflow scope)

<details>
<summary>Source</summary>

```rust
pub fn pop_workflow_context() -> Option<WorkflowBuilderRef> {
    WORKFLOW_CONTEXT_STACK.lock().pop()
}
```

</details>



### `cloacina::python::task::current_workflow_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn current_workflow_context () -> PyResult < PyWorkflowContext >
```

Get the current workflow context (used by task decorator)

<details>
<summary>Source</summary>

```rust
pub fn current_workflow_context() -> PyResult<PyWorkflowContext> {
    let stack = WORKFLOW_CONTEXT_STACK.lock();
    stack.last().map(|ref_| ref_.context.clone()).ok_or_else(|| {
        PyValueError::new_err(
            "No workflow context available. Tasks must be defined within a WorkflowBuilder context manager."
        )
    })
}
```

</details>



### `cloacina::python::task::build_retry_policy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn build_retry_policy (retry_attempts : Option < usize > , retry_backoff : Option < String > , retry_delay_ms : Option < u64 > , retry_max_delay_ms : Option < u64 > , retry_condition : Option < String > , retry_jitter : Option < bool > ,) -> crate :: retry :: RetryPolicy
```

Build retry policy from Python decorator parameters

<details>
<summary>Source</summary>

```rust
fn build_retry_policy(
    retry_attempts: Option<usize>,
    retry_backoff: Option<String>,
    retry_delay_ms: Option<u64>,
    retry_max_delay_ms: Option<u64>,
    retry_condition: Option<String>,
    retry_jitter: Option<bool>,
) -> crate::retry::RetryPolicy {
    use crate::retry::*;
    use std::time::Duration;

    let mut builder = RetryPolicy::builder();

    if let Some(attempts) = retry_attempts {
        builder = builder.max_attempts(attempts as i32);
    }

    if let Some(backoff) = retry_backoff {
        let strategy = match backoff.as_str() {
            "fixed" => BackoffStrategy::Fixed,
            "linear" => BackoffStrategy::Linear { multiplier: 1.0 },
            "exponential" => BackoffStrategy::Exponential {
                base: 2.0,
                multiplier: 1.0,
            },
            _ => BackoffStrategy::Fixed,
        };
        builder = builder.backoff_strategy(strategy);
    }

    if let Some(delay) = retry_delay_ms {
        builder = builder.initial_delay(Duration::from_millis(delay));
    }

    if let Some(max_delay) = retry_max_delay_ms {
        builder = builder.max_delay(Duration::from_millis(max_delay));
    }

    if let Some(condition) = retry_condition {
        let retry_cond = match condition.as_str() {
            "never" => RetryCondition::Never,
            "transient" => RetryCondition::TransientOnly,
            "all" => RetryCondition::AllErrors,
            _ => RetryCondition::AllErrors,
        };
        builder = builder.retry_condition(retry_cond);
    }

    if let Some(jitter) = retry_jitter {
        builder = builder.with_jitter(jitter);
    }

    builder.build()
}
```

</details>



### `cloacina::python::task::task`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.task.task](../../../cloaca/python/task.md#task)

```rust
fn task (id : Option < String > , dependencies : Option < Vec < PyObject > > , retry_attempts : Option < usize > , retry_backoff : Option < String > , retry_delay_ms : Option < u64 > , retry_max_delay_ms : Option < u64 > , retry_condition : Option < String > , retry_jitter : Option < bool > , on_success : Option < PyObject > , on_failure : Option < PyObject > ,) -> PyResult < TaskDecorator >
```

Python @task decorator function

<details>
<summary>Source</summary>

```rust
pub fn task(
    id: Option<String>,
    dependencies: Option<Vec<PyObject>>,
    retry_attempts: Option<usize>,
    retry_backoff: Option<String>,
    retry_delay_ms: Option<u64>,
    retry_max_delay_ms: Option<u64>,
    retry_condition: Option<String>,
    retry_jitter: Option<bool>,
    on_success: Option<PyObject>,
    on_failure: Option<PyObject>,
) -> PyResult<TaskDecorator> {
    let retry_policy = build_retry_policy(
        retry_attempts,
        retry_backoff,
        retry_delay_ms,
        retry_max_delay_ms,
        retry_condition,
        retry_jitter,
    );

    Ok(TaskDecorator {
        id,
        dependencies: dependencies.unwrap_or_default(),
        retry_policy,
        on_success,
        on_failure,
    })
}
```

</details>
