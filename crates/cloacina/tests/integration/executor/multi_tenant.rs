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

//! Integration tests for multi-tenant functionality

mod postgres_multi_tenant_tests {
    use cloacina::context::Context;
    use cloacina::dal::DAL;
    use cloacina::database::universal_types::UniversalUuid;
    use cloacina::executor::PipelineExecutor;
    use cloacina::runner::DefaultRunner;
    use cloacina::*;
    use cloacina::{register_task_constructor, register_workflow_constructor};
    use serde_json::Value;
    use std::env;
    use std::sync::Arc;
    use std::time::Duration;

    /// Simple task that marks its tenant in the context
    #[task(id = "tenant_marker_task", dependencies = [])]
    async fn tenant_marker_task(context: &mut Context<Value>) -> Result<(), TaskError> {
        // Just mark that we executed
        context.insert("executed", Value::Bool(true))?;
        Ok(())
    }

    /// Helper to create and register a workflow for a specific tenant schema
    fn setup_tenant_workflow(tenant_schema: &str) -> Workflow {
        let workflow_name = format!("isolation_test_{}", tenant_schema);

        let workflow = Workflow::builder(&workflow_name)
            .tenant(tenant_schema)
            .description("Test workflow for multi-tenant isolation")
            .add_task(Arc::new(tenant_marker_task_task()))
            .unwrap()
            .build()
            .unwrap();

        // Register task constructor
        let namespace = TaskNamespace::new(
            workflow.tenant(),
            workflow.package(),
            workflow.name(),
            "tenant_marker_task",
        );
        register_task_constructor(namespace, || Arc::new(tenant_marker_task_task()));

        // Register workflow constructor
        register_workflow_constructor(workflow.name().to_string(), {
            let workflow = workflow.clone();
            move || workflow.clone()
        });

        workflow
    }

    /// Test that schema-based multi-tenancy provides complete data isolation
    #[tokio::test]
    async fn test_schema_isolation() -> Result<(), Box<dyn std::error::Error>> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
        });

        // Create two runners with different schemas
        let runner_a = DefaultRunner::with_schema(&database_url, "tenant_iso_a").await?;
        let runner_b = DefaultRunner::with_schema(&database_url, "tenant_iso_b").await?;

        // Setup workflows for each tenant
        let workflow_a = setup_tenant_workflow("tenant_iso_a");
        let workflow_b = setup_tenant_workflow("tenant_iso_b");

        // Execute workflow in tenant A
        let context_a = Context::new();
        let execution_a = runner_a.execute_async(workflow_a.name(), context_a).await?;

        // Wait for execution to complete
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Get DALs for each tenant to verify isolation
        let dal_a = DAL::new(runner_a.database().clone());
        let dal_b = DAL::new(runner_b.database().clone());

        // Verify tenant A can see their execution
        let executions_a = dal_a.pipeline_execution().list_recent(100).await?;
        assert!(
            executions_a
                .iter()
                .any(|e| e.id == UniversalUuid(execution_a.execution_id)),
            "Tenant A should see their own execution"
        );

        // Verify tenant B cannot see tenant A's execution (isolation)
        let executions_b = dal_b.pipeline_execution().list_recent(100).await?;
        assert!(
            !executions_b
                .iter()
                .any(|e| e.id == UniversalUuid(execution_a.execution_id)),
            "Tenant B should NOT see tenant A's execution - isolation violated!"
        );

        // Now execute in tenant B
        let context_b = Context::new();
        let execution_b = runner_b.execute_async(workflow_b.name(), context_b).await?;

        // Wait for execution to complete
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Refresh execution lists
        let executions_a = dal_a.pipeline_execution().list_recent(100).await?;
        let executions_b = dal_b.pipeline_execution().list_recent(100).await?;

        // Verify tenant A still only sees their execution
        assert!(
            executions_a
                .iter()
                .any(|e| e.id == UniversalUuid(execution_a.execution_id)),
            "Tenant A should still see their own execution"
        );
        assert!(
            !executions_a
                .iter()
                .any(|e| e.id == UniversalUuid(execution_b.execution_id)),
            "Tenant A should NOT see tenant B's execution"
        );

        // Verify tenant B only sees their execution
        assert!(
            executions_b
                .iter()
                .any(|e| e.id == UniversalUuid(execution_b.execution_id)),
            "Tenant B should see their own execution"
        );
        assert!(
            !executions_b
                .iter()
                .any(|e| e.id == UniversalUuid(execution_a.execution_id)),
            "Tenant B should NOT see tenant A's execution"
        );

        // Shutdown executors
        runner_a.shutdown().await?;
        runner_b.shutdown().await?;

        Ok(())
    }

    /// Test that the same workflow can execute independently in different tenants
    #[tokio::test]
    async fn test_independent_execution() -> Result<(), Box<dyn std::error::Error>> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
        });

        // Create two runners with different schemas
        let runner_a = DefaultRunner::with_schema(&database_url, "tenant_indep_a").await?;
        let runner_b = DefaultRunner::with_schema(&database_url, "tenant_indep_b").await?;

        // Setup workflows
        let workflow_a = setup_tenant_workflow("tenant_indep_a");
        let workflow_b = setup_tenant_workflow("tenant_indep_b");

        // Execute in both tenants simultaneously
        let context_a = Context::new();
        let context_b = Context::new();

        let (execution_a, execution_b) = tokio::join!(
            runner_a.execute_async(workflow_a.name(), context_a),
            runner_b.execute_async(workflow_b.name(), context_b)
        );

        let execution_a = execution_a?;
        let execution_b = execution_b?;

        // Verify both executions have different IDs
        assert_ne!(
            execution_a.execution_id, execution_b.execution_id,
            "Each tenant should have unique execution IDs"
        );

        // Wait for executions
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Verify each tenant has exactly one execution
        let dal_a = DAL::new(runner_a.database().clone());
        let dal_b = DAL::new(runner_b.database().clone());

        let executions_a = dal_a.pipeline_execution().list_recent(100).await?;
        let executions_b = dal_b.pipeline_execution().list_recent(100).await?;

        // Each tenant should have their workflow execution
        let tenant_a_workflows: Vec<_> = executions_a
            .iter()
            .filter(|e| e.pipeline_name.contains("tenant_indep_a"))
            .collect();
        let tenant_b_workflows: Vec<_> = executions_b
            .iter()
            .filter(|e| e.pipeline_name.contains("tenant_indep_b"))
            .collect();

        assert!(
            !tenant_a_workflows.is_empty(),
            "Tenant A should have executions"
        );
        assert!(
            !tenant_b_workflows.is_empty(),
            "Tenant B should have executions"
        );

        // Shutdown
        runner_a.shutdown().await?;
        runner_b.shutdown().await?;

        Ok(())
    }

    /// Test that invalid schema names are rejected
    #[tokio::test]
    async fn test_invalid_schema_names() {
        let database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina";

        // Test schema name with hyphens (should fail)
        let result = DefaultRunner::with_schema(database_url, "tenant-123").await;
        assert!(result.is_err());

        // Test schema name with spaces (should fail)
        let result = DefaultRunner::with_schema(database_url, "tenant 123").await;
        assert!(result.is_err());

        // Test schema name with special characters (should fail)
        let result = DefaultRunner::with_schema(database_url, "tenant@123").await;
        assert!(result.is_err());

        // Test valid schema name (should succeed)
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| database_url.to_string());
        let result = DefaultRunner::with_schema(&database_url, "tenant_123").await;
        if let Ok(executor) = result {
            let _ = executor.shutdown().await;
        }
    }

    /// Test that schema isolation is only supported for PostgreSQL
    #[tokio::test]
    async fn test_sqlite_schema_rejection() {
        let result = DefaultRunner::builder()
            .database_url("sqlite://test.db")
            .schema("tenant_123")
            .build()
            .await;

        assert!(matches!(result, Err(PipelineError::Configuration { .. })));
    }

    /// Test builder pattern for multi-tenant setup
    #[tokio::test]
    async fn test_builder_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
        });

        let executor = DefaultRunner::builder()
            .database_url(&database_url)
            .schema("tenant_builder_test")
            .build()
            .await?;

        executor.shutdown().await?;
        Ok(())
    }
}

mod sqlite_multi_tenant_tests {
    use cloacina::context::Context;
    use cloacina::dal::DAL;
    use cloacina::database::universal_types::UniversalUuid;
    use cloacina::executor::PipelineExecutor;
    use cloacina::runner::DefaultRunner;
    use cloacina::*;
    use cloacina::{register_task_constructor, register_workflow_constructor};
    use serde_json::Value;
    use std::sync::Arc;
    use std::time::Duration;

    /// Simple task for SQLite tests
    #[task(id = "sqlite_tenant_task", dependencies = [])]
    async fn sqlite_tenant_task(context: &mut Context<Value>) -> Result<(), TaskError> {
        context.insert("sqlite_executed", Value::Bool(true))?;
        Ok(())
    }

    /// Helper to create and register a workflow for SQLite tests
    fn setup_sqlite_workflow(db_name: &str) -> Workflow {
        let workflow_name = format!("sqlite_isolation_{}", db_name);

        let workflow = Workflow::builder(&workflow_name)
            .description("Test workflow for SQLite multi-tenant isolation")
            .add_task(Arc::new(sqlite_tenant_task_task()))
            .unwrap()
            .build()
            .unwrap();

        // Register task constructor
        let namespace = TaskNamespace::new(
            workflow.tenant(),
            workflow.package(),
            workflow.name(),
            "sqlite_tenant_task",
        );
        register_task_constructor(namespace, || Arc::new(sqlite_tenant_task_task()));

        // Register workflow constructor
        register_workflow_constructor(workflow.name().to_string(), {
            let workflow = workflow.clone();
            move || workflow.clone()
        });

        workflow
    }

    /// Test that SQLite multi-tenancy works with separate database files
    #[tokio::test]
    async fn test_sqlite_file_isolation() -> Result<(), Box<dyn std::error::Error>> {
        let db_a = "sqlite_tenant_a_iso_test.db";
        let db_b = "sqlite_tenant_b_iso_test.db";

        // Clean up any existing files
        let _ = std::fs::remove_file(db_a);
        let _ = std::fs::remove_file(db_b);

        // Create two executors with different database files
        let runner_a = DefaultRunner::new(&format!("sqlite://{}", db_a)).await?;
        let runner_b = DefaultRunner::new(&format!("sqlite://{}", db_b)).await?;

        // Setup workflows
        let workflow_a = setup_sqlite_workflow("a");
        let workflow_b = setup_sqlite_workflow("b");

        // Execute workflow in tenant A
        let context_a = Context::new();
        let execution_a = runner_a.execute_async(workflow_a.name(), context_a).await?;

        // Wait for execution
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Get DALs
        let dal_a = DAL::new(runner_a.database().clone());
        let dal_b = DAL::new(runner_b.database().clone());

        // Verify tenant A sees their execution
        let executions_a = dal_a.pipeline_execution().list_recent(100).await?;
        assert!(
            executions_a
                .iter()
                .any(|e| e.id == UniversalUuid(execution_a.execution_id)),
            "Tenant A should see their own execution"
        );

        // Verify tenant B has no executions (separate database file = isolation)
        let executions_b = dal_b.pipeline_execution().list_recent(100).await?;
        assert!(
            !executions_b
                .iter()
                .any(|e| e.id == UniversalUuid(execution_a.execution_id)),
            "Tenant B should NOT see tenant A's execution - file isolation"
        );

        // Execute in tenant B
        let context_b = Context::new();
        let execution_b = runner_b.execute_async(workflow_b.name(), context_b).await?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        // Verify isolation after both execute
        let executions_a = dal_a.pipeline_execution().list_recent(100).await?;
        let executions_b = dal_b.pipeline_execution().list_recent(100).await?;

        // Tenant A only sees A's execution
        assert!(executions_a
            .iter()
            .any(|e| e.id == UniversalUuid(execution_a.execution_id)));
        assert!(!executions_a
            .iter()
            .any(|e| e.id == UniversalUuid(execution_b.execution_id)));

        // Tenant B only sees B's execution
        assert!(executions_b
            .iter()
            .any(|e| e.id == UniversalUuid(execution_b.execution_id)));
        assert!(!executions_b
            .iter()
            .any(|e| e.id == UniversalUuid(execution_a.execution_id)));

        // Shutdown executors
        runner_a.shutdown().await?;
        runner_b.shutdown().await?;

        // Clean up test files
        let _ = std::fs::remove_file(db_a);
        let _ = std::fs::remove_file(db_b);

        Ok(())
    }

    /// Test that SQLite creates separate database files
    #[tokio::test]
    async fn test_sqlite_separate_files() -> Result<(), Box<dyn std::error::Error>> {
        let db_file = "multi_tenant_test_sep.db";
        let _ = std::fs::remove_file(db_file);

        let executor = DefaultRunner::new(&format!("sqlite://{}", db_file)).await?;

        // Verify the file was created
        assert!(
            std::path::Path::new(db_file).exists(),
            "Database file should be created"
        );

        executor.shutdown().await?;

        // Clean up
        let _ = std::fs::remove_file(db_file);

        Ok(())
    }
}
