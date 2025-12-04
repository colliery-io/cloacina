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

/*!
# Simple Packaged Workflow Demo

This example demonstrates the complete end-to-end lifecycle of packaged workflows:

1. **Define** - Create a packaged workflow with tasks
2. **Compile** - Build to shared library (.so/.dylib/.dll)
3. **Package** - Create .cloacina archive
4. **Load** - Dynamically load via registry
5. **Execute** - Run tasks through scheduler

## Usage

```bash
# Step 1: Build the workflow package
cargo build --release

# Step 2: Run the packaging demo
cargo run --example package_workflow

# Step 3: Run the end-to-end demo
cargo run --example end_to_end_demo
```
*/

use cloacina_workflow::{packaged_workflow, task, Context, TaskError};

/// Simple Data Processing Workflow
///
/// A minimal workflow that demonstrates the complete packaged workflow lifecycle
/// with data processing, validation, and reporting.
#[packaged_workflow(
    name = "data_processing",
    package = "simple_demo",
    description = "Simple data processing workflow for demonstration",
    author = "Cloacina Demo Team"
)]
pub mod data_processing {
    use super::*;

    /// Step 1: Collect input data
    #[task(
        id = "collect_data",
        dependencies = [],
        retry_attempts = 2
    )]
    pub async fn collect_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("🔍 Collecting data...");

        // Simulate data collection
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let data = serde_json::json!({
            "records": 1000,
            "source": "demo_database",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        context.insert("raw_data", data)?;
        println!("✅ Collected 1000 records");
        Ok(())
    }

    /// Step 2: Process the collected data
    #[task(
        id = "process_data",
        dependencies = ["collect_data"],
        retry_attempts = 3
    )]
    pub async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("⚙️  Processing data...");

        // Get input data
        let raw_data = context
            .get("raw_data")
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "Missing raw_data".to_string(),
            })?;

        // Simulate processing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let processed = serde_json::json!({
            "processed_records": 950,  // Some records filtered out
            "original_count": raw_data["records"],
            "processing_time_ms": 200,
            "status": "completed"
        });

        context.insert("processed_data", processed)?;
        println!("✅ Processed 950 valid records");
        Ok(())
    }

    /// Step 3: Generate summary report
    #[task(
        id = "generate_report",
        dependencies = ["process_data"],
        retry_attempts = 1
    )]
    pub async fn generate_report(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        println!("📊 Generating report...");

        // Get processed data
        let processed_data =
            context
                .get("processed_data")
                .ok_or_else(|| TaskError::ValidationFailed {
                    message: "Missing processed_data".to_string(),
                })?;

        // Simulate report generation
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let report = serde_json::json!({
            "report_id": format!("RPT_{}", chrono::Utc::now().timestamp()),
            "summary": {
                "total_processed": processed_data["processed_records"],
                "success_rate": "95%",
                "processing_time": processed_data["processing_time_ms"]
            },
            "generated_at": chrono::Utc::now().to_rfc3339()
        });

        context.insert("final_report", report)?;
        println!("✅ Report generated successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_execution() {
        let mut context = Context::new();

        // Execute workflow steps in order
        data_processing::collect_data(&mut context).await.unwrap();
        data_processing::process_data(&mut context).await.unwrap();
        data_processing::generate_report(&mut context)
            .await
            .unwrap();

        // Verify final state
        let report = context.get("final_report").unwrap();
        assert!(report["report_id"].as_str().unwrap().starts_with("RPT_"));
        assert_eq!(report["summary"]["total_processed"], 950);
    }
}
