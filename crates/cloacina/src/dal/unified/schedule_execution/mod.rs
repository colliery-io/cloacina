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

mod crud;

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::schedule::{NewScheduleExecution, ScheduleExecution};
use chrono::{DateTime, Utc};

/// Statistics about schedule execution performance
#[derive(Debug)]
pub struct ScheduleExecutionStats {
    /// Total number of executions attempted
    pub total_executions: i64,
    /// Number of executions that successfully handed off to pipeline executor
    pub successful_executions: i64,
    /// Number of executions that were lost (started but never completed within expected time)
    pub lost_executions: i64,
    /// Success rate as a percentage
    pub success_rate: f64,
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
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_execution).await,
            self.create_sqlite(new_execution).await
        )
    }

    /// Retrieves a schedule execution by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<ScheduleExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(id).await,
            self.get_by_id_sqlite(id).await
        )
    }

    /// Lists schedule executions for a given schedule.
    pub async fn list_by_schedule(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_schedule_postgres(schedule_id, limit, offset)
                .await,
            self.list_by_schedule_sqlite(schedule_id, limit, offset)
                .await
        )
    }

    /// Marks a schedule execution as completed.
    pub async fn complete(
        &self,
        id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.complete_postgres(id, completed_at).await,
            self.complete_sqlite(id, completed_at).await
        )
    }

    /// Checks if there is an active (uncompleted) execution for a schedule with the given context hash.
    pub async fn has_active_execution(
        &self,
        schedule_id: UniversalUuid,
        context_hash: &str,
    ) -> Result<bool, ValidationError> {
        let context_hash_owned = context_hash.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.has_active_execution_postgres(schedule_id, context_hash_owned.clone())
                .await,
            self.has_active_execution_sqlite(schedule_id, context_hash_owned)
                .await
        )
    }

    /// Updates the pipeline execution ID for a schedule execution.
    pub async fn update_pipeline_execution_id(
        &self,
        id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_pipeline_execution_id_postgres(id, pipeline_execution_id)
                .await,
            self.update_pipeline_execution_id_sqlite(id, pipeline_execution_id)
                .await
        )
    }

    /// Finds lost executions (started but not completed) older than the specified minutes.
    pub async fn find_lost_executions(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.find_lost_executions_postgres(older_than_minutes).await,
            self.find_lost_executions_sqlite(older_than_minutes).await
        )
    }

    /// Gets the latest execution for a given schedule.
    pub async fn get_latest_by_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<ScheduleExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_latest_by_schedule_postgres(schedule_id).await,
            self.get_latest_by_schedule_sqlite(schedule_id).await
        )
    }

    /// Gets execution statistics for monitoring and alerting.
    pub async fn get_execution_stats(
        &self,
        since: DateTime<Utc>,
    ) -> Result<ScheduleExecutionStats, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_execution_stats_postgres(since).await,
            self.get_execution_stats_sqlite(since).await
        )
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
            "sqlite:///tmp/sched_exec_test_{}.db?mode=rwc",
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
            pipeline_execution_id: None,
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
        assert!(exec.pipeline_execution_id.is_none());
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

    // ── update_pipeline_execution_id ────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_update_pipeline_execution_id() {
        let dal = unique_dal().await;
        let sched_id = create_schedule(&dal).await;
        let exec = dal
            .schedule_execution()
            .create(new_exec(sched_id))
            .await
            .unwrap();
        assert!(exec.pipeline_execution_id.is_none());

        // Create a real pipeline so the FK constraint is satisfied
        use crate::models::pipeline_execution::NewPipelineExecution;
        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "fk-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .unwrap();

        dal.schedule_execution()
            .update_pipeline_execution_id(exec.id, pipeline.id)
            .await
            .unwrap();

        let updated = dal.schedule_execution().get_by_id(exec.id).await.unwrap();
        assert_eq!(updated.pipeline_execution_id, Some(pipeline.id));
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

        // Link one to a real pipeline (FK constraint requires it to exist)
        use crate::models::pipeline_execution::NewPipelineExecution;
        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "stats-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Completed".to_string(),
                context_id: None,
            })
            .await
            .unwrap();
        dal.schedule_execution()
            .update_pipeline_execution_id(exec1.id, pipeline.id)
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
