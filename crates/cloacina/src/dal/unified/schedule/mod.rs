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

mod backfill;
mod crud;

use diesel::prelude::*;

use super::DAL;
use crate::dal::unified::models::{NewUnifiedSchedule, UnifiedSchedule};
use crate::database::schema::unified::schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::schedule::{CronScheduleOptions, NewSchedule, Schedule};
use chrono::{DateTime, Utc};

/// One fire time that a claim of a cron schedule may take, with the values
/// the schedule row gets when this fire is the last one taken
/// (CLOACINA-T-1020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronFireCandidate {
    /// The fire time. It becomes `scheduled_time` of the schedule execution.
    pub scheduled_time: DateTime<Utc>,
    /// `schedules.last_run_at` when this is the last fire taken.
    pub last_run_at: DateTime<Utc>,
    /// `schedules.next_run_at` when this is the last fire taken. It may be in
    /// the past: the schedule then stays due and the next pass goes on.
    pub next_run_at: DateTime<Utc>,
}

/// A fire that a claim took: its schedule execution row exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedCronFire {
    /// Id of the schedule execution row.
    pub schedule_execution_id: UniversalUuid,
    /// The fire time.
    pub scheduled_time: DateTime<Utc>,
}

/// Result of [`ScheduleDAL::claim_cron_fires`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronClaimOutcome {
    /// The fires taken, oldest first: a prefix of the candidates.
    Claimed(Vec<ClaimedCronFire>),
    /// The schedule is not due, not active, or its row changed after the
    /// caller read it (another scheduler took the fires). Nothing changed.
    NotClaimed,
    /// The schedule has as many active runs as its limit allows. Nothing
    /// changed: the schedule stays due.
    AtLimit {
        /// Active runs counted under the claim.
        active: i64,
    },
}

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
    ///
    /// One anonymous cron schedule (no `instance_name`) can exist per
    /// workflow name, cron expression and timezone: a second `create` with
    /// the same three is a unique violation (index
    /// `idx_schedules_cron_identity`, CLOACINA-T-1042). To register such a
    /// schedule again, use [`upsert_cron`](Self::upsert_cron).
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
            max_active_runs: new_schedule.max_active_runs,
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
    ///
    /// With no type, the list holds the cron and trigger schedules. A
    /// backfill (CLOACINA-T-1020) is not a live schedule: ask for the type
    /// `"backfill"`, or use `list_backfills`.
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

            match schedule_type {
                Some(ref stype) => query = query.filter(schedules::schedule_type.eq(stype)),
                None => query = query.filter(schedules::schedule_type.ne("backfill")),
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

    /// Takes the next fires of a due cron schedule, or of a due backfill, in
    /// one transaction (CLOACINA-T-1020).
    ///
    /// `candidates` are the fire times the caller wants, oldest first. The
    /// claim takes as many of them as the limit on active runs allows, writes
    /// one schedule execution row for each, and moves `last_run_at` and
    /// `next_run_at` to the values of the last fire it took. So a fire that is
    /// not taken is not lost: the schedule row still points at it.
    ///
    /// Three things make this safe with more than one scheduler:
    ///
    /// * The first statement is an UPDATE of the schedule row that requires
    ///   `next_run_at` to equal `expected_next_run`, the value the caller
    ///   read. A scheduler that computed its candidates from an older row
    ///   matches nothing. (`next_run_at <= now` alone is not enough: a
    ///   schedule with a backlog stays due after a claim.)
    /// * That UPDATE locks the row until the transaction ends, so the count
    ///   of active runs and the new rows of two schedulers cannot interleave.
    /// * The rows are written in the same transaction, so a later claim
    ///   counts them even before the runs are handed off.
    ///
    /// A run is active while its schedule execution row is open
    /// (`completed_at IS NULL`) and is not linked to a workflow execution in
    /// a terminal state. A row with no link counts: its handoff is in flight,
    /// or it waits for the cron recovery sweep.
    pub async fn claim_cron_fires(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        expected_next_run: DateTime<Utc>,
        candidates: Vec<CronFireCandidate>,
        max_active_runs: Option<i64>,
    ) -> Result<CronClaimOutcome, ValidationError> {
        use crate::dal::unified::models::NewUnifiedScheduleExecution;
        use crate::database::schema::unified::{schedule_executions, workflow_executions};

        if candidates.is_empty() {
            return Ok(CronClaimOutcome::NotClaimed);
        }

        let current_ts = UniversalTimestamp::from(current_time);
        let expected_ts = UniversalTimestamp::from(expected_next_run);
        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        let outcome = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                // Lock the row and check that it is the row the caller read.
                // The assignment changes nothing.
                let locked = diesel::update(schedules::table.find(id))
                    .filter(schedules::schedule_type.eq_any(["cron", "backfill"]))
                    .filter(schedules::next_run_at.eq(Some(expected_ts)))
                    .filter(schedules::next_run_at.le(Some(current_ts)))
                    .filter(schedules::enabled.eq(enabled_true))
                    .filter(schedules::paused.eq(UniversalBool::from(false)))
                    .set(schedules::workflow_name.eq(schedules::workflow_name))
                    .execute(conn)?;
                if locked != 1 {
                    return Ok(CronClaimOutcome::NotClaimed);
                }

                let take = match max_active_runs {
                    None => candidates.len(),
                    Some(limit) => {
                        let active: i64 = schedule_executions::table
                            .left_join(
                                workflow_executions::table
                                    .on(schedule_executions::workflow_execution_id
                                        .eq(workflow_executions::id.nullable())),
                            )
                            .filter(schedule_executions::schedule_id.eq(id))
                            .filter(schedule_executions::completed_at.is_null())
                            .filter(
                                schedule_executions::workflow_execution_id.is_null().or(
                                    workflow_executions::status
                                        .nullable()
                                        .ne("Completed")
                                        .and(workflow_executions::status.nullable().ne("Failed"))
                                        .and(
                                            workflow_executions::status.nullable().ne("Cancelled"),
                                        ),
                                ),
                            )
                            .count()
                            .get_result(conn)?;
                        let free = (limit - active).max(0) as usize;
                        if free == 0 {
                            return Ok(CronClaimOutcome::AtLimit { active });
                        }
                        free.min(candidates.len())
                    }
                };

                let mut claimed = Vec::with_capacity(take);
                for candidate in &candidates[..take] {
                    let row_id = UniversalUuid::new_v4();
                    let row = NewUnifiedScheduleExecution {
                        id: row_id,
                        schedule_id: id,
                        workflow_execution_id: None,
                        scheduled_time: Some(UniversalTimestamp::from(candidate.scheduled_time)),
                        claimed_at: Some(now),
                        context_hash: None,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    };
                    diesel::insert_into(schedule_executions::table)
                        .values(&row)
                        .execute(conn)?;
                    claimed.push(ClaimedCronFire {
                        schedule_execution_id: row_id,
                        scheduled_time: candidate.scheduled_time,
                    });
                }

                let last = &candidates[take - 1];
                diesel::update(schedules::table.find(id))
                    .set((
                        schedules::last_run_at.eq(Some(UniversalTimestamp::from(last.last_run_at))),
                        schedules::next_run_at.eq(Some(UniversalTimestamp::from(last.next_run_at))),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                Ok(CronClaimOutcome::Claimed(claimed))
            })
        })?;

        Ok(outcome)
    }

    /// The earliest `next_run_at` of the enabled, not paused cron schedules,
    /// leaving out the schedules in `excluded` (CLOACINA-T-1020). The
    /// scheduler leaves out the schedules it holds at their limit on active
    /// runs: they stay due, and their due time must not hide the next fire of
    /// the others.
    pub async fn next_cron_due_time_excluding(
        &self,
        excluded: Vec<UniversalUuid>,
    ) -> Result<Option<DateTime<Utc>>, ValidationError> {
        let enabled_true = UniversalBool::from(true);

        let earliest: Option<UniversalTimestamp> = crate::interact_on_backend!(self.dal, |conn| {
            let mut query = schedules::table
                .filter(schedules::schedule_type.eq("cron"))
                .filter(schedules::enabled.eq(enabled_true))
                .filter(schedules::paused.eq(UniversalBool::from(false)))
                .filter(schedules::next_run_at.is_not_null())
                .into_boxed();
            for id in excluded {
                query = query.filter(schedules::id.ne(id));
            }
            query
                .order(schedules::next_run_at.asc())
                .select(schedules::next_run_at)
                .first::<Option<UniversalTimestamp>>(conn)
                .optional()
        })?
        .flatten();

        Ok(earliest.map(DateTime::<Utc>::from))
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
                max_active_runs: new_schedule.max_active_runs,
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
    /// (e.g. on every reconcile re-load) keeps the existing row, with its
    /// `next_run_at` and `last_run_at`, instead of inserting a duplicate
    /// (CLOACI-T-0669, CLOACINA-T-1041). Returns the resulting schedule.
    pub async fn upsert_cron(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        self.upsert_cron_declared(new_schedule, &CronScheduleOptions::default())
            .await
    }

    /// [`upsert_cron`](Self::upsert_cron) with what the declaration sets on
    /// the schedule (CLOACINA-T-1020). A new row gets the declared values. An
    /// existing row gets each value that is declared and keeps the others, so
    /// a re-load of a package that declares no catch-up policy does not undo
    /// a policy that an operator set on the row.
    ///
    /// Two callers at the same moment (two server replicas that load one
    /// package) get the same row: the insert is `ON CONFLICT DO NOTHING` on
    /// the unique index `idx_schedules_cron_identity`, and the caller whose
    /// insert wrote nothing reads the row of the other (CLOACINA-T-1042).
    pub async fn upsert_cron_declared(
        &self,
        mut new_schedule: NewSchedule,
        options: &CronScheduleOptions,
    ) -> Result<Schedule, ValidationError> {
        options.apply_to(&mut new_schedule);
        let declared_catchup = options
            .catchup_policy
            .as_ref()
            .map(|policy| policy.as_str().to_string());
        let declared_max_active_runs = options.max_active_runs;
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
        let next_run_at = new_schedule.next_run_at;
        let enabled = new_schedule
            .enabled
            .unwrap_or_else(|| UniversalBool::from(true));

        let existing = match self
            .find_cron_by_identity(&workflow_name, &cron_expression, timezone.as_deref())
            .await?
        {
            Some(existing) => existing,
            None => {
                let id = UniversalUuid::new_v4();
                let now = UniversalTimestamp::now();
                let new_unified = NewUnifiedSchedule {
                    id,
                    schedule_type: new_schedule.schedule_type,
                    workflow_name: new_schedule.workflow_name,
                    enabled,
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
                    max_active_runs: new_schedule.max_active_runs,
                };
                if self.insert_cron_if_absent(new_unified).await? {
                    let result: UnifiedSchedule =
                        crate::interact_on_backend!(self.dal, |conn| schedules::table
                            .find(id)
                            .first(conn))?;
                    return Ok(result.into());
                }
                // Another caller inserted the row between the read and the
                // insert. Go on with its row, as for a row that existed.
                self.find_cron_by_identity(&workflow_name, &cron_expression, timezone.as_deref())
                    .await?
                    .ok_or_else(|| ValidationError::DatabaseQuery {
                        message: format!(
                            "cron schedule of workflow '{}' was removed while it was registered",
                            workflow_name
                        ),
                    })?
            }
        };

        let now = UniversalTimestamp::now();
        let existing_id = existing.id;

        // `next_run_at` and `last_run_at` of a row that exists are the
        // state of the schedule: a registration does not move them
        // (CLOACINA-T-1041). A load after a restart would else erase the
        // fires that were missed. Only a row with no `next_run_at` gets
        // the new one.
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(existing_id))
                .set((
                    schedules::enabled.eq(enabled),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(existing_id))
                .filter(schedules::next_run_at.is_null())
                .set(schedules::next_run_at.eq(next_run_at))
                .execute(conn)
        })?;

        if let Some(policy) = declared_catchup {
            crate::interact_on_backend!(self.dal, |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set(schedules::catchup_policy.eq(Some(policy)))
                    .execute(conn)
            })?;
        }
        if let Some(limit) = declared_max_active_runs {
            crate::interact_on_backend!(self.dal, |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set(schedules::max_active_runs.eq(Some(limit)))
                    .execute(conn)
            })?;
        }

        let result: UnifiedSchedule = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table.find(existing_id).first(conn)
        })?;
        Ok(result.into())
    }

    /// The anonymous cron schedule with this identity: workflow name, cron
    /// expression and timezone (no timezone is UTC). A named instance and a
    /// row of another schedule type are never it. With more than one row (a
    /// schema that does not have the unique index yet), the oldest.
    async fn find_cron_by_identity(
        &self,
        workflow_name: &str,
        cron_expression: &str,
        timezone: Option<&str>,
    ) -> Result<Option<UnifiedSchedule>, ValidationError> {
        let wf = workflow_name.to_string();
        let cron = cron_expression.to_string();
        let tz = timezone.unwrap_or("UTC").to_string();
        let existing: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            let mut q = schedules::table
                .filter(schedules::schedule_type.eq("cron"))
                .filter(schedules::instance_name.is_null())
                .filter(schedules::workflow_name.eq(wf))
                .filter(schedules::cron_expression.eq(cron))
                .into_boxed();
            q = if tz == "UTC" {
                q.filter(schedules::timezone.is_null().or(schedules::timezone.eq(tz)))
            } else {
                q.filter(schedules::timezone.eq(tz))
            };
            q.order((schedules::created_at.asc(), schedules::id.asc()))
                .first(conn)
                .optional()
        })?;
        Ok(existing)
    }

    /// Inserts an anonymous cron schedule unless a row with its identity is
    /// there (`ON CONFLICT DO NOTHING`, on both backends). Returns whether
    /// the row was written. The unique index `idx_schedules_cron_identity`
    /// (migration 060) is what makes a second insert a conflict.
    async fn insert_cron_if_absent(
        &self,
        new_unified: NewUnifiedSchedule,
    ) -> Result<bool, ValidationError> {
        let inserted: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(schedules::table)
                .values(&new_unified)
                .on_conflict_do_nothing()
                .execute(conn)
        })?;
        Ok(inserted == 1)
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
    use crate::database::Database;
    use crate::database::universal_types::UniversalTimestamp;
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

    // ── named instances (CLOACI-T-0894 server surface) ──────────────

    /// The load-bearing safety property of an UNSCHEDULED instance: the server
    /// route creates it with `next_run_at = NULL`, and the scheduler's due
    /// query must never select it. `NULL <= now` is never true in SQL, but the
    /// whole "optional cron" design rests on that, so assert it rather than
    /// reason about it.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn unscheduled_instance_is_never_due() {
        let dal = unique_dal().await;

        let mut unscheduled = NewSchedule::cron("wf", "", UniversalTimestamp::now());
        unscheduled.cron_expression = None;
        unscheduled.next_run_at = None;
        unscheduled.instance_name = Some("nightly_params".to_string());
        unscheduled.params = Some(r#"{"mode":"copy"}"#.to_string());
        let created = dal.schedule().create(unscheduled).await.unwrap();
        assert!(created.next_run_at.is_none());
        assert!(created.is_enabled(), "enabled reflects the request");

        // Far-future `now` — a scheduled row would certainly be due by then.
        let due = dal
            .schedule()
            .get_due_cron_schedules(chrono::Utc::now() + chrono::Duration::days(3650))
            .await
            .unwrap();
        assert!(
            !due.iter().any(|s| s.id == created.id),
            "an unscheduled instance must never be selected as due"
        );
    }

    /// A scheduled instance keeps its params and instance_name through the
    /// round trip and IS selectable as due — the counterpart to the above, so
    /// the test above can't pass merely because nothing is ever due.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn scheduled_instance_round_trips_and_becomes_due() {
        let dal = unique_dal().await;

        let mut scheduled = NewSchedule::cron(
            "wf",
            "0 * * * *",
            UniversalTimestamp(chrono::Utc::now() - chrono::Duration::minutes(1)),
        );
        scheduled.instance_name = Some("prod".to_string());
        scheduled.params = Some(r#"{"source":"/data"}"#.to_string());
        let created = dal.schedule().create(scheduled).await.unwrap();

        let found = dal
            .schedule()
            .find_by_instance_name("wf", "prod")
            .await
            .unwrap()
            .expect("instance should be findable by name");
        assert_eq!(found.id, created.id);
        assert_eq!(found.params.as_deref(), Some(r#"{"source":"/data"}"#));

        let due = dal
            .schedule()
            .get_due_cron_schedules(chrono::Utc::now())
            .await
            .unwrap();
        assert!(
            due.iter().any(|s| s.id == created.id),
            "a past-due scheduled instance must be selected"
        );
    }

    /// The list endpoint filters to named instances; anonymous schedules for
    /// the same workflow must not leak into it.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn find_by_workflow_distinguishes_named_from_anonymous() {
        let dal = unique_dal().await;

        let anon = NewSchedule::cron("wf", "0 * * * *", UniversalTimestamp::now());
        dal.schedule().create(anon).await.unwrap();

        let mut named = NewSchedule::cron("wf", "0 * * * *", UniversalTimestamp::now());
        named.instance_name = Some("prod".to_string());
        dal.schedule().create(named).await.unwrap();

        let all = dal.schedule().find_by_workflow("wf").await.unwrap();
        assert_eq!(all.len(), 2);
        let named_only: Vec<_> = all.iter().filter(|s| s.instance_name.is_some()).collect();
        assert_eq!(named_only.len(), 1);
        assert_eq!(named_only[0].instance_name.as_deref(), Some("prod"));
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

    /// CLOACINA-T-1020: a declared catch-up policy is written on insert and
    /// on re-registration; a re-registration that declares none keeps the
    /// value on the row.
    /// CLOACINA-T-1042: the insert of the upsert cannot double. A second
    /// insert of the same identity writes nothing.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn insert_cron_if_absent_writes_one_row_for_one_identity() {
        let dal = unique_dal().await;
        let row = || {
            let now = UniversalTimestamp::now();
            let new = NewSchedule::cron("wf", "0 * * * *", now);
            NewUnifiedSchedule {
                id: UniversalUuid::new_v4(),
                schedule_type: new.schedule_type,
                workflow_name: new.workflow_name,
                enabled: UniversalBool::from(true),
                cron_expression: new.cron_expression,
                timezone: new.timezone,
                catchup_policy: new.catchup_policy,
                start_date: None,
                end_date: None,
                trigger_name: None,
                poll_interval_ms: None,
                allow_concurrent: None,
                next_run_at: new.next_run_at,
                created_at: now,
                updated_at: now,
                params: None,
                instance_name: None,
                max_active_runs: None,
            }
        };
        assert!(dal.schedule().insert_cron_if_absent(row()).await.unwrap());
        assert!(!dal.schedule().insert_cron_if_absent(row()).await.unwrap());
        // No timezone is UTC: the same identity.
        let mut no_timezone = row();
        no_timezone.timezone = None;
        assert!(
            !dal.schedule()
                .insert_cron_if_absent(no_timezone)
                .await
                .unwrap()
        );
        // Another timezone is another schedule.
        let mut other_timezone = row();
        other_timezone.timezone = Some("Europe/Paris".to_string());
        assert!(
            dal.schedule()
                .insert_cron_if_absent(other_timezone)
                .await
                .unwrap()
        );

        let rows = dal.schedule().find_by_workflow("wf").await.unwrap();
        assert_eq!(rows.len(), 2);
    }

    /// CLOACINA-T-1042: registrations of one schedule at the same moment (two
    /// replicas that load one package) leave one row, and each caller gets
    /// it.
    #[cfg(feature = "sqlite")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_upsert_cron_leaves_one_row() {
        // A database file: connections of a shared in-memory database do
        // not wait for each other's table locks.
        let dir = tempfile::tempdir().expect("temp dir");
        let url = format!("sqlite://{}", dir.path().join("schedules.db").display());
        let db = Database::new(&url, "", 4);
        db.run_migrations().await.expect("migrations");
        let dal = DAL::new(db);
        let next_run = UniversalTimestamp::now();
        for round in 0..25 {
            let workflow = format!("wf_{round}");
            let mut tasks = Vec::new();
            for _ in 0..4 {
                let dal = dal.clone();
                let workflow = workflow.clone();
                tasks.push(tokio::spawn(async move {
                    dal.schedule()
                        .upsert_cron(NewSchedule::cron(&workflow, "0 * * * *", next_run))
                        .await
                        .expect("upsert")
                        .id
                }));
            }
            let mut ids = Vec::new();
            for task in tasks {
                ids.push(task.await.expect("task"));
            }
            let rows = dal.schedule().find_by_workflow(&workflow).await.unwrap();
            assert_eq!(rows.len(), 1, "one row for {workflow}");
            assert!(
                ids.iter().all(|id| *id == rows[0].id),
                "each caller gets the one row"
            );
        }
    }

    /// CLOACINA-T-1042: the unique index is for anonymous cron schedules.
    /// Named instances of the same workflow and expression are free, and a
    /// second plain `create` of the same identity is refused.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn cron_identity_index_leaves_instances_free() {
        let dal = unique_dal().await;
        let now = UniversalTimestamp::now();
        let anonymous = dal
            .schedule()
            .upsert_cron(NewSchedule::cron("wf", "0 * * * *", now))
            .await
            .unwrap();
        for name in ["prod", "staging"] {
            let mut instance = NewSchedule::cron("wf", "0 * * * *", now);
            instance.instance_name = Some(name.to_string());
            dal.schedule().create(instance).await.unwrap();
        }
        // The upsert does not take an instance for the anonymous schedule.
        let again = dal
            .schedule()
            .upsert_cron(NewSchedule::cron("wf", "0 * * * *", now))
            .await
            .unwrap();
        assert_eq!(again.id, anonymous.id);
        assert_eq!(
            dal.schedule().find_by_workflow("wf").await.unwrap().len(),
            3
        );

        assert!(
            dal.schedule()
                .create(NewSchedule::cron("wf", "0 * * * *", now))
                .await
                .is_err(),
            "a second anonymous schedule of one identity is a unique violation"
        );
    }

    /// CLOACINA-T-1042: migration 060 on a database that has doubles. The
    /// row with the latest `last_run_at` stays, the schedule execution rows
    /// of the others move to it (one row per fire time), and other schedules
    /// are not touched. Run on SQLite; the PostgreSQL file has the same
    /// statements.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn migration_060_merges_double_cron_schedules() {
        use crate::models::schedule::NewScheduleExecution;
        use diesel::connection::SimpleConnection;

        const UP: &str = include_str!(
            "../../../database/migrations/sqlite/060_unique_cron_schedule_identity/up.sql"
        );
        assert_eq!(
            UP,
            include_str!(
                "../../../database/migrations/postgres/060_unique_cron_schedule_identity/up.sql"
            ),
            "both backends run the same statements"
        );

        let dal = unique_dal().await;
        let run_sql = |sql: &'static str| {
            let dal = dal.clone();
            async move {
                let conn = dal.database.get_sqlite_connection().await.expect("conn");
                conn.interact(move |conn| conn.batch_execute(sql))
                    .await
                    .expect("interact")
                    .expect("sql");
            }
        };
        // The state before the migration: no index, so doubles can exist.
        run_sql("DROP INDEX idx_schedules_cron_identity;").await;

        let hour =
            |n: i64| chrono::DateTime::<Utc>::from_timestamp(1_800_000_000 + n * 3600, 0).unwrap();
        let new =
            |workflow: &str| NewSchedule::cron(workflow, "0 * * * *", UniversalTimestamp(hour(9)));
        let a = dal.schedule().create(new("wf")).await.unwrap();
        let b = dal.schedule().create(new("wf")).await.unwrap();
        let c = dal.schedule().create(new("wf")).await.unwrap();
        let other = dal.schedule().create(new("other")).await.unwrap();
        let mut instance = new("wf");
        instance.instance_name = Some("prod".to_string());
        let instance = dal.schedule().create(instance).await.unwrap();
        // `b` ran last. `c` never ran.
        dal.schedule()
            .update_schedule_times(a.id, hour(1), hour(2))
            .await
            .unwrap();
        dal.schedule()
            .update_schedule_times(b.id, hour(3), hour(4))
            .await
            .unwrap();

        // Fire times 1 and 2 on `a`, 2 and 3 on `b` (2 is the double fire),
        // 4 on `c`, 2 on the instance and on the other schedule.
        for (schedule, fire) in [
            (a.id, 1),
            (a.id, 2),
            (b.id, 2),
            (b.id, 3),
            (c.id, 4),
            (instance.id, 2),
            (other.id, 2),
        ] {
            dal.schedule_execution()
                .create(NewScheduleExecution {
                    schedule_id: schedule,
                    workflow_execution_id: None,
                    scheduled_time: Some(UniversalTimestamp(hour(fire))),
                    claimed_at: None,
                    context_hash: None,
                })
                .await
                .unwrap();
        }
        let b_row_for_fire_2 = dal
            .schedule_execution()
            .list_by_schedule(b.id, 100, 0)
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.scheduled_time.map(|t| t.0) == Some(hour(2)))
            .unwrap()
            .id;

        run_sql(UP).await;

        let mut left: Vec<_> = dal
            .schedule()
            .find_by_workflow("wf")
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        left.sort_by_key(|id| id.0);
        let mut expected = vec![b.id, instance.id];
        expected.sort_by_key(|id| id.0);
        assert_eq!(
            left, expected,
            "the row that ran last stays; the instance stays"
        );
        let kept = dal.schedule().get_by_id(b.id).await.unwrap();
        assert_eq!(kept.last_run_at.map(|t| t.0), Some(hour(3)));
        assert_eq!(kept.next_run_at.map(|t| t.0), Some(hour(4)));

        let rows = dal
            .schedule_execution()
            .list_by_schedule(b.id, 100, 0)
            .await
            .unwrap();
        let mut fires: Vec<_> = rows
            .iter()
            .filter_map(|row| row.scheduled_time.map(|t| t.0))
            .collect();
        fires.sort();
        assert_eq!(fires, vec![hour(1), hour(2), hour(3), hour(4)]);
        assert!(
            rows.iter().any(|row| row.id == b_row_for_fire_2),
            "for the double fire, the row of the kept schedule stays"
        );
        for untouched in [instance.id, other.id] {
            let rows = dal
                .schedule_execution()
                .list_by_schedule(untouched, 100, 0)
                .await
                .unwrap();
            assert_eq!(rows.len(), 1);
        }

        // The index is there again.
        assert!(dal.schedule().create(new("wf")).await.is_err());
    }

    /// CLOACINA-T-1041: a registration of a schedule that exists (a package
    /// load after a restart) keeps `next_run_at` and `last_run_at`, so the
    /// fires that were missed are still due.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn upsert_cron_keeps_the_times_of_a_row_that_exists() {
        let dal = unique_dal().await;
        let hour =
            |n: i64| chrono::DateTime::<Utc>::from_timestamp(1_800_000_000 + n * 3600, 0).unwrap();
        let first = dal
            .schedule()
            .upsert_cron(NewSchedule::cron(
                "wf",
                "0 * * * *",
                UniversalTimestamp(hour(1)),
            ))
            .await
            .unwrap();
        dal.schedule()
            .update_schedule_times(first.id, hour(1), hour(2))
            .await
            .unwrap();

        let mut again = NewSchedule::cron("wf", "0 * * * *", UniversalTimestamp(hour(9)));
        again.enabled = Some(UniversalBool::from(true));
        let second = dal.schedule().upsert_cron(again).await.unwrap();

        assert_eq!(second.id, first.id);
        assert_eq!(second.last_run_at.map(|t| t.0), Some(hour(1)));
        assert_eq!(second.next_run_at.map(|t| t.0), Some(hour(2)));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn upsert_cron_declared_sets_and_keeps_the_catchup_policy() {
        use crate::models::schedule::{CatchupPolicy, CronScheduleOptions};

        let dal = unique_dal().await;
        let next_run = UniversalTimestamp::now();
        let new = || NewSchedule::cron("wf", "0 * * * *", next_run);
        let run_all = CronScheduleOptions {
            catchup_policy: Some(CatchupPolicy::RunAll),
            max_active_runs: None,
        };
        let skip = CronScheduleOptions {
            catchup_policy: Some(CatchupPolicy::Skip),
            max_active_runs: Some(4),
        };

        // Not declared: a new schedule gets the default.
        let plain = dal.schedule().upsert_cron(new()).await.unwrap();
        assert_eq!(plain.catchup_policy.as_deref(), Some("skip"));

        // Declared on a re-registration: the row changes.
        let declared = dal
            .schedule()
            .upsert_cron_declared(new(), &run_all)
            .await
            .unwrap();
        assert_eq!(declared.id, plain.id);
        assert_eq!(declared.catchup_policy.as_deref(), Some("run_all"));

        assert_eq!(declared.max_active_runs, None);

        // Not declared on a re-registration: the row keeps its value.
        let kept = dal.schedule().upsert_cron(new()).await.unwrap();
        assert_eq!(kept.catchup_policy.as_deref(), Some("run_all"));

        // Declared again: back to skip, now with a limit on active runs.
        let back = dal
            .schedule()
            .upsert_cron_declared(new(), &skip)
            .await
            .unwrap();
        assert_eq!(back.catchup_policy.as_deref(), Some("skip"));
        assert_eq!(back.max_active_runs, Some(4));

        // A re-registration that declares no limit keeps the limit.
        let kept = dal
            .schedule()
            .upsert_cron_declared(new(), &run_all)
            .await
            .unwrap();
        assert_eq!(kept.catchup_policy.as_deref(), Some("run_all"));
        assert_eq!(kept.max_active_runs, Some(4));

        // Declared on insert.
        let mut other = NewSchedule::cron("wf", "0 0 * * *", next_run);
        other.timezone = Some("America/New_York".to_string());
        let inserted = dal
            .schedule()
            .upsert_cron_declared(other, &run_all)
            .await
            .unwrap();
        assert_ne!(inserted.id, plain.id);
        assert_eq!(inserted.catchup_policy.as_deref(), Some("run_all"));
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

    // ── claim_cron_fires (CLOACINA-T-1020) ──────────────────────────

    /// Hourly fire times `first, first + 1h, ...`, each one moving the row to
    /// itself and to the fire after it, as the scheduler builds them for a
    /// `run_all` schedule.
    #[cfg(feature = "sqlite")]
    fn hourly_candidates(first: DateTime<Utc>, count: i64) -> Vec<CronFireCandidate> {
        (0..count)
            .map(|i| {
                let fire = first + chrono::Duration::hours(i);
                CronFireCandidate {
                    scheduled_time: fire,
                    last_run_at: fire,
                    next_run_at: fire + chrono::Duration::hours(1),
                }
            })
            .collect()
    }

    /// A cron schedule whose row says: the last fire taken was `last_run`,
    /// the next one is an hour later. Each call makes a schedule of its own
    /// workflow: one workflow has one anonymous schedule per expression.
    #[cfg(feature = "sqlite")]
    async fn schedule_with_backlog(dal: &DAL, last_run: DateTime<Utc>) -> Schedule {
        let created = dal
            .schedule()
            .create(NewSchedule::cron(
                &format!("wf_{}", uuid::Uuid::new_v4()),
                "0 * * * *",
                UniversalTimestamp::from(last_run + chrono::Duration::hours(1)),
            ))
            .await
            .unwrap();
        dal.schedule()
            .update_schedule_times(created.id, last_run, last_run + chrono::Duration::hours(1))
            .await
            .unwrap();
        dal.schedule().get_by_id(created.id).await.unwrap()
    }

    /// With no limit, a claim takes each candidate, writes a row for each and
    /// moves the schedule to the last one. The schedule stays due when the
    /// fire after the last one is in the past, and a second claim made from
    /// the row as it was before the first one matches nothing.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn claim_cron_fires_moves_the_schedule_to_the_last_fire_taken() {
        let dal = unique_dal().await;
        let now = Utc::now();
        let t0 = now - chrono::Duration::hours(10);
        let schedule = schedule_with_backlog(&dal, t0).await;
        let read_next_run = schedule.next_run_at.unwrap().0;
        let first = t0 + chrono::Duration::hours(1);

        let outcome = dal
            .schedule()
            .claim_cron_fires(
                schedule.id,
                now,
                read_next_run,
                hourly_candidates(first, 3),
                None,
            )
            .await
            .unwrap();
        let CronClaimOutcome::Claimed(claimed) = outcome else {
            panic!("expected a claim, got {outcome:?}");
        };
        assert_eq!(
            claimed.iter().map(|c| c.scheduled_time).collect::<Vec<_>>(),
            vec![
                first,
                first + chrono::Duration::hours(1),
                first + chrono::Duration::hours(2)
            ]
        );

        let after = dal.schedule().get_by_id(schedule.id).await.unwrap();
        assert_eq!(
            after.last_run_at.unwrap().0,
            first + chrono::Duration::hours(2),
            "last_run_at is the last fire taken, not the time of the claim"
        );
        assert_eq!(
            after.next_run_at.unwrap().0,
            first + chrono::Duration::hours(3)
        );
        assert!(after.next_run_at.unwrap().0 < now, "the schedule stays due");

        let rows = dal
            .schedule_execution()
            .list_by_schedule(schedule.id, 10, 0)
            .await
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| row.claimed_at.is_some()
            && row.workflow_execution_id.is_none()
            && row.completed_at.is_none()));

        // A second scheduler that read the row before the claim: the schedule
        // is still due, but its `next_run_at` is no longer the one it read.
        let stale = dal
            .schedule()
            .claim_cron_fires(
                schedule.id,
                now,
                read_next_run,
                hourly_candidates(first, 3),
                None,
            )
            .await
            .unwrap();
        assert_eq!(stale, CronClaimOutcome::NotClaimed);
        assert_eq!(
            dal.schedule_execution()
                .list_by_schedule(schedule.id, 10, 0)
                .await
                .unwrap()
                .len(),
            3,
            "a claim that matches nothing writes no row"
        );

        // The next pass reads the row again and goes on from the fourth fire.
        let next = dal
            .schedule()
            .claim_cron_fires(
                schedule.id,
                now,
                after.next_run_at.unwrap().0,
                hourly_candidates(first + chrono::Duration::hours(3), 2),
                None,
            )
            .await
            .unwrap();
        assert!(matches!(next, CronClaimOutcome::Claimed(ref c) if c.len() == 2));
    }

    /// With a limit, a claim takes only as many fires as there are free
    /// slots, and at the limit it changes nothing: the schedule stays due
    /// with the same times. A row that is marked complete frees its slot.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn claim_cron_fires_takes_what_the_limit_on_active_runs_allows() {
        let dal = unique_dal().await;
        let now = Utc::now();
        let t0 = now - chrono::Duration::hours(10);
        let schedule = schedule_with_backlog(&dal, t0).await;
        let first = t0 + chrono::Duration::hours(1);

        // Limit 2, three candidates: two are taken.
        let outcome = dal
            .schedule()
            .claim_cron_fires(
                schedule.id,
                now,
                schedule.next_run_at.unwrap().0,
                hourly_candidates(first, 3),
                Some(2),
            )
            .await
            .unwrap();
        let CronClaimOutcome::Claimed(claimed) = outcome else {
            panic!("expected a claim, got {outcome:?}");
        };
        assert_eq!(claimed.len(), 2);
        let after = dal.schedule().get_by_id(schedule.id).await.unwrap();
        assert_eq!(
            after.last_run_at.unwrap().0,
            first + chrono::Duration::hours(1)
        );
        assert_eq!(
            after.next_run_at.unwrap().0,
            first + chrono::Duration::hours(2)
        );

        // At the limit: nothing is taken and the row does not change.
        let third = first + chrono::Duration::hours(2);
        let held = dal
            .schedule()
            .claim_cron_fires(
                schedule.id,
                now,
                after.next_run_at.unwrap().0,
                hourly_candidates(third, 3),
                Some(2),
            )
            .await
            .unwrap();
        assert_eq!(held, CronClaimOutcome::AtLimit { active: 2 });
        let unchanged = dal.schedule().get_by_id(schedule.id).await.unwrap();
        assert_eq!(unchanged.last_run_at, after.last_run_at);
        assert_eq!(unchanged.next_run_at, after.next_run_at);
        assert_eq!(unchanged.updated_at, after.updated_at);

        // One run ends: one slot, one more fire.
        dal.schedule_execution()
            .complete(claimed[0].schedule_execution_id, Utc::now())
            .await
            .unwrap();
        let outcome = dal
            .schedule()
            .claim_cron_fires(
                schedule.id,
                now,
                after.next_run_at.unwrap().0,
                hourly_candidates(third, 3),
                Some(2),
            )
            .await
            .unwrap();
        let CronClaimOutcome::Claimed(claimed) = outcome else {
            panic!("expected a claim, got {outcome:?}");
        };
        assert_eq!(
            claimed.iter().map(|c| c.scheduled_time).collect::<Vec<_>>(),
            vec![third]
        );
    }

    /// A schedule that is not due, is disabled or is paused is not claimed.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn claim_cron_fires_needs_a_due_active_schedule() {
        let dal = unique_dal().await;
        let now = Utc::now();
        let t0 = now - chrono::Duration::hours(10);
        let first = t0 + chrono::Duration::hours(1);

        let disabled = schedule_with_backlog(&dal, t0).await;
        dal.schedule().disable(disabled.id).await.unwrap();
        let paused = schedule_with_backlog(&dal, t0).await;
        dal.schedule().pause(paused.id).await.unwrap();
        // Due in the future: its fire time has not come.
        let future = schedule_with_backlog(&dal, now).await;

        for schedule in [disabled, paused, future] {
            let outcome = dal
                .schedule()
                .claim_cron_fires(
                    schedule.id,
                    now,
                    schedule.next_run_at.unwrap().0,
                    hourly_candidates(first, 1),
                    Some(1),
                )
                .await
                .unwrap();
            assert_eq!(outcome, CronClaimOutcome::NotClaimed);
        }
    }

    /// The due time of a held schedule does not hide the due time of the
    /// other schedules.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn next_cron_due_time_excluding_leaves_out_the_held_schedules() {
        let dal = unique_dal().await;
        let now = Utc::now();
        let held = schedule_with_backlog(&dal, now - chrono::Duration::hours(10)).await;
        let other = schedule_with_backlog(&dal, now).await;

        assert_eq!(
            dal.schedule().next_cron_due_time().await.unwrap(),
            held.next_run_at.map(|t| t.0)
        );
        assert_eq!(
            dal.schedule()
                .next_cron_due_time_excluding(vec![held.id])
                .await
                .unwrap(),
            other.next_run_at.map(|t| t.0)
        );
        assert_eq!(
            dal.schedule()
                .next_cron_due_time_excluding(vec![held.id, other.id])
                .await
                .unwrap(),
            None
        );
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
