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

//! File format validation for dynamic libraries.

use libloading::Library;
use std::path::Path;
use tokio::fs;

use super::types::ValidationResult;
use super::PackageValidator;

impl PackageValidator {
    /// Validate file format and basic structure.
    pub(super) async fn validate_file_format(
        &self,
        package_path: &Path,
        result: &mut ValidationResult,
    ) {
        // Check for supported dynamic library formats
        match fs::read(package_path).await {
            Ok(data) if data.len() >= 4 => {
                let is_valid_format = if &data[0..4] == b"\x7fELF" {
                    // ELF format (Linux)
                    if data.len() >= 5 {
                        match data[4] {
                            1 => result.compatibility.architecture = "32-bit".to_string(),
                            2 => result.compatibility.architecture = "64-bit".to_string(),
                            _ => result.warnings.push("Unknown ELF class".to_string()),
                        }
                    }
                    true
                } else if data.len() >= 4 && &data[0..4] == b"\xcf\xfa\xed\xfe" {
                    // Mach-O 64-bit format (macOS)
                    result.compatibility.architecture = "64-bit".to_string();
                    true
                } else if data.len() >= 4 && &data[0..4] == b"\xce\xfa\xed\xfe" {
                    // Mach-O 32-bit format (macOS)
                    result.compatibility.architecture = "32-bit".to_string();
                    true
                } else if data.len() >= 2 && &data[0..2] == b"MZ" {
                    // PE format (Windows)
                    result.compatibility.architecture = "Windows".to_string();
                    true
                } else {
                    false
                };

                if !is_valid_format {
                    result.errors.push(
                        "Package is not a valid dynamic library (not ELF, Mach-O, or PE format)"
                            .to_string(),
                    );
                }
            }
            Ok(_) => result
                .errors
                .push("Package file is too small to be a valid dynamic library".to_string()),
            Err(e) => result
                .errors
                .push(format!("Failed to read package file: {}", e)),
        }

        // Try to load as dynamic library
        match unsafe { Library::new(package_path) } {
            Ok(_) => {
                // Library loads successfully
            }
            Err(e) => {
                result.errors.push(format!(
                    "Package cannot be loaded as dynamic library: {}",
                    e
                ));
            }
        }
    }
}
