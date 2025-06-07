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

//! # Data Pipeline Example
//!
//! This example demonstrates a realistic data processing pipeline using Cloacina.
//! It showcases:
//! - Task definition with the macro system
//! - Complex dependency chains
//! - Error handling and retry policies
//! - Conditional execution based on data quality
//! - Recovery from failures
//!
//! ## Pipeline Flow
//! 1. **fetch_raw_data** - Download data from external API
//! 2. **validate_data** - Check data quality and completeness
//! 3. **transform_data** - Clean and normalize data (only if validation passes)
//! 4. **enrich_data** - Add additional information from secondary sources
//! 5. **load_to_warehouse** - Store processed data
//! 6. **send_notification** - Notify stakeholders of completion
//! 7. **cleanup_temp_files** - Remove temporary files (runs on success or failure)
//!
//! ## Error Scenarios Demonstrated
//! - Network timeouts with exponential backoff retry
//! - Data validation failures that skip downstream processing
//! - Conditional cleanup that always runs
//! - Different retry policies for different task types

use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{workflow, Context};
use serde_json::json;
use tracing::info;

mod tasks;

use tasks::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("etl_example=debug,cloacina=info")
        .init();

    info!("Starting ETL Example");

    // Initialize runner with SQLite database using WAL mode for better concurrency
    let runner = DefaultRunner::with_config(
        "sqlite://tutorial-02.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        DefaultRunnerConfig::default(),
    )
    .await?;

    // Create the ETL workflow
    let _pipeline = create_etl_workflow()?;

    // Create two different input contexts
    let mut context1 = Context::new();
    context1.insert("numbers", json!([1, 2, 3, 4, 5]))?;

    let mut context2 = Context::new();
    context2.insert("numbers", json!([10, 20, 30, 40, 50]))?;

    info!("Submitting first ETL workflow with numbers [1, 2, 3, 4, 5]");
    let future1 = runner.execute("etl_workflow", context1);

    info!("Submitting second ETL workflow with numbers [10, 20, 30, 40, 50]");
    let future2 = runner.execute("etl_workflow", context2);

    info!("Both workflows submitted, waiting for completion...");

    // Wait for the second workflow first
    let result2 = future2.await?;
    info!(
        "Second workflow completed with status: {:?}",
        result2.status
    );
    info!("Second workflow execution took: {:?}", result2.duration);

    // Then wait for the first workflow
    let result1 = future1.await?;
    info!("First workflow completed with status: {:?}", result1.status);
    info!("First workflow execution took: {:?}", result1.duration);

    // Shutdown the runner
    runner.shutdown().await?;

    Ok(())
}

/// Create the ETL workflow
fn create_etl_workflow() -> Result<cloacina::Workflow, Box<dyn std::error::Error>> {
    let workflow = workflow! {
        name: "etl_workflow",
        description: "Simple ETL workflow with extract, transform, and load tasks",
        tasks: [
            extract_numbers,
            transform_numbers,
            load_numbers
        ]
    };

    Ok(workflow)
}
