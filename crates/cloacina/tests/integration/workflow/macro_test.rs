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

use cloacina::{task, workflow, Context, TaskError};

// Define test workflows using the new #[workflow] attribute macro

#[workflow(
    name = "document_processing",
    description = "Process documents into knowledge base"
)]
pub mod document_processing {
    use super::*;

    #[task(id = "fetch_document", dependencies = [])]
    pub async fn fetch_document(
        _context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(id = "extract_text", dependencies = ["fetch_document"])]
    pub async fn extract_text(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(id = "generate_embeddings", dependencies = ["extract_text"])]
    pub async fn generate_embeddings(
        _context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(id = "store_embeddings", dependencies = ["generate_embeddings"])]
    pub async fn store_embeddings(
        _context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        Ok(())
    }
}

#[test]
fn test_workflow_macro_basic() {
    let _ = tracing_subscriber::fmt::try_init();

    // Workflow is auto-registered by #[workflow] macro
    let runtime = cloacina::Runtime::new();
    let wf = runtime
        .get_workflow("document_processing")
        .expect("document_processing workflow should be auto-registered");
    assert_eq!(wf.name(), "document_processing");
    assert!(!wf.metadata().version.is_empty());
    assert_eq!(
        wf.metadata().description,
        Some("Process documents into knowledge base".to_string())
    );
}

// CLOACI-T-0732 regression guard: bare `#[task]` (no id, no deps) INSIDE a
// `#[workflow]` module. The workflow macro builds the compile-time DAG by
// reading the task attrs directly, so the id-defaults-to-fn-name behavior must
// hold here too — a downstream task can depend on a bare task by its fn name.
// (This path is what broke when only the task macro applied the default.)
#[workflow(name = "bare_task_workflow")]
pub mod bare_task_workflow {
    use super::*;

    #[task]
    pub async fn produce(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(dependencies = ["produce"])]
    pub async fn consume(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }
}

#[test]
fn test_workflow_with_bare_tasks_registers() {
    let _ = tracing_subscriber::fmt::try_init();

    let runtime = cloacina::Runtime::new();
    let wf = runtime
        .get_workflow("bare_task_workflow")
        .expect("bare_task_workflow should auto-register with id-defaulted bare tasks");
    assert_eq!(wf.name(), "bare_task_workflow");
}

#[workflow(name = "parallel_execution")]
pub mod parallel_execution {
    use super::*;

    #[task(id = "task_a", dependencies = [])]
    pub async fn task_a(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(id = "task_b", dependencies = [])]
    pub async fn task_b(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(id = "task_c", dependencies = ["task_a", "task_b"])]
    pub async fn task_c(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }
}

#[test]
fn test_workflow_macro_emits_inventory_entries() {
    // Smoke test for T-0505: confirm that `#[workflow]` and `#[task]` emit
    // `inventory::submit!` entries in addition to the legacy `#[ctor]`
    // registration. The runtime will read these once T-0506 lands.
    let workflow_names: Vec<&'static str> = inventory::iter::<cloacina::WorkflowEntry>
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert!(
        workflow_names.contains(&"document_processing"),
        "WorkflowEntry for document_processing should be present in inventory; saw {:?}",
        workflow_names
    );
    assert!(
        workflow_names.contains(&"parallel_execution"),
        "WorkflowEntry for parallel_execution should be present in inventory; saw {:?}",
        workflow_names
    );

    // Each task in the two workflows should appear in inventory too.
    let task_namespaces: Vec<String> = inventory::iter::<cloacina::TaskEntry>
        .into_iter()
        .map(|entry| (entry.namespace)().to_string())
        .collect();

    let expected_ids = [
        "fetch_document",
        "extract_text",
        "generate_embeddings",
        "store_embeddings",
        "task_a",
        "task_b",
        "task_c",
    ];
    for id in expected_ids {
        assert!(
            task_namespaces.iter().any(|ns| ns.ends_with(id)),
            "TaskEntry for {} should be present in inventory; saw {:?}",
            id,
            task_namespaces
        );
    }
}

#[test]
fn test_workflow_execution_levels() {
    let _ = tracing_subscriber::fmt::try_init();

    let runtime = cloacina::Runtime::new();
    let wf = runtime
        .get_workflow("parallel_execution")
        .expect("parallel_execution workflow should be auto-registered");

    let execution_levels = wf.get_execution_levels().unwrap();

    // Level 0: task_a and task_b (can run in parallel)
    assert_eq!(execution_levels[0].len(), 2);

    // Level 1: task_c (depends on both task_a and task_b)
    assert_eq!(execution_levels[1].len(), 1);
}
