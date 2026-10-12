---
title: "cloacina-python.task"
description: "API reference of the Python module cloacina-python.task."
sidebar:
  order: 12
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


## Classes

### <span id="class.TaskHandle"></span>`class TaskHandle`

> **Rust Implementation**: [cloacina-python::task::PyTaskHandle](../../rust/cloacina-python/task/#struct.PyTaskHandle)

Python wrapper for TaskHandle providing defer_until capability.

#### Methods

##### <span id="TaskHandle.defer_until"></span>`defer_until`

```python
def defer_until(condition: PyObject, poll_interval_ms: int) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::task::PyTaskHandle::defer_until](../../rust/cloacina-python/task/#PyTaskHandle.defer_until)

Release the concurrency slot while polling an external condition.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `condition` | `PyObject` |  |
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

        // CLOACI-I-0136: one GIL-safe async→sync bridge (was a hand-inlined
        // allow_threads + Handle::current().block_on).
        crate::gil::py_block_on(py, async {
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
        .map_err(|e| PyValueError::new_err(format!("defer_until failed: {}", e)))
    }
```

</details>



##### <span id="TaskHandle.is_slot_held"></span>`is_slot_held`

```python
def is_slot_held() -> PyResult < bool >
```

> **Rust Implementation**: [cloacina-python::task::PyTaskHandle::is_slot_held](../../rust/cloacina-python/task/#PyTaskHandle.is_slot_held)

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





### <span id="class.TaskDecorator"></span>`class TaskDecorator`

> **Rust Implementation**: [cloacina-python::task::TaskDecorator](../../rust/cloacina-python/task/#struct.TaskDecorator)

Decorator class that holds task configuration

#### Methods

##### <span id="TaskDecorator.__call__"></span>`__call__`

```python
def __call__(func: PyObject) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::task::TaskDecorator::__call__](../../rust/cloacina-python/task/#TaskDecorator.__call__)

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
        let trigger_rules = self.trigger_rules.clone();
        let function = func.clone_ref(py);
        let on_success_cb = self.on_success.as_ref().map(|f| f.clone_ref(py));
        let on_failure_cb = self.on_failure.as_ref().map(|f| f.clone_ref(py));

        // Validate `invokes` / `post_invocation` at decoration time. The
        // referenced graph must already be registered (the ComputationGraphBuilder's
        // `with` block must precede the @task that invokes it) and must be
        // trigger-less. Mirrors Rust T-0540 M3 / M5.
        if self.post_invocation.is_some() && self.invokes.is_none() {
            return Err(PyValueError::new_err(
                "@cloaca.task: `post_invocation` requires `invokes=...` to be set",
            ));
        }
        let cg_invocation = match self.invokes.as_ref() {
            None => None,
            Some(handle) => {
                let bound = handle.bind(py);
                if !bound.hasattr("NAME")? {
                    return Err(PyValueError::new_err(
                        "@cloaca.task(invokes=...): expected a ComputationGraphBuilder \
                         instance (must expose a `NAME` attribute)",
                    ));
                }
                let graph_name: String = bound.getattr("NAME")?.extract()?;
                let executor = crate::computation_graph::get_graph_executor(&graph_name)
                    .ok_or_else(|| {
                        PyValueError::new_err(format!(
                            "@cloaca.task(invokes=...): graph '{}' is not registered. \
                             The `with ComputationGraphBuilder(...)` block must run before \
                             the @cloaca.task decorator that references it.",
                            graph_name
                        ))
                    })?;
                if executor.has_reactor {
                    return Err(PyValueError::new_err(format!(
                        "@cloaca.task(invokes=...): graph '{}' is reactor-triggered. \
                         Tasks may only invoke trigger-less graphs (omit `reactor=...` \
                         on the ComputationGraphBuilder).",
                        graph_name
                    )));
                }
                let _ = executor; // existence verified above; runtime lookup happens at exec
                let post_invocation = self.post_invocation.as_ref().map(|f| f.clone_ref(py));
                Some(CGInvocation {
                    graph_name,
                    post_invocation,
                })
            }
        };
        let cg_invocation_arc = cg_invocation.map(Arc::new);

        let shared_function = Arc::new(function);
        let shared_on_success = on_success_cb.map(Arc::new);
        let shared_on_failure = on_failure_cb.map(Arc::new);
        let (tenant_id, package_name, workflow_id) = context.as_components();
        let namespace =
            cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &task_id);

        let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
            PyValueError::new_err(
                "@task decorator called outside a Runtime scope — install a ScopedRuntime \
                 before importing Python workflow modules",
            )
        })?;
        py.allow_threads(|| {
            rt.register_task(namespace.clone(), {
                let task_id_clone = task_id.clone();
                let deps_clone = deps.clone();
                let policy_clone = policy.clone();
                let function_arc = shared_function.clone();
                let on_success_arc = shared_on_success.clone();
                let on_failure_arc = shared_on_failure.clone();
                let cg_invocation_arc_inner = cg_invocation_arc.clone();
                let trigger_rules_clone = trigger_rules.clone();
                move || {
                    let function_clone = Python::with_gil(|py| function_arc.clone_ref(py));
                    let on_success_clone =
                        Python::with_gil(|py| on_success_arc.as_ref().map(|f| f.clone_ref(py)));
                    let on_failure_clone =
                        Python::with_gil(|py| on_failure_arc.as_ref().map(|f| f.clone_ref(py)));
                    let cg_invocation_clone = cg_invocation_arc_inner.as_ref().map(|cg| {
                        Python::with_gil(|py| CGInvocation {
                            graph_name: cg.graph_name.clone(),
                            post_invocation: cg.post_invocation.as_ref().map(|f| f.clone_ref(py)),
                        })
                    });
                    Arc::new(PythonTaskWrapper {
                        id: task_id_clone.clone(),
                        dependencies: deps_clone.clone(),
                        retry_policy: policy_clone.clone(),
                        python_function: function_clone,
                        on_success_callback: on_success_clone,
                        on_failure_callback: on_failure_clone,
                        requires_handle: has_handle,
                        cg_invocation: cg_invocation_clone,
                        trigger_rules: trigger_rules_clone.clone(),
                    }) as Arc<dyn cloacina::Task>
                }
            });
        });

        Ok(func)
    }
```

</details>





## Functions

### <span id="fn.task"></span>`task`

```python
def task(
    id: Option < String >,
    dependencies: Option < Vec < PyObject > >,
    retry: Option < crate :: bindings :: PyRetryPolicy >,
    retry_attempts: Option < usize >,
    retry_backoff: Option < String >,
    retry_delay_ms: Option < u64 >,
    retry_max_delay_ms: Option < u64 >,
    retry_condition: Option < String >,
    retry_jitter: Option < bool >,
    on_success: Option < PyObject >,
    on_failure: Option < PyObject >,
    invokes: Option < PyObject >,
    post_invocation: Option < PyObject >,
    trigger_rules: Option < PyObject >,
) -> PyResult < TaskDecorator >
```

> **Rust Implementation**: [cloacina-python::task::task](../../rust/cloacina-python/task/#fn.task)

Python @task decorator function

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `Option < String >` |  |
| `dependencies` | `Option < Vec < PyObject > >` |  |
| `retry` | `Option < crate :: bindings :: PyRetryPolicy >` |  |
| `retry_attempts` | `Option < usize >` |  |
| `retry_backoff` | `Option < String >` |  |
| `retry_delay_ms` | `Option < u64 >` |  |
| `retry_max_delay_ms` | `Option < u64 >` |  |
| `retry_condition` | `Option < String >` |  |
| `retry_jitter` | `Option < bool >` |  |
| `on_success` | `Option < PyObject >` |  |
| `on_failure` | `Option < PyObject >` |  |
| `invokes` | `Option < PyObject >` |  |
| `post_invocation` | `Option < PyObject >` |  |
| `trigger_rules` | `Option < PyObject >` |  |


<details>
<summary>Source</summary>

```python
pub fn task(
    py: Python,
    id: Option<String>,
    dependencies: Option<Vec<PyObject>>,
    retry: Option<crate::bindings::PyRetryPolicy>,
    retry_attempts: Option<usize>,
    retry_backoff: Option<String>,
    retry_delay_ms: Option<u64>,
    retry_max_delay_ms: Option<u64>,
    retry_condition: Option<String>,
    retry_jitter: Option<bool>,
    on_success: Option<PyObject>,
    on_failure: Option<PyObject>,
    invokes: Option<PyObject>,
    post_invocation: Option<PyObject>,
    trigger_rules: Option<PyObject>,
) -> PyResult<TaskDecorator> {
    // CLOACI-T-0882: two retry-authoring surfaces exist — the `retry_*` kwargs
    // and a `RetryPolicy` value object. Accept either, never both: silently
    // preferring one would make the ignored surface look effective, which is
    // exactly the "exposed but does nothing" failure this ticket was filed for.
    let kwargs_used: &[(&str, bool)] = &[
        ("retry_attempts", retry_attempts.is_some()),
        ("retry_backoff", retry_backoff.is_some()),
        ("retry_delay_ms", retry_delay_ms.is_some()),
        ("retry_max_delay_ms", retry_max_delay_ms.is_some()),
        ("retry_condition", retry_condition.is_some()),
        ("retry_jitter", retry_jitter.is_some()),
    ];
    let conflicting: Vec<&str> = kwargs_used
        .iter()
        .filter(|(_, used)| *used)
        .map(|(name, _)| *name)
        .collect();

    let retry_policy = match retry {
        Some(policy) => {
            if !conflicting.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "@task(retry=RetryPolicy(...)) cannot be combined with the \
                     retry_* keyword arguments (got: {}). Use the RetryPolicy \
                     object OR the keywords, not both.",
                    conflicting.join(", ")
                )));
            }
            policy.to_rust()
        }
        None => build_retry_policy(
            retry_attempts,
            retry_backoff,
            retry_delay_ms,
            retry_max_delay_ms,
            retry_condition,
            retry_jitter,
        )?,
    };

    // CLOACI-T-0763: parse + validate the trigger rule (default = Always).
    let trigger_rules = match trigger_rules {
        Some(obj) => crate::trigger_rules::parse_trigger_rules(py, &obj)?,
        None => serde_json::json!({ "type": "Always" }),
    };

    Ok(TaskDecorator {
        id,
        dependencies: dependencies.unwrap_or_default(),
        retry_policy,
        on_success,
        on_failure,
        invokes,
        post_invocation,
        trigger_rules,
    })
}
```

</details>



