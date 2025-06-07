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

//! Test to verify final context fix

use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;

/// A task that modifies the context
#[task(
    id = "modify_context",
    dependencies = []
)]
async fn modify_context(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("task_output", json!("Hello from the task!"))?;
    context.insert("processed", json!(true))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize minimal logging
    tracing_subscriber::fmt()
        .with_env_filter("error")
        .init();

    // Create runner with in-memory SQLite database
    let runner = DefaultRunner::with_config(
        "sqlite://:memory:",
        DefaultRunnerConfig::default(),
    )
    .await?;

    // Create workflow
    let _workflow = workflow! {
        name: "test_workflow",
        description: "Test workflow for final context",
        tasks: [
            modify_context
        ]
    };

    // Create input context with initial data
    let mut input_context = Context::new();
    input_context.insert("initial", json!("input data"))?;

    println!("Input context: {:?}", input_context.to_json()?);

    // Execute the workflow
    let result = runner.execute("test_workflow", input_context).await?;

    println!("Final context: {:?}", result.final_context.to_json()?);
    
    // Check if final context contains task output
    if let Some(task_output) = result.final_context.get("task_output") {
        println!("SUCCESS: Final context contains task output: {:?}", task_output);
    } else {
        println!("ERROR: Final context missing task output");
    }

    if let Some(processed) = result.final_context.get("processed") {
        println!("SUCCESS: Final context contains processed flag: {:?}", processed);
    } else {
        println!("ERROR: Final context missing processed flag");
    }

    // Shutdown the runner
    runner.shutdown().await?;

    Ok(())
}