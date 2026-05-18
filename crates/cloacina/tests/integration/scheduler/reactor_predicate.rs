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
use cloacina::context::Context;
use cloacina::cron_trigger_scheduler::{Scheduler, SchedulerConfig};
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::executor::{
    StatusCallback, WorkflowExecution, WorkflowExecutionError, WorkflowExecutionResult,
    WorkflowExecutor, WorkflowStatus,
};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::Runtime;
use serial_test::serial;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
        SchedulerConfig {
            // Fast tick + plenty of headroom; we drive the poll manually
            // via `poll_reactor_subscriptions_once`, but supply valid
            // values so a misconfigured field doesn't reject construction.
            cron_poll_interval: Duration::from_secs(30),
            max_catchup_executions: 0,
            max_acceptable_delay: Duration::from_secs(300),
            trigger_base_poll_interval: Duration::from_millis(50),
            trigger_poll_timeout: Duration::from_secs(5),
            reactor_poll_interval: Duration::from_millis(50),
            reactor_poll_batch_limit: 100,
            reactor_firings_prune_interval: Duration::from_secs(3600),
            reactor_firings_retention: Duration::from_secs(86400),
        },
        shutdown_rx,
        runtime,
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
    let ts_first = UniversalTimestamp::now();
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
    // poll_unconsumed sees both in deterministic order.
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

    runner.shutdown().await.unwrap();
}
