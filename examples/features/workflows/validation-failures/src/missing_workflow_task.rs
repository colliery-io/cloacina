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

// This example demonstrates compile-time validation of workflows referencing missing tasks
// This should FAIL to compile with an error about missing task in workflow

use cloacina::{Context, TaskError};
use cloacina_macros::{task, workflow};
use serde_json::Value;

#[workflow(name = "failing_pipeline")]
pub mod failing_pipeline {
    use super::*;

    #[task(id = "existing_task", dependencies = [])]
    pub async fn existing_task(_context: &mut Context<Value>) -> Result<(), TaskError> {
        println!("This task exists");
        Ok(())
    }

    // nonexistent_task is referenced here but not defined - should cause compile error
    #[task(id = "depends_on_missing", dependencies = ["nonexistent_task"])]
    pub async fn depends_on_missing(_context: &mut Context<Value>) -> Result<(), TaskError> {
        println!("This should never compile");
        Ok(())
    }
}

fn main() {
    println!("If you're reading this, the workflow validation failed!");
}
