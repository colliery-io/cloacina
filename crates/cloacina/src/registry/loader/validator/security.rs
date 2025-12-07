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

//! Security assessment for packages.

use std::path::Path;
use tokio::fs;

use super::types::{SecurityLevel, ValidationResult};
use super::PackageValidator;

impl PackageValidator {
    /// Perform security assessment of the package.
    pub(super) async fn assess_security(&self, package_path: &Path, result: &mut ValidationResult) {
        let mut security_issues = 0;

        // Check file permissions
        match fs::metadata(package_path).await {
            Ok(metadata) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();

                    // Check if file is world-writable
                    if mode & 0o002 != 0 {
                        result
                            .warnings
                            .push("Package file is world-writable".to_string());
                        security_issues += 1;
                    }

                    // Check if file is executable
                    if mode & 0o111 == 0 {
                        result
                            .warnings
                            .push("Package file is not executable".to_string());
                    }
                }
            }
            Err(e) => {
                result
                    .warnings
                    .push(format!("Cannot check file permissions: {}", e));
            }
        }

        // Basic heuristic checks for suspicious patterns
        match fs::read(package_path).await {
            Ok(data) => {
                // Check for suspicious strings (basic detection)
                let suspicious_patterns: [&[u8]; 6] =
                    [b"/bin/sh", b"system(", b"exec", b"curl", b"wget", b"nc "];

                for pattern in &suspicious_patterns {
                    if data.windows(pattern.len()).any(|window| window == *pattern) {
                        result.warnings.push(format!(
                            "Package contains potentially suspicious pattern: {}",
                            String::from_utf8_lossy(pattern)
                        ));
                        security_issues += 1;
                    }
                }
            }
            Err(e) => {
                result
                    .warnings
                    .push(format!("Cannot read package for security analysis: {}", e));
            }
        }

        // Determine security level
        result.security_level = if security_issues == 0 {
            SecurityLevel::Safe
        } else if security_issues <= 2 {
            SecurityLevel::Warning
        } else {
            SecurityLevel::Dangerous
        };
    }
}
