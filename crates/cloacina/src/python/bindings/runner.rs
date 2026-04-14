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

use pyo3::exceptions::{PyRuntimeError, PyValueError};
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
            Result<
                crate::executor::WorkflowExecutionResult,
                crate::executor::WorkflowExecutionError,
            >,
        >,
    },
    RegisterCronWorkflow {
        workflow_name: String,
        cron_expression: String,
        timezone: String,
        response_tx: oneshot::Sender<Result<String, crate::executor::WorkflowExecutionError>>,
    },
    ListCronSchedules {
        enabled_only: bool,
        limit: i64,
        offset: i64,
        response_tx: oneshot::Sender<
            Result<Vec<crate::models::schedule::Schedule>, crate::executor::WorkflowExecutionError>,
        >,
    },
    SetCronScheduleEnabled {
        schedule_id: String,
        enabled: bool,
        response_tx: oneshot::Sender<Result<(), crate::executor::WorkflowExecutionError>>,
    },
    DeleteCronSchedule {
        schedule_id: String,
        response_tx: oneshot::Sender<Result<(), crate::executor::WorkflowExecutionError>>,
    },
    GetCronSchedule {
        schedule_id: String,
        response_tx: oneshot::Sender<
            Result<crate::models::schedule::Schedule, crate::executor::WorkflowExecutionError>,
        >,
    },
    UpdateCronSchedule {
        schedule_id: String,
        cron_expression: String,
        timezone: String,
        response_tx: oneshot::Sender<Result<(), crate::executor::WorkflowExecutionError>>,
    },
    GetCronExecutionHistory {
        schedule_id: String,
        limit: i64,
        offset: i64,
        response_tx: oneshot::Sender<
            Result<
                Vec<crate::models::schedule::ScheduleExecution>,
                crate::executor::WorkflowExecutionError,
            >,
        >,
    },
    GetCronExecutionStats {
        since: chrono::DateTime<chrono::Utc>,
        response_tx: oneshot::Sender<
            Result<crate::dal::ScheduleExecutionStats, crate::executor::WorkflowExecutionError>,
        >,
    },
    ListTriggerSchedules {
        enabled_only: bool,
        limit: i64,
        offset: i64,
        response_tx: oneshot::Sender<
            Result<Vec<crate::models::schedule::Schedule>, crate::executor::WorkflowExecutionError>,
        >,
    },
    GetTriggerSchedule {
        trigger_name: String,
        response_tx: oneshot::Sender<
            Result<
                Option<crate::models::schedule::Schedule>,
                crate::executor::WorkflowExecutionError,
            >,
        >,
    },
    SetTriggerEnabled {
        trigger_name: String,
        enabled: bool,
        response_tx: oneshot::Sender<Result<(), crate::executor::WorkflowExecutionError>>,
    },
    GetTriggerExecutionHistory {
        trigger_name: String,
        limit: i64,
        offset: i64,
        response_tx: oneshot::Sender<
            Result<
                Vec<crate::models::schedule::ScheduleExecution>,
                crate::executor::WorkflowExecutionError,
            >,
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
    fn shutdown(&mut self) -> Result<(), ShutdownError> {
        let start = std::time::Instant::now();
        debug!("Initiating async runtime shutdown");

        // Send shutdown signal
        if let Err(e) = self.tx.send(RuntimeMessage::Shutdown) {
            error!("Failed to send shutdown signal to runtime thread: {:?}", e);
        }

        // Wait for thread to finish with timeout
        if let Some(handle) = self.thread_handle.take() {
            let (done_tx, done_rx) = std::sync::mpsc::channel();

            let join_thread = thread::spawn(move || {
                let result = handle.join();
                let _ = done_tx.send(result);
            });

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
                    drop(join_thread);
                    Err(ShutdownError::Timeout(SHUTDOWN_TIMEOUT.as_secs()))
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
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
        if let Err(e) = self.shutdown() {
            warn!("Error during AsyncRuntimeHandle drop: {}", e);
        }
    }
}

/// Python wrapper for WorkflowExecutionResult
#[pyclass(name = "WorkflowResult")]
pub struct PyWorkflowResult {
    inner: crate::executor::WorkflowExecutionResult,
}

#[pymethods]
impl PyWorkflowResult {
    #[getter]
    pub fn status(&self) -> String {
        format!("{:?}", self.inner.status)
    }

    #[getter]
    pub fn start_time(&self) -> String {
        self.inner.start_time.to_rfc3339()
    }

    #[getter]
    pub fn end_time(&self) -> Option<String> {
        self.inner.end_time.map(|t| t.to_rfc3339())
    }

    #[getter]
    pub fn final_context(&self) -> PyContext {
        let new_context = self.inner.final_context.clone_data();
        PyContext::from_rust_context(new_context)
    }

    #[getter]
    pub fn error_message(&self) -> Option<&str> {
        self.inner.error_message.as_deref()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "WorkflowResult(status={}, error={})",
            self.status(),
            self.error_message().unwrap_or("None")
        )
    }
}

impl PyWorkflowResult {
    pub fn from_result(result: crate::executor::WorkflowExecutionResult) -> Self {
        PyWorkflowResult { inner: result }
    }
}

// ============================================================================
// Internal helpers — event loop, constructor, dict conversion
// ============================================================================

/// Parse a schedule ID string into a UniversalUuid.
fn parse_schedule_id(
    schedule_id: &str,
) -> Result<crate::database::universal_types::UniversalUuid, crate::executor::WorkflowExecutionError>
{
    schedule_id
        .parse::<uuid::Uuid>()
        .map(crate::UniversalUuid::from)
        .map_err(|e| crate::executor::WorkflowExecutionError::Configuration {
            message: format!("Invalid schedule ID: {}", e),
        })
}

/// Convert a cron Schedule to a Python dict.
fn schedule_to_cron_dict(
    schedule: crate::models::schedule::Schedule,
    py: Python,
) -> PyResult<PyObject> {
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
}

/// Convert a trigger Schedule to a Python dict.
fn schedule_to_trigger_dict(
    schedule: crate::models::schedule::Schedule,
    py: Python,
) -> PyResult<PyObject> {
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
    Ok(dict.into())
}

/// Convert a cron ScheduleExecution to a Python dict.
fn cron_execution_to_dict(
    execution: crate::models::schedule::ScheduleExecution,
    py: Python,
) -> PyResult<PyObject> {
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("id", execution.id.to_string())?;
    dict.set_item("schedule_id", execution.schedule_id.to_string())?;
    dict.set_item(
        "scheduled_time",
        execution.scheduled_time.map(|t| t.to_string()),
    )?;
    dict.set_item("claimed_at", execution.claimed_at.map(|t| t.to_string()))?;
    dict.set_item(
        "workflow_execution_id",
        execution.workflow_execution_id.map(|id| id.to_string()),
    )?;
    dict.set_item("created_at", execution.created_at.to_string())?;
    dict.set_item("updated_at", execution.updated_at.to_string())?;
    Ok(dict.into())
}

/// Convert a trigger ScheduleExecution to a Python dict.
fn trigger_execution_to_dict(
    execution: crate::models::schedule::ScheduleExecution,
    py: Python,
) -> PyResult<PyObject> {
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("id", execution.id.to_string())?;
    dict.set_item("schedule_id", execution.schedule_id.to_string())?;
    dict.set_item("context_hash", execution.context_hash.as_deref())?;
    dict.set_item(
        "workflow_execution_id",
        execution.workflow_execution_id.map(|id| id.to_string()),
    )?;
    dict.set_item("started_at", execution.started_at.to_string())?;
    dict.set_item(
        "completed_at",
        execution.completed_at.map(|t| t.to_string()),
    )?;
    dict.set_item("created_at", execution.created_at.to_string())?;
    Ok(dict.into())
}

/// The single event loop that dispatches RuntimeMessages to the DefaultRunner.
///
/// Previously this ~300-line match block was copy-pasted into each constructor.
async fn run_event_loop(
    runner: Arc<crate::DefaultRunner>,
    mut rx: mpsc::UnboundedReceiver<RuntimeMessage>,
) {
    use crate::executor::WorkflowExecutor;

    while let Some(message) = rx.recv().await {
        match message {
            RuntimeMessage::Execute {
                workflow_name,
                context,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner.execute(&workflow_name, context).await;
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::RegisterCronWorkflow {
                workflow_name,
                cron_expression,
                timezone,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner
                        .register_cron_workflow(&workflow_name, &cron_expression, &timezone)
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => runner.set_cron_schedule_enabled(id, enabled).await,
                        Err(e) => Err(e),
                    };
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::DeleteCronSchedule {
                schedule_id,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => runner.delete_cron_schedule(id).await,
                        Err(e) => Err(e),
                    };
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::GetCronSchedule {
                schedule_id,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => runner.get_cron_schedule(id).await,
                        Err(e) => Err(e),
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => {
                            runner
                                .update_cron_schedule(id, Some(&cron_expression), Some(&timezone))
                                .await
                        }
                        Err(e) => Err(e),
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => runner.get_cron_execution_history(id, limit, offset).await,
                        Err(e) => Err(e),
                    };
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::GetCronExecutionStats { since, response_tx } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner.get_cron_execution_stats(since).await;
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::ListTriggerSchedules {
                enabled_only,
                limit,
                offset,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let dal = runner.dal();
                    let result = if enabled_only {
                        dal.schedule().get_enabled_triggers().await
                    } else {
                        dal.schedule()
                            .list(Some("trigger"), false, limit, offset)
                            .await
                    };
                    let _ = response_tx.send(result.map_err(|e| {
                        crate::executor::WorkflowExecutionError::Configuration {
                            message: e.to_string(),
                        }
                    }));
                });
            }
            RuntimeMessage::GetTriggerSchedule {
                trigger_name,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let dal = runner.dal();
                    let result = dal.schedule().get_by_trigger_name(&trigger_name).await;
                    let _ = response_tx.send(result.map_err(|e| {
                        crate::executor::WorkflowExecutionError::Configuration {
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = async {
                        if let Some(scheduler) = runner.unified_scheduler().await {
                            if enabled {
                                scheduler.enable_trigger(&trigger_name).await
                            } else {
                                scheduler.disable_trigger(&trigger_name).await
                            }
                        } else {
                            let dal = runner.dal();
                            if let Some(schedule) =
                                dal.schedule().get_by_trigger_name(&trigger_name).await?
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
                        crate::executor::WorkflowExecutionError::Configuration {
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let dal = runner.dal();
                    let result = async {
                        let schedule_opt = dal
                            .schedule()
                            .get_by_trigger_name(&trigger_name)
                            .await
                            .map_err(|e| {
                                crate::executor::WorkflowExecutionError::Configuration {
                                    message: e.to_string(),
                                }
                            })?;
                        if let Some(schedule) = schedule_opt {
                            dal.schedule_execution()
                                .list_by_schedule(schedule.id, limit, offset)
                                .await
                                .map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
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
                info!("Received shutdown signal");
                break;
            }
        }
    }
}

/// Spawn a background thread running a Tokio runtime with a DefaultRunner
/// and wire it to an mpsc event loop.
///
/// `create_runner` is called inside the thread with a reference to the Tokio
/// runtime so it can block_on async DefaultRunner constructors.
fn spawn_runtime<F>(create_runner: F) -> PyResult<PyDefaultRunner>
where
    F: FnOnce(&Runtime) -> Result<crate::DefaultRunner, crate::executor::WorkflowExecutionError>
        + Send
        + 'static,
{
    let (tx, rx) = mpsc::unbounded_channel::<RuntimeMessage>();
    let (init_tx, init_rx) = oneshot::channel::<Result<(), String>>();

    let thread_handle = thread::spawn(move || {
        // Idempotent — only the first call in the process succeeds
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .try_init();

        let rt = Runtime::new().expect("Failed to create tokio runtime");

        let runner = match create_runner(&rt) {
            Ok(r) => {
                info!("DefaultRunner created successfully, background services running");
                let _ = init_tx.send(Ok(()));
                r
            }
            Err(e) => {
                let _ = init_tx.send(Err(format!("Failed to create DefaultRunner: {}", e)));
                return;
            }
        };

        let runner = Arc::new(runner);
        rt.block_on(run_event_loop(runner, rx));
    });

    // Wait for init — propagate errors instead of panicking
    match init_rx.blocking_recv() {
        Ok(Ok(())) => {}
        Ok(Err(msg)) => return Err(PyRuntimeError::new_err(msg)),
        Err(_) => {
            return Err(PyRuntimeError::new_err(
                "Runtime thread died during initialization",
            ))
        }
    }

    Ok(PyDefaultRunner {
        runtime_handle: Mutex::new(AsyncRuntimeHandle {
            tx,
            thread_handle: Some(thread_handle),
        }),
    })
}

// ============================================================================
// PyDefaultRunner
// ============================================================================

/// Python wrapper for DefaultRunner
#[pyclass(name = "DefaultRunner")]
pub struct PyDefaultRunner {
    runtime_handle: Mutex<AsyncRuntimeHandle>,
}

/// Internal (non-Python) helpers.
impl PyDefaultRunner {
    /// Send a message to the runtime thread and block until a response arrives.
    ///
    /// Releases the GIL while waiting so Python threads can proceed.
    fn send_and_recv<T: Send>(
        &self,
        message: RuntimeMessage,
        response_rx: oneshot::Receiver<Result<T, crate::executor::WorkflowExecutionError>>,
        py: Python,
        error_context: &str,
    ) -> PyResult<T> {
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
            result.map_err(|e| PyValueError::new_err(format!("{}: {}", error_context, e)))
        })
    }
}

#[pymethods]
impl PyDefaultRunner {
    /// Create a new DefaultRunner with database connection
    #[new]
    pub fn new(database_url: &str) -> PyResult<Self> {
        let database_url = database_url.to_string();
        spawn_runtime(move |rt| {
            info!(
                "Creating DefaultRunner with database_url: {}",
                crate::logging::mask_db_url(&database_url)
            );
            rt.block_on(crate::DefaultRunner::new(&database_url))
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
        spawn_runtime(move |rt| {
            rt.block_on(crate::DefaultRunner::with_config(
                &database_url,
                rust_config,
            ))
        })
    }

    /// Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy
    ///
    /// Each tenant gets their own schema with independent tables, migrations,
    /// and data. Requires a PostgreSQL database.
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    /// * `schema` - Schema name for tenant isolation (alphanumeric + underscores only)
    #[staticmethod]
    pub fn with_schema(database_url: &str, schema: &str) -> PyResult<PyDefaultRunner> {
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(PyValueError::new_err(
                "Schema-based multi-tenancy requires PostgreSQL. \
                 SQLite does not support database schemas. \
                 Use a PostgreSQL URL like 'postgres://user:pass@host/db'",
            ));
        }
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
        spawn_runtime(move |rt| {
            info!(
                "Creating DefaultRunner with schema: {} and database_url: {}",
                schema,
                crate::logging::mask_db_url(&database_url)
            );
            rt.block_on(crate::DefaultRunner::with_schema(&database_url, &schema))
        })
    }

    /// Execute a workflow by name with context
    pub fn execute(
        &self,
        workflow_name: &str,
        context: &PyContext,
        py: Python,
    ) -> PyResult<PyWorkflowResult> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::Execute {
            workflow_name: workflow_name.to_string(),
            context: context.clone_inner(),
            response_tx,
        };
        let result = self.send_and_recv(message, response_rx, py, "Workflow execution failed")?;
        Ok(PyWorkflowResult::from_result(result))
    }

    /// Shutdown the runner and cleanup resources
    pub fn shutdown(&self, py: Python) -> PyResult<()> {
        info!("Starting async runtime shutdown from Python");
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

    // ========================================================================
    // Cron Management
    // ========================================================================

    /// Register a cron workflow for automatic execution at scheduled times
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to execute
    /// * `cron_expression` - Standard cron expression (e.g., "0 2 * * *")
    /// * `timezone` - Timezone for cron interpretation (e.g., "UTC")
    ///
    /// # Returns
    /// * Schedule ID as a string
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
        self.send_and_recv(message, response_rx, py, "Failed to register cron workflow")
    }

    /// List all cron schedules
    ///
    /// # Arguments
    /// * `enabled_only` - If True, only return enabled schedules
    /// * `limit` - Maximum number of schedules to return (default: 100)
    /// * `offset` - Number of schedules to skip (default: 0)
    pub fn list_cron_schedules(
        &self,
        enabled_only: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::ListCronSchedules {
            enabled_only: enabled_only.unwrap_or(false),
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let schedules =
            self.send_and_recv(message, response_rx, py, "Failed to list cron schedules")?;
        schedules
            .into_iter()
            .map(|s| schedule_to_cron_dict(s, py))
            .collect()
    }

    /// Enable or disable a cron schedule
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
        self.send_and_recv(message, response_rx, py, "Failed to update cron schedule")
    }

    /// Delete a cron schedule
    pub fn delete_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::DeleteCronSchedule {
            schedule_id,
            response_tx,
        };
        self.send_and_recv(message, response_rx, py, "Failed to delete cron schedule")
    }

    /// Get details of a specific cron schedule
    pub fn get_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<PyObject> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronSchedule {
            schedule_id,
            response_tx,
        };
        let schedule =
            self.send_and_recv(message, response_rx, py, "Failed to get cron schedule")?;
        schedule_to_cron_dict(schedule, py)
    }

    /// Update a cron schedule's expression and timezone
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
        self.send_and_recv(message, response_rx, py, "Failed to update cron schedule")
    }

    /// Get execution history for a specific cron schedule
    pub fn get_cron_execution_history(
        &self,
        schedule_id: String,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronExecutionHistory {
            schedule_id,
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let executions = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get cron execution history",
        )?;
        executions
            .into_iter()
            .map(|e| cron_execution_to_dict(e, py))
            .collect()
    }

    /// Get execution statistics for cron schedules
    ///
    /// # Arguments
    /// * `since` - Start time for statistics collection (ISO 8601 string)
    pub fn get_cron_execution_stats(&self, since: String, py: Python) -> PyResult<PyObject> {
        let since_dt = chrono::DateTime::parse_from_rfc3339(&since)
            .map_err(|e| PyValueError::new_err(format!("Invalid datetime format: {}", e)))?
            .with_timezone(&chrono::Utc);

        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronExecutionStats {
            since: since_dt,
            response_tx,
        };
        let stats = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get cron execution stats",
        )?;

        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("total_executions", stats.total_executions)?;
        dict.set_item("successful_executions", stats.successful_executions)?;
        dict.set_item("lost_executions", stats.lost_executions)?;
        dict.set_item("success_rate", stats.success_rate)?;
        Ok(dict.into())
    }

    // ========================================================================
    // Trigger Management
    // ========================================================================

    /// List all trigger schedules
    #[pyo3(signature = (enabled_only=None, limit=None, offset=None))]
    pub fn list_trigger_schedules(
        &self,
        enabled_only: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::ListTriggerSchedules {
            enabled_only: enabled_only.unwrap_or(false),
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let schedules =
            self.send_and_recv(message, response_rx, py, "Failed to list trigger schedules")?;
        schedules
            .into_iter()
            .map(|s| schedule_to_trigger_dict(s, py))
            .collect()
    }

    /// Get details of a specific trigger schedule
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
        let schedule_opt =
            self.send_and_recv(message, response_rx, py, "Failed to get trigger schedule")?;
        match schedule_opt {
            Some(schedule) => Ok(Some(schedule_to_trigger_dict(schedule, py)?)),
            None => Ok(None),
        }
    }

    /// Enable or disable a trigger
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
        self.send_and_recv(message, response_rx, py, "Failed to update trigger")
    }

    /// Get execution history for a specific trigger
    #[pyo3(signature = (trigger_name, limit=None, offset=None))]
    pub fn get_trigger_execution_history(
        &self,
        trigger_name: String,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetTriggerExecutionHistory {
            trigger_name,
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let executions = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get trigger execution history",
        )?;
        executions
            .into_iter()
            .map(|e| trigger_execution_to_dict(e, py))
            .collect()
    }

    // ========================================================================
    // Lifecycle
    // ========================================================================

    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }

    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

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

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use super::*;
    use serial_test::serial;

    const TEST_PG_URL: &str = "postgres://cloacina:cloacina@localhost:5432/cloacina";

    fn unique_sqlite_url() -> String {
        format!(
            "file:cloacina_runner_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        )
    }

    #[test]
    #[serial]
    fn test_runner_repr() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        assert_eq!(
            runner.__repr__(),
            "DefaultRunner(thread_separated_async_runtime)"
        );
    }

    #[test]
    #[serial]
    fn test_runner_shutdown() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        Python::with_gil(|py| {
            runner.shutdown(py).expect("Shutdown should succeed");
        });
    }

    #[test]
    #[serial]
    fn test_runner_context_manager() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let runner = Py::new(py, PyDefaultRunner::new(&unique_sqlite_url()).unwrap()).unwrap();
            let entered = PyDefaultRunner::__enter__(runner.borrow(py));
            assert_eq!(
                entered.__repr__(),
                "DefaultRunner(thread_separated_async_runtime)"
            );
            let result = runner.borrow(py).__exit__(py, None, None, None);
            assert!(result.is_ok());
        });
    }

    #[test]
    #[serial]
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
    #[serial]
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
    #[serial]
    fn test_runner_get_trigger_schedule_not_found() {
        pyo3::prepare_freethreaded_python();
        let runner = PyDefaultRunner::new(&unique_sqlite_url()).expect("Failed to create runner");
        Python::with_gil(|py| {
            let result = runner.get_trigger_schedule("nonexistent".to_string(), py);
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
    fn test_runner_register_cron_workflow() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let schedule_id = runner
                .register_cron_workflow(
                    "test-cron-wf".to_string(),
                    "0 * * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .expect("register_cron_workflow should succeed");
            assert!(!schedule_id.is_empty());
            assert!(uuid::Uuid::parse_str(&schedule_id).is_ok());
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
    fn test_runner_list_cron_schedules_after_register() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            runner
                .register_cron_workflow(
                    "list-test-wf".to_string(),
                    "0 12 * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .unwrap();

            let schedules = runner
                .list_cron_schedules(None, None, None, py)
                .expect("list should succeed");
            assert_eq!(schedules.len(), 1);
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
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
            assert!(!schedule_obj.is_none(py));
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
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

            runner
                .set_cron_schedule_enabled(schedule_id.clone(), false, py)
                .expect("disable should succeed");
            runner
                .set_cron_schedule_enabled(schedule_id, true, py)
                .expect("enable should succeed");
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
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

            let schedules = runner.list_cron_schedules(None, None, None, py).unwrap();
            assert!(schedules.is_empty());
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
    fn test_runner_set_trigger_enabled() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let result = runner.set_trigger_enabled("nonexistent".to_string(), true, py);
            let _ = result;
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
    fn test_runner_get_trigger_execution_history() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let history =
                runner.get_trigger_execution_history("nonexistent".to_string(), None, None, py);
            let _ = history;
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
    fn test_workflow_result_completed() {
        pyo3::prepare_freethreaded_python();
        let mut ctx = crate::Context::new();
        ctx.insert("result".to_string(), serde_json::json!("done"))
            .unwrap();
        let result = crate::executor::WorkflowExecutionResult {
            execution_id: uuid::Uuid::new_v4(),
            workflow_name: "test-wf".to_string(),
            status: crate::executor::WorkflowStatus::Completed,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            duration: Some(std::time::Duration::from_secs(1)),
            final_context: ctx,
            task_results: vec![],
            error_message: None,
        };
        let py_result = PyWorkflowResult::from_result(result);

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
    #[serial]
    fn test_workflow_result_failed() {
        pyo3::prepare_freethreaded_python();
        let result = crate::executor::WorkflowExecutionResult {
            execution_id: uuid::Uuid::new_v4(),
            workflow_name: "fail-wf".to_string(),
            status: crate::executor::WorkflowStatus::Failed,
            start_time: chrono::Utc::now(),
            end_time: None,
            duration: None,
            final_context: crate::Context::new(),
            task_results: vec![],
            error_message: Some("something broke".to_string()),
        };
        let py_result = PyWorkflowResult::from_result(result);

        assert_eq!(py_result.status(), "Failed");
        assert!(py_result.end_time().is_none());
        assert_eq!(py_result.error_message(), Some("something broke"));
        assert!(py_result.__repr__().contains("something broke"));
    }

    #[test]
    #[serial]
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
    #[serial]
    fn test_runner_execute_registered_workflow() {
        pyo3::prepare_freethreaded_python();

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

            let workflow_result = result.unwrap();
            assert!(!workflow_result.status().is_empty());
            runner.shutdown(py).unwrap();
        });
    }

    #[test]
    #[serial]
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
    #[serial]
    fn test_runner_list_cron_schedules_enabled_only() {
        pyo3::prepare_freethreaded_python();
        let url = unique_sqlite_url();
        let runner = PyDefaultRunner::new(&url).expect("Failed to create runner");
        Python::with_gil(|py| {
            let id = runner
                .register_cron_workflow(
                    "filter-test".to_string(),
                    "0 0 * * *".to_string(),
                    "UTC".to_string(),
                    py,
                )
                .unwrap();
            runner.set_cron_schedule_enabled(id, false, py).unwrap();

            let enabled = runner
                .list_cron_schedules(Some(true), None, None, py)
                .unwrap();
            assert!(
                enabled.is_empty(),
                "disabled schedule should be filtered out"
            );

            let all = runner
                .list_cron_schedules(Some(false), None, None, py)
                .unwrap();
            assert_eq!(all.len(), 1);
            runner.shutdown(py).unwrap();
        });
    }

    // ── with_schema validation tests ─────────────────────────────────

    #[test]
    #[serial]
    fn test_with_schema_rejects_sqlite() {
        pyo3::prepare_freethreaded_python();
        let result = PyDefaultRunner::with_schema("sqlite:///tmp/test.db", "tenant_a");
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_with_schema_rejects_empty_schema() {
        pyo3::prepare_freethreaded_python();
        let result = PyDefaultRunner::with_schema(
            "postgres://cloacina:cloacina@localhost:5432/cloacina",
            "",
        );
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_with_schema_rejects_invalid_chars() {
        pyo3::prepare_freethreaded_python();
        let result = PyDefaultRunner::with_schema(
            "postgres://cloacina:cloacina@localhost:5432/cloacina",
            "tenant;DROP TABLE",
        );
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_shutdown_error_display() {
        let err = ShutdownError::ChannelClosed;
        assert!(format!("{}", err).contains("channel closed"));

        let err = ShutdownError::ThreadPanic;
        assert!(format!("{}", err).contains("panicked"));

        let err = ShutdownError::Timeout(5);
        assert!(format!("{}", err).contains("5"));
    }
}
