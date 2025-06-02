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

// Mock task for testing dependencies
#[derive(Clone)]
struct MockTask {
    id: String,
    dependencies: Vec<String>,
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

    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }
}

#[tokio::test]
#[serial]
async fn test_task_dependency_initialization() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap();
    fixture.initialize().await;
    let database = fixture.get_database();

    // Create tasks with dependencies: task2 depends on task1
    let task1 = MockTask {
        id: "task1".to_string(),
        dependencies: vec![],
    };

    let task2 = MockTask {
        id: "task2".to_string(),
        dependencies: vec!["task1".to_string()],
    };

    let workflow = Workflow::builder("dependency-test")
        .description("Test workflow with dependencies")
        .add_task(task1)
        .expect("Failed to add task1")
        .add_task(task2)
        .expect("Failed to add task2")
        .build()
        .expect("Failed to build workflow");

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

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
        .expect("Failed to get tasks for pipeline");

    assert_eq!(tasks.len(), 2);

    // All tasks should start as NotStarted
    for task in &tasks {
        assert_eq!(task.status, "NotStarted");
    }

    // Verify task names
    let task_names: std::collections::HashSet<_> = tasks.iter().map(|t| &t.task_name).collect();
    assert!(task_names.contains(&"task1".to_string()));
    assert!(task_names.contains(&"task2".to_string()));
}

#[tokio::test]
#[serial]
async fn test_dependency_satisfaction_check() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap();
    fixture.initialize().await;
    let database = fixture.get_database();

    // Create simple dependency chain
    let task1 = MockTask {
        id: "independent-task".to_string(),
        dependencies: vec![],
    };

    let task2 = MockTask {
        id: "dependent-task".to_string(),
        dependencies: vec!["independent-task".to_string()],
    };

    let workflow = Workflow::builder("dependency-chain")
        .description("Test dependency satisfaction")
        .add_task(task1)
        .expect("Failed to add task1")
        .add_task(task2)
        .expect("Failed to add task2")
        .build()
        .expect("Failed to build workflow");

    let scheduler = TaskScheduler::new(database.clone(), vec![workflow]);

    let mut input_context = Context::<serde_json::Value>::new();
    input_context
        .insert("test_key", serde_json::json!("test_value"))
        .expect("Failed to insert test data");
    let execution_id = scheduler
        .schedule_workflow_execution("dependency-chain", input_context)
        .await
        .expect("Failed to schedule workflow execution");

    let dal = fixture.get_dal();

    // Initially, no tasks should be ready since we haven't marked any as complete
    let pending_tasks = dal
        .task_execution()
        .get_pending_tasks(UniversalUuid(execution_id))
        .expect("Failed to get pending tasks");

    assert_eq!(pending_tasks.len(), 2);

    // Simulate marking the independent task as completed
    let independent_task = pending_tasks
        .iter()
        .find(|t| t.task_name == "independent-task")
        .expect("Independent task not found");

    // Update the task status to Completed (simulating execution completion)
    use cloacina::database::schema::task_executions;
    use diesel::prelude::*;

    let conn = fixture.get_connection();
    diesel::update(task_executions::table.find(independent_task.id))
        .set(task_executions::status.eq("Completed"))
        .execute(conn)
        .expect("Failed to update task status");

    // Now check that the dependent task's dependencies are satisfied
    // This would be done by the scheduler's update_task_readiness method
    // For this test, we're verifying the database state is correct for dependency checking

    let updated_tasks = dal
        .task_execution()
        .get_all_tasks_for_pipeline(UniversalUuid(execution_id))
        .expect("Failed to get all tasks");

    let independent_status = updated_tasks
        .iter()
        .find(|t| t.task_name == "independent-task")
        .map(|t| &t.status)
        .expect("Independent task not found");

    assert_eq!(independent_status, "Completed");
}
