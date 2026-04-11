# cloaca.python.task <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


## Classes

### `cloaca.python.task.TaskHandle`

> **Rust Implementation**: [cloacina::python::task::PyTaskHandle](../../rust/cloacina/python/task.md#class-taskhandle)

Python wrapper for TaskHandle providing defer_until capability.

#### Methods

##### `defer_until`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">defer_until</span>(condition: Any, poll_interval_ms: int) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::task::PyTaskHandle::defer_until](../../rust/cloacina/python/task.md#defer_until)

Release the concurrency slot while polling an external condition.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `condition` | `Any` |  |
| `poll_interval_ms` | `int` |  |


<details>
<summary>Source</summary>

```python
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

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">is_slot_held</span>() -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::task::PyTaskHandle::is_slot_held](../../rust/cloacina/python/task.md#is_slot_held)

Returns whether the handle currently holds a concurrency slot.

<details>
<summary>Source</summary>

```python
    pub fn is_slot_held(&self) -> PyResult<bool> {
        let handle = self
            .inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("TaskHandle has already been consumed"))?;
        Ok(handle.is_slot_held())
    }
```

</details>





### `cloaca.python.task.TaskDecorator`

> **Rust Implementation**: [cloacina::python::task::TaskDecorator](../../rust/cloacina/python/task.md#class-taskdecorator)

Decorator class that holds task configuration

#### Methods

##### `__call__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__call__</span>(func: Any) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::task::TaskDecorator::__call__](../../rust/cloacina/python/task.md#__call__)

<details>
<summary>Source</summary>

```python
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





## Functions

### `cloaca.python.task.task`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">task</span>(id: Optional[str], dependencies: Optional[List[Any]], retry_attempts: Optional[int], retry_backoff: Optional[str], retry_delay_ms: Optional[int], retry_max_delay_ms: Optional[int], retry_condition: Optional[str], retry_jitter: Optional[bool], on_success: Optional[Any], on_failure: Optional[Any]) -> <span style="color: var(--md-default-fg-color--light);">TaskDecorator</span></code>
</div>

> **Rust Implementation**: [cloacina::python::task::task](../../rust/cloacina/python/task.md#fn-task)

Python @task decorator function

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `Optional[str]` |  |
| `dependencies` | `Optional[List[Any]]` |  |
| `retry_attempts` | `Optional[int]` |  |
| `retry_backoff` | `Optional[str]` |  |
| `retry_delay_ms` | `Optional[int]` |  |
| `retry_max_delay_ms` | `Optional[int]` |  |
| `retry_condition` | `Optional[str]` |  |
| `retry_jitter` | `Optional[bool]` |  |
| `on_success` | `Optional[Any]` |  |
| `on_failure` | `Optional[Any]` |  |


<details>
<summary>Source</summary>

```python
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
