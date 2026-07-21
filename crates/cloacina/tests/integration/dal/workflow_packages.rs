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

use crate::fixtures::get_or_init_fixture;
use cloacina::models::workflow_packages::StorageType;
use cloacina::registry::error::RegistryError;
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
        workflow_name: "test_package".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test package description".to_string()),
        author: Some("Test Author".to_string()),
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec!["cloacina_execute_task".to_string()],
        workflow_triggers: vec![],
        declared_params: vec![],
        declared_surfaces: vec![],
        task_docs: Default::default(),
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
    let _package_id = workflow_packages_dal
        .store_package_metadata(&registry_id, &test_metadata, storage_type, None)
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
        workflow_name: "duplicate_test".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Duplicate test package".to_string()),
        author: Some("Test Author".to_string()),
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec![],
        workflow_triggers: vec![],
        declared_params: vec![],
        declared_surfaces: vec![],
        task_docs: Default::default(),
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
        .store_package_metadata(&registry_id, &test_metadata, storage_type, None)
        .await
        .expect("Failed to store package metadata first time");

    // Try to store the same package metadata again - should fail with PackageExists error
    let result = workflow_packages_dal
        .store_package_metadata(&registry_id, &test_metadata, storage_type, None)
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
            workflow_name: format!("list_test_package_{}", i),
            version: "1.0.0".to_string(),
            description: Some(format!("List test package {}", i)),
            author: Some("Test Author".to_string()),
            tasks: vec![],
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![],
            workflow_triggers: vec![],
            declared_params: vec![],
            declared_surfaces: vec![],
            task_docs: Default::default(),
        };

        package_names.push(test_metadata.package_name.clone());

        let storage_type = workflow_registry_storage.storage_type();
        workflow_packages_dal
            .store_package_metadata(&registry_id, &test_metadata, storage_type, None)
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
        workflow_name: "delete_test".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Package to be deleted".to_string()),
        author: Some("Test Author".to_string()),
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec![],
        workflow_triggers: vec![],
        declared_params: vec![],
        declared_surfaces: vec![],
        task_docs: Default::default(),
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
        .store_package_metadata(&registry_id, &test_metadata, storage_type, None)
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
        workflow_name: "complex_package".to_string(),
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
                doc_what: None,
                doc_why: None,
            },
            cloacina::registry::loader::package_loader::TaskMetadata {
                index: 1,
                local_id: "task2".to_string(),
                namespaced_id_template: "{tenant_id}/complex_package/task2".to_string(),
                dependencies: vec![],
                description: "Second task".to_string(),
                source_location: "src/task2.rs:20".to_string(),
                doc_what: None,
                doc_why: None,
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
        workflow_triggers: vec![],
        declared_params: vec![],
        declared_surfaces: vec![],
        task_docs: Default::default(),
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
        .store_package_metadata(&registry_id, &test_metadata, storage_type, None)
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
        workflow_name: "invalid_uuid_test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec![],
        workflow_triggers: vec![],
        declared_params: vec![],
        declared_surfaces: vec![],
        task_docs: Default::default(),
    };

    // Try to store with invalid UUID
    let result = workflow_packages_dal
        .store_package_metadata(
            "not-a-valid-uuid",
            &test_metadata,
            StorageType::Database,
            None,
        )
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
    // Under the supersede-based lifecycle (T-0497), only one row per package name is
    // active at any time. Inserting a second version for the same name without first
    // marking the predecessor as superseded would violate the partial unique index
    // `(package_name) WHERE NOT superseded` — so the direct DAL path can only hold a
    // single active version. The supersede-and-insert flow that lets a new version
    // replace the old is exercised separately via `register_workflow_package` in
    // the workflow_registry integration tests.
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let workflow_packages_dal = dal.workflow_packages();

    let package_name = "versioned_package".to_string();

    let storage = fixture.create_storage();
    let mut workflow_registry_storage = storage;
    let registry_id = workflow_registry_storage
        .store_binary(vec![1, 2, 3, 4])
        .await
        .expect("Failed to store binary in registry");
    let storage_type = workflow_registry_storage.storage_type();

    let meta_v1 = PackageMetadata {
        package_name: package_name.clone(),
        workflow_name: package_name.clone(),
        version: "1.0.0".to_string(),
        description: Some("Version 1.0.0 of the package".to_string()),
        author: Some("Versioning Author".to_string()),
        tasks: vec![],
        graph_data: None,
        architecture: "x86_64".to_string(),
        symbols: vec![],
        workflow_triggers: vec![],
        declared_params: vec![],
        declared_surfaces: vec![],
        task_docs: Default::default(),
    };
    workflow_packages_dal
        .store_package_metadata(&registry_id, &meta_v1, storage_type, None)
        .await
        .expect("Failed to store initial version");

    // A second active row for the same name is rejected by the partial unique index.
    let meta_v2 = PackageMetadata {
        version: "1.1.0".to_string(),
        description: Some("Version 1.1.0 of the package".to_string()),
        ..meta_v1.clone()
    };
    let err = workflow_packages_dal
        .store_package_metadata(&registry_id, &meta_v2, storage_type, None)
        .await
        .expect_err("second active insert for same name should be rejected");
    matches!(err, RegistryError::PackageExists { .. });

    // The original active row is still retrievable and is the sole listed entry.
    let retrieved = workflow_packages_dal
        .get_package_metadata(&package_name, "1.0.0")
        .await
        .expect("Failed to get package version");
    assert!(retrieved.is_some());

    let all_packages = workflow_packages_dal
        .list_all_packages()
        .await
        .expect("Failed to list all packages");
    let versioned_packages: Vec<_> = all_packages
        .iter()
        .filter(|p| p.package_name == package_name)
        .collect();
    assert_eq!(versioned_packages.len(), 1);
    assert_eq!(versioned_packages[0].version, "1.0.0");
}

/// CLOACI-T-0836: `package_providers` round-trip — run-verifies migration
/// `create_package_providers` plus the upsert/get DAL pair. The archive bytes are
/// opaque here (the reconciler unpacks real provider archives at load).
#[tokio::test]
async fn test_package_providers_upsert_and_get_round_trip() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let wp = dal.workflow_packages();

    // Empty for a package with no bundled providers.
    let none = wp
        .get_providers_for_package("consumer-pkg", "0.1.0", None)
        .await
        .expect("get providers (empty)");
    assert!(none.is_empty());

    // Upsert two providers for one consumer package.
    wp.upsert_provider(
        "consumer-pkg",
        "0.1.0",
        None,
        "cloacina-provider-fs",
        "0.1.0",
        "hash-a",
        vec![1, 2, 3],
        "wasm",
        None,
    )
    .await
    .expect("upsert provider fs");
    wp.upsert_provider(
        "consumer-pkg",
        "0.1.0",
        None,
        "cloacina-provider-http",
        "0.2.0",
        "hash-b",
        vec![4, 5],
        "wasm",
        None,
    )
    .await
    .expect("upsert provider http");

    let mut rows = wp
        .get_providers_for_package("consumer-pkg", "0.1.0", None)
        .await
        .expect("get providers");
    rows.sort_by(|a, b| a.provider_name.cmp(&b.provider_name));
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].provider_name, "cloacina-provider-fs");
    assert_eq!(rows[0].provider_version, "0.1.0");
    assert_eq!(rows[0].content_hash, "hash-a");
    assert_eq!(rows[0].provider_data.as_slice(), &[1, 2, 3]);
    assert_eq!(rows[1].provider_name, "cloacina-provider-http");

    // Upsert REPLACES a rebuild's row for the same (package, provider).
    wp.upsert_provider(
        "consumer-pkg",
        "0.1.0",
        None,
        "cloacina-provider-fs",
        "0.1.1",
        "hash-a2",
        vec![9, 9, 9, 9],
        "wasm",
        None,
    )
    .await
    .expect("re-upsert provider fs");
    let rows = wp
        .get_providers_for_package("consumer-pkg", "0.1.0", None)
        .await
        .expect("get providers after re-upsert");
    assert_eq!(rows.len(), 2, "replace, not accumulate");
    let fs = rows
        .iter()
        .find(|r| r.provider_name == "cloacina-provider-fs")
        .unwrap();
    assert_eq!(fs.provider_version, "0.1.1");
    assert_eq!(fs.provider_data.as_slice(), &[9, 9, 9, 9]);

    // Other package versions see nothing.
    let other = wp
        .get_providers_for_package("consumer-pkg", "0.2.0", None)
        .await
        .expect("get providers other version");
    assert!(other.is_empty());
}

/// CLOACI-T-0908: per-arch NATIVE provider rows — the primary (NULL-triple) row
/// and per-arch rows COEXIST (a second-arch per-target compile must never
/// clobber the primary, which is exactly what the old triple-less delete-filter
/// did), and `select_provider_rows_for_target` picks the reader's own arch with
/// primary fallback, never another arch's bytes.
#[tokio::test]
async fn test_provider_rows_are_triple_scoped_and_selected_per_target() {
    use cloacina::dal::unified::workflow_packages::select_provider_rows_for_target;

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;
    fixture.reset_database().await;

    let dal = fixture.get_dal();
    let wp = dal.workflow_packages();

    // Primary build (compiler host arch) + two per-arch NATIVE rebuilds, plus an
    // arch-neutral wasm provider that only ever has the primary row.
    wp.upsert_provider(
        "consumer-pkg",
        "0.1.0",
        None,
        "cloacina-provider-kafka",
        "0.1.0",
        "hash-primary",
        vec![0xAA],
        "native",
        None,
    )
    .await
    .expect("primary native row");
    wp.upsert_provider(
        "consumer-pkg",
        "0.1.0",
        None,
        "cloacina-provider-kafka",
        "0.1.0",
        "hash-arm",
        vec![0xBB],
        "native",
        Some("aarch64-linux"),
    )
    .await
    .expect("arm native row");
    wp.upsert_provider(
        "consumer-pkg",
        "0.1.0",
        None,
        "cloacina-provider-kafka",
        "0.1.0",
        "hash-x86",
        vec![0xCC],
        "native",
        Some("x86_64-linux"),
    )
    .await
    .expect("x86 native row");
    wp.upsert_provider(
        "consumer-pkg",
        "0.1.0",
        None,
        "cloacina-provider-fs",
        "0.1.0",
        "hash-wasm",
        vec![0xDD],
        "wasm",
        None,
    )
    .await
    .expect("wasm primary row");

    // COEXISTENCE: all four rows survive (the old delete-filter would have left one
    // kafka row — whichever arch wrote last).
    let rows = wp
        .get_providers_for_package("consumer-pkg", "0.1.0", None)
        .await
        .expect("all rows");
    assert_eq!(rows.len(), 4, "primary + 2 per-arch + wasm rows coexist");

    // Re-upserting ONE arch replaces only that arch's row.
    wp.upsert_provider(
        "consumer-pkg",
        "0.1.0",
        None,
        "cloacina-provider-kafka",
        "0.1.0",
        "hash-arm2",
        vec![0xBE],
        "native",
        Some("aarch64-linux"),
    )
    .await
    .expect("re-upsert arm row");
    let rows = wp
        .get_providers_for_package("consumer-pkg", "0.1.0", None)
        .await
        .expect("rows after per-arch re-upsert");
    assert_eq!(rows.len(), 4, "replace within the triple, not across");

    // SELECTION: an arm agent gets ITS kafka build + the arch-neutral wasm row.
    let mut selected = select_provider_rows_for_target(rows.clone(), "aarch64-linux");
    selected.sort_by(|a, b| a.provider_name.cmp(&b.provider_name));
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].provider_name, "cloacina-provider-fs");
    assert_eq!(selected[0].provider_data.as_slice(), &[0xDD]);
    assert_eq!(selected[1].provider_name, "cloacina-provider-kafka");
    assert_eq!(
        selected[1].provider_data.as_slice(),
        &[0xBE],
        "the arm agent gets the arm build (the re-upserted one)"
    );

    // FALLBACK: an arch with no per-target build gets the primary row — never
    // another arch's bytes.
    let selected = select_provider_rows_for_target(rows, "riscv64-linux");
    let kafka = selected
        .iter()
        .find(|r| r.provider_name == "cloacina-provider-kafka")
        .expect("kafka selected via fallback");
    assert_eq!(
        kafka.provider_data.as_slice(),
        &[0xAA],
        "unknown arch falls back to the PRIMARY build"
    );
}
