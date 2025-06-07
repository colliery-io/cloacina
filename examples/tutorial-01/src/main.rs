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

//! Simple Cloacina Example
//!
//! This example demonstrates the most basic usage of Cloacina with a single task.

use cloacina::executor::PipelineExecutor;

use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;
use tracing::info;

/// A simple task that just logs a message
#[task(
    id = "hello_world",
    dependencies = []
)]
async fn hello_world(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Hello from Cloacina!");

    // Add some data to context for demonstration
    context.insert("message", json!("Hello World!"))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("simple_example=debug,cloacina=debug")
        .init();

    info!("Starting Simple Cloacina Example");

    // Initialize runner with SQLite database using WAL mode for better concurrency

    let runner = DefaultRunner::with_config(
        "sqlite://tutorial-01.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        DefaultRunnerConfig::default(),
    )
    .await?;

    // Create a simple workflow (automatically registers in global registry)
    let _workflow = workflow! {
        name: "simple_workflow",
        description: "A simple workflow with one task",
        tasks: [
            hello_world
        ]
    };

    // Create input context
    let input_context = Context::new();

    info!("Executing workflow");

    // Execute the workflow (scheduler and executor managed automatically)
    let result = runner.execute("simple_workflow", input_context).await?;

    info!("Workflow completed with status: {:?}", result.status);
    info!("Final context: {:?}", result.final_context);

    // Shutdown the runner
    runner.shutdown().await?;

    info!("Simple example completed!");

    Ok(())
}
