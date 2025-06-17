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

// Mock task for testing dependencies
#[derive(Clone)]
struct MockTask {
    id: String,
    dependencies: Vec<TaskNamespace>,
}

#[async_trait]
impl Task for MockTask {
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        // Mock execution - just return the context
        Ok(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }
}

#[tokio::test]
#[serial]
async fn test_task_dependency_initialization() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    // Reset the database to ensure a clean state
    fixture.reset_database().await;

    let database = fixture.get_database();

    // Create tasks with dependencies: task2 depends on task1
    let task1 = MockTask {
        id: "task1".to_string(),
        dependencies: vec![],
    };

    let task1_ns = TaskNamespace::new("public", "embedded", "dependency-test", "task1");
    let task2 = MockTask {
        id: "task2".to_string(),
        dependencies: vec![task1_ns],
    };

    let workflow = Workflow::builder("dependency-test")
        .description("Test workflow with dependencies")
        .add_task(Arc::new(task1))
        .expect("Failed to add task1")
        .add_task(Arc::new(task2))
        .expect("Failed to add task2")
        .build()
        .expect("Failed to build workflow");

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor("dependency-test".to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    let scheduler = TaskScheduler::new(database.clone()).await.unwrap();

    let mut input_context = Context::<serde_json::Value>::new();
    input_context
        .insert("test_key", serde_json::json!("test_value"))
        .expect("Failed to insert test data");
    let execution_id = scheduler
        .schedule_workflow_execution("dependency-test", input_context)
        .await
        .expect("Failed to schedule workflow execution");

    // Verify both tasks were initialized
    let dal = fixture.get_dal();
    let tasks = dal
        .task_execution()
        .get_all_tasks_for_pipeline(UniversalUuid(execution_id))
        .await
        .expect("Failed to get tasks for pipeline");

    assert_eq!(tasks.len(), 2);

    // All tasks should start as NotStarted
    for task in &tasks {
        assert_eq!(task.status, "NotStarted");
    }

    // Verify task names (should be full namespaces now)
    let task_names: std::collections::HashSet<_> = tasks.iter().map(|t| &t.task_name).collect();
    let expected_task1 = format!(
        "{}::{}::{}::task1",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    let expected_task2 = format!(
        "{}::{}::{}::task2",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    assert!(task_names.contains(&expected_task1));
    assert!(task_names.contains(&expected_task2));
}

#[tokio::test]
#[serial]
async fn test_dependency_satisfaction_check() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    // Reset the database to ensure a clean state
    fixture.reset_database().await;

    let database = fixture.get_database();

    // Create tasks with dependencies: task2 depends on task1
    let task1 = MockTask {
        id: "task1".to_string(),
        dependencies: vec![],
    };

    let task1_ns = TaskNamespace::new("public", "embedded", "dependency-chain", "task1");
    let task2 = MockTask {
        id: "task2".to_string(),
        dependencies: vec![task1_ns],
    };

    let workflow = Workflow::builder("dependency-chain")
        .description("Test dependency satisfaction")
        .add_task(Arc::new(task1))
        .expect("Failed to add task1")
        .add_task(Arc::new(task2))
        .expect("Failed to add task2")
        .build()
        .expect("Failed to build workflow");

    // Register workflow in global registry for scheduler to find
    register_workflow_constructor("dependency-chain".to_string(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    let scheduler = TaskScheduler::new(database.clone()).await.unwrap();

    let mut input_context = Context::<serde_json::Value>::new();
    input_context
        .insert("test_key", serde_json::json!("test_value"))
        .expect("Failed to insert test data");
    let execution_id = scheduler
        .schedule_workflow_execution("dependency-chain", input_context)
        .await
        .expect("Failed to schedule workflow execution");

    // Verify both tasks were initialized
    let dal = fixture.get_dal();
    let tasks = dal
        .task_execution()
        .get_all_tasks_for_pipeline(UniversalUuid(execution_id))
        .await
        .expect("Failed to get tasks for pipeline");

    assert_eq!(tasks.len(), 2);

    // All tasks should start as NotStarted
    for task in &tasks {
        assert_eq!(task.status, "NotStarted");
    }

    // Verify task names (should be full namespaces now)
    let task_names: std::collections::HashSet<_> = tasks.iter().map(|t| &t.task_name).collect();
    let expected_task1 = format!(
        "{}::{}::{}::task1",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    let expected_task2 = format!(
        "{}::{}::{}::task2",
        workflow.tenant(),
        workflow.package(),
        workflow.name()
    );
    assert!(task_names.contains(&expected_task1));
    assert!(task_names.contains(&expected_task2));
}
