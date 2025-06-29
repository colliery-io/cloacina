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

//! Pipeline Performance Test
//!
//! Based on tutorial-02, this measures throughput of sequential 3-task pipelines.

use clap::Parser;
use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;
use std::time::Instant;
use tracing::error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of workflows to execute
    #[arg(short, long, default_value = "150")]
    iterations: usize,

    /// Maximum concurrent tasks
    #[arg(short, long, default_value = "32")]
    concurrency: usize,
}

/// Extract numbers from the input context
#[task(
    id = "extract_numbers",
    dependencies = []
)]
async fn extract_numbers(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Add some data to context for demonstration
    context.insert("extracted_numbers", json!([1, 2, 3, 4, 5]))?;
    Ok(())
}

/// Transform the numbers (multiply by 2)
#[task(
    id = "transform_numbers",
    dependencies = ["extract_numbers"]
)]
async fn transform_numbers(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let numbers = context
        .get("extracted_numbers")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|n| n.as_i64())
        .map(|n| n * 2)
        .collect::<Vec<_>>();

    context.insert("transformed_numbers", json!(numbers))?;
    Ok(())
}

/// Load the transformed numbers
#[task(
    id = "load_numbers",
    dependencies = ["transform_numbers"]
)]
async fn load_numbers(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let empty_vec = vec![];
    let numbers = context
        .get("transformed_numbers")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec);

    context.insert("loaded_numbers", json!(numbers))?;
    context.insert("load_status", json!("success"))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging (disabled for performance)
    tracing_subscriber::fmt().with_env_filter("off").init();

    println!("Starting Pipeline Performance Test");

    // Initialize runner with SQLite database using WAL mode for better concurrency
    let mut config = DefaultRunnerConfig::default();
    config.max_concurrent_tasks = args.concurrency;

    let runner = DefaultRunner::with_config(
        "sqlite://performance-pipeline.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        config,
    )
    .await?;

    // Create a simple 3-task pipeline workflow (automatically registers in global registry)
    let _workflow = workflow! {
        name: "etl_workflow",
        description: "Simple ETL workflow with extract, transform, and load tasks",
        tasks: [
            extract_numbers,
            transform_numbers,
            load_numbers
        ]
    };

    let overall_start = Instant::now();

    // Submit all workflows concurrently - let executor handle the concurrency
    let mut futures = Vec::new();

    for _i in 1..=args.iterations {
        let input_context = Context::new();
        let future = runner.execute("etl_workflow", input_context);
        futures.push(future);
    }

    // Wait for all workflows to complete
    let results = futures::future::join_all(futures).await;

    let total_duration = overall_start.elapsed();

    // Process results
    let mut successful_workflows = 0;
    let mut failed_workflows = 0;

    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(result) => {
                successful_workflows += 1;
                // Check if workflow completed successfully
                if !matches!(
                    result.status,
                    cloacina::executor::pipeline_executor::PipelineStatus::Completed
                ) {
                    error!(
                        "Workflow {} completed with status: {:?}",
                        i + 1,
                        result.status
                    );
                }
                if result.final_context.get("load_status").is_none() {
                    error!(
                        "No load_status found in final context for iteration {}!",
                        i + 1
                    );
                }
            }
            Err(e) => {
                failed_workflows += 1;
                error!("Workflow {} failed: {}", i + 1, e);
            }
        }
    }

    let workflows_per_second = args.iterations as f64 / total_duration.as_secs_f64();

    // Shutdown the runner
    runner.shutdown().await?;

    println!("Performance test completed!");
    println!(
        "Configuration: {} iterations, {} concurrency",
        args.iterations, args.concurrency
    );
    println!("Total time: {:.2}s", total_duration.as_secs_f64());
    println!("Workflows per second: {:.2}", workflows_per_second);
    println!(
        "Success rate: {}/{} ({:.1}%)",
        successful_workflows,
        args.iterations,
        (successful_workflows as f64 / args.iterations as f64) * 100.0
    );

    Ok(())
}
