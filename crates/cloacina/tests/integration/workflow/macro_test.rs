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

use cloacina::{task, workflow, Context, TaskError, TaskNamespace};

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
    let registry = cloacina::workflow::global_workflow_registry();
    let guard = registry.read();
    assert!(
        guard.contains_key("document_processing"),
        "document_processing workflow should be auto-registered"
    );

    // Construct the workflow to verify properties
    let constructor = guard.get("document_processing").unwrap();
    let wf = constructor();
    assert_eq!(wf.name(), "document_processing");
    assert!(!wf.metadata().version.is_empty());
    assert_eq!(
        wf.metadata().description,
        Some("Process documents into knowledge base".to_string())
    );
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
fn test_workflow_execution_levels() {
    let _ = tracing_subscriber::fmt::try_init();

    let registry = cloacina::workflow::global_workflow_registry();
    let guard = registry.read();
    let constructor = guard.get("parallel_execution").unwrap();
    let wf = constructor();

    let execution_levels = wf.get_execution_levels().unwrap();

    // Level 0: task_a and task_b (can run in parallel)
    assert_eq!(execution_levels[0].len(), 2);

    // Level 1: task_c (depends on both task_a and task_b)
    assert_eq!(execution_levels[1].len(), 1);
}
