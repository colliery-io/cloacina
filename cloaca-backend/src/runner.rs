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

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::context::PyContext;

/// Python wrapper for PipelineResult
#[pyclass(name = "PipelineResult")]
pub struct PyPipelineResult {
    inner: cloacina::executor::PipelineResult,
}

#[pymethods]
impl PyPipelineResult {
    /// Get the execution status
    #[getter]
    pub fn status(&self) -> String {
        format!("{:?}", self.inner.status)
    }

    /// Get execution start time as ISO string
    #[getter]
    pub fn start_time(&self) -> String {
        self.inner.start_time.to_rfc3339()
    }

    /// Get execution end time as ISO string
    #[getter]
    pub fn end_time(&self) -> Option<String> {
        self.inner.end_time.map(|t| t.to_rfc3339())
    }

    /// Get the final context
    #[getter]
    pub fn final_context(&self) -> PyContext {
        // Create a new context by cloning the data without the dependency loader
        let new_context = self.inner.final_context.clone_data();
        PyContext::from_rust_context(new_context)
    }

    /// Get error message if execution failed
    #[getter]
    pub fn error_message(&self) -> Option<&str> {
        self.inner.error_message.as_deref()
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!(
            "PipelineResult(status={}, error={})",
            self.status(),
            self.error_message().unwrap_or("None")
        )
    }
}

/// Python wrapper for DefaultRunner
#[pyclass(name = "DefaultRunner")]
pub struct PyDefaultRunner {
    inner: Arc<cloacina::DefaultRunner>,
    runtime: Arc<Runtime>,
}

#[pymethods]
impl PyDefaultRunner {
    /// Create a new DefaultRunner with database connection
    #[new]
    pub fn new(database_url: &str) -> PyResult<Self> {
        // Create a tokio runtime for async operations
        let runtime = Runtime::new().map_err(|e| {
            PyValueError::new_err(format!("Failed to create tokio runtime: {}", e))
        })?;

        // Use the runtime to create the DefaultRunner
        let runner = runtime.block_on(async {
            cloacina::DefaultRunner::new(database_url).await
        }).map_err(|e| {
            PyValueError::new_err(format!("Failed to create DefaultRunner: {}", e))
        })?;

        Ok(PyDefaultRunner {
            inner: Arc::new(runner),
            runtime: Arc::new(runtime),
        })
    }

    /// Create a new DefaultRunner with custom configuration
    #[staticmethod]
    pub fn with_config(
        database_url: &str,
        config: &crate::context::PyDefaultRunnerConfig,
    ) -> PyResult<PyDefaultRunner> {
        // Create a tokio runtime for async operations
        let runtime = Runtime::new().map_err(|e| {
            PyValueError::new_err(format!("Failed to create tokio runtime: {}", e))
        })?;

        // Convert Python config to Rust config
        let rust_config = config.to_rust_config();

        // Use the runtime to create the DefaultRunner
        let runner = runtime.block_on(async {
            cloacina::DefaultRunner::with_config(database_url, rust_config).await
        }).map_err(|e| {
            PyValueError::new_err(format!("Failed to create DefaultRunner: {}", e))
        })?;

        Ok(PyDefaultRunner {
            inner: Arc::new(runner),
            runtime: Arc::new(runtime),
        })
    }

    /// Execute a workflow by name with context
    pub fn execute(&self, workflow_name: &str, context: &PyContext) -> PyResult<PyPipelineResult> {
        // Clone the PyContext to get a Rust Context
        let rust_context = context.clone_inner();

        eprintln!("DEBUG: Python execute() called for workflow: {}", workflow_name);
        
        // Create a completely new runtime for this execution to avoid any conflicts
        let rt = Runtime::new().map_err(|e| {
            PyValueError::new_err(format!("Failed to create execution runtime: {}", e))
        })?;
        
        let inner = self.inner.clone();
        let workflow_name = workflow_name.to_string();
        
        eprintln!("DEBUG: About to block_on with fresh runtime");
        
        // Use a fresh runtime to execute - this avoids any nested runtime issues
        let result = rt.block_on(async move {
            eprintln!("DEBUG: Inside fresh runtime, calling PipelineExecutor::execute");
            use cloacina::executor::PipelineExecutor;
            let exec_result = inner.execute(&workflow_name, rust_context).await;
            eprintln!("DEBUG: PipelineExecutor::execute returned: {:?}", exec_result.is_ok());
            exec_result
        }).map_err(|e| {
            eprintln!("DEBUG: Execution failed with error: {}", e);
            PyValueError::new_err(format!("Workflow execution failed: {}", e))
        })?;

        eprintln!("DEBUG: Execution completed successfully");
        Ok(PyPipelineResult::from_result(result))
    }

    /// Start the runner (task scheduler and executor)
    pub fn start(&self) -> PyResult<()> {
        // Start the runner in background
        // For now, return an error indicating this limitation
        Err(PyValueError::new_err(
            "Runner startup requires async runtime support. \
             This will be implemented in a future update."
        ))
    }

    /// Stop the runner
    pub fn stop(&self) -> PyResult<()> {
        // Stop the runner
        // For now, return an error indicating this limitation
        Err(PyValueError::new_err(
            "Runner shutdown requires async runtime support. \
             This will be implemented in a future update."
        ))
    }
    
    /// Shutdown the runner and cleanup resources
    pub fn shutdown(&self) -> PyResult<()> {
        // Shutdown the runtime to prevent hanging
        // Note: This is a best-effort cleanup - once shutdown, the runner cannot be reused
        // For now, we'll just return Ok since the runtime will be dropped when the object is destroyed
        Ok(())
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        "DefaultRunner(async_runtime_required)".to_string()
    }
}

impl PyPipelineResult {
    pub fn from_result(result: cloacina::executor::PipelineResult) -> Self {
        PyPipelineResult { inner: result }
    }
}