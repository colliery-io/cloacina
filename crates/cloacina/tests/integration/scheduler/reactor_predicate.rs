/*
 *  Copyright 2026 Colliery Software
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

//! CLOACI-T-0602 — end-to-end test for CEL predicate filtering on
//! reactor-triggered workflows.
//!
//! Earlier T-0602 coverage:
//!   - unit tests on the pure CEL eval helper
//!     (`cron_trigger_scheduler::tests::cel_*`)
//!   - DAL integration tests for predicate persistence + rejection of
//!     malformed CEL (`tests/integration/dal/reactor_subscriptions.rs`)
//!
//! This file closes the gap between them: it exercises the actual
//! scheduler-side wiring — read predicate from the subscription,
//! evaluate it against the firing payload, dispatch or skip, advance
//! watermark — against a real database via `poll_reactor_subscriptions_once`.

use crate::fixtures::get_or_init_fixture;
use async_trait::async_trait;
use chrono::Timelike;
use cloacina::context::Context;
use cloacina::cron_trigger_scheduler::MAX_CONSECUTIVE_PREDICATE_ERRORS;
use cloacina::cron_trigger_scheduler::{Scheduler, SchedulerConfig};
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::executor::{
    StatusCallback, WorkflowExecution, WorkflowExecutionError, WorkflowExecutionResult,
    WorkflowExecutor, WorkflowStatus,
};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::Runtime;
use metrics_util::debugging::{DebugValue, DebuggingRecorder, Snapshotter};
use serial_test::serial;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::watch;
use uuid::Uuid;

/// Stub executor that records every `execute_async` call.
///
/// We don't actually need a workflow to run — just to know whether the
/// scheduler attempted to dispatch one. Calls are recorded as
/// `(workflow_name, payload_value)` pairs lifted from the firing
/// context; we deliberately don't try to clone the full `Context`
/// (which isn't Clone). All other trait methods are unreachable in
/// the reactor-poll code path and panic if called.
#[derive(Default)]
struct RecordingExecutor {
    /// (workflow_name, serialised "payload value" key extracted from
    /// the firing context). Lifting just the values we care about
    /// keeps the recording cheap and Clone-able.
    calls: Mutex<Vec<(String, Option<serde_json::Value>)>>,
    /// Inner DefaultRunner used solely to build a `WorkflowExecution`
    /// handle (its constructor requires one). Never actually invoked.
    inner_runner: Mutex<Option<DefaultRunner>>,
}

impl RecordingExecutor {
    fn snapshot(&self) -> Vec<(String, Option<serde_json::Value>)> {
        self.calls.lock().unwrap().clone()
    }

    fn set_inner(&self, runner: DefaultRunner) {
        *self.inner_runner.lock().unwrap() = Some(runner);
    }

    fn inner(&self) -> DefaultRunner {
        self.inner_runner
            .lock()
            .unwrap()
            .clone()
            .expect("RecordingExecutor inner runner not set — test bug")
    }
}

#[async_trait]
impl WorkflowExecutor for RecordingExecutor {
    async fn execute(
        &self,
        _workflow_name: &str,
        _context: Context<serde_json::Value>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        unreachable!("scheduler reactor path uses execute_async only")
    }

    async fn execute_async(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<WorkflowExecution, WorkflowExecutionError> {
        let value = context.get("value").cloned();
        self.calls
            .lock()
            .unwrap()
            .push((workflow_name.to_string(), value));
        Ok(WorkflowExecution::new(
            Uuid::new_v4(),
            workflow_name.to_string(),
            self.inner(),
        ))
    }

    async fn get_execution_status(
        &self,
        _execution_id: Uuid,
    ) -> Result<WorkflowStatus, WorkflowExecutionError> {
        unreachable!()
    }
    async fn get_execution_result(
        &self,
        _execution_id: Uuid,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        unreachable!()
    }
    async fn cancel_execution(&self, _execution_id: Uuid) -> Result<(), WorkflowExecutionError> {
        unreachable!()
    }
    async fn pause_execution(
        &self,
        _execution_id: Uuid,
        _reason: Option<&str>,
    ) -> Result<(), WorkflowExecutionError> {
        unreachable!()
    }
    async fn resume_execution(&self, _execution_id: Uuid) -> Result<(), WorkflowExecutionError> {
        unreachable!()
    }
    async fn execute_with_callback(
        &self,
        _workflow_name: &str,
        _context: Context<serde_json::Value>,
        _callback: Box<dyn StatusCallback>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        unreachable!()
    }
    async fn list_executions(
        &self,
    ) -> Result<Vec<WorkflowExecutionResult>, WorkflowExecutionError> {
        unreachable!()
    }
    async fn shutdown(&self) -> Result<(), WorkflowExecutionError> {
        Ok(())
    }
}

/// Build a `(source -> JSON-encoded bytes)` map into the bincode form
/// the scheduler expects for `reactor_firings.payload`. Tutorial-shaped
/// helper: payload key becomes a top-level context key, JSON-decoded.
fn build_firing_payload(source: &str, value: serde_json::Value) -> Vec<u8> {
    let inner = serde_json::to_vec(&value).expect("encode value");
    let mut map: HashMap<String, Vec<u8>> = HashMap::new();
    map.insert(source.to_string(), inner);
    bincode::serialize(&map).expect("bincode encode")
}

/// Scheduler config used by every test in this file. We drive the reactor
/// poll manually via `poll_reactor_subscriptions_once`, but every field
/// still gets a valid value so construction can't be rejected.
fn test_scheduler_config() -> SchedulerConfig {
    SchedulerConfig {
        cron_poll_interval: Duration::from_secs(30),
        max_catchup_executions: 0,
        max_acceptable_delay: Duration::from_secs(300),
        trigger_base_poll_interval: Duration::from_millis(50),
        trigger_poll_timeout: Duration::from_secs(5),
        reactor_poll_interval: Duration::from_millis(50),
        reactor_poll_batch_limit: 100,
        reactor_firings_prune_interval: Duration::from_secs(3600),
        reactor_firings_retention: Duration::from_secs(86400),
    }
}

/// A μs-aligned timestamp. Postgres `TIMESTAMP` stores microsecond
/// precision, so sub-μs nanos are lost on roundtrip; aligning the inputs
/// keeps watermark comparisons well-defined on both backends.
fn aligned_now() -> UniversalTimestamp {
    let now = UniversalTimestamp::now().0;
    let truncated_nanos = (now.timestamp_subsec_nanos() / 1_000) * 1_000;
    UniversalTimestamp(
        now.with_nanosecond(truncated_nanos)
            .expect("truncate nanos"),
    )
}

// ─────────────────────────────────────────────────────────────────────
// CLOACI-T-0922 — metric capture.
//
// `DebuggingRecorder::install` is process-global and one-shot, so it is
// installed lazily and shared. `Snapshotter::snapshot` SWAPS counters to
// zero, so every snapshot is the delta since the previous one.
// ─────────────────────────────────────────────────────────────────────
static METRICS_SNAPSHOTTER: OnceLock<Snapshotter> = OnceLock::new();

fn metrics_snapshotter() -> &'static Snapshotter {
    METRICS_SNAPSHOTTER.get_or_init(|| {
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        recorder
            .install()
            .expect("no other global metrics recorder should be installed in this test binary");
        snapshotter
    })
}

/// Sum of the counter `name` across all label sets, since the last
/// snapshot. Returns 0 when the counter was never touched.
fn counter_delta(name: &str) -> u64 {
    metrics_snapshotter()
        .snapshot()
        .into_vec()
        .into_iter()
        .filter(|(ck, _, _, _)| ck.key().name() == name)
        .filter_map(|(_, _, _, value)| match value {
            DebugValue::Counter(n) => Some(n),
            _ => None,
        })
        .sum()
}

/// End-to-end: subscribe with a CEL filter, insert two firings (one
/// matching the predicate, one not), run a single scheduler tick, and
/// verify:
///
///   - The matching firing's workflow was dispatched (exactly once).
///   - The non-matching firing's workflow was NOT dispatched.
///   - The watermark advanced past BOTH firings (the spec: filtered
///     firings still advance the watermark; they were *seen*).
#[tokio::test]
#[serial]
async fn test_predicate_filters_dispatch_and_advances_watermark_for_skips() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database = fixture.get_database();
    let dal = fixture.get_dal();

    // A DefaultRunner is only needed to satisfy WorkflowExecution::new
    // in the recording executor; we don't actually drive workflows
    // through it in this test. Reactor scheduling stays disabled on
    // the runner so its own background loop doesn't race ours.
    let runner = DefaultRunner::with_config(
        &fixture.get_database_url(),
        DefaultRunnerConfig::builder()
            .enable_cron_scheduling(false)
            .build()
            .unwrap(),
    )
    .await
    .expect("runner");

    let executor = Arc::new(RecordingExecutor::default());
    executor.set_inner(runner.clone());

    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    let runtime = Arc::new(Runtime::empty());
    let scheduler = Scheduler::new(
        Arc::new(cloacina::dal::DAL::new(database)),
        executor.clone(),
        test_scheduler_config(),
        shutdown_rx,
        runtime,
        Arc::new(tokio::sync::Notify::new()),
    );

    let tenant = format!("tenant-{}", Uuid::new_v4());
    let reactor = "rt_predicate_e2e".to_string();
    let workflow = "wf_predicate_e2e".to_string();

    // Subscribe with a predicate that matches only the second firing.
    dal.reactor_subscriptions()
        .subscribe(&reactor, &workflow, &tenant, Some("payload.value > 100"))
        .await
        .expect("subscribe with predicate");

    // Insert two firings with different `value` payloads.
    //
    // Truncate to microseconds: postgres TIMESTAMP stores μs precision,
    // so any sub-μs nanos in the source timestamp are lost on roundtrip.
    // The watermark we read back later will be μs-aligned; align the
    // inputs the same way so the >= comparison at the end of the test
    // is well-defined on both backends.
    let ts_first = aligned_now();
    dal.reactor_subscriptions()
        .insert_firing(
            &reactor,
            &tenant,
            Some(build_firing_payload(
                "value",
                serde_json::Value::Number(50.into()),
            )),
            ts_first,
        )
        .await
        .expect("insert firing 1");

    // Ensure the second firing has a strictly later timestamp so
    // poll_unconsumed sees both in deterministic order. +1ms keeps the
    // result μs-aligned (ts_first is already μs-aligned, and 1ms = 1000μs).
    let ts_second = UniversalTimestamp(ts_first.0 + chrono::Duration::milliseconds(1));
    dal.reactor_subscriptions()
        .insert_firing(
            &reactor,
            &tenant,
            Some(build_firing_payload(
                "value",
                serde_json::Value::Number(150.into()),
            )),
            ts_second,
        )
        .await
        .expect("insert firing 2");

    // Drive the scheduler. One pass over enabled subscriptions = one
    // batch of poll_unconsumed for this subscription, returning both
    // firings in `fired_at` order.
    scheduler
        .poll_reactor_subscriptions_once()
        .await
        .expect("scheduler tick");

    // Exactly one workflow dispatch — the matching firing.
    let calls = executor.snapshot();
    assert_eq!(
        calls.len(),
        1,
        "exactly one dispatch expected (the matching firing), got {}: {:?}",
        calls.len(),
        calls
    );
    assert_eq!(calls[0].0, workflow, "wrong workflow name dispatched");
    assert_eq!(
        calls[0].1,
        Some(serde_json::Value::Number(150.into())),
        "the matching firing's payload should be threaded into the context"
    );

    // Watermark must have advanced past BOTH firings — the filtered
    // firing was still observed.
    let subs = dal
        .reactor_subscriptions()
        .list_subscriptions(&tenant)
        .await
        .expect("list subscriptions");
    let sub = subs
        .iter()
        .find(|s| s.reactor_name == reactor && s.workflow_name == workflow)
        .expect("subscription row exists");
    let watermark = sub
        .last_seen_fired_at
        .as_ref()
        .expect("watermark should be set after poll");
    assert!(
        watermark.0 >= ts_second.0,
        "watermark {:?} did not advance past the second firing {:?} \
         (filtered-skip should still advance)",
        watermark.0,
        ts_second.0,
    );

    // CLOACI-T-0922 — the distinction that used to be missing: a
    // predicate that returns FALSE is a working predicate. It must not
    // leave any error/degraded residue behind.
    assert_eq!(
        sub.predicate_error_count, 0,
        "a false predicate is not an error and must not count against the retry budget"
    );
    assert!(
        sub.predicate_degraded.is_false(),
        "a false predicate must not mark the subscription degraded"
    );
    assert!(
        sub.last_predicate_error.is_none(),
        "a false predicate must not record an error, got: {:?}",
        sub.last_predicate_error
    );

    runner.shutdown().await.unwrap();
}

/// CLOACI-T-0922 — an *erroring* predicate must NOT advance the watermark.
///
/// This is the regression that motivated the ticket: `Err(_)` used to take
/// the same path as `Ok(false)` (skip + advance), which silently and
/// permanently destroyed the firing. Fail-closed means "don't dispatch";
/// it never meant "throw the event away".
///
/// Asserts, across two consecutive polls of the same firing:
///   - no dispatch (fail-closed still holds)
///   - the watermark is still unset, so the firing is re-polled
///   - the consecutive-error count climbs 1 -> 2 (durably, on the row)
///   - `cloacina_reactor_predicate_errors_total` increments per error
#[tokio::test]
#[serial]
async fn test_predicate_error_holds_watermark_and_counts() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database = fixture.get_database();
    let dal = fixture.get_dal();

    let runner = DefaultRunner::with_config(
        &fixture.get_database_url(),
        DefaultRunnerConfig::builder()
            .enable_cron_scheduling(false)
            .build()
            .unwrap(),
    )
    .await
    .expect("runner");

    let executor = Arc::new(RecordingExecutor::default());
    executor.set_inner(runner.clone());

    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    let scheduler = Scheduler::new(
        Arc::new(cloacina::dal::DAL::new(database)),
        executor.clone(),
        test_scheduler_config(),
        shutdown_rx,
        Arc::new(Runtime::empty()),
        Arc::new(tokio::sync::Notify::new()),
    );

    let tenant = format!("tenant-{}", Uuid::new_v4());
    let reactor = "rt_predicate_err".to_string();
    let workflow = "wf_predicate_err".to_string();

    // `payload.value` on a firing that carries no `value` key is a CEL
    // *evaluation* error (no such key) — it compiles fine and passes the
    // subscribe-time lint (it only references `payload`), which is exactly
    // the payload-shape-drift scenario the ticket describes.
    let sub_id = dal
        .reactor_subscriptions()
        .subscribe(&reactor, &workflow, &tenant, Some("payload.value > 100"))
        .await
        .expect("subscribe with predicate");

    let ts = aligned_now();
    dal.reactor_subscriptions()
        .insert_firing(
            &reactor,
            &tenant,
            Some(build_firing_payload(
                "other",
                serde_json::Value::Number(1.into()),
            )),
            ts,
        )
        .await
        .expect("insert firing");

    // Reset the metric delta window before the poll we measure.
    let _ = counter_delta("cloacina_reactor_predicate_errors_total");

    scheduler
        .poll_reactor_subscriptions_once()
        .await
        .expect("first tick");

    assert!(
        executor.snapshot().is_empty(),
        "a broken predicate must never dispatch (fail-closed), got: {:?}",
        executor.snapshot()
    );

    let sub = dal
        .reactor_subscriptions()
        .get_subscription(sub_id)
        .await
        .expect("get subscription")
        .expect("subscription row exists");
    assert!(
        sub.last_seen_fired_at.is_none(),
        "watermark MUST be held on a predicate error so the firing is retried; \
         it advanced to {:?} instead",
        sub.last_seen_fired_at
    );
    assert_eq!(
        sub.predicate_error_count, 1,
        "first failure should record one attempt"
    );
    assert_eq!(
        sub.predicate_error_firing_id.map(|id| id.0),
        Some(
            dal.reactor_subscriptions()
                .poll_unconsumed(&tenant, &reactor, None, 10)
                .await
                .expect("poll firings")[0]
                .id
                .0
        ),
        "the attempt count must be attributed to the failing firing"
    );
    assert!(
        sub.last_predicate_error.is_some(),
        "the error text must be recorded durably"
    );
    assert!(
        sub.predicate_degraded.is_false(),
        "one failure is far short of the bound; not degraded yet"
    );

    // Second tick: the SAME firing must still be head-of-line.
    scheduler
        .poll_reactor_subscriptions_once()
        .await
        .expect("second tick");

    let sub = dal
        .reactor_subscriptions()
        .get_subscription(sub_id)
        .await
        .expect("get subscription")
        .expect("subscription row exists");
    assert!(
        sub.last_seen_fired_at.is_none(),
        "watermark still held on the second failure"
    );
    assert_eq!(
        sub.predicate_error_count, 2,
        "consecutive failures on the same firing must accumulate"
    );

    assert_eq!(
        counter_delta("cloacina_reactor_predicate_errors_total"),
        2,
        "every predicate evaluation error must increment the counter"
    );

    runner.shutdown().await.unwrap();
}

/// CLOACI-T-0922 — the bound: holding the watermark forever would wedge
/// the subscription, so after `MAX_CONSECUTIVE_PREDICATE_ERRORS` attempts
/// the poison firing is dead-lettered (recorded on the row + counted) and
/// the watermark force-advances.
///
/// Then the recovery half: a later firing that evaluates cleanly clears
/// the retry budget and the degraded flag, while `last_predicate_error`
/// survives as the forensic trail of what was dropped.
#[tokio::test]
#[serial]
async fn test_predicate_error_dead_letters_at_the_bound_then_recovers() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database = fixture.get_database();
    let dal = fixture.get_dal();

    let runner = DefaultRunner::with_config(
        &fixture.get_database_url(),
        DefaultRunnerConfig::builder()
            .enable_cron_scheduling(false)
            .build()
            .unwrap(),
    )
    .await
    .expect("runner");

    let executor = Arc::new(RecordingExecutor::default());
    executor.set_inner(runner.clone());

    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    let scheduler = Scheduler::new(
        Arc::new(cloacina::dal::DAL::new(database)),
        executor.clone(),
        test_scheduler_config(),
        shutdown_rx,
        Arc::new(Runtime::empty()),
        Arc::new(tokio::sync::Notify::new()),
    );

    let tenant = format!("tenant-{}", Uuid::new_v4());
    let reactor = "rt_predicate_deadletter".to_string();
    let workflow = "wf_predicate_deadletter".to_string();

    let sub_id = dal
        .reactor_subscriptions()
        .subscribe(&reactor, &workflow, &tenant, Some("payload.value > 100"))
        .await
        .expect("subscribe with predicate");

    // The poison firing: no `value` key, so the predicate errors forever.
    let ts_poison = aligned_now();
    dal.reactor_subscriptions()
        .insert_firing(
            &reactor,
            &tenant,
            Some(build_firing_payload(
                "other",
                serde_json::Value::Number(1.into()),
            )),
            ts_poison,
        )
        .await
        .expect("insert poison firing");

    let _ = counter_delta("cloacina_reactor_predicate_dead_letters_total");

    // Poll up to (but not including) the bound: still held.
    for tick in 1..MAX_CONSECUTIVE_PREDICATE_ERRORS {
        scheduler
            .poll_reactor_subscriptions_once()
            .await
            .expect("tick");
        let sub = dal
            .reactor_subscriptions()
            .get_subscription(sub_id)
            .await
            .expect("get subscription")
            .expect("row");
        assert!(
            sub.last_seen_fired_at.is_none(),
            "watermark must still be held at attempt {} of {}",
            tick,
            MAX_CONSECUTIVE_PREDICATE_ERRORS
        );
        assert!(sub.predicate_degraded.is_false());
    }

    // The tick that hits the bound dead-letters and force-advances.
    scheduler
        .poll_reactor_subscriptions_once()
        .await
        .expect("bound tick");

    let sub = dal
        .reactor_subscriptions()
        .get_subscription(sub_id)
        .await
        .expect("get subscription")
        .expect("row");
    let watermark = sub
        .last_seen_fired_at
        .expect("watermark must advance past a dead-lettered firing so the subscription resumes");
    assert!(
        watermark.0 >= ts_poison.0,
        "watermark {:?} should have advanced past the poison firing {:?}",
        watermark.0,
        ts_poison.0
    );
    assert!(
        sub.predicate_degraded.is_true(),
        "dead-lettering a firing must mark the subscription degraded"
    );
    assert!(
        sub.last_predicate_error.is_some(),
        "the dead-lettered firing's error must be recorded durably"
    );
    assert!(
        sub.last_predicate_error_at.is_some(),
        "the dead-letter must be timestamped"
    );
    assert_eq!(
        sub.predicate_error_count, 0,
        "the next firing gets a fresh retry budget"
    );
    assert!(
        executor.snapshot().is_empty(),
        "a dead-lettered firing is never dispatched"
    );
    assert_eq!(
        counter_delta("cloacina_reactor_predicate_dead_letters_total"),
        1,
        "the dead-letter must be counted exactly once"
    );

    // ── Recovery: a well-shaped firing evaluates cleanly again. ──
    let ts_good = UniversalTimestamp(ts_poison.0 + chrono::Duration::milliseconds(1));
    dal.reactor_subscriptions()
        .insert_firing(
            &reactor,
            &tenant,
            Some(build_firing_payload(
                "value",
                serde_json::Value::Number(150.into()),
            )),
            ts_good,
        )
        .await
        .expect("insert good firing");

    scheduler
        .poll_reactor_subscriptions_once()
        .await
        .expect("recovery tick");

    let calls = executor.snapshot();
    assert_eq!(
        calls.len(),
        1,
        "the subscription must not be wedged — the next good firing dispatches, got: {:?}",
        calls
    );
    assert_eq!(calls[0].0, workflow);

    let sub = dal
        .reactor_subscriptions()
        .get_subscription(sub_id)
        .await
        .expect("get subscription")
        .expect("row");
    assert!(
        sub.predicate_degraded.is_false(),
        "a clean evaluation clears the degraded marker"
    );
    assert_eq!(sub.predicate_error_count, 0);
    assert!(
        sub.last_predicate_error.is_some(),
        "recovery must NOT erase the record of the firing that was dropped"
    );
    assert!(
        sub.last_seen_fired_at.expect("watermark").0 >= ts_good.0,
        "watermark advances past the dispatched firing"
    );

    runner.shutdown().await.unwrap();
}

/// CLOACI-T-0922 item 4 — the subscribe-time unbound-variable lint is
/// reachable through the real DAL entry point, not just as a pure
/// function. `tenant` is bound; `tennant` is a typo that used to compile
/// fine and then silently never fire.
#[tokio::test]
#[serial]
async fn test_subscribe_rejects_predicate_with_unbound_variable() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let tenant = format!("tenant-{}", Uuid::new_v4());

    dal.reactor_subscriptions()
        .subscribe(
            "rt_lint",
            "wf_lint",
            &tenant,
            Some("payload.x > 1 && tenant == 'acme'"),
        )
        .await
        .expect("a predicate over the bound variables must be accepted");

    let err = dal
        .reactor_subscriptions()
        .subscribe(
            "rt_lint_bad",
            "wf_lint_bad",
            &tenant,
            Some("payload.x > 1 && tennant == 'acme'"),
        )
        .await
        .expect_err("a predicate referencing an unbound variable must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("tennant"),
        "the rejection should name the offending variable, got: {}",
        msg
    );

    // ...and the bad subscription must not have been written.
    let subs = dal
        .reactor_subscriptions()
        .list_subscriptions(&tenant)
        .await
        .expect("list");
    assert!(
        subs.iter().all(|s| s.reactor_name != "rt_lint_bad"),
        "a rejected predicate must never land in the DB"
    );
}
