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

// Recovery tests run against both PostgreSQL and SQLite to verify consistent
// behavior across backends.

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_postgres_fixture;
    use cloacina::dal::DAL;
    use cloacina::database::schema::postgres::task_executions;
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
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with orphaned task");

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

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying task was abandoned due to unavailable workflow");

        let abandoned_task = dal
            .task_execution()
            .get_by_id(orphaned_task.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task.status, "Failed");
        assert!(abandoned_task
            .error_details
            .unwrap()
            .contains("Workflow 'recovery-test' no longer available"));
        assert!(abandoned_task.completed_at.is_some());

        let failed_pipeline = dal
            .pipeline_execution()
            .get_by_id(pipeline_execution.id)
            .await
            .unwrap();
        assert_eq!(failed_pipeline.status, "Failed");

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
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with task at max recovery attempts");

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
        let conn = database.get_postgres_connection().await.unwrap();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id.0))
                .set(task_executions::recovery_attempts.eq(3))
                .execute(conn)
        })
        .await
        .unwrap()
        .unwrap();

        info!("Creating scheduler with recovery (empty workflow registry) - should abandon task");

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying task was abandoned due to unavailable workflow");

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

        let failed_pipeline = dal
            .pipeline_execution()
            .get_by_id(pipeline_execution.id)
            .await
            .unwrap();
        assert_eq!(failed_pipeline.status, "Failed");

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
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with normal task states");

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

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying no recovery events were created");

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
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with multiple orphaned tasks");

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
        let conn = database.get_postgres_connection().await.unwrap();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id.0))
                .set(task_executions::recovery_attempts.eq(3))
                .execute(conn)
        })
        .await
        .unwrap()
        .unwrap();

        info!(
            "Creating scheduler with recovery (empty workflow registry) - should abandon all tasks"
        );

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying all tasks were abandoned due to unavailable workflow");

        let abandoned_task1 = dal
            .task_execution()
            .get_by_id(orphaned_task1.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task1.status, "Failed");
        assert!(abandoned_task1
            .error_details
            .unwrap()
            .contains("Workflow 'multi-recovery-test' no longer available"));

        let abandoned_task2 = dal
            .task_execution()
            .get_by_id(orphaned_task2.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task2.status, "Failed");
        assert!(abandoned_task2
            .error_details
            .unwrap()
            .contains("Workflow 'multi-recovery-test' no longer available"));

        let abandoned_task3 = dal
            .task_execution()
            .get_by_id(max_retry_task.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task3.status, "Failed");
        assert!(abandoned_task3
            .error_details
            .unwrap()
            .contains("Workflow 'multi-recovery-test' no longer available"));

        let failed_pipeline = dal
            .pipeline_execution()
            .get_by_id(pipeline_execution.id)
            .await
            .unwrap();
        assert_eq!(failed_pipeline.status, "Failed");

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
        assert!(!workflow_unavailable_events.is_empty());

        info!("Multiple workflow unavailable abandonment test completed successfully");
    }

    #[tokio::test]
    #[serial]
    async fn test_recovery_event_details() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline to verify recovery event details");

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

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying workflow unavailable recovery event details");

        let recovery_events = dal
            .recovery_event()
            .get_by_task(orphaned_task.id)
            .await
            .unwrap();
        assert_eq!(recovery_events.len(), 1);

        let event = &recovery_events[0];
        assert_eq!(event.recovery_type, "workflow_unavailable");
        assert_eq!(event.pipeline_execution_id, pipeline_execution.id);
        assert_eq!(event.task_execution_id, Some(orphaned_task.id));

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
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with unknown workflow");

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

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying tasks were abandoned gracefully");

        let abandoned_task1 = dal
            .task_execution()
            .get_by_id(orphaned_task1.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task1.status, "Failed");
        assert!(abandoned_task1
            .error_details
            .unwrap()
            .contains("Workflow 'unknown-workflow' no longer available"));
        assert!(abandoned_task1.completed_at.is_some());

        let abandoned_task2 = dal
            .task_execution()
            .get_by_id(orphaned_task2.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task2.status, "Failed");
        assert!(abandoned_task2
            .error_details
            .unwrap()
            .contains("Workflow 'unknown-workflow' no longer available"));
        assert!(abandoned_task2.completed_at.is_some());

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
        assert_eq!(task_events.len(), 2);

        let pipeline_events: Vec<_> = workflow_unavailable_events
            .iter()
            .filter(|e| e.task_execution_id.is_none())
            .collect();
        assert_eq!(pipeline_events.len(), 1);

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
}

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use crate::fixtures::get_or_init_sqlite_fixture;
    use cloacina::dal::DAL;
    use cloacina::database::schema::sqlite::task_executions;
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
        let fixture = get_or_init_sqlite_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with orphaned task (SQLite)");

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

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying task was abandoned due to unavailable workflow");

        let abandoned_task = dal
            .task_execution()
            .get_by_id(orphaned_task.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task.status, "Failed");
        assert!(abandoned_task
            .error_details
            .unwrap()
            .contains("Workflow 'recovery-test' no longer available"));
        assert!(abandoned_task.completed_at.is_some());

        let failed_pipeline = dal
            .pipeline_execution()
            .get_by_id(pipeline_execution.id)
            .await
            .unwrap();
        assert_eq!(failed_pipeline.status, "Failed");

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

        info!("Workflow unavailable recovery test completed successfully (SQLite)");
    }

    #[tokio::test]
    #[serial]
    async fn test_task_abandonment_after_max_retries() {
        let fixture = get_or_init_sqlite_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with task at max recovery attempts (SQLite)");

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
        // For SQLite, we need to convert UUID to bytes (BLOB format)
        let task_id_bytes = task_with_max_retries.id.0.as_bytes().to_vec();
        let conn = database.get_sqlite_connection().await.unwrap();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.filter(task_executions::id.eq(task_id_bytes)))
                .set(task_executions::recovery_attempts.eq(3))
                .execute(conn)
        })
        .await
        .unwrap()
        .unwrap();
        // IMPORTANT: Drop connection before TaskScheduler::new to avoid deadlock
        // SQLite pool has size 1, so we must return this connection before acquiring another
        drop(conn);

        info!("Creating scheduler with recovery (empty workflow registry) - should abandon task");

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying task was abandoned due to unavailable workflow");

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

        let failed_pipeline = dal
            .pipeline_execution()
            .get_by_id(pipeline_execution.id)
            .await
            .unwrap();
        assert_eq!(failed_pipeline.status, "Failed");

        let recovery_events = dal
            .recovery_event()
            .get_by_task(task_with_max_retries.id)
            .await
            .unwrap();
        assert_eq!(recovery_events.len(), 1);
        assert_eq!(recovery_events[0].recovery_type, "workflow_unavailable");

        info!("Workflow unavailable abandonment test completed successfully (SQLite)");
    }

    #[tokio::test]
    #[serial]
    async fn test_no_recovery_needed() {
        let fixture = get_or_init_sqlite_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with normal task states (SQLite)");

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

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying no recovery events were created");

        let recovery_events = dal
            .recovery_event()
            .get_by_pipeline(pipeline_execution.id)
            .await
            .unwrap();
        assert_eq!(recovery_events.len(), 0);

        info!("No recovery test completed successfully (SQLite)");
    }

    #[tokio::test]
    #[serial]
    async fn test_multiple_orphaned_tasks_recovery() {
        let fixture = get_or_init_sqlite_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with multiple orphaned tasks (SQLite)");

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
        // For SQLite, we need to convert UUID to bytes (BLOB format)
        let task_id_bytes = max_retry_task.id.0.as_bytes().to_vec();
        let conn = database.get_sqlite_connection().await.unwrap();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.filter(task_executions::id.eq(task_id_bytes)))
                .set(task_executions::recovery_attempts.eq(3))
                .execute(conn)
        })
        .await
        .unwrap()
        .unwrap();
        // IMPORTANT: Drop connection before TaskScheduler::new to avoid deadlock
        // SQLite pool has size 1, so we must return this connection before acquiring another
        drop(conn);

        info!(
            "Creating scheduler with recovery (empty workflow registry) - should abandon all tasks"
        );

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying all tasks were abandoned due to unavailable workflow");

        let abandoned_task1 = dal
            .task_execution()
            .get_by_id(orphaned_task1.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task1.status, "Failed");
        assert!(abandoned_task1
            .error_details
            .unwrap()
            .contains("Workflow 'multi-recovery-test' no longer available"));

        let abandoned_task2 = dal
            .task_execution()
            .get_by_id(orphaned_task2.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task2.status, "Failed");
        assert!(abandoned_task2
            .error_details
            .unwrap()
            .contains("Workflow 'multi-recovery-test' no longer available"));

        let abandoned_task3 = dal
            .task_execution()
            .get_by_id(max_retry_task.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task3.status, "Failed");
        assert!(abandoned_task3
            .error_details
            .unwrap()
            .contains("Workflow 'multi-recovery-test' no longer available"));

        let failed_pipeline = dal
            .pipeline_execution()
            .get_by_id(pipeline_execution.id)
            .await
            .unwrap();
        assert_eq!(failed_pipeline.status, "Failed");

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
        assert!(!workflow_unavailable_events.is_empty());

        info!("Multiple workflow unavailable abandonment test completed successfully (SQLite)");
    }

    #[tokio::test]
    #[serial]
    async fn test_recovery_event_details() {
        let fixture = get_or_init_sqlite_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline to verify recovery event details (SQLite)");

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

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying workflow unavailable recovery event details");

        let recovery_events = dal
            .recovery_event()
            .get_by_task(orphaned_task.id)
            .await
            .unwrap();
        assert_eq!(recovery_events.len(), 1);

        let event = &recovery_events[0];
        assert_eq!(event.recovery_type, "workflow_unavailable");
        assert_eq!(event.pipeline_execution_id, pipeline_execution.id);
        assert_eq!(event.task_execution_id, Some(orphaned_task.id));

        let details_str = event.details.as_ref().unwrap();
        let details: serde_json::Value = serde_json::from_str(details_str).unwrap();
        assert_eq!(details["task_name"], "detail-test-task");
        assert_eq!(details["workflow_name"], "event-details-test");
        assert_eq!(details["reason"], "Workflow not in current registry");
        assert_eq!(details["action"], "abandoned");
        assert!(details["available_workflows"].is_array());

        info!("Workflow unavailable recovery event details test completed successfully (SQLite)");
    }

    #[tokio::test]
    #[serial]
    async fn test_graceful_recovery_for_unknown_workflow() {
        let fixture = get_or_init_sqlite_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        info!("Creating test pipeline with unknown workflow (SQLite)");

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

        let _scheduler = TaskScheduler::new(database).await.unwrap();

        info!("Verifying tasks were abandoned gracefully");

        let abandoned_task1 = dal
            .task_execution()
            .get_by_id(orphaned_task1.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task1.status, "Failed");
        assert!(abandoned_task1
            .error_details
            .unwrap()
            .contains("Workflow 'unknown-workflow' no longer available"));
        assert!(abandoned_task1.completed_at.is_some());

        let abandoned_task2 = dal
            .task_execution()
            .get_by_id(orphaned_task2.id)
            .await
            .unwrap();
        assert_eq!(abandoned_task2.status, "Failed");
        assert!(abandoned_task2
            .error_details
            .unwrap()
            .contains("Workflow 'unknown-workflow' no longer available"));
        assert!(abandoned_task2.completed_at.is_some());

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
        assert_eq!(task_events.len(), 2);

        let pipeline_events: Vec<_> = workflow_unavailable_events
            .iter()
            .filter(|e| e.task_execution_id.is_none())
            .collect();
        assert_eq!(pipeline_events.len(), 1);

        let task_event = &task_events[0];
        assert_eq!(task_event.recovery_type, "workflow_unavailable");
        assert_eq!(task_event.pipeline_execution_id, pipeline_execution.id);

        let details_str = task_event.details.as_ref().unwrap();
        let details: serde_json::Value = serde_json::from_str(details_str).unwrap();
        assert_eq!(details["workflow_name"], "unknown-workflow");
        assert_eq!(details["reason"], "Workflow not in current registry");
        assert_eq!(details["action"], "abandoned");

        info!("Graceful recovery test completed successfully (SQLite)");
    }
}
