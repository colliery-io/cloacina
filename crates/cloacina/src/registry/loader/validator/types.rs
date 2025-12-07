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

//! Types for package validation results and assessments.

/// Package validation results
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the package passed all validations
    pub is_valid: bool,
    /// List of validation errors (if any)
    pub errors: Vec<String>,
    /// List of validation warnings (non-fatal issues)
    pub warnings: Vec<String>,
    /// Security assessment
    pub security_level: SecurityLevel,
    /// Compatibility assessment
    pub compatibility: CompatibilityInfo,
}

/// Security assessment levels for packages
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    /// Package appears safe for production use
    Safe,
    /// Package has minor security concerns but is likely safe
    Warning,
    /// Package has significant security risks
    Dangerous,
    /// Package cannot be assessed (insufficient information)
    Unknown,
}

/// Compatibility information for packages
#[derive(Debug, Clone)]
pub struct CompatibilityInfo {
    /// Target architecture of the package
    pub architecture: String,
    /// Required symbols present
    pub required_symbols: Vec<String>,
    /// Missing required symbols
    pub missing_symbols: Vec<String>,
    /// cloacina version compatibility (if detectable)
    pub cloacina_version: Option<String>,
}
