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

//! # Task Scheduler
//!
//! The Task Scheduler converts Workflow definitions into persistent database execution plans
//! and manages task readiness based on dependencies and trigger rules.
//!
//! ## Overview
//!
//! The scheduler builds on existing Cloacina components:
//! - **Workflow**: Task definitions and dependency graphs
//! - **Context**: Type-safe serializable execution context
//! - **Database**: Persistent execution state tracking
//! - **DAL**: Data access layer for database operations
//!
//! ## Key Features
//!
//! - Convert Workflow instances into database execution plans
//! - Manage task state transitions based on dependencies
//! - Support advanced trigger rules for conditional execution
//! - Coordinate with executor through database state
//! - Context management and merging for task dependencies
//!
//! Recovery of orphaned tasks is handled out-of-band by
//! [`StaleClaimSweeper`](crate::execution_planner::stale_claim_sweeper::StaleClaimSweeper), which releases stale claims
//! based on heartbeat expiry rather than wall-clock task status.
//!
//! ## Task State Management
//!
//! Tasks transition through the following states:
//! - **NotStarted**: Initial state when task is created
//! - **Pending**: Waiting for dependencies to complete
//! - **Ready**: Dependencies satisfied, ready for execution
//! - **Running**: Currently being executed
//! - **Completed**: Successfully finished
//! - **Failed**: Execution failed
//! - **Skipped**: Skipped due to trigger rules
//! - **Abandoned**: Permanently failed after recovery attempts
//!
//! ## Error Handling & Recovery
//!
//! The scheduler relies on per-task retry policies for failure handling.
//! Orphaned tasks (claimed by a runner whose heartbeat has expired) are
//! reclaimed by the stale claim sweeper rather than by scheduler startup.
//!
//! ## Context Management
//!
//! Context handling follows these rules:
//! - Initial context provided at workflow execution
//! - Single dependency: inherits context directly
//! - Multiple dependencies: merges contexts with later overrides
//! - Type-safe serialization/deserialization
//! - Validation of context values in trigger rules
//!
//! ## Performance Considerations
//!
//! - Scheduling loop runs every second by default
//! - Efficient database queries for task state updates
//! - Batch processing of task readiness checks
//! - Optimized context merging for multiple dependencies
//! - Minimal database locking for concurrent operations
//!
//! ## Thread Safety
//!
//! The scheduler is designed for concurrent operation:
//! - Thread-safe database operations
//! - Atomic task state transitions
//! - Safe context merging for parallel tasks
//! - Lock-free trigger rule evaluation
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cloacina::{workflow, task, Context, Database, TaskError};
//! use cloacina::execution_planner::TaskScheduler;
//!
//! // Define tasks
//! #[task(id = "fetch-data", dependencies = [])]
//! async fn fetch_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     context.insert("data", serde_json::json!({"status": "fetched"}))?;
//!     Ok(())
//! }
//!
//! // Create workflow
//! let workflow = workflow! {
//!     name: "data-pipeline",
//!     description: "Simple data processing workflow",
//!     tasks: [fetch_data]
//! };
//!
//! // Schedule execution
//! let database = Database::new("postgresql://localhost/cloacina")?;
//! let scheduler = TaskScheduler::new(database, vec![workflow]);
//! let input_context = Context::new();
//! let execution_id = scheduler.schedule_workflow_execution("data-pipeline", input_context).await?;
//!
//! // Run scheduling loop
//! scheduler.run_scheduling_loop().await?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod context_manager;
pub mod priority;
mod scheduler_loop;
pub mod stale_claim_sweeper;
mod state_manager;
mod trigger_rules;

// Re-export public types
pub use priority::{PriorityAuthority, PriorityChange, RunPriority, WaitingOrder};
pub use trigger_rules::{TriggerCondition, TriggerRule, ValueOperator};

use std::sync::Arc;
use std::time::Duration;

use diesel::Connection;
use diesel::prelude::*;
use tracing::{info, warn};
use uuid::Uuid;

use crate::Runtime;
use crate::dal::DAL;
use crate::dal::unified::models::{NewUnifiedTaskExecution, NewUnifiedWorkflowExecution};
use crate::database::schema::unified::{task_executions, workflow_executions};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::dispatcher::Dispatcher;
use crate::error::ValidationError;
use crate::task::TaskNamespace;
use crate::{Context, Database, Workflow};

use scheduler_loop::SchedulerLoop;

/// One task row the planner writes for a new workflow execution.
struct PlannedTaskRow {
    task_name: String,
    trigger_rules: String,
    task_config: String,
    max_attempts: i32,
    /// CLOACINA-T-1027: the task's agent pool (`None` = `default`) and its
    /// requests, from `Task::placement()`.
    pool: Option<String>,
    cpu_request_millis: Option<i64>,
    memory_request_bytes: Option<i64>,
    /// CLOACINA-T-1045: the task's own priority, as given.
    task_priority: i32,
    /// CLOACINA-T-1123: the priority the task declares, as asked; `None`
    /// when it declares none.
    task_priority_asked: Option<i32>,
}

/// The priority of a run as the planner worked it out (CLOACINA-T-1123).
struct ResolvedPriority {
    /// The workflow priority: asked and given.
    grant: cloacina_api_types::PriorityGrant,
    /// The limit of what the package's code declares, for the tasks.
    declared_limit: Option<u8>,
    /// The record whose last given value this run sets: kind, target, setter.
    tracked: Option<(&'static str, String, crate::dal::unified::PrioritySetter)>,
}

/// The main Task Scheduler that manages workflow execution and task readiness.
///
/// The TaskScheduler converts Workflow definitions into persistent database execution plans,
/// tracks task state transitions, and manages dependencies through trigger rules.
///
/// # Thread Safety
///
/// The TaskScheduler is designed to be thread-safe and can be shared across multiple threads.
/// All database operations are performed through a connection pool, and state transitions
/// are handled atomically.
///
/// # Error Handling
///
/// The scheduler implements comprehensive error handling:
/// - Database errors are wrapped in ValidationError
/// - Workflow validation errors are caught early
/// - Context evaluation errors are handled gracefully
///
/// # Performance
///
/// The scheduler is optimized for:
/// - Efficient database operations
/// - Minimal locking
/// - Batch processing where possible
/// - Memory-efficient context management
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::{Database, TaskScheduler};
/// use cloacina::workflow::Workflow;
///
/// // Create a new scheduler with recovery
/// let database = Database::new("postgresql://localhost/cloacina")?;
/// let scheduler = TaskScheduler::with_global_workflows_and_recovery(database).await?;
///
/// // Run the scheduling loop
/// scheduler.run_scheduling_loop().await?;
/// ```
pub struct TaskScheduler {
    dal: DAL,
    runtime: Arc<Runtime>,
    instance_id: Uuid,
    poll_interval: Duration,
    /// Optional dispatcher for push-based task execution
    dispatcher: Option<Arc<dyn Dispatcher>>,
    /// Shutdown signal for graceful termination of the scheduling loop.
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
    /// CLOACINA-T-1045: who limits the priority a package declares. `None`:
    /// a declared priority is given as declared.
    priority_authority: std::sync::RwLock<Option<Arc<dyn PriorityAuthority>>>,
    /// CLOACINA-T-1045: the ageing interval of waiting work.
    priority_aging_interval: Duration,
}

impl TaskScheduler {
    /// Creates a new TaskScheduler instance with default configuration using global workflow registry.
    ///
    /// This is the recommended constructor for most use cases. The TaskScheduler will:
    /// - Use all workflows registered in the global registry
    /// - Use default poll interval (100ms)
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    ///
    /// # Returns
    ///
    /// A new TaskScheduler instance ready to schedule and manage workflow executions.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cloacina::{Database, TaskScheduler};
    ///
    /// let database = Database::new("postgresql://localhost/cloacina")?;
    /// let scheduler = TaskScheduler::new(database).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// May return ValidationError if construction fails.
    pub async fn new(database: Database) -> Result<Self, ValidationError> {
        let scheduler = Self::with_poll_interval(database, Duration::from_millis(100)).await?;
        Ok(scheduler)
    }

    /// Creates a new TaskScheduler with custom poll interval using global workflow registry.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    /// * `poll_interval` - How often to check for ready tasks
    ///
    /// # Returns
    ///
    /// A new TaskScheduler instance ready to schedule and manage workflow executions.
    pub async fn with_poll_interval(
        database: Database,
        poll_interval: Duration,
    ) -> Result<Self, ValidationError> {
        Ok(Self::with_poll_interval_sync(database, poll_interval))
    }

    /// Creates a new TaskScheduler with custom poll interval (synchronous version).
    pub(crate) fn with_poll_interval_sync(database: Database, poll_interval: Duration) -> Self {
        let dal = DAL::new(database.clone());

        Self {
            dal,
            runtime: Arc::new(Runtime::new()),
            instance_id: Uuid::new_v4(),
            poll_interval,
            dispatcher: None,
            shutdown_rx: None,
            priority_authority: std::sync::RwLock::new(None),
            priority_aging_interval: cloacina_api_types::DEFAULT_PRIORITY_AGING_INTERVAL,
        }
    }

    /// Sets who limits the priority that a package declares
    /// (CLOACINA-T-1045). With none set, a declared priority is given as
    /// declared.
    pub fn with_priority_authority(self, authority: Arc<dyn PriorityAuthority>) -> Self {
        self.set_priority_authority(authority);
        self
    }

    /// [`with_priority_authority`](Self::with_priority_authority) on a
    /// scheduler that is already built.
    pub fn set_priority_authority(&self, authority: Arc<dyn PriorityAuthority>) {
        *self
            .priority_authority
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(authority);
    }

    fn priority_authority(&self) -> Option<Arc<dyn PriorityAuthority>> {
        self.priority_authority
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Sets the ageing interval of waiting work (CLOACINA-T-1045). For each
    /// interval a task has waited, its priority in the order improves by
    /// one. The default is two minutes. Zero switches ageing off.
    pub fn with_priority_aging_interval(mut self, interval: Duration) -> Self {
        self.priority_aging_interval = interval;
        self
    }

    /// Sets the runtime for this scheduler, replacing the default.
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = runtime;
        self
    }

    /// Returns a reference to the runtime used by this scheduler.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Sets the shutdown receiver for graceful termination of the scheduling loop.
    pub fn with_shutdown(mut self, shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.shutdown_rx = Some(shutdown_rx);
        self
    }

    /// Sets the dispatcher for push-based task execution.
    ///
    /// When a dispatcher is configured, the scheduler will dispatch task events
    /// when tasks become ready, in addition to marking them Ready in the database.
    ///
    /// # Arguments
    ///
    /// * `dispatcher` - The dispatcher to use for task events
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn with_dispatcher(mut self, dispatcher: Arc<dyn Dispatcher>) -> Self {
        self.dispatcher = Some(dispatcher);
        self
    }

    /// Returns a reference to the dispatcher if configured.
    pub fn dispatcher(&self) -> Option<&Arc<dyn Dispatcher>> {
        self.dispatcher.as_ref()
    }

    /// Schedules a new workflow execution with the provided input context.
    ///
    /// This method:
    /// 1. Validates the workflow exists in the registry
    /// 2. Stores the input context in the database
    /// 3. Creates a new workflow execution record
    /// 4. Initializes task execution records for all workflow tasks
    ///
    /// # Arguments
    ///
    /// * `workflow_name` - Name of the workflow to execute
    /// * `input_context` - Context containing input data for the workflow
    ///
    /// # Returns
    ///
    /// The UUID of the created workflow execution on success.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cloacina::{Context, TaskScheduler};
    /// use serde_json::json;
    ///
    /// let scheduler = TaskScheduler::new(database).await?;
    /// let mut context = Context::new();
    /// context.insert("input", json!({"key": "value"}))?;
    ///
    /// let execution_id = scheduler.schedule_workflow_execution("my-workflow", context).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::WorkflowNotFound` if the workflow doesn't exist in the registry,
    /// or other validation errors if database operations fail.
    ///
    /// # Performance
    ///
    /// This operation performs multiple database transactions:
    /// - Context storage
    /// - Workflow execution creation
    /// - Task execution initialization
    ///
    /// All operations are performed in a single transaction for consistency.
    pub async fn schedule_workflow_execution(
        &self,
        workflow_name: &str,
        input_context: Context<serde_json::Value>,
    ) -> Result<Uuid, ValidationError> {
        self.schedule_workflow_execution_with_priority(
            workflow_name,
            input_context,
            RunPriority::default(),
        )
        .await
        .map(|(id, _)| id)
    }

    /// [`schedule_workflow_execution`](Self::schedule_workflow_execution)
    /// for a run that asks for a workflow priority (CLOACINA-T-1045).
    ///
    /// Returns the execution id and the priority: asked and given. The given
    /// priority is the asked one held to `run.limit`. A run that asks for
    /// nothing gets what the workflow declares, held to the limit of its
    /// package ([`PriorityAuthority`]). Each task row gets the workflow's
    /// given priority and its own: what the task declares, held to the same
    /// package limit, or the workflow's when it declares none.
    pub async fn schedule_workflow_execution_with_priority(
        &self,
        workflow_name: &str,
        input_context: Context<serde_json::Value>,
        run: RunPriority,
    ) -> Result<(Uuid, cloacina_api_types::PriorityGrant), ValidationError> {
        info!("Scheduling workflow execution: {}", workflow_name);

        // Look up workflow in scoped runtime registry
        let workflow = match self.runtime.get_workflow(workflow_name) {
            Some(wf) => wf,
            None => return Err(ValidationError::WorkflowNotFound(workflow_name.to_string())),
        };

        let current_version = workflow.metadata().version.clone();
        let last_version = self
            .dal
            .workflow_execution()
            .get_last_version(workflow_name)
            .await?;

        if last_version.as_ref() != Some(&current_version) {
            info!(
                "Workflow '{}' version changed: {} -> {}",
                workflow_name,
                last_version.unwrap_or_else(|| "none".to_string()),
                current_version
            );
        }

        // Store context first (separate operation - needed before main transaction)
        let stored_context = self.dal.context().create(&input_context).await?;

        // CLOACINA-T-1045: the priority of the run. The limit of what the
        // package's code may declare is read now, at the start of the run:
        // a limit that was raised or lowered since the publish applies.
        // CLOACINA-T-1123: the limit is the one of who published the package
        // (or created the schedule), as it is now.
        let authority = self.priority_authority();
        let resolved = self
            .resolve_run_priority(run, &workflow, workflow_name, authority.as_deref())
            .await;
        let declared_limit = resolved.declared_limit;
        let workflow_priority = resolved.grant;

        // Build all task data BEFORE the transaction
        let task_ids = workflow.topological_sort()?;
        let mut task_data: Vec<PlannedTaskRow> = Vec::with_capacity(task_ids.len());

        for task_id in &task_ids {
            let trigger_rules = self.get_task_trigger_rules(&workflow, task_id);
            let task_config = self.get_task_configuration(&workflow, task_id);
            let max_attempts = workflow
                .get_task(task_id)
                .map(|t| t.retry_policy().max_attempts)
                .unwrap_or(3);

            // CLOACINA-T-1027: the task's agent pool and requests go on its
            // row, so the fleet packs by them and the waiting gauges group
            // by pool.
            let placement = workflow
                .get_task(task_id)
                .map(|t| t.placement())
                .unwrap_or_default();

            task_data.push(PlannedTaskRow {
                task_name: task_id.to_string(),
                trigger_rules: trigger_rules.to_string(),
                task_config: task_config.to_string(),
                max_attempts,
                pool: placement.pool.clone(),
                cpu_request_millis: placement.cpu_millis().and_then(|v| i64::try_from(v).ok()),
                memory_request_bytes: placement.memory_bytes().and_then(|v| i64::try_from(v).ok()),
                task_priority: i32::from(priority::task_priority(
                    placement.priority,
                    declared_limit,
                    workflow_priority.given,
                )),
                task_priority_asked: placement.priority.map(i32::from),
            });
        }

        // Prepare workflow execution data
        let workflow_execution_id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let wf_name = workflow_name.to_string();
        let wf_version = current_version.clone();

        // Create workflow execution AND tasks in a single atomic transaction
        // This prevents the race condition where the scheduler sees a workflow execution before tasks exist
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_workflow_execution_postgres(
                workflow_execution_id,
                now,
                wf_name,
                wf_version,
                stored_context,
                task_data,
                workflow_priority,
            )
            .await?,
            self.create_workflow_execution_sqlite(
                workflow_execution_id,
                now,
                wf_name,
                wf_version,
                stored_context,
                task_data,
                workflow_priority,
            )
            .await?
        );

        if workflow_priority.is_held() {
            metrics::counter!(
                "cloacina_priority_held_total",
                "tenant" => self.dal.tenant_label().to_string(),
            )
            .increment(1);
            if let Some(authority) = &authority {
                authority.run_held(workflow_name, workflow_priority).await;
            }
        }

        // CLOACINA-T-1123: the given priority of a package or a schedule is
        // stored; the one run start that changes it, on any replica, reports
        // the change.
        if let (Some(authority), Some((kind, target, setter))) = (&authority, resolved.tracked) {
            match self
                .dal
                .priority_grants()
                .set_last_given(kind, &target, workflow_priority.given)
                .await
            {
                Ok(true) => {
                    authority
                        .given_changed(&priority::PriorityChange {
                            kind: kind.to_string(),
                            target,
                            workflow_name: workflow_name.to_string(),
                            setter,
                            grant: workflow_priority,
                        })
                        .await
                }
                Ok(false) => {}
                Err(e) => warn!(
                    workflow = %workflow_name,
                    "the given priority of the run was not stored: {e}"
                ),
            }
        }

        // NOTE: cloacina_active_workflows gauge is SQL-derived — re-seeded
        // each tick in SchedulerLoop::process_active_executions from the
        // workflow_executions row count. We intentionally do NOT increment
        // here; doing so would cause gauge drift on any code path that skips
        // finalize_workflow_execution (crash, claim loss, etc.).
        info!(
            priority = workflow_priority.given,
            priority_asked = workflow_priority.asked,
            "Workflow execution scheduled: {}",
            workflow_execution_id
        );
        Ok((workflow_execution_id.into(), workflow_priority))
    }

    /// The priority of a run (CLOACINA-T-1045, CLOACINA-T-1123).
    ///
    /// - A schedule that fires the run and was created with a priority: that
    ///   priority, held to the limit of its creator now.
    /// - A run that asks for a priority: that, held to `run.limit`.
    /// - Else what the workflow declares, held to the limit of who published
    ///   the package now. A package with no record of its publisher (one
    ///   published before CLOACINA-T-1123) is held to
    ///   [`PriorityAuthority::declared_limit`].
    ///
    /// With no authority (the embedded use) nothing is held and nothing is
    /// tracked.
    async fn resolve_run_priority(
        &self,
        run: RunPriority,
        workflow: &Workflow,
        workflow_name: &str,
        authority: Option<&dyn PriorityAuthority>,
    ) -> ResolvedPriority {
        use crate::dal::unified::priority_grants::{KIND_PACKAGE, KIND_SCHEDULE};

        let Some(authority) = authority else {
            return ResolvedPriority {
                grant: priority::workflow_grant(run, workflow.priority(), None),
                declared_limit: None,
                tracked: None,
            };
        };
        let grants = self.dal.priority_grants();
        let read = |result: Result<
            Option<crate::dal::unified::PriorityGrantRow>,
            ValidationError,
        >| {
            result.unwrap_or_else(|e| {
                warn!(workflow = %workflow_name, "the record of who set the priority was not read: {e}");
                None
            })
        };

        // The package: the limit of its publisher, else the old default.
        let package = workflow.package().to_string();
        let package_row = if package.is_empty() {
            None
        } else {
            read(grants.get(KIND_PACKAGE, &package).await)
        };
        let declared_limit = match &package_row {
            Some(row) => authority.limit_of(&row.setter()).await,
            None => authority.declared_limit(workflow_name).await,
        };

        // A schedule created with a priority.
        if let Some(schedule) = run.schedule {
            let target = schedule.to_string();
            if let Some(row) = read(grants.get(KIND_SCHEDULE, &target).await)
                && let Some(asked) = row.asked()
            {
                let limit = authority.limit_of(&row.setter()).await;
                return ResolvedPriority {
                    grant: priority::grant_under(asked, limit),
                    declared_limit,
                    tracked: Some((KIND_SCHEDULE, target, row.setter())),
                };
            }
        }

        if run.asked.is_some() {
            return ResolvedPriority {
                grant: priority::workflow_grant(run, workflow.priority(), declared_limit),
                declared_limit,
                tracked: None,
            };
        }
        ResolvedPriority {
            grant: priority::workflow_grant(run, workflow.priority(), declared_limit),
            declared_limit,
            tracked: package_row.map(|row| (KIND_PACKAGE, package, row.setter())),
        }
    }

    /// Creates workflow execution and tasks in PostgreSQL.
    #[cfg(feature = "postgres")]
    async fn create_workflow_execution_postgres(
        &self,
        workflow_execution_id: UniversalUuid,
        now: UniversalTimestamp,
        workflow_name: String,
        workflow_version: String,
        stored_context: Option<UniversalUuid>,
        task_data: Vec<PlannedTaskRow>,
        workflow_priority: cloacina_api_types::PriorityGrant,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database()
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction(|conn| {
                // Insert workflow execution
                diesel::insert_into(workflow_executions::table)
                    .values(&NewUnifiedWorkflowExecution {
                        id: workflow_execution_id,
                        workflow_name,
                        workflow_version,
                        status: "Pending".to_string(),
                        context_id: stored_context,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                        priority_asked: Some(i32::from(workflow_priority.asked)),
                        priority: Some(i32::from(workflow_priority.given)),
                    })
                    .execute(conn)?;

                // Insert all tasks
                for task in task_data {
                    diesel::insert_into(task_executions::table)
                        .values(&NewUnifiedTaskExecution {
                            id: UniversalUuid::new_v4(),
                            workflow_execution_id,
                            task_name: task.task_name,
                            status: "NotStarted".to_string(),
                            attempt: 1,
                            max_attempts: task.max_attempts,
                            trigger_rules: task.trigger_rules,
                            task_configuration: task.task_config,
                            pool: task.pool,
                            cpu_request_millis: task.cpu_request_millis,
                            memory_request_bytes: task.memory_request_bytes,
                            created_at: now,
                            updated_at: now,
                            workflow_priority: Some(i32::from(workflow_priority.given)),
                            task_priority: Some(task.task_priority),
                            task_priority_asked: task.task_priority_asked,
                        })
                        .execute(conn)?;
                }

                Ok::<_, diesel::result::Error>(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Creates workflow execution and tasks in SQLite.
    #[cfg(feature = "sqlite")]
    async fn create_workflow_execution_sqlite(
        &self,
        workflow_execution_id: UniversalUuid,
        now: UniversalTimestamp,
        workflow_name: String,
        workflow_version: String,
        stored_context: Option<UniversalUuid>,
        task_data: Vec<PlannedTaskRow>,
        workflow_priority: cloacina_api_types::PriorityGrant,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database()
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction(|conn| {
                // Insert workflow execution
                diesel::insert_into(workflow_executions::table)
                    .values(&NewUnifiedWorkflowExecution {
                        id: workflow_execution_id,
                        workflow_name,
                        workflow_version,
                        status: "Pending".to_string(),
                        context_id: stored_context,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                        priority_asked: Some(i32::from(workflow_priority.asked)),
                        priority: Some(i32::from(workflow_priority.given)),
                    })
                    .execute(conn)?;

                // Insert all tasks
                for task in task_data {
                    diesel::insert_into(task_executions::table)
                        .values(&NewUnifiedTaskExecution {
                            id: UniversalUuid::new_v4(),
                            workflow_execution_id,
                            task_name: task.task_name,
                            status: "NotStarted".to_string(),
                            attempt: 1,
                            max_attempts: task.max_attempts,
                            trigger_rules: task.trigger_rules,
                            task_configuration: task.task_config,
                            pool: task.pool,
                            cpu_request_millis: task.cpu_request_millis,
                            memory_request_bytes: task.memory_request_bytes,
                            created_at: now,
                            updated_at: now,
                            workflow_priority: Some(i32::from(workflow_priority.given)),
                            task_priority: Some(task.task_priority),
                            task_priority_asked: task.task_priority_asked,
                        })
                        .execute(conn)?;
                }

                Ok::<_, diesel::result::Error>(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Runs the main scheduling loop that continuously processes active workflow executions.
    ///
    /// This loop:
    /// 1. Checks for active workflow executions
    /// 2. Updates task readiness based on dependencies and trigger rules
    /// 3. Marks completed workflow executions
    /// 4. Repeats every second
    ///
    /// # Returns
    ///
    /// This method runs indefinitely until an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cloacina::TaskScheduler;
    ///
    /// let scheduler = TaskScheduler::with_global_workflows(database);
    /// scheduler.run_scheduling_loop().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns validation errors if database operations fail during the scheduling loop.
    /// The loop will continue running on non-fatal errors, with errors logged.
    ///
    /// # Performance
    ///
    /// The scheduling loop:
    /// - Runs every second by default
    /// - Processes all active workflow executions in each iteration
    /// - Uses efficient batch queries where possible
    /// - Implements backoff for database errors
    ///
    /// # Thread Safety
    ///
    /// The scheduling loop is designed to be run in a separate thread or task.
    /// Multiple instances should not be run simultaneously.
    pub async fn run_scheduling_loop(&self) -> Result<(), ValidationError> {
        let mut scheduler_loop = SchedulerLoop::with_dispatcher(
            &self.dal,
            self.runtime.clone(),
            self.instance_id,
            self.poll_interval,
            self.dispatcher.clone(),
        )
        .with_priority_aging_interval(self.priority_aging_interval);
        if let Some(ref shutdown_rx) = self.shutdown_rx {
            scheduler_loop = scheduler_loop.with_shutdown(shutdown_rx.clone());
        }
        scheduler_loop.run().await
    }

    /// Processes all active workflow executions to update task readiness.
    pub async fn process_active_executions(&self) -> Result<(), ValidationError> {
        let scheduler_loop = SchedulerLoop::with_dispatcher(
            &self.dal,
            self.runtime.clone(),
            self.instance_id,
            self.poll_interval,
            self.dispatcher.clone(),
        )
        .with_priority_aging_interval(self.priority_aging_interval);
        scheduler_loop.process_active_executions().await
    }

    /// Gets trigger rules for a specific task from the task implementation.
    fn get_task_trigger_rules(
        &self,
        workflow: &Workflow,
        task_namespace: &TaskNamespace,
    ) -> serde_json::Value {
        workflow
            .get_task(task_namespace)
            .map(|task| task.trigger_rules())
            .unwrap_or_else(|_| serde_json::json!({"type": "Always"}))
    }

    /// Gets task configuration (currently returns empty object).
    fn get_task_configuration(
        &self,
        _workflow: &Workflow,
        _task_namespace: &TaskNamespace,
    ) -> serde_json::Value {
        // In the future, this could include task-specific configuration
        serde_json::json!({})
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod priority_tests;

#[cfg(all(test, feature = "sqlite"))]
mod placement_row_tests {
    //! CLOACINA-T-1027: the planner writes a task's agent pool and requests
    //! on its row when it creates the rows of a workflow execution. The fleet
    //! packs by the requests, and the waiting gauges group by the pool.

    use super::*;
    use crate::{Task, TaskError};
    use cloacina_workflow::TaskPlacement;

    struct PlacedStub {
        id: String,
        placement: TaskPlacement,
    }

    #[async_trait::async_trait]
    impl Task for PlacedStub {
        async fn execute(
            &self,
            context: Context<serde_json::Value>,
        ) -> Result<Context<serde_json::Value>, TaskError> {
            Ok(context)
        }
        fn id(&self) -> &str {
            &self.id
        }
        fn dependencies(&self) -> &[TaskNamespace] {
            &[]
        }
        fn placement(&self) -> TaskPlacement {
            self.placement.clone()
        }
    }

    fn stub(id: &str, placement: TaskPlacement) -> Arc<dyn Task> {
        Arc::new(PlacedStub {
            id: id.to_string(),
            placement,
        })
    }

    #[tokio::test]
    async fn the_planner_writes_the_pool_and_requests_on_the_task_rows() {
        let url = format!(
            "file:placement_rows_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let database = Database::new(&url, "", 5);
        database.run_migrations().await.expect("migrations");

        let build = || {
            Workflow::builder("placement_rows_workflow")
                .add_task(stub(
                    "big",
                    TaskPlacement {
                        pool: Some("large".to_string()),
                        cpu: Some("500m".to_string()),
                        memory: Some("32Gi".to_string()),
                        ..Default::default()
                    },
                ))
                .expect("add big")
                .add_task(stub("plain", TaskPlacement::default()))
                .expect("add plain")
                .build()
                .expect("build")
        };
        let runtime = Arc::new(Runtime::empty());
        runtime.register_workflow("placement_rows_workflow".to_string(), build);

        let scheduler =
            TaskScheduler::with_poll_interval_sync(database.clone(), Duration::from_millis(100))
                .with_runtime(runtime);
        let execution_id = scheduler
            .schedule_workflow_execution("placement_rows_workflow", Context::new())
            .await
            .expect("schedule");

        let dal = DAL::new(database);
        let rows = dal
            .task_execution()
            .get_all_tasks_for_workflow(UniversalUuid(execution_id))
            .await
            .expect("task rows");
        assert_eq!(rows.len(), 2);
        let row = |suffix: &str| {
            rows.iter()
                .find(|t| t.task_name.ends_with(suffix))
                .unwrap_or_else(|| panic!("a row for {suffix}: {rows:?}"))
        };
        let big = row("::big");
        assert_eq!(big.pool.as_deref(), Some("large"));
        assert_eq!(big.cpu_request_millis, Some(500));
        assert_eq!(big.memory_request_bytes, Some(32 << 30));
        assert_eq!(big.fleet_agent_id, None);
        let plain = row("::plain");
        assert_eq!(plain.pool, None);
        assert_eq!(plain.cpu_request_millis, None);
        assert_eq!(plain.memory_request_bytes, None);
    }
}
