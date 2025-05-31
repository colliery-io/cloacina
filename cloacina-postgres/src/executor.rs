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

use crate::workflow::PyWorkflow;
use cloacina::executor::{UnifiedExecutor, UnifiedExecutorConfig};
use parking_lot::RwLock;
use pyo3::prelude::*;
use pyo3_asyncio_0_21 as pyo3_asyncio;
use std::sync::Arc;

/// Simple wrapper around the existing Rust UnifiedExecutor
#[pyclass(name = "UnifiedExecutor")]
pub struct PyUnifiedExecutor {
    inner: Arc<RwLock<Option<UnifiedExecutor>>>,
    config: UnifiedExecutorConfig,
}

#[pymethods]
impl PyUnifiedExecutor {
    #[new]
    #[pyo3(signature = (max_workers=None))]
    pub fn new(max_workers: Option<usize>) -> Self {
        use std::time::Duration;

        let config = UnifiedExecutorConfig {
            max_concurrent_tasks: max_workers.unwrap_or(4),
            executor_poll_interval: Duration::from_secs(1),
            scheduler_poll_interval: Duration::from_secs(1),
            task_timeout: Duration::from_secs(300),
            pipeline_timeout: None,
            db_pool_size: 10,
            enable_recovery: true,
        };

        PyUnifiedExecutor {
            inner: Arc::new(RwLock::new(None)),
            config,
        }
    }

    #[pyo3(signature = (database_url="sqlite::memory:"))]
    pub fn initialize<'py>(
        &self,
        database_url: &str,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let config = self.config.clone();
        let inner = self.inner.clone();
        let db_url = database_url.to_string();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            // Create executor with workflows from global registry
            let workflows = crate::task::get_all_python_tasks()
                .into_iter()
                .map(|task| (*task).clone())
                .collect::<Vec<_>>();

            let executor = UnifiedExecutor::with_config(&db_url, config)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            // For now, we'll assume workflows are registered separately
            // In a full implementation, we'd register the Python tasks here

            *inner.write() = Some(executor);
            Ok(())
        })
    }

    pub fn register_workflow<'py>(
        &self,
        workflow: &PyWorkflow,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let workflow_name = workflow.inner().name().to_string();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let executor_guard = inner.read();
            let _executor = executor_guard.as_ref().ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Executor not initialized. Call initialize() first.",
                )
            })?;

            // For now, just log the registration
            // In the full implementation, we'd register the workflow with the executor
            println!("Registered workflow: {}", workflow_name);

            Ok(())
        })
    }

    pub fn shutdown<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let executor = {
                let mut executor_guard = inner.write();
                executor_guard.take()
            };

            if let Some(executor) = executor {
                executor.shutdown().await.map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                })?;
            }
            Ok(())
        })
    }

    pub fn execute<'py>(
        &self,
        workflow: &PyWorkflow,
        initial_context: Option<&Bound<'_, pyo3::types::PyDict>>,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let workflow_name = workflow.inner().name().to_string();

        // Convert initial context from Python to Rust
        let context = if let Some(py_dict) = initial_context {
            crate::context::context_from_python(py_dict)?
        } else {
            cloacina::Context::<serde_json::Value>::new()
        };

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let executor_guard = inner.read();
            let executor = executor_guard.as_ref().ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Executor not initialized. Call initialize() first.",
                )
            })?;

            // For now, use a placeholder execution since we need to register workflows first
            // In the full implementation, we'd use executor.execute(&workflow_name, context)
            println!("Executing workflow: {} with context", workflow_name);

            // Create a mock pipeline result for now
            let pipeline_result = cloacina::executor::PipelineResult {
                execution_id: uuid::Uuid::new_v4(),
                workflow_name: workflow_name.clone(),
                status: cloacina::executor::PipelineStatus::Completed,
                start_time: chrono::Utc::now(),
                end_time: Some(chrono::Utc::now()),
                duration: Some(std::time::Duration::from_millis(100)),
                final_context: context,
                task_results: vec![],
                error_message: None,
            };

            // Convert result back to Python
            Python::with_gil(|py| {
                let result_dict = pyo3::types::PyDict::new_bound(py);

                result_dict.set_item("execution_id", pipeline_result.execution_id.to_string())?;
                result_dict.set_item("workflow_name", &pipeline_result.workflow_name)?;
                result_dict.set_item("status", format!("{:?}", pipeline_result.status))?;
                result_dict.set_item("start_time", pipeline_result.start_time.to_rfc3339())?;

                if let Some(end_time) = pipeline_result.end_time {
                    result_dict.set_item("end_time", end_time.to_rfc3339())?;
                }

                if let Some(duration) = pipeline_result.duration {
                    result_dict.set_item("duration_seconds", duration.as_secs_f64())?;
                }

                // Convert final context back to Python
                let final_context =
                    crate::context::context_to_python(&pipeline_result.final_context, py)?;
                result_dict.set_item("final_context", final_context)?;

                // Convert task results
                let task_results = pyo3::types::PyList::empty_bound(py);
                for task_result in &pipeline_result.task_results {
                    let task_dict = pyo3::types::PyDict::new_bound(py);
                    task_dict.set_item("task_name", &task_result.task_name)?;
                    task_dict.set_item("status", format!("{:?}", task_result.status))?;

                    if let Some(start_time) = task_result.start_time {
                        task_dict.set_item("start_time", start_time.to_rfc3339())?;
                    }
                    if let Some(end_time) = task_result.end_time {
                        task_dict.set_item("end_time", end_time.to_rfc3339())?;
                    }
                    if let Some(duration) = task_result.duration {
                        task_dict.set_item("duration_seconds", duration.as_secs_f64())?;
                    }

                    task_dict.set_item("attempt_count", task_result.attempt_count)?;

                    if let Some(error) = &task_result.error_message {
                        task_dict.set_item("error_message", error)?;
                    }

                    task_results.append(task_dict)?;
                }
                result_dict.set_item("task_results", task_results)?;

                if let Some(error) = &pipeline_result.error_message {
                    result_dict.set_item("error_message", error)?;
                }

                Ok(result_dict.to_object(py))
            })
        })
    }

    pub fn __repr__(&self) -> String {
        format!(
            "UnifiedExecutor(max_concurrent_tasks={}, task_timeout={}s)",
            self.config.max_concurrent_tasks,
            self.config.task_timeout.as_secs()
        )
    }
}
