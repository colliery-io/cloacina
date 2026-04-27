/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use super::workflow_context::PyWorkflowContext;
use async_trait::async_trait;
use parking_lot::Mutex;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::Arc;
use std::time::Duration;

/// Python wrapper for TaskHandle providing defer_until capability.
#[pyclass(name = "TaskHandle")]
pub struct PyTaskHandle {
    inner: Option<cloacina::TaskHandle>,
}

#[pymethods]
impl PyTaskHandle {
    /// Release the concurrency slot while polling an external condition.
    #[pyo3(signature = (condition, poll_interval_ms = 1000))]
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

    /// Returns whether the handle currently holds a concurrency slot.
    pub fn is_slot_held(&self) -> PyResult<bool> {
        let handle = self
            .inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("TaskHandle has already been consumed"))?;
        Ok(handle.is_slot_held())
    }
}

/// Workflow builder reference for automatic task registration
#[derive(Clone)]
pub struct WorkflowBuilderRef {
    pub context: PyWorkflowContext,
}

/// Global context stack for workflow-scoped task registration
static WORKFLOW_CONTEXT_STACK: Mutex<Vec<WorkflowBuilderRef>> = Mutex::new(Vec::new());

/// Push a workflow context onto the stack (called when entering workflow scope)
pub fn push_workflow_context(context: PyWorkflowContext) {
    WORKFLOW_CONTEXT_STACK
        .lock()
        .push(WorkflowBuilderRef { context });
}

/// Pop a workflow context from the stack (called when exiting workflow scope)
pub fn pop_workflow_context() -> Option<WorkflowBuilderRef> {
    WORKFLOW_CONTEXT_STACK.lock().pop()
}

/// Get the current workflow context (used by task decorator)
pub fn current_workflow_context() -> PyResult<PyWorkflowContext> {
    let stack = WORKFLOW_CONTEXT_STACK.lock();
    stack.last().map(|ref_| ref_.context.clone()).ok_or_else(|| {
        PyValueError::new_err(
            "No workflow context available. Tasks must be defined within a WorkflowBuilder context manager."
        )
    })
}

/// Optional `@cloaca.task(invokes=..., post_invocation=...)` plumbing.
///
/// When set, the task's `execute()` runs the user body first, then calls
/// the named (trigger-less) computation graph with the task context, routes
/// each terminal output back into the context under its terminal node name,
/// and finally invokes the optional `post_invocation` callback. Mirrors
/// Rust's `#[task(invokes = computation_graph(...))]` (T-0540 M3+M5).
pub struct CGInvocation {
    pub graph_name: String,
    pub terminal_names: Vec<String>,
    pub post_invocation: Option<PyObject>,
}

/// Python task wrapper implementing Rust Task trait
pub struct PythonTaskWrapper {
    id: String,
    dependencies: Vec<cloacina::TaskNamespace>,
    retry_policy: cloacina::retry::RetryPolicy,
    python_function: PyObject,
    on_success_callback: Option<PyObject>,
    on_failure_callback: Option<PyObject>,
    requires_handle: bool,
    cg_invocation: Option<CGInvocation>,
}

impl PythonTaskWrapper {
    /// Helper: invoke an `on_failure` callback (if any) with a fresh
    /// `PyContext` built from the current task context. Errors from the
    /// callback are logged and swallowed — the task already has its own
    /// terminal error to surface.
    fn fire_on_failure(
        &self,
        task_id: &str,
        message: &str,
        context: &cloacina::Context<serde_json::Value>,
        on_failure: Option<&PyObject>,
    ) {
        let Some(callback) = on_failure else {
            return;
        };
        Python::with_gil(|py| {
            let mut ctx = cloacina::Context::new();
            for (k, v) in context.data().iter() {
                ctx.insert(k.clone(), v.clone()).ok();
            }
            let py_ctx = super::context::PyContext::from_rust_context(ctx);
            if let Err(e) = callback.call1(py, (task_id, message, py_ctx)) {
                eprintln!(
                    "[cloaca] on_failure callback failed for task '{}': {}",
                    task_id, e
                );
            }
        });
    }
}

// SAFETY: PythonTaskWrapper holds PyObject fields which are not Send/Sync.
// This is safe because ALL access to PyObject fields goes through Python::with_gil()
// or tokio::task::spawn_blocking + Python::with_gil(), ensuring the GIL is held
// before any PyObject is touched. The execute() method clones PyObjects inside
// with_gil and runs Python calls inside spawn_blocking(with_gil(...)). No code
// path accesses PyObject fields without the GIL.
unsafe impl Send for PythonTaskWrapper {}
unsafe impl Sync for PythonTaskWrapper {}

#[async_trait]
impl cloacina::Task for PythonTaskWrapper {
    async fn execute(
        &self,
        context: cloacina::Context<serde_json::Value>,
    ) -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError> {
        use super::context::PyContext;

        let function = Python::with_gil(|py| self.python_function.clone_ref(py));
        let on_success =
            Python::with_gil(|py| self.on_success_callback.as_ref().map(|f| f.clone_ref(py)));
        let on_failure =
            Python::with_gil(|py| self.on_failure_callback.as_ref().map(|f| f.clone_ref(py)));
        let task_id = self.id.clone();
        let task_id_for_error = self.id.clone();
        let needs_handle = self.requires_handle;

        let task_handle = if needs_handle {
            Some(cloacina::take_task_handle())
        } else {
            None
        };

        // 1. Run the user body (sync, on the spawn_blocking pool, under the
        //    GIL). on_success / post_invocation / CG invocation all happen
        //    later in the async caller so the graph can be `.await`ed.
        let task_id_for_body = task_id.clone();
        let on_failure_for_body =
            Python::with_gil(|py| on_failure.as_ref().map(|f| f.clone_ref(py)));
        let (mut final_context, returned_handle) = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let original_data = context.data().clone();
                let py_context = PyContext::from_rust_context(context);

                let (result, recovered_handle) = if let Some(handle) = task_handle {
                    let py_handle = Py::new(
                        py,
                        PyTaskHandle {
                            inner: Some(handle),
                        },
                    )
                    .map_err(|e| cloacina::TaskError::ExecutionFailed {
                        message: format!("Failed to create PyTaskHandle: {}", e),
                        task_id: task_id_for_body.clone(),
                        timestamp: chrono::Utc::now(),
                    })?;
                    let call_result =
                        function.call1(py, (py_context.clone(), py_handle.clone_ref(py)));
                    let recovered = py_handle.borrow_mut(py).inner.take();
                    (call_result, recovered)
                } else {
                    let call_result = function.call1(py, (py_context.clone(),));
                    (call_result, None)
                };

                match result {
                    Ok(returned) => {
                        let final_context = if returned.is_none(py) {
                            let mut new_context = cloacina::Context::new();
                            for (key, value) in original_data.iter() {
                                new_context.insert(key.clone(), value.clone()).unwrap();
                            }
                            new_context
                        } else {
                            let returned_context: PyContext =
                                returned.extract(py).map_err(|e| {
                                    cloacina::TaskError::ExecutionFailed {
                                        message: format!("Python task execution failed: {}", e),
                                        task_id: task_id_for_body.clone(),
                                        timestamp: chrono::Utc::now(),
                                    }
                                })?;
                            returned_context.into_inner()
                        };
                        Ok::<_, cloacina::TaskError>((final_context, recovered_handle))
                    }
                    Err(e) => {
                        let error_message = format!("Python task execution failed: {}", e);
                        if let Some(callback) = on_failure_for_body {
                            if let Err(callback_err) =
                                callback.call1(py, (&task_id_for_body, &error_message, py_context))
                            {
                                eprintln!(
                                    "[cloaca] on_failure callback failed for task '{}': {}",
                                    task_id_for_body, callback_err
                                );
                            }
                        }
                        Err(cloacina::TaskError::ExecutionFailed {
                            message: error_message,
                            task_id: task_id_for_body.clone(),
                            timestamp: chrono::Utc::now(),
                        })
                    }
                }
            })
        })
        .await
        .map_err(|e| cloacina::TaskError::ExecutionFailed {
            message: format!("Task execution panicked: {}", e),
            task_id: task_id_for_error.clone(),
            timestamp: chrono::Utc::now(),
        })??;

        // 2. Optional CG invocation (mirrors Rust T-0540 M3): run the graph
        //    with the task context, route each terminal back under its node
        //    name, error out on graph failure (after firing on_failure).
        if let Some(invocation) = self.cg_invocation.as_ref() {
            let executor = crate::computation_graph::get_graph_executor(&invocation.graph_name)
                .ok_or_else(|| {
                    let msg = format!(
                        "task '{}' invokes computation graph '{}' which is not registered",
                        task_id, invocation.graph_name
                    );
                    self.fire_on_failure(&task_id, &msg, &final_context, on_failure.as_ref());
                    cloacina::TaskError::ExecutionFailed {
                        message: msg,
                        task_id: task_id.clone(),
                        timestamp: chrono::Utc::now(),
                    }
                })?;

            let py_ctx_obj = Python::with_gil(|py| {
                let mut ctx = cloacina::Context::new();
                for (k, v) in final_context.data().iter() {
                    ctx.insert(k.clone(), v.clone()).ok();
                }
                Py::new(py, PyContext::from_rust_context(ctx)).map(|p| p.into_any())
            })
            .map_err(|e| {
                let msg = format!("failed to materialize PyContext for CG invocation: {}", e);
                cloacina::TaskError::ExecutionFailed {
                    message: msg,
                    task_id: task_id.clone(),
                    timestamp: chrono::Utc::now(),
                }
            })?;

            let graph_result = executor.execute_trigger_less(py_ctx_obj).await;
            match graph_result {
                cloacina::computation_graph::GraphResult::Completed { outputs } => {
                    for (idx, name) in invocation.terminal_names.iter().enumerate() {
                        if let Some(boxed) = outputs.get(idx) {
                            if let Some(value) = boxed.downcast_ref::<serde_json::Value>() {
                                if final_context.get(name).is_some() {
                                    let _ = final_context.update(name, value.clone());
                                } else {
                                    let _ = final_context.insert(name.clone(), value.clone());
                                }
                            }
                        }
                    }
                }
                cloacina::computation_graph::GraphResult::Error(graph_err) => {
                    let msg = format!(
                        "computation graph '{}' invocation failed: {}",
                        invocation.graph_name, graph_err
                    );
                    self.fire_on_failure(&task_id, &msg, &final_context, on_failure.as_ref());
                    return Err(cloacina::TaskError::ExecutionFailed {
                        message: msg,
                        task_id: task_id.clone(),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }

            // 3. Optional post_invocation hook (mirrors Rust T-0540 M5).
            if let Some(post) = invocation.post_invocation.as_ref() {
                let post_result = Python::with_gil(|py| -> PyResult<()> {
                    let mut ctx = cloacina::Context::new();
                    for (k, v) in final_context.data().iter() {
                        ctx.insert(k.clone(), v.clone()).ok();
                    }
                    let py_ctx = PyContext::from_rust_context(ctx);
                    let returned = post.call1(py, (py_ctx,))?;
                    if !returned.is_none(py) {
                        if let Ok(updated) = returned.extract::<PyContext>(py) {
                            final_context = updated.into_inner();
                        }
                    }
                    Ok(())
                });
                if let Err(e) = post_result {
                    let msg = format!("post_invocation callback failed: {}", e);
                    self.fire_on_failure(&task_id, &msg, &final_context, on_failure.as_ref());
                    return Err(cloacina::TaskError::ExecutionFailed {
                        message: msg,
                        task_id: task_id.clone(),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
        }

        // 4. on_success runs last, after CG invocation + post_invocation.
        if let Some(callback) = on_success {
            Python::with_gil(|py| {
                let mut callback_ctx = cloacina::Context::new();
                for (k, v) in final_context.data().iter() {
                    callback_ctx.insert(k.clone(), v.clone()).ok();
                }
                let callback_context = PyContext::from_rust_context(callback_ctx);
                if let Err(e) = callback.call1(py, (&task_id, callback_context)) {
                    eprintln!(
                        "[cloaca] on_success callback failed for task '{}': {}",
                        task_id, e
                    );
                }
            });
        }

        if let Some(handle) = returned_handle {
            cloacina::return_task_handle(handle);
        }

        Ok(final_context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[cloacina::TaskNamespace] {
        &self.dependencies
    }

    fn retry_policy(&self) -> cloacina::retry::RetryPolicy {
        self.retry_policy.clone()
    }

    fn requires_handle(&self) -> bool {
        self.requires_handle
    }

    fn checkpoint(
        &self,
        _context: &cloacina::Context<serde_json::Value>,
    ) -> Result<(), cloacina::CheckpointError> {
        Ok(())
    }

    fn trigger_rules(&self) -> serde_json::Value {
        serde_json::json!({"type": "Always"})
    }

    fn code_fingerprint(&self) -> Option<String> {
        None
    }
}

/// Build retry policy from Python decorator parameters
fn build_retry_policy(
    retry_attempts: Option<usize>,
    retry_backoff: Option<String>,
    retry_delay_ms: Option<u64>,
    retry_max_delay_ms: Option<u64>,
    retry_condition: Option<String>,
    retry_jitter: Option<bool>,
) -> cloacina::retry::RetryPolicy {
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

/// Decorator class that holds task configuration
#[pyclass]
pub struct TaskDecorator {
    id: Option<String>,
    dependencies: Vec<PyObject>,
    retry_policy: cloacina::retry::RetryPolicy,
    on_success: Option<PyObject>,
    on_failure: Option<PyObject>,
    /// `invokes=GraphHandle` — an instance of `ComputationGraphBuilder`
    /// (or any object exposing `NAME`) that names a trigger-less graph.
    invokes: Option<PyObject>,
    /// `post_invocation=fn` — only meaningful when `invokes` is set.
    post_invocation: Option<PyObject>,
}

#[pymethods]
impl TaskDecorator {
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
                let post_invocation = self.post_invocation.as_ref().map(|f| f.clone_ref(py));
                Some(CGInvocation {
                    graph_name,
                    terminal_names: executor.terminal_names(),
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
                move || {
                    let function_clone = Python::with_gil(|py| function_arc.clone_ref(py));
                    let on_success_clone =
                        Python::with_gil(|py| on_success_arc.as_ref().map(|f| f.clone_ref(py)));
                    let on_failure_clone =
                        Python::with_gil(|py| on_failure_arc.as_ref().map(|f| f.clone_ref(py)));
                    let cg_invocation_clone = cg_invocation_arc_inner.as_ref().map(|cg| {
                        Python::with_gil(|py| CGInvocation {
                            graph_name: cg.graph_name.clone(),
                            terminal_names: cg.terminal_names.clone(),
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
                    }) as Arc<dyn cloacina::Task>
                }
            });
        });

        Ok(func)
    }
}

impl TaskDecorator {
    /// Convert mixed dependencies (strings and function objects) to TaskNamespace objects
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
}

/// Python @task decorator function
#[pyfunction]
#[pyo3(signature = (
    *,
    id = None,
    dependencies = None,
    retry_attempts = None,
    retry_backoff = None,
    retry_delay_ms = None,
    retry_max_delay_ms = None,
    retry_condition = None,
    retry_jitter = None,
    on_success = None,
    on_failure = None,
    invokes = None,
    post_invocation = None
))]
#[allow(clippy::too_many_arguments)]
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
    invokes: Option<PyObject>,
    post_invocation: Option<PyObject>,
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
        invokes,
        post_invocation,
    })
}

#[cfg(test)]
mod m3_tests {
    use super::*;
    use crate::computation_graph;
    use crate::reactor;
    use crate::runtime_scope::ScopedRuntime;
    use pyo3::ffi::c_str;
    use serial_test::serial;
    use std::sync::Arc;

    /// Inject the cloaca decorators a Python `with`/decorator block needs.
    fn build_locals(py: Python<'_>) -> Bound<'_, pyo3::types::PyDict> {
        let locals = pyo3::types::PyDict::new(py);
        locals
            .set_item(
                "node",
                pyo3::wrap_pyfunction!(computation_graph::node, py).unwrap(),
            )
            .unwrap();
        locals
            .set_item(
                "ComputationGraphBuilder",
                py.get_type::<computation_graph::PyComputationGraphBuilder>(),
            )
            .unwrap();
        locals
            .set_item(
                "reactor",
                pyo3::wrap_pyfunction!(reactor::reactor, py).unwrap(),
            )
            .unwrap();
        locals
    }

    #[tokio::test]
    #[serial]
    async fn test_task_invokes_trigger_less_routes_terminal_into_context() {
        pyo3::prepare_freethreaded_python();

        let rt = Arc::new(cloacina::Runtime::empty());
        let _scope = ScopedRuntime::new(rt.clone()).unwrap();

        // Build a trigger-less graph and a task that invokes it.
        Python::with_gil(|py| {
            super::push_workflow_context(super::PyWorkflowContext::new(
                "public",
                "embedded",
                "m3_test_wf",
            ));
            let globals = py.import("builtins").unwrap().dict();
            let locals = build_locals(py);

            py.run(
                c_str!(
                    r#"
score_graph = ComputationGraphBuilder("score_graph", graph={"score": {}})
with score_graph:
    @node
    def score(ctx):
        return {"score": 99}
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();

            // Pull the live builder back out for the @task decorator below.
            let score_graph = locals.get_item("score_graph").unwrap().unwrap();

            let decorator = task(
                Some("scorer".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(score_graph.unbind()),
                None,
            )
            .unwrap();

            let user_fn = py
                .eval(c_str!("lambda ctx: ctx"), Some(&globals), Some(&locals))
                .unwrap();
            decorator.__call__(py, user_fn.into()).unwrap();
            super::pop_workflow_context();
        });

        let ns = cloacina::TaskNamespace::new("public", "embedded", "m3_test_wf", "scorer");
        let task = rt.get_task(&ns).expect("scorer task registered");

        let ctx = cloacina::Context::new();
        let out_ctx = task.execute(ctx).await.unwrap();

        let score_value = out_ctx
            .get("score")
            .expect("terminal 'score' should be routed into the context");
        assert_eq!(score_value, &serde_json::json!({"score": 99}));
    }

    #[tokio::test]
    #[serial]
    async fn test_task_post_invocation_runs_after_graph() {
        pyo3::prepare_freethreaded_python();

        let rt = Arc::new(cloacina::Runtime::empty());
        let _scope = ScopedRuntime::new(rt.clone()).unwrap();

        Python::with_gil(|py| {
            super::push_workflow_context(super::PyWorkflowContext::new(
                "public",
                "embedded",
                "m3_post_wf",
            ));
            let globals = py.import("builtins").unwrap().dict();
            let locals = build_locals(py);

            py.run(
                c_str!(
                    r#"
g = ComputationGraphBuilder("post_graph", graph={"emit": {}})
with g:
    @node
    def emit(ctx):
        return {"emitted": True}

def post(ctx):
    ctx.set("post_ran", True)
    return ctx
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();

            let g = locals.get_item("g").unwrap().unwrap();
            let post = locals.get_item("post").unwrap().unwrap();

            let decorator = task(
                Some("post_task".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(g.unbind()),
                Some(post.unbind()),
            )
            .unwrap();

            let user_fn = py
                .eval(c_str!("lambda ctx: ctx"), Some(&globals), Some(&locals))
                .unwrap();
            decorator.__call__(py, user_fn.into()).unwrap();
            super::pop_workflow_context();
        });

        let ns = cloacina::TaskNamespace::new("public", "embedded", "m3_post_wf", "post_task");
        let task = rt.get_task(&ns).expect("post_task registered");
        let out_ctx = task.execute(cloacina::Context::new()).await.unwrap();

        assert_eq!(
            out_ctx.get("emit").expect("terminal routed"),
            &serde_json::json!({"emitted": true})
        );
        assert_eq!(
            out_ctx.get("post_ran").expect("post_invocation ran"),
            &serde_json::json!(true)
        );
    }

    #[test]
    #[serial]
    fn test_task_post_invocation_without_invokes_errors() {
        pyo3::prepare_freethreaded_python();
        let rt = Arc::new(cloacina::Runtime::empty());
        let _scope = ScopedRuntime::new(rt).unwrap();

        Python::with_gil(|py| {
            super::push_workflow_context(super::PyWorkflowContext::new(
                "public",
                "embedded",
                "m3_no_invokes",
            ));
            let post = py.eval(c_str!("lambda ctx: ctx"), None, None).unwrap();
            let decorator = task(
                Some("bad".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(post.unbind().into()),
            )
            .unwrap();

            let user_fn = py.eval(c_str!("lambda ctx: ctx"), None, None).unwrap();
            let err = decorator.__call__(py, user_fn.into()).unwrap_err();
            assert!(err.to_string().contains("post_invocation"));
            super::pop_workflow_context();
        });
    }

    #[test]
    #[serial]
    fn test_task_invokes_reactor_triggered_graph_rejected() {
        pyo3::prepare_freethreaded_python();
        let rt = Arc::new(cloacina::Runtime::empty());
        let _scope = ScopedRuntime::new(rt).unwrap();

        Python::with_gil(|py| {
            super::push_workflow_context(super::PyWorkflowContext::new(
                "public",
                "embedded",
                "m3_reactor_target",
            ));
            let globals = py.import("builtins").unwrap().dict();
            let locals = build_locals(py);

            py.run(
                c_str!(
                    r#"
@reactor(name="rx_for_invokes", accumulators=["a"], mode="when_any")
class RxForInvokes: pass

g = ComputationGraphBuilder("reactor_graph", reactor=RxForInvokes,
                            graph={"e": {"inputs": ["a"]}})
with g:
    @node
    def e(a):
        return {"x": 1}
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();

            let g = locals.get_item("g").unwrap().unwrap();
            let user_fn = py.eval(c_str!("lambda ctx: ctx"), None, None).unwrap();
            let decorator = task(
                Some("rx_invoker".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(g.unbind()),
                None,
            )
            .unwrap();
            let err = decorator.__call__(py, user_fn.into()).unwrap_err();
            assert!(err.to_string().contains("reactor-triggered"));
            super::pop_workflow_context();
        });
    }
}
