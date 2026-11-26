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

//! Concurrency tests for runner-level task claiming (horizontal scaling).
//!
//! These tests verify that the `claim_for_runner`/`heartbeat`/`release_runner_claim`
//! mechanism prevents race conditions where multiple runners might claim the
//! same task simultaneously.
//!
//! Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.

use crate::fixtures::get_all_fixtures;
use cloacina::dal::DAL;
use cloacina::dal::unified::task_execution::{HeartbeatResult, RunnerClaimResult};
use cloacina::database::universal_types::UniversalUuid;
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::models::workflow_execution::NewWorkflowExecution;
use serde_json::json;

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

/// CLOACINA-T-1012: a deferred dispatch keeps the attempt count and hides the
/// task from the ready query until its time.
#[tokio::test]
async fn test_defer_dispatch_keeps_attempt_and_delays_the_task() {
    use cloacina::database::universal_types::UniversalTimestamp;

    for (backend, fixture) in get_all_fixtures().await {
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let (_exec_id, task_id) = create_running_task(&dal).await;

        // A Running task is not changed.
        let later = UniversalTimestamp(chrono::Utc::now() + chrono::Duration::seconds(60));
        let changed = dal
            .task_execution()
            .defer_dispatch(task_id, later)
            .await
            .expect("defer_dispatch on a Running task");
        assert!(
            !changed,
            "[{}] a Running task must not be deferred",
            backend
        );

        dal.task_execution()
            .mark_ready(task_id)
            .await
            .expect("mark_ready");
        let ready = dal.task_execution().get_ready_for_retry().await.unwrap();
        assert!(
            ready.iter().any(|t| t.id == task_id),
            "[{}] a Ready task is in the ready query",
            backend
        );
        let waiting_since = dal
            .task_execution()
            .get_by_id(task_id)
            .await
            .unwrap()
            .waiting_since;
        assert!(
            waiting_since.is_some(),
            "[{}] mark_ready sets waiting_since",
            backend
        );

        // Deferred to the future: out of the ready query, attempt unchanged.
        let changed = dal
            .task_execution()
            .defer_dispatch(task_id, later)
            .await
            .expect("defer_dispatch");
        assert!(changed, "[{}] a Ready task is deferred", backend);
        let ready = dal.task_execution().get_ready_for_retry().await.unwrap();
        assert!(
            !ready.iter().any(|t| t.id == task_id),
            "[{}] a deferred task is not ready before its time",
            backend
        );
        let task = dal.task_execution().get_by_id(task_id).await.unwrap();
        assert_eq!(
            task.attempt, 1,
            "[{}] the attempt count is unchanged",
            backend
        );
        assert_eq!(task.status, "Ready", "[{}]", backend);

        // CLOACINA-T-1036: the deferral does not move waiting_since, and the
        // task is still counted as waiting, with its age.
        assert_eq!(
            task.waiting_since, waiting_since,
            "[{}] a deferral keeps waiting_since",
            backend
        );
        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 1, "[{}] a deferred task waits", backend);
        assert_eq!(
            stats.oldest_waiting_since,
            waiting_since.map(|t| t.0),
            "[{}]",
            backend
        );

        // Deferred to the past: ready again.
        let past = UniversalTimestamp(chrono::Utc::now() - chrono::Duration::seconds(1));
        dal.task_execution()
            .defer_dispatch(task_id, past)
            .await
            .expect("defer_dispatch to the past");
        let ready = dal.task_execution().get_ready_for_retry().await.unwrap();
        assert!(
            ready.iter().any(|t| t.id == task_id),
            "[{}] the task is ready when its time has come",
            backend
        );
    }
}

/// CLOACINA-T-1013: a terminal write guarded by a runner id applies only while
/// that runner holds the claim. The fleet executor guards its result writes
/// this way, so a result that comes back after the claim moved changes nothing.
#[tokio::test]
async fn test_guarded_completion_needs_the_claim() {
    for (backend, fixture) in get_all_fixtures().await {
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let (_exec_id, task_id) = create_running_task(&dal).await;
        let owner = UniversalUuid::new_v4();
        let late = UniversalUuid::new_v4();
        assert_eq!(
            dal.task_execution()
                .claim_for_runner(task_id, owner)
                .await
                .expect("claim"),
            RunnerClaimResult::Claimed
        );

        // A runner without the claim cannot complete or fail the task.
        let applied = dal
            .task_execution()
            .mark_completed(task_id, Some(late))
            .await
            .expect("mark_completed by a runner without the claim");
        assert!(
            !applied,
            "[{}] completion without the claim must not apply",
            backend
        );
        dal.task_execution()
            .mark_failed(task_id, "late failure", Some(late))
            .await
            .expect("mark_failed by a runner without the claim");
        let task = dal.task_execution().get_by_id(task_id).await.unwrap();
        assert_eq!(
            task.status, "Running",
            "[{}] the task is unchanged",
            backend
        );

        // The owner can.
        let applied = dal
            .task_execution()
            .mark_completed(task_id, Some(owner))
            .await
            .expect("mark_completed by the owner");
        assert!(applied, "[{}] the owner completes the task", backend);
        let task = dal.task_execution().get_by_id(task_id).await.unwrap();
        assert_eq!(task.status, "Completed", "[{}]", backend);
    }
}
