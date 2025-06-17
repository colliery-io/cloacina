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
use async_trait::async_trait;
use cloacina::task_scheduler::TaskScheduler;
use cloacina::*;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

// Simple mock task for testing
#[derive(Clone)]
struct SimpleTask {
    id: String,
}

#[async_trait]
impl Task for SimpleTask {
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        Ok(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &[]
    }
}

#[tokio::test]
#[serial]
async fn test_schedule_workflow_execution() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    let database = fixture.get_database();

    // Create a simple workflow with a task
    let simple_task = SimpleTask {
        id: "test-task".to_string(),
    };
    let workflow = Workflow::builder("test-workflow")
        .description("Test workflow for scheduling")
        .add_task(Arc::new(simple_task))
        .expect("Failed to add task")
        .build()
        .expect("Failed to build workflow");

    // Register workflow globally before creating scheduler
    cloacina::register_workflow_constructor("test-workflow".to_string(), move || workflow.clone());

    let scheduler = TaskScheduler::new(database.clone())
        .await
        .expect("Failed to create scheduler");

    let mut input_context = Context::<serde_json::Value>::new();
    input_context
        .insert("test_key", serde_json::json!("test_value"))
        .expect("Failed to insert test data");
    let execution_id = scheduler
        .schedule_workflow_execution("test-workflow", input_context)
        .await
        .expect("Failed to schedule workflow execution");

    assert_ne!(execution_id, Uuid::nil());

    // Verify pipeline execution was created
    let dal = fixture.get_dal();
    let pipeline = dal
        .pipeline_execution()
        .get_by_id(UniversalUuid(execution_id))
        .await
        .expect("Failed to get pipeline execution");

    assert_eq!(pipeline.pipeline_name, "test-workflow");
    assert_eq!(pipeline.status, "Pending");
}

#[tokio::test]
#[serial]
async fn test_schedule_nonexistent_workflow() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    let database = fixture.get_database();

    let scheduler = TaskScheduler::new(database)
        .await
        .expect("Failed to create scheduler");

    let mut input_context = Context::<serde_json::Value>::new();
    input_context
        .insert("test_key", serde_json::json!("test_value"))
        .expect("Failed to insert test data");
    let result = scheduler
        .schedule_workflow_execution("nonexistent-workflow", input_context)
        .await;

    assert!(result.is_err());
    if let Err(ValidationError::WorkflowNotFound(name)) = result {
        assert_eq!(name, "nonexistent-workflow");
    } else {
        panic!("Expected WorkflowNotFound error");
    }
}

#[tokio::test]
#[serial]
async fn test_workflow_version_tracking() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    let database = fixture.get_database();

    // Create workflow with auto-versioning
    let simple_task = SimpleTask {
        id: "version-task".to_string(),
    };
    let workflow = Workflow::builder("versioned-workflow")
        .description("Workflow with version tracking")
        .add_task(Arc::new(simple_task))
        .expect("Failed to add task")
        .build()
        .expect("Failed to build workflow");

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor("versioned-workflow".to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    let scheduler = TaskScheduler::new(database.clone()).await.unwrap();

    let mut input_context = Context::<serde_json::Value>::new();
    input_context
        .insert("test_key", serde_json::json!("test_value"))
        .expect("Failed to insert test data");
    let execution_id = scheduler
        .schedule_workflow_execution("versioned-workflow", input_context)
        .await
        .expect("Failed to schedule workflow execution");

    // Verify version was stored correctly
    let dal = fixture.get_dal();
    let pipeline = dal
        .pipeline_execution()
        .get_by_id(UniversalUuid(execution_id))
        .await
        .expect("Failed to get pipeline execution");

    // Since we're using auto-versioning, just verify a version was set
    assert!(!pipeline.pipeline_version.is_empty());
}
