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

//! The scheduler pass over backfills (CLOACINA-T-1020).
//!
//! A backfill is a `schedules` row of type `backfill`: a bounded second
//! schedule for one workflow (see [`crate::models::backfill`]). The cron pass
//! calls [`Scheduler::check_and_execute_backfills`] each time it runs, and
//! the cron loop sleeps as long as [`Scheduler::next_cron_wake`] says.
//!
//! What is shared with the cron fire path: the claim
//! (`ScheduleDAL::claim_cron_fires`: the kept backlog, the limit on active
//! runs, safety with two schedulers), the keys of a run
//! (`stamp_cron_fire_keys`) and the wait for the end of a run. What is apart:
//! the candidates stop at the end of the range, and the row disables itself
//! when every interval is taken and no run is active.
//!
//! A backfill stays due while it has intervals to take, so one that cannot
//! take a fire now is put in `backfill_held`. The loop leaves the held ones
//! out when it computes its sleep, looks at them again each
//! `cron_held_poll_interval`, and at once when a run of one ends.

use std::time::Duration;

use chrono::{DateTime, Utc};
use tracing::{debug, error, info, warn};

use super::{Scheduler, compute_cron_sleep_delay};
use crate::cron_evaluator::{CronError, CronEvaluator};
use crate::dal::unified::schedule::{CronClaimOutcome, CronFireCandidate};
use crate::database::universal_types::UniversalUuid;
use crate::executor::WorkflowExecutionError;
use crate::models::backfill::BackfillSpec;
use crate::models::schedule::Schedule;

/// The fire times that a pass over a due backfill wants, oldest first, and
/// whether the range is used up (no fire of it is after `last_run_at`).
///
/// A fire is wanted when it is after `last_run_at`, not after the last fire
/// of the range, and not after `now`: a range that reaches into the future
/// is taken as its fire times pass. At most `batch` fires.
///
/// After fire `F` the row has `last_run_at = F` and `next_run_at` = the next
/// fire of the range. After the last fire of the range, `next_run_at = F`:
/// the row stays due, and the pass that finds the range used up disables it.
pub(super) fn backfill_fire_candidates(
    spec: &BackfillSpec,
    evaluator: &CronEvaluator,
    last_run_at: DateTime<Utc>,
    now: DateTime<Utc>,
    batch: usize,
) -> Result<(Vec<CronFireCandidate>, bool), CronError> {
    let batch = batch.max(1);

    // The fires after `last_run_at`, to the end of the range: one more than
    // the batch, so the last one taken knows the fire after it.
    let mut upcoming: Vec<DateTime<Utc>> = Vec::with_capacity(batch + 1);
    match &spec.fire_times {
        Some(times) => {
            let from = times.partition_point(|t| *t <= last_run_at);
            upcoming.extend(times[from..].iter().take(batch + 1).copied());
        }
        None => {
            let mut cursor = last_run_at;
            while upcoming.len() < batch + 1 {
                let next = evaluator.next_execution(cursor)?;
                if next > spec.last_fire {
                    break;
                }
                upcoming.push(next);
                cursor = next;
            }
        }
    }
    if upcoming.is_empty() {
        return Ok((Vec::new(), true));
    }

    let mut candidates = Vec::with_capacity(batch);
    for (index, fire) in upcoming.iter().enumerate().take(batch) {
        if *fire > now {
            break;
        }
        candidates.push(CronFireCandidate {
            scheduled_time: *fire,
            last_run_at: *fire,
            next_run_at: upcoming.get(index + 1).copied().unwrap_or(*fire),
        });
    }
    Ok((candidates, false))
}

impl Scheduler {
    /// How long the cron loop sleeps before its next check, and the earliest
    /// due time of the cron schedules (for the lag gauge).
    ///
    /// The sleep of the cron schedules (`next_live_cron_wake`), or less when
    /// a backfill is due sooner. The lag gauge reads the cron schedules
    /// only: a backfill is behind by design, by the age of its range.
    pub(super) async fn next_cron_wake(&self) -> (Option<DateTime<Utc>>, Duration) {
        let (next_due, delay) = self.next_live_cron_wake().await;
        (next_due, delay.min(self.backfill_sleep_delay().await))
    }

    /// How long until a backfill needs a pass: until the earliest due time
    /// of those not held, and no longer than `cron_held_poll_interval` while
    /// one is held.
    async fn backfill_sleep_delay(&self) -> Duration {
        let held: Vec<UniversalUuid> = self.backfill_held.lock().iter().copied().collect();
        let any_held = !held.is_empty();
        let delay = match self
            .dal
            .schedule()
            .next_backfill_due_time_excluding(held)
            .await
        {
            Ok(due) => compute_cron_sleep_delay(due, Utc::now(), self.config.cron_poll_interval),
            Err(e) => {
                warn!("Failed to query next backfill due time: {}", e);
                self.config.cron_poll_interval
            }
        };
        if any_held {
            delay.min(self.config.cron_held_poll_interval)
        } else {
            delay
        }
    }

    /// Finds the due backfills and processes each on its own task. Errors
    /// are logged: a backfill must not stop the cron pass.
    pub(super) async fn check_and_execute_backfills(&self, now: DateTime<Utc>) {
        let due = match self.dal.schedule().get_due_backfills(now).await {
            Ok(due) => due,
            Err(e) => {
                error!("Failed to read due backfills: {}", e);
                return;
            }
        };

        // A held backfill that is no longer due (cancelled, paused, deleted,
        // disabled by another scheduler) is no longer held.
        {
            let mut held = self.backfill_held.lock();
            if !held.is_empty() {
                held.retain(|id| due.iter().any(|schedule| schedule.id == *id));
            }
        }

        for schedule in due {
            let this = self.clone();
            tokio::spawn(async move {
                if let Err(e) = this.process_backfill(&schedule, now).await {
                    error!("Failed to process backfill {}: {}", schedule.id, e);
                    // It stays due: look again after the held interval, not
                    // at once.
                    this.backfill_held.lock().insert(schedule.id);
                }
            });
        }
    }

    /// Processes one due backfill: takes its next intervals and hands each
    /// run off. It does not wait for a run to end.
    async fn process_backfill(
        &self,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> Result<(), WorkflowExecutionError> {
        let failed = |message: String| WorkflowExecutionError::ExecutionFailed { message };

        let (Some(expected_next_run), Some(last_run_at)) = (
            schedule.next_run_at.map(|t| t.0),
            schedule.last_run_at.map(|t| t.0),
        ) else {
            return Err(failed("the backfill row has no position".to_string()));
        };
        let spec = BackfillSpec::parse(schedule.backfill.as_deref().unwrap_or_default())
            .map_err(failed)?;
        let cron_expr = schedule.cron_expression.as_deref().unwrap_or_default();
        let tz = schedule.timezone.as_deref().unwrap_or("UTC");
        let (candidates, used_up) = CronEvaluator::new(cron_expr, tz)
            .and_then(|evaluator| {
                backfill_fire_candidates(
                    &spec,
                    &evaluator,
                    last_run_at,
                    now,
                    self.config.max_catchup_executions,
                )
            })
            .map_err(|e| failed(format!("Cron evaluation error: {}", e)))?;

        if used_up {
            return self.finish_backfill_when_idle(schedule).await;
        }
        if candidates.is_empty() {
            // Due, with no fire to take yet. Not expected: look again later.
            self.backfill_held.lock().insert(schedule.id);
            return Ok(());
        }

        let outcome = self
            .dal
            .schedule()
            .claim_cron_fires(
                schedule.id,
                now,
                expected_next_run,
                candidates,
                self.max_active_runs(schedule),
            )
            .await
            .map_err(|e| failed(e.to_string()))?;

        let claimed = match outcome {
            CronClaimOutcome::Claimed(claimed) => claimed,
            CronClaimOutcome::NotClaimed => {
                debug!(
                    "Backfill {} was already taken by another scheduler",
                    schedule.id
                );
                return Ok(());
            }
            CronClaimOutcome::AtLimit { active } => {
                debug!(
                    "Backfill {} has {} active run(s), its limit; it stays due",
                    schedule.id, active
                );
                self.backfill_held.lock().insert(schedule.id);
                return Ok(());
            }
        };
        self.backfill_held.lock().remove(&schedule.id);

        info!(
            "Backfill {} of workflow {}: took {} interval(s), {} to {}",
            schedule.id,
            schedule.workflow_name,
            claimed.len(),
            claimed.first().map(|f| f.scheduled_time).unwrap_or(now),
            claimed.last().map(|f| f.scheduled_time).unwrap_or(now),
        );

        for fire in claimed {
            let schedule_execution_id = fire.schedule_execution_id;
            let scheduled_time = fire.scheduled_time;

            // The same handoff as a cron fire: the keys of the run come from
            // `stamp_cron_fire_keys`, which adds `backfill_id` for this type.
            match self.execute_cron_workflow(schedule, scheduled_time).await {
                Ok(handle) => {
                    let workflow_execution_id = UniversalUuid(handle.execution_id);
                    if let Err(e) = self
                        .dal
                        .schedule_execution()
                        .update_workflow_execution_id(schedule_execution_id, workflow_execution_id)
                        .await
                    {
                        error!(
                            "Failed to link schedule execution {} to workflow execution {} for backfill {}: {}",
                            schedule_execution_id, workflow_execution_id, schedule.id, e
                        );
                    }

                    let this = self.clone();
                    let schedule_id = schedule.id;
                    let workflow_name = schedule.workflow_name.clone();
                    tokio::spawn(async move {
                        this.complete_trigger_execution_when_done(
                            schedule_execution_id,
                            workflow_execution_id,
                            &workflow_name,
                        )
                        .await;
                        this.release_backfill(schedule_id);
                    });
                }
                Err(e) => {
                    // The interval counts as failed: its row is closed with
                    // no workflow execution. The backfill goes on.
                    error!(
                        "Failed to start workflow {} for backfill {} (interval end {}): {}",
                        schedule.workflow_name, schedule.id, scheduled_time, e
                    );
                    if let Err(e) = self
                        .dal
                        .schedule_execution()
                        .complete(schedule_execution_id, Utc::now())
                        .await
                    {
                        warn!(
                            "Failed to close schedule execution {} of backfill {}: {}",
                            schedule_execution_id, schedule.id, e
                        );
                    }
                    self.release_backfill(schedule.id);
                }
            }
        }

        Ok(())
    }

    /// A slot of the backfill is free, or its state changed: it is no longer
    /// held, and the cron loop looks again at once.
    fn release_backfill(&self, schedule_id: UniversalUuid) {
        self.backfill_held.lock().remove(&schedule_id);
        self.cron_change.notify_one();
    }

    /// Every interval of the backfill is taken. With no run active, the row
    /// disables itself. With a run active it stays due and held, so the
    /// recovery sweep still covers its runs; the end of a run brings the
    /// pass back here.
    async fn finish_backfill_when_idle(
        &self,
        schedule: &Schedule,
    ) -> Result<(), WorkflowExecutionError> {
        let failed = |e: crate::error::ValidationError| WorkflowExecutionError::ExecutionFailed {
            message: e.to_string(),
        };
        let counts = self
            .dal
            .schedule()
            .backfill_counts(schedule.id)
            .await
            .map_err(failed)?;
        if counts.running > 0 {
            self.backfill_held.lock().insert(schedule.id);
            return Ok(());
        }
        self.dal
            .schedule()
            .disable(schedule.id)
            .await
            .map_err(failed)?;
        self.backfill_held.lock().remove(&schedule.id);
        info!(
            "Backfill {} of workflow {} is done: {} interval(s), {} completed, {} failed",
            schedule.id, schedule.workflow_name, counts.taken, counts.completed, counts.failed
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::backfill::NewBackfill;
    use chrono::TimeZone;

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, day, hour, 0, 0).unwrap()
    }

    fn hourly() -> CronEvaluator {
        CronEvaluator::new("0 * * * *", "UTC").unwrap()
    }

    fn times(candidates: &[CronFireCandidate]) -> Vec<DateTime<Utc>> {
        candidates.iter().map(|c| c.scheduled_time).collect()
    }

    /// A range of 10 hourly fires, a batch of 4: three passes take 4, 4 and
    /// 2, in order, and stop at the end of the range. Then it is used up.
    #[test]
    fn backfill_candidates_go_through_the_range_in_batches_and_stop_at_its_end() {
        let new = NewBackfill::for_range("wf", "0 * * * *", "UTC", at(1, 0), at(1, 9), None, None)
            .unwrap();
        let now = at(20, 0);
        let mut last = new.seed_last_run_at;
        let mut passes = Vec::new();
        loop {
            let (candidates, used_up) =
                backfill_fire_candidates(&new.spec, &hourly(), last, now, 4).unwrap();
            if used_up {
                assert!(candidates.is_empty());
                break;
            }
            let end = candidates.last().unwrap();
            assert_eq!(end.last_run_at, end.scheduled_time);
            last = end.last_run_at;
            passes.push(candidates);
        }
        assert_eq!(
            passes.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![4, 4, 2]
        );
        let all: Vec<_> = passes.iter().flat_map(|p| times(p)).collect();
        assert_eq!(all, (0..10).map(|h| at(1, h)).collect::<Vec<_>>());

        // Inside the range `next_run_at` is the next fire. On the last fire
        // of the range it is that fire: the row stays due and the next pass
        // finds the range used up.
        assert_eq!(passes[0][0].next_run_at, at(1, 1));
        assert_eq!(passes[0][3].next_run_at, at(1, 4));
        assert_eq!(passes[2][1].scheduled_time, at(1, 9));
        assert_eq!(passes[2][1].next_run_at, at(1, 9));
    }

    /// A range that reaches past `now` is taken as its fire times pass.
    #[test]
    fn backfill_candidates_do_not_pass_now() {
        let new = NewBackfill::for_range("wf", "0 * * * *", "UTC", at(1, 0), at(1, 9), None, None)
            .unwrap();
        let now = at(1, 2) + chrono::Duration::minutes(30);
        let (candidates, used_up) =
            backfill_fire_candidates(&new.spec, &hourly(), new.seed_last_run_at, now, 100).unwrap();
        assert!(!used_up);
        assert_eq!(times(&candidates), vec![at(1, 0), at(1, 1), at(1, 2)]);
        assert_eq!(candidates[2].next_run_at, at(1, 3));

        // Nothing to take yet, and not used up.
        let (candidates, used_up) =
            backfill_fire_candidates(&new.spec, &hourly(), at(1, 2), now, 100).unwrap();
        assert!(candidates.is_empty());
        assert!(!used_up);
    }

    /// A retry takes the fire times of its list only.
    #[test]
    fn backfill_candidates_of_a_retry_follow_its_list() {
        let list = vec![at(1, 3), at(1, 7), at(2, 5)];
        let new =
            NewBackfill::for_fire_times("wf", "0 * * * *", "UTC", list.clone(), None, None, None)
                .unwrap();
        let now = at(20, 0);
        let (first, used_up) =
            backfill_fire_candidates(&new.spec, &hourly(), new.seed_last_run_at, now, 2).unwrap();
        assert!(!used_up);
        assert_eq!(times(&first), vec![at(1, 3), at(1, 7)]);
        assert_eq!(first[0].next_run_at, at(1, 7));
        assert_eq!(first[1].next_run_at, at(2, 5));

        let (second, _) = backfill_fire_candidates(&new.spec, &hourly(), at(1, 7), now, 2).unwrap();
        assert_eq!(times(&second), vec![at(2, 5)]);
        assert_eq!(second[0].next_run_at, at(2, 5));

        let (third, used_up) =
            backfill_fire_candidates(&new.spec, &hourly(), at(2, 5), now, 2).unwrap();
        assert!(third.is_empty());
        assert!(used_up);
    }

    /// The backfill with a real runner on SQLite.
    #[cfg(feature = "sqlite")]
    mod runs {
        use super::super::super::*;
        use super::at;
        use crate::models::backfill::{BackfillCounts, BackfillSpec, BackfillStatus, NewBackfill};
        use crate::retry::RetryPolicy;
        use crate::runner::{DefaultRunner, DefaultRunnerConfig};
        use crate::{Task, TaskError, TaskNamespace, Workflow};
        use async_trait::async_trait;
        use std::collections::HashSet;

        /// What one run saw.
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct SeenRun {
            scheduled_time: String,
            interval_start: String,
            interval_end: String,
            backfill_id: String,
            region: String,
        }

        /// A task that records the keys of its run, waits until the gate is
        /// open, and fails when the end of its interval is in `fail_on`.
        struct IntervalTask {
            seen: Arc<parking_lot::Mutex<Vec<SeenRun>>>,
            gate: watch::Receiver<bool>,
            fail_on: Arc<parking_lot::Mutex<HashSet<String>>>,
        }

        #[async_trait]
        impl Task for IntervalTask {
            async fn execute(
                &self,
                context: Context<serde_json::Value>,
            ) -> Result<Context<serde_json::Value>, TaskError> {
                let text = |key: &str| {
                    context
                        .get(key)
                        .and_then(|value| value.as_str())
                        .unwrap_or("missing")
                        .to_string()
                };
                let run = SeenRun {
                    scheduled_time: text("scheduled_time"),
                    interval_start: text("interval_start"),
                    interval_end: text("interval_end"),
                    backfill_id: text("backfill_id"),
                    region: text("region"),
                };
                let interval_end = run.interval_end.clone();
                self.seen.lock().push(run);
                let mut gate = self.gate.clone();
                while !*gate.borrow() {
                    if gate.changed().await.is_err() {
                        break;
                    }
                }
                if self.fail_on.lock().contains(&interval_end) {
                    return Err(TaskError::ExecutionFailed {
                        message: format!("no data for the interval that ends {interval_end}"),
                        task_id: "work".to_string(),
                        timestamp: Utc::now(),
                    });
                }
                Ok(context)
            }

            fn id(&self) -> &str {
                "work"
            }

            fn dependencies(&self) -> &[TaskNamespace] {
                &[]
            }

            fn retry_policy(&self) -> RetryPolicy {
                RetryPolicy::builder().max_attempts(0).build()
            }
        }

        struct Registered {
            seen: Arc<parking_lot::Mutex<Vec<SeenRun>>>,
            gate: watch::Sender<bool>,
            fail_on: Arc<parking_lot::Mutex<HashSet<String>>>,
        }

        fn register_workflow(runtime: &Runtime, name: &str, gate_open: bool) -> Registered {
            let seen = Arc::new(parking_lot::Mutex::new(Vec::new()));
            let fail_on = Arc::new(parking_lot::Mutex::new(HashSet::new()));
            let (gate, gate_rx) = watch::channel(gate_open);
            let task: Arc<dyn Task> = Arc::new(IntervalTask {
                seen: seen.clone(),
                gate: gate_rx,
                fail_on: fail_on.clone(),
            });
            let workflow = Workflow::builder(name)
                .add_task(task.clone())
                .expect("add task")
                .build()
                .expect("build workflow");
            let namespace = TaskNamespace::new(
                workflow.tenant(),
                workflow.package(),
                workflow.name(),
                "work",
            );
            runtime.register_task(namespace, move || task.clone());
            runtime.register_workflow(name.to_string(), move || workflow.clone());
            Registered {
                seen,
                gate,
                fail_on,
            }
        }

        async fn start_runner(
            runtime: &Arc<Runtime>,
            dir: &tempfile::TempDir,
            max_catchup: usize,
        ) -> DefaultRunner {
            let url = format!("sqlite://{}", dir.path().join("scheduler.db").display());
            let config = DefaultRunnerConfig::builder()
                .enable_cron_scheduling(true)
                .cron_poll_interval(Duration::from_millis(200))
                .cron_max_catchup_executions(max_catchup)
                .cron_enable_recovery(false)
                .enable_trigger_scheduling(false)
                .enable_registry_reconciler(false)
                .build()
                .expect("runner config");
            DefaultRunner::builder()
                .database_url(&url)
                .with_config(config)
                .runtime_arc(runtime.clone())
                .build()
                .await
                .expect("runner")
        }

        async fn wait_until<F, Fut>(timeout: Duration, mut check: F) -> bool
        where
            F: FnMut() -> Fut,
            Fut: std::future::Future<Output = bool>,
        {
            let deadline = Instant::now() + timeout;
            loop {
                if check().await {
                    return true;
                }
                if Instant::now() >= deadline {
                    return false;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }

        async fn counts(dal: &DAL, id: UniversalUuid) -> BackfillCounts {
            dal.schedule().backfill_counts(id).await.expect("counts")
        }

        /// Waits until the backfill row is disabled. Returns the most runs
        /// seen active at one look.
        async fn wait_disabled(dal: &DAL, id: UniversalUuid, timeout: Duration) -> (bool, u64) {
            let most_active = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let done = wait_until(timeout, || {
                let dal = dal.clone();
                let most_active = most_active.clone();
                async move {
                    let running = counts(&dal, id).await.running;
                    most_active.fetch_max(running, std::sync::atomic::Ordering::SeqCst);
                    !dal.schedule()
                        .get_by_id(id)
                        .await
                        .expect("row")
                        .is_enabled()
                }
            })
            .await;
            (done, most_active.load(std::sync::atomic::Ordering::SeqCst))
        }

        fn status(row: &Schedule, counts: &BackfillCounts) -> BackfillStatus {
            let spec = BackfillSpec::parse(row.backfill.as_deref().unwrap()).unwrap();
            let evaluator = CronEvaluator::new(
                row.cron_expression.as_deref().unwrap(),
                row.timezone.as_deref().unwrap(),
            )
            .unwrap();
            let remaining = spec
                .remaining_after(&evaluator, row.last_run_at.unwrap().0)
                .unwrap();
            BackfillStatus::derive(row.is_enabled(), row.is_paused(), remaining, counts.running)
        }

        /// A backfill of 1,000 hourly intervals with a claim batch of 100
        /// and a limit of 25 active runs: ten batches. Each interval runs
        /// once, with its own interval keys, the id of the backfill and the
        /// params. One interval in fifty fails: the backfill goes on,
        /// reports them, and disables itself after the last interval. A
        /// retry of the failed intervals runs those only.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn backfill_beyond_the_batch_runs_each_interval_and_reports_the_failed_ones() {
            run_backfill_and_retry(1_000).await;
        }

        /// The load that the backfill was built for: 5,520 hourly intervals
        /// (230 days). Not in the default lane, for its run time:
        /// `cargo test -p cloacina --lib --features postgres,sqlite,macros
        /// backfill_of_the_motivating_history_load -- --ignored --nocapture`.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        #[ignore = "5,520 runs: run it by name"]
        async fn backfill_of_the_motivating_history_load() {
            run_backfill_and_retry(5_520).await;
        }

        async fn run_backfill_and_retry(total: i64) {
            const LIMIT: u32 = 25;
            let failures = (total as usize - 8) / 50 + 1;
            let runtime = Arc::new(Runtime::empty());
            let workflow = register_workflow(&runtime, "history_load", true);
            let dir = tempfile::tempdir().expect("temp dir");
            let runner = start_runner(&runtime, &dir, 100).await;
            let dal = runner.dal();

            let start = at(1, 0);
            let fire = |n: i64| start + chrono::Duration::hours(n);
            let failing: Vec<DateTime<Utc>> =
                (0..total).filter(|n| n % 50 == 7).map(fire).collect();
            assert_eq!(failing.len(), failures);
            *workflow.fail_on.lock() = failing.iter().map(|t| t.to_rfc3339()).collect();

            let new = NewBackfill::for_range(
                "history_load",
                "0 * * * *",
                "UTC",
                start,
                fire(total - 1),
                Some(LIMIT),
                Some(r#"{"region":"eu"}"#.to_string()),
            )
            .expect("backfill");
            assert_eq!(new.spec.total, total as u64);
            let started = Instant::now();
            let backfill = dal.schedule().create_backfill(new).await.expect("row");
            runner.notify_cron_change();

            // Progress is readable while it runs.
            assert!(
                wait_until(Duration::from_secs(120), || async {
                    counts(&dal, backfill.id).await.taken >= 100
                })
                .await,
                "the first batch was not taken"
            );
            let midway = counts(&dal, backfill.id).await;
            let row = dal.schedule().get_by_id(backfill.id).await.expect("row");
            assert!(midway.taken <= total as u64);
            assert_eq!(
                midway.taken,
                midway.completed + midway.failed + midway.running
            );
            if midway.taken < total as u64 {
                assert_eq!(status(&row, &midway), BackfillStatus::Running);
            }

            let (done, most_active) =
                wait_disabled(&dal, backfill.id, Duration::from_secs(900)).await;
            let end = counts(&dal, backfill.id).await;
            assert!(done, "the backfill did not disable itself: {end:?}");
            println!(
                "backfill of {total} intervals, limit {LIMIT}, batch 100: {:?}, {failures} failed, most active at one look {most_active}",
                started.elapsed()
            );
            assert!(
                most_active <= u64::from(LIMIT),
                "limit passed: {most_active}"
            );
            assert_eq!(
                end,
                BackfillCounts {
                    taken: total as u64,
                    completed: total as u64 - failures as u64,
                    failed: failures as u64,
                    running: 0,
                }
            );
            let row = dal.schedule().get_by_id(backfill.id).await.expect("row");
            assert_eq!(status(&row, &end), BackfillStatus::Completed);
            assert_eq!(row.last_run_at.map(|t| t.0), Some(fire(total - 1)));

            // Each interval ran once, with its keys.
            let id_text = backfill.id.to_string();
            let expected: HashSet<SeenRun> = (0..total)
                .map(|n| SeenRun {
                    scheduled_time: fire(n).to_rfc3339(),
                    interval_start: fire(n - 1).to_rfc3339(),
                    interval_end: fire(n).to_rfc3339(),
                    backfill_id: id_text.clone(),
                    region: "eu".to_string(),
                })
                .collect();
            let seen = workflow.seen.lock().clone();
            assert_eq!(seen.len(), total as usize, "each interval ran once");
            assert_eq!(seen.iter().cloned().collect::<HashSet<_>>(), expected);

            // The failed intervals are reported, oldest first.
            let failed = dal
                .schedule()
                .backfill_failed_intervals(backfill.id, 100_000)
                .await
                .expect("failed intervals");
            assert_eq!(
                failed.iter().map(|f| f.scheduled_time).collect::<Vec<_>>(),
                failing
            );
            assert!(failed.iter().all(|f| f.status == "Failed"));
            assert!(failed.iter().all(|f| f.workflow_execution_id.is_some()));

            // The backfill is not in the list of live schedules.
            assert!(
                dal.schedule()
                    .list(None, false, 100, 0)
                    .await
                    .expect("list")
                    .is_empty()
            );

            // Retry: a new backfill of the failed intervals only.
            workflow.fail_on.lock().clear();
            let retry = NewBackfill::for_fire_times(
                "history_load",
                "0 * * * *",
                "UTC",
                failed.iter().map(|f| f.scheduled_time).collect(),
                Some(5),
                row.params.clone(),
                Some(backfill.id),
            )
            .expect("retry");
            let retry = dal.schedule().create_backfill(retry).await.expect("row");
            runner.notify_cron_change();
            let (done, most_active) = wait_disabled(&dal, retry.id, Duration::from_secs(120)).await;
            let end = counts(&dal, retry.id).await;
            assert!(done, "the retry did not disable itself: {end:?}");
            assert!(most_active <= 5);
            assert_eq!(
                end,
                BackfillCounts {
                    taken: failures as u64,
                    completed: failures as u64,
                    failed: 0,
                    running: 0,
                }
            );
            let retry_id = retry.id.to_string();
            // Five of them run at once, so the order they ran in is not
            // fixed: the set is. (The order is in the test with a limit of
            // one.)
            let mut reruns: Vec<SeenRun> = workflow
                .seen
                .lock()
                .iter()
                .filter(|run| run.backfill_id == retry_id)
                .cloned()
                .collect();
            reruns.sort_by(|a, b| a.interval_end.cmp(&b.interval_end));
            assert_eq!(
                reruns
                    .iter()
                    .map(|r| r.interval_end.clone())
                    .collect::<Vec<_>>(),
                failing.iter().map(|t| t.to_rfc3339()).collect::<Vec<_>>(),
                "the retry runs the failed intervals, each once"
            );
            assert!(reruns.iter().all(|r| r.region == "eu"));
            // The source keeps its record.
            assert_eq!(counts(&dal, backfill.id).await.failed, failures as u64);

            runner.shutdown().await.expect("shutdown");
        }

        /// With the default limit of one active run, the intervals run
        /// oldest first, across claim batches.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn backfill_with_a_limit_of_one_runs_oldest_first() {
            let runtime = Arc::new(Runtime::empty());
            let workflow = register_workflow(&runtime, "ordered_load", true);
            let dir = tempfile::tempdir().expect("temp dir");
            let runner = start_runner(&runtime, &dir, 5).await;
            let dal = runner.dal();

            let start = at(3, 0);
            let fire = |n: i64| start + chrono::Duration::hours(n);
            let new = NewBackfill::for_range(
                "ordered_load",
                "0 * * * *",
                "UTC",
                start,
                fire(11),
                None,
                None,
            )
            .expect("backfill");
            let backfill = dal.schedule().create_backfill(new).await.expect("row");
            runner.notify_cron_change();

            let (done, most_active) =
                wait_disabled(&dal, backfill.id, Duration::from_secs(120)).await;
            assert!(
                done,
                "the backfill did not end: {:?}",
                counts(&dal, backfill.id).await
            );
            assert_eq!(most_active, 1, "one active run at a time");
            let order: Vec<String> = workflow
                .seen
                .lock()
                .iter()
                .map(|run| run.interval_end.clone())
                .collect();
            assert_eq!(
                order,
                (0..12).map(|n| fire(n).to_rfc3339()).collect::<Vec<_>>()
            );
            runner.shutdown().await.expect("shutdown");
        }

        /// Cancel disables the row: no new interval starts, the active runs
        /// end on their own, and the backfill reads as cancelled.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn cancelled_backfill_takes_no_more_intervals() {
            let runtime = Arc::new(Runtime::empty());
            let workflow = register_workflow(&runtime, "cancelled_load", false);
            let dir = tempfile::tempdir().expect("temp dir");
            let runner = start_runner(&runtime, &dir, 100).await;
            let dal = runner.dal();

            let start = at(5, 0);
            let new = NewBackfill::for_range(
                "cancelled_load",
                "0 * * * *",
                "UTC",
                start,
                start + chrono::Duration::hours(9),
                Some(2),
                None,
            )
            .expect("backfill");
            let backfill = dal.schedule().create_backfill(new).await.expect("row");
            runner.notify_cron_change();

            // Two intervals are taken, the limit, and a run has started.
            let two_taken = wait_until(Duration::from_secs(30), || async {
                counts(&dal, backfill.id).await.taken == 2 && !workflow.seen.lock().is_empty()
            })
            .await;
            let now_counts = counts(&dal, backfill.id).await;
            assert!(
                two_taken,
                "two intervals must be taken: seen {:?}, counts {:?}",
                workflow.seen.lock(),
                now_counts
            );
            // Held at its limit: nothing more is taken.
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let held = counts(&dal, backfill.id).await;
            assert_eq!((held.taken, held.running), (2, 2));

            // Cancel, then let the two active runs end.
            dal.schedule().disable(backfill.id).await.expect("cancel");
            runner.notify_cron_change();
            let row = dal.schedule().get_by_id(backfill.id).await.expect("row");
            assert_eq!(status(&row, &held), BackfillStatus::Running);
            workflow.gate.send(true).expect("gate");
            assert!(
                wait_until(Duration::from_secs(30), || async {
                    counts(&dal, backfill.id).await.running == 0
                })
                .await,
                "the active runs must end"
            );
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let end = counts(&dal, backfill.id).await;
            assert_eq!(
                end,
                BackfillCounts {
                    taken: 2,
                    completed: 2,
                    failed: 0,
                    running: 0,
                }
            );
            assert_eq!(workflow.seen.lock().len(), 2);
            let row = dal.schedule().get_by_id(backfill.id).await.expect("row");
            assert_eq!(status(&row, &end), BackfillStatus::Cancelled);
            runner.shutdown().await.expect("shutdown");
        }
    }
}
