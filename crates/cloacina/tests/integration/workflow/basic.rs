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

use cloacina::{task, workflow};

#[workflow(name = "basic_test_pipeline")]
pub mod basic_test_pipeline {
    use super::*;

    #[task(id = "basic_workflow_task", dependencies = [])]
    pub async fn simple_task(
        _context: &mut cloacina::Context<serde_json::Value>,
    ) -> Result<(), cloacina::TaskError> {
        Ok(())
    }
}

#[test]
fn test_simple_workflow_creation() {
    // The #[workflow] macro auto-registers the workflow in the global registry
    let registry = cloacina::workflow::global_workflow_registry();
    let guard = registry.read();
    assert!(
        guard.contains_key("basic_test_pipeline"),
        "Workflow should be auto-registered"
    );
}
