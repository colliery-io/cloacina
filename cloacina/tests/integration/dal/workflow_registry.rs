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

use crate::fixtures::get_or_init_fixture;
use cloacina::packaging::{package_workflow, CompileOptions};
use cloacina::registry::error::RegistryError;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

fn create_mock_package() -> Vec<u8> {
    // Create a real package using the examples/packaged-workflow-example with unique name
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let unique_id = Uuid::new_v4().to_string();
    let package_path = temp_dir
        .path()
        .join(format!("test_package_{}.cloacina", unique_id));

    // Find the workspace root by looking for Cargo.toml
    let cargo_manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_path = std::path::PathBuf::from(&cargo_manifest_dir);
    let workspace_root = workspace_path
        .parent()
        .expect("Should have parent directory");
    let project_path = workspace_root.join("examples/packaged-workflow-example");

    if !project_path.exists() {
        panic!("Project path does not exist: {}", project_path.display());
    }

    // Create compile options
    let options = CompileOptions {
        target: None,
        profile: "debug".to_string(),
        cargo_flags: vec![],
        jobs: None,
    };

    // Use the package_workflow function to create a real package
    package_workflow(project_path, package_path.clone(), options)
        .expect("Failed to create test package");

    // Read the package data
    std::fs::read(&package_path).expect("Failed to read package file")
}

#[tokio::test]
#[serial]
async fn test_register_and_get_workflow_package() {
    // Test with database storage
    test_register_and_get_workflow_package_with_db_storage().await;
    // Test with filesystem storage
    test_register_and_get_workflow_package_with_fs_storage().await;
}

async fn test_register_and_get_workflow_package_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create mock package data
    let package_data = create_mock_package();

    // Register the package
    let package_id = registry_dal
        .register_workflow_package(package_data.clone())
        .await
        .expect("Failed to register workflow package");

    // Retrieve the package by ID
    let retrieved = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get workflow package by ID");

    assert!(retrieved.is_some());
    let (metadata, binary_data) = retrieved.unwrap();
    assert_eq!(metadata.package_name, "packaged-workflow-example");
    // Version will be the workflow fingerprint from the real package
    assert_eq!(binary_data, package_data);
}

async fn test_register_and_get_workflow_package_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create mock package data
    let package_data = create_mock_package();

    // Register the package
    let package_id = registry_dal
        .register_workflow_package(package_data.clone())
        .await
        .expect("Failed to register workflow package");

    // Retrieve the package by ID
    let retrieved = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get workflow package by ID");

    assert!(retrieved.is_some());
    let (metadata, binary_data) = retrieved.unwrap();
    assert_eq!(metadata.package_name, "packaged-workflow-example");
    // Version will be the workflow fingerprint from the real package
    assert_eq!(binary_data, package_data);
}

#[tokio::test]
#[serial]
async fn test_get_workflow_package_by_name() {
    // Test with database storage
    test_get_workflow_package_by_name_with_db_storage().await;
    // Test with filesystem storage
    test_get_workflow_package_by_name_with_fs_storage().await;
}

async fn test_get_workflow_package_by_name_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create mock package data
    let package_data = create_mock_package();

    // Register the package
    let package_id = registry_dal
        .register_workflow_package(package_data.clone())
        .await
        .expect("Failed to register workflow package");

    // Get the registered package to find out its actual name and version
    let registered = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get registered package")
        .expect("Package should exist");

    // Retrieve the package by name and version using actual values
    let retrieved = registry_dal
        .get_workflow_package_by_name(&registered.0.package_name, &registered.0.version)
        .await
        .expect("Failed to get workflow package by name");

    assert!(retrieved.is_some());
    let (metadata, binary_data) = retrieved.unwrap();
    assert_eq!(metadata.package_name, "packaged-workflow-example");
    // Version will be the workflow fingerprint from the real package
    assert_eq!(binary_data, package_data);
}

async fn test_get_workflow_package_by_name_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create mock package data
    let package_data = create_mock_package();

    // Register the package
    let package_id = registry_dal
        .register_workflow_package(package_data.clone())
        .await
        .expect("Failed to register workflow package");

    // Get the registered package to find out its actual name and version
    let registered = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get registered package")
        .expect("Package should exist");

    // Retrieve the package by name and version using actual values
    let retrieved = registry_dal
        .get_workflow_package_by_name(&registered.0.package_name, &registered.0.version)
        .await
        .expect("Failed to get workflow package by name");

    assert!(retrieved.is_some());
    let (metadata, binary_data) = retrieved.unwrap();
    assert_eq!(metadata.package_name, "packaged-workflow-example");
    // Version will be the workflow fingerprint from the real package
    assert_eq!(binary_data, package_data);
}

#[tokio::test]
#[serial]
async fn test_unregister_workflow_package_by_id() {
    // Test with database storage
    test_unregister_workflow_package_by_id_with_db_storage().await;
    // Test with filesystem storage
    test_unregister_workflow_package_by_id_with_fs_storage().await;
}

async fn test_unregister_workflow_package_by_id_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create and register a package
    let package_data = create_mock_package();
    let package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    // Verify it exists
    assert!(registry_dal
        .exists_by_id(package_id)
        .await
        .expect("Failed to check existence"));

    // Unregister the package
    registry_dal
        .unregister_workflow_package_by_id(package_id)
        .await
        .expect("Failed to unregister workflow package");

    // Verify it's gone
    assert!(!registry_dal
        .exists_by_id(package_id)
        .await
        .expect("Failed to check existence"));
}

async fn test_unregister_workflow_package_by_id_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create and register a package
    let package_data = create_mock_package();
    let package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    // Verify it exists
    assert!(registry_dal
        .exists_by_id(package_id)
        .await
        .expect("Failed to check existence"));

    // Unregister the package
    registry_dal
        .unregister_workflow_package_by_id(package_id)
        .await
        .expect("Failed to unregister workflow package");

    // Verify it's gone
    assert!(!registry_dal
        .exists_by_id(package_id)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
#[serial]
async fn test_unregister_workflow_package_by_name() {
    // Test with database storage
    test_unregister_workflow_package_by_name_with_db_storage().await;
    // Test with filesystem storage
    test_unregister_workflow_package_by_name_with_fs_storage().await;
}

async fn test_unregister_workflow_package_by_name_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create and register a package
    let package_data = create_mock_package();
    let package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    // Get the actual package metadata
    let registered = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get registered package")
        .expect("Package should exist");
    let package_name = &registered.0.package_name;
    let package_version = &registered.0.version;

    // Verify it exists
    assert!(registry_dal
        .exists_by_name(package_name, package_version)
        .await
        .expect("Failed to check existence"));

    // Unregister the package by name
    registry_dal
        .unregister_workflow_package_by_name(package_name, package_version)
        .await
        .expect("Failed to unregister workflow package");

    // Verify it's gone
    assert!(!registry_dal
        .exists_by_name(package_name, package_version)
        .await
        .expect("Failed to check existence"));
}

async fn test_unregister_workflow_package_by_name_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create and register a package
    let package_data = create_mock_package();
    let package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    // Get the actual package metadata
    let registered = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get registered package")
        .expect("Package should exist");
    let package_name = &registered.0.package_name;
    let package_version = &registered.0.version;

    // Verify it exists
    assert!(registry_dal
        .exists_by_name(package_name, package_version)
        .await
        .expect("Failed to check existence"));

    // Unregister the package by name
    registry_dal
        .unregister_workflow_package_by_name(package_name, package_version)
        .await
        .expect("Failed to unregister workflow package");

    // Verify it's gone
    assert!(!registry_dal
        .exists_by_name(package_name, package_version)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
#[serial]
async fn test_list_packages() {
    // Test with database storage
    test_list_packages_with_db_storage().await;
    // Test with filesystem storage
    test_list_packages_with_fs_storage().await;
}

async fn test_list_packages_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Get initial count
    let initial_packages = registry_dal
        .list_packages()
        .await
        .expect("Failed to list packages");
    let initial_count = initial_packages.len();

    // Register a package
    let package_data = create_mock_package();
    let _package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    // List packages and verify count increased
    let packages = registry_dal
        .list_packages()
        .await
        .expect("Failed to list packages");
    assert_eq!(packages.len(), initial_count + 1);

    // Find our package in the list
    let our_package = packages
        .iter()
        .find(|p| p.package_name == "packaged-workflow-example");
    assert!(our_package.is_some());
}

async fn test_list_packages_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Get initial count
    let initial_packages = registry_dal
        .list_packages()
        .await
        .expect("Failed to list packages");
    let initial_count = initial_packages.len();

    // Register a package
    let package_data = create_mock_package();
    let _package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    // List packages and verify count increased
    let packages = registry_dal
        .list_packages()
        .await
        .expect("Failed to list packages");
    assert_eq!(packages.len(), initial_count + 1);

    // Find our package in the list
    let our_package = packages
        .iter()
        .find(|p| p.package_name == "packaged-workflow-example");
    assert!(our_package.is_some());
}

#[tokio::test]
#[serial]
async fn test_register_duplicate_package() {
    // Test with database storage
    test_register_duplicate_package_with_db_storage().await;
    // Test with filesystem storage
    test_register_duplicate_package_with_fs_storage().await;
}

async fn test_register_duplicate_package_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create mock package data
    let package_data = create_mock_package();

    // Register the package first time - should succeed
    let _package_id = registry_dal
        .register_workflow_package(package_data.clone())
        .await
        .expect("Failed to register workflow package first time");

    // Try to register the same package again - should fail
    let result = registry_dal.register_workflow_package(package_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        RegistryError::PackageExists {
            package_name,
            version,
        } => {
            assert_eq!(package_name, "packaged-workflow-example");
            // Version will be the real fingerprint from the package
        }
        other => panic!("Expected PackageExists error, got: {:?}", other),
    }
}

async fn test_register_duplicate_package_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Create mock package data
    let package_data = create_mock_package();

    // Register the package first time - should succeed
    let _package_id = registry_dal
        .register_workflow_package(package_data.clone())
        .await
        .expect("Failed to register workflow package first time");

    // Try to register the same package again - should fail
    let result = registry_dal.register_workflow_package(package_data).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        RegistryError::PackageExists {
            package_name,
            version,
        } => {
            assert_eq!(package_name, "packaged-workflow-example");
            // Version will be the real fingerprint from the package
        }
        other => panic!("Expected PackageExists error, got: {:?}", other),
    }
}

#[tokio::test]
#[serial]
async fn test_exists_operations() {
    // Test with database storage
    test_exists_operations_with_db_storage().await;
    // Test with filesystem storage
    test_exists_operations_with_fs_storage().await;
}

async fn test_exists_operations_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Check non-existent package
    let fake_id = Uuid::new_v4();
    assert!(!registry_dal
        .exists_by_id(fake_id)
        .await
        .expect("Failed to check existence"));
    assert!(!registry_dal
        .exists_by_name("nonexistent", "1.0.0")
        .await
        .expect("Failed to check existence"));

    // Register a package
    let package_data = create_mock_package();
    let package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    // Check existing package
    assert!(registry_dal
        .exists_by_id(package_id)
        .await
        .expect("Failed to check existence"));
    // Get actual package metadata to check existence
    let registered = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get registered package")
        .expect("Package should exist");

    assert!(registry_dal
        .exists_by_name(&registered.0.package_name, &registered.0.version)
        .await
        .expect("Failed to check existence"));
}

async fn test_exists_operations_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    // Check non-existent package
    let fake_id = Uuid::new_v4();
    assert!(!registry_dal
        .exists_by_id(fake_id)
        .await
        .expect("Failed to check existence"));
    assert!(!registry_dal
        .exists_by_name("nonexistent", "1.0.0")
        .await
        .expect("Failed to check existence"));

    // Register a package
    let package_data = create_mock_package();
    let package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    // Check existing package
    assert!(registry_dal
        .exists_by_id(package_id)
        .await
        .expect("Failed to check existence"));
    // Get actual package metadata to check existence
    let registered = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get registered package")
        .expect("Package should exist");

    assert!(registry_dal
        .exists_by_name(&registered.0.package_name, &registered.0.version)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
#[serial]
async fn test_get_nonexistent_package() {
    // Test with database storage
    test_get_nonexistent_package_with_db_storage().await;
    // Test with filesystem storage
    test_get_nonexistent_package_with_fs_storage().await;
}

async fn test_get_nonexistent_package_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let registry_dal = dal.workflow_registry(storage);

    let fake_id = Uuid::new_v4();

    // Try to get non-existent package by ID
    let result = registry_dal
        .get_workflow_package_by_id(fake_id)
        .await
        .expect("Failed to get nonexistent package by ID");
    assert!(result.is_none());

    // Try to get non-existent package by name
    let result = registry_dal
        .get_workflow_package_by_name("nonexistent", "1.0.0")
        .await
        .expect("Failed to get nonexistent package by name");
    assert!(result.is_none());
}

async fn test_get_nonexistent_package_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let registry_dal = dal.workflow_registry(storage);

    let fake_id = Uuid::new_v4();

    // Try to get non-existent package by ID
    let result = registry_dal
        .get_workflow_package_by_id(fake_id)
        .await
        .expect("Failed to get nonexistent package by ID");
    assert!(result.is_none());

    // Try to get non-existent package by name
    let result = registry_dal
        .get_workflow_package_by_name("nonexistent", "1.0.0")
        .await
        .expect("Failed to get nonexistent package by name");
    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_unregister_nonexistent_package() {
    // Test with database storage
    test_unregister_nonexistent_package_with_db_storage().await;
    // Test with filesystem storage
    test_unregister_nonexistent_package_with_fs_storage().await;
}

async fn test_unregister_nonexistent_package_with_db_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    let fake_id = Uuid::new_v4();

    // Try to unregister non-existent package by ID - should be idempotent
    let result = registry_dal
        .unregister_workflow_package_by_id(fake_id)
        .await;
    assert!(
        result.is_ok(),
        "Unregistering nonexistent package should be idempotent"
    );

    // Try to unregister non-existent package by name - should be idempotent
    let result = registry_dal
        .unregister_workflow_package_by_name("nonexistent", "1.0.0")
        .await;
    assert!(
        result.is_ok(),
        "Unregistering nonexistent package should be idempotent"
    );
}

async fn test_unregister_nonexistent_package_with_fs_storage() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage = Arc::new(fixture.create_filesystem_storage());
    let mut registry_dal = dal.workflow_registry(storage);

    let fake_id = Uuid::new_v4();

    // Try to unregister non-existent package by ID - should be idempotent
    let result = registry_dal
        .unregister_workflow_package_by_id(fake_id)
        .await;
    assert!(
        result.is_ok(),
        "Unregistering nonexistent package should be idempotent"
    );

    // Try to unregister non-existent package by name - should be idempotent
    let result = registry_dal
        .unregister_workflow_package_by_name("nonexistent", "1.0.0")
        .await;
    assert!(
        result.is_ok(),
        "Unregistering nonexistent package should be idempotent"
    );
}
