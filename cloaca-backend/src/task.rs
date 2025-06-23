/*
 *  Copyright 2025 Colliery Software
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

use crate::value_objects::PyWorkflowContext;
use async_trait::async_trait;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

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
        .unwrap()
        .push(WorkflowBuilderRef { context });
}

/// Pop a workflow context from the stack (called when exiting workflow scope)
pub fn pop_workflow_context() -> Option<WorkflowBuilderRef> {
    WORKFLOW_CONTEXT_STACK.lock().unwrap().pop()
}

/// Get the current workflow context (used by task decorator)
pub fn current_workflow_context() -> PyResult<PyWorkflowContext> {
    let stack = WORKFLOW_CONTEXT_STACK.lock().unwrap();
    stack.last().map(|ref_| ref_.context.clone()).ok_or_else(|| {
        PyValueError::new_err(
            "No workflow context available. Tasks must be defined within a WorkflowBuilder context manager."
        )
    })
}

/// Python task wrapper implementing Rust Task trait
///
/// This struct allows Python functions to be registered and executed
/// as tasks within the Cloacina execution engine.
pub struct PythonTaskWrapper {
    id: String,
    dependencies: Vec<cloacina::TaskNamespace>,
    retry_policy: cloacina::retry::RetryPolicy,
    python_function: PyObject, // Stored Python function
}

// Implement Send + Sync for PythonTaskWrapper
// PyObject is already Send + Sync
unsafe impl Send for PythonTaskWrapper {}
unsafe impl Sync for PythonTaskWrapper {}

#[async_trait]
impl cloacina::Task for PythonTaskWrapper {
    async fn execute(
        &self,
        context: cloacina::Context<serde_json::Value>,
    ) -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError> {
        use crate::context::PyContext;

        // Clone PyObject inside GIL context
        let function = Python::with_gil(|py| self.python_function.clone_ref(py));
        let task_id = self.id.clone();

        // Execute Python function in a blocking task to avoid blocking the async runtime
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // Get the original context data before moving context into PyContext
                let original_data = context.data().clone();

                // Create PyContext wrapper
                let py_context = PyContext::from_rust_context(context);

                // Call Python function
                let result = function.call1(py, (py_context,))?;

                // Handle return value
                if result.is_none(py) {
                    // None means success, create a new context from the original data
                    let mut new_context = cloacina::Context::new();
                    for (key, value) in original_data.iter() {
                        new_context.insert(key.clone(), value.clone()).unwrap();
                    }
                    Ok(new_context)
                } else {
                    // Extract returned context
                    let returned_context: PyContext = result.extract(py)?;
                    Ok(returned_context.into_inner())
                }
            })
        })
        .await
        .map_err(|e| cloacina::TaskError::ExecutionFailed {
            message: format!("Task execution panicked: {}", e),
            task_id: task_id.clone(),
            timestamp: chrono::Utc::now(),
        })?
        .map_err(|e: PyErr| cloacina::TaskError::ExecutionFailed {
            message: format!("Python task execution failed: {}", e),
            task_id: self.id.clone(),
            timestamp: chrono::Utc::now(),
        })
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

    // Default implementations for optional methods
    fn checkpoint(
        &self,
        _context: &cloacina::Context<serde_json::Value>,
    ) -> Result<(), cloacina::CheckpointError> {
        Ok(())
    }

    fn trigger_rules(&self) -> serde_json::Value {
        // Default to Always trigger rule (same as Rust macro default)
        serde_json::json!({"type": "Always"})
    }

    fn code_fingerprint(&self) -> Option<String> {
        // Could implement Python function hashing in the future
        None
    }
}

/// Build retry policy from Python decorator parameters
#[allow(dead_code)] // Used by task decorator function
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
    id: Option<String>,          // Now optional - can be derived from function name
    dependencies: Vec<PyObject>, // Now supports both strings and function objects
    retry_policy: cloacina::retry::RetryPolicy,
}

#[pymethods]
impl TaskDecorator {
    pub fn __call__(&self, py: Python, func: PyObject) -> PyResult<PyObject> {
        // Get the current workflow context from the global stack
        let context = current_workflow_context()?;

        // Determine task ID - use provided ID or derive from function name
        let task_id = if let Some(id) = &self.id {
            id.clone()
        } else {
            // Extract function name
            func.getattr(py, "__name__")?.extract::<String>(py)?
        };

        // Convert dependencies from mixed PyObject list to TaskNamespace list
        let deps = match self.convert_dependencies_to_namespaces(py, &context) {
            Ok(deps) => deps,
            Err(e) => {
                eprintln!("Error converting dependencies: {}", e);
                return Err(e);
            }
        };
        let policy = self.retry_policy.clone();
        let function = func.clone_ref(py);

        // Register task constructor in global registry using context
        let shared_function = Arc::new(function);
        let (tenant_id, package_name, workflow_id) = context.as_components();
        let namespace =
            cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &task_id);
        cloacina::register_task_constructor(namespace.clone(), {
            let task_id_clone = task_id.clone();
            let deps_clone = deps.clone();
            let policy_clone = policy.clone();
            let function_arc = shared_function.clone();
            move || {
                let function_clone = Python::with_gil(|py| function_arc.clone_ref(py));
                Arc::new(PythonTaskWrapper {
                    id: task_id_clone.clone(), // Keep simple task ID
                    dependencies: deps_clone.clone(),
                    retry_policy: policy_clone.clone(),
                    python_function: function_clone,
                }) as Arc<dyn cloacina::Task>
            }
        });

        // Store task ID for the current workflow to add later
        // For now, we'll let the workflow builder handle task collection differently

        // Return the original function (decorator behavior)
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
                // It's a string - use directly
                string_dep
            } else {
                // Try to get function name
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

            // Use workflow context to create proper namespace
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
///
/// This function is exposed to Python as a decorator that registers
/// Python functions as tasks in the Cloacina execution engine.
///
/// # Examples
///
/// **Workflow-scoped approach (automatic namespace inheritance):**
/// ```python
/// with cloaca.WorkflowBuilder("my_workflow", tenant="acme", package="data") as workflow:
///     @cloaca.task(
///         id="my_task",
///         dependencies=["other_task"],
///         retry_attempts=3,
///         retry_backoff="exponential"
///     )
///     def my_task(context):
///         context.set("result", "processed")
///         return context
///
///     @cloaca.task()  # ID automatically derived from function name
///     def extract_data(context):
///         return context
///
///     @cloaca.task(dependencies=[extract_data])  # Direct function reference
///     def process_data(context):
///         return context
/// ```
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
    retry_jitter = None
))]
#[allow(dead_code)] // Used by PyO3 module exports
pub fn task(
    id: Option<String>,
    dependencies: Option<Vec<PyObject>>,
    retry_attempts: Option<usize>,
    retry_backoff: Option<String>,
    retry_delay_ms: Option<u64>,
    retry_max_delay_ms: Option<u64>,
    retry_condition: Option<String>,
    retry_jitter: Option<bool>,
) -> PyResult<TaskDecorator> {
    // Build retry policy from parameters
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
    })
}
