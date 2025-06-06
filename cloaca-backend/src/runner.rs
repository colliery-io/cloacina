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
use tokio::sync::{oneshot, mpsc};
use std::thread;

use crate::context::PyContext;

/// Message types for communication with the async runtime thread
enum RuntimeMessage {
    Execute {
        workflow_name: String,
        context: cloacina::Context<serde_json::Value>,
        response_tx: oneshot::Sender<Result<cloacina::executor::PipelineResult, cloacina::executor::PipelineError>>,
    },
    Shutdown,
}

/// Simplified runner for Python bindings that doesn't start background services
struct SimplifiedRunner {
    database: cloacina::Database,
    dal: Arc<cloacina::dal::DAL>,
}

/// Handle to the background async runtime thread
struct AsyncRuntimeHandle {
    tx: mpsc::UnboundedSender<RuntimeMessage>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl AsyncRuntimeHandle {
    /// Shutdown the runtime thread and wait for it to complete
    fn shutdown(&mut self) {
        // Send shutdown signal
        let _ = self.tx.send(RuntimeMessage::Shutdown);
        
        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AsyncRuntimeHandle {
    fn drop(&mut self) {
        // Ensure shutdown on drop
        self.shutdown();
    }
}

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
    runtime_handle: std::cell::RefCell<AsyncRuntimeHandle>,
}

#[pymethods]
impl PyDefaultRunner {
    /// Create a new DefaultRunner with database connection
    #[new]
    pub fn new(database_url: &str) -> PyResult<Self> {
        let database_url = database_url.to_string();
        
        // Create a channel for communicating with the async runtime thread
        let (tx, mut rx) = mpsc::unbounded_channel::<RuntimeMessage>();
        
        // Spawn a dedicated thread for the async runtime
        let thread_handle = thread::spawn(move || {
            // Initialize logging in this thread
            eprintln!("THREAD: Initializing logging in background thread");
            if std::env::var("RUST_LOG").is_ok() {
                eprintln!("THREAD: RUST_LOG found: {:?}", std::env::var("RUST_LOG"));
                // Try to initialize tracing in this thread
                let _ = tracing_subscriber::fmt()
                    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                    .try_init();
            } else {
                eprintln!("THREAD: No RUST_LOG environment variable found");
            }
            
            // Create the tokio runtime in the dedicated thread
            let rt = Runtime::new().expect("Failed to create tokio runtime");
            
            // Create the DefaultRunner within the async context
            let runner = rt.block_on(async {
                eprintln!("THREAD: Creating DefaultRunner and starting background services");
                let runner = cloacina::DefaultRunner::new(&database_url).await
                    .expect("Failed to create DefaultRunner");
                eprintln!("THREAD: DefaultRunner created, background services should be running");
                runner
            });
            
            let runner = Arc::new(runner);
            
            // Event loop for processing messages - spawn tasks instead of blocking
            rt.block_on(async {
                while let Some(message) = rx.recv().await {
                    match message {
                        RuntimeMessage::Execute { workflow_name, context, response_tx } => {
                            eprintln!("THREAD: Received execute request for workflow: {}", workflow_name);
                            eprintln!("THREAD: Spawning execution task");
                            
                            let runner_clone = runner.clone();
                            // Spawn the execution as a separate task to avoid blocking the message loop
                            tokio::spawn(async move {
                                eprintln!("TASK: About to call runner.execute()");
                                
                                // Execute the workflow in the async runtime
                                use cloacina::executor::PipelineExecutor;
                                let result = runner_clone.execute(&workflow_name, context).await;
                                
                                eprintln!("TASK: runner.execute() returned: {:?}", result.is_ok());
                                eprintln!("TASK: Sending response back to Python thread");
                                
                                // Send response back to the calling thread
                                let _ = response_tx.send(result);
                                eprintln!("TASK: Response sent successfully");
                            });
                        }
                        RuntimeMessage::Shutdown => {
                            eprintln!("THREAD: Received shutdown signal");
                            break;
                        }
                    }
                }
            });
            
            eprintln!("THREAD: Runtime thread shutting down");
        });
        
        Ok(PyDefaultRunner {
            runtime_handle: std::cell::RefCell::new(AsyncRuntimeHandle {
                tx,
                thread_handle: Some(thread_handle),
            }),
        })
    }

    /// Create a new DefaultRunner with custom configuration
    #[staticmethod]
    pub fn with_config(
        database_url: &str,
        config: &crate::context::PyDefaultRunnerConfig,
    ) -> PyResult<PyDefaultRunner> {
        let database_url = database_url.to_string();
        let rust_config = config.to_rust_config();
        
        // Create a channel for communicating with the async runtime thread
        let (tx, mut rx) = mpsc::unbounded_channel::<RuntimeMessage>();
        
        // Spawn a dedicated thread for the async runtime
        let thread_handle = thread::spawn(move || {
            // Initialize logging in this thread
            eprintln!("THREAD: Initializing logging in background thread");
            if std::env::var("RUST_LOG").is_ok() {
                eprintln!("THREAD: RUST_LOG found: {:?}", std::env::var("RUST_LOG"));
                // Try to initialize tracing in this thread
                let _ = tracing_subscriber::fmt()
                    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                    .try_init();
            } else {
                eprintln!("THREAD: No RUST_LOG environment variable found");
            }
            
            // Create the tokio runtime in the dedicated thread
            let rt = Runtime::new().expect("Failed to create tokio runtime");
            
            // Create the DefaultRunner within the async context
            let runner = rt.block_on(async {
                cloacina::DefaultRunner::with_config(&database_url, rust_config).await
                    .expect("Failed to create DefaultRunner")
            });
            
            let runner = Arc::new(runner);
            
            // Event loop for processing messages - spawn tasks instead of blocking
            rt.block_on(async {
                while let Some(message) = rx.recv().await {
                    match message {
                        RuntimeMessage::Execute { workflow_name, context, response_tx } => {
                            eprintln!("THREAD: Received execute request for workflow: {}", workflow_name);
                            eprintln!("THREAD: Spawning execution task");
                            
                            let runner_clone = runner.clone();
                            // Spawn the execution as a separate task to avoid blocking the message loop
                            tokio::spawn(async move {
                                eprintln!("TASK: About to call runner.execute()");
                                
                                // Execute the workflow in the async runtime
                                use cloacina::executor::PipelineExecutor;
                                let result = runner_clone.execute(&workflow_name, context).await;
                                
                                eprintln!("TASK: runner.execute() returned: {:?}", result.is_ok());
                                eprintln!("TASK: Sending response back to Python thread");
                                
                                // Send response back to the calling thread
                                let _ = response_tx.send(result);
                                eprintln!("TASK: Response sent successfully");
                            });
                        }
                        RuntimeMessage::Shutdown => {
                            eprintln!("THREAD: Received shutdown signal");
                            break;
                        }
                    }
                }
            });
            
            eprintln!("THREAD: Runtime thread shutting down");
        });
        
        Ok(PyDefaultRunner {
            runtime_handle: AsyncRuntimeHandle {
                tx,
                thread_handle: Some(thread_handle),
            },
        })
    }

    /// Execute a workflow by name with context
    pub fn execute(&self, workflow_name: &str, context: &PyContext, py: Python) -> PyResult<PyPipelineResult> {
        let rust_context = context.clone_inner();
        let workflow_name = workflow_name.to_string();

        eprintln!("THREADS: Python execute() called for workflow: {}", workflow_name);
        
        // Create a oneshot channel for the response
        let (response_tx, response_rx) = oneshot::channel();
        
        // Send the execute message to the async runtime thread
        let message = RuntimeMessage::Execute {
            workflow_name: workflow_name.clone(),
            context: rust_context,
            response_tx,
        };
        
        // Send message without holding the GIL
        let result = py.allow_threads(|| {
            eprintln!("THREADS: Sending execute message to runtime thread");
            self.runtime_handle.borrow().tx.send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;
            
            eprintln!("THREADS: Waiting for response from runtime thread");
            // Wait for the response
            let result = response_rx.blocking_recv()
                .map_err(|_| PyValueError::new_err("Failed to receive response from runtime thread"))?;
            
            eprintln!("THREADS: Received response from runtime thread");
            result.map_err(|e| PyValueError::new_err(format!("Workflow execution failed: {}", e)))
        })?;

        eprintln!("THREADS: Execution completed successfully");
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
    pub fn shutdown(&self, py: Python) -> PyResult<()> {
        eprintln!("THREADS: Starting shutdown process");
        
        // Release the GIL while waiting for the thread to complete
        py.allow_threads(|| {
            // Call shutdown on the runtime handle, which will send the message and wait for thread completion
            self.runtime_handle.borrow_mut().shutdown();
        });
        
        eprintln!("THREADS: Shutdown completed successfully");
        Ok(())
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }
}

impl PyPipelineResult {
    pub fn from_result(result: cloacina::executor::PipelineResult) -> Self {
        PyPipelineResult { inner: result }
    }
}