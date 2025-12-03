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
//! These tests verify that all storage backends correctly implement the
//! RegistryStorage trait with real database connections and filesystem operations.
//! The same test suite runs against all backends.

use cloacina::dal::FilesystemRegistryStorage;
use cloacina::registry::error::StorageError;
use cloacina::registry::traits::RegistryStorage;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

use crate::fixtures::get_or_init_fixture;

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

/// Unified storage test implementations that work with any storage backend
mod storage_tests {
    use super::*;

    /// Test store and retrieve operations
    pub async fn test_store_and_retrieve_impl<S: RegistryStorage>(mut storage: S) {
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

    /// Test retrieving non-existent data
    pub async fn test_retrieve_nonexistent_impl<S: RegistryStorage>(storage: S) {
        let fake_id = Uuid::new_v4().to_string();
        let result = storage
            .retrieve_binary(&fake_id)
            .await
            .expect("Retrieval should not fail for nonexistent file");

        assert_eq!(result, None);
    }

    /// Test delete operations
    pub async fn test_delete_impl<S: RegistryStorage>(mut storage: S) {
        let test_data = b"test data for deletion".to_vec();
        let id = storage.store_binary(test_data).await.unwrap();

        // Verify it exists
        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert!(retrieved.is_some());

        // Delete it
        storage.delete_binary(&id).await.unwrap();

        // Verify it's gone
        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, None);

        // Verify idempotent deletion
        storage.delete_binary(&id).await.unwrap();
    }

    /// Test invalid UUID handling
    pub async fn test_invalid_uuid_impl<S: RegistryStorage>(mut storage: S) {
        let result = storage.retrieve_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));

        let result = storage.delete_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));
    }

    /// Test empty data storage
    pub async fn test_empty_data_impl<S: RegistryStorage>(mut storage: S) {
        let empty_data = Vec::new();
        let id = storage.store_binary(empty_data.clone()).await.unwrap();

        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(empty_data));
    }

    /// Test large data storage
    pub async fn test_large_data_impl<S: RegistryStorage>(mut storage: S) {
        // Test with 10MB of data
        let large_data = vec![0xAB; 10 * 1024 * 1024];
        let id = storage.store_binary(large_data.clone()).await.unwrap();

        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(large_data));
    }

    /// Test UUID format validation
    pub async fn test_uuid_format_impl<S: RegistryStorage>(mut storage: S) {
        let test_data = b"test data".to_vec();
        let id = storage.store_binary(test_data).await.unwrap();

        // Verify the returned ID is a valid UUID
        let parsed_uuid = Uuid::parse_str(&id);
        assert!(
            parsed_uuid.is_ok(),
            "Returned ID should be a valid UUID: {}",
            id
        );
    }

    /// Test binary data integrity
    pub async fn test_binary_data_integrity_impl<S: RegistryStorage>(mut storage: S) {
        // Test with binary data containing all byte values
        let mut binary_data = Vec::with_capacity(256);
        for i in 0..=255u8 {
            binary_data.push(i);
        }

        let id = storage.store_binary(binary_data.clone()).await.unwrap();
        let retrieved = storage.retrieve_binary(&id).await.unwrap();

        assert_eq!(retrieved, Some(binary_data));
    }
}

// Filesystem backend tests
mod filesystem_tests {
    use super::*;

    fn create_filesystem_storage() -> (FilesystemRegistryStorage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = FilesystemRegistryStorage::new(temp_dir.path())
            .expect("Failed to create filesystem storage");
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let (storage, _temp_dir) = create_filesystem_storage();
        storage_tests::test_store_and_retrieve_impl(storage).await;
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent() {
        let (storage, _temp_dir) = create_filesystem_storage();
        storage_tests::test_retrieve_nonexistent_impl(storage).await;
    }

    #[tokio::test]
    async fn test_delete() {
        let (storage, _temp_dir) = create_filesystem_storage();
        storage_tests::test_delete_impl(storage).await;
    }

    #[tokio::test]
    async fn test_invalid_uuid() {
        let (storage, _temp_dir) = create_filesystem_storage();
        storage_tests::test_invalid_uuid_impl(storage).await;
    }

    #[tokio::test]
    async fn test_empty_data() {
        let (storage, _temp_dir) = create_filesystem_storage();
        storage_tests::test_empty_data_impl(storage).await;
    }

    #[tokio::test]
    async fn test_large_data() {
        let (storage, _temp_dir) = create_filesystem_storage();
        storage_tests::test_large_data_impl(storage).await;
    }

    #[tokio::test]
    async fn test_uuid_format() {
        let (storage, _temp_dir) = create_filesystem_storage();
        storage_tests::test_uuid_format_impl(storage).await;
    }

    #[tokio::test]
    async fn test_binary_data_integrity() {
        let (storage, _temp_dir) = create_filesystem_storage();
        storage_tests::test_binary_data_integrity_impl(storage).await;
    }
}

// Database backend tests (PostgreSQL/SQLite)
mod database_tests {
    use super::*;
    use cloacina::dal::UnifiedRegistryStorage;

    async fn create_database_storage() -> UnifiedRegistryStorage {
        let fixture = get_or_init_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.initialize().await;
        fixture.create_unified_storage()
    }

    #[tokio::test]
    #[serial]
    async fn test_store_and_retrieve() {
        let storage = create_database_storage().await;
        storage_tests::test_store_and_retrieve_impl(storage).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_retrieve_nonexistent() {
        let storage = create_database_storage().await;
        storage_tests::test_retrieve_nonexistent_impl(storage).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_delete() {
        let storage = create_database_storage().await;
        storage_tests::test_delete_impl(storage).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_invalid_uuid() {
        let storage = create_database_storage().await;
        storage_tests::test_invalid_uuid_impl(storage).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_data() {
        let storage = create_database_storage().await;
        storage_tests::test_empty_data_impl(storage).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_large_data() {
        let storage = create_database_storage().await;
        storage_tests::test_large_data_impl(storage).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_uuid_format() {
        let storage = create_database_storage().await;
        storage_tests::test_uuid_format_impl(storage).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_binary_data_integrity() {
        let storage = create_database_storage().await;
        storage_tests::test_binary_data_integrity_impl(storage).await;
    }
}
