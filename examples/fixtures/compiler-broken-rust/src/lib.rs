/*
 *  Copyright 2026 Colliery Software
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

// Broken fixture: structurally valid packaged workflow, but the task body
// references an undefined symbol so `cargo build` fails deterministically.
// build_status → failed, build_error populated, reconciler never sees it
// (success-only filter in list_workflows).

use cloacina_workflow::{task, workflow, Context, TaskError};

#[workflow(
    name = "compiler_broken_workflow",
    description = "compiler-e2e failed-build fixture",
    author = "compiler-e2e"
)]
pub mod compiler_broken_workflow {
    use super::*;

    #[task(
        id = "broken",
        dependencies = [],
        retry_attempts = 0
    )]
    pub async fn broken(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // Deliberate: this identifier doesn't exist anywhere. cargo will bail
        // with a clean unresolved-path error that the compiler service
        // captures into build_error.
        let _ = this_symbol_does_not_exist();
        Ok(())
    }
}
