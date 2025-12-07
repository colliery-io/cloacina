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

//! Package extraction and format detection utilities.

use flate2::read::GzDecoder;
use std::io::Read;
use tar::Archive;

use super::WorkflowRegistryImpl;
use crate::registry::error::RegistryError;
use crate::registry::traits::RegistryStorage;

impl<S: RegistryStorage> WorkflowRegistryImpl<S> {
    /// Check if package data is a .cloacina archive (tar.gz format)
    pub(super) fn is_cloacina_package(data: &[u8]) -> bool {
        // Check for gzip magic number at the start
        data.len() >= 3 && data[0] == 0x1f && data[1] == 0x8b && data[2] == 0x08
    }

    /// Extract .so file from .cloacina package archive
    pub(super) async fn extract_so_from_cloacina(
        package_data: &[u8],
    ) -> Result<Vec<u8>, RegistryError> {
        // Create a cursor from the package data
        let cursor = std::io::Cursor::new(package_data);
        let gz_decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(gz_decoder);

        // Look for .so file in the archive
        for entry in archive
            .entries()
            .map_err(|e| RegistryError::ValidationError {
                reason: format!("Failed to read archive entries: {}", e),
            })?
        {
            let mut entry = entry.map_err(|e| RegistryError::ValidationError {
                reason: format!("Failed to read archive entry: {}", e),
            })?;

            let path = entry.path().map_err(|e| RegistryError::ValidationError {
                reason: format!("Failed to get entry path: {}", e),
            })?;

            // Check if this is a dynamic library file (.so on Linux, .dylib on macOS, .dll on Windows)
            if let Some(extension) = path.extension() {
                if extension == "so" || extension == "dylib" || extension == "dll" {
                    let mut library_data = Vec::new();
                    entry.read_to_end(&mut library_data).map_err(|e| {
                        RegistryError::ValidationError {
                            reason: format!("Failed to read library file from archive: {}", e),
                        }
                    })?;
                    return Ok(library_data);
                }
            }
        }

        Err(RegistryError::ValidationError {
            reason: "No dynamic library file (.so/.dylib/.dll) found in .cloacina package"
                .to_string(),
        })
    }
}
