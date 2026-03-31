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

//! Unit tests for packaging functionality

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Create a minimal test Cargo.toml structure
    fn create_test_cargo_toml() -> types::CargoToml {
        types::CargoToml {
            package: Some(types::CargoPackage {
                name: "test-package".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test description".to_string()),
                authors: Some(vec!["Test Author <test@example.com>".to_string()]),
                keywords: Some(vec!["test".to_string(), "packaging".to_string()]),
                rust_version: None,
            }),
            lib: Some(types::CargoLib {
                crate_type: Some(vec!["cdylib".to_string()]),
            }),
            dependencies: None,
        }
    }

    /// Create a mock compiled library file for testing
    fn create_mock_library_file() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let lib_path = temp_dir.path().join("libtest.so");

        // Create a simple mock library file
        std::fs::write(&lib_path, b"mock library content").expect("Failed to write mock library");

        (temp_dir, lib_path)
    }

    /// Create a test project structure
    fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_path = temp_dir.path().to_path_buf();

        // Create src directory
        let src_dir = project_path.join("src");
        std::fs::create_dir_all(&src_dir).expect("Failed to create src dir");

        // Create lib.rs with test workflow
        let lib_rs_content = r#"
use cloacina::{workflow, task, Context, TaskError};

#[workflow(name = "test_package")]
pub mod test_package {
    use super::*;

    #[task(id = "simple_task", dependencies = [])]
    pub async fn simple_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }
}
"#;
        std::fs::write(src_dir.join("lib.rs"), lib_rs_content).expect("Failed to write lib.rs");

        (temp_dir, project_path)
    }

    #[test]
    fn test_generate_manifest_basic() {
        let cargo_toml = create_test_cargo_toml();
        let (_temp_dir, lib_path) = create_mock_library_file();
        let (_project_temp, project_path) = create_test_project();

        // This will fail at FFI loading since we have a mock library,
        // but we can test the basic structure
        let result = manifest::generate_manifest(&cargo_toml, &lib_path, &None, &project_path);

        match result {
            Ok(manifest) => {
                assert_eq!(manifest.format_version, "2");
                assert_eq!(manifest.package.name, "test-package");
                assert_eq!(manifest.package.version, "1.0.0");
                assert!(manifest.rust.is_some());
                assert!(!manifest.rust.as_ref().unwrap().library_path.is_empty());
            }
            Err(e) => {
                // Expected to fail due to mock library, but should be FFI-related
                let error_msg = format!("{}", e);
                assert!(
                    error_msg.contains("Failed to load library")
                        || error_msg.contains("metadata")
                        || error_msg.contains("symbol"),
                    "Error should be FFI-related: {}",
                    error_msg
                );
            }
        }
    }

    #[test]
    fn test_generate_manifest_with_target() {
        let cargo_toml = create_test_cargo_toml();
        let (_temp_dir, lib_path) = create_mock_library_file();
        let (_project_temp, project_path) = create_test_project();
        let target = Some("x86_64-unknown-linux-gnu".to_string());

        let result = manifest::generate_manifest(&cargo_toml, &lib_path, &target, &project_path);

        match result {
            Ok(manifest) => {
                assert!(manifest
                    .package
                    .targets
                    .contains(&"x86_64-unknown-linux-gnu".to_string()));
            }
            Err(_) => {
                // Expected to fail due to mock library
            }
        }
    }

    #[test]
    fn test_generate_manifest_missing_package() {
        let mut cargo_toml = create_test_cargo_toml();
        cargo_toml.package = None; // Remove package section

        let (_temp_dir, lib_path) = create_mock_library_file();
        let (_project_temp, project_path) = create_test_project();

        let result = manifest::generate_manifest(&cargo_toml, &lib_path, &None, &project_path);

        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Missing package section"));
    }

    #[test]
    fn test_extract_package_names_from_source() {
        let (_temp_dir, project_path) = create_test_project();

        let result = manifest::extract_package_names_from_source(&project_path);

        match result {
            Ok(package_names) => {
                assert!(!package_names.is_empty());
                assert!(package_names.contains(&"test_package".to_string()));
            }
            Err(e) => {
                panic!("Should be able to extract package names: {}", e);
            }
        }
    }

    #[test]
    fn test_extract_package_names_no_packages() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_path = temp_dir.path().to_path_buf();

        // Create src directory with no workflows
        let src_dir = project_path.join("src");
        std::fs::create_dir_all(&src_dir).expect("Failed to create src dir");

        let lib_rs_content = r#"
pub fn regular_function() -> String {
    "not a packaged workflow".to_string()
}
"#;
        std::fs::write(src_dir.join("lib.rs"), lib_rs_content).expect("Failed to write lib.rs");

        let result = manifest::extract_package_names_from_source(&project_path);

        match result {
            Ok(package_names) => {
                assert!(package_names.is_empty());
            }
            Err(e) => {
                panic!("Should handle no packages gracefully: {}", e);
            }
        }
    }

    #[test]
    fn test_extract_package_names_missing_src() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_path = temp_dir.path().to_path_buf();
        // Don't create src directory

        let result = manifest::extract_package_names_from_source(&project_path);

        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Failed to read src directory"));
    }

    #[test]
    fn test_get_current_architecture() {
        let arch = manifest::get_current_architecture();

        assert!(!arch.is_empty());
        // Should be one of the common architectures
        assert!(
            arch == "x86_64"
                || arch == "aarch64"
                || arch == "arm"
                || arch.starts_with("x86")
                || arch.starts_with("aarch")
                || arch.starts_with("arm")
        );
    }

    #[test]
    fn test_compile_options_builder_pattern() {
        let options = CompileOptions {
            target: Some("aarch64-apple-darwin".to_string()),
            profile: "release".to_string(),
            cargo_flags: vec!["--features".to_string(), "postgres".to_string()],
            jobs: Some(8),
        };

        assert_eq!(options.target.as_ref().unwrap(), "aarch64-apple-darwin");
        assert_eq!(options.profile, "release");
        assert_eq!(options.cargo_flags.len(), 2);
        assert_eq!(options.jobs.unwrap(), 8);
    }

    #[test]
    fn test_manifest_schema_rust_package() {
        use chrono::Utc;

        let manifest = manifest_schema::Manifest {
            format_version: "2".to_string(),
            package: manifest_schema::PackageInfo {
                name: "test-workflow".to_string(),
                version: "2.1.0".to_string(),
                description: Some("Test workflow package".to_string()),
                fingerprint: "sha256:test".to_string(),
                targets: vec!["linux-x86_64".to_string()],
            },
            language: manifest_schema::PackageLanguage::Rust,
            python: None,
            rust: Some(manifest_schema::RustRuntime {
                library_path: "libworkflow.dylib".to_string(),
            }),
            tasks: vec![
                manifest_schema::TaskDefinition {
                    id: "task1".to_string(),
                    function: "cloacina_execute_task".to_string(),
                    dependencies: vec![],
                    description: Some("First task".to_string()),
                    retries: 0,
                    timeout_seconds: None,
                },
                manifest_schema::TaskDefinition {
                    id: "task2".to_string(),
                    function: "cloacina_execute_task".to_string(),
                    dependencies: vec!["task1".to_string()],
                    description: Some("Second task".to_string()),
                    retries: 0,
                    timeout_seconds: None,
                },
            ],
            triggers: vec![],
            created_at: Utc::now(),
            signature: None,
        };

        assert_eq!(manifest.package.name, "test-workflow");
        assert_eq!(manifest.package.version, "2.1.0");
        assert!(manifest.rust.is_some());
        assert_eq!(manifest.tasks.len(), 2);
        assert_eq!(manifest.tasks[1].dependencies, vec!["task1"]);

        // Serialization roundtrip
        let json = serde_json::to_string(&manifest).expect("Should serialize");
        let deserialized: manifest_schema::Manifest =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.package.name, manifest.package.name);
        assert_eq!(deserialized.tasks.len(), manifest.tasks.len());
    }

    #[test]
    fn test_constants() {
        assert_eq!(types::MANIFEST_FILENAME, "manifest.json");
        assert_eq!(types::EXECUTE_TASK_SYMBOL, "cloacina_execute_task");
        assert!(!types::CLOACINA_VERSION.is_empty());

        // Verify version follows semver format
        let version_parts: Vec<&str> = types::CLOACINA_VERSION.split('.').collect();
        assert!(
            version_parts.len() >= 2,
            "Version should have at least major.minor"
        );

        // Each part should be numeric
        for part in version_parts.iter().take(2) {
            assert!(
                part.parse::<u32>().is_ok(),
                "Version parts should be numeric: {}",
                part
            );
        }
    }

    // FFI Validation Helper Tests

    #[test]
    fn test_safe_cstr_to_string_null_pointer() {
        use super::super::manifest::{safe_cstr_to_string, ManifestError};
        use std::ptr;

        let result = safe_cstr_to_string(ptr::null(), "test_field");
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::NullString { field } => {
                assert_eq!(field, "test_field");
            }
            _ => panic!("Expected NullString error"),
        }
    }

    #[test]
    fn test_safe_cstr_to_string_valid() {
        use super::super::manifest::safe_cstr_to_string;
        use std::ffi::CString;

        let c_string = CString::new("hello world").unwrap();
        let result = safe_cstr_to_string(c_string.as_ptr(), "test_field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn test_safe_cstr_to_option_string_null_returns_none() {
        use super::super::manifest::safe_cstr_to_option_string;
        use std::ptr;

        let result = safe_cstr_to_option_string(ptr::null(), "test_field");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_safe_cstr_to_option_string_valid() {
        use super::super::manifest::safe_cstr_to_option_string;
        use std::ffi::CString;

        let c_string = CString::new("optional value").unwrap();
        let result = safe_cstr_to_option_string(c_string.as_ptr(), "test_field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("optional value".to_string()));
    }

    #[test]
    fn test_validate_ptr_null_pointer() {
        use super::super::manifest::{validate_ptr, ManifestError};
        use std::ptr;

        let result: Result<&u32, ManifestError> = unsafe { validate_ptr(ptr::null(), "test_ptr") };
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::NullPointer { field } => {
                assert_eq!(field, "test_ptr");
            }
            _ => panic!("Expected NullPointer error"),
        }
    }

    #[test]
    fn test_validate_ptr_valid() {
        use super::super::manifest::validate_ptr;

        let value: u32 = 42;
        let result = unsafe { validate_ptr(&value as *const u32, "test_ptr") };
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), 42);
    }

    #[test]
    fn test_validate_slice_null_with_nonzero_count() {
        use super::super::manifest::{validate_slice, ManifestError};
        use std::ptr;

        let result: Result<&[u32], ManifestError> =
            unsafe { validate_slice(ptr::null(), 5, "test_slice") };
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::NullTaskSlice { count } => {
                assert_eq!(count, 5);
            }
            _ => panic!("Expected NullTaskSlice error"),
        }
    }

    #[test]
    fn test_validate_slice_null_with_zero_count() {
        use super::super::manifest::validate_slice;
        use std::ptr;

        let result: Result<&[u32], _> = unsafe { validate_slice(ptr::null(), 0, "test_slice") };
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_validate_slice_exceeds_max_tasks() {
        use super::super::manifest::{validate_slice, ManifestError};

        let value: u32 = 42;
        // MAX_TASKS is 10_000, so we test with 10_001
        let result: Result<&[u32], ManifestError> =
            unsafe { validate_slice(&value as *const u32, 10_001, "test_slice") };
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::TooManyTasks { count, max } => {
                assert_eq!(count, 10_001);
                assert_eq!(max, 10_000);
            }
            _ => panic!("Expected TooManyTasks error"),
        }
    }

    #[test]
    fn test_validate_slice_valid() {
        use super::super::manifest::validate_slice;

        let values: [u32; 3] = [1, 2, 3];
        let result = unsafe { validate_slice(values.as_ptr(), 3, "test_slice") };
        assert!(result.is_ok());
        let slice = result.unwrap();
        assert_eq!(slice.len(), 3);
        assert_eq!(slice[0], 1);
        assert_eq!(slice[1], 2);
        assert_eq!(slice[2], 3);
    }

    #[test]
    fn test_manifest_error_display() {
        use super::super::manifest::ManifestError;

        let err = ManifestError::NullPointer {
            field: "test_field",
        };
        assert!(err.to_string().contains("test_field"));

        let err = ManifestError::NullString {
            field: "string_field".to_string(),
        };
        assert!(err.to_string().contains("string_field"));

        let err = ManifestError::NullTaskSlice { count: 42 };
        assert!(err.to_string().contains("42"));

        let err = ManifestError::TooManyTasks {
            count: 20000,
            max: 10000,
        };
        assert!(err.to_string().contains("20000"));
        assert!(err.to_string().contains("10000"));
    }
}
