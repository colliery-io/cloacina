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

//! Concurrency tests for task claiming operations.
//!
//! These tests verify that the task claiming mechanism prevents race conditions
//! where multiple workers might claim the same task simultaneously.
//!
//! Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.

use crate::fixtures::get_all_fixtures;
use cloacina::dal::unified::task_execution::{HeartbeatResult, RunnerClaimResult};
use cloacina::dal::DAL;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::models::workflow_execution::NewWorkflowExecution;
use serde_json::json;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;

/// Disjoint, complete claiming under concurrent schedulers
/// (CLOACI-T-0818 / ADR CLOACI-A-0008).
///
/// This is the AUTHORITATIVE validation of the multi-replica scheduler property:
/// when two or more `cloacina-server` replicas each run the per-tenant task
/// scheduler and all poll the SAME set of ready outbox rows, every ready task is
/// dispatched to EXACTLY ONE scheduler — no double-dispatch and no lost work. The
/// scheduler is NOT leader-gated (every replica runs it unconditionally), so the
/// `task_outbox` claim is the only thing standing between two replicas and a
/// double-dispatch. The `k8s-leader` e2e's assertion 4 (`--claiming`) is the
/// full-stack analogue of this property, but it is opt-in / best-effort because a
/// helm-only server ships no compiler (uploaded packages never build and so never
/// execute); this DAL-level test proves the same invariant deterministically and
/// is what the e2e docstring points at as the real proof.
///
/// Setup mirrors N replicas: `NUM_WORKERS` concurrent claimers, each on its OWN
/// pooled connection (a fresh `DAL` over a cloned `Database`), all released from a
/// `Barrier` so they genuinely race for the same outbox rows. The claim path under
/// test is `claim_ready_task` -> Postgres `DELETE ... FOR UPDATE SKIP LOCKED`
/// (and, on SQLite, the `BEGIN IMMEDIATE` serialization). The pool must hold more
/// than one connection for the race to be real — see the pool-size note in
/// `claiming.rs` (CLOACI-T-0622, where a pool of 1 hid a TOCTOU bug); the test
/// fixtures use pool size 10 (postgres) / 5 (sqlite).
///
/// Workers drain to completion (claim until the shared counter shows all N have
/// been claimed, bounded by a safety deadline), then we assert with ZERO slack:
/// 1. DISJOINT — no task id claimed by more than one worker (no double-dispatch).
/// 2. COMPLETE — the union of claimed ids == all N seeded tasks (no lost work).
/// 3. SOUND    — every claimed id belongs to the seeded set (set equality).
#[tokio::test]
async fn test_concurrent_task_claiming_no_duplicates() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!(
            "Running test_concurrent_task_claiming_no_duplicates on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        // Create a test workflow execution
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "concurrent-claim-test".to_string(),
                workflow_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .expect("Failed to create workflow execution");

        // Create multiple tasks and mark them ready (which populates the outbox)
        const NUM_TASKS: usize = 50;
        let mut created_task_ids = Vec::new();

        for i in 0..NUM_TASKS {
            let task = dal
                .task_execution()
                .create(NewTaskExecution {
                    workflow_execution_id: wf_exec.id,
                    task_name: format!("concurrent-task-{}", i),
                    status: "NotStarted".to_string(),
                    attempt: 1,
                    max_attempts: 3,
                    trigger_rules: json!({"type": "Always"}).to_string(),
                    task_configuration: json!({}).to_string(),
                })
                .await
                .expect("Failed to create task");

            // Mark as ready - this adds to the outbox
            dal.task_execution()
                .mark_ready(task.id)
                .await
                .expect("Failed to mark task ready");

            created_task_ids.push(task.id);
        }

        // Verify outbox has entries before workers start
        let outbox_count = dal
            .task_outbox()
            .count_pending()
            .await
            .expect("Failed to count outbox");
        assert_eq!(
            outbox_count as usize, NUM_TASKS,
            "[{}] Outbox should have {} entries, got {}",
            backend, NUM_TASKS, outbox_count
        );

        // Release the fixture lock before spawning concurrent tasks
        drop(guard);

        // Spawn N concurrent schedulers (one per simulated server replica). Each
        // gets its OWN pooled connection via a fresh DAL over a cloned Database,
        // and all are released simultaneously by the Barrier so they genuinely
        // race for the same ready outbox rows.
        const NUM_WORKERS: usize = 4;
        let barrier = Arc::new(Barrier::new(NUM_WORKERS));
        // Shared count of tasks claimed across all workers; drives drain-to-
        // completion termination (vs. a fixed iteration budget that could leave
        // rows unclaimed and hide lost work).
        let claimed_total = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for worker_id in 0..NUM_WORKERS {
            let db_clone = database.clone();
            let barrier_clone = barrier.clone();
            let claimed_total = claimed_total.clone();

            let handle = tokio::spawn(async move {
                let dal = DAL::new(db_clone);

                // Wait for all workers to be ready before claiming
                barrier_clone.wait().await;

                // Drain to completion: keep claiming until every seeded task has
                // been claimed (shared counter), bounded by a safety deadline. A
                // genuinely lost/stuck task simply fails the COMPLETE assertion
                // below rather than hanging the test forever.
                let deadline = Instant::now() + Duration::from_secs(30);
                let mut claimed = Vec::new();
                loop {
                    if claimed_total.load(Ordering::SeqCst) >= NUM_TASKS {
                        break;
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                    match dal.task_execution().claim_ready_task(2).await {
                        Ok(results) if !results.is_empty() => {
                            claimed_total.fetch_add(results.len(), Ordering::SeqCst);
                            for result in results {
                                claimed.push((worker_id, result.id));
                            }
                        }
                        Ok(_) => {
                            // Empty: either truly drained, or peers currently hold
                            // the remaining rows' locks (SKIP LOCKED returns them to
                            // nobody). Back off and re-check the shared counter.
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }
                        Err(e) => {
                            // Transient contention (e.g. sqlite busy). Back off + retry.
                            tracing::debug!("Worker {} claim error: {:?}", worker_id, e);
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }
                    }
                }
                claimed
            });

            handles.push(handle);
        }

        // Collect all claimed task IDs from all workers
        let mut all_claimed: Vec<(usize, UniversalUuid)> = Vec::new();
        for handle in handles {
            let claimed = handle.await.expect("Worker task panicked");
            all_claimed.extend(claimed);
        }

        let claimed_ids: Vec<UniversalUuid> = all_claimed.iter().map(|(_, id)| *id).collect();
        let unique_ids: HashSet<UniversalUuid> = claimed_ids.iter().copied().collect();
        let created_set: HashSet<UniversalUuid> = created_task_ids.iter().copied().collect();

        // 1. DISJOINT — no task id claimed by more than one worker (no double-dispatch).
        assert_eq!(
            claimed_ids.len(),
            unique_ids.len(),
            "[{}] DOUBLE-DISPATCH: a task was claimed by more than one scheduler. \
             Total claims: {}, unique tasks: {}. Concurrent claimers share ready \
             outbox rows but the SKIP LOCKED + claimed_by claim must hand each row \
             to exactly one.",
            backend,
            claimed_ids.len(),
            unique_ids.len()
        );

        // 2. COMPLETE — every seeded task was claimed exactly once (no lost work).
        assert_eq!(
            unique_ids.len(),
            NUM_TASKS,
            "[{}] LOST WORK: claimed {} distinct tasks but seeded {}. Missing: {:?}",
            backend,
            unique_ids.len(),
            NUM_TASKS,
            created_set.difference(&unique_ids).collect::<Vec<_>>()
        );

        // 3. SOUND — the claimed set is exactly the seeded set (nothing extra).
        assert_eq!(
            unique_ids, created_set,
            "[{}] claimed set != seeded set (unexpected or missing task ids)",
            backend
        );

        tracing::info!(
            "[{}] Disjoint-claiming test passed: {} concurrent schedulers claimed all {} \
             tasks exactly once with no double-dispatch (CLOACI-T-0818 / A-0008)",
            backend,
            NUM_WORKERS,
            unique_ids.len()
        );
    }
}

/// Test that claimed tasks have their status properly updated to Running.
#[tokio::test]
async fn test_claimed_tasks_marked_running() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!("Running test_claimed_tasks_marked_running on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        // Create a test workflow execution
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "claim-status-test".to_string(),
                workflow_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .expect("Failed to create workflow execution");

        // Create a task and mark it ready (which populates the outbox)
        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf_exec.id,
                task_name: "status-test-task".to_string(),
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");

        // Mark as ready - this adds to the outbox
        dal.task_execution()
            .mark_ready(task.id)
            .await
            .expect("Failed to mark task ready");

        // Claim the task
        let claimed = dal
            .task_execution()
            .claim_ready_task(1)
            .await
            .expect("Failed to claim task");

        assert_eq!(
            claimed.len(),
            1,
            "[{}] Should claim exactly one task",
            backend
        );
        assert_eq!(
            claimed[0].id, task.id,
            "[{}] Should claim our task",
            backend
        );

        // Verify the task status is now Running
        let updated_task = dal
            .task_execution()
            .get_by_id(task.id)
            .await
            .expect("Failed to get task");

        assert_eq!(
            updated_task.status, "Running",
            "[{}] Claimed task should have status 'Running'",
            backend
        );
        assert!(
            updated_task.started_at.is_some(),
            "[{}] Claimed task should have started_at timestamp",
            backend
        );

        tracing::info!("test_claimed_tasks_marked_running passed on {}", backend);
    }
}

/// Test that already-running tasks cannot be claimed again.
#[tokio::test]
async fn test_running_tasks_not_claimable() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!("Running test_running_tasks_not_claimable on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        // Create a test workflow execution
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "running-not-claimable-test".to_string(),
                workflow_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .expect("Failed to create workflow execution");

        // Create a task that's already running
        let _running_task = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf_exec.id,
                task_name: "already-running-task".to_string(),
                status: "Running".to_string(), // Already running
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");

        // Try to claim - should get nothing since the task is already running
        let claimed = dal
            .task_execution()
            .claim_ready_task(10)
            .await
            .expect("Failed to attempt claim");

        assert!(
            claimed.is_empty(),
            "[{}] Should not claim any tasks when all are already running",
            backend
        );

        tracing::info!("test_running_tasks_not_claimable passed on {}", backend);
    }
}

// ============================================================================
// Runner-level claiming tests (horizontal scaling)
// ============================================================================

/// Helper: create a workflow execution and a Running task for runner claiming tests.
async fn create_running_task(dal: &DAL) -> (UniversalUuid, UniversalUuid) {
    let wf_exec = dal
        .workflow_execution()
        .create(NewWorkflowExecution {
            workflow_name: "runner-claim-test".to_string(),
            workflow_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("create workflow execution");

    let task = dal
        .task_execution()
        .create(NewTaskExecution {
            workflow_execution_id: wf_exec.id,
            task_name: "claimable-task".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .expect("create task");

    (wf_exec.id, task.id)
}

/// Double-claim prevention: two runners claim the same task — exactly one wins.
#[tokio::test]
async fn test_runner_double_claim_prevention() {
    for (backend, fixture) in get_all_fixtures().await {
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let (_exec_id, task_id) = create_running_task(&dal).await;

        let runner_a = UniversalUuid::new_v4();
        let runner_b = UniversalUuid::new_v4();

        // Runner A claims first
        let result_a = dal
            .task_execution()
            .claim_for_runner(task_id, runner_a)
            .await
            .expect("claim_for_runner A");
        assert_eq!(
            result_a,
            RunnerClaimResult::Claimed,
            "[{}] Runner A should claim",
            backend
        );

        // Runner B tries to claim the same task
        let result_b = dal
            .task_execution()
            .claim_for_runner(task_id, runner_b)
            .await
            .expect("claim_for_runner B");
        assert_eq!(
            result_b,
            RunnerClaimResult::AlreadyClaimed,
            "[{}] Runner B should get AlreadyClaimed",
            backend
        );

        // Verify the task is claimed by runner A
        let task = dal
            .task_execution()
            .get_by_id(task_id)
            .await
            .expect("get task");
        assert_eq!(
            task.claimed_by,
            Some(runner_a),
            "[{}] claimed_by should be runner A",
            backend
        );
        assert!(
            task.heartbeat_at.is_some(),
            "[{}] heartbeat_at should be set",
            backend
        );

        tracing::info!("[{}] test_runner_double_claim_prevention passed", backend);
    }
}

/// Heartbeat succeeds when runner owns the claim, fails when claim is lost.
#[tokio::test]
async fn test_heartbeat_ownership_guard() {
    for (backend, fixture) in get_all_fixtures().await {
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let (_exec_id, task_id) = create_running_task(&dal).await;

        let runner_a = UniversalUuid::new_v4();
        let runner_b = UniversalUuid::new_v4();

        // Runner A claims
        dal.task_execution()
            .claim_for_runner(task_id, runner_a)
            .await
            .expect("claim");

        // Runner A heartbeats — should succeed
        let hb = dal
            .task_execution()
            .heartbeat(task_id, runner_a)
            .await
            .expect("heartbeat A");
        assert_eq!(
            hb,
            HeartbeatResult::Ok,
            "[{}] Runner A heartbeat should succeed",
            backend
        );

        // Runner B tries to heartbeat — should fail (not the owner)
        let hb_b = dal
            .task_execution()
            .heartbeat(task_id, runner_b)
            .await
            .expect("heartbeat B");
        assert_eq!(
            hb_b,
            HeartbeatResult::ClaimLost,
            "[{}] Runner B heartbeat should return ClaimLost",
            backend
        );

        tracing::info!("[{}] test_heartbeat_ownership_guard passed", backend);
    }
}

/// Release claim clears claimed_by and heartbeat_at.
#[tokio::test]
async fn test_release_claim_clears_fields() {
    for (backend, fixture) in get_all_fixtures().await {
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let (_exec_id, task_id) = create_running_task(&dal).await;
        let runner = UniversalUuid::new_v4();

        // Claim
        dal.task_execution()
            .claim_for_runner(task_id, runner)
            .await
            .expect("claim");

        // Verify claimed
        let task = dal.task_execution().get_by_id(task_id).await.expect("get");
        assert!(task.claimed_by.is_some(), "[{}] should be claimed", backend);

        // Release
        dal.task_execution()
            .release_runner_claim(task_id)
            .await
            .expect("release");

        // Verify released
        let task = dal.task_execution().get_by_id(task_id).await.expect("get");
        assert!(
            task.claimed_by.is_none(),
            "[{}] claimed_by should be None after release",
            backend
        );
        assert!(
            task.heartbeat_at.is_none(),
            "[{}] heartbeat_at should be None after release",
            backend
        );

        tracing::info!("[{}] test_release_claim_clears_fields passed", backend);
    }
}

/// After release, another runner can claim the task.
#[tokio::test]
async fn test_reclaim_after_release() {
    for (backend, fixture) in get_all_fixtures().await {
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let (_exec_id, task_id) = create_running_task(&dal).await;
        let runner_a = UniversalUuid::new_v4();
        let runner_b = UniversalUuid::new_v4();

        // Runner A claims and releases
        dal.task_execution()
            .claim_for_runner(task_id, runner_a)
            .await
            .expect("claim A");
        dal.task_execution()
            .release_runner_claim(task_id)
            .await
            .expect("release A");

        // Runner B can now claim
        let result = dal
            .task_execution()
            .claim_for_runner(task_id, runner_b)
            .await
            .expect("claim B");
        assert_eq!(
            result,
            RunnerClaimResult::Claimed,
            "[{}] Runner B should claim after release",
            backend
        );

        // Runner A's heartbeat should fail
        let hb = dal
            .task_execution()
            .heartbeat(task_id, runner_a)
            .await
            .expect("hb A");
        assert_eq!(
            hb,
            HeartbeatResult::ClaimLost,
            "[{}] Runner A heartbeat should fail after reclaim",
            backend
        );

        tracing::info!("[{}] test_reclaim_after_release passed", backend);
    }
}

/// Find stale claims returns tasks with old heartbeats.
#[tokio::test]
async fn test_find_stale_claims() {
    for (backend, fixture) in get_all_fixtures().await {
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let (_exec_id, task_id) = create_running_task(&dal).await;
        let runner = UniversalUuid::new_v4();

        // Claim the task
        dal.task_execution()
            .claim_for_runner(task_id, runner)
            .await
            .expect("claim");

        // With a very short threshold (0s), the claim should immediately appear stale
        let stale = dal
            .task_execution()
            .find_stale_claims(std::time::Duration::from_secs(0))
            .await
            .expect("find stale");

        assert!(
            stale.iter().any(|s| s.task_id == task_id),
            "[{}] Task should appear in stale claims with 0s threshold",
            backend
        );

        // With a very long threshold, nothing should be stale
        let not_stale = dal
            .task_execution()
            .find_stale_claims(std::time::Duration::from_secs(9999))
            .await
            .expect("find not stale");

        assert!(
            !not_stale.iter().any(|s| s.task_id == task_id),
            "[{}] Task should NOT appear in stale claims with 9999s threshold",
            backend
        );

        tracing::info!("[{}] test_find_stale_claims passed", backend);
    }
}
