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

use cloacina::{task, workflow, Context, TaskError, TaskNamespace};

// Define some test tasks using the task macro
#[task(id = "fetch-document", dependencies = [])]
async fn fetch_document(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Simulate fetching a document
    Ok(())
}

#[task(id = "extract-text", dependencies = ["fetch-document"])]
async fn extract_text(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Simulate text extraction
    Ok(())
}

#[task(id = "generate-embeddings", dependencies = ["extract-text"])]
async fn generate_embeddings(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Simulate embedding generation
    Ok(())
}

#[task(id = "store-embeddings", dependencies = ["generate-embeddings"])]
async fn store_embeddings(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Simulate storing embeddings
    Ok(())
}

#[test]
fn test_workflow_macro_basic() {
    // Initialize logging for test
    let _ = tracing_subscriber::fmt::try_init();

    // Create a Workflow using the workflow! macro
    let document_processing_workflow = workflow! {
        name: "document-processing",
        description: "Process documents into knowledge base",
        tasks: [
            fetch_document,
            extract_text,
            generate_embeddings,
            store_embeddings
        ]
    };

    // Verify Workflow properties
    assert_eq!(document_processing_workflow.name(), "document-processing");
    // Version is now auto-generated based on task content
    assert!(!document_processing_workflow.metadata().version.is_empty());
    assert_eq!(
        document_processing_workflow.metadata().description,
        Some("Process documents into knowledge base".to_string())
    );

    // Verify all tasks are present
    let fetch_ns = TaskNamespace::new(
        "public",
        "embedded",
        "document-processing",
        "fetch-document",
    );
    let extract_ns =
        TaskNamespace::new("public", "embedded", "document-processing", "extract-text");
    let embeddings_ns = TaskNamespace::new(
        "public",
        "embedded",
        "document-processing",
        "generate-embeddings",
    );

    assert!(document_processing_workflow.get_task(&fetch_ns).is_ok());
    assert!(document_processing_workflow.get_task(&extract_ns).is_ok());
    assert!(document_processing_workflow
        .get_task(&embeddings_ns)
        .is_ok());
    let store_ns = TaskNamespace::new(
        "public",
        "embedded",
        "document-processing",
        "store-embeddings",
    );
    assert!(document_processing_workflow.get_task(&store_ns).is_ok());

    // Verify topological order
    let execution_order = document_processing_workflow.topological_sort().unwrap();

    let fetch_pos = execution_order
        .iter()
        .position(|x| x.task_id == "fetch-document")
        .unwrap();
    let extract_pos = execution_order
        .iter()
        .position(|x| x.task_id == "extract-text")
        .unwrap();
    let embed_pos = execution_order
        .iter()
        .position(|x| x.task_id == "generate-embeddings")
        .unwrap();
    let store_pos = execution_order
        .iter()
        .position(|x| x.task_id == "store-embeddings")
        .unwrap();

    assert!(fetch_pos < extract_pos);
    assert!(extract_pos < embed_pos);
    assert!(embed_pos < store_pos);
}

#[test]
fn test_workflow_macro_minimal() {
    // Initialize logging for test
    let _ = tracing_subscriber::fmt::try_init();

    // Test minimal Workflow with just name and tasks
    let simple_workflow = workflow! {
        name: "simple-pipeline",
        tasks: [fetch_document]
    };

    assert_eq!(simple_workflow.name(), "simple-pipeline");
    // Version is now auto-generated based on task content
    assert!(!simple_workflow.metadata().version.is_empty());
    let fetch_ns = TaskNamespace::new("public", "embedded", "simple-pipeline", "fetch-document");
    assert!(simple_workflow.get_task(&fetch_ns).is_ok());
}

// Define parallel tasks for testing execution levels
#[task(id = "task-a", dependencies = [])]
async fn task_a(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[task(id = "task-b", dependencies = [])]
async fn task_b(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[task(id = "task-c", dependencies = ["task-a", "task-b"])]
async fn task_c(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[test]
fn test_workflow_execution_levels() {
    // Initialize logging for test
    let _ = tracing_subscriber::fmt::try_init();

    let parallel_workflow = workflow! {
        name: "parallel-execution",
        tasks: [task_a, task_b, task_c]
    };

    let execution_levels = parallel_workflow.get_execution_levels().unwrap();

    // Level 0: task_a and task_b (can run in parallel)
    assert_eq!(execution_levels[0].len(), 2);
    let task_a_ns = TaskNamespace::new("public", "embedded", "parallel-execution", "task-a");
    let task_b_ns = TaskNamespace::new("public", "embedded", "parallel-execution", "task-b");
    let task_c_ns = TaskNamespace::new("public", "embedded", "parallel-execution", "task-c");
    assert!(execution_levels[0].contains(&task_a_ns));
    assert!(execution_levels[0].contains(&task_b_ns));

    // Level 1: task_c (depends on both task_a and task_b)
    assert_eq!(execution_levels[1].len(), 1);
    assert!(execution_levels[1].contains(&task_c_ns));

    // Verify parallel execution capability
    assert!(parallel_workflow.can_run_parallel(&task_a_ns, &task_b_ns));
    assert!(!parallel_workflow.can_run_parallel(&task_a_ns, &task_c_ns));
    assert!(!parallel_workflow.can_run_parallel(&task_b_ns, &task_c_ns));
}

#[test]
fn test_workflow_roots_and_leaves() {
    // Initialize logging for test
    let _ = tracing_subscriber::fmt::try_init();

    let workflow = workflow! {
        name: "test-workflow",
        tasks: [
            fetch_document,
            extract_text,
            generate_embeddings,
            store_embeddings
        ]
    };

    // Root tasks (no dependencies)
    let roots = workflow.get_roots();
    assert_eq!(roots.len(), 1);
    let fetch_ns = TaskNamespace::new("public", "embedded", "test-workflow", "fetch-document");
    assert!(roots.contains(&fetch_ns));

    // Leaf tasks (no dependents)
    let leaves = workflow.get_leaves();
    assert_eq!(leaves.len(), 1);
    let store_ns = TaskNamespace::new("public", "embedded", "test-workflow", "store-embeddings");
    assert!(leaves.contains(&store_ns));
}
