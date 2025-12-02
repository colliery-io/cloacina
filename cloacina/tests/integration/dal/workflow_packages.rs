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
use cloacina::models::workflow_packages::StorageType;
use cloacina::registry::loader::package_loader::PackageMetadata;
use cloacina::registry::traits::RegistryStorage;

#[tokio::test]
async fn test_store_and_get_package_metadata() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    // Create test package metadata
    let test_metadata = PackageMetadata {
        package_name: "test_package".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test package description".to_string()),
        author: Some("Test Author".to_string()),
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec!["cloacina_execute_task".to_string()],
    };

    // Create a corresponding workflow_registry entry first
    let storage = fixture.create_storage();
    let mut workflow_registry_storage = storage;
    let mock_binary = vec![1, 2, 3, 4]; // Mock binary data
    let registry_id = workflow_registry_storage
        .store_binary(mock_binary)
        .await
        .expect("Failed to store binary in registry");

    // Store the package metadata
    let storage_type = workflow_registry_storage.storage_type();
    let package_id = workflow_packages_dal
        .store_package_metadata(&registry_id, &test_metadata, storage_type)
        .await
        .expect("Failed to store package metadata");

    // Retrieve the package metadata
    let retrieved = workflow_packages_dal
        .get_package_metadata("test_package", "1.0.0")
        .await
        .expect("Failed to get package metadata");

    assert!(retrieved.is_some());
    let (retrieved_registry_id, retrieved_metadata) = retrieved.unwrap();

    assert_eq!(retrieved_registry_id, registry_id);
    assert_eq!(retrieved_metadata.package_name, test_metadata.package_name);
    assert_eq!(retrieved_metadata.version, test_metadata.version);
    assert_eq!(retrieved_metadata.description, test_metadata.description);
    assert_eq!(retrieved_metadata.author, test_metadata.author);
    assert_eq!(retrieved_metadata.architecture, test_metadata.architecture);
}

#[tokio::test]
async fn test_store_duplicate_package_metadata() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    // Create test package metadata
    let test_metadata = PackageMetadata {
        package_name: "duplicate_test".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Duplicate test package".to_string()),
        author: Some("Test Author".to_string()),
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec![],
    };

    // Create a corresponding workflow_registry entry first
    let storage = fixture.create_storage();
    let mut workflow_registry_storage = storage;
    let mock_binary = vec![1, 2, 3, 4]; // Mock binary data
    let registry_id = workflow_registry_storage
        .store_binary(mock_binary)
        .await
        .expect("Failed to store binary in registry");

    // Store the package metadata first time - should succeed
    let storage_type = workflow_registry_storage.storage_type();
    let _package_id = workflow_packages_dal
        .store_package_metadata(&registry_id, &test_metadata, storage_type)
        .await
        .expect("Failed to store package metadata first time");

    // Try to store the same package metadata again - should fail with PackageExists error
    let result = workflow_packages_dal
        .store_package_metadata(&registry_id, &test_metadata, storage_type)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        cloacina::registry::error::RegistryError::PackageExists {
            package_name,
            version,
        } => {
            assert_eq!(package_name, "duplicate_test");
            assert_eq!(version, "1.0.0");
        }
        other => panic!("Expected PackageExists error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_list_all_packages() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    // Get initial count
    let initial_packages = workflow_packages_dal
        .list_all_packages()
        .await
        .expect("Failed to list packages initially");
    let initial_count = initial_packages.len();

    // Create and store multiple test packages
    let mut package_names = vec![];

    for i in 0..3 {
        // Create a corresponding workflow_registry entry for each package
        let storage = fixture.create_storage();
        let mut workflow_registry_storage = storage;
        let mock_binary = vec![1, 2, 3, 4]; // Mock binary data
        let registry_id = workflow_registry_storage
            .store_binary(mock_binary)
            .await
            .expect("Failed to store binary in registry");
        let test_metadata = PackageMetadata {
            package_name: format!("list_test_package_{}", i),
            version: "1.0.0".to_string(),
            description: Some(format!("List test package {}", i)),
            author: Some("Test Author".to_string()),
            tasks: vec![],
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![],
        };

        package_names.push(test_metadata.package_name.clone());

        let storage_type = workflow_registry_storage.storage_type();
        workflow_packages_dal
            .store_package_metadata(&registry_id, &test_metadata, storage_type)
            .await
            .expect("Failed to store test package");
    }

    // List all packages and verify our test packages are included
    let all_packages = workflow_packages_dal
        .list_all_packages()
        .await
        .expect("Failed to list all packages");

    assert_eq!(all_packages.len(), initial_count + 3);

    // Verify our test packages are in the list
    for package_name in &package_names {
        let found = all_packages.iter().any(|p| p.package_name == *package_name);
        assert!(found, "Package {} not found in list", package_name);
    }
}

#[tokio::test]
async fn test_delete_package_metadata() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    // Create and store test package
    let test_metadata = PackageMetadata {
        package_name: "delete_test".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Package to be deleted".to_string()),
        author: Some("Test Author".to_string()),
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec![],
    };

    // Create a corresponding workflow_registry entry first
    let storage = fixture.create_storage();
    let mut workflow_registry_storage = storage;
    let mock_binary = vec![1, 2, 3, 4]; // Mock binary data
    let registry_id = workflow_registry_storage
        .store_binary(mock_binary)
        .await
        .expect("Failed to store binary in registry");

    // Store the package
    let storage_type = workflow_registry_storage.storage_type();
    let _package_id = workflow_packages_dal
        .store_package_metadata(&registry_id, &test_metadata, storage_type)
        .await
        .expect("Failed to store package metadata");

    // Verify it exists
    let retrieved = workflow_packages_dal
        .get_package_metadata("delete_test", "1.0.0")
        .await
        .expect("Failed to get package metadata");
    assert!(retrieved.is_some());

    // Delete the package
    workflow_packages_dal
        .delete_package_metadata("delete_test", "1.0.0")
        .await
        .expect("Failed to delete package metadata");

    // Verify it's gone
    let retrieved_after_delete = workflow_packages_dal
        .get_package_metadata("delete_test", "1.0.0")
        .await
        .expect("Failed to get package metadata after delete");
    assert!(retrieved_after_delete.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_package() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    // Try to delete a package that doesn't exist - should succeed (idempotent)
    let result = workflow_packages_dal
        .delete_package_metadata("nonexistent", "1.0.0")
        .await;

    assert!(
        result.is_ok(),
        "Deleting nonexistent package should be idempotent"
    );
}

#[tokio::test]
async fn test_get_nonexistent_package() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    // Try to get a package that doesn't exist
    let result = workflow_packages_dal
        .get_package_metadata("nonexistent", "1.0.0")
        .await
        .expect("Failed to get nonexistent package");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_store_package_with_complex_metadata() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    // Create package metadata with complex data
    let test_metadata = PackageMetadata {
        package_name: "complex_package".to_string(),
        version: "2.1.0".to_string(),
        description: Some("A complex package with detailed metadata".to_string()),
        author: Some("Complex Author <author@example.com>".to_string()),
        tasks: vec![
            cloacina::registry::loader::package_loader::TaskMetadata {
                index: 0,
                local_id: "task1".to_string(),
                namespaced_id_template: "{tenant_id}/complex_package/task1".to_string(),
                dependencies: vec!["task2".to_string()],
                description: "First task".to_string(),
                source_location: "src/task1.rs:10".to_string(),
            },
            cloacina::registry::loader::package_loader::TaskMetadata {
                index: 1,
                local_id: "task2".to_string(),
                namespaced_id_template: "{tenant_id}/complex_package/task2".to_string(),
                dependencies: vec![],
                description: "Second task".to_string(),
                source_location: "src/task2.rs:20".to_string(),
            },
        ],
        graph_data: Some(serde_json::json!({
            "nodes": ["task1", "task2"],
            "edges": [["task2", "task1"]]
        })),
        architecture: "aarch64".to_string(),
        symbols: vec![
            "cloacina_execute_task".to_string(),
            "cloacina_get_task_metadata".to_string(),
        ],
    };

    // Create a corresponding workflow_registry entry first
    let storage = fixture.create_storage();
    let mut workflow_registry_storage = storage;
    let mock_binary = vec![1, 2, 3, 4]; // Mock binary data
    let registry_id = workflow_registry_storage
        .store_binary(mock_binary)
        .await
        .expect("Failed to store binary in registry");

    // Store the complex package
    let storage_type = workflow_registry_storage.storage_type();
    let _package_id = workflow_packages_dal
        .store_package_metadata(&registry_id, &test_metadata, storage_type)
        .await
        .expect("Failed to store complex package metadata");

    // Retrieve and verify all fields
    let retrieved = workflow_packages_dal
        .get_package_metadata("complex_package", "2.1.0")
        .await
        .expect("Failed to get complex package metadata");

    assert!(retrieved.is_some());
    let (retrieved_registry_id, retrieved_metadata) = retrieved.unwrap();

    assert_eq!(retrieved_registry_id, registry_id);
    assert_eq!(retrieved_metadata.package_name, test_metadata.package_name);
    assert_eq!(retrieved_metadata.version, test_metadata.version);
    assert_eq!(retrieved_metadata.description, test_metadata.description);
    assert_eq!(retrieved_metadata.author, test_metadata.author);
    assert_eq!(retrieved_metadata.architecture, test_metadata.architecture);
    assert_eq!(retrieved_metadata.symbols, test_metadata.symbols);
    assert_eq!(retrieved_metadata.tasks.len(), 2);

    // Verify task details
    let task1 = &retrieved_metadata.tasks[0];
    assert_eq!(task1.local_id, "task1");
    assert_eq!(task1.dependencies, vec!["task2"]);

    let task2 = &retrieved_metadata.tasks[1];
    assert_eq!(task2.local_id, "task2");
    assert_eq!(task2.dependencies.len(), 0);

    // Verify graph data
    assert!(retrieved_metadata.graph_data.is_some());
    let graph_data = retrieved_metadata.graph_data.unwrap();
    assert_eq!(graph_data["nodes"], serde_json::json!(["task1", "task2"]));
}

#[tokio::test]
async fn test_store_package_with_invalid_uuid() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    let test_metadata = PackageMetadata {
        package_name: "invalid_uuid_test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec![],
    };

    // Try to store with invalid UUID
    let result = workflow_packages_dal
        .store_package_metadata("not-a-valid-uuid", &test_metadata, StorageType::Database)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        cloacina::registry::error::RegistryError::InvalidUuid(_) => {
            // Expected error
        }
        other => panic!("Expected InvalidUuid error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_package_versioning() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    let package_name = "versioned_package".to_string();

    // Store multiple versions of the same package
    for version in ["1.0.0", "1.1.0", "2.0.0"] {
        // Create a corresponding workflow_registry entry for each version
        let storage = fixture.create_storage();
        let mut workflow_registry_storage = storage;
        let mock_binary = vec![1, 2, 3, 4]; // Mock binary data
        let registry_id = workflow_registry_storage
            .store_binary(mock_binary)
            .await
            .expect("Failed to store binary in registry");
        let test_metadata = PackageMetadata {
            package_name: package_name.clone(),
            version: version.to_string(),
            description: Some(format!("Version {} of the package", version)),
            author: Some("Versioning Author".to_string()),
            tasks: vec![],
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![],
        };

        let storage_type = workflow_registry_storage.storage_type();
        workflow_packages_dal
            .store_package_metadata(&registry_id, &test_metadata, storage_type)
            .await
            .expect(&format!("Failed to store version {}", version));
    }

    // Verify we can retrieve each version individually
    for version in ["1.0.0", "1.1.0", "2.0.0"] {
        let retrieved = workflow_packages_dal
            .get_package_metadata(&package_name, version)
            .await
            .expect("Failed to get package version");

        assert!(retrieved.is_some());
        let (_, metadata) = retrieved.unwrap();
        assert_eq!(metadata.version, version);
        assert_eq!(
            metadata.description,
            Some(format!("Version {} of the package", version))
        );
    }

    // Verify all versions show up in the list
    let all_packages = workflow_packages_dal
        .list_all_packages()
        .await
        .expect("Failed to list all packages");

    let versioned_packages: Vec<_> = all_packages
        .iter()
        .filter(|p| p.package_name == package_name)
        .collect();

    assert_eq!(versioned_packages.len(), 3);
}
