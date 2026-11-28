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

//! Background retention of run history and acked outbox rows (CLOACINA-T-1023).
//!
//! Every finished workflow run leaves rows in `workflow_executions`,
//! `task_executions`, `task_execution_metadata`, `contexts`,
//! `execution_events`, `recovery_events`, `schedule_executions` and
//! `delivery_outbox`. Without this loop nothing removes them.
//!
//! Both periods are off by default (CLOACINA-T-1092 for the outbox period):
//! an operator sets a period to turn a delete on. With both off the loop is
//! not started.
//!
//! On each tick the loop, for the admin (`public`) schema and then for every
//! tenant schema:
//!
//! 1. deletes finished runs older than the run-history period, with their
//!    dependent rows ([`cloacina::dal::unified::RetentionDAL`]);
//! 2. deletes `delivery_outbox` rows acked longer ago than the outbox period.
//!
//! Both steps work in bounded batches, with a cap on batches per schema per
//! tick, so one tick cannot run for long or hold large transactions. A backlog
//! (first start on an old database) drains over several ticks.
//!
//! # One replica at a time
//!
//! Every replica runs the loop, and each tick is gated on a Postgres advisory
//! lock with its own key ([`RETENTION_LOCK_KEY`]), the pattern of
//! CLOACINA-A-0008. A replica that does not get the lock skips the tick. The
//! lock is taken per tick, so two replicas can each run a pass within one
//! interval, one after the other; the deletes are idempotent, so that only
//! costs a second scan.
//!
//! This is separate from `--log-retention-days`, which only limits the number
//! of log files on disk.

use std::time::Duration;

use cloacina::dal::unified::{DAL, RunHistoryDeleted};
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::error::ValidationError;
use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::AppState;

/// Advisory-lock key of the retention loop. Distinct from the fleet control
/// key so the two loops never block each other. Encodes the task number
/// (`1023`) so it is greppable.
pub const RETENTION_LOCK_KEY: i64 = 8_111_023;

/// Finished runs removed per transaction.
const RUN_HISTORY_BATCH: i64 = 200;
/// Cap on run-history batches per schema per tick (5,000 runs).
const RUN_HISTORY_MAX_BATCHES: usize = 25;
/// Acked outbox rows removed per statement.
const OUTBOX_BATCH: i64 = 5_000;
/// Cap on outbox batches per schema per tick (100,000 rows).
const OUTBOX_MAX_BATCHES: usize = 20;

/// Counter of rows removed, labelled by `table`.
pub const ROWS_DELETED_METRIC: &str = "cloacina_retention_rows_deleted_total";
/// Counter of schema passes that failed.
pub const ERRORS_METRIC: &str = "cloacina_retention_errors_total";

/// What the retention loop keeps and how often it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionConfig {
    /// Keep finished runs this long. `None` turns run-history retention off.
    pub run_history: Option<Duration>,
    /// Keep acked `delivery_outbox` rows this long. `None` turns it off.
    pub outbox_acked: Option<Duration>,
    /// Time between passes.
    pub interval: Duration,
}

impl RetentionConfig {
    /// Builds the config from the server flags. `0` for either period means
    /// "off" (keep for ever); the interval is at least one second.
    pub fn from_flags(run_history_days: u64, outbox_acked_s: u64, interval_s: u64) -> Self {
        Self {
            run_history: (run_history_days > 0)
                .then(|| Duration::from_secs(run_history_days.saturating_mul(86_400))),
            outbox_acked: (outbox_acked_s > 0).then(|| Duration::from_secs(outbox_acked_s)),
            interval: Duration::from_secs(interval_s.max(1)),
        }
    }

    /// `false` when both periods are off: the loop is not started.
    pub fn enabled(&self) -> bool {
        self.run_history.is_some() || self.outbox_acked.is_some()
    }
}

impl Default for RetentionConfig {
    /// Run history and acked outbox rows kept for ever (each delete is off
    /// until an operator sets its period), a pass every five minutes: the
    /// defaults of the server flags.
    fn default() -> Self {
        Self::from_flags(0, 0, 300)
    }
}

/// Rows one pass removed from one schema.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SchemaSweep {
    /// Rows removed with finished runs, per table.
    pub run_history: RunHistoryDeleted,
    /// Acked `delivery_outbox` rows removed.
    pub outbox_acked: usize,
    /// A batch cap stopped the pass; the next tick continues.
    pub more: bool,
}

/// `now - period`, saturating far in the past for a period chrono cannot hold.
fn cutoff(now: chrono::DateTime<chrono::Utc>, period: Duration) -> UniversalTimestamp {
    let period = chrono::Duration::from_std(period).unwrap_or(chrono::Duration::MAX);
    UniversalTimestamp::from(
        now.checked_sub_signed(period)
            .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC),
    )
}

/// Runs one retention pass over the schema that `dal` is bound to.
///
/// The outbox step runs even when the run-history step fails, and the other
/// way round; the first error is returned after both were tried, with what was
/// removed so far.
pub async fn sweep_schema(
    dal: &DAL,
    cfg: &RetentionConfig,
    now: chrono::DateTime<chrono::Utc>,
) -> (SchemaSweep, Option<ValidationError>) {
    let mut sweep = SchemaSweep::default();
    let mut first_error = None;

    if let Some(period) = cfg.run_history {
        match dal
            .retention()
            .delete_finished_executions(
                cutoff(now, period),
                RUN_HISTORY_BATCH,
                RUN_HISTORY_MAX_BATCHES,
            )
            .await
        {
            Ok((deleted, more)) => {
                sweep.run_history = deleted;
                sweep.more |= more;
            }
            Err(e) => first_error = Some(e),
        }
    }

    if let Some(period) = cfg.outbox_acked {
        let cut = cutoff(now, period);
        let mut hit_cap = true;
        for _ in 0..OUTBOX_MAX_BATCHES {
            match dal
                .delivery_outbox()
                .delete_acked_older_than(cut, OUTBOX_BATCH)
                .await
            {
                Ok(n) => {
                    sweep.outbox_acked += n;
                    if (n as i64) < OUTBOX_BATCH {
                        hit_cap = false;
                        break;
                    }
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                    hit_cap = false;
                    break;
                }
            }
        }
        sweep.more |= hit_cap;
    }

    (sweep, first_error)
}

/// Adds one schema's counts to the per-table counter.
fn record(sweep: &SchemaSweep) {
    for (table, n) in sweep.run_history.per_table() {
        // `delivery_outbox` gets the acked rows too: one series per table.
        let n = if table == "delivery_outbox" {
            n + sweep.outbox_acked
        } else {
            n
        };
        if n > 0 {
            metrics::counter!(ROWS_DELETED_METRIC, "table" => table).increment(n as u64);
        }
    }
}

fn describe_metrics() {
    metrics::describe_counter!(
        ROWS_DELETED_METRIC,
        "Rows removed by the background retention job (CLOACINA-T-1023), by \
         `table`: workflow_executions, task_executions, \
         task_execution_metadata, contexts, execution_events, recovery_events, \
         schedule_executions, delivery_outbox. Summed over the admin schema \
         and all tenant schemas."
    );
    metrics::describe_counter!(
        ERRORS_METRIC,
        "Retention passes over one schema that failed. The pass is retried on \
         the next tick; sustained growth means run history is not being \
         removed."
    );
}

/// Starts the retention loop. Does nothing when both periods are off.
#[cfg(feature = "postgres")]
pub fn spawn(state: AppState, cfg: RetentionConfig, mut shutdown: watch::Receiver<bool>) {
    describe_metrics();
    if !cfg.enabled() {
        info!("retention: off (both periods are 0)");
        return;
    }
    info!(
        run_history_days = cfg.run_history.map(|d| d.as_secs() / 86_400),
        outbox_acked_s = cfg.outbox_acked.map(|d| d.as_secs()),
        interval_s = cfg.interval.as_secs(),
        "retention loop started (one replica per tick)"
    );

    tokio::spawn(async move {
        // The first pass runs one interval after boot, not during startup.
        let mut ticker =
            tokio::time::interval_at(tokio::time::Instant::now() + cfg.interval, cfg.interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    crate::autoscaler::leader::with_advisory_leadership(
                        &state.database,
                        RETENTION_LOCK_KEY,
                        "retention",
                        || run_pass(&state, &cfg),
                    )
                    .await;
                }
                res = shutdown.changed() => {
                    if res.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    });
}

/// One pass: the admin schema, then every tenant schema.
#[cfg(feature = "postgres")]
async fn run_pass(state: &AppState, cfg: &RetentionConfig) {
    let now = chrono::Utc::now();
    let mut schemas: Vec<String> = vec!["public".to_string()];
    match cloacina::database::DatabaseAdmin::new(state.database.clone())
        .list_tenant_schemas()
        .await
    {
        Ok(tenants) => schemas.extend(tenants),
        Err(e) => {
            // Still sweep the admin schema; the tenants wait for the next tick.
            metrics::counter!(ERRORS_METRIC).increment(1);
            warn!(error = %e, "retention: could not list tenant schemas");
        }
    }

    for schema in schemas {
        let db = match state
            .tenant_databases
            .resolve(&schema, &state.database)
            .await
        {
            Ok(db) => db,
            Err(e) => {
                metrics::counter!(ERRORS_METRIC).increment(1);
                warn!(schema = %schema, error = %e, "retention: no database for schema");
                continue;
            }
        };
        let (sweep, error) = sweep_schema(&DAL::new(db), cfg, now).await;
        record(&sweep);
        if let Some(e) = error {
            metrics::counter!(ERRORS_METRIC).increment(1);
            warn!(schema = %schema, error = %e, "retention: pass failed; retrying next tick");
        }
        let removed = sweep.run_history.total() + sweep.outbox_acked;
        if removed > 0 {
            info!(
                schema = %schema,
                runs = sweep.run_history.workflow_executions,
                rows = removed,
                outbox_acked = sweep.outbox_acked,
                more = sweep.more,
                "retention: removed rows"
            );
        } else {
            debug!(schema = %schema, "retention: nothing to remove");
        }
    }

    // CLOACINA-T-1080: the global outbox, `cloacina_global.delivery_outbox`,
    // through the admin pool. `public`'s own notices were cleared above.
    let global_cfg = RetentionConfig {
        run_history: None,
        ..*cfg
    };
    let (sweep, error) = sweep_schema(&DAL::new(state.database.clone()), &global_cfg, now).await;
    record(&sweep);
    if let Some(e) = error {
        metrics::counter!(ERRORS_METRIC).increment(1);
        warn!(error = %e, "retention: global outbox pass failed; retrying next tick");
    } else if sweep.outbox_acked > 0 {
        info!(
            outbox_acked = sweep.outbox_acked,
            more = sweep.more,
            "retention: removed rows of the global outbox"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_run_history_and_acked_outbox_rows() {
        let cfg = RetentionConfig::default();
        assert_eq!(cfg.run_history, None);
        assert_eq!(cfg.outbox_acked, None);
        assert_eq!(cfg.interval, Duration::from_secs(300));
        assert!(!cfg.enabled());
    }

    #[test]
    fn zero_turns_a_period_off() {
        let cfg = RetentionConfig::from_flags(0, 60, 30);
        assert_eq!(cfg.run_history, None);
        assert_eq!(cfg.outbox_acked, Some(Duration::from_secs(60)));
        assert!(cfg.enabled());

        let cfg = RetentionConfig::from_flags(7, 0, 30);
        assert_eq!(cfg.run_history, Some(Duration::from_secs(7 * 86_400)));
        assert_eq!(cfg.outbox_acked, None);
        assert!(cfg.enabled());

        assert!(!RetentionConfig::from_flags(0, 0, 30).enabled());
    }

    #[test]
    fn interval_is_at_least_one_second_and_huge_values_do_not_overflow() {
        assert_eq!(
            RetentionConfig::from_flags(1, 1, 0).interval,
            Duration::from_secs(1)
        );
        let cfg = RetentionConfig::from_flags(u64::MAX, u64::MAX, u64::MAX);
        // A period longer than time itself: the cutoff saturates, nothing is
        // old enough, nothing panics.
        let cut = cutoff(chrono::Utc::now(), cfg.run_history.unwrap());
        assert!(cut.0 < chrono::Utc::now() - chrono::Duration::days(365 * 1000));
    }

    #[test]
    fn lock_key_differs_from_the_fleet_control_key() {
        assert_ne!(
            RETENTION_LOCK_KEY,
            crate::autoscaler::leader::FLEET_CONTROL_LOCK_KEY
        );
    }
}
