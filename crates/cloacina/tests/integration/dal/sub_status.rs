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

use cloacina::dal::DAL;
use cloacina::models::pipeline_execution::NewPipelineExecution;
use cloacina::models::task_execution::NewTaskExecution;
use serde_json::json;

#[cfg(feature = "sqlite")]
use crate::fixtures::get_or_init_sqlite_fixture;

/// Tests all sub_status operations in a single test to avoid fixture contention.
///
/// Verifies:
/// - Setting sub_status to "Active"
/// - Setting sub_status to "Deferred"
/// - Clearing sub_status to None
/// - Full lifecycle: None → Active → Deferred → Active → None
#[cfg(feature = "sqlite")]
#[tokio::test]
async fn test_sub_status_crud_operations() {
    let fixture = get_or_init_sqlite_fixture().await;
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
            task_name: "sub-status-task".to_string(),
            status: "Running".to_string(),
            attempt: 1,
            max_attempts: 3,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .expect("Failed to create task");

    let task_id = task.id;

    // 1. Initially sub_status should be None
    let task = dal.task_execution().get_by_id(task_id).await.unwrap();
    assert_eq!(task.sub_status, None, "Initial sub_status should be None");

    // 2. Set to "Active"
    dal.task_execution()
        .set_sub_status(task_id, Some("Active"))
        .await
        .expect("Failed to set sub_status to Active");

    let task = dal.task_execution().get_by_id(task_id).await.unwrap();
    assert_eq!(
        task.sub_status,
        Some("Active".to_string()),
        "sub_status should be Active"
    );

    // 3. Transition to "Deferred"
    dal.task_execution()
        .set_sub_status(task_id, Some("Deferred"))
        .await
        .expect("Failed to set sub_status to Deferred");

    let task = dal.task_execution().get_by_id(task_id).await.unwrap();
    assert_eq!(
        task.sub_status,
        Some("Deferred".to_string()),
        "sub_status should be Deferred"
    );

    // 4. Back to "Active"
    dal.task_execution()
        .set_sub_status(task_id, Some("Active"))
        .await
        .expect("Failed to set sub_status back to Active");

    let task = dal.task_execution().get_by_id(task_id).await.unwrap();
    assert_eq!(
        task.sub_status,
        Some("Active".to_string()),
        "sub_status should be Active again"
    );

    // 5. Clear to None (simulating task completion)
    dal.task_execution()
        .set_sub_status(task_id, None)
        .await
        .expect("Failed to clear sub_status");

    let task = dal.task_execution().get_by_id(task_id).await.unwrap();
    assert_eq!(
        task.sub_status, None,
        "sub_status should be None after clearing"
    );
}
