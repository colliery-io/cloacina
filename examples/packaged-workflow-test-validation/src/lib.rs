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

/*
 * Test package to validate compile-time validation of packaged workflows
 */

use cloacina::{Context, TaskError};
use cloacina_macros::{packaged_workflow, task};

/// Test package with a missing dependency - this should fail to compile
#[packaged_workflow(
    package = "validation_test",
    version = "1.0.0",
    description = "Test package for validation",
    author = "Test"
)]
pub mod missing_dependency_workflow {
    use super::*;

    #[task(
        id = "task_a",
        dependencies = ["nonexistent_task"], // This task doesn't exist
    )]
    pub async fn task_a(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(
        id = "task_b",
        dependencies = [],
    )]
    pub async fn task_b(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }
}
