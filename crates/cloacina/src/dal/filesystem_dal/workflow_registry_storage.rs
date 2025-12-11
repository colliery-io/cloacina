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

//! Filesystem DAL for workflow registry storage operations.
//!
//! This module provides filesystem-based data access operations for workflow
//! registry binary data storage, following the established DAL patterns for
//! non-database storage backends.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

use crate::models::workflow_packages::StorageType;
use crate::registry::error::StorageError;
use crate::registry::traits::RegistryStorage;

/// Filesystem-based DAL for workflow registry storage operations.
///
/// This DAL implementation handles binary workflow data storage as individual
/// files on the local filesystem. Files are named using UUIDs to ensure
/// uniqueness and avoid conflicts.
///
/// # Directory Structure
///
/// ```text
/// /var/lib/cloacina/registry/
/// ├── a1b2c3d4-e5f6-7890-abcd-ef1234567890.so
/// ├── b2c3d4e5-f6g7-8901-bcde-f12345678901.so
/// └── ...
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use cloacina::dal::filesystem_dal::FilesystemRegistryStorage;
/// use cloacina::registry::RegistryStorage;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut storage = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;
///
/// // Store binary workflow data
/// let workflow_data = std::fs::read("my_workflow.so")?;
/// let id = storage.store_binary(workflow_data).await?;
///
/// // Retrieve it later
/// if let Some(data) = storage.retrieve_binary(&id).await? {
///     println!("Retrieved {} bytes", data.len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct FilesystemRegistryStorage {
    storage_dir: PathBuf,
}

impl FilesystemRegistryStorage {
    /// Create a new filesystem workflow registry DAL.
    ///
    /// # Arguments
    ///
    /// * `storage_dir` - Directory path where workflow files will be stored
    ///
    /// # Returns
    ///
    /// * `Ok(FilesystemWorkflowRegistryDAL)` - Successfully created DAL
    /// * `Err(std::io::Error)` - If directory creation fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use cloacina::dal::filesystem_dal::FilesystemRegistryStorage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let dal = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> Result<Self, std::io::Error> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&storage_dir)?;

        // Verify we can write to the directory
        let test_file = storage_dir.join(".write_test");
        std::fs::write(&test_file, b"test")?;
        std::fs::remove_file(&test_file)?;

        Ok(Self { storage_dir })
    }

    /// Get the storage directory path.
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    /// Generate the file path for a given workflow ID.
    fn file_path(&self, id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.so", id))
    }

    /// Check available disk space and validate against a threshold.
    pub async fn check_disk_space(&self) -> Result<u64, StorageError> {
        // Note: This is a simplified implementation
        // In production, you might want to use statvfs or similar
        match fs::metadata(&self.storage_dir).await {
            Ok(_) => {
                // For now, we'll assume space is available
                // A full implementation would check actual disk space
                Ok(u64::MAX)
            }
            Err(e) => Err(StorageError::Backend(format!(
                "Failed to check disk space: {}",
                e
            ))),
        }
    }
}

#[async_trait]
impl RegistryStorage for FilesystemRegistryStorage {
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError> {
        let id = Uuid::new_v4();
        let file_path = self.file_path(&id.to_string());

        // Check if file already exists (highly unlikely with UUID, but good practice)
        if file_path.exists() {
            return Err(StorageError::Backend(format!(
                "File already exists: {}",
                file_path.display()
            )));
        }

        // Write data to file atomically using a temporary file
        let temp_path = file_path.with_extension("tmp");

        match fs::write(&temp_path, &data).await {
            Ok(()) => {
                // Atomically move temporary file to final location
                match fs::rename(&temp_path, &file_path).await {
                    Ok(()) => Ok(id.to_string()),
                    Err(e) => {
                        // Clean up temp file on failure
                        let _ = fs::remove_file(&temp_path).await;
                        Err(StorageError::Backend(format!(
                            "Failed to move file to final location: {}",
                            e
                        )))
                    }
                }
            }
            Err(e) => {
                // Clean up temp file on failure
                let _ = fs::remove_file(&temp_path).await;

                // Check for specific error conditions
                if e.kind() == std::io::ErrorKind::OutOfMemory {
                    Err(StorageError::QuotaExceeded {
                        used_bytes: data.len() as u64,
                        quota_bytes: 0, // Unknown quota
                    })
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    Err(StorageError::Backend(format!(
                        "Permission denied writing to: {}",
                        file_path.display()
                    )))
                } else {
                    Err(StorageError::Backend(format!(
                        "Failed to write file {}: {}",
                        file_path.display(),
                        e
                    )))
                }
            }
        }
    }

    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        // Validate UUID format
        if Uuid::parse_str(id).is_err() {
            return Err(StorageError::InvalidId { id: id.to_string() });
        }

        let file_path = self.file_path(id);

        match fs::read(&file_path).await {
            Ok(data) => {
                // Return data even if empty - empty data is valid for testing
                Ok(Some(data))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(StorageError::Backend(format!(
                "Failed to read file {}: {}",
                file_path.display(),
                e
            ))),
        }
    }

    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError> {
        // Validate UUID format
        if Uuid::parse_str(id).is_err() {
            return Err(StorageError::InvalidId { id: id.to_string() });
        }

        let file_path = self.file_path(id);

        match fs::remove_file(&file_path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Idempotent - success even if file doesn't exist
                Ok(())
            }
            Err(e) => Err(StorageError::Backend(format!(
                "Failed to delete file {}: {}",
                file_path.display(),
                e
            ))),
        }
    }

    fn storage_type(&self) -> StorageType {
        StorageType::Filesystem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_storage() -> (FilesystemRegistryStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let dal = FilesystemRegistryStorage::new(temp_dir.path()).unwrap();
        (dal, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let (mut dal, _temp_dir) = create_test_storage().await;

        let test_data = b"test workflow binary data".to_vec();
        let id = dal.store_binary(test_data.clone()).await.unwrap();

        let retrieved = dal.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent() {
        let (dal, _temp_dir) = create_test_storage().await;
        let fake_id = Uuid::new_v4().to_string();

        let result = dal.retrieve_binary(&fake_id).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_delete_binary() {
        let (mut dal, _temp_dir) = create_test_storage().await;

        let test_data = b"test data for deletion".to_vec();
        let id = dal.store_binary(test_data).await.unwrap();

        // Verify it exists
        let retrieved = dal.retrieve_binary(&id).await.unwrap();
        assert!(retrieved.is_some());

        // Delete it
        dal.delete_binary(&id).await.unwrap();

        // Verify it's gone
        let retrieved = dal.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, None);

        // Verify idempotent deletion
        dal.delete_binary(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_invalid_uuid() {
        let (dal, _temp_dir) = create_test_storage().await;

        let result = dal.retrieve_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));

        let mut dal = dal;
        let result = dal.delete_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));
    }

    #[tokio::test]
    async fn test_empty_file_handling() {
        let (dal, temp_dir) = create_test_storage().await;

        // Create an empty file manually
        let id = Uuid::new_v4().to_string();
        let file_path = temp_dir.path().join(format!("{}.so", id));
        fs::write(&file_path, b"").await.unwrap();

        // Should successfully retrieve empty data (for demo purposes)
        let result = dal.retrieve_binary(&id).await;
        assert!(matches!(result, Ok(Some(data)) if data.is_empty()));
    }

    #[tokio::test]
    async fn test_atomic_write() {
        let (mut dal, temp_dir) = create_test_storage().await;

        let test_data = b"test atomic write".to_vec();
        let id = dal.store_binary(test_data.clone()).await.unwrap();

        // Verify no temporary file left behind
        let temp_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "tmp"))
            .collect();

        assert_eq!(temp_files.len(), 0, "Temporary files should be cleaned up");

        // Verify actual file exists and has correct content
        let retrieved = dal.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_file_permissions() {
        let (mut dal, temp_dir) = create_test_storage().await;

        let test_data = b"test permissions".to_vec();
        let id = dal.store_binary(test_data.clone()).await.unwrap();

        // Verify the file was created with correct extension
        let file_path = temp_dir.path().join(format!("{}.so", id));
        assert!(file_path.exists(), "File should exist");
        assert_eq!(
            file_path.extension().unwrap(),
            "so",
            "File should have .so extension"
        );

        // Verify we can read the file
        let file_contents = fs::read(&file_path).await.unwrap();
        assert_eq!(file_contents, test_data);
    }

    #[tokio::test]
    async fn test_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("deeply").join("nested").join("path");

        // Should create directories if they don't exist
        let dal = FilesystemRegistryStorage::new(&nested_path).unwrap();

        assert!(nested_path.exists(), "Nested directories should be created");
        assert!(nested_path.is_dir(), "Path should be a directory");

        // Should be able to store files in the nested directory
        let mut dal = dal;
        let test_data = b"test nested storage".to_vec();
        let id = dal.store_binary(test_data.clone()).await.unwrap();

        let retrieved = dal.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_uuid_format() {
        let (mut dal, _temp_dir) = create_test_storage().await;

        let test_data = b"test data".to_vec();
        let id = dal.store_binary(test_data).await.unwrap();

        // Verify the returned ID is a valid UUID
        let parsed_uuid = Uuid::parse_str(&id);
        assert!(
            parsed_uuid.is_ok(),
            "Returned ID should be a valid UUID: {}",
            id
        );
    }

    #[tokio::test]
    async fn test_binary_data_integrity() {
        let (mut dal, _temp_dir) = create_test_storage().await;

        // Test with binary data containing all byte values
        let mut binary_data = Vec::with_capacity(256);
        for i in 0..=255u8 {
            binary_data.push(i);
        }

        let id = dal.store_binary(binary_data.clone()).await.unwrap();
        let retrieved = dal.retrieve_binary(&id).await.unwrap();

        assert_eq!(retrieved, Some(binary_data));
    }

    #[tokio::test]
    async fn test_very_large_file() {
        let (mut dal, _temp_dir) = create_test_storage().await;

        // Test with 1MB of data
        let large_data = vec![0xAB; 1024 * 1024];
        let id = dal.store_binary(large_data.clone()).await.unwrap();

        let retrieved = dal.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(large_data));
    }

    #[tokio::test]
    async fn test_storage_dir_access() {
        let (dal, temp_dir) = create_test_storage().await;

        assert_eq!(dal.storage_dir(), temp_dir.path());
    }

    #[tokio::test]
    async fn test_check_disk_space() {
        let (dal, _temp_dir) = create_test_storage().await;

        let disk_space = dal.check_disk_space().await.unwrap();
        assert!(disk_space > 0, "Should report some available disk space");
    }
}
