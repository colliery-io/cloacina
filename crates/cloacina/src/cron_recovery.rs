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

//! Cron execution recovery service for handling lost executions.
//!
//! This module provides a recovery mechanism that detects and retries cron executions
//! that were claimed but never successfully handed off to the workflow executor.
//! It implements the recovery side of the guaranteed execution pattern.
//!
//! # Architecture
//!
//! The recovery service runs as a background task that periodically:
//! 1. Queries for lost executions (claimed but no workflow execution)
//! 2. Determines if recovery is appropriate
//! 3. Retries the execution through the workflow executor
//! 4. Updates audit records to reflect recovery attempts
//!
//! # Recovery Policy
//!
//! Executions are considered "lost" if:
//! - They have a schedule_executions record (were claimed)
//! - They are NOT linked to a live workflow execution (the scheduler links
//!   the audit row to the workflow execution at handoff — CLOACI-T-0914)
//! - They were claimed more than X minutes ago (configurable)
//!
//! A row linked to a workflow execution in a non-terminal state is a
//! legitimately running workflow, not a lost handoff — it is skipped. A row
//! linked to a terminal execution that was never marked complete only has its
//! completion accounting backfilled; it is never re-fired.
//!
//! Recovery is skipped if:
//! - The schedule is disabled
//! - The schedule has been deleted
//! - Too many recovery attempts have been made
//! - The execution is too old (beyond recovery window)
//! - Another recovery service already owns the handoff
//!
//! # Ownership and accounting (CLOACI-T-0926)
//!
//! Both live on the `schedule_executions` row, not in this process:
//!
//! * `recovery_attempts` is the attempt cap. It used to be an in-process
//!   `HashMap`, so a schedule that reliably failed recovery got a fresh budget
//!   on every restart.
//! * `recovery_claimed_by` / `recovery_heartbeat_at` are a compare-and-set
//!   claim. `find_lost_executions` is a plain SELECT, so every replica sees
//!   the same lost rows; the claim is what makes exactly one of them re-fire.
//!   Because a re-fire blocks for the workflow's whole (unbounded) duration,
//!   the owner beats the claim while it works instead of relying on a fixed
//!   expiry window — a stale beat is then a true death signal, so a crashed
//!   recovery service's claim is taken over rather than held forever.

use crate::context::Context;
use crate::dal::unified::{RecoveryClaimResult, RecoveryHeartbeatResult};
use crate::dal::DAL;
use crate::database::UniversalUuid;
use crate::executor::{WorkflowExecutionError, WorkflowExecutor};
use crate::models::schedule::ScheduleExecution;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Configuration for the cron recovery service.
#[derive(Debug, Clone)]
pub struct CronRecoveryConfig {
    /// How often to check for lost executions
    pub check_interval: Duration,
    /// Consider executions lost if claimed more than this many minutes ago
    pub lost_threshold_minutes: i32,
    /// Maximum age of executions to recover (older ones are abandoned)
    pub max_recovery_age: Duration,
    /// Maximum number of recovery attempts per execution
    pub max_recovery_attempts: usize,
    /// Whether to recover executions for disabled schedules
    pub recover_disabled_schedules: bool,
    /// How often the owner of a recovery claim refreshes its heartbeat while
    /// the re-fire is in flight (CLOACI-T-0926). Must be comfortably smaller
    /// than `claim_stale_after`.
    pub claim_heartbeat_interval: Duration,
    /// How long a recovery claim may go without a heartbeat before another
    /// recovery service may take it over (CLOACI-T-0926).
    pub claim_stale_after: Duration,
}

impl Default for CronRecoveryConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(300), // 5 minutes
            lost_threshold_minutes: 10,
            max_recovery_age: Duration::from_secs(86400), // 24 hours
            max_recovery_attempts: 3,
            recover_disabled_schedules: false,
            claim_heartbeat_interval: Duration::from_secs(30),
            claim_stale_after: Duration::from_secs(120),
        }
    }
}

/// Recovery service for lost cron executions.
///
/// This service implements the recovery side of the guaranteed execution pattern,
/// detecting executions that were claimed but never handed off and retrying them.
#[derive(Clone)]
pub struct CronRecoveryService {
    dal: Arc<DAL>,
    executor: Arc<dyn WorkflowExecutor>,
    config: CronRecoveryConfig,
    shutdown: watch::Receiver<bool>,
    /// Identity this service writes into `schedule_executions.recovery_claimed_by`
    /// when it wins the CAS for a lost handoff (CLOACI-T-0926). Fresh per
    /// service instance, exactly like a runner id for task claiming — the
    /// durable state that must survive a restart is the attempt count, which
    /// lives on the row, not this.
    owner_id: UniversalUuid,
}

impl CronRecoveryService {
    /// Creates a new cron recovery service.
    ///
    /// # Arguments
    /// * `dal` - Data access layer for database operations
    /// * `executor` - Workflow executor for retrying executions
    /// * `config` - Recovery service configuration
    /// * `shutdown` - Shutdown signal receiver
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        config: CronRecoveryConfig,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
            owner_id: UniversalUuid::new_v4(),
        }
    }

    /// Creates a new recovery service with default configuration.
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self::new(dal, executor, CronRecoveryConfig::default(), shutdown)
    }

    /// Runs the recovery service loop.
    ///
    /// This method starts an infinite loop that periodically checks for and
    /// recovers lost executions until a shutdown signal is received.
    pub async fn run_recovery_loop(&mut self) -> Result<(), WorkflowExecutionError> {
        info!(
            "Starting cron recovery service (interval: {:?}, threshold: {} minutes)",
            self.config.check_interval, self.config.lost_threshold_minutes
        );

        let mut interval = tokio::time::interval(self.config.check_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.check_and_recover_lost_executions().await {
                        error!("Error in cron recovery service: {}", e);
                        // Continue running despite errors
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Cron recovery service received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Cron recovery service stopped");
        Ok(())
    }

    /// Runs one recovery pass over currently-lost executions.
    ///
    /// Exposed so integration tests (and ad-hoc operator scripts) can drive
    /// the recovery logic deterministically without waiting on the background
    /// `check_interval` tick (CLOACI-T-0914).
    pub async fn recover_lost_executions_once(&self) -> Result<(), WorkflowExecutionError> {
        self.check_and_recover_lost_executions().await
    }

    /// Checks for lost executions and attempts to recover them.
    async fn check_and_recover_lost_executions(&self) -> Result<(), WorkflowExecutionError> {
        debug!("Checking for lost cron executions");

        // Find lost executions
        let lost_executions = self
            .dal
            .schedule_execution()
            .find_lost_executions(self.config.lost_threshold_minutes)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to find lost executions: {}", e),
            })?;

        if lost_executions.is_empty() {
            debug!("No lost executions found");
            return Ok(());
        }

        info!("Found {} lost cron execution(s)", lost_executions.len());

        // Attempt to recover each lost execution
        for execution in lost_executions {
            if let Err(e) = self.recover_execution(&execution).await {
                error!(
                    "Failed to recover execution {} for schedule {}: {}",
                    execution.id, execution.schedule_id, e
                );
                // Continue with other executions
            }
        }

        Ok(())
    }

    /// Attempts to recover a single lost execution.
    async fn recover_execution(
        &self,
        execution: &ScheduleExecution,
    ) -> Result<(), WorkflowExecutionError> {
        // CLOACI-T-0914 finding 5: a linked execution is not a lost handoff.
        // The scheduler links the audit row to the workflow execution at
        // handoff, so consult the live workflow execution row before even
        // considering a re-fire. Re-firing while the workflow was
        // legitimately running (anything longer than lost_threshold_minutes)
        // was the cron duplicate-fire bug.
        if let Some(workflow_execution_id) = execution.workflow_execution_id {
            match self
                .dal
                .workflow_execution()
                .get_by_id(workflow_execution_id)
                .await
            {
                Ok(workflow_execution) => match workflow_execution.status.as_str() {
                    // Terminal set for workflow executions
                    // (mirrors WorkflowStatus::is_terminal).
                    "Completed" | "Failed" | "Cancelled" => {
                        // The workflow reached a terminal state but the audit
                        // row was never marked complete (crash between link
                        // and complete, or the scheduler's completion wait was
                        // interrupted). Backfill the completion accounting; do
                        // NOT re-fire — the execution already happened.
                        info!(
                            "Execution {} is linked to terminal workflow execution {} (status: {}); backfilling completion accounting instead of re-firing",
                            execution.id, workflow_execution_id, workflow_execution.status
                        );
                        if let Err(e) = self
                            .dal
                            .schedule_execution()
                            .complete(execution.id, Utc::now())
                            .await
                        {
                            warn!(
                                "Failed to backfill completion for execution {}: {}",
                                execution.id, e
                            );
                        }
                        // No attempt bookkeeping to clear: the count lives on
                        // the row, and the row just left the lost set for good.
                        return Ok(());
                    }
                    _ => {
                        debug!(
                            "Execution {} is linked to live workflow execution {} (status: {}); not lost, skipping recovery",
                            execution.id, workflow_execution_id, workflow_execution.status
                        );
                        return Ok(());
                    }
                },
                Err(e) => {
                    // Fail toward "no duplicate": if the linked execution row
                    // cannot be read, skip this pass and re-check on the next
                    // sweep rather than risking a duplicate fire.
                    warn!(
                        "Execution {} is linked to workflow execution {} but the row could not be read; skipping recovery this pass: {}",
                        execution.id, workflow_execution_id, e
                    );
                    return Ok(());
                }
            }
        }

        // Use scheduled_time if available; fall back to created_at
        let scheduled_time = execution
            .scheduled_time
            .as_ref()
            .map(|t| t.0)
            .unwrap_or(execution.created_at.0);

        let execution_age = Utc::now() - scheduled_time;

        // Check if execution is too old to recover. `from_std` returns
        // `Err` only when the duration overflows chrono's range (~290 yr);
        // treat that as "max recovery age is effectively infinite" rather
        // than panicking — a poisoned config value must not crash the
        // recovery loop. CLOACI-I-0110 / COR-06.
        let max_recovery_age = chrono::Duration::from_std(self.config.max_recovery_age)
            .unwrap_or_else(|e| {
                warn!(
                    "max_recovery_age out of chrono::Duration range ({:?}): {}; treating as MAX",
                    self.config.max_recovery_age, e
                );
                chrono::Duration::MAX
            });
        if execution_age > max_recovery_age {
            warn!(
                "Execution {} is too old to recover (age: {:?}), abandoning",
                execution.id, execution_age
            );
            return Ok(());
        }

        // CLOACI-T-0926: take exclusive ownership of this handoff before doing
        // anything that fires a workflow. Without the CAS, every replica's
        // recovery service saw the same row from `find_lost_executions` and
        // each one re-fired it.
        match self
            .dal
            .schedule_execution()
            .claim_for_recovery(execution.id, self.owner_id, self.config.claim_stale_after)
            .await
        {
            Ok(RecoveryClaimResult::Claimed) => {}
            Ok(RecoveryClaimResult::NotClaimed) => {
                debug!(
                    "Execution {} is owned by another recovery service (or already completed); skipping",
                    execution.id
                );
                return Ok(());
            }
            Err(e) => {
                // Fail toward "no duplicate", same as the unreadable-link case
                // above: if we cannot establish ownership, do not fire.
                warn!(
                    "Failed to claim execution {} for recovery; skipping this pass: {}",
                    execution.id, e
                );
                return Ok(());
            }
        }

        // From here on we own the row, so every exit path must release it.
        let outcome = self.recover_claimed_execution(execution).await;

        if let Err(e) = self
            .dal
            .schedule_execution()
            .release_recovery_claim(execution.id, self.owner_id)
            .await
        {
            // Not fatal: the claim heartbeat stops with this pass, so the claim
            // goes stale and the next sweep can take it over.
            warn!(
                "Failed to release recovery claim on execution {}: {}",
                execution.id, e
            );
        }

        outcome
    }

    /// Recovery body that runs while this service holds the row's recovery
    /// claim. Split out so `recover_execution` can release the claim on every
    /// exit path (CLOACI-T-0926).
    async fn recover_claimed_execution(
        &self,
        execution: &ScheduleExecution,
    ) -> Result<(), WorkflowExecutionError> {
        // Re-read under the claim. The row we were handed came from a SELECT
        // that may predate a concurrent winner's re-fire; if it has since been
        // linked, that winner already fired and this pass must not.
        match self.dal.schedule_execution().get_by_id(execution.id).await {
            Ok(fresh) => {
                if fresh.workflow_execution_id.is_some() || fresh.completed_at.is_some() {
                    debug!(
                        "Execution {} was linked/completed by another recovery pass; skipping",
                        execution.id
                    );
                    return Ok(());
                }
            }
            Err(e) => {
                warn!(
                    "Failed to re-read execution {} under its recovery claim; skipping: {}",
                    execution.id, e
                );
                return Ok(());
            }
        }

        // CLOACI-T-0926: the attempt count lives on the audit row, so a
        // schedule that reliably fails recovery cannot earn a fresh budget by
        // bouncing the process. Increment first, then compare — preserving the
        // pre-existing semantics where attempts 1..=max fire and the next one
        // abandons, and where a disabled schedule still consumes an attempt.
        let attempt_count = match self
            .dal
            .schedule_execution()
            .increment_recovery_attempts(execution.id)
            .await
        {
            Ok(count) => count,
            Err(e) => {
                // Fail toward "no duplicate": an unrecorded attempt is an
                // uncapped attempt.
                warn!(
                    "Failed to record a recovery attempt for execution {}; skipping this pass: {}",
                    execution.id, e
                );
                return Ok(());
            }
        };

        if attempt_count as usize > self.config.max_recovery_attempts {
            error!(
                "Execution {} has exceeded max recovery attempts ({}), abandoning",
                execution.id, self.config.max_recovery_attempts
            );
            return Ok(());
        }

        // `scheduled_time` was already resolved by the caller; recompute here
        // so the recovery context carries the same value.
        let scheduled_time = execution
            .scheduled_time
            .as_ref()
            .map(|t| t.0)
            .unwrap_or(execution.created_at.0);

        info!(
            "Attempting recovery of execution {} (schedule: {}, attempt: {}/{})",
            execution.id, execution.schedule_id, attempt_count, self.config.max_recovery_attempts
        );

        // Get the schedule to check if it's still active
        let schedule = match self.dal.schedule().get_by_id(execution.schedule_id).await {
            Ok(sched) => sched,
            Err(e) => {
                warn!(
                    "Schedule {} not found for execution {}, skipping recovery: {}",
                    execution.schedule_id, execution.id, e
                );
                return Ok(());
            }
        };

        // Check if schedule is enabled (unless configured to recover disabled schedules)
        if !self.config.recover_disabled_schedules && !schedule.enabled.is_true() {
            info!(
                "Schedule {} is disabled, skipping recovery of execution {}",
                schedule.id, execution.id
            );
            return Ok(());
        }

        // Create recovery context
        let mut context = Context::new();

        // Add recovery metadata
        context
            .insert("is_recovery", serde_json::json!(true))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("recovery_attempt", serde_json::json!(attempt_count))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "original_execution_id",
                serde_json::json!(execution.id.to_string()),
            )
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        // Add original scheduling metadata
        context
            .insert(
                "scheduled_time",
                serde_json::json!(scheduled_time.to_rfc3339()),
            )
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("schedule_id", serde_json::json!(schedule.id.to_string()))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "schedule_timezone",
                serde_json::json!(schedule.timezone.as_deref().unwrap_or("UTC")),
            )
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "schedule_expression",
                serde_json::json!(schedule.cron_expression.as_deref().unwrap_or("")),
            )
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        // Execute the workflow
        info!(
            "Executing recovery for workflow '{}' (execution: {}, schedule: {})",
            schedule.workflow_name, execution.id, schedule.id
        );

        // `execute` blocks for the workflow's full duration, which is
        // unbounded. Beat the claim while we wait so another recovery service
        // never mistakes a long re-fire for a dead one and steals the row
        // (CLOACI-T-0926). The guard aborts the beat on every exit path.
        let _heartbeat = self.spawn_claim_heartbeat(execution.id);

        match self
            .executor
            .execute(&schedule.workflow_name, context)
            .await
        {
            Ok(workflow_result) => {
                // Update the audit record with the new workflow execution ID
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .update_workflow_execution_id(
                        execution.id,
                        crate::database::UniversalUuid(workflow_result.execution_id),
                    )
                    .await
                {
                    error!(
                        "Failed to update audit record for recovered execution {}: {}",
                        execution.id, e
                    );
                    // Continue - the recovery succeeded, just audit update failed
                }

                // `execute` blocks until the workflow reaches a terminal
                // state, so completion accounting is accurate here. Without
                // this, the recovered row stayed `completed_at IS NULL` and
                // was re-found as lost on every subsequent sweep
                // (CLOACI-T-0914 finding 5).
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .complete(execution.id, Utc::now())
                    .await
                {
                    warn!(
                        "Failed to mark recovered execution {} complete: {}",
                        execution.id, e
                    );
                }

                info!(
                    "Successfully recovered execution {} (new workflow execution: {})",
                    execution.id, workflow_result.execution_id
                );

                // Clear recovery attempts on success. Durable equivalent of
                // the old in-memory `attempts.remove(...)`: the row is complete
                // so it will not be swept again, but leaving a spent budget on
                // it would misreport the handoff's history.
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .reset_recovery_attempts(execution.id)
                    .await
                {
                    warn!(
                        "Failed to reset recovery attempts for execution {}: {}",
                        execution.id, e
                    );
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to recover execution {} for workflow '{}': {}",
                    execution.id, schedule.workflow_name, e
                );
                Err(e)
            }
        }
    }

    /// Spawns the claim heartbeat for an in-flight re-fire.
    ///
    /// Returns a guard that aborts the beat when dropped, so the claim starts
    /// ageing the moment this recovery pass leaves the executor — whether it
    /// returned, errored, or the whole service was dropped.
    fn spawn_claim_heartbeat(&self, execution_id: UniversalUuid) -> ClaimHeartbeat {
        let dal = self.dal.clone();
        let owner_id = self.owner_id;
        let interval = self.config.claim_heartbeat_interval;

        ClaimHeartbeat(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // The claim was written moments ago; skip the immediate first tick.
            ticker.tick().await;

            loop {
                ticker.tick().await;
                match dal
                    .schedule_execution()
                    .recovery_heartbeat(execution_id, owner_id)
                    .await
                {
                    Ok(RecoveryHeartbeatResult::Ok) => {}
                    Ok(RecoveryHeartbeatResult::ClaimLost) => {
                        warn!(
                            "Recovery claim on execution {} was taken over while the re-fire was still running",
                            execution_id
                        );
                        break;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to beat the recovery claim on execution {}: {}",
                            execution_id, e
                        );
                    }
                }
            }
        }))
    }

    /// Clears the durable recovery attempt count for one execution.
    ///
    /// Useful for testing, or for an operator who wants to give a previously
    /// abandoned handoff a fresh budget. CLOACI-T-0926 moved the count from
    /// process memory onto the audit row, so this now takes the execution to
    /// reset rather than wiping a process-local cache.
    pub async fn clear_recovery_attempts(
        &self,
        execution_id: UniversalUuid,
    ) -> Result<(), crate::error::ValidationError> {
        self.dal
            .schedule_execution()
            .reset_recovery_attempts(execution_id)
            .await?;
        info!("Cleared recovery attempts for execution {}", execution_id);
        Ok(())
    }

    /// Gets the durable recovery attempt count for an execution.
    ///
    /// Returns 0 if the row cannot be read; callers use this for reporting,
    /// never as the cap check (the cap reads the value inside the same
    /// claimed critical section that increments it).
    pub async fn get_recovery_attempts(&self, execution_id: UniversalUuid) -> usize {
        match self.dal.schedule_execution().get_by_id(execution_id).await {
            Ok(row) => row.recovery_attempts.max(0) as usize,
            Err(e) => {
                warn!(
                    "Failed to read recovery attempts for execution {}: {}",
                    execution_id, e
                );
                0
            }
        }
    }
}

/// Abort-on-drop handle for a recovery claim heartbeat task (CLOACI-T-0926).
struct ClaimHeartbeat(tokio::task::JoinHandle<()>);

impl Drop for ClaimHeartbeat {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_config_default() {
        let config = CronRecoveryConfig::default();
        assert_eq!(config.check_interval, Duration::from_secs(300));
        assert_eq!(config.lost_threshold_minutes, 10);
        assert_eq!(config.max_recovery_age, Duration::from_secs(86400));
        assert_eq!(config.max_recovery_attempts, 3);
        assert!(!config.recover_disabled_schedules);
        // The heartbeat must beat several times inside the staleness window,
        // or a healthy owner would look dead (CLOACI-T-0926).
        assert!(config.claim_heartbeat_interval < config.claim_stale_after);
        assert_eq!(config.claim_heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.claim_stale_after, Duration::from_secs(120));
    }

    #[test]
    fn test_recovery_config_custom() {
        let config = CronRecoveryConfig {
            check_interval: Duration::from_secs(60),
            lost_threshold_minutes: 5,
            max_recovery_age: Duration::from_secs(3600),
            max_recovery_attempts: 5,
            recover_disabled_schedules: true,
            claim_heartbeat_interval: Duration::from_secs(5),
            claim_stale_after: Duration::from_secs(20),
        };

        assert_eq!(config.check_interval, Duration::from_secs(60));
        assert_eq!(config.lost_threshold_minutes, 5);
        assert_eq!(config.max_recovery_age, Duration::from_secs(3600));
        assert_eq!(config.max_recovery_attempts, 5);
        assert!(config.recover_disabled_schedules);
    }

    #[test]
    fn test_recovery_config_clone() {
        let config = CronRecoveryConfig::default();
        let cloned = config.clone();
        assert_eq!(config.check_interval, cloned.check_interval);
        assert_eq!(config.lost_threshold_minutes, cloned.lost_threshold_minutes);
        assert_eq!(config.max_recovery_attempts, cloned.max_recovery_attempts);
    }

    #[test]
    fn test_recovery_config_default_recovery_window() {
        let config = CronRecoveryConfig::default();
        // Default max_recovery_age is 24 hours
        assert_eq!(config.max_recovery_age.as_secs(), 86400);
        // Default check interval is 5 minutes
        assert_eq!(config.check_interval.as_secs(), 300);
    }
}
