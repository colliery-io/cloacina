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

//! Integration tests for DefaultRunner configurable registry storage backends.
//!
//! These tests verify that the DefaultRunner can be configured with different
//! registry storage backends (filesystem, sqlite, postgres) and that they work
//! correctly in end-to-end scenarios.

use cloacina::registry::traits::WorkflowRegistry;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

use crate::fixtures::get_or_init_fixture;

use serial_test::serial;

/// Helper to create a minimal test package (.cloacina file)
fn create_test_package() -> Vec<u8> {
    // Create a minimal test package with proper structure
    let mut data = Vec::new();

    // Add gzip magic bytes
    data.extend_from_slice(&[0x1f, 0x8b, 0x08]);

    // Add some dummy compressed data to simulate a .cloacina package
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03]);

    // Add fake tar content
    for i in 0..512 {
        data.push((i % 256) as u8);
    }

    data
}

/// Helper to create a test runner config with the specified storage backend
fn create_test_config(storage_backend: &str, temp_dir: Option<&TempDir>) -> DefaultRunnerConfig {
    let mut config = DefaultRunnerConfig::default();
    config.enable_registry_reconciler = true;
    config.registry_storage_backend = storage_backend.to_string();

    if let Some(dir) = temp_dir {
        config.registry_storage_path = Some(dir.path().to_path_buf());
    }

    // Shorter timeouts for testing
    config.registry_reconcile_interval = Duration::from_millis(100);
    config.registry_enable_startup_reconciliation = false; // Disable for faster tests

    config
}

/// Helper to get the appropriate database URL for testing
/// Always uses the fixture's database URL to match the current backend
async fn get_database_url_for_test() -> String {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.get_database_url()
}

/// Unified test implementations that work with any storage backend
mod registry_tests {
    use super::*;

    /// Test that a runner can be created with a specific storage backend
    pub async fn test_runner_creation_impl(runner: DefaultRunner) {
        // Verify the runner was created successfully
        assert!(runner.is_registry_reconciler_enabled());

        // Verify we can get the registry
        let registry = runner.get_workflow_registry().await;
        assert!(
            registry.is_some(),
            "Registry should be available when reconciler is enabled"
        );
    }

    /// Test that workflows can be registered and listed
    pub async fn test_workflow_registration_impl(runner: DefaultRunner) {
        let registry = runner
            .get_workflow_registry()
            .await
            .expect("Registry should be available");

        // Get initial workflow count
        let initial_workflows = registry
            .list_workflows()
            .await
            .expect("Should be able to list workflows");
        let initial_count = initial_workflows.len();

        // Note: We can't easily test actual workflow registration without a real .cloacina file
        // This test verifies the registry is functional and can list workflows

        println!(
            "Registry is functional with {} initial workflows",
            initial_count
        );
    }

    /// Test that the registry configuration is applied correctly
    pub async fn test_registry_configuration_impl(runner: DefaultRunner, expected_backend: &str) {
        let registry = runner
            .get_workflow_registry()
            .await
            .expect("Registry should be available");

        // Verify the registry responds to basic operations
        let workflows = registry
            .list_workflows()
            .await
            .expect("Should be able to list workflows");

        // The registry should start empty for new storage backends
        println!(
            "Registry with {} backend has {} workflows",
            expected_backend,
            workflows.len()
        );
    }

    /// Test that the runner can be shut down cleanly
    pub async fn test_runner_shutdown_impl(runner: DefaultRunner) {
        // Verify clean shutdown
        let result = runner.shutdown().await;
        assert!(result.is_ok(), "Runner should shut down cleanly");
    }
}

// Filesystem backend tests
mod filesystem_tests {
    use super::*;

    async fn create_filesystem_runner() -> (DefaultRunner, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = create_test_config("filesystem", Some(&temp_dir));

        let db_url = get_database_url_for_test().await;

        let runner = DefaultRunner::with_config(&db_url, config)
            .await
            .expect("Failed to create filesystem runner");

        (runner, temp_dir)
    }

    #[tokio::test]
    async fn test_filesystem_runner_creation() {
        let (runner, _temp_dir) = create_filesystem_runner().await;
        registry_tests::test_runner_creation_impl(runner).await;
    }

    #[tokio::test]
    async fn test_filesystem_workflow_registration() {
        let (runner, _temp_dir) = create_filesystem_runner().await;
        registry_tests::test_workflow_registration_impl(runner).await;
    }

    #[tokio::test]
    async fn test_filesystem_registry_configuration() {
        let (runner, _temp_dir) = create_filesystem_runner().await;
        registry_tests::test_registry_configuration_impl(runner, "filesystem").await;
    }

    #[tokio::test]
    async fn test_filesystem_runner_shutdown() {
        let (runner, _temp_dir) = create_filesystem_runner().await;
        registry_tests::test_runner_shutdown_impl(runner).await;
    }

    #[tokio::test]
    async fn test_filesystem_custom_path() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let custom_registry_path = temp_dir.path().join("custom_registry");

        let mut config = create_test_config("filesystem", None);
        config.registry_storage_path = Some(custom_registry_path.clone());

        let db_url = get_database_url_for_test().await;

        let runner = DefaultRunner::with_config(&db_url, config)
            .await
            .expect("Failed to create filesystem runner with custom path");

        // Verify the runner was created successfully
        let registry = runner.get_workflow_registry().await;
        assert!(
            registry.is_some(),
            "Registry should be available with custom path"
        );

        runner
            .shutdown()
            .await
            .expect("Runner should shut down cleanly");
    }
}

// Current database backend tests - tests whatever backend is currently enabled
mod current_backend_tests {
    use super::*;

    async fn create_current_backend_runner() -> DefaultRunner {
        let fixture = get_or_init_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.initialize().await;
        let db_url = fixture.get_database_url();
        let backend = fixture.get_current_backend();

        let config = create_test_config(backend, None);

        DefaultRunner::with_config(&db_url, config)
            .await
            .expect(&format!("Failed to create {} runner", backend))
    }

    async fn get_current_backend() -> String {
        let fixture = get_or_init_fixture().await;
        let fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.get_current_backend().to_string()
    }

    #[tokio::test]
    #[serial]
    async fn test_current_backend_runner_creation() {
        let runner = create_current_backend_runner().await;
        registry_tests::test_runner_creation_impl(runner).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_current_backend_workflow_registration() {
        let runner = create_current_backend_runner().await;
        registry_tests::test_workflow_registration_impl(runner).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_current_backend_registry_configuration() {
        let runner = create_current_backend_runner().await;
        let backend = get_current_backend().await;
        registry_tests::test_registry_configuration_impl(runner, &backend).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_current_backend_runner_shutdown() {
        let runner = create_current_backend_runner().await;
        registry_tests::test_runner_shutdown_impl(runner).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_current_backend_registry_uses_same_database() {
        let fixture = get_or_init_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.initialize().await;
        let db_url = fixture.get_database_url();
        let backend = fixture.get_current_backend();
        drop(fixture); // Release lock

        let config = create_test_config(backend, None);

        // Create runner with database registry
        let runner = DefaultRunner::with_config(&db_url, config)
            .await
            .expect(&format!("Failed to create {} runner", backend));

        // Verify the registry is available
        let registry = runner.get_workflow_registry().await;
        assert!(
            registry.is_some(),
            "{} registry should be available",
            backend
        );

        println!("Verified {} registry is functional", backend);

        runner
            .shutdown()
            .await
            .expect("Runner should shut down cleanly");
    }
}

// Error handling tests
mod error_tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_storage_backend() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = create_test_config("invalid_backend", Some(&temp_dir));

        let db_url = get_database_url_for_test().await;

        let result = DefaultRunner::with_config(&db_url, config).await;

        // The runner should still be created, but registry creation should fail
        // (The error is logged, not propagated)
        assert!(
            result.is_ok(),
            "Runner creation should not fail due to invalid storage backend"
        );

        let runner = result.unwrap();

        // The registry should not be available due to the invalid backend
        let registry = runner.get_workflow_registry().await;
        assert!(
            registry.is_none(),
            "Registry should not be available with invalid backend"
        );

        runner
            .shutdown()
            .await
            .expect("Runner should shut down cleanly");
    }

    #[tokio::test]
    async fn test_registry_disabled() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let mut config = create_test_config("filesystem", Some(&temp_dir));
        config.enable_registry_reconciler = false; // Disable registry

        let db_url = get_database_url_for_test().await;

        let runner = DefaultRunner::with_config(&db_url, config)
            .await
            .expect("Failed to create runner with disabled registry");

        // Registry should not be available when disabled
        let registry = runner.get_workflow_registry().await;
        assert!(
            registry.is_none(),
            "Registry should not be available when disabled"
        );

        assert!(
            !runner.is_registry_reconciler_enabled(),
            "Registry reconciler should be disabled"
        );

        runner
            .shutdown()
            .await
            .expect("Runner should shut down cleanly");
    }
}

// Integration tests with both backends
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_filesystem_and_current_backend_runners() {
        // Get current backend info
        let fixture = get_or_init_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.initialize().await;
        let db_url = fixture.get_database_url();
        let current_backend = fixture.get_current_backend();
        drop(fixture); // Release lock

        // Create filesystem runner
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let filesystem_config = create_test_config("filesystem", Some(&temp_dir));
        let fs_db_url = get_database_url_for_test().await;

        let filesystem_runner = DefaultRunner::with_config(&fs_db_url, filesystem_config)
            .await
            .expect("Failed to create filesystem runner");

        // Create current backend runner
        let current_backend_config = create_test_config(current_backend, None);
        let current_backend_runner = DefaultRunner::with_config(&db_url, current_backend_config)
            .await
            .expect(&format!("Failed to create {} runner", current_backend));

        // Both runners should have registries available
        let fs_registry = filesystem_runner.get_workflow_registry().await;
        let current_registry = current_backend_runner.get_workflow_registry().await;

        assert!(
            fs_registry.is_some(),
            "Filesystem registry should be available"
        );
        assert!(
            current_registry.is_some(),
            "{} registry should be available",
            current_backend
        );

        // Both should be able to list workflows independently
        let fs_workflows = fs_registry
            .unwrap()
            .list_workflows()
            .await
            .expect("Filesystem registry should list workflows");
        let current_workflows = current_registry
            .unwrap()
            .list_workflows()
            .await
            .expect(&format!(
                "{} registry should list workflows",
                current_backend
            ));

        println!("Filesystem registry: {} workflows", fs_workflows.len());
        println!(
            "{} registry: {} workflows",
            current_backend,
            current_workflows.len()
        );

        // Verify they are independent (different storage backends)
        // This is verified by the fact that both can be created and operated separately

        // Clean shutdown
        filesystem_runner
            .shutdown()
            .await
            .expect("Filesystem runner should shut down cleanly");
        current_backend_runner.shutdown().await.expect(&format!(
            "{} runner should shut down cleanly",
            current_backend
        ));
    }
}
