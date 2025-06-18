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

use cloacina::{task, Context, Task, TaskError, TaskNamespace, TaskRegistry};
use serde_json::Value;

#[task(id = "macro-test-simple-task", dependencies = [])]
async fn simple_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    context
        .insert("processed", Value::Bool(true))
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("Context error: {:?}", e),
            task_id: "macro-test-simple-task".to_string(),
            timestamp: chrono::Utc::now(),
        })?;
    Ok(())
}

#[task(id = "macro-test-dependent-task", dependencies = ["macro-test-simple-task"])]
async fn dependent_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    let _processed = context
        .get("processed")
        .ok_or_else(|| TaskError::ExecutionFailed {
            message: "Missing 'processed' key".to_string(),
            task_id: "macro-test-dependent-task".to_string(),
            timestamp: chrono::Utc::now(),
        })?;

    context
        .insert("dependent_processed", Value::Bool(true))
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("Context error: {:?}", e),
            task_id: "macro-test-dependent-task".to_string(),
            timestamp: chrono::Utc::now(),
        })?;
    Ok(())
}

#[tokio::test]
async fn test_macro_generated_task() {
    // Test that the macro generates the correct task struct
    let task = simple_task_task();

    assert_eq!(task.id(), "macro-test-simple-task");
    assert_eq!(task.dependencies(), &[] as &[TaskNamespace]);

    // Test execution
    let context = Context::new();
    let result = task.execute(context).await;
    assert!(result.is_ok());

    let context = result.unwrap();
    assert_eq!(context.get("processed"), Some(&Value::Bool(true)));
}

#[tokio::test]
async fn test_macro_with_dependencies() {
    let task = dependent_task_task();

    assert_eq!(task.id(), "macro-test-dependent-task");
    // Test the static dependency IDs
    assert_eq!(
        DependentTaskTask::dependency_task_ids(),
        &["macro-test-simple-task"]
    );
    // Runtime dependencies are empty until populated by workflow builder
    assert_eq!(task.dependencies(), &[] as &[TaskNamespace]);
}

#[tokio::test]
async fn test_task_registry_with_macro_tasks() {
    let mut registry = TaskRegistry::new();

    // Register macro-generated tasks
    let ns1 = TaskNamespace::new(
        "public",
        "embedded",
        "test_workflow",
        "macro-test-simple-task",
    );
    let ns2 = TaskNamespace::new(
        "public",
        "embedded",
        "test_workflow",
        "macro-test-dependent-task",
    );

    // When testing tasks outside a workflow, we need to manually set up dependencies
    let simple_task = simple_task_task();
    let dependent_task = dependent_task_task().with_dependencies(vec![ns1.clone()]);

    registry.register(ns1.clone(), simple_task).unwrap();
    registry.register(ns2.clone(), dependent_task).unwrap();

    // Validate dependencies
    assert!(registry.validate_dependencies().is_ok());

    // Test topological sort
    let sorted = registry.topological_sort().unwrap();
    let simple_pos = sorted
        .iter()
        .position(|x| x.task_id == "macro-test-simple-task")
        .unwrap();
    let dependent_pos = sorted
        .iter()
        .position(|x| x.task_id == "macro-test-dependent-task")
        .unwrap();

    assert!(simple_pos < dependent_pos);
}

#[tokio::test]
async fn test_task_execution_flow() {
    let mut registry = TaskRegistry::new();
    let ns1 = TaskNamespace::new(
        "public",
        "embedded",
        "test_workflow",
        "macro-test-simple-task",
    );
    let ns2 = TaskNamespace::new(
        "public",
        "embedded",
        "test_workflow",
        "macro-test-dependent-task",
    );

    // When testing tasks outside a workflow, we need to manually set up dependencies
    let simple_task = simple_task_task();
    let dependent_task = dependent_task_task().with_dependencies(vec![ns1.clone()]);

    registry.register(ns1.clone(), simple_task).unwrap();
    registry.register(ns2.clone(), dependent_task).unwrap();

    let sorted = registry.topological_sort().unwrap();
    let mut context = Context::new();

    // Execute tasks in dependency order
    for task_ns in sorted {
        let task = registry.get_task(&task_ns).unwrap();
        context = task.execute(context).await.unwrap();
    }

    // Verify both tasks processed successfully
    assert_eq!(context.get("processed"), Some(&Value::Bool(true)));
    assert_eq!(context.get("dependent_processed"), Some(&Value::Bool(true)));
}

// Test original function is still available
#[tokio::test]
async fn test_original_function_available() {
    let mut context = Context::new();
    let result = simple_task(&mut context).await;
    assert!(result.is_ok());
    assert_eq!(context.get("processed"), Some(&Value::Bool(true)));
}
