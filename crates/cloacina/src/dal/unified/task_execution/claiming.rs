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
use crate::dal::unified::models::{
    NewUnifiedExecutionEvent, TaskAttemptError, UnifiedTaskExecution,
};
use crate::database::schema::unified::{execution_events, task_attempt_errors, task_executions};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::ExecutionEventType;
use crate::models::task_execution::TaskExecution;
use diesel::prelude::*;

/// The heartbeat time before which a claim is stale.
fn stale_cutoff(threshold: std::time::Duration) -> UniversalTimestamp {
    UniversalTimestamp(
        chrono::Utc::now()
            - chrono::Duration::from_std(threshold).unwrap_or(chrono::Duration::seconds(60)),
    )
}

impl<'a> TaskExecutionDAL<'a> {
    /// Updates a task's retry schedule with a new attempt count and retry time.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    ///
    /// Sets `waiting_since` to `retry_at` (CLOACINA-T-1036): the backoff is
    /// not a wait for capacity, so the wait starts when the retry is due.
    /// Until then the value is in the future and the task is not counted as
    /// waiting.
    ///
    /// Clears `artifact_fetch_wait_since` (CLOACINA-T-1064): the wait for the
    /// package artifact has its limit per attempt, and this starts a new one.
    ///
    /// `error` is the failure text of the attempt that failed (CLOACINA-T-1100):
    /// with one, a row of `task_attempt_errors` keeps it for that attempt
    /// (`new_attempt - 1`), in the same transaction.
    pub async fn schedule_retry(
        &self,
        task_id: UniversalUuid,
        retry_at: UniversalTimestamp,
        new_attempt: i32,
        error: Option<&str>,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let error = error.map(str::to_string);
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                if let Some(text) = &error {
                    diesel::insert_into(task_attempt_errors::table)
                        .values(&TaskAttemptError::new(
                            task_id,
                            task.workflow_execution_id,
                            new_attempt - 1,
                            text,
                            now,
                        ))
                        .execute(conn)?;
                }

                // Update task retry state
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Ready"),
                        task_executions::attempt.eq(new_attempt),
                        task_executions::retry_at.eq(Some(retry_at)),
                        task_executions::waiting_since.eq(Some(retry_at)),
                        task_executions::artifact_fetch_wait_since.eq(None::<UniversalTimestamp>),
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

    /// Put a task back in the queue until `not_before`, with no change to its
    /// attempt count (CLOACINA-T-1012).
    ///
    /// For a dispatch that could not start for a reason that is not a failure
    /// of the task: no agent of its tenant has a free slot, or the agent it
    /// was sent to was full. [`schedule_retry`](Self::schedule_retry) uses up
    /// an attempt; this does not, so a task can wait for capacity for as long
    /// as it takes. The caller releases its claim after this call. Writes no
    /// execution event: a waiting task would write one every few seconds.
    ///
    /// `waiting_since` is not written (CLOACINA-T-1036): the task has waited
    /// for capacity since it became `Ready`, and each deferral moves only
    /// `retry_at`. So the task stays in
    /// [`waiting_task_stats`](Self::waiting_task_stats) between tries and its
    /// age keeps growing.
    ///
    /// Only a `Ready` task is changed. Returns `false` when the task is in
    /// another state.
    pub async fn defer_dispatch(
        &self,
        task_id: UniversalUuid,
        not_before: UniversalTimestamp,
    ) -> Result<bool, ValidationError> {
        let now = UniversalTimestamp::now();
        let rows: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                task_executions::table
                    .find(task_id)
                    .filter(task_executions::status.eq("Ready")),
            )
            .set((
                task_executions::retry_at.eq(Some(not_before)),
                task_executions::started_at.eq(None::<UniversalTimestamp>),
                task_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(rows > 0)
    }

    /// Undo the start of a dispatch and put the task back in the queue until
    /// `not_before`, with no change to its attempt count (CLOACINA-T-1043).
    ///
    /// For the fleet: the executor stamps the start
    /// ([`mark_started`](Self::mark_started): `Running`, `started_at`, no
    /// `waiting_since`) when the packet is handed to an agent. When that agent
    /// then refuses the packet as full, the task did not run.
    /// [`defer_dispatch`](Self::defer_dispatch) changes only a `Ready` task,
    /// so it cannot take this case: the task would stay `Running`, and once
    /// the caller releases its claim nothing would pick it up.
    ///
    /// The task goes back to `Ready` with no start time. `waiting_since` is
    /// put back: the value the row still has, else `waiting_since` (what the
    /// caller read before the start), else now. So a refusal does not reset
    /// how long the task has waited, and the task is in
    /// [`waiting_task_stats`](Self::waiting_task_stats) again. Writes no
    /// execution event, like `defer_dispatch`.
    ///
    /// Only a `Running` or `Ready` task whose claim `runner_id` holds is
    /// changed (CLOACINA-A-0017: state changes only under the claim). Returns
    /// `false` for any other task. The caller releases its claim after this
    /// call.
    pub async fn defer_dispatch_after_start(
        &self,
        task_id: UniversalUuid,
        runner_id: UniversalUuid,
        not_before: UniversalTimestamp,
        waiting_since: Option<UniversalTimestamp>,
    ) -> Result<bool, ValidationError> {
        self.defer_started_dispatch(task_id, runner_id, not_before, waiting_since, None)
            .await
    }

    /// [`defer_dispatch_after_start`](Self::defer_dispatch_after_start) for a
    /// dispatch that the agent refused because it could not fetch the package
    /// artifact (CLOACINA-T-1064).
    ///
    /// The same write, and it also records from when the task waits for its
    /// artifact: `artifact_fetch_wait_since` keeps the value the row has,
    /// else it gets `fetch_wait_since`. So the first such refusal of an
    /// attempt stamps the time and the later ones keep it, on every server
    /// replica. The caller reads the column before the dispatch and stops
    /// the wait at its limit; [`schedule_retry`](Self::schedule_retry) clears
    /// it for the next attempt.
    pub async fn defer_dispatch_after_fetch_refusal(
        &self,
        task_id: UniversalUuid,
        runner_id: UniversalUuid,
        not_before: UniversalTimestamp,
        waiting_since: Option<UniversalTimestamp>,
        fetch_wait_since: UniversalTimestamp,
    ) -> Result<bool, ValidationError> {
        self.defer_started_dispatch(
            task_id,
            runner_id,
            not_before,
            waiting_since,
            Some(fetch_wait_since),
        )
        .await
    }

    async fn defer_started_dispatch(
        &self,
        task_id: UniversalUuid,
        runner_id: UniversalUuid,
        not_before: UniversalTimestamp,
        waiting_since: Option<UniversalTimestamp>,
        fetch_wait_since: Option<UniversalTimestamp>,
    ) -> Result<bool, ValidationError> {
        use diesel::connection::Connection;

        let rows: usize = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<usize, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();
                let kept: Option<(Option<UniversalTimestamp>, Option<UniversalTimestamp>)> =
                    task_executions::table
                        .find(task_id)
                        .select((
                            task_executions::waiting_since,
                            task_executions::artifact_fetch_wait_since,
                        ))
                        .first(conn)
                        .optional()?;
                let Some((kept, kept_fetch_wait)) = kept else {
                    return Ok(0);
                };
                let waiting_since = kept.or(waiting_since).unwrap_or(now);
                let fetch_wait_since = kept_fetch_wait.or(fetch_wait_since);
                diesel::update(
                    task_executions::table
                        .find(task_id)
                        .filter(task_executions::claimed_by.eq(Some(runner_id)))
                        .filter(task_executions::status.eq_any(vec!["Running", "Ready"])),
                )
                .set((
                    task_executions::status.eq("Ready"),
                    task_executions::retry_at.eq(Some(not_before)),
                    task_executions::started_at.eq(None::<UniversalTimestamp>),
                    task_executions::waiting_since.eq(Some(waiting_since)),
                    task_executions::artifact_fetch_wait_since.eq(fetch_wait_since),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
            })
        })?;
        Ok(rows > 0)
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
    /// Clears `claimed_by` and `heartbeat_at`, and `fleet_agent_id`
    /// (CLOACINA-T-1027): a task with no dispatch is on no agent. Also the
    /// fleet dispatch (`fleet_dispatch_id`, `fleet_packet_row_id`,
    /// CLOACINA-T-1084): a dispatch lasts as long as its claim.
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
                    task_executions::fleet_agent_id.eq(None::<String>),
                    task_executions::fleet_dispatch_id.eq(None::<String>),
                    task_executions::fleet_packet_row_id.eq(None::<i64>),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Stamp the start of a fleet dispatch and record the dispatch
    /// (CLOACINA-T-1084), in one write: what
    /// [`mark_started`](Self::mark_started) writes (`Running`, `started_at`,
    /// no `waiting_since`), plus the `dispatch_id` of the work packet and the
    /// packet's `delivery_outbox` row. Written by the fleet executor that
    /// holds the claim, when it hands the packet to an agent; the dispatch
    /// is cleared with the claim
    /// ([`release_runner_claim`](Self::release_runner_claim)).
    ///
    /// With these, another server replica can take over the wait of the
    /// dispatch when the replica that sent it stops
    /// ([`find_stale_fleet_dispatches`](Self::find_stale_fleet_dispatches)).
    ///
    /// Only a row whose claim `runner_id` holds is changed. Returns `false`
    /// for any other row.
    pub async fn mark_started_in_fleet_dispatch(
        &self,
        task_id: UniversalUuid,
        runner_id: UniversalUuid,
        dispatch_id: &str,
        packet_row_id: i64,
    ) -> Result<bool, ValidationError> {
        let now = UniversalTimestamp::now();
        let dispatch_id = dispatch_id.to_string();
        let rows: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                task_executions::table
                    .find(task_id)
                    .filter(task_executions::claimed_by.eq(Some(runner_id))),
            )
            .set((
                task_executions::status.eq("Running"),
                task_executions::started_at.eq(Some(now)),
                task_executions::waiting_since.eq(None::<UniversalTimestamp>),
                task_executions::fleet_dispatch_id.eq(Some(dispatch_id)),
                task_executions::fleet_packet_row_id.eq(Some(packet_row_id)),
                task_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(rows > 0)
    }

    /// The fleet dispatches whose executor stopped (CLOACINA-T-1084): rows in
    /// a fleet dispatch (`fleet_dispatch_id` set) with a claim whose
    /// heartbeat is older than `threshold`, `Running` or `Ready`.
    ///
    /// The fleet executor reads these to take over the wait of a dispatch
    /// that is still in flight on an agent
    /// ([`take_over_stale_claim`](Self::take_over_stale_claim)), or to give a
    /// lost one back to the stale claim sweeper
    /// ([`hand_back_lost_fleet_dispatch`](Self::hand_back_lost_fleet_dispatch)).
    pub async fn find_stale_fleet_dispatches(
        &self,
        threshold: std::time::Duration,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let cutoff = stale_cutoff(threshold);
        let stale: Vec<UnifiedTaskExecution> = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::claimed_by.is_not_null())
                .filter(task_executions::heartbeat_at.lt(Some(cutoff)))
                .filter(task_executions::fleet_dispatch_id.is_not_null())
                .filter(task_executions::status.eq_any(vec!["Running", "Ready"]))
                .load(conn)
        })?;
        Ok(stale.into_iter().map(Into::into).collect())
    }

    /// Move a stale claim to `new_runner` (CLOACINA-T-1084): the claim is
    /// still `old_runner`'s and its heartbeat is still older than
    /// `threshold`. Sets the heartbeat to now. The fleet dispatch columns are
    /// kept: the dispatch goes on under the new claim.
    ///
    /// The compare and set is the guard between replicas: of two that try,
    /// one moves the claim. A runner that heartbeats again in the meantime
    /// keeps its claim. Returns `true` when this call moved it.
    pub async fn take_over_stale_claim(
        &self,
        task_id: UniversalUuid,
        old_runner: UniversalUuid,
        new_runner: UniversalUuid,
        threshold: std::time::Duration,
    ) -> Result<bool, ValidationError> {
        let cutoff = stale_cutoff(threshold);
        let now = UniversalTimestamp::now();
        let rows: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                task_executions::table
                    .find(task_id)
                    .filter(task_executions::claimed_by.eq(Some(old_runner)))
                    .filter(task_executions::heartbeat_at.lt(Some(cutoff)))
                    .filter(task_executions::status.eq_any(vec!["Running", "Ready"])),
            )
            .set((
                task_executions::claimed_by.eq(Some(new_runner)),
                task_executions::heartbeat_at.eq(Some(now)),
                task_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(rows > 0)
    }

    /// Give a stale fleet dispatch that is lost back to the stale claim
    /// sweeper (CLOACINA-T-1084): clear the fleet dispatch columns while the
    /// claim is still `old_runner`'s and still stale. The sweeper then
    /// recovers the row as any stale claim, with its recovery count and cap.
    /// Returns `true` when this call cleared them.
    pub async fn hand_back_lost_fleet_dispatch(
        &self,
        task_id: UniversalUuid,
        old_runner: UniversalUuid,
        threshold: std::time::Duration,
    ) -> Result<bool, ValidationError> {
        let cutoff = stale_cutoff(threshold);
        let now = UniversalTimestamp::now();
        let rows: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                task_executions::table
                    .find(task_id)
                    .filter(task_executions::claimed_by.eq(Some(old_runner)))
                    .filter(task_executions::heartbeat_at.lt(Some(cutoff))),
            )
            .set((
                task_executions::fleet_dispatch_id.eq(None::<String>),
                task_executions::fleet_packet_row_id.eq(None::<i64>),
                task_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(rows > 0)
    }

    /// [`find_stale_claims`](Self::find_stale_claims) without the rows in a
    /// fleet dispatch (`fleet_dispatch_id` set, CLOACINA-T-1084).
    ///
    /// For a runner whose fleet executor takes over those rows itself: a
    /// fleet dispatch whose executor stopped can still be in flight on an
    /// agent, and making it `Ready` would send the task a second time. The
    /// fleet executor gives the lost ones back by clearing the columns.
    pub async fn find_stale_claims_outside_fleet_dispatches(
        &self,
        threshold: std::time::Duration,
    ) -> Result<Vec<StaleClaim>, ValidationError> {
        self.stale_claims(threshold, true).await
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
        self.stale_claims(threshold, false).await
    }

    async fn stale_claims(
        &self,
        threshold: std::time::Duration,
        outside_fleet_dispatches: bool,
    ) -> Result<Vec<StaleClaim>, ValidationError> {
        let cutoff = stale_cutoff(threshold);

        let stale: Vec<UnifiedTaskExecution> = crate::interact_on_backend!(self.dal, |conn| {
            let mut query = task_executions::table
                .filter(task_executions::claimed_by.is_not_null())
                .filter(task_executions::heartbeat_at.lt(Some(cutoff)))
                .filter(task_executions::status.eq_any(vec!["Running", "Ready"]))
                .into_boxed();
            if outside_fleet_dispatches {
                query = query.filter(task_executions::fleet_dispatch_id.is_null());
            }
            query.load(conn)
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
                    // claimed_by (status stays Ready until the executor stamps the
                    // start with `mark_started`, which makes it Running), and
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

    /// The waiting set (CLOACINA-T-1045): every `Ready`, unclaimed task that
    /// is due, and every one that was put off for capacity (its `retry_at`
    /// is in the future but it waits since before now). A retry in backoff
    /// is not in the set: it waits from its due time.
    ///
    /// The rows come in the order of the stored priorities: workflow
    /// priority, task priority, time waited. The scheduler loop applies
    /// ageing and decides which of them to dispatch
    /// (`execution_planner::priority::dispatch_order`).
    pub async fn get_waiting_for_dispatch(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        let now = UniversalTimestamp::now();
        let waiting: Vec<UnifiedTaskExecution> = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::status.eq("Ready"))
                .filter(
                    task_executions::retry_at
                        .is_null()
                        .or(task_executions::retry_at.le(now))
                        .or(task_executions::waiting_since.le(now)),
                )
                .filter(task_executions::claimed_by.is_null())
                .order((
                    task_executions::workflow_priority.asc(),
                    task_executions::task_priority.asc(),
                    task_executions::waiting_since.asc(),
                    task_executions::created_at.asc(),
                ))
                .load(conn)
        })?;

        Ok(waiting.into_iter().map(Into::into).collect())
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod defer_dispatch_after_start_tests {
    use crate::dal::DAL;
    use crate::database::Database;
    use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
    use crate::models::task_execution::{NewTaskExecution, TaskExecution};
    use crate::models::workflow_execution::NewWorkflowExecution;
    use chrono::{Duration, Utc};

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:defer_after_start_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    /// A `Ready` task on attempt 2 of 3.
    async fn ready_task(dal: &DAL) -> UniversalUuid {
        let workflow_id = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "refused_workflow".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id;
        dal.task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: workflow_id,
                task_name: "refused".into(),
                status: "Ready".into(),
                attempt: 2,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id
    }

    async fn row(dal: &DAL, task_id: UniversalUuid) -> TaskExecution {
        dal.task_execution().get_by_id(task_id).await.unwrap()
    }

    fn in_seconds(seconds: i64) -> UniversalTimestamp {
        UniversalTimestamp(Utc::now() + Duration::seconds(seconds))
    }

    /// What the fleet executor does up to the agent's refusal: claim, read the
    /// wait, stamp the start. Returns the wait it read.
    async fn claim_and_start(
        dal: &DAL,
        task_id: UniversalUuid,
        runner: UniversalUuid,
    ) -> Option<UniversalTimestamp> {
        dal.task_execution()
            .claim_for_runner(task_id, runner)
            .await
            .unwrap();
        let waiting_before = row(dal, task_id).await.waiting_since;
        dal.task_execution().mark_started(task_id).await.unwrap();
        waiting_before
    }

    #[tokio::test]
    async fn defer_dispatch_does_not_change_a_started_task() {
        // The hole of CLOACINA-T-1043: the deferral of CLOACINA-T-1012 is for
        // a `Ready` task only.
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        claim_and_start(&dal, task_id, UniversalUuid::new_v4()).await;

        let changed = dal
            .task_execution()
            .defer_dispatch(task_id, in_seconds(2))
            .await
            .unwrap();

        assert!(!changed);
        assert_eq!(row(&dal, task_id).await.status, "Running");
    }

    #[tokio::test]
    async fn a_started_task_goes_back_to_ready_with_its_attempt_and_its_wait() {
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let runner = UniversalUuid::new_v4();
        let waiting_before = claim_and_start(&dal, task_id, runner).await;
        assert!(waiting_before.is_some());
        let started = row(&dal, task_id).await;
        assert_eq!(started.status, "Running");
        assert!(started.started_at.is_some());
        assert_eq!(started.waiting_since, None);

        let changed = dal
            .task_execution()
            .defer_dispatch_after_start(task_id, runner, in_seconds(-1), waiting_before)
            .await
            .unwrap();

        assert!(changed);
        let after = row(&dal, task_id).await;
        assert_eq!(after.status, "Ready");
        assert_eq!(after.attempt, 2);
        assert_eq!(after.started_at, None);
        assert_eq!(after.waiting_since, waiting_before);
        assert_eq!(
            after.claimed_by,
            Some(runner),
            "the caller releases the claim"
        );

        // Counted as waiting again, and in the scheduler's read once the
        // claim is released and the delay is over.
        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 1);
        dal.task_execution()
            .release_runner_claim(task_id)
            .await
            .unwrap();
        let ready = dal.task_execution().get_ready_for_retry().await.unwrap();
        assert!(ready.iter().any(|task| task.id == task_id));
    }

    #[tokio::test]
    async fn a_task_whose_start_was_not_stamped_keeps_the_wait_it_has() {
        // `mark_started` is best-effort in the fleet executor. When it did not
        // apply, the row is still `Ready` with its `waiting_since`.
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let runner = UniversalUuid::new_v4();
        dal.task_execution()
            .claim_for_runner(task_id, runner)
            .await
            .unwrap();
        let waiting_before = row(&dal, task_id).await.waiting_since;

        let changed = dal
            .task_execution()
            .defer_dispatch_after_start(task_id, runner, in_seconds(2), None)
            .await
            .unwrap();

        assert!(changed);
        let after = row(&dal, task_id).await;
        assert_eq!(after.status, "Ready");
        assert_eq!(after.waiting_since, waiting_before);
        assert!(after.retry_at.is_some());
    }

    #[tokio::test]
    async fn a_started_task_with_no_wait_on_record_waits_from_now() {
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let runner = UniversalUuid::new_v4();
        claim_and_start(&dal, task_id, runner).await;
        let before = Utc::now() - Duration::seconds(1);

        dal.task_execution()
            .defer_dispatch_after_start(task_id, runner, in_seconds(2), None)
            .await
            .unwrap();

        let since = row(&dal, task_id).await.waiting_since.expect("a wait");
        assert!(since.0 >= before);
    }

    #[tokio::test]
    async fn a_task_claimed_by_another_runner_is_not_changed() {
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let owner = UniversalUuid::new_v4();
        let waiting_before = claim_and_start(&dal, task_id, owner).await;

        let changed = dal
            .task_execution()
            .defer_dispatch_after_start(
                task_id,
                UniversalUuid::new_v4(),
                in_seconds(2),
                waiting_before,
            )
            .await
            .unwrap();

        assert!(!changed);
        let after = row(&dal, task_id).await;
        assert_eq!(after.status, "Running");
        assert!(after.started_at.is_some());
    }

    #[tokio::test]
    async fn a_task_in_an_end_state_is_not_changed() {
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let runner = UniversalUuid::new_v4();
        let waiting_before = claim_and_start(&dal, task_id, runner).await;
        dal.task_execution()
            .mark_failed(task_id, "failed elsewhere", Some(runner))
            .await
            .unwrap();

        let changed = dal
            .task_execution()
            .defer_dispatch_after_start(task_id, runner, in_seconds(2), waiting_before)
            .await
            .unwrap();

        assert!(!changed);
        assert_eq!(row(&dal, task_id).await.status, "Failed");
    }

    #[tokio::test]
    async fn an_unknown_task_is_not_an_error() {
        let dal = unique_dal().await;
        let changed = dal
            .task_execution()
            .defer_dispatch_after_start(
                UniversalUuid::new_v4(),
                UniversalUuid::new_v4(),
                in_seconds(2),
                None,
            )
            .await
            .unwrap();
        assert!(!changed);
    }

    // CLOACINA-T-1064: the wait for the package artifact.

    /// `stored` is `expected`, to the precision the database keeps.
    fn is_time(stored: Option<UniversalTimestamp>, expected: UniversalTimestamp) -> bool {
        stored.is_some_and(|at| (at.0 - expected.0).num_milliseconds().abs() < 5)
    }

    #[tokio::test]
    async fn a_capacity_deferral_does_not_stamp_the_fetch_wait() {
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let runner = UniversalUuid::new_v4();
        let waiting_before = claim_and_start(&dal, task_id, runner).await;

        dal.task_execution()
            .defer_dispatch_after_start(task_id, runner, in_seconds(2), waiting_before)
            .await
            .unwrap();

        assert_eq!(row(&dal, task_id).await.artifact_fetch_wait_since, None);
    }

    #[tokio::test]
    async fn the_first_fetch_refusal_stamps_the_fetch_wait_and_the_next_ones_keep_it() {
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let runner = UniversalUuid::new_v4();
        let first = in_seconds(-40);

        let waiting_before = claim_and_start(&dal, task_id, runner).await;
        let changed = dal
            .task_execution()
            .defer_dispatch_after_fetch_refusal(
                task_id,
                runner,
                in_seconds(2),
                waiting_before,
                first,
            )
            .await
            .unwrap();
        assert!(changed);

        let after = row(&dal, task_id).await;
        assert_eq!(after.status, "Ready");
        assert_eq!(after.attempt, 2, "the attempt is kept");
        assert_eq!(after.started_at, None);
        assert_eq!(after.waiting_since, waiting_before);
        assert!(is_time(after.artifact_fetch_wait_since, first));
        let stamped = after.artifact_fetch_wait_since;

        // The next refusal, and a refusal for capacity between them, keep
        // the first time.
        dal.task_execution()
            .release_runner_claim(task_id)
            .await
            .unwrap();
        let waiting_before = claim_and_start(&dal, task_id, runner).await;
        dal.task_execution()
            .defer_dispatch_after_start(task_id, runner, in_seconds(2), waiting_before)
            .await
            .unwrap();
        assert_eq!(row(&dal, task_id).await.artifact_fetch_wait_since, stamped);
        dal.task_execution()
            .release_runner_claim(task_id)
            .await
            .unwrap();
        let waiting_before = claim_and_start(&dal, task_id, runner).await;
        dal.task_execution()
            .defer_dispatch_after_fetch_refusal(
                task_id,
                runner,
                in_seconds(2),
                waiting_before,
                in_seconds(0),
            )
            .await
            .unwrap();
        assert_eq!(row(&dal, task_id).await.artifact_fetch_wait_since, stamped);
    }

    #[tokio::test]
    async fn a_scheduled_retry_clears_the_fetch_wait() {
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let runner = UniversalUuid::new_v4();
        let waiting_before = claim_and_start(&dal, task_id, runner).await;
        dal.task_execution()
            .defer_dispatch_after_fetch_refusal(
                task_id,
                runner,
                in_seconds(2),
                waiting_before,
                in_seconds(-40),
            )
            .await
            .unwrap();
        assert!(row(&dal, task_id).await.artifact_fetch_wait_since.is_some());

        dal.task_execution()
            .schedule_retry(task_id, in_seconds(5), 3, None)
            .await
            .unwrap();

        let after = row(&dal, task_id).await;
        assert_eq!(after.attempt, 3);
        assert_eq!(
            after.artifact_fetch_wait_since, None,
            "a new attempt gets the full wait"
        );
    }

    #[tokio::test]
    async fn a_fetch_refusal_of_a_task_claimed_by_another_runner_changes_nothing() {
        let dal = unique_dal().await;
        let task_id = ready_task(&dal).await;
        let waiting_before = claim_and_start(&dal, task_id, UniversalUuid::new_v4()).await;

        let changed = dal
            .task_execution()
            .defer_dispatch_after_fetch_refusal(
                task_id,
                UniversalUuid::new_v4(),
                in_seconds(2),
                waiting_before,
                in_seconds(0),
            )
            .await
            .unwrap();

        assert!(!changed);
        let after = row(&dal, task_id).await;
        assert_eq!(after.status, "Running");
        assert_eq!(after.artifact_fetch_wait_since, None);
    }
}

/// CLOACINA-T-1084: the fleet dispatch on the task row, and the takeover of
/// a stale one.
#[cfg(all(test, feature = "sqlite"))]
mod fleet_dispatch_tests {
    use crate::dal::DAL;
    use crate::database::Database;
    use crate::database::schema::unified::task_executions;
    use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
    use crate::models::task_execution::{NewTaskExecution, TaskExecution};
    use crate::models::workflow_execution::NewWorkflowExecution;
    use diesel::prelude::*;
    use std::time::Duration;

    const STALE: Duration = Duration::from_secs(60);

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:fleet_dispatch_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    /// A task that `runner` claimed and started, in the fleet dispatch `d1`
    /// with outbox row 7.
    async fn dispatched_task(dal: &DAL, runner: UniversalUuid) -> UniversalUuid {
        let workflow_id = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "wf".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id;
        let task_id = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: workflow_id,
                task_name: "t".into(),
                status: "Ready".into(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id;
        dal.task_execution()
            .claim_for_runner(task_id, runner)
            .await
            .unwrap();
        assert!(
            dal.task_execution()
                .mark_started_in_fleet_dispatch(task_id, runner, "d1", 7)
                .await
                .unwrap()
        );
        task_id
    }

    async fn row(dal: &DAL, task_id: UniversalUuid) -> TaskExecution {
        dal.task_execution().get_by_id(task_id).await.unwrap()
    }

    /// Make the claim's heartbeat `seconds` old: its runner stopped.
    async fn age_heartbeat(dal: &DAL, task_id: UniversalUuid, seconds: i64) {
        let at = UniversalTimestamp(chrono::Utc::now() - chrono::Duration::seconds(seconds));
        let aged: Result<usize, diesel::result::Error> = crate::interact_on_backend!(dal, |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set(task_executions::heartbeat_at.eq(Some(at)))
                .execute(conn)
        });
        assert_eq!(aged.unwrap(), 1);
    }

    #[tokio::test]
    async fn the_dispatch_is_recorded_under_the_claim_and_cleared_with_it() {
        let dal = unique_dal().await;
        let runner = UniversalUuid::new_v4();
        let task_id = dispatched_task(&dal, runner).await;
        let task = row(&dal, task_id).await;
        assert_eq!(task.status, "Running");
        assert!(task.started_at.is_some());
        assert_eq!(task.waiting_since, None);
        assert_eq!(task.fleet_dispatch_id.as_deref(), Some("d1"));
        assert_eq!(task.fleet_packet_row_id, Some(7));

        // Another runner cannot write it.
        assert!(
            !dal.task_execution()
                .mark_started_in_fleet_dispatch(task_id, UniversalUuid::new_v4(), "d2", 8)
                .await
                .unwrap()
        );
        assert_eq!(
            row(&dal, task_id).await.fleet_dispatch_id.as_deref(),
            Some("d1")
        );

        dal.task_execution()
            .release_runner_claim(task_id)
            .await
            .unwrap();
        let task = row(&dal, task_id).await;
        assert_eq!(task.fleet_dispatch_id, None);
        assert_eq!(task.fleet_packet_row_id, None);
    }

    #[tokio::test]
    async fn a_stale_fleet_dispatch_is_found_and_left_out_of_the_other_stale_claims() {
        let dal = unique_dal().await;
        let task_id = dispatched_task(&dal, UniversalUuid::new_v4()).await;
        let tasks = dal.task_execution();

        // A live heartbeat: nothing is stale.
        assert!(
            tasks
                .find_stale_fleet_dispatches(STALE)
                .await
                .unwrap()
                .is_empty()
        );

        age_heartbeat(&dal, task_id, 120).await;
        let found = tasks.find_stale_fleet_dispatches(STALE).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, task_id);
        assert_eq!(found[0].fleet_dispatch_id.as_deref(), Some("d1"));

        // The sweeper's read sees it, unless it leaves fleet dispatches out.
        assert_eq!(tasks.find_stale_claims(STALE).await.unwrap().len(), 1);
        assert!(
            tasks
                .find_stale_claims_outside_fleet_dispatches(STALE)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn one_runner_takes_over_a_stale_claim_and_a_live_one_keeps_it() {
        let dal = unique_dal().await;
        let old = UniversalUuid::new_v4();
        let task_id = dispatched_task(&dal, old).await;
        let tasks = dal.task_execution();
        let (b, c) = (UniversalUuid::new_v4(), UniversalUuid::new_v4());

        // Not stale yet: no takeover.
        assert!(
            !tasks
                .take_over_stale_claim(task_id, old, b, STALE)
                .await
                .unwrap()
        );

        age_heartbeat(&dal, task_id, 120).await;
        assert!(
            tasks
                .take_over_stale_claim(task_id, old, b, STALE)
                .await
                .unwrap()
        );
        // The second replica finds the claim moved.
        assert!(
            !tasks
                .take_over_stale_claim(task_id, old, c, STALE)
                .await
                .unwrap()
        );

        let task = row(&dal, task_id).await;
        assert_eq!(task.claimed_by, Some(b));
        assert!(
            task.heartbeat_at
                .is_some_and(|at| (chrono::Utc::now() - at.0).num_seconds() < 5)
        );
        assert_eq!(task.status, "Running");
        assert_eq!(
            task.fleet_dispatch_id.as_deref(),
            Some("d1"),
            "the dispatch goes on"
        );
        assert!(
            tasks
                .find_stale_fleet_dispatches(STALE)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn a_lost_dispatch_handed_back_is_an_ordinary_stale_claim() {
        let dal = unique_dal().await;
        let old = UniversalUuid::new_v4();
        let task_id = dispatched_task(&dal, old).await;
        let tasks = dal.task_execution();
        age_heartbeat(&dal, task_id, 120).await;

        // Another runner's claim is not handed back.
        assert!(
            !tasks
                .hand_back_lost_fleet_dispatch(task_id, UniversalUuid::new_v4(), STALE)
                .await
                .unwrap()
        );
        assert!(
            tasks
                .hand_back_lost_fleet_dispatch(task_id, old, STALE)
                .await
                .unwrap()
        );

        let task = row(&dal, task_id).await;
        assert_eq!(task.fleet_dispatch_id, None);
        assert_eq!(task.fleet_packet_row_id, None);
        assert_eq!(
            task.claimed_by,
            Some(old),
            "the claim stays for the sweeper"
        );
        let swept = tasks
            .find_stale_claims_outside_fleet_dispatches(STALE)
            .await
            .unwrap();
        assert_eq!(swept.len(), 1);
        assert_eq!(swept[0].claimed_by, old);
    }
}
