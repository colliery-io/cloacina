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
# Packaged Workflow Example

This example demonstrates how to create distributable workflow packages using the
`#[packaged_workflow]` macro. These packages can be compiled into shared libraries
and dynamically loaded by Cloacina executors.

## Key Features Demonstrated

1. **Package Metadata**: Version, description, and author information
2. **Namespace Isolation**: Tasks are registered under package-specific namespaces
3. **Automatic Task Discovery**: All `#[task]` functions are automatically detected
4. **ABI Compatibility**: Standard entry points for dynamic loading
5. **Multi-tenant Support**: Configurable tenant isolation

## Usage

```bash
# Compile as a shared library
cargo build --release

# The resulting .so file can be loaded dynamically by executors
```
*/

use cloacina::{Context, TaskError};
use cloacina_macros::{packaged_workflow, task};

/// Analytics Pipeline - A complete data processing workflow package
///
/// This package demonstrates a real-world analytics pipeline with data extraction,
/// transformation, validation, and reporting tasks. Each task is isolated within
/// the package namespace to prevent conflicts with other workflow packages.
#[packaged_workflow(
    package = "analytics_pipeline",
    version = "2.1.0",
    description = "Real-time analytics and data processing pipeline",
    author = "Analytics Team <analytics@company.com>"
)]
pub mod analytics_workflow {
    use super::*;

    /// Extract raw data from multiple sources
    ///
    /// This task handles data ingestion from various sources including databases,
    /// APIs, and file systems. Data is normalized and prepared for processing.
    #[task(
        id = "extract_data",
        dependencies = [],
        retry_attempts = 3,
        retry_backoff = "exponential"
    )]
    pub async fn extract_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("🔍 Extracting data from sources...");

        // Simulate data extraction
        let sources = vec!["database", "api", "files"];
        let mut extracted_records = 0;

        for source in sources {
            println!("  📥 Processing source: {}", source);
            // Simulate extraction time
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            extracted_records += match source {
                "database" => 1500,
                "api" => 800,
                "files" => 300,
                _ => 0,
            };
        }

        context.insert("extracted_records", serde_json::json!(extracted_records))?;
        context.insert(
            "extraction_timestamp",
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        )?;

        println!("✅ Extracted {} records", extracted_records);
        Ok(())
    }

    /// Validate and clean extracted data
    ///
    /// Performs data quality checks, removes duplicates, handles missing values,
    /// and ensures data consistency before transformation.
    #[task(
        id = "validate_data",
        dependencies = ["extract_data"],
        retry_attempts = 2,
        retry_backoff = "linear"
    )]
    pub async fn validate_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("🔍 Validating extracted data...");

        let extracted_records = context
            .get("extracted_records")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Simulate validation checks
        let validation_rules = vec![
            "null_check",
            "duplicate_removal",
            "format_validation",
            "business_rules",
        ];

        let mut valid_records = extracted_records;

        for rule in validation_rules {
            println!("  🔍 Applying rule: {}", rule);
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Simulate some data being filtered out
            match rule {
                "null_check" => valid_records = (valid_records as f64 * 0.98) as u64,
                "duplicate_removal" => valid_records = (valid_records as f64 * 0.95) as u64,
                "format_validation" => valid_records = (valid_records as f64 * 0.99) as u64,
                "business_rules" => valid_records = (valid_records as f64 * 0.97) as u64,
                _ => {}
            }
        }

        context.insert("valid_records", serde_json::json!(valid_records))?;
        context.insert(
            "validation_timestamp",
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        )?;

        println!(
            "✅ Validated {} records ({} removed)",
            valid_records,
            extracted_records - valid_records
        );
        Ok(())
    }

    /// Transform validated data into required format
    ///
    /// Applies business logic transformations, aggregations, and enrichment
    /// to prepare data for analysis and reporting.
    #[task(
        id = "transform_data",
        dependencies = ["validate_data"],
        retry_attempts = 3,
        retry_backoff = "exponential"
    )]
    pub async fn transform_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("🔄 Transforming validated data...");

        let valid_records = context
            .get("valid_records")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Simulate data transformation steps
        let transformations = vec![
            "normalize_formats",
            "calculate_metrics",
            "enrich_data",
            "aggregate_summaries",
        ];

        let mut transformed_records = valid_records;

        for transformation in transformations {
            println!("  🔄 Applying: {}", transformation);
            tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;

            // Simulate transformation effects
            match transformation {
                "normalize_formats" => {
                    // No record loss in normalization
                }
                "calculate_metrics" => {
                    // Might create additional derived records
                    transformed_records = (transformed_records as f64 * 1.1) as u64;
                }
                "enrich_data" => {
                    // External lookups might filter some records
                    transformed_records = (transformed_records as f64 * 0.98) as u64;
                }
                "aggregate_summaries" => {
                    // Aggregation produces fewer output records but richer content
                    transformed_records = (transformed_records as f64 * 0.3) as u64;
                }
                _ => {}
            }
        }

        context.insert(
            "transformed_records",
            serde_json::json!(transformed_records),
        )?;
        context.insert(
            "transformation_timestamp",
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        )?;

        println!("✅ Transformed to {} enriched records", transformed_records);
        Ok(())
    }

    /// Generate comprehensive reports from transformed data
    ///
    /// Creates various output formats including dashboards, alerts, and scheduled
    /// reports for different stakeholder groups.
    #[task(
        id = "generate_reports",
        dependencies = ["transform_data"],
        retry_attempts = 2,
        retry_backoff = "fixed"
    )]
    pub async fn generate_reports(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        println!("📊 Generating comprehensive reports...");

        let transformed_records = context
            .get("transformed_records")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Calculate analytics metrics
        let total_records = context
            .get("extracted_records")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Simulate report generation
        let report_types = vec![
            "executive_dashboard",
            "operational_metrics",
            "quality_report",
            "performance_analytics",
        ];

        let mut generated_reports = Vec::new();

        for report_type in report_types {
            println!("  📋 Creating: {}", report_type);
            tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;

            let report = serde_json::json!({
                "type": report_type,
                "records_processed": transformed_records,
                "generated_at": chrono::Utc::now().to_rfc3339(),
                "format": "json",
                "status": "completed"
            });

            generated_reports.push(report);
        }

        context.insert("generated_reports", serde_json::json!(generated_reports))?;
        context.insert(
            "pipeline_completed_at",
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        )?;

        // Summary output
        if transformed_records > 0 {
            println!("📈 Analytics Pipeline Summary:");
            println!("   📊 Processed {} total records", total_records);
            println!("   📋 Generated {} reports", generated_reports.len());
        }

        Ok(())
    }
}
