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

//! Task metadata extraction from dynamic libraries via fidius-host.

use tokio::fs;

use super::types::{OwnedTaskMetadata, OwnedTaskMetadataCollection};
use super::TaskRegistrar;
use crate::registry::error::LoaderError;
use crate::registry::loader::package_loader::get_library_extension;

impl TaskRegistrar {
    /// Extract task metadata from a library using the fidius-host plugin API.
    ///
    /// Writes the package data to a temporary file, loads it via `fidius_host::load_library`,
    /// and calls `get_task_metadata` (method index 0) through a `PluginHandle`.
    ///
    /// Returns `OwnedTaskMetadataCollection` — all strings are owned, no raw pointers
    /// remain after this function returns.
    pub(super) async fn extract_task_metadata_from_library(
        &self,
        package_data: &[u8],
    ) -> Result<OwnedTaskMetadataCollection, LoaderError> {
        // Write package to temporary file with the correct platform extension
        let library_extension = get_library_extension();
        let temp_path = self.temp_dir.path().join(format!(
            "tasks_{}.{}",
            uuid::Uuid::new_v4(),
            library_extension
        ));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        // Load via fidius-host — validates magic, ABI version, wire format, etc.
        let loaded = fidius_host::loader::load_library(&temp_path).map_err(
            |e: fidius_host::LoadError| LoaderError::LibraryLoad {
                path: temp_path.to_string_lossy().to_string(),
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
        let metadata: cloacina_workflow_plugin::PackageTasksMetadata =
            handle
                .call_method(0, &())
                .map_err(|e| LoaderError::MetadataExtraction {
                    reason: format!("Failed to call get_task_metadata: {}", e),
                })?;

        // Convert PackageTasksMetadata → OwnedTaskMetadataCollection
        let task_count = metadata.tasks.len();
        let tasks: Vec<OwnedTaskMetadata> = metadata
            .tasks
            .into_iter()
            .map(|t| OwnedTaskMetadata {
                local_id: t.id,
                dependencies_json: serde_json::to_string(&t.dependencies)
                    .unwrap_or_else(|_| "[]".to_string()),
            })
            .collect();

        // Keep handle alive to prevent dlclose — inventory linked-list corruption
        if let Ok(mut cache) = self.handle_cache.lock() {
            cache.push(handle);
        }

        tracing::debug!(
            "Extracted metadata via fidius: package={}, workflow={}, task_count={}",
            metadata.package_name,
            metadata.workflow_name,
            task_count
        );

        Ok(OwnedTaskMetadataCollection {
            workflow_name: metadata.workflow_name,
            package_name: metadata.package_name,
            tasks,
        })
    }
}
