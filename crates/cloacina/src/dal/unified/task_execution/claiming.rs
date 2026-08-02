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

//! Task claiming and retry scheduling operations.
//!
//! All operations are transactional: state changes and execution events
//! are written atomically. If either fails, both are rolled back.

use super::{HeartbeatResult, RunnerClaimResult, StaleClaim, TaskExecutionDAL};
use crate::dal::unified::models::{NewUnifiedExecutionEvent, UnifiedTaskExecution};
use crate::database::schema::unified::{execution_events, task_executions};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::ExecutionEventType;
use crate::models::task_execution::TaskExecution;
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Updates a task's retry schedule with a new attempt count and retry time.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn schedule_retry(
        &self,
        task_id: UniversalUuid,
        retry_at: UniversalTimestamp,
        new_attempt: i32,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task retry state
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Ready"),
                        task_executions::attempt.eq(new_attempt),
                        task_executions::retry_at.eq(Some(retry_at)),
                        task_executions::started_at.eq(None::<UniversalTimestamp>),
                        task_executions::completed_at.eq(None::<UniversalTimestamp>),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with retry details
                let event_data = serde_json::json!({
                    "attempt": new_attempt,
                    "retry_at": retry_at.to_string()
                })
                .to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: task.workflow_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskRetryScheduled.as_str().to_string(),
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

    // ========================================================================
    // Runner-level claiming (for horizontal scaling)
    // ========================================================================

    /// Atomically claim a task for a specific runner.
    ///
    /// Only succeeds if `claimed_by` is currently NULL. Sets `claimed_by` to
    /// the runner's UUID and `heartbeat_at` to now.
    pub async fn claim_for_runner(
        &self,
        task_id: UniversalUuid,
        runner_id: UniversalUuid,
    ) -> Result<RunnerClaimResult, ValidationError> {
        let now = UniversalTimestamp::now();
        let rows_updated: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                task_executions::table
                    .find(task_id)
                    .filter(task_executions::claimed_by.is_null()),
            )
            .set((
                task_executions::claimed_by.eq(Some(runner_id)),
                task_executions::heartbeat_at.eq(Some(now)),
                task_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;

        Ok(if rows_updated > 0 {
            RunnerClaimResult::Claimed
        } else {
            RunnerClaimResult::AlreadyClaimed
        })
    }

    /// Update heartbeat for a claimed task.
    ///
    /// Only succeeds if `claimed_by` matches the given `runner_id`.
    /// Returns `ClaimLost` if another runner has taken over.
    pub async fn heartbeat(
        &self,
        task_id: UniversalUuid,
        runner_id: UniversalUuid,
    ) -> Result<HeartbeatResult, ValidationError> {
        let now = UniversalTimestamp::now();
        let rows_updated: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                task_executions::table
                    .find(task_id)
                    .filter(task_executions::claimed_by.eq(Some(runner_id))),
            )
            .set((
                task_executions::heartbeat_at.eq(Some(now)),
                task_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;

        Ok(if rows_updated > 0 {
            HeartbeatResult::Ok
        } else {
            HeartbeatResult::ClaimLost
        })
    }

    /// Release a runner's claim on a task (on completion or failure).
    ///
    /// Clears `claimed_by` and `heartbeat_at`.
    pub async fn release_runner_claim(
        &self,
        task_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::claimed_by.eq(None::<UniversalUuid>),
                    task_executions::heartbeat_at.eq(None::<UniversalTimestamp>),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Find tasks with stale claims (heartbeat older than threshold).
    ///
    /// Returns tasks where `claimed_by` is not NULL and `heartbeat_at` is
    /// older than `now - threshold`.
    ///
    /// Covers `Running` AND `Ready` rows (CLOACI-T-0914): the claim is taken
    /// while the task is still `Ready` (`claim_for_runner` — status flips to
    /// `Running` only at `mark_started`, after the semaphore wait), so a
    /// crash in that window used to leave a `Ready`+claimed row that neither
    /// this sweeper (then Running-only) nor dispatch (`claimed_by IS NULL`)
    /// would ever touch. The heartbeat starts at claim time, so a stale
    /// heartbeat is a true death signal in both states. Terminal rows can
    /// also retain a claim (crash after the terminal write, before release) —
    /// they must NOT be re-Readied, hence the explicit status list rather
    /// than claim-presence alone.
    pub async fn find_stale_claims(
        &self,
        threshold: std::time::Duration,
    ) -> Result<Vec<StaleClaim>, ValidationError> {
        let cutoff = UniversalTimestamp(
            chrono::Utc::now()
                - chrono::Duration::from_std(threshold).unwrap_or(chrono::Duration::seconds(60)),
        );

        let stale: Vec<UnifiedTaskExecution> = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::claimed_by.is_not_null())
                .filter(task_executions::heartbeat_at.lt(Some(cutoff)))
                .filter(task_executions::status.eq_any(vec!["Running", "Ready"]))
                .load(conn)
        })?;

        Ok(stale
            .into_iter()
            .filter_map(|t| {
                Some(StaleClaim {
                    task_id: t.id,
                    claimed_by: t.claimed_by?,
                    heartbeat_at: t.heartbeat_at?.0,
                    recovery_attempts: t.recovery_attempts,
                })
            })
            .collect())
    }

    /// Retrieves tasks that are ready for retry (retry_at time has passed).
    pub async fn get_ready_for_retry(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        let now = UniversalTimestamp::now();
        let ready_tasks: Vec<UnifiedTaskExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Ready"))
                    .filter(
                        task_executions::retry_at
                            .is_null()
                            .or(task_executions::retry_at.le(now)),
                    )
                    // CLOACI-T-0745: exclude already-claimed (in-flight) tasks so
                    // fire-and-forget dispatch doesn't re-select a task whose
                    // dispatch is still running. `claim_for_runner` only sets
                    // claimed_by (status stays Ready until the agent reports), and
                    // the claim is released (claimed_by -> NULL) on every executor
                    // exit BEFORE a retry re-marks the task Ready — so fresh and
                    // retried Ready tasks (claimed_by NULL) are still selected,
                    // while in-flight ones are skipped. claim_for_runner's atomic
                    // CAS remains the exactly-once guard for the residual race.
                    .filter(task_executions::claimed_by.is_null())
                    .load(conn)
            })?;

        Ok(ready_tasks.into_iter().map(Into::into).collect())
    }
}
