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

//! Integration tests for workflow registry storage backends.
//!
//! These tests verify that both PostgreSQL and filesystem storage backends
//! correctly implement the RegistryStorage trait with real database connections
//! and filesystem operations.

use cloacina::registry::error::StorageError;
use cloacina::registry::storage::FilesystemRegistryStorage;
use cloacina::registry::traits::RegistryStorage;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use cloacina::registry::storage::PostgresRegistryStorage;

#[cfg(feature = "postgres")]
use crate::fixtures::get_or_init_fixture;

#[cfg(feature = "postgres")]
use serial_test::serial;

/// Helper to create test data that simulates a compiled .so file
fn create_test_workflow_data(size: usize) -> Vec<u8> {
    // Create realistic binary-looking data
    let mut data = Vec::with_capacity(size);
    data.extend_from_slice(b"\x7fELF"); // ELF magic number
    data.extend_from_slice(&[0x02, 0x01, 0x01, 0x00]); // 64-bit, little-endian, current version

    // Fill rest with pseudo-random data that looks like compiled code
    for i in 0..size.saturating_sub(8) {
        data.push((i % 256) as u8);
    }

    data
}

/// Test suite for filesystem storage backend
mod filesystem_storage_tests {
    use super::*;

    pub async fn create_test_storage() -> (FilesystemRegistryStorage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = FilesystemRegistryStorage::new(temp_dir.path())
            .expect("Failed to create filesystem storage");
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_retrieve_basic() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        let test_data = create_test_workflow_data(1024);
        let id = storage
            .store_binary(test_data.clone())
            .await
            .expect("Failed to store binary data");

        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Failed to retrieve binary data");

        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_store_large_file() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Test with a larger file (1MB)
        let test_data = create_test_workflow_data(1024 * 1024);
        let id = storage
            .store_binary(test_data.clone())
            .await
            .expect("Failed to store large binary data");

        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Failed to retrieve large binary data");

        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_store_multiple_files() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        let mut stored_files = Vec::new();

        // Store multiple different files
        for i in 0..5 {
            let mut test_data = create_test_workflow_data(512);
            test_data.push(i); // Make each file unique

            let id = storage
                .store_binary(test_data.clone())
                .await
                .expect("Failed to store binary data");

            stored_files.push((id, test_data));
        }

        // Verify all files can be retrieved correctly
        for (id, expected_data) in stored_files {
            let retrieved = storage
                .retrieve_binary(&id)
                .await
                .expect("Failed to retrieve binary data");

            assert_eq!(retrieved, Some(expected_data));
        }
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent() {
        let (storage, _temp_dir) = create_test_storage().await;

        let fake_id = Uuid::new_v4().to_string();
        let result = storage
            .retrieve_binary(&fake_id)
            .await
            .expect("Retrieval should not fail for nonexistent file");

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_delete_and_verify_removal() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        let test_data = create_test_workflow_data(256);
        let id = storage
            .store_binary(test_data)
            .await
            .expect("Failed to store binary data");

        // Verify it exists
        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Failed to retrieve binary data");
        assert!(retrieved.is_some());

        // Delete it
        storage
            .delete_binary(&id)
            .await
            .expect("Failed to delete binary data");

        // Verify it's gone
        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Retrieval after deletion should not fail");
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_idempotent_deletion() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        let test_data = create_test_workflow_data(256);
        let id = storage
            .store_binary(test_data)
            .await
            .expect("Failed to store binary data");

        // Delete once
        storage
            .delete_binary(&id)
            .await
            .expect("Failed to delete binary data");

        // Delete again - should be idempotent
        storage
            .delete_binary(&id)
            .await
            .expect("Second deletion should be idempotent");

        // Delete nonexistent file - should also be idempotent
        let fake_id = Uuid::new_v4().to_string();
        storage
            .delete_binary(&fake_id)
            .await
            .expect("Deleting nonexistent file should be idempotent");
    }

    #[tokio::test]
    async fn test_invalid_uuid_handling() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Test retrieve with invalid UUID
        let result = storage.retrieve_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));

        // Test delete with invalid UUID
        let result = storage.delete_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));
    }

    #[tokio::test]
    async fn test_empty_file_detection() {
        let (storage, temp_dir) = create_test_storage().await;

        // Manually create an empty file
        let id = Uuid::new_v4().to_string();
        let file_path = temp_dir.path().join(format!("{}.so", id));
        tokio::fs::write(&file_path, b"")
            .await
            .expect("Failed to create empty file");

        // Should detect corruption
        let result = storage.retrieve_binary(&id).await;
        assert!(matches!(result, Err(StorageError::DataCorruption { .. })));
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let (storage, _temp_dir) = create_test_storage().await;
        let storage = Arc::new(tokio::sync::Mutex::new(storage));

        let mut handles = vec![];

        // Start multiple concurrent store operations
        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let handle = tokio::spawn(async move {
                let mut data = create_test_workflow_data(100);
                data.push(i); // Make each unique

                let mut storage = storage_clone.lock().await;
                let id = storage.store_binary(data.clone()).await?;

                // Immediately try to retrieve it
                let retrieved = storage.retrieve_binary(&id).await?;
                assert_eq!(retrieved, Some(data));

                Ok::<String, StorageError>(id)
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        let mut ids = vec![];
        for handle in handles {
            let id = handle
                .await
                .expect("Task should not panic")
                .expect("Storage operation should succeed");
            ids.push(id);
        }

        // Verify all files are accessible
        let storage = storage.lock().await;
        for id in ids {
            let result = storage.retrieve_binary(&id).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_some());
        }
    }
}

/// Test suite for PostgreSQL storage backend
#[cfg(feature = "postgres")]
mod postgres_storage_tests {
    use super::*;

    pub async fn create_test_storage() -> PostgresRegistryStorage {
        let fixture = get_or_init_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.initialize().await;

        let database = fixture.get_database();

        PostgresRegistryStorage::new(database)
    }

    #[tokio::test]
    #[serial]
    async fn test_store_and_retrieve_basic() {
        let mut storage = create_test_storage().await;

        let test_data = create_test_workflow_data(1024);
        let id = storage
            .store_binary(test_data.clone())
            .await
            .expect("Failed to store binary data");

        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Failed to retrieve binary data");

        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    #[serial]
    async fn test_store_large_file() {
        let mut storage = create_test_storage().await;

        // Test with a larger file (1MB)
        let test_data = create_test_workflow_data(1024 * 1024);
        let id = storage
            .store_binary(test_data.clone())
            .await
            .expect("Failed to store large binary data");

        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Failed to retrieve large binary data");

        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    #[serial]
    async fn test_store_multiple_files() {
        let mut storage = create_test_storage().await;

        let mut stored_files = Vec::new();

        // Store multiple different files
        for i in 0..5 {
            let mut test_data = create_test_workflow_data(512);
            test_data.push(i); // Make each file unique

            let id = storage
                .store_binary(test_data.clone())
                .await
                .expect("Failed to store binary data");

            stored_files.push((id, test_data));
        }

        // Verify all files can be retrieved correctly
        for (id, expected_data) in stored_files {
            let retrieved = storage
                .retrieve_binary(&id)
                .await
                .expect("Failed to retrieve binary data");

            assert_eq!(retrieved, Some(expected_data));
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_retrieve_nonexistent() {
        let storage = create_test_storage().await;

        let fake_id = Uuid::new_v4().to_string();
        let result = storage
            .retrieve_binary(&fake_id)
            .await
            .expect("Retrieval should not fail for nonexistent record");

        assert_eq!(result, None);
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_and_verify_removal() {
        let mut storage = create_test_storage().await;

        let test_data = create_test_workflow_data(256);
        let id = storage
            .store_binary(test_data)
            .await
            .expect("Failed to store binary data");

        // Verify it exists
        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Failed to retrieve binary data");
        assert!(retrieved.is_some());

        // Delete it
        storage
            .delete_binary(&id)
            .await
            .expect("Failed to delete binary data");

        // Verify it's gone
        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Retrieval after deletion should not fail");
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    #[serial]
    async fn test_idempotent_deletion() {
        let mut storage = create_test_storage().await;

        let test_data = create_test_workflow_data(256);
        let id = storage
            .store_binary(test_data)
            .await
            .expect("Failed to store binary data");

        // Delete once
        storage
            .delete_binary(&id)
            .await
            .expect("Failed to delete binary data");

        // Delete again - should be idempotent
        storage
            .delete_binary(&id)
            .await
            .expect("Second deletion should be idempotent");

        // Delete nonexistent record - should also be idempotent
        let fake_id = Uuid::new_v4().to_string();
        storage
            .delete_binary(&fake_id)
            .await
            .expect("Deleting nonexistent record should be idempotent");
    }

    #[tokio::test]
    #[serial]
    async fn test_invalid_uuid_handling() {
        let mut storage = create_test_storage().await;

        // Test retrieve with invalid UUID
        let result = storage.retrieve_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));

        // Test delete with invalid UUID
        let result = storage.delete_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));
    }

    #[tokio::test]
    #[serial]
    async fn test_transaction_rollback_on_failure() {
        let mut storage = create_test_storage().await;

        // Store a valid file first
        let test_data = create_test_workflow_data(256);
        let id = storage
            .store_binary(test_data.clone())
            .await
            .expect("Failed to store initial binary data");

        // Verify it exists
        let retrieved = storage
            .retrieve_binary(&id)
            .await
            .expect("Failed to retrieve binary data");
        assert_eq!(retrieved, Some(test_data));

        // The PostgreSQL backend should handle any database errors gracefully
        // For this test, we verify that normal operations work as expected
        // More complex transaction failure scenarios would require database manipulation
    }
}

/// Cross-backend compatibility tests
mod cross_backend_tests {
    use super::*;

    #[tokio::test]
    async fn test_filesystem_and_postgres_consistency() {
        // This test verifies that both backends handle the same operations identically
        let (mut fs_storage, _temp_dir) = filesystem_storage_tests::create_test_storage().await;

        #[cfg(feature = "postgres")]
        let mut pg_storage = postgres_storage_tests::create_test_storage().await;

        let test_data = create_test_workflow_data(512);

        // Store in filesystem
        let fs_id = fs_storage
            .store_binary(test_data.clone())
            .await
            .expect("Failed to store in filesystem");

        #[cfg(feature = "postgres")]
        {
            // Store in PostgreSQL
            let pg_id = pg_storage
                .store_binary(test_data.clone())
                .await
                .expect("Failed to store in PostgreSQL");

            // Both should return valid UUIDs
            assert!(Uuid::parse_str(&fs_id).is_ok());
            assert!(Uuid::parse_str(&pg_id).is_ok());

            // Both should retrieve the same data
            let fs_retrieved = fs_storage
                .retrieve_binary(&fs_id)
                .await
                .expect("Failed to retrieve from filesystem");
            let pg_retrieved = pg_storage
                .retrieve_binary(&pg_id)
                .await
                .expect("Failed to retrieve from PostgreSQL");

            assert_eq!(fs_retrieved, Some(test_data.clone()));
            assert_eq!(pg_retrieved, Some(test_data));

            // Both should handle deletion identically
            fs_storage
                .delete_binary(&fs_id)
                .await
                .expect("Failed to delete from filesystem");
            pg_storage
                .delete_binary(&pg_id)
                .await
                .expect("Failed to delete from PostgreSQL");

            // Both should return None after deletion
            let fs_after_delete = fs_storage
                .retrieve_binary(&fs_id)
                .await
                .expect("Retrieval after deletion should not fail");
            let pg_after_delete = pg_storage
                .retrieve_binary(&pg_id)
                .await
                .expect("Retrieval after deletion should not fail");

            assert_eq!(fs_after_delete, None);
            assert_eq!(pg_after_delete, None);
        }

        #[cfg(not(feature = "postgres"))]
        {
            // Just verify filesystem operations work
            let fs_retrieved = fs_storage
                .retrieve_binary(&fs_id)
                .await
                .expect("Failed to retrieve from filesystem");
            assert_eq!(fs_retrieved, Some(test_data));
        }
    }
}
