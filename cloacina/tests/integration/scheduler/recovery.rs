/*
 *  Copyright 2025 Colliery Software
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

use crate::fixtures::get_or_init_fixture;
use cloacina::dal::DAL;
use cloacina::database::schema::task_executions;
use cloacina::models::pipeline_execution::NewPipelineExecution;
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::*;
use diesel::prelude::*;
use serde_json::json;
use serial_test::serial;
use tracing::info;

#[tokio::test]
#[serial]
async fn test_orphaned_task_recovery() {
    let fixture = get_or_init_fixture().await;
    let mut guard = fixture.lock().unwrap();
    guard.initialize().await;
    let database = guard.get_database();
    let dal = DAL::new(database.pool());

    info!("Creating test pipeline with orphaned task");

    // Create a test pipeline execution
    let pipeline_execution = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "recovery-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .unwrap();

    // Create a task stuck in "Running" state (orphaned)
    let orphaned_task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "orphaned-task".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    info!("Creating scheduler with recovery (empty workflow registry)");

    // Create scheduler with recovery - empty registry means workflow is unavailable
    let _scheduler = TaskScheduler::new_with_recovery(database, vec![])
        .await
        .unwrap();

    info!("Verifying task was abandoned due to unavailable workflow");

    // With new graceful recovery, task should be abandoned since workflow is not in registry
    let abandoned_task = dal.task_execution().get_by_id(orphaned_task.id).await.unwrap();
    assert_eq!(abandoned_task.status, "Failed");
    assert!(abandoned_task
        .error_details
        .unwrap()
        .contains("Workflow 'recovery-test' no longer available"));
    assert!(abandoned_task.completed_at.is_some());

    // Verify pipeline was marked as failed
    let failed_pipeline = dal
        .pipeline_execution()
        .get_by_id(pipeline_execution.id)
        .await
        .unwrap();
    assert_eq!(failed_pipeline.status, "Failed");

    // Verify workflow unavailable recovery event was recorded
    let recovery_events = dal
        .recovery_event()
        .get_by_pipeline(pipeline_execution.id)
        .await
        .unwrap();
    assert!(!recovery_events.is_empty());
    let task_events: Vec<_> = recovery_events
        .iter()
        .filter(|e| e.task_execution_id.is_some())
        .collect();
    assert_eq!(task_events.len(), 1);
    assert_eq!(task_events[0].recovery_type, "workflow_unavailable");
    assert_eq!(task_events[0].task_execution_id, Some(orphaned_task.id));

    info!("Workflow unavailable recovery test completed successfully");
}

#[tokio::test]
#[serial]
async fn test_task_abandonment_after_max_retries() {
    let fixture = get_or_init_fixture().await;
    let mut guard = fixture.lock().unwrap();
    guard.initialize().await;
    let database = guard.get_database();
    let dal = DAL::new(database.pool());

    info!("Creating test pipeline with task at max recovery attempts");

    // Create a test pipeline execution
    let pipeline_execution = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "abandonment-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .unwrap();

    // Create a task stuck in "Running" state with maximum recovery attempts already reached
    let task_with_max_retries = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "max-retry-task".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    // Manually set recovery attempts to maximum (3)
    let task_id = task_with_max_retries.id;
    let conn = dal.pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(task_executions::table.find(task_id))
            .set(task_executions::recovery_attempts.eq(3))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    info!("Creating scheduler with recovery (empty workflow registry) - should abandon task");

    // Create scheduler with recovery - task will be abandoned due to workflow unavailability
    let _scheduler = TaskScheduler::new_with_recovery(database, vec![])
        .await
        .unwrap();

    info!("Verifying task was abandoned due to unavailable workflow");

    // Verify task was abandoned due to workflow unavailability (not max retries)
    let abandoned_task = dal
        .task_execution()
        .get_by_id(task_with_max_retries.id)
        .await
        .unwrap();
    assert_eq!(abandoned_task.status, "Failed");
    assert!(abandoned_task
        .error_details
        .unwrap()
        .contains("Workflow 'abandonment-test' no longer available"));
    assert!(abandoned_task.completed_at.is_some());

    // Verify pipeline was marked as failed due to abandoned task
    let failed_pipeline = dal
        .pipeline_execution()
        .get_by_id(pipeline_execution.id)
        .await
        .unwrap();
    assert_eq!(failed_pipeline.status, "Failed");

    // Verify workflow unavailable event was recorded (not task_abandoned)
    let recovery_events = dal
        .recovery_event()
        .get_by_task(task_with_max_retries.id)
        .await
        .unwrap();
    assert_eq!(recovery_events.len(), 1);
    assert_eq!(recovery_events[0].recovery_type, "workflow_unavailable");

    info!("Workflow unavailable abandonment test completed successfully");
}

#[tokio::test]
#[serial]
async fn test_no_recovery_needed() {
    let fixture = get_or_init_fixture().await;
    let mut guard = fixture.lock().unwrap();
    guard.initialize().await;
    let database = guard.get_database();
    let dal = DAL::new(database.pool());

    info!("Creating test pipeline with normal task states");

    // Create a test pipeline execution
    let pipeline_execution = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "no-recovery-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Completed".to_string(),
            context_id: None,
        })
        .await
        .unwrap();

    // Create tasks with normal states (no orphans)
    let _completed_task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "completed-task".to_string(),
            status: "Completed".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    let _ready_task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "ready-task".to_string(),
            status: "Ready".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    let _not_started_task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "not-started-task".to_string(),
            status: "NotStarted".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    info!("Creating scheduler with recovery - should find no orphans");

    // Create scheduler with recovery - should find no orphans
    let _scheduler = TaskScheduler::new_with_recovery(database, vec![])
        .await
        .unwrap();

    info!("Verifying no recovery events were created");

    // Verify no recovery events were created
    let recovery_events = dal
        .recovery_event()
        .get_by_pipeline(pipeline_execution.id)
        .await
        .unwrap();
    assert_eq!(recovery_events.len(), 0);

    info!("No recovery test completed successfully");
}

#[tokio::test]
#[serial]
async fn test_multiple_orphaned_tasks_recovery() {
    let fixture = get_or_init_fixture().await;
    let mut guard = fixture.lock().unwrap();
    guard.initialize().await;
    let database = guard.get_database();
    let dal = DAL::new(database.pool());

    info!("Creating test pipeline with multiple orphaned tasks");

    // Create a test pipeline execution
    let pipeline_execution = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "multi-recovery-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .unwrap();

    // Create multiple orphaned tasks
    let orphaned_task1 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "orphaned-task-1".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    let orphaned_task2 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "orphaned-task-2".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    // Create one task with max retries
    let max_retry_task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "max-retry-task".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    // Set max retries on one task
    let task_id = max_retry_task.id;
    let conn = dal.pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(task_executions::table.find(task_id))
            .set(task_executions::recovery_attempts.eq(3))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    info!("Creating scheduler with recovery (empty workflow registry) - should abandon all tasks");

    // Create scheduler with recovery - all tasks will be abandoned due to workflow unavailability
    let _scheduler = TaskScheduler::new_with_recovery(database, vec![])
        .await
        .unwrap();

    info!("Verifying all tasks were abandoned due to unavailable workflow");

    // All tasks should be abandoned due to workflow unavailability
    let abandoned_task1 = dal.task_execution().get_by_id(orphaned_task1.id).await.unwrap();
    assert_eq!(abandoned_task1.status, "Failed");
    assert!(abandoned_task1
        .error_details
        .unwrap()
        .contains("Workflow 'multi-recovery-test' no longer available"));

    let abandoned_task2 = dal.task_execution().get_by_id(orphaned_task2.id).await.unwrap();
    assert_eq!(abandoned_task2.status, "Failed");
    assert!(abandoned_task2
        .error_details
        .unwrap()
        .contains("Workflow 'multi-recovery-test' no longer available"));

    let abandoned_task3 = dal.task_execution().get_by_id(max_retry_task.id).await.unwrap();
    assert_eq!(abandoned_task3.status, "Failed");
    assert!(abandoned_task3
        .error_details
        .unwrap()
        .contains("Workflow 'multi-recovery-test' no longer available"));

    // Verify pipeline was marked as failed due to abandoned tasks
    let failed_pipeline = dal
        .pipeline_execution()
        .get_by_id(pipeline_execution.id)
        .await
        .unwrap();
    assert_eq!(failed_pipeline.status, "Failed");

    // Verify workflow unavailable recovery events were recorded
    let all_recovery_events = dal
        .recovery_event()
        .get_by_pipeline(pipeline_execution.id)
        .await
        .unwrap();
    assert!(!all_recovery_events.is_empty());

    let workflow_unavailable_events: Vec<_> = all_recovery_events
        .iter()
        .filter(|e| e.recovery_type == "workflow_unavailable")
        .collect();
    assert!(!workflow_unavailable_events.is_empty()); // Should have both task and pipeline events

    info!("Multiple workflow unavailable abandonment test completed successfully");
}

#[tokio::test]
#[serial]
async fn test_recovery_event_details() {
    let fixture = get_or_init_fixture().await;
    let mut guard = fixture.lock().unwrap();
    guard.initialize().await;
    let database = guard.get_database();
    let dal = DAL::new(database.pool());

    info!("Creating test pipeline to verify recovery event details");

    // Create a test pipeline execution
    let pipeline_execution = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "event-details-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .unwrap();

    // Create an orphaned task
    let orphaned_task = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "detail-test-task".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    info!("Creating scheduler with recovery (empty workflow registry)");

    // Trigger recovery - task will be abandoned due to workflow unavailability
    let _scheduler = TaskScheduler::new_with_recovery(database, vec![])
        .await
        .unwrap();

    info!("Verifying workflow unavailable recovery event details");

    // Verify workflow unavailable recovery event details
    let recovery_events = dal.recovery_event().get_by_task(orphaned_task.id).await.unwrap();
    assert_eq!(recovery_events.len(), 1);

    let event = &recovery_events[0];
    assert_eq!(event.recovery_type, "workflow_unavailable");
    assert_eq!(event.pipeline_execution_id, pipeline_execution.id);
    assert_eq!(event.task_execution_id, Some(orphaned_task.id));

    // Verify event details JSON for workflow unavailable
    let details_str = event.details.as_ref().unwrap();
    let details: serde_json::Value = serde_json::from_str(details_str).unwrap();
    assert_eq!(details["task_name"], "detail-test-task");
    assert_eq!(details["workflow_name"], "event-details-test");
    assert_eq!(details["reason"], "Workflow not in current registry");
    assert_eq!(details["action"], "abandoned");
    assert!(details["available_workflows"].is_array());

    info!("Workflow unavailable recovery event details test completed successfully");
}

#[tokio::test]
#[serial]
async fn test_graceful_recovery_for_unknown_workflow() {
    let fixture = get_or_init_fixture().await;
    let mut guard = fixture.lock().unwrap();
    guard.initialize().await;
    let database = guard.get_database();
    let dal = DAL::new(database.pool());

    info!("Creating test pipeline with unknown workflow");

    // Create a test pipeline execution from an unknown workflow
    let pipeline_execution = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "unknown-workflow".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .unwrap();

    // Create orphaned tasks from the unknown workflow
    let orphaned_task1 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "unknown-task-1".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    let orphaned_task2 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "unknown-task-2".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    info!("Creating scheduler with empty workflow registry - should gracefully abandon unknown workflow");

    // Create scheduler with empty workflow registry (no workflows available)
    let _scheduler = TaskScheduler::new_with_recovery(database, vec![])
        .await
        .unwrap();

    info!("Verifying tasks were abandoned gracefully");

    // Verify tasks were abandoned
    let abandoned_task1 = dal.task_execution().get_by_id(orphaned_task1.id).await.unwrap();
    assert_eq!(abandoned_task1.status, "Failed");
    assert!(abandoned_task1
        .error_details
        .unwrap()
        .contains("Workflow 'unknown-workflow' no longer available"));
    assert!(abandoned_task1.completed_at.is_some());

    let abandoned_task2 = dal.task_execution().get_by_id(orphaned_task2.id).await.unwrap();
    assert_eq!(abandoned_task2.status, "Failed");
    assert!(abandoned_task2
        .error_details
        .unwrap()
        .contains("Workflow 'unknown-workflow' no longer available"));
    assert!(abandoned_task2.completed_at.is_some());

    // Verify pipeline was marked as failed
    let failed_pipeline = dal
        .pipeline_execution()
        .get_by_id(pipeline_execution.id)
        .await
        .unwrap();
    assert_eq!(failed_pipeline.status, "Failed");
    assert!(failed_pipeline
        .error_details
        .unwrap()
        .contains("abandoned during recovery"));

    // Verify workflow unavailable events were recorded
    let workflow_unavailable_events = dal
        .recovery_event()
        .get_by_pipeline(pipeline_execution.id)
        .await
        .unwrap();
    assert!(!workflow_unavailable_events.is_empty());

    let task_events: Vec<_> = workflow_unavailable_events
        .iter()
        .filter(|e| e.task_execution_id.is_some())
        .collect();
    assert_eq!(task_events.len(), 2); // Two task abandonment events

    let pipeline_events: Vec<_> = workflow_unavailable_events
        .iter()
        .filter(|e| e.task_execution_id.is_none())
        .collect();
    assert_eq!(pipeline_events.len(), 1); // One pipeline failure event

    // Verify event details
    let task_event = &task_events[0];
    assert_eq!(task_event.recovery_type, "workflow_unavailable");
    assert_eq!(task_event.pipeline_execution_id, pipeline_execution.id);

    let details_str = task_event.details.as_ref().unwrap();
    let details: serde_json::Value = serde_json::from_str(details_str).unwrap();
    assert_eq!(details["workflow_name"], "unknown-workflow");
    assert_eq!(details["reason"], "Workflow not in current registry");
    assert_eq!(details["action"], "abandoned");

    info!("Graceful recovery test completed successfully");
}

#[tokio::test]
#[serial]
async fn test_recovery_event_details_multiple_tasks() {
    let fixture = get_or_init_fixture().await;
    let mut guard = fixture.lock().unwrap();
    guard.initialize().await;
    let database = guard.get_database();
    let dal = DAL::new(database.pool());

    info!("Creating test pipeline to verify recovery event details for multiple tasks");

    // Create a test pipeline execution
    let pipeline_execution = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "event-details-multiple-tasks-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .unwrap();

    // Create multiple orphaned tasks
    let orphaned_task1 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "detail-test-task-1".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    let orphaned_task2 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "detail-test-task-2".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    info!("Creating scheduler with recovery (empty workflow registry) - should record detailed events");

    // Create scheduler with recovery - should record detailed events
    let _scheduler = TaskScheduler::new_with_recovery(database, vec![])
        .await
        .unwrap();

    info!("Verifying recovery events were recorded with details");

    // Verify recovery events were recorded with details
    let recovery_events = dal
        .recovery_event()
        .get_by_pipeline(pipeline_execution.id)
        .await
        .unwrap();
    assert_eq!(recovery_events.len(), 3); // Two task events + one pipeline event

    let task_events: Vec<_> = recovery_events
        .iter()
        .filter(|e| e.task_execution_id.is_some())
        .collect();
    assert_eq!(task_events.len(), 2); // Two task events

    let pipeline_events: Vec<_> = recovery_events
        .iter()
        .filter(|e| e.task_execution_id.is_none())
        .collect();
    assert_eq!(pipeline_events.len(), 1); // One pipeline event

    // Find the event for each task by matching task IDs
    let event1 = task_events
        .iter()
        .find(|e| e.task_execution_id == Some(orphaned_task1.id))
        .expect("Could not find event for task 1");
    assert_eq!(event1.recovery_type, "workflow_unavailable");
    assert_eq!(event1.pipeline_execution_id, pipeline_execution.id);

    let details1_str = event1.details.as_ref().unwrap();
    let details1: serde_json::Value = serde_json::from_str(details1_str).unwrap();
    assert_eq!(details1["task_name"], "detail-test-task-1");
    assert_eq!(
        details1["workflow_name"],
        "event-details-multiple-tasks-test"
    );
    assert_eq!(details1["reason"], "Workflow not in current registry");
    assert_eq!(details1["action"], "abandoned");
    assert!(details1["available_workflows"].is_array());

    let event2 = task_events
        .iter()
        .find(|e| e.task_execution_id == Some(orphaned_task2.id))
        .expect("Could not find event for task 2");
    assert_eq!(event2.recovery_type, "workflow_unavailable");
    assert_eq!(event2.pipeline_execution_id, pipeline_execution.id);

    let details2_str = event2.details.as_ref().unwrap();
    let details2: serde_json::Value = serde_json::from_str(details2_str).unwrap();
    assert_eq!(details2["task_name"], "detail-test-task-2");
    assert_eq!(
        details2["workflow_name"],
        "event-details-multiple-tasks-test"
    );
    assert_eq!(details2["reason"], "Workflow not in current registry");
    assert_eq!(details2["action"], "abandoned");
    assert!(details2["available_workflows"].is_array());

    let pipeline_event = &pipeline_events[0];
    assert_eq!(pipeline_event.recovery_type, "workflow_unavailable");
    assert_eq!(pipeline_event.pipeline_execution_id, pipeline_execution.id);
    assert_eq!(pipeline_event.task_execution_id, None);

    let pipeline_details_str = pipeline_event.details.as_ref().unwrap();
    let pipeline_details: serde_json::Value = serde_json::from_str(pipeline_details_str).unwrap();
    assert_eq!(
        pipeline_details["workflow_name"],
        "event-details-multiple-tasks-test"
    );
    assert_eq!(
        pipeline_details["reason"],
        "Workflow not in current registry"
    );
    assert_eq!(pipeline_details["action"], "pipeline_failed");
    assert!(pipeline_details["available_workflows"].is_array());

    info!("Recovery event details test completed successfully");
}

#[tokio::test]
#[serial]
async fn test_recovery_event_details_unknown_workflow() {
    let fixture = get_or_init_fixture().await;
    let mut guard = fixture.lock().unwrap();
    guard.initialize().await;
    let database = guard.get_database();
    let dal = DAL::new(database.pool());

    info!("Creating test pipeline to verify recovery event details for unknown workflow");

    // Create a test pipeline execution from an unknown workflow
    let pipeline_execution = dal
        .pipeline_execution()
        .create(NewPipelineExecution {
            pipeline_name: "unknown-workflow-test".to_string(),
            pipeline_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: None,
        })
        .await
        .unwrap();

    // Create orphaned tasks from the unknown workflow
    let orphaned_task1 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "unknown-task-1".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    let orphaned_task2 = dal
        .task_execution()
        .create(NewTaskExecution {
            pipeline_execution_id: pipeline_execution.id,
            task_name: "unknown-task-2".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap();

    info!("Creating scheduler with recovery (empty workflow registry) - should handle unknown workflow gracefully");

    // Create scheduler with recovery - should handle unknown workflow gracefully
    let _scheduler = TaskScheduler::new_with_recovery(database, vec![])
        .await
        .unwrap();

    info!("Verifying graceful handling of unknown workflow");

    // Verify tasks were abandoned
    let abandoned_task1 = dal.task_execution().get_by_id(orphaned_task1.id).await.unwrap();
    assert_eq!(abandoned_task1.status, "Failed");
    assert!(abandoned_task1
        .error_details
        .unwrap()
        .contains("Workflow 'unknown-workflow-test' no longer available"));
    assert!(abandoned_task1.completed_at.is_some());

    let abandoned_task2 = dal.task_execution().get_by_id(orphaned_task2.id).await.unwrap();
    assert_eq!(abandoned_task2.status, "Failed");
    assert!(abandoned_task2
        .error_details
        .unwrap()
        .contains("Workflow 'unknown-workflow-test' no longer available"));
    assert!(abandoned_task2.completed_at.is_some());

    // Verify pipeline was marked as failed
    let failed_pipeline = dal
        .pipeline_execution()
        .get_by_id(pipeline_execution.id)
        .await
        .unwrap();
    assert_eq!(failed_pipeline.status, "Failed");
    assert!(failed_pipeline
        .error_details
        .unwrap()
        .contains("abandoned during recovery"));

    // Verify workflow unavailable events were recorded
    let workflow_unavailable_events = dal
        .recovery_event()
        .get_by_pipeline(pipeline_execution.id)
        .await
        .unwrap();
    assert!(!workflow_unavailable_events.is_empty());

    let task_events: Vec<_> = workflow_unavailable_events
        .iter()
        .filter(|e| e.task_execution_id.is_some())
        .collect();
    assert_eq!(task_events.len(), 2); // Two task abandonment events

    let pipeline_events: Vec<_> = workflow_unavailable_events
        .iter()
        .filter(|e| e.task_execution_id.is_none())
        .collect();
    assert_eq!(pipeline_events.len(), 1); // One pipeline failure event

    // Verify event details
    let task_event = &task_events[0];
    assert_eq!(task_event.recovery_type, "workflow_unavailable");
    assert_eq!(task_event.pipeline_execution_id, pipeline_execution.id);

    let details_str = task_event.details.as_ref().unwrap();
    let details: serde_json::Value = serde_json::from_str(details_str).unwrap();
    assert_eq!(details["workflow_name"], "unknown-workflow-test");
    assert_eq!(details["reason"], "Workflow not in current registry");
    assert_eq!(details["action"], "abandoned");

    info!("Graceful recovery test completed successfully");
}
