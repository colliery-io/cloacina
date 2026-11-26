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

//! The backfill (CLOACINA-T-1020): a bounded second schedule for one
//! workflow.
//!
//! A backfill runs one workflow once for each fire time of a cron expression
//! in a range, oldest first, with its own limit on active runs. It is a row
//! of `schedules` with `schedule_type = 'backfill'` and catch-up policy
//! `run_all`:
//!
//! * `last_run_at` starts at the fire time before the range, so the first
//!   fire the scheduler takes is the first fire of the range. After that it
//!   is the last fire taken: the position of the backfill.
//! * `backfill` holds a [`BackfillSpec`] as JSON: the range, the number of
//!   intervals, and for a retry the list of fire times.
//! * `start_date` and `end_date` stay empty: the scheduler compares them
//!   with the wall clock, and a backfill is bounded by fire time.
//!
//! Each fire is one interval: it ends at the fire time and starts at the
//! fire time before it. The range `start..=end` takes each fire time `F`
//! with `start <= F <= end`. A range of one interval has `start == end ==`
//! its fire time.
//!
//! The row disables itself when every interval is taken and no run of it is
//! active. A failed run does not stop it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cron_evaluator::CronEvaluator;
use crate::database::universal_types::UniversalUuid;

/// The value of `schedules.schedule_type` on a backfill row.
pub const BACKFILL_SCHEDULE_TYPE: &str = "backfill";

/// The most intervals that one backfill may hold. A larger range is an
/// error: split it.
pub const MAX_BACKFILL_INTERVALS: usize = 100_000;

/// The range of a backfill, stored as JSON in `schedules.backfill`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackfillSpec {
    /// The start of the range, as asked for. For a retry: the first fire
    /// time of the list.
    pub start: DateTime<Utc>,
    /// The end of the range, as asked for. For a retry: the last fire time
    /// of the list.
    pub end: DateTime<Utc>,
    /// The first fire time of the range.
    pub first_fire: DateTime<Utc>,
    /// The last fire time of the range: the scheduler takes no fire after
    /// it.
    pub last_fire: DateTime<Utc>,
    /// The number of intervals.
    pub total: u64,
    /// A retry runs these fire times only, oldest first. `None`: every fire
    /// time of the expression from `first_fire` to `last_fire`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_times: Option<Vec<DateTime<Utc>>>,
    /// The id of the backfill that this one retries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_of: Option<String>,
}

impl BackfillSpec {
    /// Reads the spec of a backfill row.
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid backfill range: {}", e))
    }
}

/// A backfill to create: the values of its `schedules` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBackfill {
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    /// The most runs of the backfill that may be active at once.
    pub max_active_runs: i32,
    /// Params for each run, a JSON object as text.
    pub params: Option<String>,
    pub spec: BackfillSpec,
    /// The first value of `last_run_at`: the fire time before the range.
    pub seed_last_run_at: DateTime<Utc>,
}

/// Checks the declared limit on active runs of a backfill. Not declared: 1.
fn backfill_max_active_runs(max_active_runs: Option<u32>) -> Result<i32, String> {
    match max_active_runs {
        None => Ok(1),
        Some(0) => Err("invalid max_active_runs 0: use 1 or more, or leave it out".to_string()),
        Some(limit) => i32::try_from(limit)
            .map_err(|_| format!("invalid max_active_runs {}: it is too large", limit)),
    }
}

impl NewBackfill {
    /// A backfill of each fire time `F` of the expression with
    /// `start <= F <= end`.
    pub fn for_range(
        workflow_name: &str,
        cron_expression: &str,
        timezone: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        max_active_runs: Option<u32>,
        params: Option<String>,
    ) -> Result<Self, String> {
        let max_active_runs = backfill_max_active_runs(max_active_runs)?;
        if end < start {
            return Err(format!(
                "invalid range: end {} is before start {}",
                end.to_rfc3339(),
                start.to_rfc3339()
            ));
        }
        let evaluator = CronEvaluator::new(cron_expression, timezone)
            .map_err(|e| format!("invalid cron expression or timezone: {}", e))?;
        let cron_err = |e| format!("cron evaluation error: {}", e);

        // The fire time before the range. `previous_execution` is strictly
        // before its argument, so a `start` that is a fire time is in the
        // range.
        let seed = evaluator.previous_execution(start).map_err(cron_err)?;

        let mut total: usize = 0;
        let mut first_fire = None;
        let mut last_fire = seed;
        let mut cursor = seed;
        loop {
            let next = evaluator.next_execution(cursor).map_err(cron_err)?;
            if next > end {
                break;
            }
            total += 1;
            if total > MAX_BACKFILL_INTERVALS {
                return Err(format!(
                    "the range holds more than {} intervals: split it",
                    MAX_BACKFILL_INTERVALS
                ));
            }
            first_fire.get_or_insert(next);
            last_fire = next;
            cursor = next;
        }
        let Some(first_fire) = first_fire else {
            return Err(format!(
                "the range holds no interval: '{}' ({}) has no fire time from {} to {}",
                cron_expression,
                timezone,
                start.to_rfc3339(),
                end.to_rfc3339()
            ));
        };

        Ok(Self {
            workflow_name: workflow_name.to_string(),
            cron_expression: cron_expression.to_string(),
            timezone: timezone.to_string(),
            max_active_runs,
            params,
            spec: BackfillSpec {
                start,
                end,
                first_fire,
                last_fire,
                total: total as u64,
                fire_times: None,
                retry_of: None,
            },
            seed_last_run_at: seed,
        })
    }

    /// A backfill of the given fire times only: the retry of the failed
    /// intervals of the backfill `retry_of`. Each time must be a fire time of
    /// the expression.
    pub fn for_fire_times(
        workflow_name: &str,
        cron_expression: &str,
        timezone: &str,
        mut fire_times: Vec<DateTime<Utc>>,
        max_active_runs: Option<u32>,
        params: Option<String>,
        retry_of: Option<UniversalUuid>,
    ) -> Result<Self, String> {
        let max_active_runs = backfill_max_active_runs(max_active_runs)?;
        let evaluator = CronEvaluator::new(cron_expression, timezone)
            .map_err(|e| format!("invalid cron expression or timezone: {}", e))?;
        let cron_err = |e| format!("cron evaluation error: {}", e);

        fire_times.sort();
        fire_times.dedup();
        let (Some(first_fire), Some(last_fire)) =
            (fire_times.first().copied(), fire_times.last().copied())
        else {
            return Err("no interval to run".to_string());
        };
        if fire_times.len() > MAX_BACKFILL_INTERVALS {
            return Err(format!(
                "more than {} intervals: split them",
                MAX_BACKFILL_INTERVALS
            ));
        }
        for fire in &fire_times {
            let before = evaluator.previous_execution(*fire).map_err(cron_err)?;
            if evaluator.next_execution(before).map_err(cron_err)? != *fire {
                return Err(format!(
                    "{} is not a fire time of '{}' ({})",
                    fire.to_rfc3339(),
                    cron_expression,
                    timezone
                ));
            }
        }
        let seed = evaluator.previous_execution(first_fire).map_err(cron_err)?;

        Ok(Self {
            workflow_name: workflow_name.to_string(),
            cron_expression: cron_expression.to_string(),
            timezone: timezone.to_string(),
            max_active_runs,
            params,
            spec: BackfillSpec {
                start: first_fire,
                end: last_fire,
                first_fire,
                last_fire,
                total: fire_times.len() as u64,
                fire_times: Some(fire_times),
                retry_of: retry_of.map(|id| id.to_string()),
            },
            seed_last_run_at: seed,
        })
    }
}

/// How the runs of a backfill stand, counted from its schedule execution
/// rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BackfillCounts {
    /// Intervals that the scheduler took: each has a schedule execution row.
    pub taken: u64,
    /// Runs that ended `Completed`.
    pub completed: u64,
    /// Runs that ended `Failed` or `Cancelled`, and intervals whose run
    /// could not be started.
    pub failed: u64,
    /// Runs that have not ended.
    pub running: u64,
}

/// One failed interval of a backfill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackfillFailedInterval {
    /// The fire time: the end of the interval.
    pub scheduled_time: DateTime<Utc>,
    /// The workflow execution, when a run started.
    pub workflow_execution_id: Option<UniversalUuid>,
    /// `Failed`, `Cancelled`, or `NotStarted` when no run started.
    pub status: String,
}

/// Where a backfill stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackfillStatus {
    /// It has intervals to take, or runs that have not ended.
    Running,
    /// Paused by an operator: it takes no interval until it is resumed.
    Paused,
    /// Every interval was taken and every run ended.
    Completed,
    /// Cancelled before every interval was taken, and no run is active.
    Cancelled,
}

impl BackfillStatus {
    /// The name on the API.
    pub fn as_str(&self) -> &'static str {
        match self {
            BackfillStatus::Running => "running",
            BackfillStatus::Paused => "paused",
            BackfillStatus::Completed => "completed",
            BackfillStatus::Cancelled => "cancelled",
        }
    }

    /// Reads the status from the row and the counts. Nothing else is stored:
    /// a row that is disabled before every interval is taken was cancelled.
    pub fn derive(enabled: bool, paused: bool, total: u64, counts: &BackfillCounts) -> Self {
        let all_taken = counts.taken >= total;
        if counts.running > 0 {
            return BackfillStatus::Running;
        }
        if all_taken {
            return BackfillStatus::Completed;
        }
        if !enabled {
            return BackfillStatus::Cancelled;
        }
        if paused {
            return BackfillStatus::Paused;
        }
        BackfillStatus::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    #[test]
    fn backfill_range_takes_fire_times_from_start_to_end_inclusive() {
        let b = NewBackfill::for_range(
            "wf",
            "0 * * * *",
            "UTC",
            at(2026, 1, 1, 0),
            at(2026, 1, 1, 5),
            None,
            None,
        )
        .unwrap();
        assert_eq!(b.spec.total, 6);
        assert_eq!(b.spec.first_fire, at(2026, 1, 1, 0));
        assert_eq!(b.spec.last_fire, at(2026, 1, 1, 5));
        assert_eq!(b.seed_last_run_at, at(2025, 12, 31, 23));
        assert_eq!(b.max_active_runs, 1);
        assert_eq!(b.spec.fire_times, None);
    }

    #[test]
    fn backfill_range_of_one_interval_has_start_equal_to_end() {
        let fire = at(2026, 3, 4, 7);
        let b =
            NewBackfill::for_range("wf", "0 * * * *", "UTC", fire, fire, Some(4), None).unwrap();
        assert_eq!(b.spec.total, 1);
        assert_eq!(b.spec.first_fire, fire);
        assert_eq!(b.spec.last_fire, fire);
        assert_eq!(b.max_active_runs, 4);
    }

    #[test]
    fn backfill_range_between_fire_times_starts_at_the_next_fire() {
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 30, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 1, 1, 3, 30, 0).unwrap();
        let b = NewBackfill::for_range("wf", "0 * * * *", "UTC", start, end, None, None).unwrap();
        assert_eq!(b.spec.total, 3);
        assert_eq!(b.spec.first_fire, at(2026, 1, 1, 1));
        assert_eq!(b.spec.last_fire, at(2026, 1, 1, 3));
        assert_eq!(b.seed_last_run_at, at(2026, 1, 1, 0));
    }

    #[test]
    fn backfill_range_counts_the_motivating_history_load() {
        // About 5,500 hourly intervals: 230 days.
        let start = at(2026, 1, 1, 0);
        let end = start + chrono::Duration::hours(5_519);
        let b = NewBackfill::for_range("wf", "0 * * * *", "UTC", start, end, None, None).unwrap();
        assert_eq!(b.spec.total, 5_520);
    }

    #[test]
    fn backfill_range_errors_are_clear() {
        let err = |r: Result<NewBackfill, String>| r.unwrap_err();
        let e = err(NewBackfill::for_range(
            "wf",
            "0 * * * *",
            "UTC",
            at(2026, 1, 2, 0),
            at(2026, 1, 1, 0),
            None,
            None,
        ));
        assert!(e.contains("is before start"), "{e}");

        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 10, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 1, 1, 0, 50, 0).unwrap();
        let e = err(NewBackfill::for_range(
            "wf",
            "0 * * * *",
            "UTC",
            start,
            end,
            None,
            None,
        ));
        assert!(e.contains("holds no interval"), "{e}");

        let e = err(NewBackfill::for_range(
            "wf",
            "* * * * *",
            "UTC",
            at(2020, 1, 1, 0),
            at(2026, 1, 1, 0),
            None,
            None,
        ));
        assert!(e.contains("more than 100000 intervals"), "{e}");

        let e = err(NewBackfill::for_range(
            "wf",
            "0 * * * *",
            "UTC",
            at(2026, 1, 1, 0),
            at(2026, 1, 1, 1),
            Some(0),
            None,
        ));
        assert!(e.contains("max_active_runs 0"), "{e}");

        let e = err(NewBackfill::for_range(
            "wf",
            "not a cron",
            "UTC",
            at(2026, 1, 1, 0),
            at(2026, 1, 1, 1),
            None,
            None,
        ));
        assert!(e.contains("invalid cron expression"), "{e}");
    }

    #[test]
    fn backfill_of_fire_times_sorts_them_and_checks_each() {
        let b = NewBackfill::for_fire_times(
            "wf",
            "0 * * * *",
            "UTC",
            vec![at(2026, 1, 1, 9), at(2026, 1, 1, 2), at(2026, 1, 1, 9)],
            Some(2),
            None,
            None,
        )
        .unwrap();
        assert_eq!(b.spec.total, 2);
        assert_eq!(
            b.spec.fire_times,
            Some(vec![at(2026, 1, 1, 2), at(2026, 1, 1, 9)])
        );
        assert_eq!(b.seed_last_run_at, at(2026, 1, 1, 1));

        let off = Utc.with_ymd_and_hms(2026, 1, 1, 2, 30, 0).unwrap();
        let e = NewBackfill::for_fire_times("wf", "0 * * * *", "UTC", vec![off], None, None, None)
            .unwrap_err();
        assert!(e.contains("is not a fire time"), "{e}");

        let e = NewBackfill::for_fire_times("wf", "0 * * * *", "UTC", vec![], None, None, None)
            .unwrap_err();
        assert!(e.contains("no interval"), "{e}");
    }

    #[test]
    fn backfill_spec_round_trips_as_json() {
        let b = NewBackfill::for_fire_times(
            "wf",
            "0 * * * *",
            "UTC",
            vec![at(2026, 1, 1, 2)],
            None,
            None,
            Some(UniversalUuid::new_v4()),
        )
        .unwrap();
        let json = serde_json::to_string(&b.spec).unwrap();
        assert_eq!(BackfillSpec::parse(&json).unwrap(), b.spec);
        assert!(BackfillSpec::parse("{").is_err());
    }

    #[test]
    fn backfill_status_is_read_from_the_row_and_the_counts() {
        let counts = |taken, completed, failed, running| BackfillCounts {
            taken,
            completed,
            failed,
            running,
        };
        use BackfillStatus::*;
        assert_eq!(
            BackfillStatus::derive(true, false, 10, &counts(0, 0, 0, 0)),
            Running
        );
        assert_eq!(
            BackfillStatus::derive(true, false, 10, &counts(4, 2, 1, 1)),
            Running
        );
        // All taken, the last runs still active.
        assert_eq!(
            BackfillStatus::derive(false, false, 10, &counts(10, 8, 1, 1)),
            Running
        );
        assert_eq!(
            BackfillStatus::derive(false, false, 10, &counts(10, 9, 1, 0)),
            Completed
        );
        assert_eq!(
            BackfillStatus::derive(true, false, 10, &counts(10, 9, 1, 0)),
            Completed
        );
        // Cancelled with a run still active, then with none.
        assert_eq!(
            BackfillStatus::derive(false, false, 10, &counts(4, 3, 0, 1)),
            Running
        );
        assert_eq!(
            BackfillStatus::derive(false, false, 10, &counts(4, 3, 1, 0)),
            Cancelled
        );
        assert_eq!(
            BackfillStatus::derive(true, true, 10, &counts(4, 4, 0, 0)),
            Paused
        );
    }
}
