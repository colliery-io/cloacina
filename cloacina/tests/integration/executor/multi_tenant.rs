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

//! Integration tests for multi-tenant functionality

mod postgres_multi_tenant_tests {
    use cloacina::runner::DefaultRunner;
    use cloacina::PipelineError;
    use std::env;

    /// Test that schema-based multi-tenancy provides complete isolation
    #[tokio::test]
    async fn test_schema_isolation() -> Result<(), Box<dyn std::error::Error>> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
        });

        // Create two executors with different schemas
        let tenant_a = DefaultRunner::with_schema(&database_url, "tenant_a").await?;
        let tenant_b = DefaultRunner::with_schema(&database_url, "tenant_b").await?;

        // TODO: Add workflow execution and verify isolation
        // This would require implementing a test workflow first

        // Shutdown executors
        tenant_a.shutdown().await?;
        tenant_b.shutdown().await?;

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
    use cloacina::runner::DefaultRunner;

    /// Test that SQLite multi-tenancy works with separate database files
    #[tokio::test]
    async fn test_sqlite_file_isolation() -> Result<(), Box<dyn std::error::Error>> {
        // Create two executors with different database files
        let tenant_a = DefaultRunner::new("sqlite://tenant_a_test.db").await?;
        let tenant_b = DefaultRunner::new("sqlite://tenant_b_test.db").await?;

        // TODO: Add workflow execution and verify isolation
        // This would require implementing a test workflow first

        // Shutdown executors
        tenant_a.shutdown().await?;
        tenant_b.shutdown().await?;

        // Clean up test files
        let _ = std::fs::remove_file("tenant_a_test.db");
        let _ = std::fs::remove_file("tenant_b_test.db");

        Ok(())
    }

    /// Test that SQLite creates separate database files
    #[tokio::test]
    async fn test_sqlite_separate_files() -> Result<(), Box<dyn std::error::Error>> {
        let executor = DefaultRunner::new("sqlite://multi_tenant_test.db").await?;

        // Verify the file was created
        assert!(std::path::Path::new("multi_tenant_test.db").exists());

        executor.shutdown().await?;

        // Clean up
        let _ = std::fs::remove_file("multi_tenant_test.db");

        Ok(())
    }
}
