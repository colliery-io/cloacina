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
///
/// The counter is shared (`Arc`) so two recovery services can be pointed at
/// one tally — that is how the CLOACI-T-0926 concurrency test proves a lost
/// handoff is re-fired exactly once across replicas.
struct CountingExecutor {
    dal: cloacina::dal::DAL,
    execute_calls: Arc<AtomicUsize>,
    /// Held inside `execute` so a second recovery service gets a turn while
    /// the first is mid-re-fire — the window the claim has to close.
    delay: Duration,
    /// When true, `execute` counts the fire and then fails, leaving the audit
    /// row lost so the next sweep spends another recovery attempt.
    fail: bool,
}

impl CountingExecutor {
    fn new(dal: cloacina::dal::DAL) -> Self {
        Self {
            dal,
            execute_calls: Arc::new(AtomicUsize::new(0)),
            delay: Duration::ZERO,
            fail: false,
        }
    }

    fn with_counter(dal: cloacina::dal::DAL, counter: Arc<AtomicUsize>) -> Self {
        Self {
            execute_calls: counter,
            ..Self::new(dal)
        }
    }

    fn failing(dal: cloacina::dal::DAL) -> Self {
        Self {
            fail: true,
            ..Self::new(dal)
        }
    }

    fn delayed(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
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

        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }

        if self.fail {
            return Err(WorkflowExecutionError::ExecutionFailed {
                message: "mock execute: deliberate recovery failure".to_string(),
            });
        }

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
        // Long enough that no test ever mistakes a live claim for a dead one.
        claim_heartbeat_interval: Duration::from_secs(30),
        claim_stale_after: Duration::from_secs(120),
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

/// Helper: a lost cron handoff — claimed audit row, never linked, never
/// completed — for a freshly created schedule. Returns (schedule id, audit id).
async fn lost_handoff(
    dal: &cloacina::dal::DAL,
    workflow_name: &str,
) -> (
    cloacina::database::universal_types::UniversalUuid,
    cloacina::database::universal_types::UniversalUuid,
) {
    let schedule = dal
        .schedule()
        .create(NewSchedule::cron(
            workflow_name,
            "*/15 * * * *",
            UniversalTimestamp(Utc::now()),
        ))
        .await
        .expect("create schedule");

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

    (schedule.id, audit.id)
}

/// CLOACI-T-0926 item 1: the recovery attempt cap must survive a restart.
///
/// The cap used to be an `Arc<Mutex<HashMap<..>>>` on `CronRecoveryService`,
/// so a schedule that reliably failed recovery got a fresh budget every time
/// the process bounced — "max 3 attempts" was really "max 3 per process".
/// The count now lives on the audit row: a brand-new service instance against
/// the same database sees the spent budget and abandons instead of re-firing.
#[tokio::test]
#[serial]
async fn test_recovery_attempt_cap_survives_restart() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;
    let dal = fixture.get_dal();

    let (_schedule_id, audit_id) = lost_handoff(&dal, "t0926-cap-survives-restart").await;

    let config = aged_past_threshold_config();
    let max_attempts = config.max_recovery_attempts;

    // ── Process 1: burn the whole budget on a re-fire that keeps failing.
    let executor1 = Arc::new(CountingExecutor::failing(fixture.get_dal()));
    let (_tx1, rx1) = watch::channel(false);
    let recovery1 = CronRecoveryService::new(
        Arc::new(fixture.get_dal()),
        executor1.clone(),
        config.clone(),
        rx1,
    );

    for _ in 0..max_attempts {
        // The failing executor makes each pass return Err from
        // `recover_execution`; `check_and_recover_lost_executions` logs and
        // keeps going, so the pass itself still succeeds.
        recovery1
            .recover_lost_executions_once()
            .await
            .expect("recovery pass");
    }

    assert_eq!(
        executor1.fires(),
        max_attempts,
        "the first process should spend exactly the configured budget"
    );

    let row = dal
        .schedule_execution()
        .get_by_id(audit_id)
        .await
        .expect("re-read audit row");
    assert_eq!(
        row.recovery_attempts, max_attempts as i32,
        "the attempt count must be persisted on the audit row, not in process memory"
    );
    assert!(
        row.recovery_claimed_by.is_none(),
        "a finished recovery pass must leave the claim released"
    );
    assert!(
        row.completed_at.is_none() && row.workflow_execution_id.is_none(),
        "a failed re-fire leaves the handoff lost — this is what makes it eligible again"
    );

    // ── Restart: a brand-new service instance, same database. Before
    // CLOACI-T-0926 this reset the budget to zero and re-fired.
    let executor2 = Arc::new(CountingExecutor::failing(fixture.get_dal()));
    let (_tx2, rx2) = watch::channel(false);
    let recovery2 = CronRecoveryService::new(
        Arc::new(fixture.get_dal()),
        executor2.clone(),
        config.clone(),
        rx2,
    );

    recovery2
        .recover_lost_executions_once()
        .await
        .expect("post-restart recovery pass");

    assert_eq!(
        executor2.fires(),
        0,
        "a restarted recovery service must not get a fresh attempt budget"
    );

    // The abandoned pass still records that it looked, so the budget can only
    // move in one direction.
    let row = dal
        .schedule_execution()
        .get_by_id(audit_id)
        .await
        .expect("re-read audit row after restart");
    assert!(
        row.recovery_attempts > max_attempts as i32,
        "the persisted count must keep climbing past the cap, never reset"
    );

    assert_eq!(
        recovery2.get_recovery_attempts(audit_id).await,
        row.recovery_attempts as usize,
        "the service must report the durable count, not a process-local one"
    );
}

/// CLOACI-T-0926 item 2: two recovery services against one database must
/// re-fire a lost handoff exactly once.
///
/// `find_lost_executions` is a plain SELECT, so both services see the same
/// row. The CAS claim on `schedule_executions.recovery_claimed_by` is what
/// decides ownership — mirroring `claim_for_runner` for task claiming and the
/// per-row CAS sweep in CLOACI-T-0916. The mock holds `execute` open long
/// enough that the loser is guaranteed to reach its claim attempt while the
/// winner is still mid-re-fire.
#[tokio::test]
#[serial]
async fn test_two_recovery_services_refire_lost_handoff_exactly_once() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;
    let dal = fixture.get_dal();

    let (schedule_id, audit_id) = lost_handoff(&dal, "t0926-concurrent-recovery").await;

    // One tally, two services — exactly as if two runners were sweeping.
    let fires = Arc::new(AtomicUsize::new(0));

    let executor_a = Arc::new(
        CountingExecutor::with_counter(fixture.get_dal(), fires.clone())
            .delayed(Duration::from_millis(250)),
    );
    let executor_b = Arc::new(
        CountingExecutor::with_counter(fixture.get_dal(), fires.clone())
            .delayed(Duration::from_millis(250)),
    );

    let (_tx_a, rx_a) = watch::channel(false);
    let (_tx_b, rx_b) = watch::channel(false);
    let recovery_a = CronRecoveryService::new(
        Arc::new(fixture.get_dal()),
        executor_a.clone(),
        aged_past_threshold_config(),
        rx_a,
    );
    let recovery_b = CronRecoveryService::new(
        Arc::new(fixture.get_dal()),
        executor_b.clone(),
        aged_past_threshold_config(),
        rx_b,
    );

    let (a, b) = tokio::join!(
        recovery_a.recover_lost_executions_once(),
        recovery_b.recover_lost_executions_once(),
    );
    a.expect("recovery pass A");
    b.expect("recovery pass B");

    assert_eq!(
        fires.load(Ordering::SeqCst),
        1,
        "two concurrent recovery services must re-fire a lost handoff exactly once"
    );

    let executions = dal
        .schedule_execution()
        .list_by_schedule(schedule_id, 100, 0)
        .await
        .expect("list schedule executions");
    assert_eq!(executions.len(), 1, "no second audit row may be created");

    let row = dal
        .schedule_execution()
        .get_by_id(audit_id)
        .await
        .expect("re-read audit row");
    assert!(
        row.workflow_execution_id.is_some(),
        "the winning service must link the audit row"
    );
    assert!(
        row.completed_at.is_some(),
        "the winning service must complete the audit row"
    );
    assert!(
        row.recovery_claimed_by.is_none(),
        "the winner must release its claim when the pass ends"
    );
    assert_eq!(
        row.recovery_attempts, 0,
        "a successful re-fire resets the durable attempt budget"
    );

    // A further sweep by either service must not fire again.
    recovery_b
        .recover_lost_executions_once()
        .await
        .expect("follow-up recovery pass");
    assert_eq!(
        fires.load(Ordering::SeqCst),
        1,
        "a recovered handoff must not be re-fired on subsequent sweeps"
    );
}

/// A crashed recovery service must not lock a handoff forever. Its claim is
/// only as good as its heartbeat: once the beat goes stale, the next sweep
/// takes the row over. Simulated by writing a claim with an ancient heartbeat
/// (what a crashed owner leaves behind) and running a service whose staleness
/// window has already elapsed.
#[tokio::test]
#[serial]
async fn test_stale_recovery_claim_is_taken_over() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;
    let dal = fixture.get_dal();

    let (_schedule_id, audit_id) = lost_handoff(&dal, "t0926-stale-claim-takeover").await;

    // A dead owner's claim: written, then never beaten again.
    let dead_owner = cloacina::database::universal_types::UniversalUuid::new_v4();
    assert!(
        matches!(
            dal.schedule_execution()
                .claim_for_recovery(audit_id, dead_owner, Duration::from_secs(120))
                .await
                .expect("dead owner claims"),
            cloacina::dal::unified::RecoveryClaimResult::Claimed
        ),
        "the first claimant wins"
    );

    // A live service with the same staleness window must NOT steal a fresh
    // claim — that is the guarantee that keeps a long re-fire safe.
    let executor = Arc::new(CountingExecutor::new(fixture.get_dal()));
    let (_tx, rx) = watch::channel(false);
    let recovery = CronRecoveryService::new(
        Arc::new(fixture.get_dal()),
        executor.clone(),
        aged_past_threshold_config(),
        rx,
    );
    recovery
        .recover_lost_executions_once()
        .await
        .expect("recovery pass against a live claim");
    assert_eq!(
        executor.fires(),
        0,
        "a live (recently beaten) claim must not be stolen"
    );

    // Now age the claim past the window: zero staleness makes any existing
    // heartbeat older than the cutoff, which is what a crashed owner looks
    // like after `claim_stale_after` has elapsed.
    let mut takeover_config = aged_past_threshold_config();
    takeover_config.claim_stale_after = Duration::ZERO;

    let executor2 = Arc::new(CountingExecutor::new(fixture.get_dal()));
    let (_tx2, rx2) = watch::channel(false);
    let recovery2 = CronRecoveryService::new(
        Arc::new(fixture.get_dal()),
        executor2.clone(),
        takeover_config,
        rx2,
    );
    recovery2
        .recover_lost_executions_once()
        .await
        .expect("recovery pass against a stale claim");

    assert_eq!(
        executor2.fires(),
        1,
        "a stale claim must be taken over so a crashed service cannot lock a handoff forever"
    );
}
