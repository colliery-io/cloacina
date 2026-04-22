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

//! Integration tests for cooperative task cancellation on claim loss
//! (CLOACI-T-0487).
//!
//! Verifies both layers of the cancellation design:
//!
//! - **Layer 1 (universal)**: the executor wraps `task.execute()` in a
//!   `tokio::select!` racing against a watch channel the heartbeat loop
//!   fires on `ClaimLost`. A task that ignores the signal entirely is still
//!   dropped mid-execution and never completes naturally.
//! - **Layer 2 (opt-in)**: tasks that accept a `TaskHandle` can observe the
//!   signal via `handle.cancelled().await` and shut down gracefully.
//!
//! The setup uses `DefaultRunner` with a short heartbeat interval and
//! forcibly rewrites `claimed_by` on the `task_executions` row to simulate
//! a competing runner stealing the claim.

use cloacina::database::schema::unified::task_executions;
use cloacina::database::universal_types::{UniversalTimestamp, UniversalUuid};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::*;
use diesel::prelude::*;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::fixtures::{get_or_init_fixture, poll_until};

// ---------------------------------------------------------------------------
// Test fixtures — `static` flags observe the task's control flow
// ---------------------------------------------------------------------------

/// Set inside the Layer 1 task body if the sleep ran to completion. If the
/// executor cancels the future mid-sleep, this stays false.
static LAYER_1_COMPLETED_NATURALLY: AtomicBool = AtomicBool::new(false);

/// Set inside the Layer 2 task body when it observes cancellation via the
/// `TaskHandle`. Proves Layer 2 fired before Layer 1 would have dropped
/// the future.
static LAYER_2_OBSERVED_CANCEL: AtomicBool = AtomicBool::new(false);

// The task IDs below are intentionally namespaced with `t0487_` so they
// don't collide with any other scenario's fixtures if the test harness
// reuses a registry across modules.

#[task(id = "t0487_layer1_sleep", dependencies = [])]
async fn layer1_sleep_task(_context: &mut Context<Value>) -> Result<(), TaskError> {
    // Long enough that Layer 1 cancellation must fire for the assertion
    // below to pass. If the future is dropped mid-sleep, the store never
    // executes.
    tokio::time::sleep(Duration::from_secs(30)).await;
    LAYER_1_COMPLETED_NATURALLY.store(true, Ordering::SeqCst);
    Ok(())
}

#[task(id = "t0487_layer2_cooperative", dependencies = [])]
async fn layer2_cooperative_task(
    _context: &mut Context<Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(30)) => {}
        _ = handle.cancelled() => {
            LAYER_2_OBSERVED_CANCEL.store(true, Ordering::SeqCst);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Force-reassign `claimed_by` on the task_executions row to simulate a
/// competing runner stealing the claim. Bypasses the DAL's "claim only if
/// NULL" guard — this is the whole point of the test.
async fn steal_claim(database: &cloacina::database::Database, task_execution_id: UniversalUuid) {
    let thief = UniversalUuid::new_v4();
    let now = UniversalTimestamp::now();

    cloacina::dispatch_backend!(
        database.backend(),
        {
            let conn = database
                .get_postgres_connection()
                .await
                .expect("postgres connection");
            let rows: usize = conn
                .interact(move |conn| {
                    diesel::update(task_executions::table.find(task_execution_id))
                        .set((
                            task_executions::claimed_by.eq(Some(thief)),
                            task_executions::heartbeat_at.eq(Some(now)),
                            task_executions::updated_at.eq(now),
                        ))
                        .execute(conn)
                })
                .await
                .expect("interact")
                .expect("update");
            assert_eq!(rows, 1, "claim steal must hit exactly one row");
        },
        {
            let conn = database
                .get_sqlite_connection()
                .await
                .expect("sqlite connection");
            let rows: usize = conn
                .interact(move |conn| {
                    diesel::update(task_executions::table.find(task_execution_id))
                        .set((
                            task_executions::claimed_by.eq(Some(thief)),
                            task_executions::heartbeat_at.eq(Some(now)),
                            task_executions::updated_at.eq(now),
                        ))
                        .execute(conn)
                })
                .await
                .expect("interact")
                .expect("update");
            assert_eq!(rows, 1, "claim steal must hit exactly one row");
        }
    );
}

/// Look up the task_execution row id for a given workflow execution + task
/// short name. Returns `None` until the scheduler has inserted the row.
async fn find_claimed_task_id(
    database: &cloacina::database::Database,
    workflow_execution_id: UniversalUuid,
    task_short_name: &str,
) -> Option<UniversalUuid> {
    let needle = format!("%::{}", task_short_name);

    cloacina::dispatch_backend!(
        database.backend(),
        {
            let conn = database.get_postgres_connection().await.ok()?;
            conn.interact(move |conn| {
                task_executions::table
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                    .filter(task_executions::task_name.like(needle))
                    .filter(task_executions::claimed_by.is_not_null())
                    .select(task_executions::id)
                    .first::<UniversalUuid>(conn)
                    .optional()
            })
            .await
            .ok()
            .and_then(|r| r.ok())
            .flatten()
        },
        {
            let conn = database.get_sqlite_connection().await.ok()?;
            conn.interact(move |conn| {
                task_executions::table
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                    .filter(task_executions::task_name.like(needle))
                    .filter(task_executions::claimed_by.is_not_null())
                    .select(task_executions::id)
                    .first::<UniversalUuid>(conn)
                    .optional()
            })
            .await
            .ok()
            .and_then(|r| r.ok())
            .flatten()
        }
    )
}

async fn wait_for_claim(
    database: &cloacina::database::Database,
    workflow_execution_id: UniversalUuid,
    task_short_name: &str,
) -> UniversalUuid {
    let owned_name = task_short_name.to_string();
    let db = database.clone();
    poll_until(
        Duration::from_secs(10),
        Duration::from_millis(50),
        "task should be claimed by the runner",
        move || {
            let db = db.clone();
            let owned_name = owned_name.clone();
            async move {
                find_claimed_task_id(&db, workflow_execution_id, &owned_name)
                    .await
                    .is_some()
            }
        },
    )
    .await;

    find_claimed_task_id(database, workflow_execution_id, task_short_name)
        .await
        .expect("claim was observed above — must still exist")
}

fn short_heartbeat_config() -> DefaultRunnerConfig {
    DefaultRunnerConfig::builder()
        .heartbeat_interval(Duration::from_millis(100))
        .build()
        .expect("valid config")
}

// ---------------------------------------------------------------------------
// Layer 1 — universal cancellation via tokio::select!
// ---------------------------------------------------------------------------

#[tokio::test]
async fn layer_1_heartbeat_cancellation_aborts_sleeping_task() {
    LAYER_1_COMPLETED_NATURALLY.store(false, Ordering::SeqCst);

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();
    let schema = fixture.get_schema();

    let runtime = cloacina::Runtime::empty();
    let workflow = Workflow::builder("t0487_layer_1_workflow")
        .description("Layer 1 cancellation — task ignores the signal")
        .add_task(Arc::new(layer1_sleep_task_task()))
        .unwrap()
        .build()
        .unwrap();
    let ns = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "t0487_layer1_sleep",
    );
    runtime.register_task(ns, || Arc::new(layer1_sleep_task_task()) as Arc<dyn Task>);
    runtime.register_workflow("t0487_layer_1_workflow".to_string(), {
        let w = workflow.clone();
        move || w.clone()
    });

    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .runtime(runtime)
        .with_config(short_heartbeat_config())
        .build()
        .await
        .unwrap();

    let execution = runner
        .execute_async("t0487_layer_1_workflow", Context::new())
        .await
        .unwrap();

    let task_execution_id = wait_for_claim(
        &database,
        UniversalUuid(execution.execution_id),
        "t0487_layer1_sleep",
    )
    .await;

    // Steal the claim — heartbeat should detect it on the next 100ms tick
    // and fire the cancellation watch channel.
    steal_claim(&database, task_execution_id).await;

    // Give the heartbeat loop several ticks plus slack for scheduler + DB
    // round-trips. We only need ~100ms for claim-loss detection; 2s is
    // generous.
    tokio::time::sleep(Duration::from_secs(2)).await;

    assert!(
        !LAYER_1_COMPLETED_NATURALLY.load(Ordering::SeqCst),
        "task future must be cancelled mid-sleep; it completed naturally, \
         which means the executor is not honoring the claim-loss signal"
    );

    runner.shutdown().await.ok();
}

// ---------------------------------------------------------------------------
// Layer 2 — cooperative observation via TaskHandle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn layer_2_cooperative_cancellation_via_task_handle() {
    LAYER_2_OBSERVED_CANCEL.store(false, Ordering::SeqCst);

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let database = fixture.get_database();
    let schema = fixture.get_schema();

    let runtime = cloacina::Runtime::empty();
    let workflow = Workflow::builder("t0487_layer_2_workflow")
        .description("Layer 2 cancellation — cooperative TaskHandle observer")
        .add_task(Arc::new(layer2_cooperative_task_task()))
        .unwrap()
        .build()
        .unwrap();
    let ns = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "t0487_layer2_cooperative",
    );
    runtime.register_task(ns, || {
        Arc::new(layer2_cooperative_task_task()) as Arc<dyn Task>
    });
    runtime.register_workflow("t0487_layer_2_workflow".to_string(), {
        let w = workflow.clone();
        move || w.clone()
    });

    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .runtime(runtime)
        .with_config(short_heartbeat_config())
        .build()
        .await
        .unwrap();

    let execution = runner
        .execute_async("t0487_layer_2_workflow", Context::new())
        .await
        .unwrap();

    let task_execution_id = wait_for_claim(
        &database,
        UniversalUuid(execution.execution_id),
        "t0487_layer2_cooperative",
    )
    .await;

    steal_claim(&database, task_execution_id).await;

    // Layer 2 must fire before Layer 1 would (both ride the same watch
    // channel; Layer 2 simply wins the select! inside the task body).
    poll_until(
        Duration::from_secs(5),
        Duration::from_millis(25),
        "cooperative task must observe cancellation via TaskHandle",
        || async { LAYER_2_OBSERVED_CANCEL.load(Ordering::SeqCst) },
    )
    .await;

    runner.shutdown().await.ok();
}
