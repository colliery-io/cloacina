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

use cloacina::Context;
use packaged_workflow_example::{analytics_workflow, marketing_workflow};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running Packaged Workflows Demo\n");

    // Analytics Pipeline Execution
    println!("=======================================");
    println!("ANALYTICS PIPELINE EXECUTION");
    println!("=======================================\n");

    let mut analytics_context = Context::new();

    println!("Step 1: Data Extraction");
    analytics_workflow::extract_data(&mut analytics_context).await?;
    println!();

    println!("Step 2: Data Validation");
    analytics_workflow::validate_data(&mut analytics_context).await?;
    println!();

    println!("Step 3: Data Transformation");
    analytics_workflow::transform_data(&mut analytics_context).await?;
    println!();

    println!("Step 4: Report Generation");
    analytics_workflow::generate_reports(&mut analytics_context).await?;
    println!();

    // Show final context state
    println!("Final Analytics Context:");
    if let Some(reports) = analytics_context.get("generated_reports") {
        println!("   Generated Reports: {}", reports);
    }
    if let Some(metrics) = analytics_context.get("analytics_metrics") {
        println!("   Analytics Metrics: {}", metrics);
    }
    println!();

    // Marketing Campaign Execution
    println!("=======================================");
    println!("MARKETING CAMPAIGN EXECUTION");
    println!("=======================================\n");

    let mut marketing_context = Context::new();

    println!("Step 1: Customer Segmentation");
    marketing_workflow::segment_customers(&mut marketing_context).await?;
    println!();

    println!("Step 2: Campaign Creation");
    marketing_workflow::create_campaigns(&mut marketing_context).await?;
    println!();

    // Show final context state
    println!("Final Marketing Context:");
    if let Some(segments) = marketing_context.get("customer_segments") {
        println!("   Customer Segments: {}", segments);
    }
    if let Some(campaigns) = marketing_context.get("campaigns") {
        println!("   Created Campaigns: {}", campaigns);
    }
    println!();

    // Show package information
    println!("=======================================");
    println!("PACKAGE INFORMATION");
    println!("=======================================\n");

    let analytics_meta = analytics_workflow::get_package_metadata();
    let marketing_meta = marketing_workflow::get_package_metadata();

    println!(
        "Analytics Package: {} v{}",
        analytics_meta.package, analytics_meta.version
    );
    println!(
        "Marketing Package: {} v{}",
        marketing_meta.package, marketing_meta.version
    );
    println!();

    println!("All packaged workflows executed successfully!");
    println!("These workflows can be compiled to .so files for dynamic loading");

    Ok(())
}
