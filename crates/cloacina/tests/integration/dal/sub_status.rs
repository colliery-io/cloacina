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

//! Integration tests for the `sub_status` column on task executions.
//!
//! Verifies that `set_sub_status()` correctly persists Active, Deferred,
//! and None values, and that the full lifecycle (Active → Deferred → Active → None)
//! works end-to-end.
//!
//! Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.

use crate::fixtures::get_all_fixtures;
use cloacina::dal::DAL;
use cloacina::models::pipeline_execution::NewPipelineExecution;
use cloacina::models::task_execution::NewTaskExecution;
use serde_json::json;

/// Tests all sub_status operations in a single test to avoid fixture contention.
///
/// Verifies:
/// - Setting sub_status to "Active"
/// - Setting sub_status to "Deferred"
/// - Clearing sub_status to None
/// - Full lifecycle: None → Active → Deferred → Active → None
#[tokio::test]
async fn test_sub_status_crud_operations() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!("Running test_sub_status_crud_operations on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        // Create a pipeline and task
        let pipeline = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "sub-status-test".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Running".to_string(),
                context_id: None,
            })
            .await
            .expect("Failed to create pipeline");

        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline.id,
                task_name: "sub-status-test-task".to_string(),
                status: "Running".to_string(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: json!({"type": "Always"}).to_string(),
                task_configuration: json!({}).to_string(),
            })
            .await
            .expect("Failed to create task");

        // Verify initial sub_status is None
        let initial = dal
            .task_execution()
            .get_by_id(task.id)
            .await
            .expect("Failed to get task");
        assert_eq!(
            initial.sub_status, None,
            "[{}] Initial sub_status should be None",
            backend
        );

        // Test 1: Set sub_status to "Active"
        dal.task_execution()
            .set_sub_status(task.id, Some("Active"))
            .await
            .expect("Failed to set sub_status to Active");

        let after_active = dal
            .task_execution()
            .get_by_id(task.id)
            .await
            .expect("Failed to get task");
        assert_eq!(
            after_active.sub_status,
            Some("Active".to_string()),
            "[{}] sub_status should be 'Active'",
            backend
        );

        // Test 2: Set sub_status to "Deferred"
        dal.task_execution()
            .set_sub_status(task.id, Some("Deferred"))
            .await
            .expect("Failed to set sub_status to Deferred");

        let after_deferred = dal
            .task_execution()
            .get_by_id(task.id)
            .await
            .expect("Failed to get task");
        assert_eq!(
            after_deferred.sub_status,
            Some("Deferred".to_string()),
            "[{}] sub_status should be 'Deferred'",
            backend
        );

        // Test 3: Set back to "Active"
        dal.task_execution()
            .set_sub_status(task.id, Some("Active"))
            .await
            .expect("Failed to set sub_status back to Active");

        let after_active_again = dal
            .task_execution()
            .get_by_id(task.id)
            .await
            .expect("Failed to get task");
        assert_eq!(
            after_active_again.sub_status,
            Some("Active".to_string()),
            "[{}] sub_status should be 'Active' again",
            backend
        );

        // Test 4: Clear sub_status to None
        dal.task_execution()
            .set_sub_status(task.id, None)
            .await
            .expect("Failed to clear sub_status");

        let after_clear = dal
            .task_execution()
            .get_by_id(task.id)
            .await
            .expect("Failed to get task");
        assert_eq!(
            after_clear.sub_status, None,
            "[{}] sub_status should be None after clearing",
            backend
        );

        tracing::info!("test_sub_status_crud_operations passed on {}", backend);
    }
}
