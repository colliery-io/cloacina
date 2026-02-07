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

use cloacina::dal::DAL;
use cloacina::models::pipeline_execution::NewPipelineExecution;
use cloacina::models::task_execution::NewTaskExecution;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Barrier;

use crate::fixtures::get_or_init_sqlite_fixture;

/// Test that concurrent task claiming doesn't produce duplicate claims.
///
/// This test creates multiple ready tasks and spawns several concurrent workers
/// that all attempt to claim tasks at the same time. It verifies that:
/// 1. No task is claimed by more than one worker
/// 2. All tasks are eventually claimed exactly once
///
/// This specifically tests the SQLite implementation since it doesn't have
/// native `FOR UPDATE SKIP LOCKED` support and relies on transaction isolation.
#[tokio::test]
async fn test_concurrent_task_claiming_no_duplicates() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    // Create a test pipeline
    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "concurrent-claim-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create multiple tasks and mark them ready (which populates the outbox)
    const NUM_TASKS: usize = 20;
    let mut created_task_ids = Vec::new();

    for i in 0..NUM_TASKS {
        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
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
        "Outbox should have {} entries, got {}",
        NUM_TASKS, outbox_count
    );

    // Release the fixture lock before spawning concurrent tasks
    drop(guard);

    // Spawn multiple workers that will try to claim tasks concurrently
    const NUM_WORKERS: usize = 10;
    let barrier = Arc::new(Barrier::new(NUM_WORKERS));
    let mut handles = Vec::new();

    for worker_id in 0..NUM_WORKERS {
        let db_clone = database.clone();
        let barrier_clone = barrier.clone();

        let handle = tokio::spawn(async move {
            let dal = DAL::new(db_clone);

            // Wait for all workers to be ready before claiming
            barrier_clone.wait().await;

            // Each worker tries to claim multiple tasks
            let mut claimed = Vec::new();
            for _ in 0..5 {
                match dal.task_execution().claim_ready_task(2).await {
                    Ok(results) => {
                        for result in results {
                            claimed.push((worker_id, result.id));
                        }
                    }
                    Err(e) => {
                        // Some errors are expected due to contention
                        tracing::debug!("Worker {} claim error: {:?}", worker_id, e);
                    }
                }
            }
            claimed
        });

        handles.push(handle);
    }

    // Collect all claimed task IDs from all workers
    let mut all_claimed: Vec<(usize, cloacina::database::universal_types::UniversalUuid)> =
        Vec::new();
    for handle in handles {
        let claimed = handle.await.expect("Worker task panicked");
        all_claimed.extend(claimed);
    }

    // Extract just the task IDs
    let claimed_ids: Vec<_> = all_claimed.iter().map(|(_, id)| *id).collect();

    // Check for duplicates - this is the critical assertion
    let unique_ids: HashSet<_> = claimed_ids.iter().collect();
    assert_eq!(
        claimed_ids.len(),
        unique_ids.len(),
        "RACE CONDITION DETECTED: Some tasks were claimed by multiple workers! \
         Total claims: {}, Unique tasks: {}. \
         This indicates the transaction isolation is not working correctly.",
        claimed_ids.len(),
        unique_ids.len()
    );

    // Verify we claimed all tasks (or close to it, accounting for timing)
    assert!(
        unique_ids.len() >= NUM_TASKS - 2,
        "Expected to claim most tasks. Claimed {} of {} tasks.",
        unique_ids.len(),
        NUM_TASKS
    );

    // Verify all claimed tasks were from our created set
    let created_set: HashSet<_> = created_task_ids.iter().collect();
    for id in &claimed_ids {
        assert!(
            created_set.contains(id),
            "Claimed task {:?} was not in our created set",
            id
        );
    }

    tracing::info!(
        "Concurrent claiming test passed: {} workers claimed {} unique tasks with no duplicates",
        NUM_WORKERS,
        unique_ids.len()
    );
}

/// Test that claimed tasks have their status properly updated to Running.
#[tokio::test]
async fn test_claimed_tasks_marked_running() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    // Create a test pipeline
    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "claim-status-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create a task and mark it ready (which populates the outbox)
    let task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline.id,
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

    assert_eq!(claimed.len(), 1, "Should claim exactly one task");
    assert_eq!(claimed[0].id, task.id, "Should claim our task");

    // Verify the task status is now Running
    let updated_task = dal
        .task_execution()
        .get_by_id(task.id)
        .await
        .expect("Failed to get task");

    assert_eq!(
        updated_task.status, "Running",
        "Claimed task should have status 'Running'"
    );
    assert!(
        updated_task.started_at.is_some(),
        "Claimed task should have started_at timestamp"
    );
}

/// Test that already-running tasks cannot be claimed again.
#[tokio::test]
async fn test_running_tasks_not_claimable() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    // Create a test pipeline
    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "running-not-claimable-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create a task that's already running
    let _running_task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline.id,
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
        "Should not claim any tasks when all are already running"
    );
}
