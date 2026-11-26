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

//! Unified scheduler for both cron and trigger-based workflow execution.
//!
//! This module provides a single `Scheduler` that replaces the separate
//! `CronScheduler` and `TriggerScheduler`, driving both cron and trigger
//! schedules from one run loop backed by the unified `schedules` and
//! `schedule_executions` tables.
//!
//! # Key Features
//!
//! - **Single Run Loop**: One tick drives both cron and trigger checks
//! - **Atomic Claiming**: Prevents duplicate cron executions across instances
//! - **Per-trigger Poll Intervals**: Each trigger retains its own polling frequency
//! - **Context-based Deduplication**: Prevents duplicate trigger executions
//! - **Catchup Policies**: Configurable handling of missed cron executions
//! - **Audit Trail**: Records every handoff via `schedule_executions`
//! - **Saga Pattern**: Clean separation between scheduling and execution
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────┐    claim / fire   ┌──────────────────┐    execute    ┌─────────────┐
//! │   Scheduler   │   & hand off      │ WorkflowExecutor │  workflows   │   Tasks     │
//! │               │ ─────────────────▶│                  │ ────────────▶│             │
//! │ • Poll cron   │                   │ • Execute        │              │ • Business  │
//! │ • Poll trigs  │                   │ • Retry          │              │   Logic     │
//! │ • Deduplicate │                   │ • Recovery       │              │ • Context   │
//! │ • Audit log   │                   │                  │              │             │
//! └───────────────┘                   └──────────────────┘              └─────────────┘
//! ```

use crate::context::Context;
use crate::cron_evaluator::CronEvaluator;
use crate::dal::DAL;
use crate::dal::UnifiedRegistryStorage;
use crate::dal::unified::schedule::{CronClaimOutcome, CronFireCandidate};
use crate::dal::unified::truncate_predicate_text;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::executor::{WorkflowExecutionError, WorkflowExecutor};
use crate::models::schedule::{CatchupPolicy, NewSchedule, NewScheduleExecution, Schedule};
use crate::registry::workflow_registry::WorkflowRegistryImpl;
use crate::runtime::Runtime;
use crate::trigger::{Trigger, TriggerError};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Notify, watch};
use tracing::{debug, error, info, warn};

/// CLOACI-T-0922 — how many consecutive CEL evaluation errors on the SAME
/// reactor firing are tolerated before that firing is dead-lettered and the
/// watermark force-advances past it.
///
/// The bound exists because holding the watermark forever would wedge the
/// subscription: a firing whose payload shape genuinely breaks the predicate
/// never becomes evaluable, and every later firing would queue behind it. Five
/// poll ticks is enough to ride out a transient fault (a momentarily missing
/// payload key, a blip in payload decode) while still resuming promptly on a
/// structural one.
pub const MAX_CONSECUTIVE_PREDICATE_ERRORS: i32 = 5;

/// How often the completion accounting of a trigger-fired run reads the
/// status of its workflow execution (CLOACINA-T-1019).
const TRIGGER_COMPLETION_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// How many status reads in a row may fail before the completion accounting
/// of a trigger-fired run gives up and leaves the row open (CLOACINA-T-1019).
const TRIGGER_COMPLETION_MAX_READ_ERRORS: u32 = 20;

/// Configuration for the unified scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Backstop for the timer-driven cron loop (CLOACI-T-0743). The scheduler
    /// sleeps until the next due schedule's exact instant; this caps that sleep
    /// so the loop still re-checks the DB at least this often even absent a
    /// change notification — a safety net for missed notifies and the
    /// multi-instance case (an in-process notify only wakes the local replica).
    /// It is NOT a fixed poll: when the next fire is sooner than this, the loop
    /// wakes exactly at the fire time, not on this interval.
    pub cron_poll_interval: Duration,
    /// The most missed fire times that one claim of a `run_all` schedule
    /// takes. The fires beyond it are not lost: the schedule stays due and
    /// the next pass takes the next batch (CLOACINA-T-1020).
    pub max_catchup_executions: usize,
    /// How often the cron loop looks again at a schedule that it holds at
    /// its limit on active runs (CLOACINA-T-1020). A run of the schedule that
    /// ends in this process wakes the loop at once; this interval covers a
    /// run that another scheduler started, and a missed wake.
    pub cron_held_poll_interval: Duration,
    /// Base poll interval — the tick rate of the run loop.
    pub trigger_base_poll_interval: Duration,
    /// Maximum time to wait for a single trigger poll operation.
    pub trigger_poll_timeout: Duration,
    /// How often to poll reactor subscriptions for new firings
    /// (CLOACI-I-0100 / T-0599). Defaults to the base tick interval.
    pub reactor_poll_interval: Duration,
    /// Maximum number of unconsumed firings to drain per subscription
    /// per tick. Caps unbounded backlog work on a single tick.
    pub reactor_poll_batch_limit: i64,
    /// How often to prune old `reactor_firings` rows
    /// (CLOACI-I-0100 / T-0601). Defaults to 1 hour.
    pub reactor_firings_prune_interval: Duration,
    /// Retention window for `reactor_firings` rows. Anything with
    /// `fired_at < now - retention` is deleted on each prune sweep.
    /// Defaults to 7 days. Subscriptions whose watermark predates the
    /// retention window will miss firings — documented gotcha.
    pub reactor_firings_retention: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            cron_poll_interval: Duration::from_secs(30),
            max_catchup_executions: 100,
            cron_held_poll_interval: Duration::from_secs(1),
            trigger_base_poll_interval: Duration::from_secs(1),
            trigger_poll_timeout: Duration::from_secs(30),
            reactor_poll_interval: Duration::from_secs(1),
            reactor_poll_batch_limit: 100,
            reactor_firings_prune_interval: Duration::from_secs(60 * 60),
            reactor_firings_retention: Duration::from_secs(7 * 24 * 60 * 60),
        }
    }
}

/// Unified scheduler for both cron and trigger-based workflow execution.
///
/// The scheduler runs a single polling loop that:
/// 1. Ticks at `trigger_base_poll_interval` (default 1 s)
/// 2. Every `cron_poll_interval`, queries due cron schedules and processes them
/// 3. Every tick, checks enabled triggers respecting per-trigger poll intervals
/// 4. Records audit trail for every handoff via `schedule_executions`
///
/// # Responsibilities
///
/// **What Scheduler Does:**
/// - Poll database for due cron schedules and enabled triggers
/// - Atomically claim cron schedules
/// - Calculate missed execution times (catchup)
/// - Poll trigger functions and deduplicate
/// - Hand off workflow executions to the workflow executor
/// - Record execution audit trail
/// - Move on immediately (no waiting for completion)
///
/// **What Scheduler Does NOT Do:**
/// - Execute workflows directly
/// - Handle task retries or failures
/// - Wait for workflow completion
/// - Manage workflow state or recovery
#[derive(Clone)]
pub struct Scheduler {
    dal: Arc<DAL>,
    executor: Arc<dyn WorkflowExecutor>,
    config: SchedulerConfig,
    shutdown: watch::Receiver<bool>,
    /// Scoped runtime used to look up trigger constructors.
    runtime: Arc<Runtime>,
    /// Tracks when each trigger was last polled (by trigger name).
    last_poll_times: HashMap<String, Instant>,
    /// Wakes the timer-driven cron loop when schedules change (registered,
    /// enabled/disabled, deleted) so a new schedule fires on time instead of
    /// waiting for the backstop (CLOACI-T-0743). Shared with the cron registrar
    /// / runner cron API, which call `notify_one` after mutating schedules.
    cron_change: Arc<Notify>,
    /// The cron schedules that are due but held at their limit on active
    /// runs (CLOACINA-T-1020). They stay due in the database, so the loop
    /// leaves them out when it computes how long to sleep, and looks at them
    /// again each `cron_held_poll_interval` and when one of their runs ends.
    /// Shared by the clones of the scheduler that process schedules.
    cron_held: CronHeldSet,
    /// Tracks when reactor subscriptions were last polled
    /// (CLOACI-I-0100 / T-0599).
    last_reactor_poll: Option<Instant>,
    /// Tracks when the `reactor_firings` TTL prune last ran
    /// (CLOACI-I-0100 / T-0601).
    last_reactor_prune: Option<Instant>,
    /// Per-subscription compiled CEL predicate cache (CLOACI-T-0602).
    /// Key is the subscription id; value is `(expression_string, program)`
    /// so we can invalidate on expression-text change without restart.
    /// Arc<Mutex> for shared interior mutability across Scheduler clones
    /// (the active poller is single-threaded, but Clone is on the type).
    predicate_cache: PredicateCache,
}

/// Counts the passes of the cron check in this process, so a test can see
/// that a schedule held at its limit does not make the loop spin.
#[cfg(test)]
static CRON_CHECK_PASSES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The ids of the cron schedules held at their limit on active runs.
type CronHeldSet = Arc<parking_lot::Mutex<std::collections::HashSet<UniversalUuid>>>;

/// CLOACI-T-0602 — alias to satisfy clippy::type_complexity on the
/// Scheduler's predicate cache field.
type PredicateCache =
    Arc<parking_lot::Mutex<HashMap<UniversalUuid, (String, Arc<cel_interpreter::Program>)>>>;

/// How long the timer-driven cron loop should sleep before its next check
/// (CLOACI-T-0743), given the next due instant, the current time, and the
/// backstop. Pure so it's deterministically testable:
/// - next due in the future, sooner than the backstop → sleep until it
/// - next due in the future, beyond the backstop → sleep the backstop (re-check)
/// - next due now/past → `ZERO` (fire immediately)
/// - no schedules (`None`) → sleep the backstop
fn compute_cron_sleep_delay(
    next_due: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    backstop: Duration,
) -> Duration {
    match next_due {
        Some(t) if t <= now => Duration::ZERO,
        Some(t) => (t - now).to_std().unwrap_or(Duration::ZERO).min(backstop),
        None => backstop,
    }
}

/// Writes the keys that the scheduler owns into the context of a cron run:
/// `scheduled_time`, `schedule_id`, `schedule_timezone`,
/// `schedule_expression`, and the interval that the run covers
/// (CLOACINA-T-1020): `interval_start` is the fire time of the expression
/// before `scheduled_time`, `interval_end` is `scheduled_time`. Times are
/// RFC 3339 in UTC.
///
/// Called after the bound params are merged, and it replaces a key that is
/// already there, so on a scheduled run the scheduler's values win. The cron
/// fire path and the cron recovery sweep both use it, so a recovered run gets
/// the same interval as the run it replaces.
pub(crate) fn stamp_cron_fire_keys(
    context: &mut Context<serde_json::Value>,
    schedule: &Schedule,
    scheduled_time: DateTime<Utc>,
) -> Result<(), WorkflowExecutionError> {
    let cron_expr = schedule.cron_expression.as_deref().unwrap_or("* * * * *");
    let tz = schedule.timezone.as_deref().unwrap_or("UTC");
    let interval_start = CronEvaluator::new(cron_expr, tz)
        .and_then(|evaluator| evaluator.previous_execution(scheduled_time))
        .map_err(|e| WorkflowExecutionError::ExecutionFailed {
            message: format!("Cron evaluation error: {}", e),
        })?;

    let keys = [
        ("scheduled_time", scheduled_time.to_rfc3339()),
        ("schedule_id", schedule.id.to_string()),
        ("schedule_timezone", tz.to_string()),
        (
            "schedule_expression",
            schedule.cron_expression.clone().unwrap_or_default(),
        ),
        (
            crate::workflow_instance::INTERVAL_START_KEY,
            interval_start.to_rfc3339(),
        ),
        (
            crate::workflow_instance::INTERVAL_END_KEY,
            scheduled_time.to_rfc3339(),
        ),
    ];
    for (key, value) in keys {
        let value = serde_json::Value::String(value);
        if context.update(key, value.clone()).is_err() {
            context
                .insert(key, value)
                .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                    message: format!("Context error: {}", e),
                })?;
        }
    }
    Ok(())
}

/// Schedule lag in seconds (CLOACINA-T-1024): how far in the past the
/// earliest `next_run_at` of the enabled, not paused cron schedules is. 0 when
/// no schedule is due or there is none.
fn schedule_lag_seconds(next_due: Option<DateTime<Utc>>, now: DateTime<Utc>) -> f64 {
    match next_due {
        Some(t) if t < now => (now - t).num_milliseconds() as f64 / 1000.0,
        _ => 0.0,
    }
}

impl Scheduler {
    /// Creates a new unified scheduler.
    ///
    /// # Arguments
    /// * `dal` - Data access layer for database operations
    /// * `executor` - Workflow executor for workflow execution
    /// * `config` - Scheduler configuration
    /// * `shutdown` - Shutdown signal receiver
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        config: SchedulerConfig,
        shutdown: watch::Receiver<bool>,
        runtime: Arc<Runtime>,
        cron_change: Arc<Notify>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
            runtime,
            cron_change,
            cron_held: Arc::new(parking_lot::Mutex::new(std::collections::HashSet::new())),
            last_poll_times: HashMap::new(),
            last_reactor_poll: None,
            last_reactor_prune: None,
            predicate_cache: Arc::new(parking_lot::Mutex::new(HashMap::new())),
        }
    }

    /// Creates a new unified scheduler with default configuration.
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        shutdown: watch::Receiver<bool>,
        runtime: Arc<Runtime>,
    ) -> Self {
        Self::new(
            dal,
            executor,
            SchedulerConfig::default(),
            shutdown,
            runtime,
            Arc::new(Notify::new()),
        )
    }

    // -----------------------------------------------------------------------
    // Run loop
    // -----------------------------------------------------------------------

    /// Runs the main polling loop.
    ///
    /// Ticks at `trigger_base_poll_interval`. On each tick it:
    /// - Checks cron schedules if `cron_poll_interval` has elapsed since the
    ///   last cron check.
    /// - Checks all enabled triggers, respecting per-trigger poll intervals.
    ///
    /// The loop continues until a shutdown signal is received.
    pub async fn run_polling_loop(&mut self) -> Result<(), WorkflowExecutionError> {
        info!(
            "Starting unified scheduler (cron interval: {:?}, trigger base interval: {:?})",
            self.config.cron_poll_interval, self.config.trigger_base_poll_interval,
        );

        let mut interval = tokio::time::interval(self.config.trigger_base_poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Timer-driven cron (CLOACI-T-0743): instead of sweeping every
        // `cron_poll_interval`, cache the next due instant and sleep until it.
        // Recomputed only after a fire or a `cron_change` notification, so the
        // 1 s trigger/reactor tick does NOT re-query the DB for cron.
        //
        // CLOACI-T-0928: the sleep must be created ONCE and live across loop
        // iterations. The earlier shape rebuilt it from zero every iteration,
        // and the 1 s trigger tick always won the select first — so a sleep
        // longer than ~1 s never accumulated progress and the backstop could
        // never elapse. Now only the branches that consume or invalidate the
        // deadline reset it.
        let (mut next_cron_due, first_delay) = self.next_cron_wake().await;
        let cron_sleep = tokio::time::sleep(first_delay);
        tokio::pin!(cron_sleep);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // --- Schedule-lag gauge (CLOACINA-T-1024) ---
                    self.record_schedule_lag(next_cron_due);

                    // --- Triggers ---
                    if let Err(e) = self.check_and_process_triggers().await {
                        error!("Error processing triggers: {}", e);
                    }

                    // --- Reactor subscriptions (CLOACI-I-0100 / T-0599) ---
                    let now = Instant::now();
                    let should_poll_reactors = match self.last_reactor_poll {
                        Some(last) => now.duration_since(last) >= self.config.reactor_poll_interval,
                        None => true,
                    };
                    if should_poll_reactors {
                        self.last_reactor_poll = Some(now);
                        if let Err(e) = self.check_and_process_reactor_subscriptions().await {
                            error!("Error processing reactor subscriptions: {}", e);
                        }
                    }

                    // --- Reactor firings TTL prune (CLOACI-I-0100 / T-0601) ---
                    let should_prune = match self.last_reactor_prune {
                        Some(last) => {
                            now.duration_since(last) >= self.config.reactor_firings_prune_interval
                        }
                        None => true,
                    };
                    if should_prune {
                        self.last_reactor_prune = Some(now);
                        self.prune_reactor_firings().await;
                    }
                }
                // --- Cron: wake exactly at the next due instant (or backstop) ---
                _ = &mut cron_sleep => {
                    if let Err(e) = self.check_and_execute_cron_schedules().await {
                        error!("Error processing cron schedules: {}", e);
                    }
                    let (due, delay) = self.next_cron_wake().await;
                    next_cron_due = due;
                    // An elapsed Sleep stays ready forever — re-arm it, or this
                    // branch would win every subsequent select (busy loop).
                    cron_sleep.as_mut().reset(tokio::time::Instant::now() + delay);
                }
                // --- Cron schedule changed (registered/enabled/deleted): re-arm ---
                _ = self.cron_change.notified() => {
                    debug!("Cron change notification — recomputing next due time");
                    let (due, delay) = self.next_cron_wake().await;
                    next_cron_due = due;
                    cron_sleep.as_mut().reset(tokio::time::Instant::now() + delay);
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Unified scheduler received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Unified scheduler polling loop stopped");
        Ok(())
    }

    /// Query the earliest `next_run_at` over enabled cron schedules. Errors are
    /// logged and treated as "unknown" (`None`) so a transient DB hiccup falls
    /// back to the backstop rather than stalling the loop (CLOACI-T-0743).
    async fn query_next_cron_due(&self) -> Option<DateTime<Utc>> {
        match self.dal.schedule().next_cron_due_time().await {
            Ok(due) => due,
            Err(e) => {
                warn!("Failed to query next cron due time: {}", e);
                None
            }
        }
    }

    /// Set `cloacina_schedule_lag_seconds` from the cached next-due time
    /// (CLOACINA-T-1024). Runs on each base tick and reads no database: the
    /// cache is the earliest `next_run_at` of the enabled, not paused cron
    /// schedules, read again after each cron check, each cron change and at
    /// least once per `cron_poll_interval`. A due schedule that is not picked
    /// up keeps its `next_run_at`, so the gauge grows until it runs.
    fn record_schedule_lag(&self, next_due: Option<DateTime<Utc>>) {
        metrics::gauge!(
            "cloacina_schedule_lag_seconds",
            "tenant" => self.dal.tenant_label().to_string(),
        )
        .set(schedule_lag_seconds(next_due, Utc::now()));
    }

    /// The earliest due time of all cron schedules (for the lag gauge) and how
    /// long the cron loop sleeps before its next check.
    ///
    /// With no schedule held at its limit on active runs, the sleep follows
    /// the earliest due time, as before. A held schedule stays due, so its
    /// due time is in the past and would make the sleep zero for as long as
    /// it is held (CLOACINA-T-1020). So the sleep then follows the earliest
    /// due time of the OTHER schedules, capped at `cron_held_poll_interval`:
    /// the others still fire on time and the held ones are looked at again.
    async fn next_cron_wake(&self) -> (Option<DateTime<Utc>>, Duration) {
        let next_due = self.query_next_cron_due().await;
        let held: Vec<UniversalUuid> = self.cron_held.lock().iter().copied().collect();
        if held.is_empty() {
            return (next_due, self.cron_sleep_delay(next_due));
        }
        let delay = match self.dal.schedule().next_cron_due_time_excluding(held).await {
            Ok(others_due) => self
                .cron_sleep_delay(others_due)
                .min(self.config.cron_held_poll_interval),
            Err(e) => {
                warn!("Failed to query next cron due time: {}", e);
                self.config.cron_held_poll_interval
            }
        };
        (next_due, delay)
    }

    /// How long to sleep before the next cron check, given the cached next-due
    /// instant. Sleeps exactly until the due time when it's known and sooner
    /// than the backstop; otherwise caps at `cron_poll_interval` (the backstop)
    /// so the loop still re-checks periodically. A due time in the past yields
    /// `ZERO` (fire immediately). (CLOACI-T-0743)
    fn cron_sleep_delay(&self, next_due: Option<DateTime<Utc>>) -> Duration {
        compute_cron_sleep_delay(next_due, Utc::now(), self.config.cron_poll_interval)
    }

    // -----------------------------------------------------------------------
    // Cron schedule processing
    // -----------------------------------------------------------------------

    /// Checks for due cron schedules and executes them.
    async fn check_and_execute_cron_schedules(&self) -> Result<(), WorkflowExecutionError> {
        let now = Utc::now();
        debug!("Checking for due cron schedules at {}", now);
        #[cfg(test)]
        CRON_CHECK_PASSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let due_schedules = self
            .dal
            .schedule()
            .get_due_cron_schedules(now)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: e.to_string(),
            })?;

        // A held schedule that is no longer due (disabled, paused, deleted,
        // or moved on by another scheduler) is no longer held.
        {
            let mut held = self.cron_held.lock();
            if !held.is_empty() {
                held.retain(|id| due_schedules.iter().any(|schedule| schedule.id == *id));
            }
        }

        if due_schedules.is_empty() {
            debug!("No due cron schedules found");
            return Ok(());
        }

        info!("Found {} due cron schedule(s)", due_schedules.len());

        // Process each due schedule on its own task (CLOACI-T-0743). The handoff
        // `executor.execute(...)` blocks until the workflow runs, so processing
        // sequentially made a second schedule due at the same instant wait for
        // the first workflow's entire execution before it was even picked up
        // (observed: ~3–10s, the first workflow's run time). Spawning keeps the
        // scheduler loop non-blocking ("move on immediately" per this module's
        // contract) so co-due schedules dispatch concurrently and the loop
        // returns to its sleep immediately. Per-row `claim_and_update_cron` is
        // atomic, so concurrent processing of distinct schedules is safe.
        for schedule in due_schedules {
            let this = self.clone();
            tokio::spawn(async move {
                if let Err(e) = this.process_cron_schedule(&schedule, now).await {
                    error!("Failed to process cron schedule {}: {}", schedule.id, e);
                }
            });
        }

        Ok(())
    }

    /// Processes one due cron schedule: takes its next fire times and hands
    /// each run off. It does not wait for a run to end.
    ///
    /// The claim (`ScheduleDAL::claim_cron_fires`) decides how many of the
    /// fire times are taken now: all of them, or as many as the limit on
    /// active runs leaves room for. The schedule row moves only past the
    /// fires that were taken, so with `run_all` a backlog larger than
    /// `max_catchup_executions`, or larger than the limit, stays due and the
    /// next pass goes on with it, oldest first (CLOACINA-T-1020).
    async fn process_cron_schedule(
        &self,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> Result<(), WorkflowExecutionError> {
        debug!(
            "Processing cron schedule: {} (workflow: {})",
            schedule.id, schedule.workflow_name
        );

        // Check active time window
        if !self.is_cron_schedule_active(schedule, now) {
            debug!(
                "Cron schedule {} is outside its active time window, skipping",
                schedule.id
            );
            return Ok(());
        }

        let Some(expected_next_run) = schedule.next_run_at.map(|t| t.0) else {
            return Ok(());
        };

        // The fire times this pass wants, with the schedule times for each.
        let candidates = self.cron_fire_candidates(schedule, now)?;
        if candidates.is_empty() {
            debug!(
                "No execution times calculated for cron schedule {}",
                schedule.id
            );
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
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: e.to_string(),
            })?;

        let claimed = match outcome {
            CronClaimOutcome::Claimed(claimed) => claimed,
            CronClaimOutcome::NotClaimed => {
                debug!(
                    "Cron schedule {} was already claimed by another instance",
                    schedule.id
                );
                return Ok(());
            }
            CronClaimOutcome::AtLimit { active } => {
                debug!(
                    "Cron schedule {} has {} active run(s), its limit; it stays due",
                    schedule.id, active
                );
                self.cron_held.lock().insert(schedule.id);
                return Ok(());
            }
        };
        self.cron_held.lock().remove(&schedule.id);

        info!(
            "Successfully claimed cron schedule {} for {} execution(s)",
            schedule.id,
            claimed.len()
        );

        for fire in claimed {
            let audit_record_id = fire.schedule_execution_id;
            let scheduled_time = fire.scheduled_time;

            // Hand off to the workflow executor (async — the execution id
            // exists as soon as the handoff is accepted). The schedule
            // execution row was written by the claim, before the handoff.
            match self.execute_cron_workflow(schedule, scheduled_time).await {
                Ok(handle) => {
                    let workflow_execution_id = UniversalUuid(handle.execution_id);

                    // Link the row AT HANDOFF (CLOACI-T-0914 finding 5), so
                    // cron_recovery reads the live workflow execution and
                    // does not take a long run for a lost handoff.
                    if let Err(e) = self
                        .dal
                        .schedule_execution()
                        .update_workflow_execution_id(audit_record_id, workflow_execution_id)
                        .await
                    {
                        error!(
                            "Failed to link audit record {} to workflow execution {} for cron schedule {}: {}",
                            audit_record_id, workflow_execution_id, schedule.id, e
                        );
                    }

                    info!(
                        "Handed off workflow {} for cron schedule {} (scheduled: {})",
                        schedule.workflow_name, schedule.id, scheduled_time
                    );

                    // Mark the row complete when the run ends, on a task of
                    // its own: this function does not wait for the run. The
                    // limit on active runs is what keeps the runs of a
                    // `run_all` backlog from all starting at once.
                    let this = self.clone();
                    let schedule_id = schedule.id;
                    let workflow_name = schedule.workflow_name.clone();
                    tokio::spawn(async move {
                        this.complete_cron_execution_when_done(
                            schedule_id,
                            audit_record_id,
                            workflow_execution_id,
                            &workflow_name,
                        )
                        .await;
                    });
                }
                Err(e) => {
                    error!(
                        "Failed to execute workflow {} for cron schedule {} (scheduled: {}): {}",
                        schedule.workflow_name, schedule.id, scheduled_time, e
                    );
                    // Mark execution complete (failed) so cron_recovery does not
                    // treat it as lost and retry it indefinitely.
                    if let Err(e) = self
                        .dal
                        .schedule_execution()
                        .complete(audit_record_id, Utc::now())
                        .await
                    {
                        warn!(
                            "Failed to mark cron schedule execution {} complete after failure: {}",
                            audit_record_id, e
                        );
                    }
                    // The slot of this run is free again.
                    self.cron_change.notify_one();
                }
            }
        }

        Ok(())
    }

    /// Waits for a cron-fired workflow execution to reach a terminal state,
    /// marks its schedule execution row complete, and wakes the cron loop so
    /// a schedule held at its limit on active runs takes its next fire at
    /// once (CLOACINA-T-1020).
    ///
    /// Runs on its own task. When the wait gives up (shutdown, or the status
    /// cannot be read), the row stays open and linked: the claim does not
    /// count a row whose run is terminal, and the cron recovery sweep marks
    /// such a row complete.
    async fn complete_cron_execution_when_done(
        &self,
        schedule_id: UniversalUuid,
        schedule_execution_id: UniversalUuid,
        workflow_execution_id: UniversalUuid,
        workflow_name: &str,
    ) {
        self.complete_trigger_execution_when_done(
            schedule_execution_id,
            workflow_execution_id,
            workflow_name,
        )
        .await;
        self.cron_held.lock().remove(&schedule_id);
        self.cron_change.notify_one();
    }

    /// The limit on active runs of a cron schedule. `None` is no limit.
    ///
    /// The declared `max_active_runs` of the schedule, or else the default
    /// of its catch-up policy: one for `run_all`, none for `skip`. See
    /// [`effective_max_active_runs`](crate::models::schedule::effective_max_active_runs).
    fn max_active_runs(&self, schedule: &Schedule) -> Option<i64> {
        crate::models::schedule::effective_max_active_runs(
            schedule.catchup_policy.as_deref(),
            schedule.max_active_runs,
        )
    }

    /// Checks if a cron schedule is within its active time window.
    fn is_cron_schedule_active(&self, schedule: &Schedule, now: DateTime<Utc>) -> bool {
        if let Some(start) = &schedule.start_date {
            if now < start.0 {
                return false;
            }
        }
        if let Some(end) = &schedule.end_date {
            if now > end.0 {
                return false;
            }
        }
        true
    }

    /// The fire times that a pass over a due schedule wants, oldest first,
    /// each with the times the schedule row gets when it is the last fire
    /// taken.
    ///
    /// * `skip`: one fire, at the due time of the schedule. The row moves to
    ///   the next fire after `now`, so the fire times missed in between are
    ///   skipped.
    /// * `run_all`: each fire time after the last one taken (`last_run_at`,
    ///   or `created_at` for a schedule that never ran) and before `now`, at
    ///   most `max_catchup_executions` of them. After fire `F` the row has
    ///   `last_run_at = F` and `next_run_at` = the fire after `F`. When that
    ///   is in the past the schedule stays due, and the next pass starts at
    ///   `F`: nothing beyond the batch is lost (CLOACINA-T-1020).
    fn cron_fire_candidates(
        &self,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> Result<Vec<CronFireCandidate>, WorkflowExecutionError> {
        let to_err =
            |e: crate::cron_evaluator::CronError| WorkflowExecutionError::ExecutionFailed {
                message: format!("Cron evaluation error: {}", e),
            };
        let cron_expr = schedule.cron_expression.as_deref().unwrap_or("* * * * *");
        let tz = schedule.timezone.as_deref().unwrap_or("UTC");
        let evaluator = CronEvaluator::new(cron_expr, tz).map_err(to_err)?;

        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        match CatchupPolicy::from(policy_str.to_string()) {
            CatchupPolicy::Skip => {
                let scheduled_time = schedule.next_run_at.map(|t| t.0).unwrap_or(now);
                Ok(vec![CronFireCandidate {
                    scheduled_time,
                    last_run_at: now,
                    next_run_at: evaluator.next_execution(now).map_err(to_err)?,
                }])
            }
            CatchupPolicy::RunAll => {
                let start_time = schedule
                    .last_run_at
                    .map(|t| t.0)
                    .unwrap_or(schedule.created_at.0);

                // At least one: a batch of zero would take nothing and leave
                // the schedule due for good.
                let batch = self.config.max_catchup_executions.max(1);
                let missed = evaluator
                    .executions_between(start_time, now, batch)
                    .map_err(to_err)?;

                if missed.len() >= batch {
                    debug!(
                        "Cron schedule {} has at least {} missed fire(s); this pass takes at most that many and the schedule stays due",
                        schedule.id, batch
                    );
                }

                let mut candidates = Vec::with_capacity(missed.len());
                for (index, fire) in missed.iter().enumerate() {
                    let next_run_at = match missed.get(index + 1) {
                        Some(next) => *next,
                        None => evaluator.next_execution(*fire).map_err(to_err)?,
                    };
                    candidates.push(CronFireCandidate {
                        scheduled_time: *fire,
                        last_run_at: *fire,
                        next_run_at,
                    });
                }
                Ok(candidates)
            }
        }
    }

    /// Executes a cron workflow by handing it off to the workflow executor.
    ///
    /// Returns the execution handle from `execute_async` so the caller can
    /// link the audit record at handoff (the execution id exists immediately)
    /// and separately wait for completion (CLOACI-T-0914 finding 5).
    async fn execute_cron_workflow(
        &self,
        schedule: &Schedule,
        scheduled_time: DateTime<Utc>,
    ) -> Result<crate::executor::WorkflowExecution, WorkflowExecutionError> {
        let mut context = Context::new();
        // CLOACI-I-0116: a named instance's bound params are delivered as
        // flat context keys; the reserved scheduler keys below are stamped
        // AFTER (merge skips them), so a binding can never spoof them.
        if let Some(ref params_json) = schedule.params {
            crate::workflow_instance::merge_instance_params(&mut context, params_json).map_err(
                |e| WorkflowExecutionError::ExecutionFailed {
                    message: format!("instance params merge: {}", e),
                },
            )?;
        }
        stamp_cron_fire_keys(&mut context, schedule, scheduled_time)?;

        info!(
            "Executing workflow '{}' for cron schedule {} (scheduled time: {})",
            schedule.workflow_name, schedule.id, scheduled_time
        );

        let handle = self
            .executor
            .execute_async(&schedule.workflow_name, context)
            .await?;

        debug!(
            "Successfully handed off workflow '{}' to executor (execution_id: {})",
            schedule.workflow_name, handle.execution_id
        );

        Ok(handle)
    }

    // -----------------------------------------------------------------------
    // Trigger schedule processing
    // -----------------------------------------------------------------------

    /// Checks all enabled triggers and processes those that are due.
    async fn check_and_process_triggers(&mut self) -> Result<(), WorkflowExecutionError> {
        debug!("Checking trigger schedules");

        let schedules = self
            .dal
            .schedule()
            .get_enabled_triggers()
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get trigger schedules: {}", e),
            })?;

        if schedules.is_empty() {
            debug!("No enabled trigger schedules found");
            return Ok(());
        }

        let now = Instant::now();

        for schedule in schedules {
            let trigger_name = schedule
                .trigger_name
                .as_deref()
                .unwrap_or("unknown")
                .to_string();

            // Check if this trigger is due for polling
            let poll_interval = schedule
                .poll_interval()
                .unwrap_or(self.config.trigger_base_poll_interval);
            let last_poll = self.last_poll_times.get(&trigger_name);

            let should_poll = match last_poll {
                Some(last) => now.duration_since(*last) >= poll_interval,
                None => true,
            };

            if !should_poll {
                continue;
            }

            // Process this trigger
            if let Err(e) = self.process_trigger(&schedule).await {
                error!("Failed to process trigger '{}': {}", trigger_name, e);
            }

            // Update last poll time
            self.last_poll_times.insert(trigger_name, now);
        }

        Ok(())
    }

    /// Processes a single trigger schedule.
    async fn process_trigger(&self, schedule: &Schedule) -> Result<(), TriggerError> {
        let trigger_name = schedule.trigger_name.as_deref().unwrap_or("unknown");

        debug!(
            "Processing trigger '{}' (workflow: {})",
            trigger_name, schedule.workflow_name
        );

        // Get the trigger instance from the scoped runtime
        let trigger = self.runtime.get_trigger(trigger_name).ok_or_else(|| {
            TriggerError::TriggerNotFound {
                name: trigger_name.to_string(),
            }
        })?;

        // Poll the trigger with timeout
        let poll_result = tokio::time::timeout(self.config.trigger_poll_timeout, trigger.poll())
            .await
            .map_err(|_| TriggerError::PollError {
                message: format!(
                    "Trigger '{}' poll timed out after {:?}",
                    trigger_name, self.config.trigger_poll_timeout
                ),
            })?
            .map_err(|e| {
                error!("Trigger '{}' poll error: {}", trigger_name, e);
                e
            })?;

        // Update last poll time in database
        let now = Utc::now();
        if let Err(e) = self.dal.schedule().update_last_poll(schedule.id, now).await {
            warn!(
                "Failed to update last_poll_at for trigger '{}': {}",
                trigger_name, e
            );
        }

        // Check if trigger should fire
        if !poll_result.should_fire() {
            debug!("Trigger '{}' returned Skip", trigger_name);
            return Ok(());
        }

        // Compute context hash for deduplication
        let context_hash = poll_result.context_hash();

        // Check for duplicate active execution (unless allow_concurrent)
        if !schedule.allows_concurrent() {
            let has_active = self
                .dal
                .schedule_execution()
                .has_active_execution(schedule.id, &context_hash)
                .await
                .map_err(|e| TriggerError::ConnectionPool(e.to_string()))?;

            if has_active {
                debug!(
                    "Trigger '{}' has active execution with same context hash, skipping",
                    trigger_name
                );
                return Ok(());
            }
        }

        info!(
            "Trigger '{}' fired, scheduling workflow '{}'",
            trigger_name, schedule.workflow_name
        );

        // Create execution audit record before handoff
        let execution = self
            .create_trigger_execution_audit(schedule.id, &context_hash)
            .await?;

        // Extract context from result
        let context = poll_result.into_context().unwrap_or_else(Context::new);

        // Hand off to the workflow executor. The handoff does not wait for the
        // run: a wait here would hold the one scheduler loop, and with it
        // every other trigger poll and cron fire, for the length of the run
        // (CLOACINA-T-1019).
        match self.execute_trigger_workflow(schedule, context).await {
            Ok(workflow_execution_id) => {
                // Link the execution to the workflow execution at handoff
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .update_workflow_execution_id(execution.id, workflow_execution_id)
                    .await
                {
                    warn!(
                        "Failed to link schedule execution to workflow execution: {}",
                        e
                    );
                }

                info!(
                    "Successfully scheduled workflow '{}' for trigger '{}' (execution: {})",
                    schedule.workflow_name, trigger_name, workflow_execution_id
                );

                // Completion accounting, off the loop: the row stays open
                // while the run is in flight (that is what the dedup by
                // context hash reads) and is marked complete when it ends.
                let this = self.clone();
                let schedule_execution_id = execution.id;
                let trigger_name = trigger_name.to_string();
                tokio::spawn(async move {
                    this.complete_trigger_execution_when_done(
                        schedule_execution_id,
                        workflow_execution_id,
                        &trigger_name,
                    )
                    .await;
                });
            }
            Err(e) => {
                error!(
                    "Failed to execute workflow '{}' for trigger '{}': {}",
                    schedule.workflow_name, trigger_name, e
                );
                // Mark execution as completed (failed)
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .complete(execution.id, Utc::now())
                    .await
                {
                    warn!(
                        "Failed to mark schedule execution as completed after failure: {}",
                        e
                    );
                }
                return Err(TriggerError::WorkflowSchedulingFailed {
                    workflow: schedule.workflow_name.clone(),
                    message: e.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Waits for a trigger-fired workflow execution to reach a terminal state
    /// and then marks its schedule execution row complete (CLOACINA-T-1019).
    ///
    /// Runs on its own task, never inside the scheduler loop. A failed status
    /// read is retried: marking the row complete while the run may still be
    /// live would let the same context fire a second, overlapping run. After
    /// [`TRIGGER_COMPLETION_MAX_READ_ERRORS`] consecutive failures, or on
    /// shutdown, the row is left open and linked; the cron recovery sweep
    /// backfills the completion of a linked row once its run is terminal.
    async fn complete_trigger_execution_when_done(
        &self,
        schedule_execution_id: UniversalUuid,
        workflow_execution_id: UniversalUuid,
        trigger_name: &str,
    ) {
        let mut shutdown = self.shutdown.clone();
        let mut consecutive_errors = 0u32;

        loop {
            if *shutdown.borrow() {
                debug!(
                    "Shutdown while waiting for trigger '{}' execution {}; leaving schedule execution {} open",
                    trigger_name, workflow_execution_id, schedule_execution_id
                );
                return;
            }

            match self
                .executor
                .get_execution_status(workflow_execution_id.0)
                .await
            {
                Ok(status) if status.is_terminal() => break,
                Ok(_) => consecutive_errors = 0,
                Err(e) => {
                    consecutive_errors += 1;
                    if consecutive_errors >= TRIGGER_COMPLETION_MAX_READ_ERRORS {
                        warn!(
                            "Could not read the status of workflow execution {} (trigger '{}') {} times in a row; leaving schedule execution {} open: {}",
                            workflow_execution_id,
                            trigger_name,
                            consecutive_errors,
                            schedule_execution_id,
                            e
                        );
                        return;
                    }
                }
            }

            tokio::select! {
                _ = tokio::time::sleep(TRIGGER_COMPLETION_POLL_INTERVAL) => {}
                changed = shutdown.changed() => {
                    if changed.is_err() {
                        // The shutdown sender is gone: the scheduler is gone.
                        return;
                    }
                }
            }
        }

        if let Err(e) = self
            .dal
            .schedule_execution()
            .complete(schedule_execution_id, Utc::now())
            .await
        {
            warn!(
                "Failed to mark schedule execution {} complete for trigger '{}': {}",
                schedule_execution_id, trigger_name, e
            );
        }
    }

    /// Creates an audit record for a trigger execution.
    async fn create_trigger_execution_audit(
        &self,
        schedule_id: UniversalUuid,
        context_hash: &str,
    ) -> Result<crate::models::schedule::ScheduleExecution, TriggerError> {
        let new_execution = NewScheduleExecution {
            schedule_id,
            workflow_execution_id: None,
            scheduled_time: None,
            claimed_at: None,
            context_hash: Some(context_hash.to_string()),
        };

        let execution = self
            .dal
            .schedule_execution()
            .create(new_execution)
            .await
            .map_err(|e| TriggerError::ConnectionPool(e.to_string()))?;

        debug!(
            "Created trigger execution audit record {} for schedule {}",
            execution.id, schedule_id
        );

        Ok(execution)
    }

    /// Hands a trigger workflow off to the workflow executor and returns the
    /// id of the workflow execution. It does not wait for the run to end.
    async fn execute_trigger_workflow(
        &self,
        schedule: &Schedule,
        mut context: Context<serde_json::Value>,
    ) -> Result<UniversalUuid, WorkflowExecutionError> {
        let trigger_name = schedule.trigger_name.as_deref().unwrap_or("unknown");

        // CLOACI-I-0116: bound instance params override same-named keys in
        // the trigger-produced payload (OQ-3); reserved keys stamped after.
        if let Some(ref params_json) = schedule.params {
            crate::workflow_instance::merge_instance_params(&mut context, params_json).map_err(
                |e| WorkflowExecutionError::ExecutionFailed {
                    message: format!("instance params merge: {}", e),
                },
            )?;
        }

        context
            .insert("trigger_name", serde_json::json!(trigger_name))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("triggered_at", serde_json::json!(Utc::now().to_rfc3339()))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        // CLOACI-T-0778: snapshot the context before executing so every
        // fanned-out workflow receives an identical copy (Context isn't Clone).
        let ctx_json = context
            .to_json()
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context serialize error: {}", e),
            })?;

        // Primary: the trigger's `on` workflow. Drives the audit record + return
        // value, and propagates a handoff error. `execute_async` returns as
        // soon as the execution exists (CLOACINA-T-1019).
        let handle = self
            .executor
            .execute_async(&schedule.workflow_name, context)
            .await?;
        let execution_id = handle.execution_id;

        debug!(
            "Successfully handed off workflow '{}' to executor (execution_id: {})",
            schedule.workflow_name, execution_id
        );

        // CLOACI-T-0778: fan out to every OTHER workflow subscribed to this
        // trigger via `#[workflow(triggers = […])]` — a trigger is a single point
        // that multiple workflows link to, and an auto-fire must reach all of them
        // (matching the manual fire, CLOACI-T-0777). Best-effort: a secondary
        // failure is logged, never fails the primary. Skipped for cron schedules
        // (no trigger_name) — they bind exactly one workflow.
        if schedule.trigger_name.is_some() {
            let storage = UnifiedRegistryStorage::new(self.dal.database().clone());
            if let Ok(registry) = WorkflowRegistryImpl::new(storage, self.dal.database().clone()) {
                match registry.find_trigger_subscribers(trigger_name).await {
                    Ok(subscribers) => {
                        for wf in subscribers {
                            if wf == schedule.workflow_name {
                                continue;
                            }
                            match Context::from_json(ctx_json.clone()) {
                                Ok(ctx) => match self.executor.execute_async(&wf, ctx).await {
                                    Ok(r) => debug!(
                                        "trigger '{}' fan-out: fired '{}' (execution_id: {})",
                                        trigger_name, wf, r.execution_id
                                    ),
                                    Err(e) => warn!(
                                        "trigger '{}' fan-out: failed to fire '{}': {}",
                                        trigger_name, wf, e
                                    ),
                                },
                                Err(e) => warn!(
                                    "trigger '{}' fan-out: context rebuild failed for '{}': {}",
                                    trigger_name, wf, e
                                ),
                            }
                        }
                    }
                    Err(e) => warn!(
                        "trigger '{}' fan-out: subscriber lookup failed: {}",
                        trigger_name, e
                    ),
                }
            }
        }

        Ok(UniversalUuid(execution_id))
    }

    // -----------------------------------------------------------------------
    // Reactor subscription processing (CLOACI-I-0100 / T-0599)
    // -----------------------------------------------------------------------

    /// Polls the `reactor_trigger_subscriptions` table and dispatches one
    /// workflow execution per unconsumed `reactor_firings` row.
    ///
    /// Watermark advance happens after dispatch — at-least-once on crash.
    /// Workflow idempotency is the user's concern (same as cron-triggered
    /// workflows).
    /// Run one pass over enabled reactor subscriptions, draining new
    /// firings and dispatching workflows. Exposed publicly so that
    /// integration tests can drive the loop deterministically without
    /// waiting on the background tick, and so operators can trigger
    /// an immediate poll in ad-hoc scripts.
    pub async fn poll_reactor_subscriptions_once(&self) -> Result<(), WorkflowExecutionError> {
        self.check_and_process_reactor_subscriptions().await
    }

    async fn check_and_process_reactor_subscriptions(&self) -> Result<(), WorkflowExecutionError> {
        let subs = match self.dal.reactor_subscriptions().list_all_enabled().await {
            Ok(rows) => rows,
            Err(e) => {
                warn!("Failed to list reactor subscriptions: {}", e);
                return Ok(());
            }
        };

        if subs.is_empty() {
            return Ok(());
        }

        debug!(
            "Polling {} reactor subscription(s) for new firings",
            subs.len()
        );

        for sub in subs {
            if let Err(e) = self.process_reactor_subscription(&sub).await {
                error!(
                    subscription = %sub.id.0,
                    reactor = %sub.reactor_name,
                    workflow = %sub.workflow_name,
                    "Failed to process reactor subscription: {}",
                    e
                );
            }
        }

        Ok(())
    }

    /// Drain new firings for one subscription and dispatch each as a
    /// workflow execution.
    async fn process_reactor_subscription(
        &self,
        sub: &crate::dal::unified::ReactorSubscription,
    ) -> Result<(), WorkflowExecutionError> {
        let firings = self
            .dal
            .reactor_subscriptions()
            .poll_unconsumed(
                &sub.tenant_id,
                &sub.reactor_name,
                sub.last_seen_fired_at,
                self.config.reactor_poll_batch_limit,
            )
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!(
                    "reactor poll_unconsumed failed for subscription {}: {}",
                    sub.id.0, e
                ),
            })?;

        // CLOACI-T-0602 — borrow the CEL predicate string (if any) so we
        // can evaluate it inside the per-firing loop below.
        let predicate_expr = sub.predicate_expression.as_deref();

        // CLOACI-T-0922 — the recovery write ("this predicate works again")
        // is issued at most once per drain, guarded by this flag plus the
        // row snapshot we already hold.
        let mut predicate_error_cleared = false;

        for firing in firings {
            // Build the workflow's input context from the firing payload.
            let mut context = Context::<serde_json::Value>::new();

            if let Some(payload) = &firing.payload {
                match bincode::deserialize::<std::collections::HashMap<String, Vec<u8>>>(
                    payload.as_slice(),
                ) {
                    Ok(entries) => {
                        for (source, bytes) in entries {
                            // Boundary payloads are bincode; surface as JSON
                            // when we can, otherwise as a hex string. The
                            // workflow author knows the source schema and
                            // can re-decode as needed.
                            let value = match serde_json::from_slice::<serde_json::Value>(&bytes) {
                                Ok(v) => v,
                                Err(_) => serde_json::json!(hex::encode(&bytes)),
                            };
                            if let Err(e) = context.insert(&source, value) {
                                warn!(
                                    "reactor firing {}: failed to insert source '{}' into context: {}",
                                    firing.id.0, source, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "reactor firing {}: failed to decode payload, dispatching with empty context: {}",
                            firing.id.0, e
                        );
                    }
                }
            }

            let _ = context.insert("reactor_name", serde_json::json!(sub.reactor_name.clone()));
            let _ = context.insert("reactor_firing_id", serde_json::json!(firing.id.0));
            let _ = context.insert(
                "reactor_fired_at",
                serde_json::json!(firing.fired_at.0.to_rfc3339()),
            );

            // CLOACI-T-0602 / T-0922 — predicate evaluation. If the
            // subscription carries a CEL filter, evaluate it now.
            //
            // The three outcomes are DELIBERATELY distinct (T-0922 —
            // they used to be two):
            //
            //   Ok(true)  — dispatch.
            //   Ok(false) — the filter did its job: skip dispatch AND
            //               advance the watermark. The firing was *seen*
            //               and consciously rejected.
            //   Err(_)    — the filter is BROKEN. Fail-closed still holds
            //               (we do not dispatch), but "don't dispatch"
            //               never implied "throw the firing away": the
            //               watermark is HELD so the firing is re-polled
            //               on the next tick. After
            //               MAX_CONSECUTIVE_PREDICATE_ERRORS attempts the
            //               firing is dead-lettered on the subscription
            //               row and the watermark force-advances, so one
            //               poison firing cannot wedge the subscription
            //               forever.
            if let Some(expr) = predicate_expr {
                match self.evaluate_predicate(sub.id, expr, &sub.tenant_id, &context) {
                    Ok(true) => {
                        self.clear_predicate_error_if_needed(sub, &mut predicate_error_cleared)
                            .await;
                    }
                    Ok(false) => {
                        self.clear_predicate_error_if_needed(sub, &mut predicate_error_cleared)
                            .await;
                        debug!(
                            subscription = %sub.id.0,
                            firing = %firing.id.0,
                            "predicate evaluated false; skipping dispatch + advancing watermark",
                        );
                        if let Err(e) = self
                            .dal
                            .reactor_subscriptions()
                            .advance_watermark(sub.id.0, firing.fired_at)
                            .await
                        {
                            warn!(
                                subscription = %sub.id.0,
                                firing = %firing.id.0,
                                "watermark advance failed for filtered firing; \
                                 it may re-evaluate next tick: {}",
                                e
                            );
                            return Ok(());
                        }
                        continue;
                    }
                    Err(eval_err) => {
                        metrics::counter!(
                            "cloacina_reactor_predicate_errors_total",
                            "reactor" => sub.reactor_name.clone(),
                        )
                        .increment(1);
                        warn!(
                            subscription = %sub.id.0,
                            firing = %firing.id.0,
                            reactor = %sub.reactor_name,
                            predicate = %truncate_predicate_text(expr),
                            "reactor predicate evaluation failed; HOLDING the watermark so \
                             the firing is retried (not dropped): {}",
                            eval_err
                        );

                        let attempts = match self
                            .dal
                            .reactor_subscriptions()
                            .record_predicate_error(sub.id.0, firing.id, &eval_err)
                            .await
                        {
                            Ok(n) => n,
                            Err(db_err) => {
                                warn!(
                                    subscription = %sub.id.0,
                                    firing = %firing.id.0,
                                    "failed to persist predicate error state; \
                                     watermark held, firing retried next tick: {}",
                                    db_err
                                );
                                // Can't count the attempt — hold anyway.
                                // Holding is the safe direction: the firing
                                // survives, and a persistently broken DB is
                                // its own louder alarm.
                                return Ok(());
                            }
                        };

                        if attempts < MAX_CONSECUTIVE_PREDICATE_ERRORS {
                            // Stop draining this subscription: the watermark
                            // stays put and this firing is head-of-line again
                            // next tick. Ordering is preserved and other
                            // subscriptions still progress.
                            return Ok(());
                        }

                        // Bound exceeded — dead-letter and force-advance.
                        error!(
                            subscription = %sub.id.0,
                            firing = %firing.id.0,
                            reactor = %sub.reactor_name,
                            workflow = %sub.workflow_name,
                            predicate = %truncate_predicate_text(expr),
                            attempts,
                            "reactor predicate failed {} consecutive times; DEAD-LETTERING \
                             this firing and advancing the watermark so the subscription \
                             resumes. Subscription marked degraded: {}",
                            attempts, eval_err
                        );
                        metrics::counter!(
                            "cloacina_reactor_predicate_dead_letters_total",
                            "reactor" => sub.reactor_name.clone(),
                        )
                        .increment(1);

                        if let Err(db_err) = self
                            .dal
                            .reactor_subscriptions()
                            .mark_predicate_degraded(sub.id.0, firing.id, &eval_err)
                            .await
                        {
                            warn!(
                                subscription = %sub.id.0,
                                firing = %firing.id.0,
                                "failed to persist degraded marker; \
                                 holding the watermark instead of advancing blind: {}",
                                db_err
                            );
                            return Ok(());
                        }

                        if let Err(db_err) = self
                            .dal
                            .reactor_subscriptions()
                            .advance_watermark(sub.id.0, firing.fired_at)
                            .await
                        {
                            warn!(
                                subscription = %sub.id.0,
                                firing = %firing.id.0,
                                "watermark advance failed after dead-lettering: {}",
                                db_err
                            );
                            return Ok(());
                        }
                        continue;
                    }
                }
            }

            // Dispatch — fire-and-forget. The poller hands off the
            // workflow and moves on; failures are surfaced via the
            // standard execution audit, not by blocking this tick.
            match self
                .executor
                .execute_async(&sub.workflow_name, context)
                .await
            {
                Ok(handle) => {
                    debug!(
                        subscription = %sub.id.0,
                        firing = %firing.id.0,
                        execution = %handle.execution_id,
                        "dispatched workflow '{}' for reactor '{}'",
                        sub.workflow_name, sub.reactor_name,
                    );
                }
                Err(e) => {
                    error!(
                        subscription = %sub.id.0,
                        firing = %firing.id.0,
                        "failed to dispatch workflow '{}' for reactor '{}': {}",
                        sub.workflow_name, sub.reactor_name, e
                    );
                    // Stop draining this subscription on dispatch error so
                    // the watermark stays put and the firing is retried on
                    // the next tick. Other subscriptions still progress.
                    return Err(e);
                }
            }

            // Advance watermark only after successful dispatch.
            if let Err(e) = self
                .dal
                .reactor_subscriptions()
                .advance_watermark(sub.id.0, firing.fired_at)
                .await
            {
                warn!(
                    subscription = %sub.id.0,
                    firing = %firing.id.0,
                    "watermark advance failed; firing may be re-dispatched: {}",
                    e
                );
                return Ok(());
            }
        }

        Ok(())
    }

    /// CLOACI-T-0922 — clear the persisted predicate-error state after a
    /// clean evaluation, at most once per drain.
    ///
    /// `sub` is the snapshot read at the top of the poll, so it tells us
    /// whether there is anything to clear without an extra read. Best
    /// effort: a failure here only means the retry budget is stale, which
    /// the next successful evaluation fixes.
    async fn clear_predicate_error_if_needed(
        &self,
        sub: &crate::dal::unified::ReactorSubscription,
        already_cleared: &mut bool,
    ) {
        if *already_cleared {
            return;
        }
        if sub.predicate_error_count == 0 && sub.predicate_degraded.is_false() {
            return;
        }
        *already_cleared = true;
        match self
            .dal
            .reactor_subscriptions()
            .clear_predicate_error(sub.id.0)
            .await
        {
            Ok(()) => info!(
                subscription = %sub.id.0,
                reactor = %sub.reactor_name,
                "reactor predicate evaluated cleanly again; cleared error count \
                 and degraded marker (last_predicate_error retained as history)",
            ),
            Err(e) => warn!(
                subscription = %sub.id.0,
                "failed to clear predicate error state after recovery: {}",
                e
            ),
        }
    }

    /// Evaluate a CEL predicate for a subscription firing
    /// (CLOACI-T-0602).
    ///
    /// Compiles `expr` on first sight per subscription id and caches
    /// the `Program` for future firings. If the expression text changes
    /// (subscriber re-subscribes with a different `when=`), the cache
    /// entry is invalidated by comparing the stored expression string.
    ///
    /// Returns:
    /// - `Ok(true)`  — predicate fired, dispatch should proceed.
    /// - `Ok(false)` — predicate did not fire, skip + advance watermark.
    /// - `Err(_)`    — compile error or runtime evaluation error. The
    ///   caller skips dispatch (fail-closed) but HOLDS the watermark and
    ///   retries, bounded by [`MAX_CONSECUTIVE_PREDICATE_ERRORS`]
    ///   (CLOACI-T-0922). "Don't dispatch" never meant "drop the event".
    ///
    /// Variables exposed to the CEL expression:
    /// - `payload`  — a map keyed by boundary source name, values are
    ///   the JSON-decoded payloads (or hex strings for non-JSON bytes).
    /// - `reactor`  — the reactor name (string).
    /// - `tenant`   — the tenant id (string).
    fn evaluate_predicate(
        &self,
        sub_id: UniversalUuid,
        expr: &str,
        tenant: &str,
        context: &Context<serde_json::Value>,
    ) -> Result<bool, String> {
        // Cache lookup. Re-compile only when the stored expression
        // string doesn't match — handles "subscriber upserted with a
        // new `when=`" without an explicit invalidation API.
        let program = {
            let mut cache = self.predicate_cache.lock();
            match cache.get(&sub_id) {
                Some((cached_expr, prog)) if cached_expr == expr => prog.clone(),
                _ => {
                    // CLOACI-T-0922 — panic-safe compile: the CEL parser
                    // panics on some malformed input, and this runs inside
                    // the scheduler's poll task.
                    let prog = Arc::new(
                        crate::dal::unified::compile_predicate(expr)
                            .map_err(|e| format!("compile error: {}", e))?,
                    );
                    cache.insert(sub_id, (expr.to_string(), prog.clone()));
                    prog
                }
            }
        };
        eval_cel_predicate_program(&program, tenant, context)
    }

    /// TTL prune of `reactor_firings` (CLOACI-I-0100 / T-0601).
    ///
    /// Best-effort: errors log warn and never propagate. Subscriptions
    /// whose `last_seen_fired_at` predates the cutoff will skip past
    /// firings that get pruned — documented gotcha in the tutorial.
    async fn prune_reactor_firings(&self) {
        let cutoff_dt = Utc::now()
            - chrono::Duration::from_std(self.config.reactor_firings_retention)
                .unwrap_or(chrono::Duration::days(7));
        let cutoff = UniversalTimestamp(cutoff_dt);

        match self
            .dal
            .reactor_subscriptions()
            .prune_firings_older_than(cutoff)
            .await
        {
            Ok(0) => {
                debug!("reactor_firings prune: no rows older than {}", cutoff_dt);
            }
            Ok(n) => {
                debug!(
                    "reactor_firings prune: deleted {} row(s) older than {}",
                    n, cutoff_dt
                );
                metrics::counter!("cloacina_reactor_firings_pruned_total").increment(n as u64);
            }
            Err(e) => {
                warn!("reactor_firings prune failed: {}", e);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Trigger management (public API)
    // -----------------------------------------------------------------------

    /// Registers a trigger with the scheduler.
    ///
    /// Persists the trigger configuration to the database for recovery across
    /// restarts. The trigger must also be registered in the global trigger
    /// registry for the actual polling function.
    ///
    /// # Arguments
    /// * `trigger` - The trigger instance to register
    /// * `workflow_name` - Name of the workflow to fire when trigger activates
    pub async fn register_trigger(
        &self,
        trigger: &dyn Trigger,
        workflow_name: &str,
    ) -> Result<Schedule, ValidationError> {
        let mut new_schedule =
            NewSchedule::trigger(trigger.name(), workflow_name, trigger.poll_interval());
        new_schedule.allow_concurrent = Some(crate::database::universal_types::UniversalBool::new(
            trigger.allow_concurrent(),
        ));

        // Upsert to handle re-registration
        self.dal.schedule().upsert_trigger(new_schedule).await
    }

    /// Disables a trigger by name.
    pub async fn disable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .schedule()
            .get_by_trigger_name(trigger_name)
            .await?
        {
            self.dal.schedule().disable(schedule.id).await?;
            info!("Disabled trigger '{}'", trigger_name);
        }
        Ok(())
    }

    /// Enables a trigger by name.
    pub async fn enable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .schedule()
            .get_by_trigger_name(trigger_name)
            .await?
        {
            self.dal.schedule().enable(schedule.id).await?;
            info!("Enabled trigger '{}'", trigger_name);
        }
        Ok(())
    }
}

/// Evaluate a compiled CEL `Program` against a workflow context, returning
/// the boolean result. CLOACI-T-0602 helper, factored so the cache + pure
/// evaluation logic can be tested independently.
fn eval_cel_predicate_program(
    program: &cel_interpreter::Program,
    tenant: &str,
    context: &Context<serde_json::Value>,
) -> Result<bool, String> {
    use cel_interpreter::{Context as CelContext, Value as CelValue};

    let mut cel_ctx = CelContext::default();
    let mut payload = serde_json::Map::new();
    for (k, v) in context.data().iter() {
        if k == "reactor_name" || k == "reactor_firing_id" || k == "reactor_fired_at" {
            continue;
        }
        payload.insert(k.clone(), v.clone());
    }
    cel_ctx
        .add_variable("payload", serde_json::Value::Object(payload))
        .map_err(|e| format!("cel add_variable(payload): {}", e))?;
    cel_ctx
        .add_variable(
            "reactor",
            context.get("reactor_name").cloned().unwrap_or_default(),
        )
        .map_err(|e| format!("cel add_variable(reactor): {}", e))?;
    // CLOACI-T-0915: bind the real tenant id — this was a `""` stub while
    // the docs advertised the tenant, so any predicate referencing `tenant`
    // silently never matched (and, fail-closed, silently never fired).
    cel_ctx
        .add_variable("tenant", serde_json::Value::String(tenant.to_string()))
        .map_err(|e| format!("cel add_variable(tenant): {}", e))?;

    match program.execute(&cel_ctx) {
        Ok(CelValue::Bool(b)) => Ok(b),
        Ok(other) => Err(format!("predicate must evaluate to bool, got {:?}", other)),
        Err(e) => Err(format!("eval error: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::{UniversalBool, current_timestamp};

    fn create_test_cron_schedule(cron_expr: &str, timezone: &str) -> Schedule {
        let now = current_timestamp();
        Schedule {
            id: UniversalUuid::new_v4(),
            schedule_type: "cron".to_string(),
            workflow_name: "test_workflow".to_string(),
            enabled: UniversalBool::new(true),
            cron_expression: Some(cron_expr.to_string()),
            timezone: Some(timezone.to_string()),
            catchup_policy: Some("skip".to_string()),
            start_date: None,
            end_date: None,
            trigger_name: None,
            poll_interval_ms: None,
            allow_concurrent: None,
            next_run_at: Some(now),
            last_run_at: None,
            last_poll_at: None,
            created_at: now,
            updated_at: now,
            paused: UniversalBool::new(false),
            paused_at: None,
            params: None,
            instance_name: None,
            max_active_runs: None,
        }
    }

    fn create_test_trigger_schedule(trigger_name: &str) -> Schedule {
        let now = current_timestamp();
        Schedule {
            id: UniversalUuid::new_v4(),
            schedule_type: "trigger".to_string(),
            workflow_name: "test_workflow".to_string(),
            enabled: UniversalBool::new(true),
            cron_expression: None,
            timezone: None,
            catchup_policy: None,
            start_date: None,
            end_date: None,
            trigger_name: Some(trigger_name.to_string()),
            poll_interval_ms: Some(5000),
            allow_concurrent: Some(UniversalBool::new(false)),
            next_run_at: None,
            last_run_at: None,
            last_poll_at: None,
            created_at: now,
            updated_at: now,
            paused: UniversalBool::new(false),
            paused_at: None,
            params: None,
            instance_name: None,
            max_active_runs: None,
        }
    }

    /// CLOACINA-T-1020: a cron run gets the interval it covers, and the
    /// scheduler's values replace the ones a bound param set.
    #[test]
    fn cron_fire_keys_carry_the_interval_and_win_over_params() {
        let schedule = create_test_cron_schedule("30 2 * * *", "America/New_York");
        let mut context = Context::new();
        crate::workflow_instance::merge_instance_params(
            &mut context,
            r#"{"interval_start": "spoofed", "interval_end": "spoofed", "source": "/a"}"#,
        )
        .unwrap();

        // The day after the spring-forward change in New York: the fire of
        // the day before was moved to 03:00 EDT (07:00 UTC).
        let scheduled_time: DateTime<Utc> = "2026-03-09T06:30:00Z".parse().unwrap();
        stamp_cron_fire_keys(&mut context, &schedule, scheduled_time).unwrap();

        let get = |key: &str| context.get(key).cloned().unwrap();
        assert_eq!(get("interval_start"), "2026-03-08T07:00:00+00:00");
        assert_eq!(get("interval_end"), "2026-03-09T06:30:00+00:00");
        assert_eq!(get("scheduled_time"), "2026-03-09T06:30:00+00:00");
        assert_eq!(get("schedule_id"), schedule.id.to_string());
        assert_eq!(get("schedule_timezone"), "America/New_York");
        assert_eq!(get("schedule_expression"), "30 2 * * *");
        assert_eq!(get("source"), "/a");
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.cron_poll_interval, Duration::from_secs(30));
        assert_eq!(config.max_catchup_executions, 100);
        assert_eq!(config.trigger_base_poll_interval, Duration::from_secs(1));
        assert_eq!(config.trigger_poll_timeout, Duration::from_secs(30));
        assert_eq!(config.reactor_poll_interval, Duration::from_secs(1));
        assert_eq!(config.reactor_poll_batch_limit, 100);
        assert_eq!(
            config.reactor_firings_prune_interval,
            Duration::from_secs(3600)
        );
        assert_eq!(
            config.reactor_firings_retention,
            Duration::from_secs(7 * 86_400)
        );
    }

    #[test]
    fn test_is_cron_schedule_active_no_window() {
        let schedule = create_test_cron_schedule("0 * * * *", "UTC");
        let now = Utc::now();

        // No start/end date — always active
        let config = SchedulerConfig::default();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        // We can test the method directly by building a minimal Scheduler
        // but since it requires Arc<DAL> and Arc<dyn WorkflowExecutor>,
        // we just verify the schedule model itself
        assert!(schedule.start_date.is_none());
        assert!(schedule.end_date.is_none());
        // No window constraints => active
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(active);

        // Suppress unused variable warnings
        let _ = config;
        let _ = shutdown_rx;
    }

    #[test]
    fn test_is_cron_schedule_active_with_start_date_future() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        // Set start date to the future
        let future = Utc::now() + chrono::Duration::hours(1);
        schedule.start_date = Some(UniversalTimestamp(future));

        let now = Utc::now();
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(!active);
    }

    #[test]
    fn test_is_cron_schedule_active_with_end_date_past() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        // Set end date to the past
        let past = Utc::now() - chrono::Duration::hours(1);
        schedule.end_date = Some(UniversalTimestamp(past));

        let now = Utc::now();
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(!active);
    }

    #[test]
    fn test_catchup_policy_from_schedule() {
        let schedule = create_test_cron_schedule("0 * * * *", "UTC");
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        let policy = CatchupPolicy::from(policy_str.to_string());
        assert_eq!(policy, CatchupPolicy::Skip);
    }

    #[test]
    fn test_catchup_policy_run_all() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        schedule.catchup_policy = Some("run_all".to_string());
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        let policy = CatchupPolicy::from(policy_str.to_string());
        assert_eq!(policy, CatchupPolicy::RunAll);
    }

    #[test]
    fn test_trigger_schedule_helpers() {
        let schedule = create_test_trigger_schedule("file_watcher");
        assert!(schedule.is_trigger());
        assert!(!schedule.is_cron());
        assert!(schedule.is_enabled());
        assert_eq!(schedule.poll_interval(), Some(Duration::from_secs(5)));
        assert!(!schedule.allows_concurrent());
    }

    #[test]
    fn test_trigger_schedule_trigger_name_fallback() {
        let mut schedule = create_test_trigger_schedule("file_watcher");
        // Verify the as_deref().unwrap_or("unknown") pattern
        assert_eq!(
            schedule.trigger_name.as_deref().unwrap_or("unknown"),
            "file_watcher"
        );
        schedule.trigger_name = None;
        assert_eq!(
            schedule.trigger_name.as_deref().unwrap_or("unknown"),
            "unknown"
        );
    }

    // -----------------------------------------------------------------------
    // SchedulerConfig tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_scheduler_config_custom() {
        let config = SchedulerConfig {
            cron_poll_interval: Duration::from_secs(60),
            max_catchup_executions: 50,
            cron_held_poll_interval: Duration::from_secs(3),
            trigger_base_poll_interval: Duration::from_secs(5),
            trigger_poll_timeout: Duration::from_secs(10),
            reactor_poll_interval: Duration::from_secs(2),
            reactor_poll_batch_limit: 25,
            reactor_firings_prune_interval: Duration::from_secs(120),
            reactor_firings_retention: Duration::from_secs(86_400),
        };
        assert_eq!(config.cron_poll_interval, Duration::from_secs(60));
        assert_eq!(config.max_catchup_executions, 50);
        assert_eq!(config.trigger_base_poll_interval, Duration::from_secs(5));
        assert_eq!(config.trigger_poll_timeout, Duration::from_secs(10));
        assert_eq!(config.reactor_poll_interval, Duration::from_secs(2));
        assert_eq!(config.reactor_poll_batch_limit, 25);
        assert_eq!(
            config.reactor_firings_prune_interval,
            Duration::from_secs(120)
        );
        assert_eq!(
            config.reactor_firings_retention,
            Duration::from_secs(86_400)
        );
    }

    #[test]
    fn test_scheduler_config_clone() {
        let config = SchedulerConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.cron_poll_interval, config.cron_poll_interval);
        assert_eq!(cloned.max_catchup_executions, config.max_catchup_executions);
        assert_eq!(
            cloned.trigger_base_poll_interval,
            config.trigger_base_poll_interval
        );
        assert_eq!(cloned.trigger_poll_timeout, config.trigger_poll_timeout);
    }

    #[test]
    fn test_scheduler_config_debug() {
        let config = SchedulerConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("SchedulerConfig"));
        assert!(debug_str.contains("cron_poll_interval"));
    }

    // -----------------------------------------------------------------------
    // Cron schedule active window tests (expanded)
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_cron_schedule_active_both_bounds_containing_now() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        let past = Utc::now() - chrono::Duration::hours(1);
        let future = Utc::now() + chrono::Duration::hours(1);
        schedule.start_date = Some(UniversalTimestamp(past));
        schedule.end_date = Some(UniversalTimestamp(future));

        let now = Utc::now();
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(active);
    }

    #[test]
    fn test_is_cron_schedule_active_both_bounds_excluding_now() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        // Both in the future
        let future1 = Utc::now() + chrono::Duration::hours(1);
        let future2 = Utc::now() + chrono::Duration::hours(2);
        schedule.start_date = Some(UniversalTimestamp(future1));
        schedule.end_date = Some(UniversalTimestamp(future2));

        let now = Utc::now();
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(!active);
    }

    // -----------------------------------------------------------------------
    // Catchup policy parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_catchup_policy_unknown_defaults_to_skip() {
        let policy = CatchupPolicy::from("unknown_policy".to_string());
        assert_eq!(policy, CatchupPolicy::Skip);
    }

    #[test]
    fn test_catchup_policy_none_defaults_to_skip() {
        let schedule = create_test_cron_schedule("0 * * * *", "UTC");
        // catchup_policy is Some("skip") by default in our helper
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        assert_eq!(policy_str, "skip");
    }

    #[test]
    fn test_catchup_policy_missing_defaults_correctly() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        schedule.catchup_policy = None;
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        let policy = CatchupPolicy::from(policy_str.to_string());
        assert_eq!(policy, CatchupPolicy::Skip);
    }

    // -----------------------------------------------------------------------
    // Cron schedule model tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cron_schedule_helpers() {
        let schedule = create_test_cron_schedule("*/5 * * * *", "America/New_York");
        assert!(schedule.is_cron());
        assert!(!schedule.is_trigger());
        assert!(schedule.is_enabled());
        assert_eq!(schedule.cron_expression.as_deref(), Some("*/5 * * * *"));
        assert_eq!(schedule.timezone.as_deref(), Some("America/New_York"));
    }

    #[test]
    fn test_trigger_schedule_no_poll_interval() {
        let mut schedule = create_test_trigger_schedule("webhook");
        schedule.poll_interval_ms = None;
        // With no poll_interval_ms, poll_interval() should return None
        assert_eq!(schedule.poll_interval(), None);
    }

    #[test]
    fn test_trigger_schedule_allows_concurrent() {
        let mut schedule = create_test_trigger_schedule("queue_trigger");
        schedule.allow_concurrent = Some(UniversalBool::new(true));
        assert!(schedule.allows_concurrent());
    }

    #[test]
    fn test_trigger_schedule_no_concurrent_flag_defaults_false() {
        let mut schedule = create_test_trigger_schedule("queue_trigger");
        schedule.allow_concurrent = None;
        assert!(!schedule.allows_concurrent());
    }

    // ─────────────────────────────────────────────────────────────────
    // CLOACI-T-0602 — CEL predicate evaluation
    // ─────────────────────────────────────────────────────────────────

    fn ctx_with_payload(items: &[(&str, serde_json::Value)]) -> Context<serde_json::Value> {
        let mut c = Context::<serde_json::Value>::new();
        for (k, v) in items {
            c.insert(*k, v.clone()).unwrap();
        }
        c
    }

    #[test]
    fn cel_predicate_true_when_payload_matches() {
        let prog = cel_interpreter::Program::compile(
            "payload.quote.price > 100 && payload.quote.region == 'us-east'",
        )
        .unwrap();
        let ctx = ctx_with_payload(&[(
            "quote",
            serde_json::json!({"price": 150, "region": "us-east"}),
        )]);
        assert!(eval_cel_predicate_program(&prog, "public", &ctx).unwrap());
    }

    #[test]
    fn cel_predicate_false_when_payload_does_not_match() {
        let prog = cel_interpreter::Program::compile("payload.quote.price > 100").unwrap();
        let ctx = ctx_with_payload(&[("quote", serde_json::json!({"price": 50}))]);
        assert!(!eval_cel_predicate_program(&prog, "public", &ctx).unwrap());
    }

    #[test]
    fn cel_predicate_skips_bookkeeping_keys_from_payload() {
        // reactor_name / reactor_firing_id / reactor_fired_at are
        // exposed at the top level (`reactor`, no payload access), NOT
        // under `payload.*`. Predicate using `payload.reactor_name`
        // should not see anything.
        let prog = cel_interpreter::Program::compile("has(payload.reactor_name)").unwrap();
        let mut ctx = ctx_with_payload(&[("quote", serde_json::json!({"price": 50}))]);
        ctx.insert("reactor_name", serde_json::json!("pricing"))
            .unwrap();
        // With the bookkeeping keys stripped, payload.reactor_name
        // doesn't exist → has() returns false.
        assert!(!eval_cel_predicate_program(&prog, "public", &ctx).unwrap());
    }

    #[test]
    fn cel_predicate_tenant_binds_real_tenant_id() {
        // CLOACI-T-0915: `tenant` was a stub bound to "" — any predicate
        // referencing it silently never matched. It must carry the
        // subscription's tenant id.
        let prog = cel_interpreter::Program::compile("tenant == 'acme'").unwrap();
        let ctx = ctx_with_payload(&[("quote", serde_json::json!({"price": 50}))]);
        assert!(eval_cel_predicate_program(&prog, "acme", &ctx).unwrap());
        assert!(!eval_cel_predicate_program(&prog, "public", &ctx).unwrap());
        // The historical stub value must no longer match a real predicate.
        let prog_empty = cel_interpreter::Program::compile("tenant == ''").unwrap();
        assert!(!eval_cel_predicate_program(&prog_empty, "acme", &ctx).unwrap());
    }

    #[test]
    fn cel_predicate_non_bool_result_is_error() {
        let prog = cel_interpreter::Program::compile("payload.quote.price").unwrap();
        let ctx = ctx_with_payload(&[("quote", serde_json::json!({"price": 50}))]);
        let err = eval_cel_predicate_program(&prog, "public", &ctx).unwrap_err();
        assert!(
            err.contains("must evaluate to bool"),
            "expected bool-type error, got: {}",
            err
        );
    }

    #[test]
    fn cel_compile_rejects_malformed_expressions() {
        // Smoke that the upstream compile fails on garbage — this is
        // what `ReactorSubscriptionsDAL::subscribe` relies on to reject
        // bad predicates before the row is written.
        assert!(cel_interpreter::Program::compile("this is &&& not valid").is_err());
    }

    // --- Timer-driven cron sleep math (CLOACI-T-0743) ---

    #[test]
    fn cron_sleep_due_soon_sleeps_until_due_not_backstop() {
        let now = Utc::now();
        let backstop = Duration::from_secs(30);
        // Next fire in 3s, well under the 30s backstop → sleep ~3s.
        let due = now + chrono::Duration::seconds(3);
        let delay = compute_cron_sleep_delay(Some(due), now, backstop);
        assert!(
            delay >= Duration::from_millis(2500) && delay <= Duration::from_secs(3),
            "expected ~3s, got {:?}",
            delay
        );
    }

    #[test]
    fn cron_sleep_due_far_is_capped_at_backstop() {
        let now = Utc::now();
        let backstop = Duration::from_secs(30);
        // Next fire in 1 hour → capped at the 30s backstop (periodic re-check).
        let due = now + chrono::Duration::hours(1);
        assert_eq!(compute_cron_sleep_delay(Some(due), now, backstop), backstop);
    }

    #[test]
    fn cron_sleep_due_in_past_is_zero() {
        let now = Utc::now();
        let backstop = Duration::from_secs(30);
        let due = now - chrono::Duration::seconds(5);
        assert_eq!(
            compute_cron_sleep_delay(Some(due), now, backstop),
            Duration::ZERO
        );
    }

    // --- Schedule lag (CLOACINA-T-1024) ---

    #[test]
    fn schedule_lag_is_zero_with_no_schedule_or_none_due() {
        let now = Utc::now();
        assert_eq!(schedule_lag_seconds(None, now), 0.0);
        assert_eq!(schedule_lag_seconds(Some(now), now), 0.0);
        assert_eq!(
            schedule_lag_seconds(Some(now + chrono::Duration::seconds(30)), now),
            0.0
        );
    }

    #[test]
    fn schedule_lag_is_the_age_of_the_oldest_due_time() {
        let now = Utc::now();
        let due = now - chrono::Duration::milliseconds(90_500);
        assert_eq!(schedule_lag_seconds(Some(due), now), 90.5);
    }

    #[test]
    fn cron_sleep_no_schedules_uses_backstop() {
        let now = Utc::now();
        let backstop = Duration::from_secs(45);
        assert_eq!(compute_cron_sleep_delay(None, now, backstop), backstop);
    }

    // -----------------------------------------------------------------------
    // CLOACINA-T-1019 — a long run fired by a poll trigger must not hold the loop
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    mod trigger_handoff {
        use super::super::*;
        use crate::runner::{DefaultRunner, DefaultRunnerConfig};
        use crate::trigger::TriggerResult;
        use crate::{Task, TaskError, TaskNamespace, Workflow};
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// A task that counts its starts and then holds for `hold`, or until
        /// the test releases it.
        struct HoldTask {
            hold: Duration,
            started: Arc<AtomicUsize>,
            release: Arc<Notify>,
        }

        #[async_trait]
        impl Task for HoldTask {
            async fn execute(
                &self,
                context: Context<serde_json::Value>,
            ) -> Result<Context<serde_json::Value>, TaskError> {
                self.started.fetch_add(1, Ordering::SeqCst);
                if !self.hold.is_zero() {
                    tokio::select! {
                        _ = tokio::time::sleep(self.hold) => {}
                        _ = self.release.notified() => {}
                    }
                }
                Ok(context)
            }

            fn id(&self) -> &str {
                "work"
            }

            fn dependencies(&self) -> &[TaskNamespace] {
                &[]
            }
        }

        /// A poll trigger that fires on every poll with the same context.
        #[derive(Debug)]
        struct AlwaysFire {
            name: String,
            polls: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl Trigger for AlwaysFire {
            fn name(&self) -> &str {
                &self.name
            }

            fn poll_interval(&self) -> Duration {
                Duration::from_millis(100)
            }

            fn allow_concurrent(&self) -> bool {
                false
            }

            async fn poll(&self) -> Result<TriggerResult, cloacina_workflow::TriggerError> {
                self.polls.fetch_add(1, Ordering::SeqCst);
                let mut context = Context::new();
                context
                    .insert("source", serde_json::json!(self.name))
                    .expect("context insert");
                Ok(TriggerResult::Fire(Some(context)))
            }
        }

        /// Registers a one-task workflow and returns the start counter and the
        /// release handle of its task.
        fn register_workflow(
            runtime: &Runtime,
            name: &str,
            hold: Duration,
        ) -> (Arc<AtomicUsize>, Arc<Notify>) {
            let started = Arc::new(AtomicUsize::new(0));
            let release = Arc::new(Notify::new());
            let task: Arc<dyn Task> = Arc::new(HoldTask {
                hold,
                started: started.clone(),
                release: release.clone(),
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
            (started, release)
        }

        fn register_trigger(runtime: &Runtime, name: &str) -> Arc<AtomicUsize> {
            let polls = Arc::new(AtomicUsize::new(0));
            let trigger: Arc<dyn Trigger> = Arc::new(AlwaysFire {
                name: name.to_string(),
                polls: polls.clone(),
            });
            runtime.register_trigger(name.to_string(), move || trigger.clone());
            polls
        }

        /// Polls `check` every 50 ms until it is true or `timeout` passes.
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
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        /// A poll trigger fires a workflow that runs for 2 minutes. While that
        /// run is in flight, a second trigger and a cron schedule must still
        /// fire (hand their workflow off) within seconds, the slow trigger
        /// must not fire a second run for the same context, and each schedule
        /// execution row must be linked at handoff and marked complete when
        /// its run ends.
        ///
        /// "Fire" here is the handoff, read from the schedule execution rows.
        /// The runs of the other workflows cannot start while the slow task
        /// holds: on SQLite the runner dispatches tasks one at a time.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn slow_trigger_run_does_not_hold_other_triggers_or_cron() {
            let runtime = Arc::new(Runtime::empty());
            let (slow_started, slow_release) =
                register_workflow(&runtime, "slow_workflow", Duration::from_secs(120));
            let (fast_started, _) = register_workflow(&runtime, "fast_workflow", Duration::ZERO);
            register_workflow(&runtime, "cron_workflow", Duration::ZERO);
            let slow_polls = register_trigger(&runtime, "slow_trigger");
            let fast_polls = register_trigger(&runtime, "fast_trigger");

            let dir = tempfile::tempdir().expect("temp dir");
            let url = format!("sqlite://{}", dir.path().join("scheduler.db").display());
            let config = DefaultRunnerConfig::builder()
                .enable_cron_scheduling(true)
                .cron_poll_interval(Duration::from_millis(200))
                .cron_enable_recovery(false)
                .enable_trigger_scheduling(true)
                .trigger_base_poll_interval(Duration::from_millis(100))
                .enable_registry_reconciler(false)
                .build()
                .expect("runner config");
            let runner = DefaultRunner::builder()
                .database_url(&url)
                .with_config(config)
                .runtime_arc(runtime.clone())
                .build()
                .await
                .expect("runner");
            let dal = runner.dal();

            // The slow trigger fires first. Wait until its run is in flight.
            let slow_schedule = dal
                .schedule()
                .upsert_trigger(NewSchedule::trigger(
                    "slow_trigger",
                    "slow_workflow",
                    Duration::from_millis(100),
                ))
                .await
                .expect("slow trigger schedule");
            assert!(
                wait_until(Duration::from_secs(15), || async {
                    slow_started.load(Ordering::SeqCst) >= 1
                })
                .await,
                "the slow workflow did not start"
            );

            // Now add a second trigger and a cron schedule due in one second.
            let fast_schedule = dal
                .schedule()
                .upsert_trigger(NewSchedule::trigger(
                    "fast_trigger",
                    "fast_workflow",
                    Duration::from_millis(100),
                ))
                .await
                .expect("fast trigger schedule");
            let cron_due = Utc::now() + chrono::Duration::seconds(1);
            let cron_schedule = dal
                .schedule()
                .create(NewSchedule::cron(
                    "cron_workflow",
                    "* * * * *",
                    UniversalTimestamp(cron_due),
                ))
                .await
                .expect("cron schedule");

            let added = Instant::now();
            let linked_rows = |schedule_id: UniversalUuid| {
                let dal = dal.clone();
                async move {
                    dal.schedule_execution()
                        .list_by_schedule(schedule_id, 50, 0)
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|row| row.workflow_execution_id.is_some())
                        .count()
                }
            };
            let fired_in_time = wait_until(Duration::from_secs(6), || async {
                linked_rows(fast_schedule.id).await >= 1 && linked_rows(cron_schedule.id).await >= 1
            })
            .await;
            assert!(
                fired_in_time,
                "while the slow run was in flight, after {:?}: fast trigger polls = {}, fires = {}; \
                 cron fires = {} (each must be at least 1)",
                added.elapsed(),
                fast_polls.load(Ordering::SeqCst),
                linked_rows(fast_schedule.id).await,
                linked_rows(cron_schedule.id).await,
            );

            // The slow run is still in flight: one run, one open, linked row,
            // although the trigger kept firing the same context.
            assert!(slow_polls.load(Ordering::SeqCst) >= 2, "slow trigger polls");
            assert_eq!(slow_started.load(Ordering::SeqCst), 1);
            let slow_rows = dal
                .schedule_execution()
                .list_by_schedule(slow_schedule.id, 10, 0)
                .await
                .expect("slow rows");
            assert_eq!(slow_rows.len(), 1, "dedup by context hash must hold");
            assert!(slow_rows[0].completed_at.is_none());
            let slow_execution_id = slow_rows[0]
                .workflow_execution_id
                .expect("the row is linked at handoff");
            let slow_execution = dal
                .workflow_execution()
                .get_by_id(slow_execution_id)
                .await
                .expect("slow workflow execution");
            assert!(
                !matches!(slow_execution.status.as_str(), "Completed" | "Failed"),
                "the slow run must still be in flight, got {}",
                slow_execution.status
            );

            // Let the slow run end: its row is marked complete.
            slow_release.notify_one();
            assert!(
                wait_until(Duration::from_secs(15), || async {
                    dal.schedule_execution()
                        .get_by_id(slow_rows[0].id)
                        .await
                        .map(|r| r.completed_at.is_some())
                        .unwrap_or(false)
                })
                .await,
                "the slow run's row must be marked complete when the run ends"
            );

            // The fast runs can start now. A fast run ends, its row is marked
            // complete, and the same context fires again.
            assert!(
                wait_until(Duration::from_secs(15), || async {
                    let rows = dal
                        .schedule_execution()
                        .list_by_schedule(fast_schedule.id, 50, 0)
                        .await
                        .unwrap_or_default();
                    rows.len() >= 2 && rows.iter().any(|r| r.completed_at.is_some())
                })
                .await,
                "a finished fast run must complete its row and let the trigger fire again \
                 (fast runs started = {})",
                fast_started.load(Ordering::SeqCst)
            );

            let _ = dal.schedule().delete(slow_schedule.id).await;
            let _ = dal.schedule().delete(fast_schedule.id).await;
            let _ = dal.schedule().delete(cron_schedule.id).await;
            runner.shutdown().await.expect("shutdown");
        }
    }

    /// CLOACINA-T-1020: the cron fire path with a real runner on SQLite.
    mod cron_backlog {
        use super::super::*;
        use crate::runner::{DefaultRunner, DefaultRunnerConfig};
        use crate::{Task, TaskError, TaskNamespace, Workflow};
        use async_trait::async_trait;

        /// What one run saw: its fire time and its interval.
        #[derive(Debug, Clone, PartialEq, Eq)]
        struct SeenRun {
            scheduled_time: String,
            interval_start: String,
            interval_end: String,
        }

        /// A task that records the interval of its run, and then waits until
        /// the gate is open.
        struct IntervalTask {
            seen: Arc<parking_lot::Mutex<Vec<SeenRun>>>,
            gate: watch::Receiver<bool>,
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
                self.seen.lock().push(SeenRun {
                    scheduled_time: text("scheduled_time"),
                    interval_start: text("interval_start"),
                    interval_end: text("interval_end"),
                });
                let mut gate = self.gate.clone();
                while !*gate.borrow() {
                    if gate.changed().await.is_err() {
                        break;
                    }
                }
                Ok(context)
            }

            fn id(&self) -> &str {
                "work"
            }

            fn dependencies(&self) -> &[TaskNamespace] {
                &[]
            }
        }

        /// Registers a one-task workflow. Returns what its runs saw and the
        /// gate: send `true` to let each run, now and later, end.
        fn register_workflow(
            runtime: &Runtime,
            name: &str,
            gate_open: bool,
        ) -> (Arc<parking_lot::Mutex<Vec<SeenRun>>>, watch::Sender<bool>) {
            let seen = Arc::new(parking_lot::Mutex::new(Vec::new()));
            let (gate_tx, gate_rx) = watch::channel(gate_open);
            let task: Arc<dyn Task> = Arc::new(IntervalTask {
                seen: seen.clone(),
                gate: gate_rx,
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
            (seen, gate_tx)
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

        /// An every-minute `run_all` schedule whose row says that its last
        /// fire was `missed` minutes before the current minute. Returns the
        /// schedule and that last fire time.
        async fn schedule_with_missed_minutes(
            dal: &DAL,
            workflow: &str,
            missed: i64,
            max_active_runs: Option<i32>,
        ) -> (Schedule, DateTime<Utc>) {
            let this_minute = DateTime::<Utc>::from_timestamp(Utc::now().timestamp() / 60 * 60, 0)
                .expect("minute");
            let last_fire = this_minute - chrono::Duration::minutes(missed);
            // Created not due, so the scheduler leaves it until the row has
            // its backlog.
            let mut new = NewSchedule::cron(
                workflow,
                "* * * * *",
                UniversalTimestamp(Utc::now() + chrono::Duration::hours(1)),
            );
            new.catchup_policy = Some("run_all".to_string());
            new.max_active_runs = max_active_runs;
            let created = dal.schedule().create(new).await.expect("schedule");
            dal.schedule()
                .update_schedule_times(
                    created.id,
                    last_fire,
                    last_fire + chrono::Duration::minutes(1),
                )
                .await
                .expect("backlog");
            (created, last_fire)
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

        /// A `run_all` schedule missed 8 fire times and one claim takes at
        /// most 3. Each of the 8 runs, oldest first, one at a time, and each
        /// run gets the interval of its fire time. Before CLOACINA-T-1020
        /// the first claim took 3 and dropped the other 5.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn run_all_backlog_beyond_the_batch_is_kept_and_runs_in_order() {
            let runtime = Arc::new(Runtime::empty());
            let (seen, _gate) = register_workflow(&runtime, "history_workflow", true);
            let dir = tempfile::tempdir().expect("temp dir");
            let runner = start_runner(&runtime, &dir, 3).await;
            let dal = runner.dal();

            let (schedule, last_fire) =
                schedule_with_missed_minutes(&dal, "history_workflow", 8, None).await;
            runner.notify_cron_change();

            // Wait for 8 finished runs. On each look, count the active runs:
            // the open rows with no workflow execution yet, or with one that
            // has not ended. The limit of a `run_all` schedule is one. (A row
            // can stay open a moment after its run ended; it is not active.)
            let most_active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let drained = wait_until(Duration::from_secs(60), || {
                let dal = dal.clone();
                let most_active = most_active.clone();
                async move {
                    let rows = dal
                        .schedule_execution()
                        .list_by_schedule(schedule.id, 100, 0)
                        .await
                        .unwrap_or_default();
                    let mut active = 0;
                    for row in rows.iter().filter(|r| r.completed_at.is_none()) {
                        let ended = match row.workflow_execution_id {
                            None => false,
                            Some(id) => dal
                                .workflow_execution()
                                .get_by_id(id)
                                .await
                                .map(|run| {
                                    matches!(
                                        run.status.as_str(),
                                        "Completed" | "Failed" | "Cancelled"
                                    )
                                })
                                .unwrap_or(false),
                        };
                        if !ended {
                            active += 1;
                        }
                    }
                    most_active.fetch_max(active, std::sync::atomic::Ordering::SeqCst);
                    rows.iter().filter(|r| r.completed_at.is_some()).count() >= 8
                }
            })
            .await;
            dal.schedule().disable(schedule.id).await.expect("disable");
            assert!(
                drained,
                "8 missed fires must all run; runs seen: {:?}",
                seen.lock()
            );
            assert_eq!(
                most_active.load(std::sync::atomic::Ordering::SeqCst),
                1,
                "one active run at a time"
            );

            // The first 8 runs, in the order they ran, are the 8 missed fire
            // times in order, each with its own interval. (A ninth can follow
            // when the test crosses a minute.)
            let fire = |n: i64| (last_fire + chrono::Duration::minutes(n)).to_rfc3339();
            let expected: Vec<SeenRun> = (1..=8)
                .map(|n| SeenRun {
                    scheduled_time: fire(n),
                    interval_start: fire(n - 1),
                    interval_end: fire(n),
                })
                .collect();
            let seen_runs = seen.lock().clone();
            assert_eq!(&seen_runs[..8], &expected[..]);

            // The row moved to the last fire taken; nothing points back.
            let after = dal.schedule().get_by_id(schedule.id).await.expect("row");
            let taken = seen_runs.len() as i64;
            assert_eq!(
                after.last_run_at.map(|t| t.0),
                Some(last_fire + chrono::Duration::minutes(taken))
            );
            assert_eq!(
                after.next_run_at.map(|t| t.0),
                Some(last_fire + chrono::Duration::minutes(taken + 1))
            );

            let mut scheduled: Vec<DateTime<Utc>> = dal
                .schedule_execution()
                .list_by_schedule(schedule.id, 100, 0)
                .await
                .expect("rows")
                .into_iter()
                .filter_map(|row| row.scheduled_time.map(|t| t.0))
                .collect();
            scheduled.sort();
            scheduled.dedup();
            assert_eq!(scheduled.len() as i64, taken, "each fire time ran once");

            runner.shutdown().await.expect("shutdown");
        }

        /// A `run_all` schedule whose run does not end is at its limit of one
        /// active run. It does not fire again and stays due, with the times
        /// of the fire it took. Another schedule still fires on time, and
        /// the scheduler loop keeps sleeping between its checks. When the
        /// run ends, the next fire is taken at once.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn schedule_at_its_limit_stays_due_and_other_schedules_fire() {
            let runtime = Arc::new(Runtime::empty());
            let (held_seen, gate) = register_workflow(&runtime, "held_workflow", false);
            register_workflow(&runtime, "other_workflow", true);
            let dir = tempfile::tempdir().expect("temp dir");
            let runner = start_runner(&runtime, &dir, 100).await;
            let dal = runner.dal();

            let (held, last_fire) =
                schedule_with_missed_minutes(&dal, "held_workflow", 5, None).await;
            runner.notify_cron_change();
            assert!(
                wait_until(Duration::from_secs(15), || async {
                    held_seen.lock().len() == 1
                })
                .await,
                "the first missed fire did not start"
            );

            // A second schedule, due in one second, while the first is held.
            let other_due = Utc::now() + chrono::Duration::seconds(1);
            let other = dal
                .schedule()
                .create(NewSchedule::cron(
                    "other_workflow",
                    "* * * * *",
                    UniversalTimestamp(other_due),
                ))
                .await
                .expect("other schedule");
            runner.notify_cron_change();
            let handed_off = |schedule_id: UniversalUuid| {
                let dal = dal.clone();
                async move {
                    dal.schedule_execution()
                        .list_by_schedule(schedule_id, 50, 0)
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|row| row.workflow_execution_id.is_some())
                        .count()
                }
            };
            assert!(
                wait_until(Duration::from_secs(5), || async {
                    handed_off(other.id).await >= 1
                })
                .await,
                "a schedule at its limit must not keep another schedule from firing"
            );
            let other_row = dal
                .schedule_execution()
                .list_by_schedule(other.id, 10, 0)
                .await
                .expect("other rows")
                .remove(0);
            let late = other_row.created_at.0 - other_due;
            assert!(
                late < chrono::Duration::milliseconds(900),
                "the other schedule fired {late} after its due time"
            );

            // Three seconds on, the held schedule still has one run and its
            // row has not moved: four missed fires wait, none is lost.
            let passes_before = CRON_CHECK_PASSES.load(std::sync::atomic::Ordering::Relaxed);
            tokio::time::sleep(Duration::from_secs(3)).await;
            // The held schedule is due the whole time, and the loop must
            // still sleep between its checks. This runner checks 5 times a
            // second at most (its backstop is 200 ms); the count is shared
            // with the other tests of this process, so the bound is loose. A
            // loop that does not sleep makes thousands of passes.
            let passes =
                CRON_CHECK_PASSES.load(std::sync::atomic::Ordering::Relaxed) - passes_before;
            println!("cron checks in 3 s with a held schedule: {passes}");
            assert!(
                passes < 300,
                "{passes} cron checks in 3 seconds: the loop spins on the held schedule"
            );
            let rows = dal
                .schedule_execution()
                .list_by_schedule(held.id, 50, 0)
                .await
                .expect("held rows");
            assert_eq!(rows.len(), 1, "no second run while the first is active");
            assert_eq!(held_seen.lock().len(), 1);
            let row = dal.schedule().get_by_id(held.id).await.expect("held row");
            let first_fire = last_fire + chrono::Duration::minutes(1);
            assert_eq!(row.last_run_at.map(|t| t.0), Some(first_fire));
            assert_eq!(
                row.next_run_at.map(|t| t.0),
                Some(first_fire + chrono::Duration::minutes(1)),
                "the schedule stays due at its next missed fire"
            );

            // The run ends: the next fire is taken without waiting for a
            // poll. The gate stays open, so the rest of the backlog runs too.
            let released = Instant::now();
            gate.send(true).expect("open the gate");
            assert!(
                wait_until(Duration::from_secs(10), || async {
                    handed_off(held.id).await >= 2
                })
                .await,
                "the next missed fire must be taken when the run ends"
            );
            let waited = released.elapsed();
            assert!(
                wait_until(Duration::from_secs(30), || async {
                    held_seen.lock().len() >= 5
                })
                .await,
                "the rest of the backlog must run; runs seen: {:?}",
                held_seen.lock()
            );
            let seen_runs = held_seen.lock().clone();
            for (index, run) in seen_runs.iter().take(5).enumerate() {
                let n = index as i64 + 1;
                assert_eq!(
                    run.scheduled_time,
                    (last_fire + chrono::Duration::minutes(n)).to_rfc3339()
                );
            }
            println!("next fire taken {waited:?} after the run was released");

            let _ = dal.schedule().delete(held.id).await;
            let _ = dal.schedule().delete(other.id).await;
            runner.shutdown().await.expect("shutdown");
        }

        /// A `run_all` schedule that declares `max_active_runs = 3` and
        /// missed 7 fire times starts 3 runs and no more while they are
        /// active: the oldest 3, with the row at the third. When runs end,
        /// the rest follow.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn declared_max_active_runs_sets_how_fast_a_backlog_drains() {
            let runtime = Arc::new(Runtime::empty());
            let (seen, gate) = register_workflow(&runtime, "wide_workflow", false);
            let dir = tempfile::tempdir().expect("temp dir");
            let runner = start_runner(&runtime, &dir, 100).await;
            let dal = runner.dal();

            let (schedule, last_fire) =
                schedule_with_missed_minutes(&dal, "wide_workflow", 7, Some(3)).await;
            runner.notify_cron_change();

            let rows = || {
                let dal = dal.clone();
                async move {
                    dal.schedule_execution()
                        .list_by_schedule(schedule.id, 100, 0)
                        .await
                        .unwrap_or_default()
                }
            };
            assert!(
                wait_until(Duration::from_secs(15), || async {
                    rows()
                        .await
                        .iter()
                        .filter(|row| row.workflow_execution_id.is_some())
                        .count()
                        >= 3
                })
                .await,
                "3 runs must be handed off"
            );

            // Two seconds on: still 3 runs, the 3 oldest fire times.
            tokio::time::sleep(Duration::from_secs(2)).await;
            let mut taken: Vec<DateTime<Utc>> = rows()
                .await
                .into_iter()
                .filter_map(|row| row.scheduled_time.map(|t| t.0))
                .collect();
            taken.sort();
            let fire = |n: i64| last_fire + chrono::Duration::minutes(n);
            assert_eq!(taken, vec![fire(1), fire(2), fire(3)]);
            let row = dal.schedule().get_by_id(schedule.id).await.expect("row");
            assert_eq!(row.last_run_at.map(|t| t.0), Some(fire(3)));
            assert_eq!(row.next_run_at.map(|t| t.0), Some(fire(4)));

            // Runs end: the rest of the backlog runs.
            gate.send(true).expect("open the gate");
            assert!(
                wait_until(Duration::from_secs(30), || async { seen.lock().len() >= 7 }).await,
                "the 7 missed fires must all run; runs seen: {:?}",
                seen.lock()
            );
            let mut seen_times: Vec<String> = seen
                .lock()
                .iter()
                .take(7)
                .map(|run| run.scheduled_time.clone())
                .collect();
            seen_times.sort();
            assert_eq!(
                seen_times,
                (1..=7).map(|n| fire(n).to_rfc3339()).collect::<Vec<_>>()
            );

            let _ = dal.schedule().delete(schedule.id).await;
            runner.shutdown().await.expect("shutdown");
        }

        /// A `skip` schedule that fires each second and declares
        /// `max_active_runs = 1` does not start a second run while the first
        /// is active. Without the limit it would hand off one run a second.
        /// When the run ends, the schedule fires again, once, for the time it
        /// was held.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn skip_schedule_with_a_limit_does_not_overlap_its_runs() {
            let runtime = Arc::new(Runtime::empty());
            let (seen, gate) = register_workflow(&runtime, "no_overlap_workflow", false);
            let dir = tempfile::tempdir().expect("temp dir");
            let runner = start_runner(&runtime, &dir, 100).await;
            let dal = runner.dal();

            let mut new = NewSchedule::cron(
                "no_overlap_workflow",
                "* * * * * *",
                UniversalTimestamp(Utc::now()),
            );
            new.max_active_runs = Some(1);
            let schedule = dal.schedule().create(new).await.expect("schedule");
            runner.notify_cron_change();

            let row_count = || {
                let dal = dal.clone();
                async move {
                    dal.schedule_execution()
                        .list_by_schedule(schedule.id, 100, 0)
                        .await
                        .unwrap_or_default()
                        .len()
                }
            };
            assert!(
                wait_until(Duration::from_secs(15), || async { seen.lock().len() == 1 }).await,
                "the first run did not start"
            );

            // Three fire times pass. One run, one row.
            tokio::time::sleep(Duration::from_secs(3)).await;
            assert_eq!(row_count().await, 1, "no run while the first is active");
            let held = dal.schedule().get_by_id(schedule.id).await.expect("row");
            assert!(
                held.next_run_at.map(|t| t.0) < Some(Utc::now()),
                "the schedule stays due while it is held"
            );

            // The run ends, the gate closes again: one more run starts, and
            // again only one.
            gate.send(true).expect("open the gate");
            assert!(
                wait_until(Duration::from_secs(10), || async { row_count().await >= 2 }).await,
                "the schedule must fire again when its run ends"
            );
            gate.send(false).expect("close the gate");
            let rows_after_release = row_count().await;
            tokio::time::sleep(Duration::from_secs(2)).await;
            assert!(
                row_count().await <= rows_after_release + 1,
                "with the gate closed again, at most one more run may start"
            );

            dal.schedule().disable(schedule.id).await.expect("disable");
            gate.send(true).expect("open the gate");
            runner.shutdown().await.expect("shutdown");
        }
    }
}
