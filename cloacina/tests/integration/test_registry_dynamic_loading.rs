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

//! Unit tests for registry dynamic loading functionality.

use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::watch;
use uuid::Uuid;

use cloacina::registry::loader::{PackageLoader, TaskRegistrar};
use cloacina::registry::reconciler::{ReconcilerConfig, RegistryReconciler};
use cloacina::registry::storage::FilesystemRegistryStorage;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::types::WorkflowMetadata;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::Database;

/// Test that the reconciler can be created with dynamic loading components
/// This test uses SQLite in-memory database
#[tokio::test]
async fn test_reconciler_creation_with_loaders() {
    // Create temporary database
    let database = Database::new(":memory:", "", 5);

    // Run migrations
    let conn = database.pool().get().await.unwrap();
    conn.interact(move |conn| cloacina::database::run_migrations_sqlite(conn))
        .await
        .unwrap()
        .unwrap();

    // Create temporary storage
    let temp_dir = TempDir::new().unwrap();
    let storage = FilesystemRegistryStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Create workflow registry
    let workflow_registry = WorkflowRegistryImpl::new(storage, database).unwrap();
    let workflow_registry_arc = Arc::new(workflow_registry);

    // Create reconciler config
    let config = ReconcilerConfig::default();

    // Create shutdown channel
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);

    // Create reconciler - this should succeed with the new dynamic loading components
    let reconciler = RegistryReconciler::new(workflow_registry_arc, config, shutdown_rx);

    assert!(
        reconciler.is_ok(),
        "Reconciler should be created successfully with dynamic loading components"
    );
}

/// Test that PackageLoader can be created and used for metadata extraction
#[tokio::test]
async fn test_package_loader_creation() {
    let loader = PackageLoader::new();
    assert!(
        loader.is_ok(),
        "PackageLoader should be created successfully"
    );

    let loader = loader.unwrap();

    // Test with invalid package data - should fail gracefully
    let invalid_data = b"not a valid package".to_vec();
    let result = loader.extract_metadata(&invalid_data).await;
    assert!(result.is_err(), "Should fail with invalid package data");
}

/// Test that TaskRegistrar can be created and used for task registration
#[tokio::test]
async fn test_task_registrar_creation() {
    let registrar = TaskRegistrar::new();
    assert!(
        registrar.is_ok(),
        "TaskRegistrar should be created successfully"
    );

    let registrar = registrar.unwrap();

    // Check initial state
    assert_eq!(registrar.loaded_package_count(), 0);
    assert_eq!(registrar.total_registered_tasks(), 0);

    // Test unregistering non-existent package - should succeed (idempotent)
    let result = registrar.unregister_package_tasks("nonexistent");
    assert!(
        result.is_ok(),
        "Unregistering non-existent package should succeed"
    );
}

/// Test reconciler status functionality
/// This test uses SQLite in-memory database
#[tokio::test]
async fn test_reconciler_status() {
    // Create test components
    let database = Database::new(":memory:", "", 5);
    let conn = database.pool().get().await.unwrap();
    conn.interact(move |conn| cloacina::database::run_migrations_sqlite(conn))
        .await
        .unwrap()
        .unwrap();

    let temp_dir = TempDir::new().unwrap();
    let storage = FilesystemRegistryStorage::new(temp_dir.path().to_path_buf()).unwrap();
    let workflow_registry = WorkflowRegistryImpl::new(storage, database).unwrap();
    let workflow_registry_arc = Arc::new(workflow_registry);

    let config = ReconcilerConfig::default();
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);

    // Create reconciler
    let reconciler = RegistryReconciler::new(workflow_registry_arc, config, shutdown_rx).unwrap();

    // Get initial status
    let status = reconciler.get_status().await;

    // Should start with no packages loaded
    assert_eq!(status.packages_loaded, 0);
    assert_eq!(status.package_details.len(), 0);
}

/// Test reconciler configuration options
#[test]
fn test_reconciler_config() {
    let config = ReconcilerConfig::default();

    // Test default values
    assert_eq!(config.reconcile_interval.as_secs(), 30);
    assert!(config.enable_startup_reconciliation);
    assert_eq!(config.package_operation_timeout.as_secs(), 30);
    assert!(config.continue_on_package_error);
    assert_eq!(config.default_tenant_id, "public");

    // Test custom config
    let custom_config = ReconcilerConfig {
        reconcile_interval: std::time::Duration::from_secs(60),
        enable_startup_reconciliation: false,
        package_operation_timeout: std::time::Duration::from_secs(60),
        continue_on_package_error: false,
        default_tenant_id: "custom".to_string(),
    };

    assert_eq!(custom_config.reconcile_interval.as_secs(), 60);
    assert!(!custom_config.enable_startup_reconciliation);
    assert_eq!(custom_config.package_operation_timeout.as_secs(), 60);
    assert!(!custom_config.continue_on_package_error);
    assert_eq!(custom_config.default_tenant_id, "custom");
}

/// Test that loader components handle errors gracefully
#[tokio::test]
async fn test_loader_error_handling() {
    // Test PackageLoader error handling
    let loader = PackageLoader::new().unwrap();

    // Empty data
    let result = loader.extract_metadata(&[]).await;
    assert!(result.is_err(), "Should fail with empty data");

    // Invalid binary format
    let invalid_data = vec![0x00; 100];
    let result = loader.extract_metadata(&invalid_data).await;
    assert!(result.is_err(), "Should fail with invalid binary format");

    // Test TaskRegistrar error handling
    let registrar = TaskRegistrar::new().unwrap();

    // Create mock metadata
    use cloacina::registry::loader::package_loader::{PackageMetadata, TaskMetadata};

    let task_metadata = TaskMetadata {
        index: 0,
        local_id: "test_task".to_string(),
        namespaced_id_template: "{tenant_id}/{package_name}/test_task".to_string(),
        dependencies: vec![],
        description: "Test task".to_string(),
        source_location: "test.rs:1".to_string(),
    };

    let package_metadata = PackageMetadata {
        package_name: "test_package".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test package".to_string()),
        author: Some("Test Author".to_string()),
        tasks: vec![task_metadata],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec!["cloacina_execute_task".to_string()],
    };

    // Try to register with invalid binary data
    let invalid_data = b"invalid binary".to_vec();
    let result = registrar
        .register_package_tasks(
            "test_package",
            &invalid_data,
            &package_metadata,
            Some("test_tenant"),
        )
        .await;

    assert!(result.is_err(), "Should fail with invalid binary data");
}

/// Test reconciler result types
#[test]
fn test_reconcile_result_methods() {
    use cloacina::registry::reconciler::ReconcileResult;
    use std::time::Duration;

    // Test result with changes
    let result_with_changes = ReconcileResult {
        packages_loaded: vec![Uuid::new_v4()],
        packages_unloaded: vec![],
        packages_failed: vec![],
        total_packages_tracked: 1,
        reconciliation_duration: Duration::from_millis(100),
    };

    assert!(result_with_changes.has_changes());
    assert!(!result_with_changes.has_failures());

    // Test result with failures
    let result_with_failures = ReconcileResult {
        packages_loaded: vec![],
        packages_unloaded: vec![],
        packages_failed: vec![(Uuid::new_v4(), "Test error".to_string())],
        total_packages_tracked: 0,
        reconciliation_duration: Duration::from_millis(50),
    };

    assert!(!result_with_failures.has_changes());
    assert!(result_with_failures.has_failures());

    // Test result with no changes or failures
    let result_empty = ReconcileResult {
        packages_loaded: vec![],
        packages_unloaded: vec![],
        packages_failed: vec![],
        total_packages_tracked: 5,
        reconciliation_duration: Duration::from_millis(25),
    };

    assert!(!result_empty.has_changes());
    assert!(!result_empty.has_failures());
}
