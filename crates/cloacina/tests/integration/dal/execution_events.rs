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

//! Integration tests for execution events and outbox-based task distribution.
//!
//! These tests verify:
//! - Execution events are emitted correctly by DAL operations
//! - Events are queryable by pipeline_id, task_id, and event_type
//! - Outbox-based claiming works with concurrent workers
//! - No duplicate claims (exactly-once semantics)

use cloacina::dal::DAL;
use cloacina::database::universal_types::{UniversalTimestamp, UniversalUuid};
use cloacina::models::execution_event::{ExecutionEventType, NewExecutionEvent};
use cloacina::models::pipeline_execution::NewPipelineExecution;
use cloacina::models::task_execution::NewTaskExecution;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Barrier;

use crate::fixtures::get_or_init_sqlite_fixture;

#[cfg(feature = "postgres")]
use crate::fixtures::get_or_init_postgres_fixture;

// =============================================================================
// Test Case 1: Event Emission by DAL Operations
// =============================================================================

/// Test that DAL operations automatically emit execution events.
/// This verifies the core integration - task create, mark_ready, claim, complete
/// all emit their respective events.
#[tokio::test]
async fn test_dal_emits_events_on_state_transitions() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    // Create a pipeline
    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "event-emission-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create a task - should emit TaskCreated event
    let task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline.id,
            task_name: "test-task".to_string(),
            status: "NotStarted".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .expect("Failed to create task");

    // Query events after task creation
    let events_after_create = dal
        .execution_event()
        .list_by_task(task.id)
        .await
        .expect("Failed to list events");

    assert_eq!(
        events_after_create.len(),
        1,
        "Task creation should emit exactly 1 event"
    );
    assert_eq!(
        events_after_create[0].event_type,
        ExecutionEventType::TaskCreated.as_str(),
        "First event should be TaskCreated"
    );

    // Mark task ready - should emit TaskMarkedReady event
    dal.task_execution()
        .mark_ready(task.id)
        .await
        .expect("Failed to mark task ready");

    let events_after_ready = dal
        .execution_event()
        .list_by_task(task.id)
        .await
        .expect("Failed to list events");

    assert_eq!(
        events_after_ready.len(),
        2,
        "After mark_ready should have 2 events"
    );
    assert_eq!(
        events_after_ready[1].event_type,
        ExecutionEventType::TaskMarkedReady.as_str(),
        "Second event should be TaskMarkedReady"
    );

    // Claim the task - should emit TaskClaimed event
    let claimed = dal
        .task_execution()
        .claim_ready_task(1)
        .await
        .expect("Failed to claim task");
    assert_eq!(claimed.len(), 1, "Should claim 1 task");

    let events_after_claim = dal
        .execution_event()
        .list_by_task(task.id)
        .await
        .expect("Failed to list events");

    assert_eq!(
        events_after_claim.len(),
        3,
        "After claim should have 3 events"
    );
    assert_eq!(
        events_after_claim[2].event_type,
        ExecutionEventType::TaskClaimed.as_str(),
        "Third event should be TaskClaimed"
    );

    // Mark task completed - should emit TaskCompleted event
    dal.task_execution()
        .mark_completed(task.id)
        .await
        .expect("Failed to mark completed");

    let events_after_complete = dal
        .execution_event()
        .list_by_task(task.id)
        .await
        .expect("Failed to list events");

    assert_eq!(
        events_after_complete.len(),
        4,
        "After completion should have 4 events"
    );
    assert_eq!(
        events_after_complete[3].event_type,
        ExecutionEventType::TaskCompleted.as_str(),
        "Fourth event should be TaskCompleted"
    );

    // Verify events are ordered by sequence number
    for i in 1..events_after_complete.len() {
        assert!(
            events_after_complete[i].sequence_num > events_after_complete[i - 1].sequence_num,
            "Events should be ordered by sequence number"
        );
    }
}

/// Test that events can be queried by pipeline_id.
#[tokio::test]
async fn test_events_queryable_by_pipeline() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    // Create two pipelines
    let pipeline1 = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "pipeline-1".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline1");

    let pipeline2 = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "pipeline-2".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline2");

    // Create tasks in each pipeline (each creation emits an event)
    for i in 0..3 {
        dal.task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline1.id,
                task_name: format!("p1-task-{}", i),
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");
    }

    for i in 0..2 {
        dal.task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline2.id,
                task_name: format!("p2-task-{}", i),
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");
    }

    // Query events by pipeline
    let p1_events = dal
        .execution_event()
        .list_by_pipeline(pipeline1.id)
        .await
        .expect("Failed to list p1 events");

    let p2_events = dal
        .execution_event()
        .list_by_pipeline(pipeline2.id)
        .await
        .expect("Failed to list p2 events");

    // Each pipeline emits PipelineStarted, plus TaskCreated for each task
    assert_eq!(
        p1_events.len(),
        4,
        "Pipeline1 should have 4 events (1 pipeline + 3 tasks)"
    );
    assert_eq!(
        p2_events.len(),
        3,
        "Pipeline2 should have 3 events (1 pipeline + 2 tasks)"
    );

    // Verify all p1 events belong to p1
    for event in &p1_events {
        assert_eq!(
            event.pipeline_execution_id, pipeline1.id,
            "All events should belong to pipeline1"
        );
    }
}

/// Test that events can be queried by task_id.
#[tokio::test]
async fn test_events_queryable_by_task() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "task-query-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create two tasks
    let task1 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline.id,
            task_name: "task-1".to_string(),
            status: "NotStarted".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .expect("Failed to create task1");

    let task2 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline.id,
            task_name: "task-2".to_string(),
            status: "NotStarted".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .expect("Failed to create task2");

    // Mark task1 ready (adds another event for task1 only)
    dal.task_execution()
        .mark_ready(task1.id)
        .await
        .expect("Failed to mark ready");

    // Query events by task
    let t1_events = dal
        .execution_event()
        .list_by_task(task1.id)
        .await
        .expect("Failed to list t1 events");

    let t2_events = dal
        .execution_event()
        .list_by_task(task2.id)
        .await
        .expect("Failed to list t2 events");

    assert_eq!(
        t1_events.len(),
        2,
        "Task1 should have 2 events (create + ready)"
    );
    assert_eq!(
        t2_events.len(),
        1,
        "Task2 should have 1 event (create only)"
    );

    // Verify task1 events belong to task1
    for event in &t1_events {
        assert_eq!(
            event.task_execution_id,
            Some(task1.id),
            "All events should belong to task1"
        );
    }
}

/// Test that events can be queried by event type.
#[tokio::test]
async fn test_events_queryable_by_type() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "type-query-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create tasks and transition them through states
    for i in 0..3 {
        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: format!("task-{}", i),
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");

        dal.task_execution()
            .mark_ready(task.id)
            .await
            .expect("Failed to mark ready");
    }

    // Query by TaskCreated type
    let created_events = dal
        .execution_event()
        .list_by_type(ExecutionEventType::TaskCreated, 100)
        .await
        .expect("Failed to list by type");

    assert_eq!(created_events.len(), 3, "Should have 3 TaskCreated events");
    for event in &created_events {
        assert_eq!(event.event_type, "task_created");
    }

    // Query by TaskMarkedReady type
    let ready_events = dal
        .execution_event()
        .list_by_type(ExecutionEventType::TaskMarkedReady, 100)
        .await
        .expect("Failed to list by type");

    assert_eq!(
        ready_events.len(),
        3,
        "Should have 3 TaskMarkedReady events"
    );
    for event in &ready_events {
        assert_eq!(event.event_type, "task_marked_ready");
    }
}

// =============================================================================
// Test Case 2: Outbox Empty After All Tasks Claimed
// =============================================================================

/// Test that the outbox is empty after all tasks are claimed.
#[tokio::test]
async fn test_outbox_empty_after_claiming() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "outbox-empty-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create tasks and mark them ready
    const NUM_TASKS: usize = 5;
    for i in 0..NUM_TASKS {
        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: format!("outbox-task-{}", i),
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");

        dal.task_execution()
            .mark_ready(task.id)
            .await
            .expect("Failed to mark task ready");
    }

    // Verify outbox has entries
    let initial_count = dal
        .task_outbox()
        .count_pending()
        .await
        .expect("Failed to count outbox");
    assert_eq!(
        initial_count as usize, NUM_TASKS,
        "Outbox should have {} entries before claiming",
        NUM_TASKS
    );

    // Claim all tasks
    let claimed = dal
        .task_execution()
        .claim_ready_task(NUM_TASKS)
        .await
        .expect("Failed to claim tasks");

    assert_eq!(
        claimed.len(),
        NUM_TASKS,
        "Should claim all {} tasks",
        NUM_TASKS
    );

    // Verify outbox is empty
    let final_count = dal
        .task_outbox()
        .count_pending()
        .await
        .expect("Failed to count outbox");
    assert_eq!(
        final_count, 0,
        "Outbox should be empty after claiming all tasks"
    );
}

// =============================================================================
// Test Case 3: Concurrent Claiming - No Duplicates
// =============================================================================

/// Test that concurrent workers don't cause duplicate claims.
#[tokio::test]
async fn test_concurrent_claiming_no_duplicates() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "concurrent-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create tasks and mark ready
    const NUM_TASKS: usize = 15;
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

        dal.task_execution()
            .mark_ready(task.id)
            .await
            .expect("Failed to mark task ready");
    }

    // Release lock before spawning tasks
    drop(guard);

    // Spawn concurrent workers
    const NUM_WORKERS: usize = 5;
    let barrier = Arc::new(Barrier::new(NUM_WORKERS));
    let mut handles = Vec::new();

    for worker_id in 0..NUM_WORKERS {
        let db_clone = database.clone();
        let barrier_clone = barrier.clone();

        let handle = tokio::spawn(async move {
            let dal = DAL::new(db_clone);
            barrier_clone.wait().await;

            let mut claimed = Vec::new();
            for _ in 0..5 {
                match dal.task_execution().claim_ready_task(3).await {
                    Ok(results) => {
                        for result in results {
                            claimed.push((worker_id, result.id));
                        }
                    }
                    Err(_) => {}
                }
            }
            claimed
        });

        handles.push(handle);
    }

    // Collect results
    let mut all_claimed: Vec<(usize, UniversalUuid)> = Vec::new();
    for handle in handles {
        let claimed = handle.await.expect("Worker panicked");
        all_claimed.extend(claimed);
    }

    // Verify no duplicates - this is the critical assertion
    let claimed_ids: Vec<_> = all_claimed.iter().map(|(_, id)| *id).collect();
    let unique_ids: HashSet<_> = claimed_ids.iter().collect();
    assert_eq!(
        claimed_ids.len(),
        unique_ids.len(),
        "RACE CONDITION DETECTED: Some tasks were claimed by multiple workers! \
         Total claims: {}, Unique tasks: {}",
        claimed_ids.len(),
        unique_ids.len()
    );

    // Verify we claimed all or most tasks
    assert!(
        unique_ids.len() >= NUM_TASKS - 2,
        "Should claim most tasks ({} of {})",
        unique_ids.len(),
        NUM_TASKS
    );

    // Re-acquire fixture to check outbox
    let fixture = get_or_init_sqlite_fixture().await;
    let guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    // Verify outbox is empty
    let final_count = dal
        .task_outbox()
        .count_pending()
        .await
        .expect("Failed to count outbox");
    assert_eq!(
        final_count, 0,
        "Outbox should be empty after concurrent claiming"
    );
}

// =============================================================================
// Test Case 4: Event Count and Deletion (Retention Policy)
// =============================================================================

/// Test count_by_pipeline and delete_older_than for retention policy.
#[tokio::test]
async fn test_event_count_and_deletion() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "retention-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create tasks (each emits TaskCreated event)
    for i in 0..5 {
        dal.task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: format!("task-{}", i),
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");
    }

    // Verify count: 1 PipelineStarted + 5 TaskCreated = 6 events
    let count = dal
        .execution_event()
        .count_by_pipeline(pipeline.id)
        .await
        .expect("Failed to count events");
    assert_eq!(count, 6, "Should have 6 events (1 pipeline + 5 tasks)");

    // Set cutoff to future (should delete all)
    let future_cutoff = UniversalTimestamp(chrono::Utc::now() + chrono::Duration::hours(1));

    // Count events older than cutoff
    let older_count = dal
        .execution_event()
        .count_older_than(future_cutoff)
        .await
        .expect("Failed to count old events");
    assert_eq!(
        older_count, 6,
        "All 6 events should be counted as older than future"
    );

    // Delete old events
    let deleted = dal
        .execution_event()
        .delete_older_than(future_cutoff)
        .await
        .expect("Failed to delete old events");
    assert_eq!(deleted, 6, "Should delete all 6 events");

    // Verify no events remain
    let remaining = dal
        .execution_event()
        .count_by_pipeline(pipeline.id)
        .await
        .expect("Failed to count remaining");
    assert_eq!(remaining, 0, "No events should remain after deletion");
}

/// Test get_recent returns events in correct order.
#[tokio::test]
async fn test_get_recent_events() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "recent-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create 5 tasks (each emits an event)
    for i in 0..5 {
        dal.task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: format!("task-{}", i),
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");
    }

    // Get recent 3 events
    let recent = dal
        .execution_event()
        .get_recent(3)
        .await
        .expect("Failed to get recent events");

    assert_eq!(recent.len(), 3, "Should return 3 recent events");

    // Verify they are ordered by created_at descending (most recent first)
    for i in 1..recent.len() {
        assert!(
            recent[i - 1].created_at.0 >= recent[i].created_at.0,
            "Events should be ordered by created_at descending"
        );
    }
}

// =============================================================================
// Test Case 5: Manual Event Creation with Event Data
// =============================================================================

/// Test that manually created events with event_data are correctly stored.
#[tokio::test]
async fn test_manual_event_with_data() {
    let fixture = get_or_init_sqlite_fixture().await;
    let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
    guard.reset_database().await;
    guard.initialize().await;

    let database = guard.get_database();
    let dal = DAL::new(database.clone());

    let pipeline = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "event-data-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .expect("Failed to create pipeline");

    // Create event with complex event_data
    let event_data = json!({
        "error_message": "Task failed due to timeout",
        "attempt": 2,
        "duration_ms": 30000,
        "metadata": {
            "retry_count": 1,
            "worker_id": "worker-123"
        }
    });

    dal.execution_event()
        .create(NewExecutionEvent::pipeline_event(
            pipeline.id,
            ExecutionEventType::TaskFailed,
            Some(event_data.to_string()),
            Some("test-worker".to_string()),
        ))
        .await
        .expect("Failed to create event with data");

    // Query and verify data is preserved
    // Should have 2 events: PipelineStarted (auto) + TaskFailed (manual)
    let events = dal
        .execution_event()
        .list_by_pipeline(pipeline.id)
        .await
        .expect("Failed to list events");

    assert_eq!(
        events.len(),
        2,
        "Should have 2 events (1 pipeline started + 1 manual)"
    );
    // The manual event should be the second one (ordered by sequence_num)
    let event = &events[1];
    assert!(event.event_data.is_some(), "Event data should be present");

    let retrieved_data: serde_json::Value =
        serde_json::from_str(event.event_data.as_ref().unwrap())
            .expect("Event data should be valid JSON");

    assert_eq!(
        retrieved_data["error_message"], "Task failed due to timeout",
        "Event data should be preserved"
    );
    assert_eq!(
        event.worker_id,
        Some("test-worker".to_string()),
        "Worker ID should be preserved"
    );
}

// =============================================================================
// PostgreSQL-specific tests (if feature enabled)
// =============================================================================

#[cfg(feature = "postgres")]
mod postgres_tests {
    use super::*;

    /// Test execution events on PostgreSQL backend.
    #[tokio::test]
    async fn test_postgres_execution_events() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "postgres-event-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .expect("Failed to create pipeline");

        // Create a task - should emit TaskCreated event
        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "pg-task".to_string(),
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");

        // Query events
        let events = dal
            .execution_event()
            .list_by_task(task.id)
            .await
            .expect("Failed to list events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "task_created");
        assert!(
            events[0].sequence_num > 0,
            "Sequence number should be assigned"
        );
    }

    /// Test concurrent claiming on PostgreSQL with FOR UPDATE SKIP LOCKED.
    #[tokio::test]
    async fn test_postgres_concurrent_claiming() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "pg-concurrent-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .expect("Failed to create pipeline");

        // Create tasks
        const NUM_TASKS: usize = 20;
        for i in 0..NUM_TASKS {
            let task = dal
                .task_execution()
                .create(NewTaskExecution {
                    pipeline_execution_id: pipeline.id,
                    task_name: format!("pg-task-{}", i),
                    status: "NotStarted".to_string(),
                    attempt: 1,
                    max_attempts: 3,
                    trigger_rules: json!({"type": "Always"}).to_string(),
                    task_configuration: json!({}).to_string(),
                })
                .await
                .expect("Failed to create task");

            dal.task_execution()
                .mark_ready(task.id)
                .await
                .expect("Failed to mark ready");
        }

        drop(guard);

        // Spawn concurrent workers
        const NUM_WORKERS: usize = 8;
        let barrier = Arc::new(Barrier::new(NUM_WORKERS));
        let mut handles = Vec::new();

        for worker_id in 0..NUM_WORKERS {
            let db_clone = database.clone();
            let barrier_clone = barrier.clone();

            let handle = tokio::spawn(async move {
                let dal = DAL::new(db_clone);
                barrier_clone.wait().await;

                let mut claimed = Vec::new();
                for _ in 0..5 {
                    match dal.task_execution().claim_ready_task(4).await {
                        Ok(results) => {
                            for result in results {
                                claimed.push((worker_id, result.id));
                            }
                        }
                        Err(_) => {}
                    }
                }
                claimed
            });

            handles.push(handle);
        }

        let mut all_claimed: Vec<(usize, UniversalUuid)> = Vec::new();
        for handle in handles {
            let claimed = handle.await.expect("Worker panicked");
            all_claimed.extend(claimed);
        }

        // Verify no duplicates
        let claimed_ids: Vec<_> = all_claimed.iter().map(|(_, id)| *id).collect();
        let unique_ids: HashSet<_> = claimed_ids.iter().collect();
        assert_eq!(
            claimed_ids.len(),
            unique_ids.len(),
            "PostgreSQL FOR UPDATE SKIP LOCKED should prevent duplicate claims"
        );

        // Verify we claimed all or most tasks
        assert!(
            unique_ids.len() >= NUM_TASKS - 2,
            "Should claim most tasks ({} of {})",
            unique_ids.len(),
            NUM_TASKS
        );
    }
}
