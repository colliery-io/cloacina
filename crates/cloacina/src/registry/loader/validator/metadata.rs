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

//! Package metadata validation.

use std::collections::HashSet;

use crate::registry::loader::package_loader::PackageMetadata;

use super::types::ValidationResult;
use super::PackageValidator;

impl PackageValidator {
    /// Validate package metadata for consistency and safety.
    pub(super) fn validate_metadata(
        &self,
        metadata: &PackageMetadata,
        result: &mut ValidationResult,
    ) {
        // Package name validation
        if metadata.package_name.is_empty() {
            result
                .errors
                .push("Package name cannot be empty".to_string());
        } else if !metadata
            .package_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            result
                .warnings
                .push("Package name contains non-standard characters".to_string());
        }

        // Version validation
        if metadata.version.is_empty() {
            result.warnings.push("Package version is empty".to_string());
        }

        // Task validation
        if metadata.tasks.is_empty() {
            result
                .warnings
                .push("Package contains no tasks".to_string());
        } else {
            let mut task_ids = HashSet::new();
            for task in &metadata.tasks {
                // Check for duplicate task IDs
                if !task_ids.insert(&task.local_id) {
                    result
                        .errors
                        .push(format!("Duplicate task ID: {}", task.local_id));
                }

                // Validate task ID format
                if task.local_id.is_empty() {
                    result.errors.push("Task has empty ID".to_string());
                } else if !task
                    .local_id
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    result.warnings.push(format!(
                        "Task ID '{}' contains non-standard characters",
                        task.local_id
                    ));
                }
            }
        }

        // Architecture compatibility
        let current_arch = std::env::consts::ARCH;
        if !metadata.architecture.is_empty() && metadata.architecture != current_arch {
            result.warnings.push(format!(
                "Package architecture '{}' may not be compatible with current architecture '{}'",
                metadata.architecture, current_arch
            ));
        }
    }
}
