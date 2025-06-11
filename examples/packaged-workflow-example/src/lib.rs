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

    /// Transform data into analytics-ready format
    ///
    /// Applies business logic transformations, aggregations, and enrichment
    /// to prepare data for analytics and reporting.
    #[task(
        id = "transform_data",
        dependencies = ["validate_data"],
        retry_attempts = 3,
        retry_backoff = "exponential",
        retry_delay_ms = 2000
    )]
    pub async fn transform_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("🔄 Transforming data for analytics...");

        let valid_records = context
            .get("valid_records")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Simulate transformation operations
        let transformations = vec![
            "normalize_values",
            "calculate_metrics",
            "apply_business_logic",
            "create_aggregations",
        ];

        for transformation in transformations {
            println!("  🔄 Applying: {}", transformation);
            tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;
        }

        // Generate analytics metrics
        let metrics = serde_json::json!({
            "total_records": valid_records,
            "revenue_metrics": {
                "total_revenue": valid_records * 25, // $25 per record average
                "avg_transaction": 25
            },
            "performance_metrics": {
                "processing_rate": valid_records as f64 / 60.0, // per minute
                "error_rate": 0.03
            }
        });

        context.insert("analytics_metrics", metrics)?;
        context.insert(
            "transformation_timestamp",
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        )?;

        println!(
            "✅ Transformed {} records into analytics format",
            valid_records
        );
        Ok(())
    }

    /// Generate comprehensive analytics reports
    ///
    /// Creates various report formats including executive summaries, detailed
    /// analytics, and operational dashboards based on transformed data.
    #[task(
        id = "generate_reports",
        dependencies = ["transform_data"],
        retry_attempts = 2
    )]
    pub async fn generate_reports(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        println!("📊 Generating analytics reports...");

        let metrics = context
            .get("analytics_metrics")
            .cloned()
            .unwrap_or_default();

        // Simulate report generation
        let report_types = vec![
            "executive_summary",
            "detailed_analytics",
            "operational_dashboard",
            "trend_analysis",
        ];

        let mut generated_reports = Vec::new();

        for report_type in report_types {
            println!("  📋 Generating: {}", report_type);
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let report_url = format!(
                "/reports/{}/{}.html",
                chrono::Utc::now().format("%Y%m%d"),
                report_type
            );
            generated_reports.push(serde_json::json!({
                "type": report_type,
                "url": report_url,
                "generated_at": chrono::Utc::now().to_rfc3339()
            }));
        }

        context.insert("generated_reports", serde_json::json!(generated_reports))?;
        context.insert(
            "pipeline_completed_at",
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        )?;

        println!("✅ Generated {} reports", generated_reports.len());

        // Output summary
        if let Some(total_records) = metrics.get("total_records").and_then(|v| v.as_u64()) {
            println!("🎉 Analytics pipeline completed successfully!");
            println!("   📊 Processed {} total records", total_records);
            println!("   📋 Generated {} reports", generated_reports.len());
        }

        Ok(())
    }
}

/// Marketing Campaign Workflow - Another workflow package example
///
/// Demonstrates how multiple workflow packages can coexist without conflicts
/// thanks to namespace isolation.
#[packaged_workflow(
    package = "marketing_campaigns",
    version = "1.3.0",
    description = "Automated marketing campaign management and optimization",
    author = "Marketing Automation Team"
)]
pub mod marketing_workflow {
    use super::*;

    /// Segment customer base for targeted campaigns
    #[task(
        id = "segment_customers",
        dependencies = [],
        retry_attempts = 2
    )]
    pub async fn segment_customers(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        println!("👥 Segmenting customers for campaigns...");

        // Simulate customer segmentation
        let segments = vec![
            ("high_value", 1200),
            ("medium_value", 3500),
            ("new_customers", 800),
            ("at_risk", 450),
        ];

        let mut segment_data = Vec::new();
        for (segment_name, count) in segments {
            println!("  📊 Segment '{}': {} customers", segment_name, count);
            segment_data.push(serde_json::json!({
                "name": segment_name,
                "count": count,
                "created_at": chrono::Utc::now().to_rfc3339()
            }));
        }

        context.insert("customer_segments", serde_json::json!(segment_data))?;
        println!("✅ Customer segmentation completed");
        Ok(())
    }

    /// Create personalized campaign content
    #[task(
        id = "create_campaigns",
        dependencies = ["segment_customers"],
        retry_attempts = 3
    )]
    pub async fn create_campaigns(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        println!("📝 Creating personalized campaigns...");

        let default_segments = vec![];
        let segments = context
            .get("customer_segments")
            .and_then(|v| v.as_array())
            .unwrap_or(&default_segments);

        let mut campaigns = Vec::new();

        for segment in segments {
            if let Some(segment_name) = segment.get("name").and_then(|v| v.as_str()) {
                println!("  📧 Creating campaign for: {}", segment_name);
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

                campaigns.push(serde_json::json!({
                    "segment": segment_name,
                    "campaign_id": format!("camp_{}_{}",
                        segment_name,
                        chrono::Utc::now().timestamp()
                    ),
                    "status": "ready",
                    "created_at": chrono::Utc::now().to_rfc3339()
                }));
            }
        }

        context.insert("campaigns", serde_json::json!(campaigns))?;
        println!("✅ Created {} personalized campaigns", campaigns.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analytics_pipeline_metadata() {
        let metadata = analytics_workflow::get_package_metadata();
        assert_eq!(metadata.package, "analytics_pipeline");
        assert_eq!(metadata.version, "2.1.0");
        assert!(metadata.description.contains("analytics"));
        assert!(!metadata.fingerprint.is_empty());
    }

    #[tokio::test]
    async fn test_marketing_pipeline_metadata() {
        let metadata = marketing_workflow::get_package_metadata();
        assert_eq!(metadata.package, "marketing_campaigns");
        assert_eq!(metadata.version, "1.3.0");
        assert!(metadata.description.contains("marketing"));
    }

    #[tokio::test]
    async fn test_task_registration_simulation() {
        // Test that task registration function exists and can be called
        // In a real scenario, these would register tasks in the global registry
        analytics_workflow::register_package_tasks("test_tenant", "test_workflow");
        marketing_workflow::register_package_tasks("test_tenant", "test_workflow");

        // No assertions needed - if this compiles and runs, the registration functions exist
    }

    #[tokio::test]
    async fn test_analytics_workflow_tasks() {
        let mut context = Context::new();

        // Test extract_data task
        analytics_workflow::extract_data(&mut context)
            .await
            .unwrap();
        assert!(context.get("extracted_records").is_some());

        // Test validate_data task
        analytics_workflow::validate_data(&mut context)
            .await
            .unwrap();
        assert!(context.get("valid_records").is_some());

        // Test transform_data task
        analytics_workflow::transform_data(&mut context)
            .await
            .unwrap();
        assert!(context.get("analytics_metrics").is_some());

        // Test generate_reports task
        analytics_workflow::generate_reports(&mut context)
            .await
            .unwrap();
        assert!(context.get("generated_reports").is_some());
    }

    #[tokio::test]
    async fn test_marketing_workflow_tasks() {
        let mut context = Context::new();

        // Test segment_customers task
        marketing_workflow::segment_customers(&mut context)
            .await
            .unwrap();
        assert!(context.get("customer_segments").is_some());

        // Test create_campaigns task
        marketing_workflow::create_campaigns(&mut context)
            .await
            .unwrap();
        assert!(context.get("campaigns").is_some());
    }
}
