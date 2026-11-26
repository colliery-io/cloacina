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

//! Unified Workflow Execution DAL with compile-time backend selection
//!
//! All state transitions are transactional: the status update and execution event
//! are written atomically. If either fails, both are rolled back.

use super::DAL;
use super::models::{
    NewUnifiedExecutionEvent, NewUnifiedWorkflowExecution, UnifiedWorkflowExecution,
};
use crate::database::schema::unified::{delivery_outbox, execution_events, workflow_executions};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::ExecutionEventType;
use crate::models::workflow_execution::{NewWorkflowExecution, WorkflowExecutionRecord};
use crate::run_alerts::{self, RunOutcome};
use diesel::prelude::*;

/// Filter for `WorkflowExecutionDAL::list_filtered`. CLOACI-T-0594 /
/// API-02: closes the silent-filter-drop bug where the REST route's
/// `--status` / `--workflow_name` query params were discarded.
#[derive(Debug, Clone, Default)]
pub struct ExecutionListFilter {
    /// Filter by status (e.g. `Pending`, `Running`, `Completed`, `Failed`).
    /// `None` means no status filter.
    pub status: Option<String>,
    /// Filter by exact workflow name. `None` means no name filter.
    pub workflow_name: Option<String>,
    /// SQL `LIMIT`. Caller is responsible for bounding (route validates
    /// `limit <= 1000`); the DAL trusts whatever is passed.
    pub limit: i64,
    /// SQL `OFFSET`.
    pub offset: i64,
}

/// Data access layer for workflow execution operations with compile-time backend selection.
#[derive(Clone)]
pub struct WorkflowExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> WorkflowExecutionDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new workflow execution record in the database.
    ///
    /// This operation is transactional: the workflow record and execution event
    /// are written atomically.
    pub async fn create(
        &self,
        new_execution: NewWorkflowExecution,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        use diesel::connection::Connection;

        let execution: UnifiedWorkflowExecution = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let id = UniversalUuid::new_v4();
                let now = UniversalTimestamp::now();

                let unified_new = NewUnifiedWorkflowExecution {
                    id,
                    workflow_name: new_execution.workflow_name,
                    workflow_version: new_execution.workflow_version,
                    status: new_execution.status,
                    context_id: new_execution.context_id,
                    started_at: now,
                    created_at: now,
                    updated_at: now,
                };

                // Insert workflow record
                diesel::insert_into(workflow_executions::table)
                    .values(&unified_new)
                    .execute(conn)?;

                // Retrieve the created record
                let execution: UnifiedWorkflowExecution =
                    workflow_executions::table.find(id).first(conn)?;

                // Insert execution event for workflow start
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: execution.id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::WorkflowStarted.as_str().to_string(),
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

                Ok(execution)
            })
        })?;

        Ok(execution.into())
    }

    pub async fn get_by_id(
        &self,
        id: UniversalUuid,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        let execution: UnifiedWorkflowExecution = crate::interact_on_backend!(self.dal, |conn| {
            workflow_executions::table.find(id).first(conn)
        })?;

        Ok(execution.into())
    }

    pub async fn get_active_executions(
        &self,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let executions: Vec<UnifiedWorkflowExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                workflow_executions::table
                    .filter(workflow_executions::status.eq_any(vec!["Pending", "Running"]))
                    .load(conn)
            })?;

        Ok(executions.into_iter().map(Into::into).collect())
    }

    pub async fn update_status(
        &self,
        id: UniversalUuid,
        status: &str,
    ) -> Result<(), ValidationError> {
        let status = status.to_string();
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::status.eq(status),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Set a workflow execution's trigger origin (CLOACI-T-0776). Called right
    /// after creation by the REST execute endpoint to mark an operator run as
    /// `"manual"`. Best-effort metadata — failures are non-fatal to the run.
    pub async fn set_trigger_origin(
        &self,
        id: UniversalUuid,
        origin: &str,
    ) -> Result<(), ValidationError> {
        let origin = origin.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(workflow_executions::table.find(id))
                .set(workflow_executions::trigger_origin.eq(origin))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Marks a workflow execution as completed.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    ///
    /// When the server switched on run-finished notices for completed runs
    /// ([`run_alerts`]), the same transaction also writes the notice row.
    pub async fn mark_completed(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        self.mark_completed_with_notice(id, run_alerts::is_enabled(RunOutcome::Completed))
            .await
    }

    async fn mark_completed_with_notice(
        &self,
        id: UniversalUuid,
        notice: bool,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let tenant = self.dal.database.schema().map(str::to_string);
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Only transition from Running to Completed — prevents duplicate
                // WorkflowCompleted events when two scheduler ticks race.
                let rows = diesel::update(
                    workflow_executions::table
                        .find(id)
                        .filter(workflow_executions::status.ne_all(vec!["Completed", "Failed"])),
                )
                .set((
                    workflow_executions::status.eq("Completed"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)?;

                // Only insert event if we actually transitioned
                if rows > 0 {
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::WorkflowCompleted.as_str().to_string(),
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
                    if notice {
                        diesel::insert_into(delivery_outbox::table)
                            .values(&run_alerts::outbox_row(
                                id,
                                RunOutcome::Completed,
                                tenant,
                                now,
                            ))
                            .execute(conn)?;
                    }
                }

                Ok(())
            })
        })?;

        Ok(())
    }

    pub async fn get_last_version(
        &self,
        workflow_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        let workflow_name = workflow_name.to_string();
        let version: Option<String> = crate::interact_on_backend!(self.dal, |conn| {
            workflow_executions::table
                .filter(workflow_executions::workflow_name.eq(workflow_name))
                .order(workflow_executions::started_at.desc())
                .select(workflow_executions::workflow_version)
                .first(conn)
                .optional()
        })?;

        Ok(version)
    }

    /// Marks a workflow execution as failed with an error reason.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    ///
    /// When the server switched on run-finished notices for failed runs
    /// ([`run_alerts`]), the same transaction also writes the notice row.
    pub async fn mark_failed(
        &self,
        id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        self.mark_failed_with_notice(id, reason, run_alerts::is_enabled(RunOutcome::Failed))
            .await
    }

    async fn mark_failed_with_notice(
        &self,
        id: UniversalUuid,
        reason: &str,
        notice: bool,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let reason = reason.to_string();
        let tenant = self.dal.database.schema().map(str::to_string);
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Only transition from Running to Failed — prevents duplicate
                // WorkflowFailed events when two scheduler ticks race.
                let rows = diesel::update(
                    workflow_executions::table
                        .find(id)
                        .filter(workflow_executions::status.ne_all(vec!["Completed", "Failed"])),
                )
                .set((
                    workflow_executions::status.eq("Failed"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::error_details.eq(&reason),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)?;

                // Only insert event if we actually transitioned
                if rows > 0 {
                    let event_data = serde_json::json!({ "reason": reason }).to_string();
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::WorkflowFailed.as_str().to_string(),
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
                    if notice {
                        diesel::insert_into(delivery_outbox::table)
                            .values(&run_alerts::outbox_row(id, RunOutcome::Failed, tenant, now))
                            .execute(conn)?;
                    }
                }

                Ok(())
            })
        })?;

        Ok(())
    }

    /// Marks a workflow execution as cancelled.
    ///
    /// When the server switched on run-finished notices for cancelled runs
    /// ([`run_alerts`]), the same transaction also writes the notice row, but
    /// only for a run that had not already ended.
    pub async fn cancel(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        self.cancel_with_notice(id, run_alerts::is_enabled(RunOutcome::Cancelled))
            .await
    }

    async fn cancel_with_notice(
        &self,
        id: UniversalUuid,
        notice: bool,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let now = UniversalTimestamp::now();
        let tenant = self.dal.database.schema().map(str::to_string);
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                // Read the status only when a notice may be written: with the
                // notices off this is the one UPDATE it always was.
                let was_open = if notice {
                    let status: Option<String> = workflow_executions::table
                        .find(id)
                        .select(workflow_executions::status)
                        .first(conn)
                        .optional()?;
                    status.is_some_and(|s| {
                        !matches!(s.as_str(), "Completed" | "Failed" | "Cancelled")
                    })
                } else {
                    false
                };

                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Cancelled"),
                        workflow_executions::completed_at.eq(Some(now)),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                if was_open {
                    diesel::insert_into(delivery_outbox::table)
                        .values(&run_alerts::outbox_row(
                            id,
                            RunOutcome::Cancelled,
                            tenant,
                            now,
                        ))
                        .execute(conn)?;
                }

                Ok(())
            })
        })?;

        Ok(())
    }

    /// Pauses a running workflow execution.
    ///
    /// Sets the workflow status to 'Paused', records the pause timestamp,
    /// and optionally stores a reason for the pause.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn pause(
        &self,
        id: UniversalUuid,
        reason: Option<&str>,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let reason = reason.map(|r| r.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Update workflow status
                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Paused"),
                        workflow_executions::paused_at.eq(Some(now)),
                        workflow_executions::pause_reason.eq(&reason),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with pause reason
                let event_data = reason.map(|r| serde_json::json!({ "reason": r }).to_string());
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::WorkflowPaused.as_str().to_string(),
                    event_data,
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

    /// Resumes a paused workflow execution.
    ///
    /// Sets the workflow status back to 'Running' and clears the pause metadata.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn resume(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Update workflow status
                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Running"),
                        workflow_executions::paused_at.eq(None::<UniversalTimestamp>),
                        workflow_executions::pause_reason.eq(None::<String>),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::WorkflowResumed.as_str().to_string(),
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

    pub async fn update_final_context(
        &self,
        id: UniversalUuid,
        final_context_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::context_id.eq(Some(final_context_id)),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    pub async fn list_recent(
        &self,
        limit: i64,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let executions: Vec<UnifiedWorkflowExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                workflow_executions::table
                    .order(workflow_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })?;

        Ok(executions.into_iter().map(Into::into).collect())
    }

    /// CLOACI-T-0594 / API-02: filtered list endpoint. Supports
    /// `status` (e.g. "Failed", "Running") and `workflow_name`
    /// filters with bounded pagination. Replaces the old route-level
    /// `get_active_executions()` call that silently ignored the query
    /// string.
    pub async fn list_filtered(
        &self,
        filter: ExecutionListFilter,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let executions: Vec<UnifiedWorkflowExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                let mut query = workflow_executions::table.into_boxed();
                if let Some(ref status) = filter.status {
                    query = query.filter(workflow_executions::status.eq(status.clone()));
                }
                if let Some(ref name) = filter.workflow_name {
                    query = query.filter(workflow_executions::workflow_name.eq(name.clone()));
                }
                query
                    .order(workflow_executions::started_at.desc())
                    .limit(filter.limit)
                    .offset(filter.offset)
                    .load(conn)
            })?;

        Ok(executions.into_iter().map(Into::into).collect())
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod run_alert_notice_tests {
    use super::*;
    use crate::database::Database;
    use crate::run_alerts::{KIND, RECIPIENT, RunFinishedNotice};

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:run_alert_notice_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    async fn running_execution(dal: &DAL) -> UniversalUuid {
        dal.workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "run-alert-notice-test".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id
    }

    async fn notices(dal: &DAL) -> Vec<RunFinishedNotice> {
        dal.delivery_outbox()
            .list_open_for_recipient(RECIPIENT, 10)
            .await
            .unwrap()
            .iter()
            .map(|row| {
                assert_eq!(row.kind, KIND);
                assert_eq!(row.delivery_state, "pending");
                RunFinishedNotice::from_bytes(&row.payload).unwrap()
            })
            .collect()
    }

    #[tokio::test]
    async fn run_alert_notice_written_once_when_a_run_fails() {
        let dal = unique_dal().await;
        let id = running_execution(&dal).await;

        dal.workflow_execution()
            .mark_failed_with_notice(id, "1 task(s) failed", true)
            .await
            .unwrap();
        // The second call changes no row, so it writes no second notice.
        dal.workflow_execution()
            .mark_failed_with_notice(id, "1 task(s) failed", true)
            .await
            .unwrap();

        let found = notices(&dal).await;
        assert_eq!(
            found,
            vec![RunFinishedNotice::new(id.0, RunOutcome::Failed)]
        );
    }

    #[tokio::test]
    async fn run_alert_notice_written_when_a_run_completes() {
        let dal = unique_dal().await;
        let id = running_execution(&dal).await;

        dal.workflow_execution()
            .mark_completed_with_notice(id, true)
            .await
            .unwrap();

        let found = notices(&dal).await;
        assert_eq!(
            found,
            vec![RunFinishedNotice::new(id.0, RunOutcome::Completed)]
        );
    }

    #[tokio::test]
    async fn run_alert_notice_written_for_a_cancel_of_an_open_run_only() {
        let dal = unique_dal().await;
        let open = running_execution(&dal).await;
        let ended = running_execution(&dal).await;
        dal.workflow_execution()
            .mark_failed_with_notice(ended, "failed first", false)
            .await
            .unwrap();

        dal.workflow_execution()
            .cancel_with_notice(open, true)
            .await
            .unwrap();
        dal.workflow_execution()
            .cancel_with_notice(ended, true)
            .await
            .unwrap();

        let found = notices(&dal).await;
        assert_eq!(
            found,
            vec![RunFinishedNotice::new(open.0, RunOutcome::Cancelled)]
        );
        let record = dal.workflow_execution().get_by_id(open).await.unwrap();
        assert_eq!(record.status, "Cancelled");
    }

    #[tokio::test]
    async fn run_alert_notice_not_written_when_off() {
        let dal = unique_dal().await;
        let failed = running_execution(&dal).await;
        let completed = running_execution(&dal).await;
        let cancelled = running_execution(&dal).await;

        dal.workflow_execution()
            .mark_failed_with_notice(failed, "1 task(s) failed", false)
            .await
            .unwrap();
        dal.workflow_execution()
            .mark_completed_with_notice(completed, false)
            .await
            .unwrap();
        dal.workflow_execution()
            .cancel_with_notice(cancelled, false)
            .await
            .unwrap();

        assert!(notices(&dal).await.is_empty());
        assert_eq!(dal.delivery_outbox().count_open().await.unwrap(), 0);
    }
}
