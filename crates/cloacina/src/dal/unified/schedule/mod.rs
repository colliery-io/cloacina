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

use diesel::prelude::*;

use super::DAL;
use crate::dal::unified::models::{NewUnifiedSchedule, UnifiedSchedule};
use crate::database::schema::unified::schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
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
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedSchedule {
            id,
            schedule_type: new_schedule.schedule_type,
            workflow_name: new_schedule.workflow_name,
            enabled: new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true)),
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone,
            catchup_policy: new_schedule.catchup_policy,
            start_date: new_schedule.start_date,
            end_date: new_schedule.end_date,
            trigger_name: new_schedule.trigger_name,
            poll_interval_ms: new_schedule.poll_interval_ms,
            allow_concurrent: new_schedule.allow_concurrent,
            next_run_at: new_schedule.next_run_at,
            created_at: now,
            updated_at: now,
            params: new_schedule.params,
            instance_name: new_schedule.instance_name,
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(schedules::table)
                .values(&new_unified)
                .execute(conn)
        })?;

        let result: UnifiedSchedule =
            crate::interact_on_backend!(self.dal, |conn| schedules::table.find(id).first(conn))?;

        Ok(result.into())
    }

    /// Retrieves a schedule by its ID.
    /// Look up a NAMED instance schedule by `(workflow_name, instance_name)`
    /// (CLOACI-I-0116). `Ok(None)` when no such instance exists.
    pub async fn find_by_instance_name(
        &self,
        workflow_name: &str,
        instance_name: &str,
    ) -> Result<Option<Schedule>, ValidationError> {
        let wf = workflow_name.to_string();
        let inst = instance_name.to_string();
        let result: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::workflow_name.eq(wf))
                .filter(schedules::instance_name.eq(inst))
                .first(conn)
                .optional()
        })?;
        Ok(result.map(Into::into))
    }

    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<Schedule, ValidationError> {
        let result: UnifiedSchedule =
            crate::interact_on_backend!(self.dal, |conn| schedules::table.find(id).first(conn))?;
        Ok(result.into())
    }

    /// Lists schedules with optional filtering by type and enabled status.
    pub async fn list(
        &self,
        schedule_type: Option<&str>,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let schedule_type = schedule_type.map(|s| s.to_string());
        let enabled_true = UniversalBool::from(true);
        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            let mut query = schedules::table.into_boxed();

            if let Some(ref stype) = schedule_type {
                query = query.filter(schedules::schedule_type.eq(stype));
            }

            if enabled_only {
                query = query.filter(schedules::enabled.eq(enabled_true));
            }

            query
                .order(schedules::created_at.desc())
                .limit(limit)
                .offset(offset)
                .load(conn)
        })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    /// Enables a schedule.
    pub async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::enabled.eq(enabled_true),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Disables a schedule.
    pub async fn disable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let enabled_false = UniversalBool::from(false);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::enabled.eq(enabled_false),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Pauses a schedule (CLOACI-T-0749): the scheduler stops firing it until
    /// resumed. Distinct from `disable` — pause is transient operator state.
    pub async fn pause(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let paused_true = UniversalBool::from(true);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::paused.eq(paused_true),
                    schedules::paused_at.eq(Some(now)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Resumes a paused schedule (CLOACI-T-0749): clears the paused flag so the
    /// scheduler fires it again on its normal schedule. Missed fires are not
    /// caught up (skip policy).
    pub async fn resume(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let paused_false = UniversalBool::from(false);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::paused.eq(paused_false),
                    schedules::paused_at.eq(None::<UniversalTimestamp>),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Deletes a schedule from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::interact_on_backend!(self.dal, |conn| diesel::delete(schedules::table.find(id))
            .execute(conn))?;

        Ok(())
    }

    /// Retrieves all enabled cron schedules that are due for execution.
    pub async fn get_due_cron_schedules(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let now_ts = UniversalTimestamp::from(now);
        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::schedule_type.eq("cron"))
                .filter(schedules::enabled.eq(enabled_true))
                .filter(schedules::paused.eq(UniversalBool::from(false)))
                .filter(schedules::next_run_at.le(Some(now_ts)))
                .order(schedules::next_run_at.asc())
                .load(conn)
        })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    /// Earliest `next_run_at` over all enabled cron schedules, or `None` when
    /// there are none. Powers the timer-driven scheduler's sleep-until-next-due
    /// loop (CLOACI-T-0743) — the scheduler sleeps until this instant instead of
    /// polling on a fixed interval.
    pub async fn next_cron_due_time(&self) -> Result<Option<DateTime<Utc>>, ValidationError> {
        let enabled_true = UniversalBool::from(true);

        let earliest: Option<UniversalTimestamp> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::schedule_type.eq("cron"))
                .filter(schedules::enabled.eq(enabled_true))
                .filter(schedules::paused.eq(UniversalBool::from(false)))
                .filter(schedules::next_run_at.is_not_null())
                .order(schedules::next_run_at.asc())
                .select(schedules::next_run_at)
                .first::<Option<UniversalTimestamp>>(conn)
                .optional()
        })?
        .flatten();

        Ok(earliest.map(DateTime::<Utc>::from))
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
        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::last_run_at.eq(Some(last_run_ts)),
                    schedules::next_run_at.eq(Some(next_run_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Retrieves all enabled trigger schedules.
    pub async fn get_enabled_triggers(&self) -> Result<Vec<Schedule>, ValidationError> {
        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::schedule_type.eq("trigger"))
                .filter(schedules::enabled.eq(enabled_true))
                .filter(schedules::paused.eq(UniversalBool::from(false)))
                .order(schedules::created_at.asc())
                .load(conn)
        })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    /// Updates the last poll time for a trigger schedule.
    pub async fn update_last_poll(
        &self,
        id: UniversalUuid,
        last_poll_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let last_poll_ts = UniversalTimestamp::from(last_poll_at);
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::last_poll_at.eq(Some(last_poll_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Upserts a trigger schedule by trigger_name.
    pub async fn upsert_trigger(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        let trigger_name =
            new_schedule
                .trigger_name
                .clone()
                .ok_or_else(|| ValidationError::DatabaseQuery {
                    message: "trigger_name is required for upsert_trigger".to_string(),
                })?;

        let trigger_name_check = trigger_name.clone();

        // Check if a schedule with this trigger_name already exists
        let existing: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::trigger_name.eq(trigger_name_check))
                .filter(schedules::schedule_type.eq("trigger"))
                .first(conn)
                .optional()
        })?;

        if let Some(existing) = existing {
            // Update existing schedule
            let now = UniversalTimestamp::now();
            let existing_id = existing.id;
            let workflow_name = new_schedule.workflow_name;
            let poll_interval_ms = new_schedule.poll_interval_ms;
            let allow_concurrent = new_schedule.allow_concurrent;
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true));

            crate::interact_on_backend!(self.dal, |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set((
                        schedules::workflow_name.eq(workflow_name),
                        schedules::poll_interval_ms.eq(poll_interval_ms),
                        schedules::allow_concurrent.eq(allow_concurrent),
                        schedules::enabled.eq(enabled),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })?;

            let result: UnifiedSchedule = crate::interact_on_backend!(self.dal, |conn| {
                schedules::table.find(existing_id).first(conn)
            })?;

            Ok(result.into())
        } else {
            // Insert new schedule
            let id = UniversalUuid::new_v4();
            let now = UniversalTimestamp::now();

            let new_unified = NewUnifiedSchedule {
                id,
                schedule_type: new_schedule.schedule_type,
                workflow_name: new_schedule.workflow_name,
                enabled: new_schedule
                    .enabled
                    .unwrap_or_else(|| UniversalBool::from(true)),
                cron_expression: new_schedule.cron_expression,
                timezone: new_schedule.timezone,
                catchup_policy: new_schedule.catchup_policy,
                start_date: new_schedule.start_date,
                end_date: new_schedule.end_date,
                trigger_name: new_schedule.trigger_name,
                poll_interval_ms: new_schedule.poll_interval_ms,
                allow_concurrent: new_schedule.allow_concurrent,
                next_run_at: new_schedule.next_run_at,
                created_at: now,
                updated_at: now,
                params: new_schedule.params,
                instance_name: new_schedule.instance_name,
            };

            crate::interact_on_backend!(self.dal, |conn| {
                diesel::insert_into(schedules::table)
                    .values(&new_unified)
                    .execute(conn)
            })?;

            let result: UnifiedSchedule =
                crate::interact_on_backend!(self.dal, |conn| schedules::table
                    .find(id)
                    .first(conn))?;

            Ok(result.into())
        }
    }

    /// Upserts a cron schedule by its identity (workflow_name, cron_expression,
    /// timezone). Idempotent: re-registering the same packaged cron trigger
    /// (e.g. on every reconcile re-load) updates the existing row's next-run
    /// time instead of inserting a duplicate (CLOACI-T-0669). Returns the
    /// resulting schedule.
    pub async fn upsert_cron(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        // Identity of a cron schedule: (workflow_name, cron_expression,
        // timezone). Re-registering the same packaged cron on a reconcile
        // re-load updates the existing row instead of inserting a duplicate.
        let cron_expression =
            new_schedule
                .cron_expression
                .clone()
                .ok_or_else(|| ValidationError::DatabaseQuery {
                    message: "cron_expression is required for upsert_cron".to_string(),
                })?;
        let workflow_name = new_schedule.workflow_name.clone();
        let timezone = new_schedule.timezone.clone();

        let (wf, cron, tz) = (
            workflow_name.clone(),
            cron_expression.clone(),
            timezone.clone(),
        );
        let existing: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            let mut q = schedules::table
                .filter(schedules::schedule_type.eq("cron"))
                .filter(schedules::workflow_name.eq(wf))
                .filter(schedules::cron_expression.eq(cron))
                .into_boxed();
            q = match tz {
                Some(t) => q.filter(schedules::timezone.eq(t)),
                None => q.filter(schedules::timezone.is_null()),
            };
            q.first(conn).optional()
        })?;

        if let Some(existing) = existing {
            let now = UniversalTimestamp::now();
            let existing_id = existing.id;
            let next_run_at = new_schedule.next_run_at;
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true));

            crate::interact_on_backend!(self.dal, |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set((
                        schedules::next_run_at.eq(next_run_at),
                        schedules::enabled.eq(enabled),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })?;

            let result: UnifiedSchedule = crate::interact_on_backend!(self.dal, |conn| {
                schedules::table.find(existing_id).first(conn)
            })?;
            Ok(result.into())
        } else {
            let id = UniversalUuid::new_v4();
            let now = UniversalTimestamp::now();
            let new_unified = NewUnifiedSchedule {
                id,
                schedule_type: new_schedule.schedule_type,
                workflow_name: new_schedule.workflow_name,
                enabled: new_schedule
                    .enabled
                    .unwrap_or_else(|| UniversalBool::from(true)),
                cron_expression: new_schedule.cron_expression,
                timezone: new_schedule.timezone,
                catchup_policy: new_schedule.catchup_policy,
                start_date: new_schedule.start_date,
                end_date: new_schedule.end_date,
                trigger_name: new_schedule.trigger_name,
                poll_interval_ms: new_schedule.poll_interval_ms,
                allow_concurrent: new_schedule.allow_concurrent,
                next_run_at: new_schedule.next_run_at,
                created_at: now,
                updated_at: now,
                params: new_schedule.params,
                instance_name: new_schedule.instance_name,
            };
            crate::interact_on_backend!(self.dal, |conn| {
                diesel::insert_into(schedules::table)
                    .values(&new_unified)
                    .execute(conn)
            })?;

            let result: UnifiedSchedule =
                crate::interact_on_backend!(self.dal, |conn| schedules::table
                    .find(id)
                    .first(conn))?;
            Ok(result.into())
        }
    }

    /// Retrieves a schedule by its trigger name.
    pub async fn get_by_trigger_name(
        &self,
        name: &str,
    ) -> Result<Option<Schedule>, ValidationError> {
        let name_owned = name.to_string();
        let result: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::trigger_name.eq(name_owned))
                .filter(schedules::schedule_type.eq("trigger"))
                .first(conn)
                .optional()
        })?;

        Ok(result.map(|r| r.into()))
    }

    /// Finds schedules by workflow name.
    pub async fn find_by_workflow(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let name_owned = workflow_name.to_string();
        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::workflow_name.eq(name_owned))
                .order(schedules::created_at.desc())
                .load(conn)
        })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
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
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::cron_expression.eq(cron_expression_owned),
                    schedules::timezone.eq(timezone_owned),
                    schedules::next_run_at.eq(Some(next_run_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
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
            "file:sched_test_{}?mode=memory&cache=shared",
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
    async fn test_upsert_cron_is_idempotent() {
        // CLOACI-T-0669: re-registering the same packaged cron (same workflow +
        // cron + timezone) must update the existing row, not insert a duplicate.
        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();

        let first = dal
            .schedule()
            .upsert_cron(NewSchedule::cron("wf", "*/15 * * * * *", next_run))
            .await
            .unwrap();
        // A second register (as the reconciler does on every re-load) — same id.
        let second = dal
            .schedule()
            .upsert_cron(NewSchedule::cron("wf", "*/15 * * * * *", next_run))
            .await
            .unwrap();
        assert_eq!(first.id, second.id, "re-register must reuse the same row");

        let crons = dal
            .schedule()
            .list(Some("cron"), false, 100, 0)
            .await
            .unwrap();
        assert_eq!(
            crons.len(),
            1,
            "exactly one cron schedule after re-register"
        );

        // A different cron expression for the same workflow is a distinct
        // schedule, so it inserts a new row rather than overwriting.
        dal.schedule()
            .upsert_cron(NewSchedule::cron("wf", "0 0 * * *", next_run))
            .await
            .unwrap();
        let crons = dal
            .schedule()
            .list(Some("cron"), false, 100, 0)
            .await
            .unwrap();
        assert_eq!(
            crons.len(),
            2,
            "a different cron expression is a new schedule"
        );
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

    // ── pause / resume (CLOACI-T-0749) ──────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_pause_resume_excludes_due_cron() {
        let dal = unique_dal().await;
        // A past-due cron is normally returned by get_due_cron_schedules.
        let past = UniversalTimestamp::from(Utc::now() - chrono::Duration::hours(1));
        let sched = dal
            .schedule()
            .create(NewSchedule::cron("paused_wf", "0 * * * *", past))
            .await
            .unwrap();
        assert!(!sched.is_paused());

        // Pausing removes it from the due set without disabling it.
        dal.schedule().pause(sched.id).await.unwrap();
        let paused = dal.schedule().get_by_id(sched.id).await.unwrap();
        assert!(paused.is_paused());
        assert!(paused.is_enabled(), "pause must not flip enabled");
        assert!(!paused.is_active(), "paused schedule is not active");
        assert!(paused.paused_at.is_some());

        let due = dal
            .schedule()
            .get_due_cron_schedules(Utc::now())
            .await
            .unwrap();
        assert!(
            due.iter().all(|s| s.id != sched.id),
            "paused cron must be excluded from the due set"
        );
        // ...and from the timer's next-due computation (busy-loop guard).
        let next = dal.schedule().next_cron_due_time().await.unwrap();
        assert!(
            next.is_none(),
            "paused cron must not drive the next-due timer"
        );

        // Resuming restores it.
        dal.schedule().resume(sched.id).await.unwrap();
        let resumed = dal.schedule().get_by_id(sched.id).await.unwrap();
        assert!(!resumed.is_paused());
        assert!(resumed.paused_at.is_none());
        let due = dal
            .schedule()
            .get_due_cron_schedules(Utc::now())
            .await
            .unwrap();
        assert!(due.iter().any(|s| s.id == sched.id));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_pause_excludes_enabled_trigger() {
        let dal = unique_dal().await;
        let sched = dal
            .schedule()
            .create(NewSchedule::trigger("pt", "wf", Duration::from_secs(5)))
            .await
            .unwrap();

        dal.schedule().pause(sched.id).await.unwrap();
        let triggers = dal.schedule().get_enabled_triggers().await.unwrap();
        assert!(
            triggers.iter().all(|s| s.id != sched.id),
            "paused trigger must be excluded from the fired set"
        );

        dal.schedule().resume(sched.id).await.unwrap();
        let triggers = dal.schedule().get_enabled_triggers().await.unwrap();
        assert!(triggers.iter().any(|s| s.id == sched.id));
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
