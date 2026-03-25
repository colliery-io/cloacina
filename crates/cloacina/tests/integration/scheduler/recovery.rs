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

//! Recovery sweep tests — heartbeat-based orphan detection and task re-queuing.
//!
//! These tests exercise `RecoverySweepService::perform_sweep()` against both
//! PostgreSQL and SQLite backends. Tasks with stale heartbeats are recovered
//! (reset to Ready) or abandoned (marked Failed) based on recovery_attempts.

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_postgres_fixture;
    use cloacina::dal::DAL;
    use cloacina::models::pipeline_execution::NewPipelineExecution;
    use cloacina::models::task_execution::NewTaskExecution;
    use cloacina::recovery_sweep::{RecoverySweepConfig, RecoverySweepService};
    use serial_test::serial;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::watch;
    use tracing::info;

    /// Create a sweeper with tight timing for tests.
    fn test_sweeper(dal: Arc<DAL>, shutdown_rx: watch::Receiver<bool>) -> RecoverySweepService {
        RecoverySweepService::new(
            dal,
            RecoverySweepConfig {
                sweep_interval: Duration::from_millis(100),
                orphan_threshold: Duration::from_secs(0), // anything stale is orphaned
                startup_grace: Duration::from_secs(0),    // skip grace period
                max_recovery_attempts: 3,
            },
            shutdown_rx,
        )
    }

    /// Set a task to Running with a stale heartbeat in the past.
    async fn make_task_stale(
        database: &cloacina::Database,
        task_id: cloacina::database::universal_types::UniversalUuid,
    ) {
        use diesel::RunQueryDsl;
        let tid = task_id.0.to_string();
        let conn = database.get_postgres_connection().await.unwrap();
        conn.interact(move |conn| {
            diesel::sql_query(format!(
                "UPDATE task_executions SET status = 'Running', heartbeat_at = NOW() - INTERVAL '5 minutes' WHERE id = '{}'",
                tid
            ))
            .execute(conn)
        })
        .await
        .unwrap()
        .unwrap();
    }

    /// Set recovery_attempts on a task.
    async fn set_recovery_attempts(
        database: &cloacina::Database,
        task_id: cloacina::database::universal_types::UniversalUuid,
        attempts: i32,
    ) {
        use diesel::RunQueryDsl;
        let tid = task_id.0.to_string();
        let conn = database.get_postgres_connection().await.unwrap();
        conn.interact(move |conn| {
            diesel::sql_query(format!(
                "UPDATE task_executions SET recovery_attempts = {} WHERE id = '{}'",
                attempts, tid
            ))
            .execute(conn)
        })
        .await
        .unwrap()
        .unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_orphaned_task_recovery() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        info!("Creating pipeline with orphaned task (stale heartbeat)");

        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "recovery-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .unwrap();

        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "orphaned-task".to_string(),
                status: "Running".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: serde_json::json!({"type": "Always"}).to_string(),
                task_configuration: serde_json::json!({}).to_string(),
            })
            .await
            .unwrap();

        make_task_stale(&database, task.id).await;

        info!("Running recovery sweep");
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let sweeper = test_sweeper(dal.clone(), shutdown_rx);
        sweeper.perform_sweep_public().await.unwrap();

        info!("Verifying task was reset to Ready for re-execution");
        let recovered = dal.task_execution().get_by_id(task.id).await.unwrap();
        assert_eq!(
            recovered.status, "Ready",
            "Orphaned task should be reset to Ready"
        );

        info!("Orphaned task recovery test passed");
    }

    #[tokio::test]
    #[serial]
    async fn test_task_abandonment_after_max_retries() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        info!("Creating task at max recovery attempts with stale heartbeat");

        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "abandonment-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .unwrap();

        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "max-retry-task".to_string(),
                status: "Running".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: serde_json::json!({"type": "Always"}).to_string(),
                task_configuration: serde_json::json!({}).to_string(),
            })
            .await
            .unwrap();

        make_task_stale(&database, task.id).await;
        set_recovery_attempts(&database, task.id, 3).await;

        info!("Running recovery sweep — should abandon task");
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let sweeper = test_sweeper(dal.clone(), shutdown_rx);
        sweeper.perform_sweep_public().await.unwrap();

        let abandoned = dal.task_execution().get_by_id(task.id).await.unwrap();
        assert_eq!(
            abandoned.status, "Failed",
            "Task should be abandoned (Failed)"
        );
        assert!(
            abandoned.last_error.as_ref().unwrap().contains("ABANDONED"),
            "last_error should mention ABANDONED, got: {:?}",
            abandoned.last_error
        );

        info!("Task abandonment test passed");
    }

    #[tokio::test]
    #[serial]
    async fn test_no_recovery_needed() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        info!("Creating tasks in non-orphaned states");

        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "no-recovery-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Completed".to_string(),
                context_id: None,
            })
            .await
            .unwrap();

        let completed = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "completed-task".to_string(),
                status: "Completed".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: serde_json::json!({"type": "Always"}).to_string(),
                task_configuration: serde_json::json!({}).to_string(),
            })
            .await
            .unwrap();

        let ready = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "ready-task".to_string(),
                status: "Ready".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: serde_json::json!({"type": "Always"}).to_string(),
                task_configuration: serde_json::json!({}).to_string(),
            })
            .await
            .unwrap();

        info!("Running recovery sweep — should find no orphans");
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let sweeper = test_sweeper(dal.clone(), shutdown_rx);
        sweeper.perform_sweep_public().await.unwrap();

        // Verify states unchanged
        let c = dal.task_execution().get_by_id(completed.id).await.unwrap();
        assert_eq!(c.status, "Completed");
        let r = dal.task_execution().get_by_id(ready.id).await.unwrap();
        assert_eq!(r.status, "Ready");

        info!("No recovery needed test passed");
    }

    #[tokio::test]
    #[serial]
    async fn test_multiple_orphaned_tasks_recovery() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        info!("Creating pipeline with multiple orphaned tasks");

        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "multi-recovery-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .unwrap();

        let task1 = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "orphan-1".to_string(),
                status: "Running".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: serde_json::json!({"type": "Always"}).to_string(),
                task_configuration: serde_json::json!({}).to_string(),
            })
            .await
            .unwrap();

        let task2 = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "orphan-2".to_string(),
                status: "Running".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: serde_json::json!({"type": "Always"}).to_string(),
                task_configuration: serde_json::json!({}).to_string(),
            })
            .await
            .unwrap();

        let task3_max = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "max-retry".to_string(),
                status: "Running".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: serde_json::json!({"type": "Always"}).to_string(),
                task_configuration: serde_json::json!({}).to_string(),
            })
            .await
            .unwrap();

        // Make all stale
        make_task_stale(&database, task1.id).await;
        make_task_stale(&database, task2.id).await;
        make_task_stale(&database, task3_max.id).await;
        set_recovery_attempts(&database, task3_max.id, 3).await;

        info!("Running recovery sweep — should recover 2, abandon 1");
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let sweeper = test_sweeper(dal.clone(), shutdown_rx);
        sweeper.perform_sweep_public().await.unwrap();

        let t1 = dal.task_execution().get_by_id(task1.id).await.unwrap();
        assert_eq!(t1.status, "Ready", "Task 1 should be recovered to Ready");

        let t2 = dal.task_execution().get_by_id(task2.id).await.unwrap();
        assert_eq!(t2.status, "Ready", "Task 2 should be recovered to Ready");

        let t3 = dal.task_execution().get_by_id(task3_max.id).await.unwrap();
        assert_eq!(t3.status, "Failed", "Task 3 should be abandoned (Failed)");
        assert!(
            t3.last_error.as_ref().unwrap().contains("ABANDONED"),
            "last_error should mention ABANDONED, got: {:?}",
            t3.last_error
        );

        info!("Multiple orphaned tasks recovery test passed");
    }

    #[tokio::test]
    #[serial]
    async fn test_recovery_sweep_respects_fresh_heartbeats() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.initialize().await;
        let database = guard.get_database();
        let dal = Arc::new(DAL::new(database.clone()));

        info!("Creating Running task with FRESH heartbeat (should NOT be recovered)");

        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "fresh-heartbeat-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .unwrap();

        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "active-task".to_string(),
                status: "Running".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: serde_json::json!({"type": "Always"}).to_string(),
                task_configuration: serde_json::json!({}).to_string(),
            })
            .await
            .unwrap();

        // Set heartbeat to NOW (fresh — should NOT be recovered)
        {
            use diesel::RunQueryDsl;
            let tid = task.id.0.to_string();
            let conn = database.get_postgres_connection().await.unwrap();
            conn.interact(move |conn| {
                diesel::sql_query(format!(
                    "UPDATE task_executions SET heartbeat_at = NOW() WHERE id = '{}'",
                    tid
                ))
                .execute(conn)
            })
            .await
            .unwrap()
            .unwrap();
        }

        // Use a sweeper with 60s orphan threshold (task heartbeat is fresh)
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let sweeper = RecoverySweepService::new(
            dal.clone(),
            RecoverySweepConfig {
                sweep_interval: Duration::from_millis(100),
                orphan_threshold: Duration::from_secs(60),
                startup_grace: Duration::from_secs(0),
                max_recovery_attempts: 3,
            },
            shutdown_rx,
        );
        sweeper.perform_sweep_public().await.unwrap();

        let still_running = dal.task_execution().get_by_id(task.id).await.unwrap();
        assert_eq!(
            still_running.status, "Running",
            "Task with fresh heartbeat should NOT be recovered"
        );

        info!("Fresh heartbeat test passed");
    }
}
