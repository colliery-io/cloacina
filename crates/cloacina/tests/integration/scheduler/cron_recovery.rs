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

//! Regression tests for CLOACI-T-0914 finding 5: the cron recovery service
//! must not duplicate-fire a schedule whose workflow is still legitimately
//! running past the lost threshold.
//!
//! Before the fix, the scheduler linked the `schedule_executions` audit row
//! to its workflow execution only AFTER the (blocking) handoff completed, and
//! `find_lost_executions` selected on `completed_at IS NULL` alone — so any
//! workflow running longer than `lost_threshold_minutes` (default 10) looked
//! like a lost handoff and was re-fired by `CronRecoveryService`.
//!
//! The fix links the audit row at handoff and makes recovery consult the
//! linked workflow execution's live status: non-terminal = not lost (skip),
//! terminal-but-unaccounted = backfill completion (no re-fire). Only
//! genuinely unlinked handoffs are re-fired.

use crate::fixtures::get_or_init_fixture;
use async_trait::async_trait;
use chrono::Utc;
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::executor::workflow_executor::{
    StatusCallback, WorkflowExecution, WorkflowExecutionError, WorkflowExecutionResult,
    WorkflowExecutor, WorkflowStatus,
};
use cloacina::models::schedule::{NewSchedule, NewScheduleExecution};
use cloacina::models::workflow_execution::NewWorkflowExecution;
use cloacina::{Context, CronRecoveryConfig, CronRecoveryService};
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use uuid::Uuid;

/// Counting mock executor for the recovery service. `execute` records the
/// fire and creates a REAL `workflow_executions` row (status Completed) so
/// the recovery service's audit-link write satisfies the FK constraint.
/// The remaining trait methods are never called by `CronRecoveryService`.
struct CountingExecutor {
    dal: cloacina::dal::DAL,
    execute_calls: AtomicUsize,
}

impl CountingExecutor {
    fn new(dal: cloacina::dal::DAL) -> Self {
        Self {
            dal,
            execute_calls: AtomicUsize::new(0),
        }
    }

    fn fires(&self) -> usize {
        self.execute_calls.load(Ordering::SeqCst)
    }
}

fn not_used<T>(method: &str) -> Result<T, WorkflowExecutionError> {
    Err(WorkflowExecutionError::ExecutionFailed {
        message: format!("CountingExecutor: {} not expected in this test", method),
    })
}

#[async_trait]
impl WorkflowExecutor for CountingExecutor {
    async fn execute(
        &self,
        workflow_name: &str,
        _context: Context<serde_json::Value>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        self.execute_calls.fetch_add(1, Ordering::SeqCst);

        let row = self
            .dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: workflow_name.to_string(),
                workflow_version: "1.0".to_string(),
                status: "Completed".to_string(),
                context_id: None,
            })
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("mock execute: failed to create workflow execution: {}", e),
            })?;

        Ok(WorkflowExecutionResult {
            execution_id: row.id.0,
            workflow_name: workflow_name.to_string(),
            status: WorkflowStatus::Completed,
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration: None,
            final_context: Context::new(),
            task_results: vec![],
            error_message: None,
        })
    }

    async fn execute_async(
        &self,
        _workflow_name: &str,
        _context: Context<serde_json::Value>,
    ) -> Result<WorkflowExecution, WorkflowExecutionError> {
        not_used("execute_async")
    }

    async fn get_execution_status(
        &self,
        _execution_id: Uuid,
    ) -> Result<WorkflowStatus, WorkflowExecutionError> {
        not_used("get_execution_status")
    }

    async fn get_execution_result(
        &self,
        _execution_id: Uuid,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        not_used("get_execution_result")
    }

    async fn cancel_execution(&self, _execution_id: Uuid) -> Result<(), WorkflowExecutionError> {
        not_used("cancel_execution")
    }

    async fn pause_execution(
        &self,
        _execution_id: Uuid,
        _reason: Option<&str>,
    ) -> Result<(), WorkflowExecutionError> {
        not_used("pause_execution")
    }

    async fn resume_execution(&self, _execution_id: Uuid) -> Result<(), WorkflowExecutionError> {
        not_used("resume_execution")
    }

    async fn execute_with_callback(
        &self,
        _workflow_name: &str,
        _context: Context<serde_json::Value>,
        _callback: Box<dyn StatusCallback>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        not_used("execute_with_callback")
    }

    async fn list_executions(
        &self,
    ) -> Result<Vec<WorkflowExecutionResult>, WorkflowExecutionError> {
        not_used("list_executions")
    }

    async fn shutdown(&self) -> Result<(), WorkflowExecutionError> {
        Ok(())
    }
}

/// Recovery config whose lost threshold lies in the future (negative
/// minutes), so freshly created audit rows are "aged past the threshold"
/// without wall-clock games — the same technique as the T-0572 regression
/// test.
fn aged_past_threshold_config() -> CronRecoveryConfig {
    CronRecoveryConfig {
        check_interval: Duration::from_secs(300),
        lost_threshold_minutes: -1,
        max_recovery_age: Duration::from_secs(86400),
        max_recovery_attempts: 3,
        recover_disabled_schedules: false,
    }
}

/// CLOACI-T-0914 finding 5 regression: an audit row linked at handoff to a
/// workflow execution held in `Running` must NOT be re-fired by a recovery
/// pass, no matter how far past the lost threshold it is. Once the linked
/// execution reaches a terminal state, recovery backfills the completion
/// accounting — still without re-firing.
#[tokio::test]
#[serial]
async fn test_long_running_cron_workflow_not_duplicate_fired() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;
    let dal = fixture.get_dal();

    // Schedule whose workflow "runs long".
    let schedule = dal
        .schedule()
        .create(NewSchedule::cron(
            "t0914-long-running-wf",
            "*/15 * * * *",
            UniversalTimestamp(Utc::now()),
        ))
        .await
        .expect("create schedule");

    // The workflow execution the scheduler handed off — held in Running.
    let wf_exec = dal
        .workflow_execution()
        .create(NewWorkflowExecution {
            workflow_name: "t0914-long-running-wf".to_string(),
            workflow_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("create running workflow execution");

    // The cron audit row, linked AT HANDOFF (what the fixed scheduler does).
    let audit = dal
        .schedule_execution()
        .create(NewScheduleExecution {
            schedule_id: schedule.id,
            workflow_execution_id: None,
            scheduled_time: Some(UniversalTimestamp(Utc::now())),
            claimed_at: Some(UniversalTimestamp(Utc::now())),
            context_hash: None,
        })
        .await
        .expect("create schedule_execution");
    dal.schedule_execution()
        .update_workflow_execution_id(audit.id, wf_exec.id)
        .await
        .expect("link audit row at handoff");

    // Recovery service with the row aged past the lost threshold.
    let executor = Arc::new(CountingExecutor::new(fixture.get_dal()));
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    let recovery = CronRecoveryService::new(
        Arc::new(fixture.get_dal()),
        executor.clone(),
        aged_past_threshold_config(),
        shutdown_rx,
    );

    // Recovery pass while the workflow is live: linked + non-terminal = not
    // lost. No re-fire.
    recovery
        .recover_lost_executions_once()
        .await
        .expect("recovery pass");

    assert_eq!(
        executor.fires(),
        0,
        "recovery must not re-fire a schedule whose linked workflow execution is Running"
    );
    let executions = dal
        .schedule_execution()
        .list_by_schedule(schedule.id, 100, 0)
        .await
        .expect("list schedule executions");
    assert_eq!(
        executions.len(),
        1,
        "exactly one cron execution must exist — no second fire"
    );
    let row = &executions[0];
    assert_eq!(row.workflow_execution_id, Some(wf_exec.id));
    assert!(
        row.completed_at.is_none(),
        "the audit row stays open while the workflow is live"
    );

    // The workflow finishes, but the completion accounting write was lost
    // (crash between link and complete). Recovery must backfill completion —
    // still without re-firing.
    dal.workflow_execution()
        .update_status(wf_exec.id, "Completed")
        .await
        .expect("mark workflow execution completed");

    recovery
        .recover_lost_executions_once()
        .await
        .expect("second recovery pass");

    assert_eq!(
        executor.fires(),
        0,
        "a linked terminal execution must be accounted, never re-fired"
    );
    let row = dal
        .schedule_execution()
        .get_by_id(audit.id)
        .await
        .expect("re-read audit row");
    assert!(
        row.completed_at.is_some(),
        "recovery must backfill completion accounting for a linked terminal execution"
    );
}

/// Companion: a genuinely-lost handoff — claimed audit row, never linked to
/// any workflow execution — IS re-fired by the recovery pass, and the re-fire
/// links + completes the audit row so subsequent passes do not fire again.
#[tokio::test]
#[serial]
async fn test_genuinely_lost_cron_handoff_is_refired() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;
    let dal = fixture.get_dal();

    let schedule = dal
        .schedule()
        .create(NewSchedule::cron(
            "t0914-lost-handoff-wf",
            "*/15 * * * *",
            UniversalTimestamp(Utc::now()),
        ))
        .await
        .expect("create schedule");

    // Claimed but never handed off: no workflow_execution_id, no completion.
    let audit = dal
        .schedule_execution()
        .create(NewScheduleExecution {
            schedule_id: schedule.id,
            workflow_execution_id: None,
            scheduled_time: Some(UniversalTimestamp(Utc::now())),
            claimed_at: Some(UniversalTimestamp(Utc::now())),
            context_hash: None,
        })
        .await
        .expect("create schedule_execution");

    let executor = Arc::new(CountingExecutor::new(fixture.get_dal()));
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    let recovery = CronRecoveryService::new(
        Arc::new(fixture.get_dal()),
        executor.clone(),
        aged_past_threshold_config(),
        shutdown_rx,
    );

    recovery
        .recover_lost_executions_once()
        .await
        .expect("recovery pass");

    assert_eq!(
        executor.fires(),
        1,
        "a genuinely-lost (unlinked) handoff past the threshold must be re-fired"
    );

    let row = dal
        .schedule_execution()
        .get_by_id(audit.id)
        .await
        .expect("re-read audit row");
    assert!(
        row.workflow_execution_id.is_some(),
        "re-fire must link the audit row to the new workflow execution"
    );
    assert!(
        row.completed_at.is_some(),
        "re-fire (which blocks to completion) must complete the audit row"
    );

    // A second pass must not fire again — the row left the lost set.
    recovery
        .recover_lost_executions_once()
        .await
        .expect("second recovery pass");
    assert_eq!(
        executor.fires(),
        1,
        "a recovered execution must not be re-fired on subsequent sweeps"
    );
}
