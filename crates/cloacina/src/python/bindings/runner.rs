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

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use thiserror::Error;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

use crate::python::context::PyContext;

/// Timeout for waiting on runtime thread shutdown
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Errors that can occur during async runtime shutdown
#[derive(Debug, Error)]
pub enum ShutdownError {
    /// Failed to send shutdown signal to runtime thread
    #[error("Failed to send shutdown signal: channel closed")]
    ChannelClosed,

    /// Runtime thread panicked during shutdown
    #[error("Runtime thread panicked during shutdown")]
    ThreadPanic,

    /// Shutdown timed out waiting for thread to complete
    #[error("Shutdown timed out after {} seconds", .0)]
    Timeout(u64),
}

/// Message types for communication with the async runtime thread
enum RuntimeMessage {
    Execute {
        workflow_name: String,
        context: crate::Context<serde_json::Value>,
        response_tx: oneshot::Sender<
            Result<crate::executor::PipelineResult, crate::executor::PipelineError>,
        >,
    },
    RegisterCronWorkflow {
        workflow_name: String,
        cron_expression: String,
        timezone: String,
        response_tx: oneshot::Sender<Result<String, crate::executor::PipelineError>>,
    },
    ListCronSchedules {
        enabled_only: bool,
        limit: i64,
        offset: i64,
        response_tx: oneshot::Sender<
            Result<Vec<crate::models::schedule::Schedule>, crate::executor::PipelineError>,
        >,
    },
    SetCronScheduleEnabled {
        schedule_id: String,
        enabled: bool,
        response_tx: oneshot::Sender<Result<(), crate::executor::PipelineError>>,
    },
    DeleteCronSchedule {
        schedule_id: String,
        response_tx: oneshot::Sender<Result<(), crate::executor::PipelineError>>,
    },
    GetCronSchedule {
        schedule_id: String,
        response_tx: oneshot::Sender<
            Result<crate::models::schedule::Schedule, crate::executor::PipelineError>,
        >,
    },
    UpdateCronSchedule {
        schedule_id: String,
        cron_expression: String,
        timezone: String,
        response_tx: oneshot::Sender<Result<(), crate::executor::PipelineError>>,
    },
    GetCronExecutionHistory {
        schedule_id: String,
        limit: i64,
        offset: i64,
        response_tx: oneshot::Sender<
            Result<Vec<crate::models::schedule::ScheduleExecution>, crate::executor::PipelineError>,
        >,
    },
    GetCronExecutionStats {
        since: chrono::DateTime<chrono::Utc>,
        response_tx: oneshot::Sender<
            Result<crate::dal::ScheduleExecutionStats, crate::executor::PipelineError>,
        >,
    },
    // Trigger management messages
    ListTriggerSchedules {
        enabled_only: bool,
        limit: i64,
        offset: i64,
        response_tx: oneshot::Sender<
            Result<Vec<crate::models::schedule::Schedule>, crate::executor::PipelineError>,
        >,
    },
    GetTriggerSchedule {
        trigger_name: String,
        response_tx: oneshot::Sender<
            Result<Option<crate::models::schedule::Schedule>, crate::executor::PipelineError>,
        >,
    },
    SetTriggerEnabled {
        trigger_name: String,
        enabled: bool,
        response_tx: oneshot::Sender<Result<(), crate::executor::PipelineError>>,
    },
    GetTriggerExecutionHistory {
        trigger_name: String,
        limit: i64,
        offset: i64,
        response_tx: oneshot::Sender<
            Result<Vec<crate::models::schedule::ScheduleExecution>, crate::executor::PipelineError>,
        >,
    },
    Shutdown,
}

/// Handle to the background async runtime thread
struct AsyncRuntimeHandle {
    tx: mpsc::UnboundedSender<RuntimeMessage>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl AsyncRuntimeHandle {
    /// Shutdown the runtime thread and wait for it to complete
    ///
    /// This method sends a shutdown signal to the runtime thread and waits
    /// for it to complete with a timeout. Errors are logged and returned.
    fn shutdown(&mut self) -> Result<(), ShutdownError> {
        let start = std::time::Instant::now();
        debug!("Initiating async runtime shutdown");

        // Send shutdown signal
        if let Err(e) = self.tx.send(RuntimeMessage::Shutdown) {
            error!("Failed to send shutdown signal to runtime thread: {:?}", e);
            // Continue anyway - thread might already be dead
        }

        // Wait for thread to finish with timeout
        if let Some(handle) = self.thread_handle.take() {
            // Use a channel to implement timeout on join
            let (done_tx, done_rx) = std::sync::mpsc::channel();

            // Spawn a helper thread to do the blocking join
            let join_thread = thread::spawn(move || {
                let result = handle.join();
                let _ = done_tx.send(result);
            });

            // Wait for completion with timeout
            match done_rx.recv_timeout(SHUTDOWN_TIMEOUT) {
                Ok(Ok(())) => {
                    debug!(
                        duration_ms = start.elapsed().as_millis() as u64,
                        "Async runtime shutdown completed successfully"
                    );
                    Ok(())
                }
                Ok(Err(_panic_payload)) => {
                    error!("Async runtime thread panicked during shutdown");
                    Err(ShutdownError::ThreadPanic)
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    error!(
                        timeout_secs = SHUTDOWN_TIMEOUT.as_secs(),
                        "Async runtime shutdown timed out - thread may be stuck"
                    );
                    // Detach the join thread - we can't wait forever
                    drop(join_thread);
                    Err(ShutdownError::Timeout(SHUTDOWN_TIMEOUT.as_secs()))
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Join thread finished but channel was dropped - treat as panic
                    error!("Join thread disconnected unexpectedly");
                    Err(ShutdownError::ThreadPanic)
                }
            }
        } else {
            debug!("Async runtime already shut down");
            Ok(())
        }
    }
}

impl Drop for AsyncRuntimeHandle {
    fn drop(&mut self) {
        // Ensure shutdown on drop, logging any errors
        if let Err(e) = self.shutdown() {
            warn!("Error during AsyncRuntimeHandle drop: {}", e);
        }
    }
}

/// Python wrapper for PipelineResult
#[pyclass(name = "PipelineResult")]
pub struct PyPipelineResult {
    inner: crate::executor::PipelineResult,
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
    runtime_handle: Mutex<AsyncRuntimeHandle>,
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
            eprintln!("THREAD: Initializing async runtime thread for DefaultRunner");
            eprintln!("THREAD: Checking RUST_LOG: {:?}", std::env::var("RUST_LOG"));

            // Try to initialize tracing
            use tracing::{debug, info};
            let _guard = tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                        eprintln!("THREAD: No RUST_LOG found, using info level");
                        tracing_subscriber::EnvFilter::new("info")
                    }),
                )
                .try_init();

            eprintln!("THREAD: Tracing initialized");
            info!("Background thread started with tracing");

            // Create the tokio runtime in the dedicated thread
            eprintln!("THREAD: Creating tokio runtime");
            debug!("Creating tokio runtime");
            let rt = Runtime::new().expect("Failed to create tokio runtime");
            eprintln!("THREAD: Tokio runtime created successfully");
            info!("Tokio runtime created successfully");

            // Create the DefaultRunner within the async context
            let runner = rt.block_on(async {
                eprintln!(
                    "THREAD: Creating DefaultRunner with database_url: {}",
                    database_url
                );
                info!("Creating DefaultRunner with database_url: {}", database_url);
                eprintln!("THREAD: About to call crate::DefaultRunner::new()");
                debug!("About to call crate::DefaultRunner::new()");
                let runner = crate::DefaultRunner::new(&database_url)
                    .await
                    .expect("Failed to create DefaultRunner");
                eprintln!(
                    "THREAD: DefaultRunner created successfully, background services running"
                );
                info!("DefaultRunner created successfully, background services running");
                runner
            });
            eprintln!("THREAD: DefaultRunner creation completed");
            info!("DefaultRunner creation completed");

            let runner = Arc::new(runner);

            // Event loop for processing messages - spawn tasks instead of blocking
            rt.block_on(async {
                while let Some(message) = rx.recv().await {
                    match message {
                        RuntimeMessage::Execute {
                            workflow_name,
                            context,
                            response_tx,
                        } => {
                            eprintln!(
                                "THREAD: Received execute request for workflow: {}",
                                workflow_name
                            );
                            eprintln!("THREAD: Spawning execution task");

                            let runner_clone = runner.clone();
                            // Spawn the execution as a separate task to avoid blocking the message loop
                            tokio::spawn(async move {
                                eprintln!("TASK: About to call runner.execute()");

                                // Execute the workflow in the async runtime
                                use crate::executor::PipelineExecutor;
                                let result = runner_clone.execute(&workflow_name, context).await;

                                eprintln!("TASK: runner.execute() returned: {:?}", result.is_ok());
                                eprintln!("TASK: Sending response back to Python thread");

                                // Send response back to the calling thread
                                let _ = response_tx.send(result);
                                eprintln!("TASK: Response sent successfully");
                            });
                        }
                        RuntimeMessage::RegisterCronWorkflow {
                            workflow_name,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            eprintln!(
                                "THREAD: Received register cron workflow request for: {}",
                                workflow_name
                            );

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .register_cron_workflow(
                                        &workflow_name,
                                        &cron_expression,
                                        &timezone,
                                    )
                                    .await
                                    .map(|uuid| uuid.to_string());

                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::ListCronSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received list cron schedules request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .list_cron_schedules(enabled_only, limit, offset)
                                    .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::SetCronScheduleEnabled {
                            schedule_id,
                            enabled,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received set cron schedule enabled request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .set_cron_schedule_enabled(universal_uuid, enabled)
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::DeleteCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received delete cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.delete_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.get_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::UpdateCronSchedule {
                            schedule_id,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received update cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .update_cron_schedule(
                                                universal_uuid,
                                                Some(&cron_expression),
                                                Some(&timezone),
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionHistory {
                            schedule_id,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get cron execution history request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .get_cron_execution_history(
                                                universal_uuid,
                                                limit,
                                                offset,
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionStats { since, response_tx } => {
                            eprintln!("THREAD: Received get cron execution stats request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone.get_cron_execution_stats(since).await;
                                let _ = response_tx.send(result);
                            });
                        }
                        // Trigger management messages
                        RuntimeMessage::ListTriggerSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received list trigger schedules request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = if enabled_only {
                                    dal.schedule().get_enabled_triggers().await
                                } else {
                                    dal.schedule()
                                        .list(Some("trigger"), false, limit, offset)
                                        .await
                                };
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerSchedule {
                            trigger_name,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get trigger schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result =
                                    dal.schedule().get_by_trigger_name(&trigger_name).await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::SetTriggerEnabled {
                            trigger_name,
                            enabled,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received set trigger enabled request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = async {
                                    if let Some(scheduler) = runner_clone.unified_scheduler().await
                                    {
                                        if enabled {
                                            scheduler.enable_trigger(&trigger_name).await
                                        } else {
                                            scheduler.disable_trigger(&trigger_name).await
                                        }
                                    } else {
                                        // No unified scheduler, use DAL directly
                                        let dal = runner_clone.dal();
                                        if let Some(schedule) = dal
                                            .schedule()
                                            .get_by_trigger_name(&trigger_name)
                                            .await?
                                        {
                                            if enabled {
                                                dal.schedule().enable(schedule.id).await
                                            } else {
                                                dal.schedule().disable(schedule.id).await
                                            }
                                        } else {
                                            Ok(())
                                        }
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerExecutionHistory {
                            trigger_name,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get trigger execution history request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = async {
                                    // Look up schedule by trigger name, then list executions
                                    let schedule_opt = dal
                                        .schedule()
                                        .get_by_trigger_name(&trigger_name)
                                        .await
                                        .map_err(|e| {
                                            crate::executor::PipelineError::Configuration {
                                                message: e.to_string(),
                                            }
                                        })?;
                                    if let Some(schedule) = schedule_opt {
                                        dal.schedule_execution()
                                            .list_by_schedule(schedule.id, limit, offset)
                                            .await
                                            .map_err(|e| {
                                                crate::executor::PipelineError::Configuration {
                                                    message: e.to_string(),
                                                }
                                            })
                                    } else {
                                        Ok(vec![])
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result);
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
            runtime_handle: Mutex::new(AsyncRuntimeHandle {
                tx,
                thread_handle: Some(thread_handle),
            }),
        })
    }

    /// Create a new DefaultRunner with custom configuration
    #[staticmethod]
    pub fn with_config(
        database_url: &str,
        config: &super::context::PyDefaultRunnerConfig,
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
                crate::DefaultRunner::with_config(&database_url, rust_config)
                    .await
                    .expect("Failed to create DefaultRunner")
            });

            let runner = Arc::new(runner);

            // Event loop for processing messages - spawn tasks instead of blocking
            rt.block_on(async {
                while let Some(message) = rx.recv().await {
                    match message {
                        RuntimeMessage::Execute {
                            workflow_name,
                            context,
                            response_tx,
                        } => {
                            eprintln!(
                                "THREAD: Received execute request for workflow: {}",
                                workflow_name
                            );
                            eprintln!("THREAD: Spawning execution task");

                            let runner_clone = runner.clone();
                            // Spawn the execution as a separate task to avoid blocking the message loop
                            tokio::spawn(async move {
                                eprintln!("TASK: About to call runner.execute()");

                                // Execute the workflow in the async runtime
                                use crate::executor::PipelineExecutor;
                                let result = runner_clone.execute(&workflow_name, context).await;

                                eprintln!("TASK: runner.execute() returned: {:?}", result.is_ok());
                                eprintln!("TASK: Sending response back to Python thread");

                                // Send response back to the calling thread
                                let _ = response_tx.send(result);
                                eprintln!("TASK: Response sent successfully");
                            });
                        }
                        RuntimeMessage::RegisterCronWorkflow {
                            workflow_name,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            eprintln!(
                                "THREAD: Received register cron workflow request for: {}",
                                workflow_name
                            );

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .register_cron_workflow(
                                        &workflow_name,
                                        &cron_expression,
                                        &timezone,
                                    )
                                    .await
                                    .map(|uuid| uuid.to_string());

                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::ListCronSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received list cron schedules request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .list_cron_schedules(enabled_only, limit, offset)
                                    .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::SetCronScheduleEnabled {
                            schedule_id,
                            enabled,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received set cron schedule enabled request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .set_cron_schedule_enabled(universal_uuid, enabled)
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::DeleteCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received delete cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.delete_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.get_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::UpdateCronSchedule {
                            schedule_id,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received update cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .update_cron_schedule(
                                                universal_uuid,
                                                Some(&cron_expression),
                                                Some(&timezone),
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionHistory {
                            schedule_id,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get cron execution history request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .get_cron_execution_history(
                                                universal_uuid,
                                                limit,
                                                offset,
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionStats { since, response_tx } => {
                            eprintln!("THREAD: Received get cron execution stats request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone.get_cron_execution_stats(since).await;
                                let _ = response_tx.send(result);
                            });
                        }
                        // Trigger management messages
                        RuntimeMessage::ListTriggerSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received list trigger schedules request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = if enabled_only {
                                    dal.schedule().get_enabled_triggers().await
                                } else {
                                    dal.schedule()
                                        .list(Some("trigger"), false, limit, offset)
                                        .await
                                };
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerSchedule {
                            trigger_name,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get trigger schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result =
                                    dal.schedule().get_by_trigger_name(&trigger_name).await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::SetTriggerEnabled {
                            trigger_name,
                            enabled,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received set trigger enabled request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = async {
                                    if let Some(scheduler) = runner_clone.unified_scheduler().await
                                    {
                                        if enabled {
                                            scheduler.enable_trigger(&trigger_name).await
                                        } else {
                                            scheduler.disable_trigger(&trigger_name).await
                                        }
                                    } else {
                                        // No unified scheduler, use DAL directly
                                        let dal = runner_clone.dal();
                                        if let Some(schedule) = dal
                                            .schedule()
                                            .get_by_trigger_name(&trigger_name)
                                            .await?
                                        {
                                            if enabled {
                                                dal.schedule().enable(schedule.id).await
                                            } else {
                                                dal.schedule().disable(schedule.id).await
                                            }
                                        } else {
                                            Ok(())
                                        }
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerExecutionHistory {
                            trigger_name,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get trigger execution history request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = async {
                                    // Look up schedule by trigger name, then list executions
                                    let schedule_opt = dal
                                        .schedule()
                                        .get_by_trigger_name(&trigger_name)
                                        .await
                                        .map_err(|e| {
                                            crate::executor::PipelineError::Configuration {
                                                message: e.to_string(),
                                            }
                                        })?;
                                    if let Some(schedule) = schedule_opt {
                                        dal.schedule_execution()
                                            .list_by_schedule(schedule.id, limit, offset)
                                            .await
                                            .map_err(|e| {
                                                crate::executor::PipelineError::Configuration {
                                                    message: e.to_string(),
                                                }
                                            })
                                    } else {
                                        Ok(vec![])
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result);
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
            runtime_handle: Mutex::new(AsyncRuntimeHandle {
                tx,
                thread_handle: Some(thread_handle),
            }),
        })
    }

    /// Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy
    ///
    /// This method enables multi-tenant deployments by using PostgreSQL schemas
    /// for complete data isolation between tenants. Each tenant gets their own
    /// schema with independent tables, migrations, and data.
    ///
    /// Note: This method requires a PostgreSQL database. SQLite does not support
    /// database schemas.
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    /// * `schema` - Schema name for tenant isolation (alphanumeric + underscores only)
    ///
    /// # Returns
    /// A new DefaultRunner instance configured for the specified tenant schema
    ///
    /// # Example
    /// ```python
    /// # Create tenant-specific runners
    /// tenant_a = DefaultRunner.with_schema(
    ///     "postgresql://user:pass@localhost/db",
    ///     "tenant_acme"
    /// )
    /// tenant_b = DefaultRunner.with_schema(
    ///     "postgresql://user:pass@localhost/db",
    ///     "tenant_globex"
    /// )
    /// ```
    #[staticmethod]
    pub fn with_schema(database_url: &str, schema: &str) -> PyResult<PyDefaultRunner> {
        // Runtime check for PostgreSQL - schema-based multi-tenancy requires PostgreSQL
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(PyValueError::new_err(
                "Schema-based multi-tenancy requires PostgreSQL. \
                 SQLite does not support database schemas. \
                 Use a PostgreSQL URL like 'postgres://user:pass@host/db'",
            ));
        }

        info!("Creating DefaultRunner with PostgreSQL schema: {}", schema);

        // Validate schema name format
        if schema.is_empty() {
            return Err(PyValueError::new_err("Schema name cannot be empty"));
        }

        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(PyValueError::new_err(
                "Schema name must contain only alphanumeric characters and underscores",
            ));
        }

        let database_url = database_url.to_string();
        let schema = schema.to_string();

        // Create channel for communication with the async thread
        let (tx, mut rx) = mpsc::unbounded_channel::<RuntimeMessage>();

        // Try to create the DefaultRunner first to catch errors early
        let database_url_clone = database_url.clone();
        let schema_clone = schema.clone();

        // Test the connection and schema creation in a temporary runtime
        let rt = Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create Tokio runtime: {}", e)))?;
        let _runner = rt
            .block_on(async {
                crate::DefaultRunner::with_schema(&database_url_clone, &schema_clone).await
            })
            .map_err(|e| {
                PyValueError::new_err(format!("Failed to create DefaultRunner with schema: {}", e))
            })?;

        // If we got here, the creation succeeded, so spawn the background thread
        let thread_handle = thread::spawn(move || {
            eprintln!(
                "THREAD: Starting async runtime thread for schema: {}",
                schema
            );
            info!("Starting async runtime thread for schema: {}", schema);

            // Create a new Tokio runtime
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            eprintln!(
                "THREAD: Tokio runtime created successfully for schema: {}",
                schema
            );
            info!("Tokio runtime created successfully for schema: {}", schema);

            // Create the DefaultRunner with schema within the async context
            let runner = rt.block_on(async {
                eprintln!("THREAD: Creating DefaultRunner with schema: {} and database_url: {}", schema, database_url);
                info!("Creating DefaultRunner with schema: {} and database_url: {}", schema, database_url);
                eprintln!("THREAD: About to call crate::DefaultRunner::with_schema()");
                debug!("About to call crate::DefaultRunner::with_schema()");
                let runner = crate::DefaultRunner::with_schema(&database_url, &schema).await
                    .expect("Failed to create DefaultRunner with schema - this should not fail since we tested it above");
                eprintln!("THREAD: DefaultRunner with schema created successfully, background services running");
                info!("DefaultRunner with schema created successfully, background services running");
                runner
            });
            eprintln!("THREAD: DefaultRunner with schema creation completed");
            info!("DefaultRunner with schema creation completed");

            let runner = Arc::new(runner);

            // Event loop for processing messages - identical to standard runner
            rt.block_on(async {
                while let Some(message) = rx.recv().await {
                    match message {
                        RuntimeMessage::Execute {
                            workflow_name,
                            context,
                            response_tx,
                        } => {
                            eprintln!(
                                "THREAD: Received execute request for workflow: {}",
                                workflow_name
                            );
                            eprintln!("THREAD: Spawning execution task");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                eprintln!("TASK: About to call runner.execute()");

                                use crate::executor::PipelineExecutor;
                                let result = runner_clone.execute(&workflow_name, context).await;

                                eprintln!("TASK: runner.execute() returned: {:?}", result.is_ok());
                                eprintln!("TASK: Sending response back to Python thread");

                                let _ = response_tx.send(result);
                                eprintln!("TASK: Response sent successfully");
                            });
                        }
                        RuntimeMessage::RegisterCronWorkflow {
                            workflow_name,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            eprintln!(
                                "THREAD: Received register cron workflow request for: {}",
                                workflow_name
                            );

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .register_cron_workflow(
                                        &workflow_name,
                                        &cron_expression,
                                        &timezone,
                                    )
                                    .await
                                    .map(|uuid| uuid.to_string());

                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::ListCronSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received list cron schedules request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .list_cron_schedules(enabled_only, limit, offset)
                                    .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::SetCronScheduleEnabled {
                            schedule_id,
                            enabled,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received set cron schedule enabled request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .set_cron_schedule_enabled(universal_uuid, enabled)
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::DeleteCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received delete cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.delete_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.get_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::UpdateCronSchedule {
                            schedule_id,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received update cron schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .update_cron_schedule(
                                                universal_uuid,
                                                Some(&cron_expression),
                                                Some(&timezone),
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionHistory {
                            schedule_id,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get cron execution history request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .get_cron_execution_history(
                                                universal_uuid,
                                                limit,
                                                offset,
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::PipelineError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionStats { since, response_tx } => {
                            eprintln!("THREAD: Received get cron execution stats request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone.get_cron_execution_stats(since).await;
                                let _ = response_tx.send(result);
                            });
                        }
                        // Trigger management messages
                        RuntimeMessage::ListTriggerSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received list trigger schedules request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = if enabled_only {
                                    dal.schedule().get_enabled_triggers().await
                                } else {
                                    dal.schedule()
                                        .list(Some("trigger"), false, limit, offset)
                                        .await
                                };
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerSchedule {
                            trigger_name,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get trigger schedule request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result =
                                    dal.schedule().get_by_trigger_name(&trigger_name).await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::SetTriggerEnabled {
                            trigger_name,
                            enabled,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received set trigger enabled request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = async {
                                    if let Some(scheduler) = runner_clone.unified_scheduler().await
                                    {
                                        if enabled {
                                            scheduler.enable_trigger(&trigger_name).await
                                        } else {
                                            scheduler.disable_trigger(&trigger_name).await
                                        }
                                    } else {
                                        // No unified scheduler, use DAL directly
                                        let dal = runner_clone.dal();
                                        if let Some(schedule) = dal
                                            .schedule()
                                            .get_by_trigger_name(&trigger_name)
                                            .await?
                                        {
                                            if enabled {
                                                dal.schedule().enable(schedule.id).await
                                            } else {
                                                dal.schedule().disable(schedule.id).await
                                            }
                                        } else {
                                            Ok(())
                                        }
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::PipelineError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerExecutionHistory {
                            trigger_name,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            eprintln!("THREAD: Received get trigger execution history request");

                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = async {
                                    // Look up schedule by trigger name, then list executions
                                    let schedule_opt = dal
                                        .schedule()
                                        .get_by_trigger_name(&trigger_name)
                                        .await
                                        .map_err(|e| {
                                            crate::executor::PipelineError::Configuration {
                                                message: e.to_string(),
                                            }
                                        })?;
                                    if let Some(schedule) = schedule_opt {
                                        dal.schedule_execution()
                                            .list_by_schedule(schedule.id, limit, offset)
                                            .await
                                            .map_err(|e| {
                                                crate::executor::PipelineError::Configuration {
                                                    message: e.to_string(),
                                                }
                                            })
                                    } else {
                                        Ok(vec![])
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::Shutdown => {
                            eprintln!(
                                "THREAD: Received shutdown message, breaking from event loop"
                            );
                            info!("Received shutdown message, breaking from event loop");
                            break;
                        }
                    }
                }
            });

            eprintln!("THREAD: Event loop finished, thread ending");
            info!("Event loop finished, thread ending");
        });

        // Return the Python wrapper
        Ok(PyDefaultRunner {
            runtime_handle: Mutex::new(AsyncRuntimeHandle {
                tx,
                thread_handle: Some(thread_handle),
            }),
        })
    }

    /// Execute a workflow by name with context
    pub fn execute(
        &self,
        workflow_name: &str,
        context: &PyContext,
        py: Python,
    ) -> PyResult<PyPipelineResult> {
        let rust_context = context.clone_inner();
        let workflow_name = workflow_name.to_string();

        eprintln!(
            "THREADS: Python execute() called for workflow: {}",
            workflow_name
        );

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
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            eprintln!("THREADS: Waiting for response from runtime thread");
            // Wait for the response
            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

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
             This will be implemented in a future update.",
        ))
    }

    /// Stop the runner
    pub fn stop(&self) -> PyResult<()> {
        // Stop the runner
        // For now, return an error indicating this limitation
        Err(PyValueError::new_err(
            "Runner shutdown requires async runtime support. \
             This will be implemented in a future update.",
        ))
    }

    /// Shutdown the runner and cleanup resources
    ///
    /// This method sends a shutdown signal to the async runtime thread and waits
    /// for it to complete (with a 5-second timeout). Errors during shutdown are
    /// raised as Python exceptions.
    ///
    /// # Raises
    /// * `ValueError` - If shutdown fails (timeout, thread panic, or channel error)
    pub fn shutdown(&self, py: Python) -> PyResult<()> {
        info!("Starting async runtime shutdown from Python");

        // Release the GIL while waiting for the thread to complete
        let result = py.allow_threads(|| self.runtime_handle.lock().unwrap().shutdown());

        match result {
            Ok(()) => {
                info!("Async runtime shutdown completed successfully");
                Ok(())
            }
            Err(e) => {
                error!("Async runtime shutdown failed: {}", e);
                Err(PyValueError::new_err(format!(
                    "Failed to shutdown async runtime: {}",
                    e
                )))
            }
        }
    }

    /// Register a cron workflow for automatic execution at scheduled times
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to execute
    /// * `cron_expression` - Standard cron expression (e.g., "0 2 * * *" for daily at 2 AM)
    /// * `timezone` - Timezone for cron interpretation (e.g., "UTC", "America/New_York")
    ///
    /// # Returns
    /// * Schedule ID as a string
    ///
    /// # Examples
    /// ```python
    /// # Daily backup at 2 AM UTC
    /// schedule_id = runner.register_cron_workflow("backup_workflow", "0 2 * * *", "UTC")
    ///
    /// # Business hours processing (9 AM - 5 PM, weekdays, Eastern Time)
    /// schedule_id = runner.register_cron_workflow("business_workflow", "0 9-17 * * 1-5", "America/New_York")
    /// ```
    pub fn register_cron_workflow(
        &self,
        workflow_name: String,
        cron_expression: String,
        timezone: String,
        py: Python,
    ) -> PyResult<String> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::RegisterCronWorkflow {
            workflow_name,
            cron_expression,
            timezone,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| {
                PyValueError::new_err(format!("Failed to register cron workflow: {}", e))
            })
        })
    }

    /// List all cron schedules
    ///
    /// # Arguments
    /// * `enabled_only` - If True, only return enabled schedules
    /// * `limit` - Maximum number of schedules to return (default: 100)
    /// * `offset` - Number of schedules to skip (default: 0)
    ///
    /// # Returns
    /// * List of dictionaries containing schedule information
    pub fn list_cron_schedules(
        &self,
        enabled_only: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let enabled_only = enabled_only.unwrap_or(false);
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::ListCronSchedules {
            enabled_only,
            limit,
            offset,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let schedules = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to list cron schedules: {}", e))
            })?;

            // Convert schedules to Python dictionaries
            let py_schedules: Result<Vec<PyObject>, PyErr> = schedules
                .into_iter()
                .map(|schedule| {
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        dict.set_item("id", schedule.id.to_string())?;
                        dict.set_item("workflow_name", &schedule.workflow_name)?;
                        dict.set_item(
                            "cron_expression",
                            schedule.cron_expression.as_deref().unwrap_or(""),
                        )?;
                        dict.set_item("timezone", schedule.timezone.as_deref().unwrap_or("UTC"))?;
                        dict.set_item("enabled", schedule.enabled.is_true())?;
                        dict.set_item(
                            "catchup_policy",
                            schedule.catchup_policy.as_deref().unwrap_or("skip"),
                        )?;
                        dict.set_item("next_run_at", schedule.next_run_at.map(|t| t.to_string()))?;
                        dict.set_item("last_run_at", schedule.last_run_at.map(|t| t.to_string()))?;
                        dict.set_item("created_at", schedule.created_at.to_string())?;
                        dict.set_item("updated_at", schedule.updated_at.to_string())?;
                        Ok(dict.into())
                    })
                })
                .collect();

            py_schedules
        })
    }

    /// Enable or disable a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - Schedule ID to modify
    /// * `enabled` - True to enable, False to disable
    pub fn set_cron_schedule_enabled(
        &self,
        schedule_id: String,
        enabled: bool,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::SetCronScheduleEnabled {
            schedule_id,
            enabled,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| {
                PyValueError::new_err(format!("Failed to update cron schedule: {}", e))
            })
        })
    }

    /// Delete a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - Schedule ID to delete
    pub fn delete_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::DeleteCronSchedule {
            schedule_id,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| {
                PyValueError::new_err(format!("Failed to delete cron schedule: {}", e))
            })
        })
    }

    /// Get details of a specific cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - Schedule ID to retrieve
    ///
    /// # Returns
    /// * Dictionary containing schedule information
    pub fn get_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<PyObject> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetCronSchedule {
            schedule_id,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let schedule = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get cron schedule: {}", e))
            })?;

            // Convert schedule to Python dictionary
            Python::with_gil(|py| {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("id", schedule.id.to_string())?;
                dict.set_item("workflow_name", &schedule.workflow_name)?;
                dict.set_item(
                    "cron_expression",
                    schedule.cron_expression.as_deref().unwrap_or(""),
                )?;
                dict.set_item("timezone", schedule.timezone.as_deref().unwrap_or("UTC"))?;
                dict.set_item("enabled", schedule.enabled.is_true())?;
                dict.set_item(
                    "catchup_policy",
                    schedule.catchup_policy.as_deref().unwrap_or("skip"),
                )?;
                dict.set_item("next_run_at", schedule.next_run_at.map(|t| t.to_string()))?;
                dict.set_item("last_run_at", schedule.last_run_at.map(|t| t.to_string()))?;
                dict.set_item("created_at", schedule.created_at.to_string())?;
                dict.set_item("updated_at", schedule.updated_at.to_string())?;
                Ok(dict.into())
            })
        })
    }

    /// Update a cron schedule's expression and timezone
    ///
    /// # Arguments
    /// * `schedule_id` - Schedule ID to update
    /// * `cron_expression` - New cron expression
    /// * `timezone` - New timezone
    pub fn update_cron_schedule(
        &self,
        schedule_id: String,
        cron_expression: String,
        timezone: String,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::UpdateCronSchedule {
            schedule_id,
            cron_expression,
            timezone,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| {
                PyValueError::new_err(format!("Failed to update cron schedule: {}", e))
            })
        })
    }

    /// Get execution history for a specific cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - Schedule ID to get history for
    /// * `limit` - Maximum number of executions to return (default: 100)
    /// * `offset` - Number of executions to skip (default: 0)
    ///
    /// # Returns
    /// * List of dictionaries containing execution information
    pub fn get_cron_execution_history(
        &self,
        schedule_id: String,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetCronExecutionHistory {
            schedule_id,
            limit,
            offset,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let executions = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get cron execution history: {}", e))
            })?;

            // Convert executions to Python dictionaries
            let py_executions: Result<Vec<PyObject>, PyErr> = executions
                .into_iter()
                .map(|execution| {
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        dict.set_item("id", execution.id.to_string())?;
                        dict.set_item("schedule_id", execution.schedule_id.to_string())?;
                        dict.set_item(
                            "scheduled_time",
                            execution.scheduled_time.map(|t| t.to_string()),
                        )?;
                        dict.set_item("claimed_at", execution.claimed_at.map(|t| t.to_string()))?;
                        dict.set_item(
                            "pipeline_execution_id",
                            execution.pipeline_execution_id.map(|id| id.to_string()),
                        )?;
                        dict.set_item("created_at", execution.created_at.to_string())?;
                        dict.set_item("updated_at", execution.updated_at.to_string())?;
                        Ok(dict.into())
                    })
                })
                .collect();

            py_executions
        })
    }

    /// Get execution statistics for cron schedules
    ///
    /// # Arguments
    /// * `since` - Start time for statistics collection (ISO 8601 string)
    ///
    /// # Returns
    /// * Dictionary containing execution statistics
    pub fn get_cron_execution_stats(&self, since: String, py: Python) -> PyResult<PyObject> {
        // Parse the since string as ISO 8601 datetime
        let since_dt = chrono::DateTime::parse_from_rfc3339(&since)
            .map_err(|e| PyValueError::new_err(format!("Invalid datetime format: {}", e)))?
            .with_timezone(&chrono::Utc);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetCronExecutionStats {
            since: since_dt,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let stats = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get cron execution stats: {}", e))
            })?;

            // Convert stats to Python dictionary
            Python::with_gil(|py| {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("total_executions", stats.total_executions)?;
                dict.set_item("successful_executions", stats.successful_executions)?;
                dict.set_item("lost_executions", stats.lost_executions)?;
                dict.set_item("success_rate", stats.success_rate)?;
                Ok(dict.into())
            })
        })
    }

    // ========================================================================
    // Trigger Management Methods
    // ========================================================================

    /// List all trigger schedules
    ///
    /// # Arguments
    /// * `enabled_only` - If True, only return enabled triggers
    /// * `limit` - Maximum number of triggers to return (default: 100)
    /// * `offset` - Number of triggers to skip (default: 0)
    ///
    /// # Returns
    /// * List of dictionaries containing trigger schedule information
    #[pyo3(signature = (enabled_only=None, limit=None, offset=None))]
    pub fn list_trigger_schedules(
        &self,
        enabled_only: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let enabled_only = enabled_only.unwrap_or(false);
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::ListTriggerSchedules {
            enabled_only,
            limit,
            offset,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let schedules = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to list trigger schedules: {}", e))
            })?;

            // Convert schedules to Python dictionaries
            let py_schedules: Result<Vec<PyObject>, PyErr> = schedules
                .into_iter()
                .map(|schedule| {
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        dict.set_item("id", schedule.id.to_string())?;
                        dict.set_item(
                            "trigger_name",
                            schedule.trigger_name.as_deref().unwrap_or(""),
                        )?;
                        dict.set_item("workflow_name", &schedule.workflow_name)?;
                        dict.set_item("poll_interval_ms", schedule.poll_interval_ms.unwrap_or(0))?;
                        dict.set_item("allow_concurrent", schedule.allows_concurrent())?;
                        dict.set_item("enabled", schedule.enabled.is_true())?;
                        dict.set_item(
                            "last_poll_at",
                            schedule.last_poll_at.map(|t| t.to_string()),
                        )?;
                        dict.set_item("created_at", schedule.created_at.to_string())?;
                        dict.set_item("updated_at", schedule.updated_at.to_string())?;
                        Ok(dict.into())
                    })
                })
                .collect();

            py_schedules
        })
    }

    /// Get details of a specific trigger schedule
    ///
    /// # Arguments
    /// * `trigger_name` - Name of the trigger to retrieve
    ///
    /// # Returns
    /// * Dictionary containing trigger schedule information, or None if not found
    pub fn get_trigger_schedule(
        &self,
        trigger_name: String,
        py: Python,
    ) -> PyResult<Option<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetTriggerSchedule {
            trigger_name,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let schedule_opt = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get trigger schedule: {}", e))
            })?;

            // Convert schedule to Python dictionary if present
            match schedule_opt {
                Some(schedule) => Python::with_gil(|py| {
                    let dict = pyo3::types::PyDict::new(py);
                    dict.set_item("id", schedule.id.to_string())?;
                    dict.set_item(
                        "trigger_name",
                        schedule.trigger_name.as_deref().unwrap_or(""),
                    )?;
                    dict.set_item("workflow_name", &schedule.workflow_name)?;
                    dict.set_item("poll_interval_ms", schedule.poll_interval_ms.unwrap_or(0))?;
                    dict.set_item("allow_concurrent", schedule.allows_concurrent())?;
                    dict.set_item("enabled", schedule.enabled.is_true())?;
                    dict.set_item("last_poll_at", schedule.last_poll_at.map(|t| t.to_string()))?;
                    dict.set_item("created_at", schedule.created_at.to_string())?;
                    dict.set_item("updated_at", schedule.updated_at.to_string())?;
                    Ok(Some(dict.into()))
                }),
                None => Ok(None),
            }
        })
    }

    /// Enable or disable a trigger
    ///
    /// # Arguments
    /// * `trigger_name` - Name of the trigger to modify
    /// * `enabled` - True to enable, False to disable
    pub fn set_trigger_enabled(
        &self,
        trigger_name: String,
        enabled: bool,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::SetTriggerEnabled {
            trigger_name,
            enabled,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| PyValueError::new_err(format!("Failed to update trigger: {}", e)))
        })
    }

    /// Get execution history for a specific trigger
    ///
    /// # Arguments
    /// * `trigger_name` - Name of the trigger to get history for
    /// * `limit` - Maximum number of executions to return (default: 100)
    /// * `offset` - Number of executions to skip (default: 0)
    ///
    /// # Returns
    /// * List of dictionaries containing execution information
    #[pyo3(signature = (trigger_name, limit=None, offset=None))]
    pub fn get_trigger_execution_history(
        &self,
        trigger_name: String,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetTriggerExecutionHistory {
            trigger_name,
            limit,
            offset,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let executions = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get trigger execution history: {}", e))
            })?;

            // Convert executions to Python dictionaries
            let py_executions: Result<Vec<PyObject>, PyErr> = executions
                .into_iter()
                .map(|execution| {
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        dict.set_item("id", execution.id.to_string())?;
                        dict.set_item("schedule_id", execution.schedule_id.to_string())?;
                        dict.set_item("context_hash", execution.context_hash.as_deref())?;
                        dict.set_item(
                            "pipeline_execution_id",
                            execution.pipeline_execution_id.map(|id| id.to_string()),
                        )?;
                        dict.set_item("started_at", execution.started_at.to_string())?;
                        dict.set_item(
                            "completed_at",
                            execution.completed_at.map(|t| t.to_string()),
                        )?;
                        dict.set_item("created_at", execution.created_at.to_string())?;
                        Ok(dict.into())
                    })
                })
                .collect();

            py_executions
        })
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }

    /// Context manager entry
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Context manager exit - automatically shutdown
    pub fn __exit__(
        &self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        self.shutdown(py)?;
        Ok(false) // Don't suppress exceptions
    }
}

impl PyPipelineResult {
    pub fn from_result(result: crate::executor::PipelineResult) -> Self {
        PyPipelineResult { inner: result }
    }
}

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use super::*;

    const TEST_PG_URL: &str = "postgres://cloacina:cloacina@localhost:5432/cloacina";

    fn unique_sqlite_url() -> String {
        format!(
            "sqlite:///tmp/cloacina_runner_test_{}.db?mode=rwc",
            uuid::Uuid::new_v4()
        )
    }

    #[test]
    fn test_runner_repr() {
        pyo3::prepare_freethreaded_python();
        // Create runner with SQLite (lighter weight, no Postgres needed)
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        assert_eq!(
            runner.__repr__(),
            "DefaultRunner(thread_separated_async_runtime)"
        );
    }

    #[test]
    fn test_runner_start_returns_not_implemented() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        let result = runner.start();
        assert!(result.is_err());
    }

    #[test]
    fn test_runner_stop_returns_not_implemented() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        let result = runner.stop();
        assert!(result.is_err());
    }

    #[test]
    fn test_runner_shutdown() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        Python::with_gil(|py| {
            runner.shutdown(py).expect("Shutdown should succeed");
        });
    }

    #[test]
    fn test_runner_context_manager() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let runner = Py::new(py, PyDefaultRunner::new(&unique_sqlite_url()).unwrap()).unwrap();
            // __enter__ returns self
            let entered = PyDefaultRunner::__enter__(runner.borrow(py));
            assert_eq!(
                entered.__repr__(),
                "DefaultRunner(thread_separated_async_runtime)"
            );
            // __exit__ shuts down
            let result = runner.borrow(py).__exit__(py, None, None, None);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_runner_list_cron_schedules_empty() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        Python::with_gil(|py| {
            let schedules = runner
                .list_cron_schedules(None, None, None, py)
                .expect("Should return empty list");
            assert!(schedules.is_empty());
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_list_trigger_schedules_empty() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        Python::with_gil(|py| {
            let schedules = runner
                .list_trigger_schedules(None, None, None, py)
                .expect("Should return empty list");
            assert!(schedules.is_empty());
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_get_trigger_schedule_not_found() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        Python::with_gil(|py| {
            let result = runner.get_trigger_schedule("nonexistent".to_string(), py);
            // Should return Ok(None) or Ok with empty result
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_register_cron_workflow() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            // Register a cron workflow — should return a schedule ID string
            let schedule_id = runner
                .register_cron_workflow(
                    "test-cron-wf".to_string(),
                    "0 * * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .expect("register_cron_workflow should succeed");
            assert!(!schedule_id.is_empty());
            // Should be a valid UUID
            assert!(uuid::Uuid::parse_str(&schedule_id).is_ok());

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_list_cron_schedules_after_register() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            // Register a workflow
            runner
                .register_cron_workflow(
                    "list-test-wf".to_string(),
                    "0 12 * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .unwrap();

            // List should now have one schedule
            let schedules = runner
                .list_cron_schedules(None, None, None, py)
                .expect("list should succeed");
            assert_eq!(schedules.len(), 1);

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_get_cron_schedule() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let schedule_id = runner
                .register_cron_workflow(
                    "get-test-wf".to_string(),
                    "30 2 * * *".to_string(),
                    "America/New_York".to_string(),
                    py,
                )
                .unwrap();

            let schedule_obj = runner
                .get_cron_schedule(schedule_id, py)
                .expect("get should succeed");
            // Should be a PyDict
            assert!(!schedule_obj.is_none(py));

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_set_cron_schedule_enabled() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let schedule_id = runner
                .register_cron_workflow(
                    "enable-test-wf".to_string(),
                    "0 0 * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .unwrap();

            // Disable
            runner
                .set_cron_schedule_enabled(schedule_id.clone(), false, py)
                .expect("disable should succeed");

            // Re-enable
            runner
                .set_cron_schedule_enabled(schedule_id, true, py)
                .expect("enable should succeed");

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_delete_cron_schedule() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let schedule_id = runner
                .register_cron_workflow(
                    "delete-test-wf".to_string(),
                    "0 0 * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .unwrap();

            runner
                .delete_cron_schedule(schedule_id, py)
                .expect("delete should succeed");

            // List should now be empty
            let schedules = runner.list_cron_schedules(None, None, None, py).unwrap();
            assert!(schedules.is_empty());

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_update_cron_schedule() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let schedule_id = runner
                .register_cron_workflow(
                    "update-test-wf".to_string(),
                    "0 0 * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .unwrap();

            runner
                .update_cron_schedule(
                    schedule_id,
                    "30 6 * * *".to_string(),
                    "America/Chicago".to_string(),
                    py,
                )
                .expect("update should succeed");

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_get_cron_execution_history_empty() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let schedule_id = runner
                .register_cron_workflow(
                    "history-test-wf".to_string(),
                    "0 0 * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .unwrap();

            let history = runner
                .get_cron_execution_history(schedule_id, None, None, py)
                .expect("history should succeed");
            assert!(history.is_empty());

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_get_cron_execution_stats() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let since = chrono::Utc::now() - chrono::Duration::try_hours(1).unwrap();
            let stats = runner
                .get_cron_execution_stats(since.to_rfc3339(), py)
                .expect("stats should succeed");
            assert!(!stats.is_none(py));

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_set_cron_schedule_enabled_invalid_id() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let result = runner.set_cron_schedule_enabled("not-a-uuid".to_string(), false, py);
            assert!(result.is_err());

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_set_trigger_enabled() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            // Enabling a nonexistent trigger — should error or succeed gracefully
            let result = runner.set_trigger_enabled("nonexistent".to_string(), true, py);
            // Either way, runner shouldn't crash
            let _ = result;

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_get_trigger_execution_history() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let history =
                runner.get_trigger_execution_history("nonexistent".to_string(), None, None, py);
            // Should return empty or error, not crash
            let _ = history;

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_pipeline_result_completed() {
        pyo3::prepare_freethreaded_python();
        let mut ctx = crate::Context::new();
        ctx.insert("result".to_string(), serde_json::json!("done"))
            .unwrap();
        let result = crate::executor::PipelineResult {
            execution_id: uuid::Uuid::new_v4(),
            workflow_name: "test-wf".to_string(),
            status: crate::executor::PipelineStatus::Completed,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            duration: Some(std::time::Duration::from_secs(1)),
            final_context: ctx,
            task_results: vec![],
            error_message: None,
        };
        let py_result = PyPipelineResult::from_result(result);

        assert_eq!(py_result.status(), "Completed");
        assert!(!py_result.start_time().is_empty());
        assert!(py_result.end_time().is_some());
        assert!(py_result.error_message().is_none());

        Python::with_gil(|_py| {
            let ctx = py_result.final_context();
            assert_eq!(ctx.inner.get("result"), Some(&serde_json::json!("done")));
        });

        let repr = py_result.__repr__();
        assert!(repr.contains("Completed"));
        assert!(repr.contains("None")); // no error
    }

    #[test]
    fn test_pipeline_result_failed() {
        pyo3::prepare_freethreaded_python();
        let result = crate::executor::PipelineResult {
            execution_id: uuid::Uuid::new_v4(),
            workflow_name: "fail-wf".to_string(),
            status: crate::executor::PipelineStatus::Failed,
            start_time: chrono::Utc::now(),
            end_time: None,
            duration: None,
            final_context: crate::Context::new(),
            task_results: vec![],
            error_message: Some("something broke".to_string()),
        };
        let py_result = PyPipelineResult::from_result(result);

        assert_eq!(py_result.status(), "Failed");
        assert!(py_result.end_time().is_none());
        assert_eq!(py_result.error_message(), Some("something broke"));
        assert!(py_result.__repr__().contains("something broke"));
    }

    #[test]
    fn test_runner_execute_nonexistent_workflow() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let ctx = crate::python::context::PyContext::new(None).unwrap();
            let result = runner.execute("nonexistent_workflow", &ctx, py);
            assert!(
                result.is_err(),
                "Execute of nonexistent workflow should error"
            );

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_execute_registered_workflow() {
        pyo3::prepare_freethreaded_python();

        // Register a simple workflow
        use async_trait::async_trait;
        use std::sync::Arc;

        #[derive(Clone)]
        struct NoOpTask;

        #[async_trait]
        impl crate::Task for NoOpTask {
            async fn execute(
                &self,
                context: crate::Context<serde_json::Value>,
            ) -> Result<crate::Context<serde_json::Value>, crate::TaskError> {
                Ok(context)
            }
            fn id(&self) -> &str {
                "noop"
            }
            fn dependencies(&self) -> &[crate::TaskNamespace] {
                &[]
            }
        }

        let workflow = crate::Workflow::builder("py_runner_exec_test")
            .add_task(Arc::new(NoOpTask))
            .unwrap()
            .build()
            .unwrap();

        crate::register_workflow_constructor("py_runner_exec_test".to_string(), {
            let w = workflow.clone();
            move || w.clone()
        });

        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let ctx = crate::python::context::PyContext::new(None).unwrap();
            let result = runner.execute("py_runner_exec_test", &ctx, py);
            assert!(result.is_ok(), "Execute should succeed: {:?}", result.err());

            let pipeline_result = result.unwrap();
            // Should have a status (may be Completed or Pending depending on timing)
            assert!(!pipeline_result.status().is_empty());

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_get_cron_execution_stats_invalid_date() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let result = runner.get_cron_execution_stats("not-a-date".to_string(), py);
            assert!(result.is_err(), "Invalid date should error");

            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    fn test_runner_list_cron_schedules_enabled_only() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            // Register a schedule and disable it
            let id = runner
                .register_cron_workflow(
                    "filter-test".to_string(),
                    "0 0 * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .unwrap();
            runner.set_cron_schedule_enabled(id, false, py).unwrap();

            // List with enabled_only=true should return empty
            let enabled = runner
                .list_cron_schedules(Some(true), None, None, py)
                .unwrap();
            assert!(
                enabled.is_empty(),
                "disabled schedule should be filtered out"
            );

            // List without filter should return one
            let all = runner
                .list_cron_schedules(Some(false), None, None, py)
                .unwrap();
            assert_eq!(all.len(), 1);

            runner.shutdown(py).unwrap();
        });
    }

    // ── with_schema validation tests ─────────────────────────────────

    #[test]
    fn test_with_schema_rejects_sqlite() {
        pyo3::prepare_freethreaded_python();
        let result = PyDefaultRunner::with_schema("sqlite:///tmp/test.db", "tenant_a");
        assert!(result.is_err());
    }

    #[test]
    fn test_with_schema_rejects_empty_schema() {
        pyo3::prepare_freethreaded_python();
        let result = PyDefaultRunner::with_schema(
            "postgres://cloacina:cloacina@localhost:5432/cloacina",
            "",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_with_schema_rejects_invalid_chars() {
        pyo3::prepare_freethreaded_python();
        let result = PyDefaultRunner::with_schema(
            "postgres://cloacina:cloacina@localhost:5432/cloacina",
            "tenant;DROP TABLE",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_shutdown_error_display() {
        let err = ShutdownError::ChannelClosed;
        assert!(format!("{}", err).contains("channel closed"));

        let err = ShutdownError::ThreadPanic;
        assert!(format!("{}", err).contains("panicked"));

        let err = ShutdownError::Timeout(5);
        assert!(format!("{}", err).contains("5"));
    }
}
