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

use cloacina::{task, workflow, TaskNamespace};

#[task(id = "basic-workflow-task", dependencies = [])]
async fn simple_task(
    _context: &mut cloacina::Context<serde_json::Value>,
) -> Result<(), cloacina::TaskError> {
    Ok(())
}

#[test]
fn test_simple_workflow_creation() {
    let simple_workflow = workflow! {
        name: "simple-pipeline",
        tasks: [simple_task]
    };

    assert_eq!(simple_workflow.name(), "simple-pipeline");
    let task_ns = TaskNamespace::new(
        "public",
        "embedded",
        "simple-pipeline",
        "basic-workflow-task",
    );
    assert!(simple_workflow.get_task(&task_ns).is_ok());
}
