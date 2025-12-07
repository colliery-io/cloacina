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

//! Symbol validation for dynamic libraries.

use libloading::Library;
use std::path::Path;

use super::types::ValidationResult;
use super::PackageValidator;

impl PackageValidator {
    /// Validate required symbols are present.
    pub(super) async fn validate_symbols(
        &self,
        package_path: &Path,
        result: &mut ValidationResult,
    ) {
        match unsafe { Library::new(package_path) } {
            Ok(lib) => {
                for symbol in &self.required_symbols {
                    match unsafe { lib.get::<unsafe extern "C" fn()>(symbol.as_bytes()) } {
                        Ok(_) => {
                            result.compatibility.required_symbols.push(symbol.clone());
                        }
                        Err(_) => {
                            result.compatibility.missing_symbols.push(symbol.clone());
                            result
                                .errors
                                .push(format!("Required symbol '{}' not found", symbol));
                        }
                    }
                }

                // Check for common optional symbols
                let optional_symbols = [
                    "cloacina_get_version",
                    "cloacina_get_author",
                    "cloacina_get_description",
                ];

                for symbol in &optional_symbols {
                    if unsafe { lib.get::<unsafe extern "C" fn()>(symbol.as_bytes()).is_ok() } {
                        result
                            .compatibility
                            .required_symbols
                            .push(symbol.to_string());
                    }
                }
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Cannot load library for symbol validation: {}", e));
            }
        }
    }
}
