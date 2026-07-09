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

//! State transition operations for task executions.
//!
//! All state transitions are transactional: the status update and execution event
//! are written atomically. If either fails, both are rolled back.

use super::TaskExecutionDAL;
use crate::dal::unified::models::{NewUnifiedExecutionEvent, UnifiedTaskExecution};
// Only the sqlite mark_ready sets created_at explicitly (no column default on
// sqlite); the postgres path now relies on DEFAULT CURRENT_TIMESTAMP, so this is
// unused under postgres-only builds without the gate.
#[cfg(feature = "sqlite")]
use crate::dal::unified::models::NewUnifiedTaskOutbox;
use crate::database::schema::unified::{execution_events, task_executions, task_outbox};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::ExecutionEventType;
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Marks a task execution as completed.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    ///
    /// When `runner_id` is `Some`, the update is guarded by a `WHERE claimed_by = runner_id`
    /// filter. Returns `false` if the claim was lost (another runner owns the task).
    /// When `runner_id` is `None`, no claim guard is applied (for non-claiming callers).
    pub async fn mark_completed(
        &self,
        task_id: UniversalUuid,
        runner_id: Option<UniversalUuid>,
    ) -> Result<bool, ValidationError> {
        use diesel::connection::Connection;

        let applied = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<bool, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status — guarded by claimed_by when runner_id provided
                let set_clause = (
                    task_executions::status.eq("Completed"),
                    task_executions::completed_at.eq(Some(now)),
                    task_executions::updated_at.eq(now),
                );
                let rows = if let Some(rid) = runner_id {
                    diesel::update(
                        task_executions::table
                            .find(task_id)
                            .filter(task_executions::claimed_by.eq(Some(rid))),
                    )
                    .set(set_clause)
                    .execute(conn)?
                } else {
                    diesel::update(task_executions::table.find(task_id))
                        .set(set_clause)
                        .execute(conn)?
                };

                // Only insert event if the update was applied
                if rows > 0 {
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: task.workflow_execution_id,
                        task_execution_id: Some(task_id),
                        event_type: ExecutionEventType::TaskCompleted.as_str().to_string(),
                        event_data: None,
                        worker_id: None,
                        created_at: now,
                        request_id: None,
                        runner_id: None,
                        tenant_id: None,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;
                }

                Ok(rows > 0)
            })
        })?;

        Ok(applied)
    }

    /// Marks a task execution as failed with an error message.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    ///
    /// When `runner_id` is `Some`, the update is guarded by a `WHERE claimed_by = runner_id`
    /// filter. Returns `false` if the claim was lost (another runner owns the task).
    /// When `runner_id` is `None`, no claim guard is applied (for non-claiming callers).
    pub async fn mark_failed(
        &self,
        task_id: UniversalUuid,
        error_message: &str,
        runner_id: Option<UniversalUuid>,
    ) -> Result<bool, ValidationError> {
        use diesel::connection::Connection;

        let error_message = error_message.to_string();
        let applied = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<bool, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status — guarded by claimed_by when runner_id provided
                let set_clause = (
                    task_executions::status.eq("Failed"),
                    task_executions::completed_at.eq(Some(now)),
                    task_executions::last_error.eq(&error_message),
                    task_executions::updated_at.eq(now),
                );
                let rows = if let Some(rid) = runner_id {
                    diesel::update(
                        task_executions::table
                            .find(task_id)
                            .filter(task_executions::claimed_by.eq(Some(rid))),
                    )
                    .set(set_clause)
                    .execute(conn)?
                } else {
                    diesel::update(task_executions::table.find(task_id))
                        .set(set_clause)
                        .execute(conn)?
                };

                // Only insert event if the update was applied
                if rows > 0 {
                    let event_data = serde_json::json!({ "error": error_message }).to_string();
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: task.workflow_execution_id,
                        task_execution_id: Some(task_id),
                        event_type: ExecutionEventType::TaskFailed.as_str().to_string(),
                        event_data: Some(event_data),
                        worker_id: None,
                        created_at: now,
                        request_id: None,
                        runner_id: None,
                        tenant_id: None,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;
                }

                Ok(rows > 0)
            })
        })?;

        Ok(applied)
    }

    /// Marks a task as ready for execution.
    ///
    /// This operation is transactional: the status update, execution event,
    /// and outbox entry are written atomically. If any fail, all are rolled back.
    ///
    /// The outbox entry enables push-based work distribution (Postgres LISTEN/NOTIFY)
    /// or polling-based distribution (SQLite).
    pub async fn mark_ready(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        // KEPT AS EXPLICIT TWINS (CLOACI-I-0135): the outbox-insert bodies genuinely
        // diverge. Postgres lets the DB stamp `created_at` via
        // `DEFAULT CURRENT_TIMESTAMP` (so the claim filter `created_at <= NOW()` and
        // the write both source the DB clock, avoiding app/DB clock skew); SQLite
        // writes `created_at = now` from the app clock. This is not a backend-agnostic
        // body, so it does not collapse to `interact_on_backend!`.
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_ready_postgres(task_id).await,
            self.mark_ready_sqlite(task_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_ready_postgres(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Ready"),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: task.workflow_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskMarkedReady.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                // Insert outbox entry for work distribution. Let the DB stamp
                // created_at via its `DEFAULT CURRENT_TIMESTAMP` instead of the
                // app clock: claim_ready_task (postgres) filters
                // `created_at <= NOW()` using the DB clock, so an app-side
                // timestamp makes a fresh row look future-dated whenever the app
                // and DB clocks diverge (Docker VM drift, or a non-UTC session
                // TZ applied to the naive TIMESTAMP column) — and the task is
                // never claimed. Sourcing both write and filter from the DB
                // clock removes the skew. (schedule_retry still sets created_at
                // = retry_at on purpose — an intentional future delay.)
                diesel::insert_into(task_outbox::table)
                    .values(task_outbox::task_execution_id.eq(task_id))
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::debug!(task_id = %task_id, "Task marked as Ready with outbox entry");
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn mark_ready_sqlite(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Ready"),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: task.workflow_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskMarkedReady.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                // Insert outbox entry for work distribution
                let outbox_entry = NewUnifiedTaskOutbox {
                    task_execution_id: task_id,
                    created_at: now,
                };
                diesel::insert_into(task_outbox::table)
                    .values(&outbox_entry)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::debug!(task_id = %task_id, "Task marked as Ready with outbox entry");
        Ok(())
    }

    /// Stamps a task's `started_at` (and flips it to `Running`) at the moment
    /// execution begins, idempotently.
    ///
    /// The distributed/claiming path stamps `started_at` inside
    /// `claim_ready_task`/`claim_for_runner`, but the embedded single-runner
    /// path executes with claiming disabled and never went through a claim — so
    /// `started_at` stayed NULL and the per-task timeline (the Gantt view) had
    /// no real start offset. This is called from the executor before a task
    /// runs; the `started_at IS NULL` guard makes it a no-op when a claim
    /// already stamped it, so the two paths don't fight. Best-effort: failures
    /// are logged by the caller, not fatal to execution.
    pub async fn mark_started(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        crate::interact_on_backend!(self.dal, |conn| {
            let now = UniversalTimestamp::now();
            diesel::update(
                task_executions::table
                    .filter(task_executions::id.eq(task_id))
                    .filter(task_executions::started_at.is_null()),
            )
            .set((
                task_executions::status.eq("Running"),
                task_executions::started_at.eq(Some(now)),
                task_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        Ok(())
    }

    /// Marks a task as skipped with a provided reason.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn mark_skipped(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let reason = reason.to_string();
        let reason_log = reason.clone();
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Skipped"),
                        task_executions::error_details.eq(&reason),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with skip reason
                let event_data = serde_json::json!({ "reason": reason }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: task.workflow_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskSkipped.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })?;

        tracing::info!(task_id = %task_id, reason = %reason_log, "Task marked as Skipped");
        Ok(())
    }

    /// Marks a task as permanently abandoned after too many recovery attempts.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn mark_abandoned(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let reason = reason.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Failed"),
                        task_executions::completed_at.eq(Some(now)),
                        task_executions::error_details.eq(format!("ABANDONED: {}", reason)),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with abandonment reason
                let event_data = serde_json::json!({ "reason": reason }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: task.workflow_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskAbandoned.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })?;

        Ok(())
    }

    /// Updates the sub_status of a running task execution.
    ///
    /// Valid values: `Some("Active")`, `Some("Deferred")`, or `None` to clear.
    ///
    /// This operation is transactional when transitioning to/from Deferred state.
    pub async fn set_sub_status(
        &self,
        task_id: UniversalUuid,
        sub_status: Option<&str>,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let sub_status_owned = sub_status.map(|s| s.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event (need to check previous sub_status)
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;
                let was_deferred = task.sub_status.as_deref() == Some("Deferred");

                // Update task sub_status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::sub_status.eq(&sub_status_owned),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Emit event only for deferred/resumed transitions
                let event_type = match (was_deferred, sub_status_owned.as_deref()) {
                    (false, Some("Deferred")) => Some(ExecutionEventType::TaskDeferred),
                    (true, Some("Active") | None) => Some(ExecutionEventType::TaskResumed),
                    _ => None,
                };

                if let Some(event_type) = event_type {
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: task.workflow_execution_id,
                        task_execution_id: Some(task_id),
                        event_type: event_type.as_str().to_string(),
                        event_data: None,
                        worker_id: None,
                        created_at: now,
                        request_id: None,
                        runner_id: None,
                        tenant_id: None,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;
                }

                Ok(())
            })
        })?;

        Ok(())
    }

    /// Resets the retry state for a task to its initial state.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn reset_retry_state(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Reset task state
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::attempt.eq(1),
                        task_executions::retry_at.eq(None::<UniversalTimestamp>),
                        task_executions::started_at.eq(None::<UniversalTimestamp>),
                        task_executions::completed_at.eq(None::<UniversalTimestamp>),
                        task_executions::last_error.eq(None::<String>),
                        task_executions::status.eq("Ready"),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: task.workflow_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskReset.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })?;

        Ok(())
    }
}
