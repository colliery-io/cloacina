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

//! Unified Schedule Execution DAL with runtime backend selection
//!
//! This module provides operations for the unified `schedule_executions` table
//! that replaces the separate `cron_executions` and `trigger_executions` tables.
//! Works with both PostgreSQL and SQLite backends, selecting the appropriate
//! implementation at runtime based on the database connection type.

use diesel::prelude::*;

use super::DAL;
use crate::dal::unified::models::{NewUnifiedScheduleExecution, UnifiedScheduleExecution};
use crate::database::schema::unified::schedule_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::schedule::{NewScheduleExecution, ScheduleExecution};
use chrono::{DateTime, Duration, Utc};

/// Statistics about schedule execution performance
#[derive(Debug)]
pub struct ScheduleExecutionStats {
    /// Total number of executions attempted
    pub total_executions: i64,
    /// Number of executions that successfully handed off to workflow executor
    pub successful_executions: i64,
    /// Number of executions that were lost (started but never completed within expected time)
    pub lost_executions: i64,
    /// Success rate as a percentage
    pub success_rate: f64,
}

/// Outcome of a compare-and-set attempt to own the recovery of a lost handoff.
///
/// CLOACI-T-0926. Mirrors `RunnerClaimResult` in the task-claiming DAL: the
/// claim is a single conditional UPDATE and `rows_updated == 1` is the winner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryClaimResult {
    /// This caller now owns the re-fire of the handoff.
    Claimed,
    /// Another recovery service owns it (or the row left the lost set).
    NotClaimed,
}

/// Outcome of a recovery claim heartbeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryHeartbeatResult {
    /// The claim is still ours.
    Ok,
    /// Another recovery service took the claim over (ours went stale).
    ClaimLost,
}

/// Data access layer for unified schedule execution operations with runtime backend selection.
#[derive(Clone)]
pub struct ScheduleExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ScheduleExecutionDAL<'a> {
    /// Creates a new ScheduleExecutionDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new schedule execution record in the database.
    pub async fn create(
        &self,
        new_execution: NewScheduleExecution,
    ) -> Result<ScheduleExecution, ValidationError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedScheduleExecution {
            id,
            schedule_id: new_execution.schedule_id,
            workflow_execution_id: new_execution.workflow_execution_id,
            scheduled_time: new_execution.scheduled_time,
            claimed_at: new_execution.claimed_at,
            context_hash: new_execution.context_hash,
            started_at: now,
            created_at: now,
            updated_at: now,
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(schedule_executions::table)
                .values(&new_unified)
                .execute(conn)
        })?;

        let result: UnifiedScheduleExecution = crate::interact_on_backend!(self.dal, |conn| {
            schedule_executions::table.find(id).first(conn)
        })?;

        Ok(result.into())
    }

    /// Retrieves a schedule execution by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<ScheduleExecution, ValidationError> {
        let result: UnifiedScheduleExecution = crate::interact_on_backend!(self.dal, |conn| {
            schedule_executions::table.find(id).first(conn)
        })?;

        Ok(result.into())
    }

    /// Lists schedule executions for a given schedule.
    pub async fn list_by_schedule(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        let results: Vec<UnifiedScheduleExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .order(schedule_executions::created_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    /// Marks a schedule execution as completed.
    pub async fn complete(
        &self,
        id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let completed_ts = UniversalTimestamp::from(completed_at);
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::completed_at.eq(Some(completed_ts)),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Checks if there is an active (uncompleted) execution for a schedule with the given context hash.
    pub async fn has_active_execution(
        &self,
        schedule_id: UniversalUuid,
        context_hash: &str,
    ) -> Result<bool, ValidationError> {
        let context_hash_owned = context_hash.to_string();

        let count: i64 = crate::interact_on_backend!(self.dal, |conn| {
            schedule_executions::table
                .filter(schedule_executions::schedule_id.eq(schedule_id))
                .filter(schedule_executions::context_hash.eq(context_hash_owned))
                .filter(schedule_executions::completed_at.is_null())
                .count()
                .get_result(conn)
        })?;

        Ok(count > 0)
    }

    /// Updates the workflow execution ID for a schedule execution.
    pub async fn update_workflow_execution_id(
        &self,
        id: UniversalUuid,
        workflow_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::workflow_execution_id.eq(Some(workflow_execution_id)),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Finds lost executions (started but not completed) older than the specified minutes.
    pub async fn find_lost_executions(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        let cutoff = Utc::now() - Duration::minutes(older_than_minutes as i64);
        let cutoff_ts = UniversalTimestamp::from(cutoff);

        let results: Vec<UnifiedScheduleExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                schedule_executions::table
                    .filter(schedule_executions::completed_at.is_null())
                    .filter(schedule_executions::started_at.lt(cutoff_ts))
                    .order(schedule_executions::started_at.asc())
                    .load(conn)
            })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    // ========================================================================
    // Recovery ownership + accounting (CLOACI-T-0926)
    // ========================================================================

    /// Atomically claim a lost handoff for recovery by `owner_id`.
    ///
    /// `find_lost_executions` is a plain SELECT, so every replica's recovery
    /// service sees the same lost rows. This compare-and-set is what makes
    /// exactly one of them re-fire, in the same shape as
    /// `TaskExecutionDAL::claim_for_runner`: one conditional UPDATE,
    /// `rows_updated == 1` wins.
    ///
    /// The claim succeeds when the row is still open AND either unclaimed or
    /// held by an owner whose heartbeat went stale. Two guards matter:
    ///
    /// * `completed_at IS NULL` is part of the CAS, not a pre-check. A loser
    ///   that selected the row before the winner finished still holds a stale
    ///   snapshot; without this predicate it would claim the (now completed)
    ///   row after the winner released it and re-fire — the duplicate this is
    ///   here to prevent.
    /// * the staleness arm uses the heartbeat, not the claim time. A re-fire
    ///   blocks for the workflow's full (unbounded) duration, so a fixed
    ///   claim-age window would expire mid-execution and hand the row to a
    ///   second service. The owner beats while it works, so a stale beat is a
    ///   true death signal and a crashed service never locks a handoff
    ///   permanently.
    pub async fn claim_for_recovery(
        &self,
        id: UniversalUuid,
        owner_id: UniversalUuid,
        stale_after: std::time::Duration,
    ) -> Result<RecoveryClaimResult, ValidationError> {
        let now = UniversalTimestamp::now();
        let cutoff = UniversalTimestamp(
            Utc::now() - Duration::from_std(stale_after).unwrap_or_else(|_| Duration::seconds(120)),
        );

        let rows_updated: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                schedule_executions::table
                    .find(id)
                    .filter(schedule_executions::completed_at.is_null())
                    .filter(
                        schedule_executions::recovery_claimed_by
                            .is_null()
                            .or(schedule_executions::recovery_heartbeat_at.lt(Some(cutoff))),
                    ),
            )
            .set((
                schedule_executions::recovery_claimed_by.eq(Some(owner_id)),
                schedule_executions::recovery_heartbeat_at.eq(Some(now)),
                schedule_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;

        Ok(if rows_updated > 0 {
            RecoveryClaimResult::Claimed
        } else {
            RecoveryClaimResult::NotClaimed
        })
    }

    /// Refresh the recovery claim heartbeat, only while we still own it.
    pub async fn recovery_heartbeat(
        &self,
        id: UniversalUuid,
        owner_id: UniversalUuid,
    ) -> Result<RecoveryHeartbeatResult, ValidationError> {
        let now = UniversalTimestamp::now();
        let rows_updated: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                schedule_executions::table
                    .find(id)
                    .filter(schedule_executions::recovery_claimed_by.eq(Some(owner_id))),
            )
            .set((
                schedule_executions::recovery_heartbeat_at.eq(Some(now)),
                schedule_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;

        Ok(if rows_updated > 0 {
            RecoveryHeartbeatResult::Ok
        } else {
            RecoveryHeartbeatResult::ClaimLost
        })
    }

    /// Release a recovery claim we hold. No-op if someone else owns it now.
    pub async fn release_recovery_claim(
        &self,
        id: UniversalUuid,
        owner_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                schedule_executions::table
                    .find(id)
                    .filter(schedule_executions::recovery_claimed_by.eq(Some(owner_id))),
            )
            .set((
                schedule_executions::recovery_claimed_by.eq(None::<UniversalUuid>),
                schedule_executions::recovery_heartbeat_at.eq(None::<UniversalTimestamp>),
                schedule_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;

        Ok(())
    }

    /// Increment the durable recovery attempt count and return the new value.
    ///
    /// The count lives on the audit row (not in the recovery service's memory)
    /// so a schedule that reliably fails recovery cannot earn a fresh budget by
    /// bouncing the process. Callers hold the recovery claim, so the read-back
    /// is not racing another attempt; the increment itself is still done
    /// in-SQL and inside a transaction so the value can never regress.
    pub async fn increment_recovery_attempts(
        &self,
        id: UniversalUuid,
    ) -> Result<i32, ValidationError> {
        use diesel::connection::Connection;

        let attempts: i32 = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();
                diesel::update(schedule_executions::table.find(id))
                    .set((
                        schedule_executions::recovery_attempts
                            .eq(schedule_executions::recovery_attempts + 1),
                        schedule_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                schedule_executions::table
                    .find(id)
                    .select(schedule_executions::recovery_attempts)
                    .first(conn)
            })
        })?;

        Ok(attempts)
    }

    /// Reset the durable recovery attempt count (used after a successful
    /// re-fire, the durable equivalent of clearing the old in-memory entry).
    pub async fn reset_recovery_attempts(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::recovery_attempts.eq(0),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Gets the latest execution for a given schedule.
    pub async fn get_latest_by_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<ScheduleExecution>, ValidationError> {
        let result: Option<UnifiedScheduleExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .order(schedule_executions::created_at.desc())
                    .first(conn)
                    .optional()
            })?;

        Ok(result.map(|r| r.into()))
    }

    /// Gets execution statistics for monitoring and alerting.
    pub async fn get_execution_stats(
        &self,
        since: DateTime<Utc>,
    ) -> Result<ScheduleExecutionStats, ValidationError> {
        use crate::database::schema::unified::workflow_executions;

        let since_ts = UniversalTimestamp::from(since);
        let lost_cutoff = UniversalTimestamp::from(Utc::now() - Duration::minutes(10));

        let (total_executions, successful_executions, lost_executions) =
            crate::interact_on_backend!(self.dal, |conn| {
                let total_executions: i64 = schedule_executions::table
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .count()
                    .first(conn)?;

                let successful_executions: i64 = schedule_executions::table
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .filter(schedule_executions::workflow_execution_id.is_not_null())
                    .count()
                    .first(conn)?;

                let lost_executions: i64 = schedule_executions::table
                    .left_join(
                        workflow_executions::table.on(schedule_executions::workflow_execution_id
                            .eq(workflow_executions::id.nullable())),
                    )
                    .filter(workflow_executions::id.is_null())
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .filter(schedule_executions::started_at.lt(lost_cutoff))
                    .count()
                    .first(conn)?;

                Ok::<(i64, i64, i64), diesel::result::Error>((
                    total_executions,
                    successful_executions,
                    lost_executions,
                ))
            })?;

        Ok(ScheduleExecutionStats {
            total_executions,
            successful_executions,
            lost_executions,
            success_rate: if total_executions > 0 {
                (successful_executions as f64 / total_executions as f64) * 100.0
            } else {
                0.0
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::UniversalTimestamp;
    use crate::database::Database;
    use crate::models::schedule::{NewSchedule, NewScheduleExecution};

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "file:sched_exec_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    /// Helper: create a cron schedule and return its ID.
    #[cfg(feature = "sqlite")]
    async fn create_schedule(dal: &DAL) -> UniversalUuid {
        let next_run = UniversalTimestamp::now();
        let schedule = dal
            .schedule()
            .create(NewSchedule::cron("test_wf", "0 * * * *", next_run))
            .await
            .unwrap();
        schedule.id
    }

    /// Helper: build a NewScheduleExecution for a given schedule.
    #[cfg(feature = "sqlite")]
    fn new_exec(schedule_id: UniversalUuid) -> NewScheduleExecution {
        NewScheduleExecution {
            schedule_id,
            workflow_execution_id: None,
            scheduled_time: Some(UniversalTimestamp::now()),
            claimed_at: Some(UniversalTimestamp::now()),
            context_hash: Some(uuid::Uuid::new_v4().to_string()),
        }
    }

    // ── create + get_by_id ──────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_execution() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;

        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();

        assert_eq!(exec.schedule_id, sched_id);
        assert!(exec.workflow_execution_id.is_none());
        assert!(exec.completed_at.is_none());
        assert!(exec.scheduled_time.is_some());
        assert!(exec.context_hash.is_some());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_by_id() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let created = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();

        let fetched = dal
            .schedule_execution()
            .get_by_id(created.id)
            .await
            .unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.schedule_id, sched_id);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let dal = unique_dal().await;
        let result = dal
            .schedule_execution()
            .get_by_id(UniversalUuid::new_v4())
            .await;
        assert!(result.is_err());
    }

    // ── list_by_schedule ────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_by_schedule() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let other_sched_id = create_schedule(&dal).await;

        // Create 3 executions for sched_id, 1 for other
        for _ in 0..3 {
            dal.schedule_execution()
                .create(new_exec(sched_id))
                .await
                .unwrap();
        }
        dal.schedule_execution()
            .create(new_exec(other_sched_id))
            .await
            .unwrap();

        let list = dal
            .schedule_execution()
            .list_by_schedule(sched_id, 100, 0)
            .await
            .unwrap();
        assert_eq!(list.len(), 3);

        // With limit
        let limited = dal
            .schedule_execution()
            .list_by_schedule(sched_id, 2, 0)
            .await
            .unwrap();
        assert_eq!(limited.len(), 2);

        // With offset
        let offset = dal
            .schedule_execution()
            .list_by_schedule(sched_id, 100, 2)
            .await
            .unwrap();
        assert_eq!(offset.len(), 1);
    }

    // ── complete ────────────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_complete_execution() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();
        assert!(exec.completed_at.is_none());

        let completed_at = Utc::now();
        dal.schedule_execution()
            .complete(exec.id, completed_at)
            .await
            .unwrap();

        let updated = dal.schedule_execution().get_by_id(exec.id).await.unwrap();
        assert!(updated.completed_at.is_some());
    }

    // ── has_active_execution ────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_has_active_execution() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;

        // No executions yet
        let active = dal
            .schedule_execution()
            .has_active_execution(sched_id, "hash1")
            .await
            .unwrap();
        assert!(!active);

        // Create an uncompleted execution
        let mut ne = new_exec(sched_id);
        ne.context_hash = Some("hash1".to_string());
        dal.schedule_execution().create(ne).await.unwrap();

        let active = dal
            .schedule_execution()
            .has_active_execution(sched_id, "hash1")
            .await
            .unwrap();
        assert!(active);

        // Different hash should not match
        let active_other = dal
            .schedule_execution()
            .has_active_execution(sched_id, "hash_other")
            .await
            .unwrap();
        assert!(!active_other);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_has_active_execution_completed_not_active() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;

        let mut ne = new_exec(sched_id);
        ne.context_hash = Some("hash_done".to_string());
        let exec = dal.schedule_execution().create(ne).await.unwrap();

        // Complete it
        dal.schedule_execution()
            .complete(exec.id, Utc::now())
            .await
            .unwrap();

        let active = dal
            .schedule_execution()
            .has_active_execution(sched_id, "hash_done")
            .await
            .unwrap();
        assert!(!active);
    }

    // ── update_workflow_execution_id ────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_update_workflow_execution_id() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();
        assert!(exec.workflow_execution_id.is_none());

        // Create a real workflow execution so the FK constraint is satisfied
        use crate::models::workflow_execution::NewWorkflowExecution;
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "fk-test".to_string(),
                workflow_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .unwrap();

        dal.schedule_execution()
            .update_workflow_execution_id(exec.id, wf_exec.id)
            .await
            .unwrap();

        let updated = dal.schedule_execution().get_by_id(exec.id).await.unwrap();
        assert_eq!(updated.workflow_execution_id, Some(wf_exec.id));
    }

    // ── get_latest_by_schedule ──────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_latest_by_schedule() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;

        // No executions => None
        let latest = dal
            .schedule_execution()
            .get_latest_by_schedule(sched_id)
            .await
            .unwrap();
        assert!(latest.is_none());

        // Create two executions; the second is "latest" by created_at
        let _first = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();
        let second = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();

        let latest = dal
            .schedule_execution()
            .get_latest_by_schedule(sched_id)
            .await
            .unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id, second.id);
    }

    // ── find_lost_executions ────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_find_lost_executions_none_lost() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;

        // Create a fresh (just-started) execution — not lost yet
        dal.schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();

        // Looking for executions older than 60 minutes — our fresh one should not appear
        let lost = dal
            .schedule_execution()
            .find_lost_executions(60)
            .await
            .unwrap();
        assert!(lost.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_find_lost_executions_completed_not_lost() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;

        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();

        // Complete it so it should never be considered "lost"
        dal.schedule_execution()
            .complete(exec.id, Utc::now())
            .await
            .unwrap();

        let lost = dal
            .schedule_execution()
            .find_lost_executions(0)
            .await
            .unwrap();
        assert!(lost.is_empty());
    }

    // ── recovery claim + attempt accounting (CLOACI-T-0926) ─────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_recovery_claim_is_exclusive() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();

        let a = UniversalUuid::new_v4();
        let b = UniversalUuid::new_v4();
        let window = std::time::Duration::from_secs(120);

        assert_eq!(
            dal.schedule_execution()
                .claim_for_recovery(exec.id, a, window)
                .await
                .unwrap(),
            RecoveryClaimResult::Claimed
        );
        // Second service loses while A's claim is fresh.
        assert_eq!(
            dal.schedule_execution()
                .claim_for_recovery(exec.id, b, window)
                .await
                .unwrap(),
            RecoveryClaimResult::NotClaimed
        );

        let row = dal.schedule_execution().get_by_id(exec.id).await.unwrap();
        assert_eq!(row.recovery_claimed_by, Some(a));
        assert!(row.recovery_heartbeat_at.is_some());

        // B may not release a claim it does not hold.
        dal.schedule_execution()
            .release_recovery_claim(exec.id, b)
            .await
            .unwrap();
        let row = dal.schedule_execution().get_by_id(exec.id).await.unwrap();
        assert_eq!(row.recovery_claimed_by, Some(a));

        dal.schedule_execution()
            .release_recovery_claim(exec.id, a)
            .await
            .unwrap();
        let row = dal.schedule_execution().get_by_id(exec.id).await.unwrap();
        assert!(row.recovery_claimed_by.is_none());
        assert!(row.recovery_heartbeat_at.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_recovery_claim_rejects_completed_row() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();
        dal.schedule_execution()
            .complete(exec.id, Utc::now())
            .await
            .unwrap();

        // `completed_at IS NULL` is part of the CAS: a loser holding a stale
        // snapshot cannot claim (and then re-fire) a finished handoff.
        assert_eq!(
            dal.schedule_execution()
                .claim_for_recovery(
                    exec.id,
                    UniversalUuid::new_v4(),
                    std::time::Duration::from_secs(120)
                )
                .await
                .unwrap(),
            RecoveryClaimResult::NotClaimed
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_stale_recovery_claim_can_be_taken_over() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();

        let dead = UniversalUuid::new_v4();
        let live = UniversalUuid::new_v4();
        dal.schedule_execution()
            .claim_for_recovery(exec.id, dead, std::time::Duration::from_secs(120))
            .await
            .unwrap();

        // A zero window makes the existing heartbeat stale — what a crashed
        // owner looks like once `claim_stale_after` has elapsed.
        assert_eq!(
            dal.schedule_execution()
                .claim_for_recovery(exec.id, live, std::time::Duration::ZERO)
                .await
                .unwrap(),
            RecoveryClaimResult::Claimed
        );

        // The evicted owner's heartbeat no longer lands.
        assert_eq!(
            dal.schedule_execution()
                .recovery_heartbeat(exec.id, dead)
                .await
                .unwrap(),
            RecoveryHeartbeatResult::ClaimLost
        );
        assert_eq!(
            dal.schedule_execution()
                .recovery_heartbeat(exec.id, live)
                .await
                .unwrap(),
            RecoveryHeartbeatResult::Ok
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_recovery_attempts_are_durable() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();
        assert_eq!(exec.recovery_attempts, 0);

        for expected in 1..=3 {
            assert_eq!(
                dal.schedule_execution()
                    .increment_recovery_attempts(exec.id)
                    .await
                    .unwrap(),
                expected
            );
        }

        let row = dal.schedule_execution().get_by_id(exec.id).await.unwrap();
        assert_eq!(row.recovery_attempts, 3);

        dal.schedule_execution()
            .reset_recovery_attempts(exec.id)
            .await
            .unwrap();
        let row = dal.schedule_execution().get_by_id(exec.id).await.unwrap();
        assert_eq!(row.recovery_attempts, 0);
    }

    // ── get_execution_stats ─────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_execution_stats_empty() {
        let dal = unique_dal().await;
        let since = Utc::now() - chrono::Duration::hours(1);

        let stats = dal
            .schedule_execution()
            .get_execution_stats(since)
            .await
            .unwrap();

        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.successful_executions, 0);
        assert_eq!(stats.lost_executions, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_execution_stats_with_data() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let since = Utc::now() - chrono::Duration::hours(1);

        // Create two executions
        let exec1 = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();
        dal.schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();

        // Link one to a real workflow execution (FK constraint requires it to exist)
        use crate::models::workflow_execution::NewWorkflowExecution;
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "stats-test".to_string(),
                workflow_version: "1.0".to_string(),
                status: "Completed".to_string(),
                context_id: None,
            })
            .await
            .unwrap();
        dal.schedule_execution()
            .update_workflow_execution_id(exec1.id, wf_exec.id)
            .await
            .unwrap();

        let stats = dal
            .schedule_execution()
            .get_execution_stats(since)
            .await
            .unwrap();

        assert_eq!(stats.total_executions, 2);
        assert_eq!(stats.successful_executions, 1);
        assert_eq!(stats.success_rate, 50.0);
    }
}
