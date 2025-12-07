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

//! Package size validation.

use super::types::ValidationResult;
use super::PackageValidator;

impl PackageValidator {
    /// Validate package size constraints.
    pub(super) fn validate_package_size(&self, package_data: &[u8], result: &mut ValidationResult) {
        let size = package_data.len() as u64;

        if size == 0 {
            result.errors.push("Package is empty".to_string());
        } else if size > self.max_package_size {
            result.errors.push(format!(
                "Package size {} bytes exceeds maximum allowed size {} bytes",
                size, self.max_package_size
            ));
        } else if size < 1024 {
            result
                .warnings
                .push("Package is unusually small (< 1KB)".to_string());
        } else if size > 50 * 1024 * 1024 {
            result
                .warnings
                .push("Package is quite large (> 50MB)".to_string());
        }
    }
}
