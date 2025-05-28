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

use cloacina::{task, workflow, Context, SubgraphError, TaskError};

// Define a set of tasks for subgraph testing
#[task(id = "root-task-a", dependencies = [])]
async fn root_task_a(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[task(id = "root-task-b", dependencies = [])]
async fn root_task_b(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[task(id = "middle-task-c", dependencies = ["root-task-a"])]
async fn middle_task_c(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[task(id = "middle-task-d", dependencies = ["root-task-b"])]
async fn middle_task_d(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[task(id = "final-task-e", dependencies = ["middle-task-c", "middle-task-d"])]
async fn final_task_e(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[test]
fn test_subgraph_unsupported_operation() {
    // Current implementation returns UnsupportedOperation
    let workflow = workflow! {
        name: "full-workflow",
        tasks: [
            root_task_a,
            root_task_b,
            middle_task_c,
            middle_task_d,
            final_task_e
        ]
    };

    // Attempt to create subgraph
    let result = workflow.subgraph(&["root-task-a", "middle-task-c"]);

    // Should return UnsupportedOperation error (as currently implemented)
    assert!(result.is_err());
    match result.unwrap_err() {
        SubgraphError::UnsupportedOperation(msg) => {
            assert!(msg.contains("Task cloning not supported"));
        }
        _ => panic!("Expected UnsupportedOperation error"),
    }
}

#[test]
fn test_subgraph_with_nonexistent_task() {
    let workflow = workflow! {
        name: "test-workflow",
        tasks: [root_task_a, middle_task_c]
    };

    // Try to create subgraph with non-existent task
    let result = workflow.subgraph(&["root-task-a", "nonexistent-task"]);

    assert!(result.is_err());
    match result.unwrap_err() {
        SubgraphError::TaskNotFound(task_id) => {
            assert_eq!(task_id, "nonexistent-task");
        }
        _ => panic!("Expected TaskNotFound error"),
    }
}

#[test]
fn test_subgraph_dependency_collection() {
    let workflow = workflow! {
        name: "dependency-test",
        tasks: [
            root_task_a,
            root_task_b,
            middle_task_c,
            middle_task_d,
            final_task_e
        ]
    };

    // Test that we can identify what tasks would be in a subgraph
    // by manually checking dependencies

    // For final_task_e, we should need: root_task_a, root_task_b, middle_task_c, middle_task_d, final_task_e
    let task_e = workflow.get_task("final-task-e").unwrap();
    let dependencies = task_e.dependencies();

    assert_eq!(dependencies.len(), 2);
    assert!(dependencies.contains(&"middle-task-c".to_string()));
    assert!(dependencies.contains(&"middle-task-d".to_string()));

    // Check transitive dependencies
    let task_c = workflow.get_task("middle-task-c").unwrap();
    let c_deps = task_c.dependencies();
    assert_eq!(c_deps.len(), 1);
    assert!(c_deps.contains(&"root-task-a".to_string()));

    let task_d = workflow.get_task("middle-task-d").unwrap();
    let d_deps = task_d.dependencies();
    assert_eq!(d_deps.len(), 1);
    assert!(d_deps.contains(&"root-task-b".to_string()));
}

#[test]
fn test_subgraph_metadata_operations() {
    let workflow = workflow! {
        name: "metadata-test",
        description: "Testing subgraph metadata",
        tasks: [root_task_a, middle_task_c]
    };

    // Test Workflow metadata is accessible
    assert_eq!(workflow.name(), "metadata-test");
    // Version is now auto-generated based on task content
    assert!(!workflow.metadata().version.is_empty());
    assert_eq!(
        workflow.metadata().description,
        Some("Testing subgraph metadata".to_string())
    );

    // When subgraph is implemented, it should inherit or modify metadata appropriately
    // For now, we test that the original Workflow has the expected metadata
}

#[test]
fn test_single_task_subgraph() {
    let workflow = workflow! {
        name: "single-task-workflow",
        tasks: [root_task_a]
    };

    // Test subgraph with just one task (should still fail with current implementation)
    let result = workflow.subgraph(&["root-task-a"]);

    assert!(result.is_err());
    match result.unwrap_err() {
        SubgraphError::UnsupportedOperation(_) => {
            // Expected with current implementation
        }
        SubgraphError::TaskNotFound(_) => {
            panic!("Task should exist");
        }
    }
}

#[test]
fn test_empty_subgraph_request() {
    let workflow = workflow! {
        name: "empty-subgraph-test",
        tasks: [root_task_a, middle_task_c]
    };

    // Test requesting subgraph with empty task list
    let result = workflow.subgraph(&[]);

    // Current implementation actually succeeds for empty subgraphs because:
    // 1. The for loop doesn't execute (no task_ids)
    // 2. subgraph_tasks remains empty
    // 3. The for loop over subgraph_tasks doesn't execute either
    // 4. An empty Workflow is returned successfully

    // Let's test that it succeeds and creates an empty Workflow
    assert!(
        result.is_ok(),
        "Empty subgraph should succeed but got error: {:?}",
        result.err()
    );

    let empty_workflow = result.unwrap();
    assert_eq!(empty_workflow.name(), "empty-subgraph-test-subgraph");
}

#[test]
fn test_subgraph_with_partial_dependencies() {
    // This test demonstrates that Workflow validation catches missing dependencies
    // We expect this to fail at Workflow creation time, not at subgraph creation time

    // Let's create a valid Workflow and test subgraph edge cases instead
    let valid_workflow = workflow! {
        name: "valid-workflow",
        tasks: [root_task_a, middle_task_c]
    };

    // Test requesting just the dependent task without its dependency
    let result = valid_workflow.subgraph(&["middle-task-c"]);
    assert!(result.is_err()); // Should fail due to missing dependency or unsupported op

    // Verify the Workflow itself is valid
    assert!(valid_workflow.validate().is_ok());
}
