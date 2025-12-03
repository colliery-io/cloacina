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

// This example demonstrates compile-time validation of missing dependencies
// This should FAIL to compile with an error about missing dependency

use cloacina::{Context, TaskError};
use cloacina_macros::{task, workflow};
use serde_json::Value;

#[task(id = "valid_task", dependencies = [])]
async fn valid_task(_context: &mut Context<Value>) -> Result<(), TaskError> {
    println!("This task has no dependencies - valid");
    Ok(())
}

// This task should cause a compile error because "nonexistent_task" doesn't exist
#[task(id = "invalid_task", dependencies = ["nonexistent_task"])]
async fn invalid_task(_context: &mut Context<Value>) -> Result<(), TaskError> {
    println!("This should never compile");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("If you're reading this, the compile-time validation failed!");

    let _pipeline = workflow! {
        name: "missing_dep_pipeline",
        tasks: [valid_task, invalid_task]
    };

    Ok(())
}
