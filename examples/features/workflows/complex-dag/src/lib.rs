/*
 *  Copyright 2025-2026 Colliery Software
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

//! Complex DAG Example
//!
//! This example demonstrates a sophisticated workflow with:
//! - Multiple root tasks (no dependencies)
//! - Tasks with multiple dependencies
//! - Diamond patterns (diverge and converge)
//! - Parallel execution paths
//! - Complex branching and merging

use cloacina::{Context, TaskError};
use cloacina_macros::{task, workflow};

// I-0102 / T-C: unified plugin shell.
cloacina_workflow_plugin::package!();

#[workflow(
    name = "complex_dag_workflow",
    description = "Complex DAG structure for testing visualization capabilities",
    author = "Cloacina Team"
)]
mod complex_dag_workflow {
    use super::*;

    // ============================================================================
    // Level 0: Root tasks (no dependencies) - these can run in parallel
    // ============================================================================

    #[task]
    pub async fn init_config(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("config_loaded", serde_json::Value::Bool(true))?;
        println!("Configuration initialized");
        Ok(())
    }

    #[task]
    pub async fn init_database(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("database_ready", serde_json::Value::Bool(true))?;
        println!("Database connection established");
        Ok(())
    }

    #[task]
    pub async fn init_logging(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("logging_enabled", serde_json::Value::Bool(true))?;
        println!("Logging system initialized");
        Ok(())
    }

    // ============================================================================
    // Level 1: Second level - depends on specific root tasks
    // ============================================================================

    #[task(dependencies = ["init_database"])]
    pub async fn load_schema(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("schema_loaded", serde_json::Value::Bool(true))?;
        println!("Database schema loaded");
        Ok(())
    }

    #[task(dependencies = ["init_config"])]
    pub async fn setup_security(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("security_configured", serde_json::Value::Bool(true))?;
        println!("Security configuration applied");
        Ok(())
    }

    #[task(dependencies = ["init_logging", "init_config"])]
    pub async fn configure_monitoring(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("monitoring_enabled", serde_json::Value::Bool(true))?;
        println!("Monitoring configured");
        Ok(())
    }

    // ============================================================================
    // Level 2: Third level - more complex dependencies
    // ============================================================================

    #[task(dependencies = ["load_schema", "setup_security"])]
    pub async fn create_tables(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("tables_created", serde_json::Value::Bool(true))?;
        println!("Database tables created");
        Ok(())
    }

    #[task(dependencies = ["load_schema"])]
    pub async fn setup_cache(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("cache_ready", serde_json::Value::Bool(true))?;
        println!("Caching layer configured");
        Ok(())
    }

    // ============================================================================
    // Level 3: Data processing branch
    // ============================================================================

    #[task(dependencies = ["create_tables"])]
    pub async fn load_raw_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("raw_data_loaded", serde_json::Value::Bool(true))?;
        println!("Raw data loaded into staging tables");
        Ok(())
    }

    #[task(dependencies = ["load_raw_data"])]
    pub async fn validate_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("data_validated", serde_json::Value::Bool(true))?;
        println!("Data validation completed");
        Ok(())
    }

    #[task(dependencies = ["validate_data"])]
    pub async fn clean_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("data_cleaned", serde_json::Value::Bool(true))?;
        println!("Data cleaning process completed");
        Ok(())
    }

    // ============================================================================
    // Level 4: Parallel transformation tasks
    // ============================================================================

    #[task(dependencies = ["clean_data"])]
    pub async fn transform_customers(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("customers_transformed", serde_json::Value::Bool(true))?;
        println!("Customer data transformed");
        Ok(())
    }

    #[task(dependencies = ["clean_data"])]
    pub async fn transform_orders(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("orders_transformed", serde_json::Value::Bool(true))?;
        println!("Order data transformed");
        Ok(())
    }

    #[task(dependencies = ["clean_data"])]
    pub async fn transform_products(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("products_transformed", serde_json::Value::Bool(true))?;
        println!("Product data transformed");
        Ok(())
    }

    // ============================================================================
    // Level 5: Aggregation tasks that depend on multiple transformations
    // ============================================================================

    #[task(dependencies = ["transform_customers", "transform_orders"])]
    pub async fn calculate_metrics(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("metrics_calculated", serde_json::Value::Bool(true))?;
        println!("Business metrics calculated");
        Ok(())
    }

    #[task(dependencies = ["transform_products", "calculate_metrics"])]
    pub async fn generate_insights(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("insights_generated", serde_json::Value::Bool(true))?;
        println!("Business insights generated");
        Ok(())
    }

    // ============================================================================
    // Level 6: Reporting branch - depends on cache and insights
    // ============================================================================

    #[task(dependencies = ["setup_cache", "generate_insights"])]
    pub async fn build_dashboard(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("dashboard_built", serde_json::Value::Bool(true))?;
        println!("Analytics dashboard built");
        Ok(())
    }

    #[task(dependencies = ["calculate_metrics", "configure_monitoring"])]
    pub async fn generate_reports(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("reports_generated", serde_json::Value::Bool(true))?;
        println!("Automated reports generated");
        Ok(())
    }

    // ============================================================================
    // Level 7: Final convergence tasks
    // ============================================================================

    #[task(dependencies = ["build_dashboard", "generate_reports"])]
    pub async fn send_notifications(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("notifications_sent", serde_json::Value::Bool(true))?;
        println!("Completion notifications sent");
        Ok(())
    }

    #[task(dependencies = ["send_notifications"])]
    pub async fn cleanup_staging(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        context.insert("staging_cleaned", serde_json::Value::Bool(true))?;
        println!("Staging data cleaned up");
        Ok(())
    }
}
