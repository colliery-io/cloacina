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

// This example demonstrates compile-time detection of duplicate task IDs
// This should FAIL to compile with an error about duplicate task IDs

use cloacina::{Context, TaskError};
use cloacina_macros::{task, workflow};
use serde_json::Value;

// First task with ID "duplicate_id"
#[task(id = "duplicate_id", dependencies = [])]
async fn task_one(_context: &mut Context<Value>) -> Result<(), TaskError> {
    println!("Task One");
    Ok(())
}

// Second task with the SAME ID "duplicate_id" - this should cause a compile error!
#[task(id = "duplicate_id", dependencies = [])]
async fn task_two(_context: &mut Context<Value>) -> Result<(), TaskError> {
    println!("Task Two");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("If you're reading this, the duplicate ID detection failed!");

    let _pipeline = workflow! {
        name: "duplicate_pipeline",
        tasks: [task_one, task_two]
    };

    Ok(())
}
