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

//! Complex DAG Example
//! 
//! This example demonstrates a sophisticated workflow with:
//! - Multiple root tasks (no dependencies)
//! - Tasks with multiple dependencies
//! - Diamond patterns (diverge and converge)
//! - Parallel execution paths
//! - Complex branching and merging

use cloacina::{Context, TaskError};
use cloacina_macros::{packaged_workflow, task};

#[packaged_workflow(
    package = "complex_dag_workflow", 
    version = "1.0.0",
    description = "Complex DAG structure for testing visualization capabilities",
    author = "Cloacina Team"
)]
mod complex_dag_workflow {
    use super::*;

    // ============================================================================
    // Level 0: Root tasks (no dependencies) - these can run in parallel
    // ============================================================================

    #[task(id = "init_config", dependencies = [])]
    async fn init_config(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("config_loaded", serde_json::Value::Bool(true))?;
        println!("Configuration initialized");
        Ok(())
    }

    #[task(id = "init_database", dependencies = [])]
    async fn init_database(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("database_ready", serde_json::Value::Bool(true))?;
        println!("Database connection established");
        Ok(())
    }

    #[task(id = "init_logging", dependencies = [])]
    async fn init_logging(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("logging_enabled", serde_json::Value::Bool(true))?;
        println!("Logging system initialized");
        Ok(())
    }

    // ============================================================================
    // Level 1: Second level - depends on specific root tasks
    // ============================================================================

    #[task(id = "load_schema", dependencies = ["init_database"])]
    async fn load_schema(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("schema_loaded", serde_json::Value::Bool(true))?;
        println!("Database schema loaded");
        Ok(())
    }

    #[task(id = "setup_security", dependencies = ["init_config"])]
    async fn setup_security(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("security_configured", serde_json::Value::Bool(true))?;
        println!("Security configuration applied");
        Ok(())
    }

    #[task(id = "configure_monitoring", dependencies = ["init_logging", "init_config"])]
    async fn configure_monitoring(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("monitoring_enabled", serde_json::Value::Bool(true))?;
        println!("Monitoring configured");
        Ok(())
    }

    // ============================================================================
    // Level 2: Third level - more complex dependencies
    // ============================================================================

    #[task(id = "create_tables", dependencies = ["load_schema", "setup_security"])]
    async fn create_tables(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("tables_created", serde_json::Value::Bool(true))?;
        println!("Database tables created");
        Ok(())
    }

    #[task(id = "setup_cache", dependencies = ["load_schema"])]
    async fn setup_cache(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("cache_ready", serde_json::Value::Bool(true))?;
        println!("Caching layer configured");
        Ok(())
    }

    // ============================================================================
    // Level 3: Data processing branch
    // ============================================================================

    #[task(id = "load_raw_data", dependencies = ["create_tables"])]
    async fn load_raw_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("raw_data_loaded", serde_json::Value::Bool(true))?;
        println!("Raw data loaded into staging tables");
        Ok(())
    }

    #[task(id = "validate_data", dependencies = ["load_raw_data"])]
    async fn validate_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("data_validated", serde_json::Value::Bool(true))?;
        println!("Data validation completed");
        Ok(())
    }

    #[task(id = "clean_data", dependencies = ["validate_data"])]
    async fn clean_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("data_cleaned", serde_json::Value::Bool(true))?;
        println!("Data cleaning process completed");
        Ok(())
    }

    // ============================================================================
    // Level 4: Parallel transformation tasks
    // ============================================================================

    #[task(id = "transform_customers", dependencies = ["clean_data"])]
    async fn transform_customers(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("customers_transformed", serde_json::Value::Bool(true))?;
        println!("Customer data transformed");
        Ok(())
    }

    #[task(id = "transform_orders", dependencies = ["clean_data"])]
    async fn transform_orders(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("orders_transformed", serde_json::Value::Bool(true))?;
        println!("Order data transformed");
        Ok(())
    }

    #[task(id = "transform_products", dependencies = ["clean_data"])]
    async fn transform_products(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("products_transformed", serde_json::Value::Bool(true))?;
        println!("Product data transformed");
        Ok(())
    }

    // ============================================================================
    // Level 5: Aggregation tasks that depend on multiple transformations
    // ============================================================================

    #[task(id = "calculate_metrics", dependencies = ["transform_customers", "transform_orders"])]
    async fn calculate_metrics(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("metrics_calculated", serde_json::Value::Bool(true))?;
        println!("Business metrics calculated");
        Ok(())
    }

    #[task(id = "generate_insights", dependencies = ["transform_products", "calculate_metrics"])]
    async fn generate_insights(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("insights_generated", serde_json::Value::Bool(true))?;
        println!("Business insights generated");
        Ok(())
    }

    // ============================================================================
    // Level 6: Reporting branch - depends on cache and insights
    // ============================================================================

    #[task(id = "build_dashboard", dependencies = ["setup_cache", "generate_insights"])]
    async fn build_dashboard(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("dashboard_built", serde_json::Value::Bool(true))?;
        println!("Analytics dashboard built");
        Ok(())
    }

    #[task(id = "generate_reports", dependencies = ["calculate_metrics", "configure_monitoring"])]
    async fn generate_reports(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("reports_generated", serde_json::Value::Bool(true))?;
        println!("Automated reports generated");
        Ok(())
    }

    // ============================================================================
    // Level 7: Final convergence tasks
    // ============================================================================

    #[task(id = "send_notifications", dependencies = ["build_dashboard", "generate_reports"])]
    async fn send_notifications(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("notifications_sent", serde_json::Value::Bool(true))?;
        println!("Completion notifications sent");
        Ok(())
    }

    #[task(id = "cleanup_staging", dependencies = ["send_notifications"])]
    async fn cleanup_staging(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("staging_cleaned", serde_json::Value::Bool(true))?;
        println!("Staging data cleaned up");
        Ok(())
    }
}