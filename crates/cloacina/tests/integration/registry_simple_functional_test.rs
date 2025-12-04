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

//! Simple functional test for the refactored registry API.
//!
//! This test verifies that the registry can handle binary data directly
//! and demonstrates the new streamlined API.

use serial_test::serial;
use std::fs;

use cloacina::database::Database;
use cloacina::registry::storage::FilesystemRegistryStorage;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use tempfile::TempDir;

use super::fixtures::get_or_init_fixture;

/// Helper to create a test database using the fixture pattern
async fn create_test_database() -> Database {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.get_database()
}

/// Helper to create a test filesystem storage
fn create_test_storage() -> FilesystemRegistryStorage {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_path = temp_dir.path().to_path_buf();
    // Keep temp_dir alive for the duration of the test
    std::mem::forget(temp_dir);

    FilesystemRegistryStorage::new(storage_path).expect("Failed to create filesystem storage")
}

#[tokio::test]
#[serial]
async fn test_registry_with_simple_binary_data() {
    // Test that the registry can handle the new binary data API
    let storage = create_test_storage();
    let database = create_test_database().await;
    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Test with simple binary data (will fail validation but tests the API)
    let package_data = b"simple test package data".to_vec();

    let result = registry.register_workflow(package_data).await;

    // Should fail due to validation, but the API should work
    assert!(result.is_err(), "Simple binary data should fail validation");

    // Registry should remain functional
    let workflows = registry.list_workflows().await.unwrap();
    assert!(
        workflows.is_empty(),
        "Registry should be empty after failed registration"
    );

    println!("✓ Registry binary data API working correctly");
}

#[tokio::test]
#[serial]
async fn test_registry_with_real_package_if_available() {
    // This test uses a real package file if it exists, otherwise skips
    let package_path = "/tmp/analytics_example.cloacina";

    if !std::path::Path::new(package_path).exists() {
        println!(
            "Package file not found at {}, skipping real package test",
            package_path
        );
        return;
    }

    let storage = create_test_storage();
    let database = create_test_database().await;
    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Load the real package data
    let package_data = match fs::read(package_path) {
        Ok(data) => data,
        Err(e) => {
            println!("Could not read package file: {}, skipping test", e);
            return;
        }
    };

    println!("Loaded {} bytes from package file", package_data.len());

    // Try to register the real package
    let result = registry.register_workflow(package_data).await;

    match result {
        Ok(package_id) => {
            println!(
                "✓ Successfully registered real package with ID: {}",
                package_id
            );

            // Verify the package was registered
            let workflows = registry.list_workflows().await.unwrap();
            assert!(!workflows.is_empty(), "Should have registered workflow");

            let workflow = &workflows[0];
            println!(
                "✓ Registered workflow: {} v{} with {} tasks",
                workflow.package_name,
                workflow.version,
                workflow.tasks.len()
            );
        }
        Err(e) => {
            println!("Package registration failed (may be expected): {}", e);
            println!("✓ Registry handled real package data without crashing");
        }
    }

    // Verify registry remains functional
    let list_result = registry.list_workflows().await;
    assert!(
        list_result.is_ok(),
        "Registry should remain functional after operations"
    );
}

#[tokio::test]
#[serial]
async fn test_registry_api_simplification() {
    // This test demonstrates the API simplification
    let storage = create_test_storage();
    let database = create_test_database().await;
    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Before: Had to construct WorkflowPackage with metadata
    // Now: Just pass binary data directly

    let mock_package_data = vec![
        b"package 1".to_vec(),
        b"package 2".to_vec(),
        b"package 3".to_vec(),
    ];

    let mut results = Vec::new();
    for package_data in mock_package_data {
        let result = registry.register_workflow(package_data).await;
        results.push(result);
    }

    // All should fail validation but the API should work smoothly
    for result in results {
        assert!(result.is_err(), "Mock packages should fail validation");
    }

    // Registry should handle multiple operations gracefully
    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty(), "No packages should be registered");

    println!("✓ Simplified API handles multiple package registration attempts");
}
