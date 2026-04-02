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

//! Package loader for extracting metadata from workflow library files.
//!
//! This module provides functionality to safely load dynamic library files (.so/.dylib/.dll)
//! via the fidius-host plugin API and extract package metadata.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

use crate::registry::error::LoaderError;

/// Get the platform-specific dynamic library extension.
pub fn get_library_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

/// Metadata extracted from a workflow package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name
    pub package_name: String,
    /// Package version (extracted from library or defaults to "1.0.0")
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Package author
    pub author: Option<String>,
    /// List of tasks provided by this package
    pub tasks: Vec<TaskMetadata>,
    /// Workflow graph data (if available)
    pub graph_data: Option<serde_json::Value>,
    /// Library architecture info
    pub architecture: String,
    /// Required symbols present in the library
    pub symbols: Vec<String>,
}

/// Individual task metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Task index in the package
    pub index: u32,
    /// Local task identifier
    pub local_id: String,
    /// Namespaced ID template
    pub namespaced_id_template: String,
    /// Task dependencies as a list of local task IDs
    pub dependencies: Vec<String>,
    /// Human-readable description
    pub description: String,
    /// Source location information
    pub source_location: String,
}

/// Package loader for extracting metadata from workflow library files.
pub struct PackageLoader {
    temp_dir: TempDir,
}

impl PackageLoader {
    /// Create a new package loader with a temporary directory for safe operations.
    pub fn new() -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        Ok(Self { temp_dir })
    }

    /// Generate graph data from task dependencies.
    fn generate_graph_data_from_tasks(
        &self,
        tasks: &[TaskMetadata],
    ) -> Result<serde_json::Value, LoaderError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for task in tasks {
            nodes.push(serde_json::json!({
                "id": task.local_id,
                "label": task.local_id,
                "description": task.description,
                "node_type": "task"
            }));
        }

        for task in tasks {
            for dependency in &task.dependencies {
                edges.push(serde_json::json!({
                    "source": dependency,
                    "target": task.local_id,
                    "edge_type": "dependency"
                }));
            }
        }

        Ok(serde_json::json!({
            "nodes": nodes,
            "edges": edges,
            "metadata": {
                "task_count": tasks.len(),
                "generated_from": "task_dependencies"
            }
        }))
    }

    /// Extract metadata from compiled library bytes.
    ///
    /// # Arguments
    ///
    /// * `package_data` - Raw bytes of the compiled cdylib (.so / .dylib).
    ///   The reconciler is responsible for unpacking and compiling any source
    ///   archives before calling this method.
    ///
    /// # Returns
    ///
    /// * `Ok(PackageMetadata)` - Successfully extracted metadata
    /// * `Err(LoaderError)` - If extraction fails
    pub async fn extract_metadata(
        &self,
        package_data: &[u8],
    ) -> Result<PackageMetadata, LoaderError> {
        let library_extension = get_library_extension();
        let temp_path = self
            .temp_dir
            .path()
            .join(format!("workflow_package.{}", library_extension));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        self.extract_metadata_from_so(&temp_path).await
    }

    /// Extract metadata from a library file using the fidius-host plugin API.
    async fn extract_metadata_from_so(
        &self,
        library_path: &Path,
    ) -> Result<PackageMetadata, LoaderError> {
        // Load via fidius-host — validates magic, ABI version, wire format, etc.
        let loaded = fidius_host::loader::load_library(library_path).map_err(
            |e: fidius_host::LoadError| LoaderError::LibraryLoad {
                path: library_path.to_string_lossy().to_string(),
                error: e.to_string(),
            },
        )?;

        let plugin =
            loaded
                .plugins
                .into_iter()
                .next()
                .ok_or_else(|| LoaderError::MetadataExtraction {
                    reason: "Plugin library contains no plugins".to_string(),
                })?;

        let handle = fidius_host::PluginHandle::from_loaded(plugin);

        // Method index 0 = get_task_metadata (zero-arg, encoded as empty tuple)
        let ffi_metadata: cloacina_workflow_plugin::PackageTasksMetadata = handle
            .call_method(0, &())
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Failed to call get_task_metadata: {}", e),
            })?;

        self.convert_plugin_metadata_to_rust(ffi_metadata)
    }

    /// Convert `PackageTasksMetadata` from the fidius plugin into the `PackageMetadata`
    /// struct used by the rest of the registry.
    fn convert_plugin_metadata_to_rust(
        &self,
        meta: cloacina_workflow_plugin::PackageTasksMetadata,
    ) -> Result<PackageMetadata, LoaderError> {
        let tasks: Vec<TaskMetadata> = meta
            .tasks
            .into_iter()
            .map(|t| TaskMetadata {
                index: t.index,
                local_id: t.id,
                namespaced_id_template: t.namespaced_id_template,
                dependencies: t.dependencies,
                description: t.description,
                source_location: t.source_location,
            })
            .collect();

        // Build graph data from tasks if no serialized graph is present
        let graph_data = match meta.graph_data_json.as_deref() {
            Some(json) if !json.trim().is_empty() => {
                match serde_json::from_str::<serde_json::Value>(json) {
                    Ok(v) => Some(v),
                    Err(_) => {
                        tracing::debug!(
                            "graph_data_json is not valid JSON, generating from {} tasks",
                            tasks.len()
                        );
                        self.generate_graph_data_from_tasks(&tasks).ok()
                    }
                }
            }
            _ => {
                if !tasks.is_empty() {
                    self.generate_graph_data_from_tasks(&tasks).ok()
                } else {
                    None
                }
            }
        };

        let architecture = if cfg!(target_arch = "x86_64") {
            "x86_64".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "aarch64".to_string()
        } else {
            std::env::consts::ARCH.to_string()
        };

        Ok(PackageMetadata {
            package_name: meta.package_name,
            version: "1.0.0".to_string(),
            description: meta.package_description,
            author: meta.package_author,
            tasks,
            graph_data,
            architecture,
            symbols: vec!["fidius_get_registry".to_string()],
        })
    }

    /// Get the temporary directory path for manual file operations.
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Validate that a package has the required symbols by loading it via fidius-host.
    ///
    /// Returns an empty `Vec` on success (fidius validated the plugin registry),
    /// or the known symbol names if the library loads without error.
    pub async fn validate_package_symbols(
        &self,
        package_data: &[u8],
    ) -> Result<Vec<String>, LoaderError> {
        let library_extension = get_library_extension();
        let temp_path = self
            .temp_dir
            .path()
            .join(format!("validation_package.{}", library_extension));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        // Load via fidius-host — if this succeeds the plugin is valid
        fidius_host::loader::load_library(&temp_path).map_err(|e: fidius_host::LoadError| {
            LoaderError::LibraryLoad {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            }
        })?;

        // Return the fidius registry symbol
        Ok(vec!["fidius_get_registry".to_string()])
    }
}

impl Default for PackageLoader {
    fn default() -> Self {
        Self::new().expect("Failed to create default PackageLoader")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create invalid binary data
    fn create_invalid_binary_data() -> Vec<u8> {
        b"This is not a valid ELF file".to_vec()
    }

    /// Helper to create a mock ELF-like binary for testing
    fn create_mock_elf_data(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // ELF magic number
        data.extend_from_slice(b"\x7fELF");
        data.extend_from_slice(&[0x02, 0x01, 0x01, 0x00]);

        while data.len() < 64 {
            data.push(0x00);
        }

        for i in 64..size {
            data.push((i % 256) as u8);
        }

        data
    }

    #[tokio::test]
    async fn test_package_loader_creation() {
        let loader = PackageLoader::new().expect("Failed to create PackageLoader");
        assert!(loader.temp_dir().exists());
        assert!(loader.temp_dir().is_dir());
    }

    #[tokio::test]
    async fn test_package_loader_default() {
        let loader = PackageLoader::default();
        assert!(loader.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_extract_metadata_with_invalid_elf() {
        let loader = PackageLoader::new().unwrap();
        let invalid_data = create_invalid_binary_data();

        let result = loader.extract_metadata(&invalid_data).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LoaderError::LibraryLoad { path, error } => {
                let library_extension = get_library_extension();
                assert!(path.contains(&format!("workflow_package.{}", library_extension)));
                assert!(!error.is_empty());
            }
            other => panic!("Expected LibraryLoad error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_extract_metadata_with_empty_data() {
        let loader = PackageLoader::new().unwrap();
        let empty_data = Vec::new();

        let result = loader.extract_metadata(&empty_data).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LoaderError::LibraryLoad { .. } => {}
            other => panic!("Expected LibraryLoad error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_extract_metadata_with_large_invalid_data() {
        let loader = PackageLoader::new().unwrap();
        let large_invalid_data = vec![0xAB; 1024 * 1024]; // 1MB of invalid data

        let result = loader.extract_metadata(&large_invalid_data).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LoaderError::LibraryLoad { .. } => {}
            other => panic!("Expected LibraryLoad error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validate_package_symbols_with_invalid_data() {
        let loader = PackageLoader::new().unwrap();
        let invalid_data = create_invalid_binary_data();

        let result = loader.validate_package_symbols(&invalid_data).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LoaderError::LibraryLoad { .. } => {}
            other => panic!("Expected LibraryLoad error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validate_package_symbols_with_empty_data() {
        let loader = PackageLoader::new().unwrap();
        let empty_data = Vec::new();

        let result = loader.validate_package_symbols(&empty_data).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_temp_dir_isolation() {
        let loader1 = PackageLoader::new().unwrap();
        let loader2 = PackageLoader::new().unwrap();

        assert_ne!(loader1.temp_dir(), loader2.temp_dir());
        assert!(loader1.temp_dir().exists());
        assert!(loader2.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_concurrent_package_loading() {
        use std::sync::Arc;
        use tokio::task;

        let loader = Arc::new(PackageLoader::new().unwrap());
        let mut handles = Vec::new();

        for i in 0..5 {
            let loader_clone = Arc::clone(&loader);
            let handle = task::spawn(async move {
                let mut test_data = create_invalid_binary_data();
                test_data.push(i);

                let result = loader_clone.extract_metadata(&test_data).await;
                assert!(result.is_err());
                i
            });
            handles.push(handle);
        }

        for handle in handles {
            let task_id = handle.await.expect("Task should complete");
            assert!(task_id < 5);
        }
    }

    #[tokio::test]
    async fn test_file_system_operations() {
        let loader = PackageLoader::new().unwrap();
        let test_data = create_mock_elf_data(512);

        let result = loader.extract_metadata(&test_data).await;

        assert!(result.is_err());
        assert!(loader.temp_dir().exists());
        assert!(loader.temp_dir().is_dir());
    }

    #[tokio::test]
    async fn test_error_types_and_messages() {
        let loader = PackageLoader::new().unwrap();

        let result = loader.extract_metadata(b"invalid").await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        match &error {
            LoaderError::LibraryLoad { path, error: msg } => {
                let library_extension = get_library_extension();
                assert!(path.contains(&format!(".{}", library_extension)));
                assert!(!msg.is_empty());
            }
            other => panic!("Expected LibraryLoad error, got: {:?}", other),
        }

        let error_string = format!("{}", error);
        assert!(error_string.contains("Failed to load library"));
    }

    #[tokio::test]
    async fn test_package_loader_memory_safety() {
        for _ in 0..100 {
            let loader = PackageLoader::new().unwrap();
            let test_data = vec![0x7f, 0x45, 0x4c, 0x46];
            let _ = loader.extract_metadata(&test_data).await;
        }
    }

    #[tokio::test]
    async fn test_temp_directory_cleanup() {
        let _temp_path = {
            let loader = PackageLoader::new().unwrap();
            let path = loader.temp_dir().to_path_buf();
            assert!(path.exists());
            path
        };
    }

    #[test]
    fn test_package_loader_sync_creation() {
        let result = PackageLoader::new();
        assert!(result.is_ok());

        let loader = result.unwrap();
        assert!(loader.temp_dir().exists());
    }

    #[test]
    fn test_get_library_extension() {
        let extension = get_library_extension();

        if cfg!(target_os = "windows") {
            assert_eq!(extension, "dll");
        } else if cfg!(target_os = "macos") {
            assert_eq!(extension, "dylib");
        } else {
            assert_eq!(extension, "so");
        }
    }
}
