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

// This example demonstrates compile-time validation of workflows referencing missing tasks
// This should FAIL to compile with an error about missing task in workflow

use cloacina::{Context, TaskError};
use cloacina_macros::{task, workflow};
use serde_json::Value;

#[task(id = "existing_task", dependencies = [])]
async fn existing_task(_context: &mut Context<Value>) -> Result<(), TaskError> {
    println!("This task exists");
    Ok(())
}

fn main() {
    println!("If you're reading this, the workflow validation failed!");

    // This should cause a compile error because "nonexistent_task" doesn't exist
    let _pipeline = workflow!(
        name: "failing_pipeline",
        tasks: [existing_task, nonexistent_task]
    );
    println!("Pipeline created: {:?}", _pipeline.name());
}
