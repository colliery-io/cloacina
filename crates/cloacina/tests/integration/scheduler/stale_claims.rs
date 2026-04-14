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

//! Integration tests for the stale claim sweeper.

use crate::fixtures::get_all_fixtures;
use cloacina::dal::DAL;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::execution_planner::stale_claim_sweeper::{
    StaleClaimSweeper, StaleClaimSweeperConfig,
};
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::models::workflow_execution::NewWorkflowExecution;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

/// Create a sweeper with a very short stale threshold for testing.
fn test_sweeper(dal: Arc<DAL>, threshold: Duration) -> StaleClaimSweeper {
    let config = StaleClaimSweeperConfig {
        sweep_interval: Duration::from_millis(100),
        stale_threshold: threshold,
    };
    let (_tx, rx) = watch::channel(false);
    StaleClaimSweeper::new(dal, config, rx)
}

/// Helper: create a pipeline + task in "Running" state with a runner claim.
///
/// Creates the task as "Running" with claimed_by and heartbeat_at set,
/// simulating a task that was claimed by a runner that has since crashed.
async fn create_claimed_task(
    dal: &DAL,
    pipeline_name: &str,
    task_name: &str,
) -> (UniversalUuid, UniversalUuid) {
    let pipeline = dal
        .workflow_execution()
        .create(NewWorkflowExecution {
            workflow_name: pipeline_name.to_string(),
            workflow_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create task directly as "Running" (skipping the outbox/claim flow)
    let task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline.id,
            task_name: task_name.to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: r#"{"type":"Always"}"#.to_string(),
            task_configuration: "{}".to_string(),
        })
        .await
        .expect("Failed to create task");

    // Set the runner claim (claimed_by + heartbeat_at)
    let runner_id = uuid::Uuid::new_v4();
    dal.task_execution()
        .claim_for_runner(task.id, UniversalUuid(runner_id))
        .await
        .expect("Failed to claim task");

    (pipeline.id, task.id)
}

#[tokio::test]
async fn test_sweep_during_grace_period_is_noop() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!(
            "Running test_sweep_during_grace_period_is_noop on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        // Create a claimed task
        let (_pipeline_id, task_id) = create_claimed_task(&dal, "grace-test", "task1").await;

        // Create sweeper with a very long threshold (so we're always in grace period)
        let sweeper = test_sweeper(dal.clone(), Duration::from_secs(3600));

        // Sweep should be a no-op (we just started → grace period)
        sweeper.sweep().await;

        // Task should still be Running (not reset to Ready)
        let task = dal
            .task_execution()
            .get_by_id(task_id)
            .await
            .expect("Failed to get task");
        assert_eq!(
            task.status, "Running",
            "Task should remain Running during grace period"
        );
    }
}

#[tokio::test]
async fn test_sweep_after_grace_period_no_stale_claims() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!(
            "Running test_sweep_after_grace_period_no_stale_claims on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        // Create sweeper with zero threshold (immediately past grace period)
        // but no stale claims exist (no tasks at all)
        let sweeper = test_sweeper(dal.clone(), Duration::from_millis(0));

        // Small delay to ensure we're past the threshold
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Should complete without error, finding nothing
        sweeper.sweep().await;
    }
}

#[tokio::test]
async fn test_sweep_resets_stale_task_to_ready() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!(
            "Running test_sweep_resets_stale_task_to_ready on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        // Create a claimed task
        let (_pipeline_id, task_id) = create_claimed_task(&dal, "stale-test", "task1").await;

        // Verify task is Running
        let task = dal.task_execution().get_by_id(task_id).await.unwrap();
        assert_eq!(task.status, "Running");

        // Wait for heartbeat to age past the stale threshold
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // Create sweeper with 1s threshold — heartbeat is now >1s old → stale
        // Grace period also 1s, but we sleep past it
        let sweeper = test_sweeper(dal.clone(), Duration::from_secs(1));
        tokio::time::sleep(Duration::from_millis(1100)).await;

        sweeper.sweep().await;

        // Task should now be Ready
        let task = dal.task_execution().get_by_id(task_id).await.unwrap();
        assert_eq!(task.status, "Ready", "Stale task should be reset to Ready");
    }
}

#[tokio::test]
async fn test_sweep_multiple_stale_tasks() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!("Running test_sweep_multiple_stale_tasks on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        // Create multiple claimed tasks
        let (_, task1_id) = create_claimed_task(&dal, "multi-stale-1", "task1").await;
        let (_, task2_id) = create_claimed_task(&dal, "multi-stale-2", "task2").await;
        let (_, task3_id) = create_claimed_task(&dal, "multi-stale-3", "task3").await;

        // Wait for heartbeats to age past threshold
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let sweeper = test_sweeper(dal.clone(), Duration::from_secs(1));
        tokio::time::sleep(Duration::from_millis(1100)).await;

        sweeper.sweep().await;

        // All three should be reset to Ready
        for (label, id) in [
            ("task1", task1_id),
            ("task2", task2_id),
            ("task3", task3_id),
        ] {
            let task = dal.task_execution().get_by_id(id).await.unwrap();
            assert_eq!(
                task.status, "Ready",
                "{} should be Ready after sweep",
                label
            );
        }
    }
}

#[tokio::test]
async fn test_sweeper_run_loop_stops_on_shutdown() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!(
            "Running test_sweeper_run_loop_stops_on_shutdown on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        let config = StaleClaimSweeperConfig {
            sweep_interval: Duration::from_millis(50),
            stale_threshold: Duration::from_millis(0),
        };
        let (tx, rx) = watch::channel(false);
        let mut sweeper = StaleClaimSweeper::new(dal, config, rx);

        // Run the sweeper in a background task
        let handle = tokio::spawn(async move {
            sweeper.run().await;
        });

        // Let it run a couple of cycles
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Signal shutdown
        tx.send(true).unwrap();

        // Should stop within a reasonable time
        let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
        assert!(result.is_ok(), "Sweeper should stop after shutdown signal");
    }
}
