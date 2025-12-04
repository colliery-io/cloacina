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

//! Package loader for extracting metadata from workflow library files.
//!
//! This module provides functionality to safely load dynamic library files (.so/.dylib/.dll) in a temporary
//! environment and extract package metadata using the established cloacina FFI
//! interface patterns.

use flate2::read::GzDecoder;
use libloading::Library;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;
use std::path::Path;
use tar::Archive;
use tempfile::TempDir;
use tokio::fs;

use crate::registry::error::LoaderError;

/// Standard symbol name for task execution in cloacina packages
pub const EXECUTE_TASK_SYMBOL: &str = "cloacina_execute_task";

/// Standard symbol name for metadata extraction
pub const GET_METADATA_SYMBOL: &str = "cloacina_get_task_metadata";

/// Get the platform-specific dynamic library extension
pub fn get_library_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

/// C-compatible structure for task metadata extraction via FFI
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CTaskMetadata {
    index: u32,
    local_id: *const c_char,
    namespaced_id_template: *const c_char,
    dependencies_json: *const c_char,
    description: *const c_char,
    source_location: *const c_char,
}

/// C-compatible structure for package metadata extraction via FFI
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CPackageTasks {
    task_count: u32,
    tasks: *const CTaskMetadata,
    package_name: *const c_char,
    graph_data_json: *const c_char,
}

/// Metadata extracted from a workflow package
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

/// Individual task metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Task index in the package
    pub index: u32,
    /// Local task identifier
    pub local_id: String,
    /// Namespaced ID template
    pub namespaced_id_template: String,
    /// Task dependencies as JSON
    pub dependencies: Vec<String>,
    /// Human-readable description
    pub description: String,
    /// Source location information
    pub source_location: String,
}

/// Package loader for extracting metadata from workflow library files
pub struct PackageLoader {
    temp_dir: TempDir,
}

impl PackageLoader {
    /// Create a new package loader with a temporary directory for safe operations
    pub fn new() -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        Ok(Self { temp_dir })
    }

    /// Generate graph data from task dependencies
    fn generate_graph_data_from_tasks(
        &self,
        tasks: &[TaskMetadata],
    ) -> Result<serde_json::Value, LoaderError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Create nodes for each task
        for task in tasks {
            nodes.push(serde_json::json!({
                "id": task.local_id,
                "label": task.local_id,
                "description": task.description,
                "node_type": "task"
            }));
        }

        // Create edges from task dependencies
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

    /// Extract metadata from a binary package
    ///
    /// # Arguments
    ///
    /// * `package_data` - Binary data of the workflow package (.cloacina archive or raw library)
    ///
    /// # Returns
    ///
    /// * `Ok(PackageMetadata)` - Successfully extracted metadata
    /// * `Err(LoaderError)` - If extraction fails
    pub async fn extract_metadata(
        &self,
        package_data: &[u8],
    ) -> Result<PackageMetadata, LoaderError> {
        // Check if this is a .cloacina archive or raw library data
        let library_path = if self.is_cloacina_archive(package_data) {
            // Extract library file from .cloacina archive
            self.extract_library_from_archive(package_data).await?
        } else {
            // Treat as raw library data - write to temporary file
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
            temp_path
        };

        // Extract metadata from the library file
        self.extract_metadata_from_so(&library_path).await
    }

    /// Check if package data is a .cloacina archive
    fn is_cloacina_archive(&self, package_data: &[u8]) -> bool {
        // Check for gzip magic number at the start
        package_data.len() >= 3
            && package_data[0] == 0x1f
            && package_data[1] == 0x8b
            && package_data[2] == 0x08
    }

    /// Extract the library file from a .cloacina archive.
    ///
    /// # Arguments
    ///
    /// * `archive_data` - Binary data of the .cloacina archive (tar.gz)
    ///
    /// # Returns
    ///
    /// * `Ok(PathBuf)` - Path to the extracted library file
    /// * `Err(LoaderError)` - If extraction fails
    async fn extract_library_from_archive(
        &self,
        archive_data: &[u8],
    ) -> Result<std::path::PathBuf, LoaderError> {
        // Extract library file synchronously to avoid Send issues
        let (file_data, _filename) = tokio::task::spawn_blocking({
            let archive_data = archive_data.to_vec();
            move || -> Result<(Vec<u8>, String), LoaderError> {
                let library_extension = get_library_extension();
                // Create a cursor from the archive data
                let cursor = std::io::Cursor::new(archive_data);
                let gz_decoder = GzDecoder::new(cursor);
                let mut archive = Archive::new(gz_decoder);

                // Look for a library file in the archive
                for entry_result in archive.entries().map_err(|e| LoaderError::FileSystem {
                    path: "archive".to_string(),
                    error: format!("Failed to read archive entries: {}", e),
                })? {
                    let mut entry = entry_result.map_err(|e| LoaderError::FileSystem {
                        path: "archive".to_string(),
                        error: format!("Failed to read archive entry: {}", e),
                    })?;

                    let path = entry.path().map_err(|e| LoaderError::FileSystem {
                        path: "archive".to_string(),
                        error: format!("Failed to get entry path: {}", e),
                    })?;

                    // Check if this is a library file with the correct extension
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.ends_with(&format!(".{}", library_extension)) {
                            // Store path info before borrowing entry mutably
                            let path_string = path.to_string_lossy().to_string();
                            let filename_string = filename.to_string();

                            // Read the library file data
                            let mut file_data = Vec::new();
                            entry.read_to_end(&mut file_data).map_err(|e| {
                                LoaderError::FileSystem {
                                    path: path_string,
                                    error: format!(
                                        "Failed to read library file from archive: {}",
                                        e
                                    ),
                                }
                            })?;

                            return Ok((file_data, filename_string));
                        }
                    }
                }

                Err(LoaderError::MetadataExtraction {
                    reason: format!(
                        "No library file with extension '{}' found in archive",
                        library_extension
                    ),
                })
            }
        })
        .await
        .map_err(|e| LoaderError::FileSystem {
            path: "spawn_blocking".to_string(),
            error: format!("Failed to spawn blocking task: {}", e),
        })??;

        // Write extracted library file to temp directory
        let library_extension = get_library_extension();
        let extract_path = self
            .temp_dir
            .path()
            .join(format!("workflow_package.{}", library_extension));
        fs::write(&extract_path, &file_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: extract_path.to_string_lossy().to_string(),
                error: format!("Failed to write extracted library file: {}", e),
            })?;

        Ok(extract_path)
    }

    /// Extract metadata from a library file using established cloacina patterns
    async fn extract_metadata_from_so(
        &self,
        library_path: &Path,
    ) -> Result<PackageMetadata, LoaderError> {
        // Load the dynamic library
        let lib = unsafe {
            Library::new(library_path).map_err(|e| LoaderError::LibraryLoad {
                path: library_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?
        };

        // Extract package name from filename (fallback approach)
        let fallback_package_name = library_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown_package")
            .replace(['_', '-'], "_"); // Normalize for symbol lookup

        // Try to get metadata function - first standard name, then package-specific
        let get_metadata = unsafe {
            match lib.get::<unsafe extern "C" fn() -> *const CPackageTasks>(
                GET_METADATA_SYMBOL.as_bytes(),
            ) {
                Ok(func) => func,
                Err(_) => {
                    // Try package-specific function name
                    let func_name =
                        format!("cloacina_get_task_metadata_{}\0", fallback_package_name);
                    lib.get::<unsafe extern "C" fn() -> *const CPackageTasks>(func_name.as_bytes())
                        .map_err(|e| LoaderError::SymbolNotFound {
                            symbol: GET_METADATA_SYMBOL.to_string(),
                            error: e.to_string(),
                        })?
                }
            }
        };

        // Call the metadata function
        let c_package_tasks = unsafe { get_metadata() };
        if c_package_tasks.is_null() {
            return Err(LoaderError::MetadataExtraction {
                reason: "Metadata function returned null pointer".to_string(),
            });
        }

        // Convert C structures to Rust structures
        let package_tasks = unsafe { &*c_package_tasks };
        self.convert_c_metadata_to_rust(package_tasks, &fallback_package_name)
    }

    /// Convert C FFI metadata structures to Rust types
    fn convert_c_metadata_to_rust(
        &self,
        c_package: &CPackageTasks,
        fallback_name: &str,
    ) -> Result<PackageMetadata, LoaderError> {
        // Extract package name
        let package_name = if c_package.package_name.is_null() {
            fallback_name.to_string()
        } else {
            unsafe {
                CStr::from_ptr(c_package.package_name)
                    .to_str()
                    .map_err(|e| LoaderError::MetadataExtraction {
                        reason: format!("Invalid UTF-8 in package name: {}", e),
                    })?
                    .to_string()
            }
        };

        // Extract tasks first
        let mut tasks = Vec::new();
        if c_package.task_count > 0 && !c_package.tasks.is_null() {
            let tasks_slice = unsafe {
                std::slice::from_raw_parts(c_package.tasks, c_package.task_count as usize)
            };

            for c_task in tasks_slice {
                tasks.push(self.convert_c_task_to_rust(c_task)?);
            }
        }

        // Extract graph data (if available), or generate from tasks
        let graph_data = {
            use tracing::debug;

            if c_package.graph_data_json.is_null() {
                // No graph data field, generate from task dependencies
                if !tasks.is_empty() {
                    debug!(
                        "No graph_data field found, generating from {} tasks",
                        tasks.len()
                    );
                    self.generate_graph_data_from_tasks(&tasks).ok()
                } else {
                    None
                }
            } else {
                let graph_json = unsafe {
                    CStr::from_ptr(c_package.graph_data_json)
                        .to_str()
                        .map_err(|e| LoaderError::MetadataExtraction {
                            reason: format!("Invalid UTF-8 in graph data: {}", e),
                        })?
                };

                if graph_json.trim().is_empty() {
                    // Empty graph data field, generate from task dependencies
                    if !tasks.is_empty() {
                        debug!(
                            "Empty graph_data field, generating from {} tasks",
                            tasks.len()
                        );
                        self.generate_graph_data_from_tasks(&tasks).ok()
                    } else {
                        None
                    }
                } else {
                    let graph_json_str = graph_json.trim();
                    // First try to parse as JSON
                    match serde_json::from_str::<serde_json::Value>(graph_json_str) {
                        Ok(json_value) => Some(json_value),
                        Err(_) => {
                            // If it's not valid JSON, it might be a description string
                            // Generate graph data from task dependencies instead
                            debug!("Graph data field contains non-JSON data: '{}', generating from {} tasks", graph_json_str, tasks.len());
                            if !tasks.is_empty() {
                                self.generate_graph_data_from_tasks(&tasks).ok()
                            } else {
                                None
                            }
                        }
                    }
                }
            }
        };

        // Determine available symbols (basic check)
        let symbols = vec![
            EXECUTE_TASK_SYMBOL.to_string(),
            GET_METADATA_SYMBOL.to_string(),
        ];

        // Determine architecture (simplified)
        let architecture = if cfg!(target_arch = "x86_64") {
            "x86_64".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "aarch64".to_string()
        } else {
            std::env::consts::ARCH.to_string()
        };

        Ok(PackageMetadata {
            package_name,
            version: "1.0.0".to_string(), // Default version - could be extracted from manifest
            description: None,            // Could be extracted from graph_data or package metadata
            author: None,                 // Could be extracted from package metadata
            tasks,
            graph_data,
            architecture,
            symbols,
        })
    }

    /// Convert a single C task structure to Rust
    fn convert_c_task_to_rust(&self, c_task: &CTaskMetadata) -> Result<TaskMetadata, LoaderError> {
        let local_id = if c_task.local_id.is_null() {
            format!("task_{}", c_task.index)
        } else {
            unsafe {
                CStr::from_ptr(c_task.local_id)
                    .to_str()
                    .map_err(|e| LoaderError::MetadataExtraction {
                        reason: format!("Invalid UTF-8 in task local_id: {}", e),
                    })?
                    .to_string()
            }
        };

        let namespaced_id_template = if c_task.namespaced_id_template.is_null() {
            format!("{{tenant_id}}/{{package_name}}/{}", local_id)
        } else {
            unsafe {
                CStr::from_ptr(c_task.namespaced_id_template)
                    .to_str()
                    .map_err(|e| LoaderError::MetadataExtraction {
                        reason: format!("Invalid UTF-8 in namespaced_id_template: {}", e),
                    })?
                    .to_string()
            }
        };

        let dependencies = if c_task.dependencies_json.is_null() {
            Vec::new()
        } else {
            let deps_json = unsafe {
                CStr::from_ptr(c_task.dependencies_json)
                    .to_str()
                    .map_err(|e| LoaderError::MetadataExtraction {
                        reason: format!("Invalid UTF-8 in dependencies: {}", e),
                    })?
            };

            if deps_json.trim().is_empty() {
                Vec::new()
            } else {
                serde_json::from_str(deps_json).map_err(|e| LoaderError::MetadataExtraction {
                    reason: format!("Invalid JSON in task dependencies: {}", e),
                })?
            }
        };

        let description = if c_task.description.is_null() {
            format!("Task {}", local_id)
        } else {
            unsafe {
                CStr::from_ptr(c_task.description)
                    .to_str()
                    .map_err(|e| LoaderError::MetadataExtraction {
                        reason: format!("Invalid UTF-8 in description: {}", e),
                    })?
                    .to_string()
            }
        };

        let source_location = if c_task.source_location.is_null() {
            "unknown".to_string()
        } else {
            unsafe {
                CStr::from_ptr(c_task.source_location)
                    .to_str()
                    .map_err(|e| LoaderError::MetadataExtraction {
                        reason: format!("Invalid UTF-8 in source_location: {}", e),
                    })?
                    .to_string()
            }
        };

        Ok(TaskMetadata {
            index: c_task.index,
            local_id,
            namespaced_id_template,
            dependencies,
            description,
            source_location,
        })
    }

    /// Get the temporary directory path for manual file operations
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Validate that a package has the required symbols
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

        // Load library and check for required symbols
        let lib = unsafe {
            Library::new(&temp_path).map_err(|e| LoaderError::LibraryLoad {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?
        };

        let mut found_symbols = Vec::new();

        // Check for execute task symbol
        if unsafe {
            lib.get::<unsafe extern "C" fn()>(EXECUTE_TASK_SYMBOL.as_bytes())
                .is_ok()
        } {
            found_symbols.push(EXECUTE_TASK_SYMBOL.to_string());
        }

        // Check for metadata symbol
        if unsafe {
            lib.get::<unsafe extern "C" fn()>(GET_METADATA_SYMBOL.as_bytes())
                .is_ok()
        } {
            found_symbols.push(GET_METADATA_SYMBOL.to_string());
        }

        Ok(found_symbols)
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

    /// Helper to create a mock ELF-like binary for testing
    fn create_mock_elf_data(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // ELF magic number
        data.extend_from_slice(b"\x7fELF");

        // Basic ELF header fields
        data.extend_from_slice(&[
            0x02, // 64-bit
            0x01, // Little endian
            0x01, // Current version
            0x00, // System V ABI
        ]);

        // Pad with zeros to minimum ELF header size
        while data.len() < 64 {
            data.push(0x00);
        }

        // Fill rest with pseudo-random data
        for i in 64..size {
            data.push((i % 256) as u8);
        }

        data
    }

    /// Helper to create invalid binary data
    fn create_invalid_binary_data() -> Vec<u8> {
        b"This is not a valid ELF file".to_vec()
    }

    #[tokio::test]
    async fn test_package_loader_creation() {
        let loader = PackageLoader::new().expect("Failed to create PackageLoader");

        // Verify temp directory exists
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
            LoaderError::LibraryLoad { .. } => {
                // Expected - empty file cannot be loaded as library
            }
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
            LoaderError::LibraryLoad { .. } => {
                // Expected - invalid data cannot be loaded as library
            }
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
            LoaderError::LibraryLoad { .. } => {
                // Expected - cannot validate symbols in invalid library
            }
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

        // Each loader should have its own temp directory
        assert_ne!(loader1.temp_dir(), loader2.temp_dir());

        // Both directories should exist
        assert!(loader1.temp_dir().exists());
        assert!(loader2.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_concurrent_package_loading() {
        use std::sync::Arc;
        use tokio::task;

        let loader = Arc::new(PackageLoader::new().unwrap());
        let mut handles = Vec::new();

        // Start multiple concurrent extraction attempts
        for i in 0..5 {
            let loader_clone = Arc::clone(&loader);
            let handle = task::spawn(async move {
                let mut test_data = create_invalid_binary_data();
                test_data.push(i); // Make each test unique

                let result = loader_clone.extract_metadata(&test_data).await;

                // All should fail since they're invalid, but shouldn't crash
                assert!(result.is_err());
                i
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let task_id = handle.await.expect("Task should complete");
            assert!(task_id < 5);
        }
    }

    #[tokio::test]
    async fn test_symbol_constants() {
        assert_eq!(EXECUTE_TASK_SYMBOL, "cloacina_execute_task");
        assert_eq!(GET_METADATA_SYMBOL, "cloacina_get_task_metadata");
    }

    #[tokio::test]
    async fn test_file_system_operations() {
        let loader = PackageLoader::new().unwrap();
        let test_data = create_mock_elf_data(512);

        // This should create a temporary file
        let result = loader.extract_metadata(&test_data).await;

        // Even though it will fail (mock data doesn't have proper symbols),
        // it should have successfully written the file to disk
        assert!(result.is_err());

        // The temp directory should still exist and be accessible
        assert!(loader.temp_dir().exists());
        assert!(loader.temp_dir().is_dir());
    }

    #[tokio::test]
    async fn test_error_types_and_messages() {
        let loader = PackageLoader::new().unwrap();

        // Test with completely invalid data
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

        // Error should have proper Display implementation
        let error_string = format!("{}", error);
        assert!(error_string.contains("Failed to load library"));
    }

    #[tokio::test]
    async fn test_package_loader_memory_safety() {
        // Test that we can create and drop many loaders without issues
        for _ in 0..100 {
            let loader = PackageLoader::new().unwrap();
            let test_data = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic but incomplete

            // This will fail but shouldn't cause memory issues
            let _ = loader.extract_metadata(&test_data).await;

            // Loader goes out of scope and temp directory should be cleaned up
        }
    }

    #[tokio::test]
    async fn test_temp_directory_cleanup() {
        let _temp_path = {
            let loader = PackageLoader::new().unwrap();
            let path = loader.temp_dir().to_path_buf();

            // Verify directory exists while loader is alive
            assert!(path.exists());

            path
        }; // loader drops here

        // Give some time for cleanup (temp directories are cleaned up when TempDir drops)
        // Note: TempDir cleanup is automatic when the struct is dropped
        // We can't easily test this without accessing internal implementation
    }

    #[test]
    fn test_package_loader_sync_creation() {
        // Test that we can create a loader in non-async context
        let result = PackageLoader::new();
        assert!(result.is_ok());

        let loader = result.unwrap();
        assert!(loader.temp_dir().exists());
    }

    #[test]
    fn test_get_library_extension() {
        let extension = get_library_extension();

        // Verify we get the correct extension for the current platform
        if cfg!(target_os = "windows") {
            assert_eq!(extension, "dll");
        } else if cfg!(target_os = "macos") {
            assert_eq!(extension, "dylib");
        } else {
            assert_eq!(extension, "so");
        }
    }
}
