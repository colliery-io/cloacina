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

//! Unified Schedule DAL with runtime backend selection
//!
//! This module provides operations for the unified `schedules` table that
//! replaces the separate `cron_schedules` and `trigger_schedules` tables.
//! Works with both PostgreSQL and SQLite backends, selecting the appropriate
//! implementation at runtime based on the database connection type.

mod crud;

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::schedule::{NewSchedule, Schedule};
use chrono::{DateTime, Utc};

/// Data access layer for unified schedule operations with runtime backend selection.
#[derive(Clone)]
pub struct ScheduleDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ScheduleDAL<'a> {
    /// Creates a new ScheduleDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new schedule record in the database.
    pub async fn create(&self, new_schedule: NewSchedule) -> Result<Schedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_schedule).await,
            self.create_sqlite(new_schedule).await
        )
    }

    /// Retrieves a schedule by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<Schedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(id).await,
            self.get_by_id_sqlite(id).await
        )
    }

    /// Lists schedules with optional filtering by type and enabled status.
    pub async fn list(
        &self,
        schedule_type: Option<&str>,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let schedule_type_owned = schedule_type.map(|s| s.to_string());
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_postgres(schedule_type_owned.clone(), enabled_only, limit, offset)
                .await,
            self.list_sqlite(schedule_type_owned, enabled_only, limit, offset)
                .await
        )
    }

    /// Enables a schedule.
    pub async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.enable_postgres(id).await,
            self.enable_sqlite(id).await
        )
    }

    /// Disables a schedule.
    pub async fn disable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.disable_postgres(id).await,
            self.disable_sqlite(id).await
        )
    }

    /// Deletes a schedule from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_postgres(id).await,
            self.delete_sqlite(id).await
        )
    }

    /// Retrieves all enabled cron schedules that are due for execution.
    pub async fn get_due_cron_schedules(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<Schedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_due_cron_schedules_postgres(now).await,
            self.get_due_cron_schedules_sqlite(now).await
        )
    }

    /// Atomically claims and updates a cron schedule's timing.
    pub async fn claim_and_update_cron(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.claim_and_update_cron_postgres(id, current_time, last_run, next_run)
                .await,
            self.claim_and_update_cron_sqlite(id, current_time, last_run, next_run)
                .await
        )
    }

    /// Updates the last run and next run times for a schedule.
    pub async fn update_schedule_times(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_schedule_times_postgres(id, last_run, next_run)
                .await,
            self.update_schedule_times_sqlite(id, last_run, next_run)
                .await
        )
    }

    /// Retrieves all enabled trigger schedules.
    pub async fn get_enabled_triggers(&self) -> Result<Vec<Schedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_enabled_triggers_postgres().await,
            self.get_enabled_triggers_sqlite().await
        )
    }

    /// Updates the last poll time for a trigger schedule.
    pub async fn update_last_poll(
        &self,
        id: UniversalUuid,
        last_poll_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_last_poll_postgres(id, last_poll_at).await,
            self.update_last_poll_sqlite(id, last_poll_at).await
        )
    }

    /// Upserts a trigger schedule by trigger_name.
    pub async fn upsert_trigger(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.upsert_trigger_postgres(new_schedule).await,
            self.upsert_trigger_sqlite(new_schedule).await
        )
    }

    /// Retrieves a schedule by its trigger name.
    pub async fn get_by_trigger_name(
        &self,
        name: &str,
    ) -> Result<Option<Schedule>, ValidationError> {
        let name_owned = name.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_trigger_name_postgres(name_owned.clone()).await,
            self.get_by_trigger_name_sqlite(name_owned).await
        )
    }

    /// Finds schedules by workflow name.
    pub async fn find_by_workflow(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let name_owned = workflow_name.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.find_by_workflow_postgres(name_owned.clone()).await,
            self.find_by_workflow_sqlite(name_owned).await
        )
    }

    /// Updates the cron expression and timezone for a cron schedule.
    pub async fn update_cron_expression_and_timezone(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let cron_expression_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_cron_expression_and_timezone_postgres(
                id,
                cron_expression_owned.clone(),
                timezone_owned.clone(),
                next_run
            )
            .await,
            self.update_cron_expression_and_timezone_sqlite(
                id,
                cron_expression_owned,
                timezone_owned,
                next_run
            )
            .await
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::UniversalTimestamp;
    use crate::database::Database;
    use crate::models::schedule::NewSchedule;
    use std::time::Duration;

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "sqlite:///tmp/sched_test_{}.db?mode=rwc",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    // ── create + get_by_id ──────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_cron_schedule() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();
        let new = NewSchedule::cron("my_workflow", "0 2 * * *", next_run);

        let schedule = dal.schedule().create(new).await.unwrap();

        assert_eq!(schedule.schedule_type, "cron");
        assert_eq!(schedule.workflow_name, "my_workflow");
        assert_eq!(schedule.cron_expression.as_deref(), Some("0 2 * * *"));
        assert_eq!(schedule.timezone.as_deref(), Some("UTC"));
        assert_eq!(schedule.catchup_policy.as_deref(), Some("skip"));
        assert!(schedule.is_enabled());
        assert!(schedule.is_cron());
        assert!(!schedule.is_trigger());
        assert!(schedule.next_run_at.is_some());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_trigger_schedule() {
        let dal = unique_dal().await;
        let new = NewSchedule::trigger("file_watcher", "process_files", Duration::from_secs(30));

        let schedule = dal.schedule().create(new).await.unwrap();

        assert_eq!(schedule.schedule_type, "trigger");
        assert_eq!(schedule.workflow_name, "process_files");
        assert_eq!(schedule.trigger_name.as_deref(), Some("file_watcher"));
        assert_eq!(schedule.poll_interval_ms, Some(30_000));
        assert!(schedule.is_trigger());
        assert!(schedule.cron_expression.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_by_id() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();
        let created = dal
            .schedule()
            .create(NewSchedule::cron("wf1", "*/5 * * * *", next_run))
            .await
            .unwrap();

        let fetched = dal.schedule().get_by_id(created.id).await.unwrap();

        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.workflow_name, "wf1");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let dal = unique_dal().await;
        let bogus_id = UniversalUuid::new_v4();
        let result = dal.schedule().get_by_id(bogus_id).await;
        assert!(result.is_err());
    }

    // ── list with filters ───────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_all() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();

        dal.schedule()
            .create(NewSchedule::cron("wf_a", "0 * * * *", next_run))
            .await
            .unwrap();
        dal.schedule()
            .create(NewSchedule::trigger(
                "trig_b",
                "wf_b",
                Duration::from_secs(10),
            ))
            .await
            .unwrap();

        let all = dal.schedule().list(None, false, 100, 0).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_by_schedule_type() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();

        dal.schedule()
            .create(NewSchedule::cron("wf_a", "0 * * * *", next_run))
            .await
            .unwrap();
        dal.schedule()
            .create(NewSchedule::trigger(
                "trig_b",
                "wf_b",
                Duration::from_secs(10),
            ))
            .await
            .unwrap();

        let crons = dal
            .schedule()
            .list(Some("cron"), false, 100, 0)
            .await
            .unwrap();
        assert_eq!(crons.len(), 1);
        assert_eq!(crons[0].schedule_type, "cron");

        let triggers = dal
            .schedule()
            .list(Some("trigger"), false, 100, 0)
            .await
            .unwrap();
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].schedule_type, "trigger");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_enabled_only() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();

        let s1 = dal
            .schedule()
            .create(NewSchedule::cron("wf_a", "0 * * * *", next_run))
            .await
            .unwrap();
        dal.schedule()
            .create(NewSchedule::cron("wf_b", "0 1 * * *", next_run))
            .await
            .unwrap();

        // Disable the first one
        dal.schedule().disable(s1.id).await.unwrap();

        let enabled = dal.schedule().list(None, true, 100, 0).await.unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].workflow_name, "wf_b");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_limit_and_offset() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();

        for i in 0..5 {
            dal.schedule()
                .create(NewSchedule::cron(
                    &format!("wf_{}", i),
                    "0 * * * *",
                    next_run,
                ))
                .await
                .unwrap();
        }

        let page1 = dal.schedule().list(None, false, 2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = dal.schedule().list(None, false, 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);

        let page3 = dal.schedule().list(None, false, 2, 4).await.unwrap();
        assert_eq!(page3.len(), 1);
    }

    // ── enable / disable ────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_enable_disable() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();
        let schedule = dal
            .schedule()
            .create(NewSchedule::cron("wf", "0 * * * *", next_run))
            .await
            .unwrap();
        assert!(schedule.is_enabled());

        dal.schedule().disable(schedule.id).await.unwrap();
        let disabled = dal.schedule().get_by_id(schedule.id).await.unwrap();
        assert!(!disabled.is_enabled());

        dal.schedule().enable(schedule.id).await.unwrap();
        let re_enabled = dal.schedule().get_by_id(schedule.id).await.unwrap();
        assert!(re_enabled.is_enabled());
    }

    // ── delete ──────────────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();
        let schedule = dal
            .schedule()
            .create(NewSchedule::cron("wf", "0 * * * *", next_run))
            .await
            .unwrap();

        dal.schedule().delete(schedule.id).await.unwrap();

        let result = dal.schedule().get_by_id(schedule.id).await;
        assert!(result.is_err());
    }

    // ── find_by_workflow ────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_find_by_workflow() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();

        dal.schedule()
            .create(NewSchedule::cron("shared_wf", "0 * * * *", next_run))
            .await
            .unwrap();
        dal.schedule()
            .create(NewSchedule::trigger(
                "trig",
                "shared_wf",
                Duration::from_secs(5),
            ))
            .await
            .unwrap();
        dal.schedule()
            .create(NewSchedule::cron("other_wf", "0 1 * * *", next_run))
            .await
            .unwrap();

        let results = dal.schedule().find_by_workflow("shared_wf").await.unwrap();
        assert_eq!(results.len(), 2);
        for s in &results {
            assert_eq!(s.workflow_name, "shared_wf");
        }
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_find_by_workflow_no_match() {
        let dal = unique_dal().await;
        let results = dal
            .schedule()
            .find_by_workflow("nonexistent")
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    // ── update_schedule_times ───────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_update_schedule_times() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();
        let schedule = dal
            .schedule()
            .create(NewSchedule::cron("wf", "0 * * * *", next_run))
            .await
            .unwrap();

        let last_run = Utc::now();
        let new_next_run = last_run + chrono::Duration::hours(1);

        dal.schedule()
            .update_schedule_times(schedule.id, last_run, new_next_run)
            .await
            .unwrap();

        let updated = dal.schedule().get_by_id(schedule.id).await.unwrap();
        assert!(updated.last_run_at.is_some());
        assert!(updated.next_run_at.is_some());
    }

    // ── get_due_cron_schedules ──────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_due_cron_schedules() {
        let dal = unique_dal().await;
        // Create a schedule that is already past due
        let past = UniversalTimestamp::from(Utc::now() - chrono::Duration::hours(1));
        let due_schedule = dal
            .schedule()
            .create(NewSchedule::cron("due_wf", "0 * * * *", past))
            .await
            .unwrap();

        // Create a schedule that is in the future
        let future = UniversalTimestamp::from(Utc::now() + chrono::Duration::hours(1));
        dal.schedule()
            .create(NewSchedule::cron("future_wf", "0 * * * *", future))
            .await
            .unwrap();

        // Create a trigger schedule (should not appear)
        dal.schedule()
            .create(NewSchedule::trigger(
                "trig",
                "trig_wf",
                Duration::from_secs(10),
            ))
            .await
            .unwrap();

        let due = dal
            .schedule()
            .get_due_cron_schedules(Utc::now())
            .await
            .unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, due_schedule.id);
    }

    // ── claim_and_update_cron ───────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_claim_and_update_cron() {
        let dal = unique_dal().await;
        let past = UniversalTimestamp::from(Utc::now() - chrono::Duration::hours(1));
        let schedule = dal
            .schedule()
            .create(NewSchedule::cron("wf", "0 * * * *", past))
            .await
            .unwrap();

        let now = Utc::now();
        let next = now + chrono::Duration::hours(1);
        let claimed = dal
            .schedule()
            .claim_and_update_cron(schedule.id, now, now, next)
            .await
            .unwrap();
        assert!(claimed);

        // Second claim should fail (next_run is now in the future)
        let claimed_again = dal
            .schedule()
            .claim_and_update_cron(schedule.id, now, now, next)
            .await
            .unwrap();
        assert!(!claimed_again);
    }

    // ── trigger-specific methods ────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_enabled_triggers() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();

        dal.schedule()
            .create(NewSchedule::trigger("trig1", "wf1", Duration::from_secs(5)))
            .await
            .unwrap();
        let s2 = dal
            .schedule()
            .create(NewSchedule::trigger(
                "trig2",
                "wf2",
                Duration::from_secs(10),
            ))
            .await
            .unwrap();
        // Cron should not appear
        dal.schedule()
            .create(NewSchedule::cron("cron_wf", "0 * * * *", next_run))
            .await
            .unwrap();

        // Disable one trigger
        dal.schedule().disable(s2.id).await.unwrap();

        let triggers = dal.schedule().get_enabled_triggers().await.unwrap();
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].trigger_name.as_deref(), Some("trig1"));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_update_last_poll() {
        let dal = unique_dal().await;
        let schedule = dal
            .schedule()
            .create(NewSchedule::trigger("trig", "wf", Duration::from_secs(5)))
            .await
            .unwrap();
        assert!(schedule.last_poll_at.is_none());

        let poll_time = Utc::now();
        dal.schedule()
            .update_last_poll(schedule.id, poll_time)
            .await
            .unwrap();

        let updated = dal.schedule().get_by_id(schedule.id).await.unwrap();
        assert!(updated.last_poll_at.is_some());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_by_trigger_name() {
        let dal = unique_dal().await;
        dal.schedule()
            .create(NewSchedule::trigger(
                "my_trigger",
                "my_wf",
                Duration::from_secs(10),
            ))
            .await
            .unwrap();

        let found = dal
            .schedule()
            .get_by_trigger_name("my_trigger")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().workflow_name, "my_wf");

        let not_found = dal
            .schedule()
            .get_by_trigger_name("nonexistent")
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_upsert_trigger_insert() {
        let dal = unique_dal().await;
        let new = NewSchedule::trigger("upsert_trig", "wf_v1", Duration::from_secs(5));
        let created = dal.schedule().upsert_trigger(new).await.unwrap();
        assert_eq!(created.workflow_name, "wf_v1");

        // Upsert again with changed workflow name
        let updated_new = NewSchedule::trigger("upsert_trig", "wf_v2", Duration::from_secs(15));
        let upserted = dal.schedule().upsert_trigger(updated_new).await.unwrap();
        assert_eq!(upserted.id, created.id); // same record
        assert_eq!(upserted.workflow_name, "wf_v2");
        assert_eq!(upserted.poll_interval_ms, Some(15_000));
    }

    // ── update_cron_expression_and_timezone ──────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_update_cron_expression_and_timezone() {
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();
        let schedule = dal
            .schedule()
            .create(NewSchedule::cron("wf", "0 * * * *", next_run))
            .await
            .unwrap();

        let new_next_run = Utc::now() + chrono::Duration::hours(2);
        dal.schedule()
            .update_cron_expression_and_timezone(
                schedule.id,
                Some("30 2 * * *"),
                Some("US/Eastern"),
                new_next_run,
            )
            .await
            .unwrap();

        let updated = dal.schedule().get_by_id(schedule.id).await.unwrap();
        assert_eq!(updated.cron_expression.as_deref(), Some("30 2 * * *"));
        assert_eq!(updated.timezone.as_deref(), Some("US/Eastern"));
    }
}
