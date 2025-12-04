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

use cloacina::{ContextError, SubgraphError, TaskError, ValidationError, WorkflowError};

#[test]
fn test_context_error_display() {
    let key_not_found = ContextError::KeyNotFound("test_key".to_string());
    assert_eq!(format!("{}", key_not_found), "Key not found: test_key");

    let key_exists = ContextError::KeyExists("duplicate_key".to_string());
    assert_eq!(
        format!("{}", key_exists),
        "Key already exists: duplicate_key"
    );

    let type_mismatch = ContextError::TypeMismatch("wrong_type_key".to_string());
    assert_eq!(
        format!("{}", type_mismatch),
        "Type mismatch for key wrong_type_key"
    );
}

#[test]
fn test_task_error_display() {
    let execution_failed = TaskError::ExecutionFailed {
        message: "Task failed to execute".to_string(),
        task_id: "test-task".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let display_msg = format!("{}", execution_failed);
    assert!(display_msg.contains("Task execution failed: Task failed to execute"));

    let dependency_not_satisfied = TaskError::DependencyNotSatisfied {
        dependency: "required-task".to_string(),
        task_id: "dependent-task".to_string(),
    };

    let display_msg = format!("{}", dependency_not_satisfied);
    assert_eq!(
        display_msg,
        "Task dependency not satisfied: required-task required by dependent-task"
    );
}

#[test]
fn test_validation_error_display() {
    let missing_dependency = ValidationError::MissingDependency {
        task: "task1".to_string(),
        dependency: "missing-dep".to_string(),
    };

    let display_msg = format!("{}", missing_dependency);
    assert_eq!(
        display_msg,
        "Missing dependency: task 'task1' depends on 'missing-dep' which is not registered"
    );

    let cyclic_dependency = ValidationError::CyclicDependency {
        cycle: vec![
            "task1".to_string(),
            "task2".to_string(),
            "task1".to_string(),
        ],
    };

    let display_msg = format!("{}", cyclic_dependency);
    assert!(display_msg.contains("Circular dependency detected"));
}

#[test]
fn test_workflow_error_display() {
    let duplicate_task = WorkflowError::DuplicateTask("duplicate-task".to_string());
    assert_eq!(
        format!("{}", duplicate_task),
        "Duplicate task: duplicate-task"
    );

    let task_not_found = WorkflowError::TaskNotFound("missing-task".to_string());
    assert_eq!(
        format!("{}", task_not_found),
        "Task not found: missing-task"
    );

    let cyclic_dependency =
        WorkflowError::CyclicDependency(vec!["task1".to_string(), "task2".to_string()]);
    let display_msg = format!("{}", cyclic_dependency);
    assert!(display_msg.contains("Cyclic dependency"));
}

#[test]
fn test_subgraph_error_display() {
    let task_not_found = SubgraphError::TaskNotFound("missing-task".to_string());
    assert_eq!(
        format!("{}", task_not_found),
        "Task not found: missing-task"
    );

    let unsupported_operation = SubgraphError::UnsupportedOperation("Not implemented".to_string());
    assert_eq!(
        format!("{}", unsupported_operation),
        "Unsupported operation: Not implemented"
    );
}

#[test]
fn test_error_source_chains() {
    // Test that errors can be chained properly
    let json_error = serde_json::from_str::<i32>("invalid json").unwrap_err();
    let context_error = ContextError::Serialization(json_error);

    // Verify the error chain
    assert!(format!("{}", context_error).contains("Serialization error"));

    // Test source chain
    let source = std::error::Error::source(&context_error);
    assert!(source.is_some());
}

#[test]
fn test_error_debug_formatting() {
    let error = ValidationError::EmptyWorkflow;
    let debug_str = format!("{:?}", error);
    assert_eq!(debug_str, "EmptyWorkflow");

    let cyclic_error = ValidationError::CyclicDependency {
        cycle: vec!["a".to_string(), "b".to_string()],
    };
    let debug_str = format!("{:?}", cyclic_error);
    assert!(debug_str.contains("CyclicDependency"));
    assert!(debug_str.contains("cycle"));
}
