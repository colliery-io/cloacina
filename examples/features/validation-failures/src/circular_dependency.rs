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

// This example demonstrates compile-time detection of circular dependencies
// This should FAIL to compile with an error about circular dependencies

use cloacina::{Context, TaskError};
use cloacina_macros::{task, workflow};
use serde_json::Value;

// Task A depends on Task B
#[task(id = "task_a", dependencies = ["task_b"])]
async fn task_a(_context: &mut Context<Value>) -> Result<(), TaskError> {
    println!("Task A");
    Ok(())
}

// Task B depends on Task A - this creates a circular dependency!
#[task(id = "task_b", dependencies = ["task_a"])]
async fn task_b(_context: &mut Context<Value>) -> Result<(), TaskError> {
    println!("Task B");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("If you're reading this, the circular dependency detection failed!");

    let _pipeline = workflow! {
        name: "circular_pipeline",
        tasks: [task_a, task_b]
    };

    Ok(())
}
