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

//! Stress Performance Test
//!
//! Based on tutorial-04, this measures performance under failure conditions
//! to test how retry policies and error handling affect throughput.

use clap::Parser;
use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use rand::Rng;
use serde_json::json;
use std::time::{Duration, Instant};
use tracing::{info, warn};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of workflows to execute
    #[arg(short, long, default_value = "50")]
    iterations: usize,

    /// Maximum concurrent tasks
    #[arg(short, long, default_value = "4")]
    concurrency: usize,

    /// Task duration in milliseconds
    #[arg(short, long, default_value = "100")]
    task_duration_ms: u64,

    /// Failure rate (0.0 = no failures, 1.0 = always fail)
    #[arg(short = 'r', long, default_value = "0.3")]
    failure_rate: f64,

    /// Output format (json, csv, human)
    #[arg(short = 'o', long, default_value = "human")]
    format: String,
}

/// Primary task that might fail based on failure rate
#[task(
    id = "primary_task",
    dependencies = [],
    retry_attempts = 3,
    retry_delay_ms = 50,
    retry_backoff = "exponential"
)]
async fn primary_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let duration_ms = context.get("task_duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(100);
    
    let failure_rate = context.get("failure_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.3);
    
    tokio::time::sleep(Duration::from_millis(duration_ms)).await;
    
    let random_value: f64 = rand::random();
    if random_value < failure_rate {
        return Err(TaskError::ExecutionFailed {
            message: "Simulated primary task failure".to_string(),
            task_id: "primary_task".to_string(),
            timestamp: chrono::Utc::now(),
        });
    }
    
    context.insert("primary_result", json!("success"))?;
    context.insert("primary_timestamp", json!(chrono::Utc::now().to_rfc3339()))?;
    
    Ok(())
}

/// Fallback task that runs when primary fails
#[task(
    id = "fallback_task",
    dependencies = ["primary_task"]
)]
async fn fallback_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let duration_ms = context.get("task_duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(100);
    
    tokio::time::sleep(Duration::from_millis(duration_ms / 2)).await;
    
    context.insert("fallback_result", json!("fallback_success"))?;
    context.insert("fallback_timestamp", json!(chrono::Utc::now().to_rfc3339()))?;
    
    Ok(())
}

/// Processing task that works with either primary or fallback results
#[task(
    id = "process_task",
    dependencies = ["primary_task", "fallback_task"]
)]
async fn process_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let duration_ms = context.get("task_duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(100);
    
    tokio::time::sleep(Duration::from_millis(duration_ms / 3)).await;
    
    let data_source = if context.get("primary_result").is_some() {
        "primary"
    } else if context.get("fallback_result").is_some() {
        "fallback"
    } else {
        "unknown"
    };
    
    context.insert("processing_result", json!({
        "data_source": data_source,
        "processed_at": chrono::Utc::now().to_rfc3339(),
        "status": "completed"
    }))?;
    
    Ok(())
}

/// Final task that might also fail occasionally
#[task(
    id = "final_task",
    dependencies = ["process_task"],
    retry_attempts = 2,
    retry_delay_ms = 25,
    retry_backoff = "fixed"
)]
async fn final_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let duration_ms = context.get("task_duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(100);
    
    tokio::time::sleep(Duration::from_millis(duration_ms / 4)).await;
    
    // Small chance of final task failure
    let random_value: f64 = rand::random();
    if random_value < 0.1 {
        return Err(TaskError::ExecutionFailed {
            message: "Simulated final task failure".to_string(),
            task_id: "final_task".to_string(),
            timestamp: chrono::Utc::now(),
        });
    }
    
    context.insert("final_result", json!("workflow_complete"))?;
    context.insert("final_timestamp", json!(chrono::Utc::now().to_rfc3339()))?;
    
    Ok(())
}

#[derive(Debug)]
struct PerformanceResults {
    total_iterations: usize,
    concurrency: usize,
    task_duration_ms: u64,
    failure_rate: f64,
    total_duration: Duration,
    workflows_per_second: f64,
    average_workflow_time: Duration,
    min_workflow_time: Duration,
    max_workflow_time: Duration,
    successful_workflows: usize,
    failed_workflows: usize,
    total_retries: usize,
    fallback_usage: usize,
}

impl PerformanceResults {
    fn print_human(&self) {
        println!("\n=== Stress Performance Test Results ===");
        println!("Configuration:");
        println!("  Iterations: {}", self.total_iterations);
        println!("  Concurrency: {}", self.concurrency);
        println!("  Task Duration: {}ms", self.task_duration_ms);
        println!("  Failure Rate: {:.1}%", self.failure_rate * 100.0);
        
        println!("\nResults:");
        println!("  Total Duration: {:.2}s", self.total_duration.as_secs_f64());
        println!("  Workflows/sec: {:.2}", self.workflows_per_second);
        println!("  Avg Workflow Time: {:.2}ms", self.average_workflow_time.as_millis());
        println!("  Min Workflow Time: {:.2}ms", self.min_workflow_time.as_millis());
        println!("  Max Workflow Time: {:.2}ms", self.max_workflow_time.as_millis());
        println!("  Success Rate: {}/{} ({:.1}%)", 
                 self.successful_workflows, 
                 self.total_iterations,
                 (self.successful_workflows as f64 / self.total_iterations as f64) * 100.0);
        
        println!("\nStress Analysis:");
        println!("  Total Retries: {}", self.total_retries);
        println!("  Fallback Usage: {} workflows ({:.1}%)", 
                 self.fallback_usage, 
                 (self.fallback_usage as f64 / self.total_iterations as f64) * 100.0);
        println!("  Avg Retries per Workflow: {:.2}", 
                 self.total_retries as f64 / self.total_iterations as f64);
        println!("  Expected Failure Rate: {:.1}%", self.failure_rate * 100.0);
        println!("  Actual Failure Rate: {:.1}%", 
                 (self.failed_workflows as f64 / self.total_iterations as f64) * 100.0);
    }

    fn print_json(&self) {
        let json = json!({
            "config": {
                "iterations": self.total_iterations,
                "concurrency": self.concurrency,
                "task_duration_ms": self.task_duration_ms,
                "failure_rate": self.failure_rate
            },
            "results": {
                "total_duration_s": self.total_duration.as_secs_f64(),
                "workflows_per_second": self.workflows_per_second,
                "average_workflow_time_ms": self.average_workflow_time.as_millis(),
                "min_workflow_time_ms": self.min_workflow_time.as_millis(),
                "max_workflow_time_ms": self.max_workflow_time.as_millis(),
                "successful_workflows": self.successful_workflows,
                "failed_workflows": self.failed_workflows,
                "success_rate": (self.successful_workflows as f64 / self.total_iterations as f64),
                "total_retries": self.total_retries,
                "fallback_usage": self.fallback_usage,
                "avg_retries_per_workflow": self.total_retries as f64 / self.total_iterations as f64,
                "expected_failure_rate": self.failure_rate,
                "actual_failure_rate": (self.failed_workflows as f64 / self.total_iterations as f64)
            }
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    fn print_csv(&self) {
        println!("iterations,concurrency,task_duration_ms,failure_rate,total_duration_s,workflows_per_second,avg_workflow_time_ms,min_workflow_time_ms,max_workflow_time_ms,successful_workflows,failed_workflows,success_rate,total_retries,fallback_usage,avg_retries_per_workflow,expected_failure_rate,actual_failure_rate");
        println!("{},{},{},{:.3},{:.2},{:.2},{},{},{},{},{},{:.3},{},{},{:.2},{:.3},{:.3}",
                 self.total_iterations,
                 self.concurrency,
                 self.task_duration_ms,
                 self.failure_rate,
                 self.total_duration.as_secs_f64(),
                 self.workflows_per_second,
                 self.average_workflow_time.as_millis(),
                 self.min_workflow_time.as_millis(),
                 self.max_workflow_time.as_millis(),
                 self.successful_workflows,
                 self.failed_workflows,
                 (self.successful_workflows as f64 / self.total_iterations as f64),
                 self.total_retries,
                 self.fallback_usage,
                 self.total_retries as f64 / self.total_iterations as f64,
                 self.failure_rate,
                 self.failed_workflows as f64 / self.total_iterations as f64);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("performance_stress=info,cloacina=warn")
        .init();

    info!("Starting Stress Performance Test");
    info!("Configuration: {} iterations, {} concurrency, {}ms task duration, {:.1}% failure rate", 
          args.iterations, args.concurrency, args.task_duration_ms, args.failure_rate * 100.0);

    // Create workflow (automatically registers in global registry)
    let _workflow = workflow! {
        name: "stress_workflow",
        description: "Stress test workflow with retries and fallbacks",
        tasks: [
            primary_task,
            fallback_task,
            process_task,
            final_task
        ]
    };

    // Initialize runner with SQLite database
    let mut config = DefaultRunnerConfig::default();
    config.max_concurrent_tasks = args.concurrency;
    config.executor_poll_interval = Duration::from_millis(10);
    config.scheduler_poll_interval = Duration::from_millis(10);

    let runner = DefaultRunner::with_config(
        "sqlite://performance-stress.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        config,
    )
    .await?;

    info!("Runner initialized, starting performance test");

    // Track performance metrics
    let mut workflow_times = Vec::new();
    let mut successful_workflows = 0;
    let mut failed_workflows = 0;
    let mut total_retries = 0;
    let mut fallback_usage = 0;

    let overall_start = Instant::now();

    // Execute workflows
    for i in 1..=args.iterations {
        if i % 25 == 0 || i == args.iterations {
            info!("Progress: {}/{} workflows completed", i, args.iterations);
        }

        let workflow_start = Instant::now();

        // Create input context with task configuration
        let mut input_context = Context::new();
        input_context.insert("task_duration_ms", json!(args.task_duration_ms))?;
        input_context.insert("failure_rate", json!(args.failure_rate))?;
        input_context.insert("iteration", json!(i))?;

        // Execute the workflow
        match runner.execute("stress_workflow", input_context).await {
            Ok(result) => {
                let workflow_time = workflow_start.elapsed();
                workflow_times.push(workflow_time);
                successful_workflows += 1;

                // Count retries and fallback usage
                for task_result in &result.task_results {
                    if task_result.attempt_count > 1 {
                        total_retries += task_result.attempt_count - 1;
                    }
                }

                // Check if fallback was used
                if result.final_context.get("fallback_result").is_some() {
                    fallback_usage += 1;
                }

                if result.status != cloacina::executor::pipeline_executor::PipelineStatus::Completed {
                    warn!("Workflow {} completed with status: {:?}", i, result.status);
                }
            }
            Err(e) => {
                warn!("Workflow {} failed: {}", i, e);
                failed_workflows += 1;
                workflow_times.push(workflow_start.elapsed());
            }
        }
    }

    let total_duration = overall_start.elapsed();

    // Calculate statistics
    let workflows_per_second = args.iterations as f64 / total_duration.as_secs_f64();
    let average_workflow_time = Duration::from_nanos(
        (workflow_times.iter().map(|d| d.as_nanos()).sum::<u128>() / workflow_times.len() as u128) as u64
    );
    let min_workflow_time = *workflow_times.iter().min().unwrap_or(&Duration::ZERO);
    let max_workflow_time = *workflow_times.iter().max().unwrap_or(&Duration::ZERO);

    let results = PerformanceResults {
        total_iterations: args.iterations,
        concurrency: args.concurrency,
        task_duration_ms: args.task_duration_ms,
        failure_rate: args.failure_rate,
        total_duration,
        workflows_per_second,
        average_workflow_time,
        min_workflow_time,
        max_workflow_time,
        successful_workflows,
        failed_workflows,
        total_retries,
        fallback_usage,
    };

    // Shutdown the runner
    runner.shutdown().await?;

    info!("Performance test completed");

    // Output results in requested format
    match args.format.as_str() {
        "json" => results.print_json(),
        "csv" => results.print_csv(),
        "human" | _ => results.print_human(),
    }

    Ok(())
}