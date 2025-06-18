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

//! Parallel Performance Test
//!
//! Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.

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

/// Initial setup task that creates data for parallel processing
#[task(
    id = "setup_data",
    dependencies = []
)]
async fn setup_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("data_batch_1", json!([1, 2, 3, 4, 5]))?;
    context.insert("data_batch_2", json!([6, 7, 8, 9, 10]))?;
    context.insert("data_batch_3", json!([11, 12, 13, 14, 15]))?;
    
    Ok(())
}

/// Process batch 1 in parallel
#[task(
    id = "process_batch_1",
    dependencies = ["setup_data"]
)]
async fn process_batch_1(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let batch = context.get("data_batch_1")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_i64())
        .map(|v| v * 2)
        .collect::<Vec<_>>();
    
    context.insert("result_batch_1", json!(batch))?;
    
    Ok(())
}

/// Process batch 2 in parallel
#[task(
    id = "process_batch_2",
    dependencies = ["setup_data"]
)]
async fn process_batch_2(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let batch = context.get("data_batch_2")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_i64())
        .map(|v| v * 3)
        .collect::<Vec<_>>();
    
    context.insert("result_batch_2", json!(batch))?;
    
    Ok(())
}

/// Process batch 3 in parallel
#[task(
    id = "process_batch_3",
    dependencies = ["setup_data"]
)]
async fn process_batch_3(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let batch = context.get("data_batch_3")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_i64())
        .map(|v| v * 4)
        .collect::<Vec<_>>();
    
    context.insert("result_batch_3", json!(batch))?;
    
    Ok(())
}

/// Merge results from all parallel tasks
#[task(
    id = "merge_results",
    dependencies = ["process_batch_1", "process_batch_2", "process_batch_3"]
)]
async fn merge_results(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let batch1 = context.get("result_batch_1")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .len();
    let batch2 = context.get("result_batch_2")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .len();
    let batch3 = context.get("result_batch_3")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .len();
    
    let total_items = batch1 + batch2 + batch3;
    
    context.insert("total_items_processed", json!(total_items))?;
    context.insert("merge_status", json!("success"))?;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging (disabled for performance)
    tracing_subscriber::fmt()
        .with_env_filter("off")
        .init();

    println!("Starting Parallel Performance Test");

    // Initialize runner with SQLite database using WAL mode for better concurrency
    let mut config = DefaultRunnerConfig::default();
    config.max_concurrent_tasks = args.concurrency;
    
    let runner = DefaultRunner::with_config(
        "sqlite://performance-parallel.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        config,
    )
    .await?;

    // Create a parallel workflow (automatically registers in global registry)
    let _workflow = workflow! {
        name: "parallel_workflow",
        description: "Fan-out/fan-in parallel workflow with 5 tasks",
        tasks: [
            setup_data,
            process_batch_1,
            process_batch_2,
            process_batch_3,
            merge_results
        ]
    };

    let overall_start = Instant::now();

    // Submit all workflows concurrently - let executor handle the concurrency
    let mut futures = Vec::new();
    
    for _i in 1..=args.iterations {
        let input_context = Context::new();
        let future = runner.execute("parallel_workflow", input_context);
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
                // Only log errors
                if result.status != cloacina::executor::pipeline_executor::PipelineStatus::Completed {
                    error!("Workflow {} completed with status: {:?}", i + 1, result.status);
                }
                if result.final_context.get("merge_status").is_none() {
                    error!("No merge_status found in final context for iteration {}!", i + 1);
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
    println!("Configuration: {} iterations, {} concurrency", args.iterations, args.concurrency);
    println!("Total time: {:.2}s", total_duration.as_secs_f64());
    println!("Workflows per second: {:.2}", workflows_per_second);
    println!("Success rate: {}/{} ({:.1}%)", 
             successful_workflows, 
             args.iterations,
             (successful_workflows as f64 / args.iterations as f64) * 100.0);

    Ok(())
}