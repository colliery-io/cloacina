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

//! Schedule rows of type `backfill` (CLOACINA-T-1020).
//!
//! See [`crate::models::backfill`] for what a backfill is and how its row
//! is used. The scheduler takes its fires with
//! [`ScheduleDAL::claim_cron_fires`], which takes both types.

use diesel::prelude::*;

use super::ScheduleDAL;
use crate::dal::unified::models::UnifiedSchedule;
use crate::database::schema::unified::{schedule_executions, schedules, workflow_executions};
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::backfill::{
    BACKFILL_SCHEDULE_TYPE, BackfillCounts, BackfillFailedInterval, NewBackfill,
};
use crate::models::schedule::Schedule;
use chrono::{DateTime, Utc};

/// The insert of a backfill row. `NewUnifiedSchedule` has no `last_run_at`
/// and no `backfill`: no other row is created with them.
#[derive(Debug, Insertable)]
#[diesel(table_name = schedules)]
struct NewBackfillRow {
    id: UniversalUuid,
    schedule_type: String,
    workflow_name: String,
    enabled: UniversalBool,
    cron_expression: Option<String>,
    timezone: Option<String>,
    catchup_policy: Option<String>,
    next_run_at: Option<UniversalTimestamp>,
    last_run_at: Option<UniversalTimestamp>,
    created_at: UniversalTimestamp,
    updated_at: UniversalTimestamp,
    params: Option<String>,
    max_active_runs: Option<i32>,
    backfill: Option<String>,
}

/// The workflow execution states that end a run.
const TERMINAL_FAILED: [&str; 2] = ["Failed", "Cancelled"];

impl<'a> ScheduleDAL<'a> {
    /// Creates a backfill row. It is due at once: `next_run_at` is the first
    /// fire time of its range.
    pub async fn create_backfill(&self, new: NewBackfill) -> Result<Schedule, ValidationError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let spec =
            serde_json::to_string(&new.spec).map_err(|e| ValidationError::ExecutionFailed {
                message: format!("backfill range is not serializable: {e}"),
            })?;

        let row = NewBackfillRow {
            id,
            schedule_type: BACKFILL_SCHEDULE_TYPE.to_string(),
            workflow_name: new.workflow_name,
            enabled: UniversalBool::from(true),
            cron_expression: Some(new.cron_expression),
            timezone: Some(new.timezone),
            catchup_policy: Some("run_all".to_string()),
            next_run_at: Some(UniversalTimestamp::from(new.spec.first_fire)),
            last_run_at: Some(UniversalTimestamp::from(new.seed_last_run_at)),
            created_at: now,
            updated_at: now,
            params: new.params,
            max_active_runs: Some(new.max_active_runs),
            backfill: Some(spec),
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(schedules::table)
                .values(&row)
                .execute(conn)
        })?;

        let result: UnifiedSchedule =
            crate::interact_on_backend!(self.dal, |conn| schedules::table.find(id).first(conn))?;
        Ok(result.into())
    }

    /// One backfill row by id. `None` when no row has the id, or the row is
    /// not a backfill.
    pub async fn get_backfill(
        &self,
        id: UniversalUuid,
    ) -> Result<Option<Schedule>, ValidationError> {
        let result: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .find(id)
                .filter(schedules::schedule_type.eq(BACKFILL_SCHEDULE_TYPE))
                .first(conn)
                .optional()
        })?;
        Ok(result.map(Into::into))
    }

    /// The backfills of one workflow, newest first.
    pub async fn list_backfills(
        &self,
        workflow_name: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let name = workflow_name.to_string();
        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::schedule_type.eq(BACKFILL_SCHEDULE_TYPE))
                .filter(schedules::workflow_name.eq(name))
                .order((schedules::created_at.desc(), schedules::id.desc()))
                .limit(limit)
                .offset(offset)
                .load(conn)
        })?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    /// The enabled, not paused backfills that are due, oldest due time
    /// first.
    pub async fn get_due_backfills(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let now_ts = UniversalTimestamp::from(now);
        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::schedule_type.eq(BACKFILL_SCHEDULE_TYPE))
                .filter(schedules::enabled.eq(UniversalBool::from(true)))
                .filter(schedules::paused.eq(UniversalBool::from(false)))
                .filter(schedules::next_run_at.le(Some(now_ts)))
                .order(schedules::next_run_at.asc())
                .load(conn)
        })?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    /// The earliest `next_run_at` of the enabled, not paused backfills,
    /// leaving out those in `excluded`: the ones the scheduler holds at
    /// their limit on active runs.
    pub async fn next_backfill_due_time_excluding(
        &self,
        excluded: Vec<UniversalUuid>,
    ) -> Result<Option<DateTime<Utc>>, ValidationError> {
        let earliest: Option<UniversalTimestamp> = crate::interact_on_backend!(self.dal, |conn| {
            let mut query = schedules::table
                .filter(schedules::schedule_type.eq(BACKFILL_SCHEDULE_TYPE))
                .filter(schedules::enabled.eq(UniversalBool::from(true)))
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

    /// Counts the runs of a backfill by how they stand.
    ///
    /// * taken: each schedule execution row of the backfill.
    /// * completed: the row is linked to a workflow execution that ended
    ///   `Completed`.
    /// * failed: the workflow execution ended `Failed` or `Cancelled`, or
    ///   the row is closed with no workflow execution (the run could not be
    ///   started).
    /// * running: the rest.
    pub async fn backfill_counts(
        &self,
        id: UniversalUuid,
    ) -> Result<BackfillCounts, ValidationError> {
        let (taken, completed, failed): (i64, i64, i64) =
            crate::interact_on_backend!(self.dal, |conn| {
                let taken: i64 = schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(id))
                    .count()
                    .get_result(conn)?;
                let completed: i64 = schedule_executions::table
                    .inner_join(
                        workflow_executions::table.on(schedule_executions::workflow_execution_id
                            .eq(workflow_executions::id.nullable())),
                    )
                    .filter(schedule_executions::schedule_id.eq(id))
                    .filter(workflow_executions::status.eq("Completed"))
                    .count()
                    .get_result(conn)?;
                let failed_runs: i64 = schedule_executions::table
                    .inner_join(
                        workflow_executions::table.on(schedule_executions::workflow_execution_id
                            .eq(workflow_executions::id.nullable())),
                    )
                    .filter(schedule_executions::schedule_id.eq(id))
                    .filter(workflow_executions::status.eq_any(TERMINAL_FAILED))
                    .count()
                    .get_result(conn)?;
                let not_started: i64 = schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(id))
                    .filter(schedule_executions::workflow_execution_id.is_null())
                    .filter(schedule_executions::completed_at.is_not_null())
                    .count()
                    .get_result(conn)?;
                Ok::<_, diesel::result::Error>((taken, completed, failed_runs + not_started))
            })?;
        let (taken, completed, failed) = (taken as u64, completed as u64, failed as u64);
        Ok(BackfillCounts {
            taken,
            completed,
            failed,
            running: taken.saturating_sub(completed + failed),
        })
    }

    /// The failed intervals of a backfill, oldest first, at most `limit`.
    /// See [`backfill_counts`](Self::backfill_counts) for what failed means.
    pub async fn backfill_failed_intervals(
        &self,
        id: UniversalUuid,
        limit: i64,
    ) -> Result<Vec<BackfillFailedInterval>, ValidationError> {
        type Row = (
            Option<UniversalTimestamp>,
            Option<UniversalUuid>,
            Option<String>,
        );
        let rows: Vec<Row> = crate::interact_on_backend!(self.dal, |conn| {
            schedule_executions::table
                .left_join(
                    workflow_executions::table.on(schedule_executions::workflow_execution_id
                        .eq(workflow_executions::id.nullable())),
                )
                .filter(schedule_executions::schedule_id.eq(id))
                .filter(schedule_executions::scheduled_time.is_not_null())
                .filter(
                    workflow_executions::status
                        .nullable()
                        .eq_any(TERMINAL_FAILED)
                        .or(schedule_executions::workflow_execution_id
                            .is_null()
                            .and(schedule_executions::completed_at.is_not_null())),
                )
                .order(schedule_executions::scheduled_time.asc())
                .limit(limit)
                .select((
                    schedule_executions::scheduled_time,
                    schedule_executions::workflow_execution_id,
                    workflow_executions::status.nullable(),
                ))
                .load(conn)
        })?;
        Ok(rows
            .into_iter()
            .filter_map(|(scheduled_time, workflow_execution_id, status)| {
                Some(BackfillFailedInterval {
                    scheduled_time: scheduled_time?.0,
                    workflow_execution_id,
                    status: status.unwrap_or_else(|| "NotStarted".to_string()),
                })
            })
            .collect())
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::dal::DAL;
    use crate::dal::unified::schedule::{CronClaimOutcome, CronFireCandidate};
    use crate::database::Database;
    use crate::models::backfill::BackfillSpec;
    use crate::models::schedule::NewSchedule;
    use chrono::TimeZone;

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:backfill_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, day, hour, 0, 0).unwrap()
    }

    fn hourly(workflow: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> NewBackfill {
        NewBackfill::for_range(workflow, "0 * * * *", "UTC", start, end, Some(2), None).unwrap()
    }

    /// Migration 059: the type check takes `backfill`, and the row holds the
    /// range, the seeded position and its own limit.
    #[tokio::test]
    async fn create_backfill_writes_a_due_row_of_its_own_type() {
        let dal = unique_dal().await;
        let created = dal
            .schedule()
            .create_backfill(hourly("wf", at(1, 0), at(1, 5)))
            .await
            .unwrap();

        assert_eq!(created.schedule_type, "backfill");
        assert!(created.is_backfill());
        assert!(!created.is_cron());
        assert!(created.is_active());
        assert_eq!(created.catchup_policy.as_deref(), Some("run_all"));
        assert_eq!(created.max_active_runs, Some(2));
        assert_eq!(created.cron_expression.as_deref(), Some("0 * * * *"));
        assert_eq!(created.next_run_at.map(|t| t.0), Some(at(1, 0)));
        assert_eq!(
            created.last_run_at.map(|t| t.0),
            Some(at(1, 0) - chrono::Duration::hours(1))
        );
        assert!(created.start_date.is_none());
        assert!(created.end_date.is_none());
        let spec = BackfillSpec::parse(created.backfill.as_deref().unwrap()).unwrap();
        assert_eq!(spec.total, 6);
        assert_eq!(spec.last_fire, at(1, 5));

        let got = dal.schedule().get_backfill(created.id).await.unwrap();
        assert_eq!(got.map(|s| s.id), Some(created.id));
    }

    /// A backfill is not a live schedule: the schedule list, the due cron
    /// query and the cron due time leave it out, and the upsert of a packaged
    /// cron with the same workflow, expression and timezone makes its own
    /// row.
    #[tokio::test]
    async fn backfill_rows_stay_out_of_the_live_schedule_queries() {
        let dal = unique_dal().await;
        let backfill = dal
            .schedule()
            .create_backfill(hourly("wf", at(1, 0), at(1, 5)))
            .await
            .unwrap();
        let now = Utc::now();

        assert!(
            dal.schedule()
                .list(None, false, 100, 0)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            dal.schedule()
                .list(Some("cron"), false, 100, 0)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            dal.schedule()
                .list(Some("backfill"), false, 100, 0)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            dal.schedule()
                .get_due_cron_schedules(now)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(dal.schedule().next_cron_due_time().await.unwrap(), None);

        let next_run = UniversalTimestamp::from(now + chrono::Duration::hours(1));
        let cron = dal
            .schedule()
            .upsert_cron(NewSchedule::cron("wf", "0 * * * *", next_run))
            .await
            .unwrap();
        assert_ne!(cron.id, backfill.id);
        assert_eq!(cron.schedule_type, "cron");
        let again = dal
            .schedule()
            .upsert_cron(NewSchedule::cron("wf", "0 * * * *", next_run))
            .await
            .unwrap();
        assert_eq!(again.id, cron.id);
        let kept = dal
            .schedule()
            .get_backfill(backfill.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(kept.next_run_at.map(|t| t.0), Some(at(1, 0)));
        assert_eq!(kept.schedule_type, "backfill");

        // The cron row is not a backfill.
        assert!(
            dal.schedule()
                .get_backfill(cron.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            dal.schedule()
                .list(None, false, 100, 0)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn due_backfills_are_the_enabled_ones_and_the_list_is_by_workflow() {
        let dal = unique_dal().await;
        let a = dal
            .schedule()
            .create_backfill(hourly("wf_a", at(1, 0), at(1, 5)))
            .await
            .unwrap();
        let b = dal
            .schedule()
            .create_backfill(hourly("wf_b", at(2, 0), at(2, 5)))
            .await
            .unwrap();
        // A range in the future is not due.
        let future = Utc::now() + chrono::Duration::days(2);
        let c = dal
            .schedule()
            .create_backfill(hourly("wf_a", future, future + chrono::Duration::hours(3)))
            .await
            .unwrap();
        let now = Utc::now();

        let due: Vec<_> = dal
            .schedule()
            .get_due_backfills(now)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(due, vec![a.id, b.id]);
        assert_eq!(
            dal.schedule()
                .next_backfill_due_time_excluding(vec![])
                .await
                .unwrap(),
            Some(at(1, 0))
        );
        assert_eq!(
            dal.schedule()
                .next_backfill_due_time_excluding(vec![a.id])
                .await
                .unwrap(),
            Some(at(2, 0))
        );

        dal.schedule().disable(a.id).await.unwrap();
        dal.schedule().pause(b.id).await.unwrap();
        assert!(
            dal.schedule()
                .get_due_backfills(now)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            dal.schedule()
                .next_backfill_due_time_excluding(vec![])
                .await
                .unwrap(),
            c.next_run_at.map(|t| t.0)
        );

        let of_a: Vec<_> = dal
            .schedule()
            .list_backfills("wf_a", 100, 0)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(of_a.len(), 2);
        assert!(of_a.contains(&a.id) && of_a.contains(&c.id));
        assert_eq!(
            dal.schedule()
                .list_backfills("wf_a", 1, 0)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            dal.schedule()
                .list_backfills("none", 100, 0)
                .await
                .unwrap()
                .is_empty()
        );
    }

    /// The claim takes the fires of a backfill row, with its limit. The
    /// counts follow the rows: taken, then failed for a row closed with no
    /// workflow execution.
    #[tokio::test]
    async fn claim_takes_backfill_fires_and_the_counts_follow() {
        let dal = unique_dal().await;
        let backfill = dal
            .schedule()
            .create_backfill(hourly("wf", at(1, 0), at(1, 5)))
            .await
            .unwrap();
        assert_eq!(
            dal.schedule().backfill_counts(backfill.id).await.unwrap(),
            BackfillCounts::default()
        );

        let candidates: Vec<CronFireCandidate> = (0..4)
            .map(|h| CronFireCandidate {
                scheduled_time: at(1, h),
                last_run_at: at(1, h),
                next_run_at: at(1, h + 1),
            })
            .collect();
        let outcome = dal
            .schedule()
            .claim_cron_fires(
                backfill.id,
                Utc::now(),
                at(1, 0),
                candidates.clone(),
                Some(2),
            )
            .await
            .unwrap();
        let CronClaimOutcome::Claimed(claimed) = outcome else {
            panic!("the claim must take fires: {outcome:?}");
        };
        assert_eq!(claimed.len(), 2, "the limit of the backfill is 2");

        let row = dal
            .schedule()
            .get_backfill(backfill.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.last_run_at.map(|t| t.0), Some(at(1, 1)));
        assert_eq!(row.next_run_at.map(|t| t.0), Some(at(1, 2)));
        let counts = dal.schedule().backfill_counts(backfill.id).await.unwrap();
        assert_eq!((counts.taken, counts.running, counts.failed), (2, 2, 0));

        // At its limit: nothing is taken.
        let held = dal
            .schedule()
            .claim_cron_fires(
                backfill.id,
                Utc::now(),
                at(1, 2),
                candidates[2..].to_vec(),
                Some(2),
            )
            .await
            .unwrap();
        assert_eq!(held, CronClaimOutcome::AtLimit { active: 2 });

        // The run of the second interval could not be started.
        dal.schedule_execution()
            .complete(claimed[1].schedule_execution_id, Utc::now())
            .await
            .unwrap();
        let counts = dal.schedule().backfill_counts(backfill.id).await.unwrap();
        assert_eq!(
            (
                counts.taken,
                counts.completed,
                counts.failed,
                counts.running
            ),
            (2, 0, 1, 1)
        );
        let failed = dal
            .schedule()
            .backfill_failed_intervals(backfill.id, 100)
            .await
            .unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].scheduled_time, at(1, 1));
        assert_eq!(failed[0].status, "NotStarted");
        assert_eq!(failed[0].workflow_execution_id, None);
    }
}
