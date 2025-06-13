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

//! Filesystem storage backend for workflow registry.
//!
//! This implementation stores binary workflow data as individual files on the
//! local filesystem. It's particularly suitable for SQLite deployments where
//! storing large binary data in the database is not optimal.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

use crate::registry::error::StorageError;
use crate::registry::traits::RegistryStorage;

/// Filesystem-based storage backend for workflow registry.
///
/// This storage backend stores each workflow package as a separate file
/// in a designated directory. Files are named using UUIDs to ensure
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
/// ```rust,no_run
/// use cloacina::registry::storage::FilesystemRegistryStorage;
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
    /// Create a new filesystem registry storage backend.
    ///
    /// # Arguments
    ///
    /// * `storage_dir` - Directory path where workflow files will be stored
    ///
    /// # Returns
    ///
    /// * `Ok(FilesystemRegistryStorage)` - Successfully created storage
    /// * `Err(std::io::Error)` - If directory creation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cloacina::registry::storage::FilesystemRegistryStorage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;
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
                // Verify data integrity (basic size check)
                if data.is_empty() {
                    Err(StorageError::DataCorruption {
                        id: id.to_string(),
                        reason: "File is empty".to_string(),
                    })
                } else {
                    Ok(Some(data))
                }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_storage() -> (FilesystemRegistryStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemRegistryStorage::new(temp_dir.path()).unwrap();
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        let test_data = b"test workflow binary data".to_vec();
        let id = storage.store_binary(test_data.clone()).await.unwrap();

        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent() {
        let (storage, _temp_dir) = create_test_storage().await;
        let fake_id = Uuid::new_v4().to_string();

        let result = storage.retrieve_binary(&fake_id).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_delete_binary() {
        let (mut storage, _temp_dir) = create_test_storage().await;

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

    #[tokio::test]
    async fn test_invalid_uuid() {
        let (storage, _temp_dir) = create_test_storage().await;

        let result = storage.retrieve_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));

        let mut storage = storage;
        let result = storage.delete_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));
    }

    #[tokio::test]
    async fn test_empty_file_corruption() {
        let (mut storage, temp_dir) = create_test_storage().await;

        // Create an empty file manually
        let id = Uuid::new_v4().to_string();
        let file_path = temp_dir.path().join(format!("{}.so", id));
        fs::write(&file_path, b"").await.unwrap();

        // Should detect corruption
        let result = storage.retrieve_binary(&id).await;
        assert!(matches!(result, Err(StorageError::DataCorruption { .. })));
    }

    #[tokio::test]
    async fn test_atomic_write() {
        let (mut storage, temp_dir) = create_test_storage().await;

        let test_data = b"test atomic write".to_vec();
        let id = storage.store_binary(test_data.clone()).await.unwrap();

        // Verify no temporary file left behind
        let temp_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "tmp"))
            .collect();

        assert_eq!(temp_files.len(), 0, "Temporary files should be cleaned up");

        // Verify actual file exists and has correct content
        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(test_data));
    }
}
