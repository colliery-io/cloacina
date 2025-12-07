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

//! Task metadata extraction from dynamic libraries.

use libloading::Library;
use std::ffi::CStr;
use tokio::fs;

use super::types::{OwnedTaskMetadata, OwnedTaskMetadataCollection, TaskMetadataCollection};
use super::TaskRegistrar;
use crate::registry::error::LoaderError;
use crate::registry::loader::package_loader::get_library_extension;

impl TaskRegistrar {
    /// Extract task metadata from library using get_task_metadata() FFI function.
    ///
    /// SAFETY: This function copies all string data from FFI pointers into owned Rust
    /// Strings BEFORE the library is unloaded. The returned `OwnedTaskMetadataCollection`
    /// contains no raw pointers and is safe to use after this function returns.
    pub(super) async fn extract_task_metadata_from_library(
        &self,
        package_data: &[u8],
    ) -> Result<OwnedTaskMetadataCollection, LoaderError> {
        // Write package to temporary file with correct extension
        let library_extension = get_library_extension();
        let temp_path = self
            .temp_dir
            .path()
            .join(format!("metadata_extract.{}", library_extension));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        // Load the library
        let lib = unsafe {
            Library::new(&temp_path).map_err(|e| LoaderError::LibraryLoad {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?
        };

        // Get the get_task_metadata function
        let get_metadata = unsafe {
            lib.get::<unsafe extern "C" fn() -> *const TaskMetadataCollection>(b"get_task_metadata")
                .map_err(|e| LoaderError::SymbolNotFound {
                    symbol: "get_task_metadata".to_string(),
                    error: e.to_string(),
                })?
        };

        // Call the FFI function to get metadata
        let metadata_ptr = unsafe { get_metadata() };
        if metadata_ptr.is_null() {
            return Err(LoaderError::MetadataExtraction {
                reason: "get_task_metadata() returned null pointer".to_string(),
            });
        }

        // CRITICAL: Copy ALL data from FFI pointers into owned Strings BEFORE lib is dropped.
        // The raw pointers in TaskMetadataCollection point to static strings inside the
        // loaded library. Once `lib` is dropped, those memory regions become invalid.
        let metadata = unsafe { &*metadata_ptr };

        // Copy workflow and package names
        let workflow_name = unsafe { CStr::from_ptr(metadata.workflow_name) }
            .to_str()
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Invalid workflow name: {}", e),
            })?
            .to_string();

        let package_name = unsafe { CStr::from_ptr(metadata.package_name) }
            .to_str()
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Invalid package name: {}", e),
            })?
            .to_string();

        // Copy all task metadata
        let tasks_slice =
            unsafe { std::slice::from_raw_parts(metadata.tasks, metadata.task_count as usize) };

        let mut owned_tasks = Vec::with_capacity(tasks_slice.len());
        for task in tasks_slice {
            let local_id = unsafe { CStr::from_ptr(task.local_id) }
                .to_str()
                .map_err(|e| LoaderError::MetadataExtraction {
                    reason: format!("Invalid task local_id: {}", e),
                })?
                .to_string();

            let dependencies_json = unsafe { CStr::from_ptr(task.dependencies_json) }
                .to_str()
                .map_err(|e| LoaderError::MetadataExtraction {
                    reason: format!("Invalid task dependencies_json: {}", e),
                })?
                .to_string();

            let constructor_fn_name = unsafe { CStr::from_ptr(task.constructor_fn_name) }
                .to_str()
                .map_err(|e| LoaderError::MetadataExtraction {
                    reason: format!("Invalid task constructor_fn_name: {}", e),
                })?
                .to_string();

            owned_tasks.push(OwnedTaskMetadata {
                local_id,
                dependencies_json,
                constructor_fn_name,
            });
        }

        tracing::debug!(
            "Extracted metadata: package={}, workflow={}, task_count={}",
            package_name,
            workflow_name,
            owned_tasks.len()
        );

        // Return owned data - safe to use after library is dropped
        // The `lib` variable is dropped here when function returns, unloading the library.
        // But we've already copied all the data we need into owned Strings.
        Ok(OwnedTaskMetadataCollection {
            workflow_name,
            package_name,
            tasks: owned_tasks,
        })
    }
}
