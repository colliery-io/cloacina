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

//! Package format detection and library extraction from .cloacina archives.

use tracing::debug;

use super::RegistryReconciler;
use crate::registry::error::RegistryError;

impl RegistryReconciler {
    /// Check if package data is a .cloacina archive
    pub(super) fn is_cloacina_package(&self, package_data: &[u8]) -> bool {
        // Check for gzip magic number at the start
        package_data.len() >= 3
            && package_data[0] == 0x1f
            && package_data[1] == 0x8b
            && package_data[2] == 0x08
    }

    /// Extract library file data from a .cloacina archive
    pub(super) async fn extract_library_from_cloacina(
        &self,
        package_data: &[u8],
    ) -> Result<Vec<u8>, RegistryError> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        use tar::Archive;

        debug!(
            "Starting library extraction from .cloacina archive, data length: {}",
            package_data.len()
        );

        // Get platform-specific library extension
        let library_extension = if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        debug!("Looking for library with extension: {}", library_extension);

        // Extract library file synchronously to avoid Send issues
        let library_data = tokio::task::spawn_blocking({
            let package_data = package_data.to_vec();
            let library_extension = library_extension.to_string();
            move || -> Result<Vec<u8>, RegistryError> {
                debug!("Starting spawn_blocking task for library extraction");

                // Create a cursor from the archive data
                let cursor = std::io::Cursor::new(package_data);
                debug!("Created cursor from package data");

                let gz_decoder = GzDecoder::new(cursor);
                debug!("Created GzDecoder");

                let mut archive = Archive::new(gz_decoder);
                debug!("Created Archive from GzDecoder");

                // Look for a library file in the archive
                debug!("Starting to iterate through archive entries");
                for entry_result in archive.entries().map_err(|e| {
                    debug!("Error reading archive entries: {}", e);
                    RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                        path: "archive".to_string(),
                        error: format!("Failed to read archive entries: {}", e),
                    })
                })? {
                    let mut entry = entry_result.map_err(|e| {
                        RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                            path: "archive".to_string(),
                            error: format!("Failed to read archive entry: {}", e),
                        })
                    })?;

                    let path = entry.path().map_err(|e| {
                        RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                            path: "archive".to_string(),
                            error: format!("Failed to get entry path: {}", e),
                        })
                    })?;

                    // Check if this is a library file with the correct extension
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.ends_with(&format!(".{}", library_extension)) {
                            // Store path info before borrowing entry mutably
                            let path_string = path.to_string_lossy().to_string();

                            // Read the library file data
                            let mut file_data = Vec::new();
                            entry.read_to_end(&mut file_data).map_err(|e| {
                                RegistryError::Loader(
                                    crate::registry::error::LoaderError::FileSystem {
                                        path: path_string,
                                        error: format!(
                                            "Failed to read library file from archive: {}",
                                            e
                                        ),
                                    },
                                )
                            })?;

                            return Ok(file_data);
                        }
                    }
                }

                Err(RegistryError::Loader(
                    crate::registry::error::LoaderError::MetadataExtraction {
                        reason: format!(
                            "No library file with extension '{}' found in archive",
                            library_extension
                        ),
                    },
                ))
            }
        })
        .await
        .map_err(|e| {
            RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                path: "spawn_blocking".to_string(),
                error: format!("Failed to spawn blocking task: {}", e),
            })
        })??;

        Ok(library_data)
    }
}
