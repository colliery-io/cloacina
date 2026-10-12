---
title: "cloacina-python::task"
description: "API reference of the Rust module cloacina-python::task."
sidebar:
  order: 40
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


## Structs

### <span id="struct.PyTaskHandle"></span>`class TaskHandle`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.task.TaskHandle](../../../cloacina-python/task/#class.TaskHandle)

Python wrapper for TaskHandle providing defer_until capability.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `Option < cloacina :: TaskHandle >` |  |

#### Methods

##### <span id="PyTaskHandle.defer_until"></span>`defer_until`

```rust
fn defer_until (& mut self , py : Python , condition : PyObject , poll_interval_ms : u64 ,) -> PyResult < () >
```

> **Python API**: [cloacina-python.task.TaskHandle.defer_until](../../../cloacina-python/task/#TaskHandle.defer_until)

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



##### <span id="PyTaskHandle.is_slot_held"></span>`is_slot_held`

```rust
fn is_slot_held (& self) -> PyResult < bool >
```

> **Python API**: [cloacina-python.task.TaskHandle.is_slot_held](../../../cloacina-python/task/#TaskHandle.is_slot_held)

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





### <span id="struct.WorkflowBuilderRef"></span>`struct WorkflowBuilderRef`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Workflow builder reference for automatic task registration

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `context` | `PyWorkflowContext` |  |



### <span id="struct.ContextStack"></span>`struct ContextStack`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


Per-thread stack. The `Drop` impl exists so an import thread that dies mid-scope (e.g. the T-0919 timeout interrupt unwinding a hung import) does not leave `ACTIVE_CONTEXT_PUSHES` permanently inflated, which would make every later "no context" error claim a cross-thread problem that isn't one.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `RefCell < Vec < WorkflowBuilderRef > >` |  |



### <span id="struct.CGInvocation"></span>`struct CGInvocation`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Optional `@cloaca.task(invokes=..., post_invocation=...)` plumbing.

When set, the task's `execute()` runs the user body first, then calls
the named (trigger-less) computation graph with the task context, routes
each terminal output back into the context under its terminal node name,
and finally invokes the optional `post_invocation` callback. Mirrors
Rust's `#[task(invokes = computation_graph(...))]` (T-0540 M3+M5).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `graph_name` | `String` |  |
| `post_invocation` | `Option < PyObject >` |  |



### <span id="struct.PythonTaskWrapper"></span>`struct PythonTaskWrapper`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Python task wrapper implementing Rust Task trait

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `String` |  |
| `dependencies` | `Vec < cloacina :: TaskNamespace >` |  |
| `retry_policy` | `cloacina :: retry :: RetryPolicy` |  |
| `python_function` | `PyObject` |  |
| `on_success_callback` | `Option < PyObject >` |  |
| `on_failure_callback` | `Option < PyObject >` |  |
| `requires_handle` | `bool` |  |
| `cg_invocation` | `Option < CGInvocation >` |  |
| `trigger_rules` | `serde_json :: Value` | Authored trigger rule as canonical TriggerRule JSON (CLOACI-T-0763).
`{"type":"Always"}` when the task declares no rule. |

#### Methods

##### <span id="PythonTaskWrapper.fire_on_failure"></span>`fire_on_failure`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn fire_on_failure (& self , task_id : & str , message : & str , context : & cloacina :: Context < serde_json :: Value > , on_failure : Option < & PyObject > ,)
```

Helper: invoke an `on_failure` callback (if any) with a fresh `PyContext` built from the current task context. Errors from the callback are logged and swallowed — the task already has its own terminal error to surface. CLOACI-T-0622 follow-up: runs the callback on the blocking pool, NOT a direct `Python::with_gil` on the async executor worker (the PyO3↔tokio deadlock that hung scenario_30, and via the CG path, scenario_32). `async` so callers `.await` it; the GIL is acquired only inside `spawn_blocking`.

<details>
<summary>Source</summary>

```rust
    async fn fire_on_failure(
        &self,
        task_id: &str,
        message: &str,
        context: &cloacina::Context<serde_json::Value>,
        on_failure: Option<&PyObject>,
    ) {
        let Some(callback) = on_failure else {
            return;
        };
        let callback = Python::with_gil(|py| callback.clone_ref(py));
        let task_id = task_id.to_string();
        let message = message.to_string();
        let data: Vec<(String, serde_json::Value)> = context
            .data()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let _ = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let mut ctx = cloacina::Context::new();
                for (k, v) in data {
                    ctx.insert(k, v).ok();
                }
                let py_ctx = super::context::PyContext::from_rust_context(ctx);
                if let Err(e) = callback.call1(py, (&task_id, &message, py_ctx)) {
                    eprintln!(
                        "[cloaca] on_failure callback failed for task '{}': {}",
                        task_id, e
                    );
                }
            })
        })
        .await;
    }
```

</details>





### <span id="struct.TaskDecorator"></span>`class TaskDecorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.task.TaskDecorator](../../../cloacina-python/task/#class.TaskDecorator)

Decorator class that holds task configuration

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Option < String >` |  |
| `dependencies` | `Vec < PyObject >` |  |
| `retry_policy` | `cloacina :: retry :: RetryPolicy` |  |
| `on_success` | `Option < PyObject >` |  |
| `on_failure` | `Option < PyObject >` |  |
| `invokes` | `Option < PyObject >` | `invokes=GraphHandle` — an instance of `ComputationGraphBuilder`
(or any object exposing `NAME`) that names a trigger-less graph. |
| `post_invocation` | `Option < PyObject >` | `post_invocation=fn` — only meaningful when `invokes` is set. |
| `trigger_rules` | `serde_json :: Value` | `trigger_rules=...` — canonical TriggerRule JSON (CLOACI-T-0763). |

#### Methods

##### <span id="TaskDecorator.__call__"></span>`__call__`

```rust
fn __call__ (& self , py : Python , func : PyObject) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.task.TaskDecorator.__call__](../../../cloacina-python/task/#TaskDecorator.__call__)

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



##### <span id="TaskDecorator.convert_dependencies_to_namespaces"></span>`convert_dependencies_to_namespaces`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn convert_dependencies_to_namespaces (& self , py : Python , context : & PyWorkflowContext ,) -> PyResult < Vec < cloacina :: TaskNamespace > >
```

Convert mixed dependencies (strings and function objects) to TaskNamespace objects

<details>
<summary>Source</summary>

```rust
    fn convert_dependencies_to_namespaces(
        &self,
        py: Python,
        context: &PyWorkflowContext,
    ) -> PyResult<Vec<cloacina::TaskNamespace>> {
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
            namespace_deps.push(cloacina::TaskNamespace::new(
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

### <span id="fn.push_workflow_context"></span>`fn push_workflow_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn push_workflow_context (context : PyWorkflowContext)
```

Push a workflow context onto this THREAD's stack (called when entering workflow scope). The matching [`pop_workflow_context`] must run on the same thread — decorated modules are imported on one thread by construction.

<details>
<summary>Source</summary>

```rust
pub fn push_workflow_context(context: PyWorkflowContext) {
    ACTIVE_CONTEXT_PUSHES.fetch_add(1, Ordering::SeqCst);
    WORKFLOW_CONTEXT_STACK.with(|stack| stack.0.borrow_mut().push(WorkflowBuilderRef { context }));
}
```

</details>



### <span id="fn.pop_workflow_context"></span>`fn pop_workflow_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn pop_workflow_context () -> Option < WorkflowBuilderRef >
```

Pop a workflow context from this thread's stack (called when exiting workflow scope).

<details>
<summary>Source</summary>

```rust
pub fn pop_workflow_context() -> Option<WorkflowBuilderRef> {
    let popped = WORKFLOW_CONTEXT_STACK.with(|stack| stack.0.borrow_mut().pop());
    if popped.is_some() {
        ACTIVE_CONTEXT_PUSHES.fetch_sub(1, Ordering::SeqCst);
    } else {
        // A pop with nothing to pop is either unbalanced `__exit__` handling
        // or a pop on the wrong thread. Neither is recoverable state, but it
        // is not worth aborting a load over — say so loudly instead.
        debug_assert!(
            false,
            "pop_workflow_context() on a thread with no workflow context — \
             unbalanced push/pop, or the pop ran on a different thread than \
             the push (CLOACI-T-0919)"
        );
        tracing::warn!(
            "pop_workflow_context() called with no context on this thread \
             (unbalanced push/pop, or cross-thread use)"
        );
    }
    popped
}
```

</details>



### <span id="fn.current_workflow_context"></span>`fn current_workflow_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn current_workflow_context () -> PyResult < PyWorkflowContext >
```

Get the current workflow context (used by the task decorator).

Errors when this thread has no context. If some OTHER thread does, the
error says so explicitly — that is the cross-thread misuse the global stack
used to paper over by handing back a foreign namespace.

<details>
<summary>Source</summary>

```rust
pub fn current_workflow_context() -> PyResult<PyWorkflowContext> {
    let local =
        WORKFLOW_CONTEXT_STACK.with(|stack| stack.0.borrow().last().map(|r| r.context.clone()));
    if let Some(context) = local {
        return Ok(context);
    }

    if ACTIVE_CONTEXT_PUSHES.load(Ordering::SeqCst) > 0 {
        // Deliberately a hard error in EVERY build rather than a
        // `debug_assert!` panic: this is reachable from user Python code
        // (a module that spawns a thread and decorates from it), and a
        // panic across the pyo3 boundary is strictly worse than a raised
        // exception. The debug assertion for genuine internal misuse lives
        // on the pop path, which user code cannot reach.
        tracing::error!(
            "cross-thread workflow-context use — a @task/constructor decorator ran on a \
             thread with no workflow context while another thread holds one (CLOACI-T-0919)"
        );
        return Err(PyValueError::new_err(
            "No workflow context on this thread, but one is active on a DIFFERENT thread. \
             The workflow context is thread-local: `@task`/`cloaca.constructor` must be \
             evaluated on the same thread that entered the WorkflowBuilder scope (i.e. at \
             module scope during import), never from a thread spawned by the module. \
             Registering from another thread would silently namespace the task under the \
             wrong tenant/package.",
        ));
    }

    Err(PyValueError::new_err(
        "No workflow context available. Tasks must be defined within a WorkflowBuilder context manager.",
    ))
}
```

</details>



### <span id="fn.build_retry_policy"></span>`fn build_retry_policy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn build_retry_policy (retry_attempts : Option < usize > , retry_backoff : Option < String > , retry_delay_ms : Option < u64 > , retry_max_delay_ms : Option < u64 > , retry_condition : Option < String > , retry_jitter : Option < bool > ,) -> PyResult < cloacina :: retry :: RetryPolicy >
```

Build retry policy from Python decorator parameters.

CLOACI-T-0930: unrecognized `retry_backoff` / `retry_condition` strings are
ERRORS, not silent defaults. The old `_ =>` fallthroughs meant
`retry_backoff="exponentail"` (typo) quietly configured Fixed backoff and an
unknown condition quietly retried ALL errors — the workflow ran with retry
semantics its author never asked for, invisibly. Failing loudly here is a
deliberate behavior change: the population currently hitting the fallback
is exactly the misconfigured one.

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
) -> PyResult<cloacina::retry::RetryPolicy> {
    use cloacina::retry::*;
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
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown retry_backoff {other:?}: expected one of \
                     \"fixed\", \"linear\", \"exponential\""
                )))
            }
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
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown retry_condition {other:?}: expected one of \
                     \"never\", \"transient\", \"all\""
                )))
            }
        };
        builder = builder.retry_condition(retry_cond);
    }

    if let Some(jitter) = retry_jitter {
        builder = builder.with_jitter(jitter);
    }

    Ok(builder.build())
}
```

</details>



### <span id="fn.task"></span>`fn task`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.task.task](../../../cloacina-python/task/#fn.task)

```rust
fn task (py : Python , id : Option < String > , dependencies : Option < Vec < PyObject > > , retry : Option < crate :: bindings :: PyRetryPolicy > , retry_attempts : Option < usize > , retry_backoff : Option < String > , retry_delay_ms : Option < u64 > , retry_max_delay_ms : Option < u64 > , retry_condition : Option < String > , retry_jitter : Option < bool > , on_success : Option < PyObject > , on_failure : Option < PyObject > , invokes : Option < PyObject > , post_invocation : Option < PyObject > , trigger_rules : Option < PyObject > ,) -> PyResult < TaskDecorator >
```

Python @task decorator function

<details>
<summary>Source</summary>

```rust
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



