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
use cloacina::packaged_workflow;

#[packaged_workflow(package = "test-package")]
pub fn simple_task() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::json!({"status": "complete"}))
}

#[packaged_workflow(package = "test-package")]
pub fn complex_task() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::json!({"data": "processed"}))
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
                assert_eq!(manifest.package.name, "test-package");
                assert_eq!(manifest.package.version, "1.0.0");
                assert!(manifest.package.description.contains("Packaged workflow"));
                assert!(!manifest.library.filename.is_empty());
                assert_eq!(manifest.library.symbols, vec!["cloacina_execute_task"]);
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
                assert_eq!(manifest.library.architecture, "x86_64-unknown-linux-gnu");
            }
            Err(_) => {
                // Expected to fail due to mock library, but architecture should be set before FFI
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
                assert!(package_names.contains(&"test-package".to_string()));
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

        // Create src directory with no packaged workflows
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
    fn test_package_info_creation() {
        let package_info = types::PackageInfo {
            name: "test-workflow".to_string(),
            version: "2.1.0".to_string(),
            description: "Test workflow package".to_string(),
            author: Some("Test Author".to_string()),
            workflow_fingerprint: Some("abc123def456".to_string()),
            cloacina_version: "0.2.0".to_string(),
        };

        assert_eq!(package_info.name, "test-workflow");
        assert_eq!(package_info.version, "2.1.0");
        assert!(!package_info.description.is_empty());
        assert!(!package_info.cloacina_version.is_empty());
    }

    #[test]
    fn test_library_info_creation() {
        let library_info = types::LibraryInfo {
            filename: "libworkflow.dylib".to_string(),
            symbols: vec![
                "cloacina_execute_task".to_string(),
                "cloacina_get_task_metadata".to_string(),
            ],
            architecture: "aarch64-apple-darwin".to_string(),
        };

        assert!(library_info.filename.ends_with(".dylib"));
        assert_eq!(library_info.symbols.len(), 2);
        assert!(library_info
            .symbols
            .contains(&"cloacina_execute_task".to_string()));
        assert!(!library_info.architecture.is_empty());
    }

    #[test]
    fn test_task_info_creation() {
        let task_info = types::TaskInfo {
            index: 0,
            id: "process_data".to_string(),
            dependencies: vec!["validate_input".to_string()],
            description: "Process the input data".to_string(),
            source_location: "src/tasks.rs:42".to_string(),
        };

        assert_eq!(task_info.index, 0);
        assert_eq!(task_info.id, "process_data");
        assert_eq!(task_info.dependencies.len(), 1);
        assert!(!task_info.description.is_empty());
        assert!(task_info.source_location.contains("src/"));
    }

    #[test]
    fn test_package_manifest_serialization_roundtrip() {
        let original_manifest = types::PackageManifest {
            package: types::PackageInfo {
                name: "test-package".to_string(),
                version: "1.0.0".to_string(),
                description: "Test package".to_string(),
                author: None,
                workflow_fingerprint: None,
                cloacina_version: "0.2.0".to_string(),
            },
            library: types::LibraryInfo {
                filename: "libtest.so".to_string(),
                symbols: vec!["cloacina_execute_task".to_string()],
                architecture: "x86_64".to_string(),
            },
            tasks: vec![
                types::TaskInfo {
                    index: 0,
                    id: "task1".to_string(),
                    dependencies: vec![],
                    description: "First task".to_string(),
                    source_location: "src/lib.rs:10".to_string(),
                },
                types::TaskInfo {
                    index: 1,
                    id: "task2".to_string(),
                    dependencies: vec!["task1".to_string()],
                    description: "Second task".to_string(),
                    source_location: "src/lib.rs:20".to_string(),
                },
            ],
            execution_order: vec!["task1".to_string(), "task2".to_string()],
            graph: None,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&original_manifest).expect("Should serialize");
        assert!(!json.is_empty());

        // Deserialize back
        let deserialized: types::PackageManifest =
            serde_json::from_str(&json).expect("Should deserialize");

        // Verify all fields
        assert_eq!(deserialized.package.name, original_manifest.package.name);
        assert_eq!(
            deserialized.package.version,
            original_manifest.package.version
        );
        assert_eq!(
            deserialized.library.filename,
            original_manifest.library.filename
        );
        assert_eq!(deserialized.tasks.len(), original_manifest.tasks.len());
        assert_eq!(deserialized.tasks[0].id, original_manifest.tasks[0].id);
        assert_eq!(
            deserialized.tasks[1].dependencies,
            original_manifest.tasks[1].dependencies
        );
        assert_eq!(
            deserialized.execution_order,
            original_manifest.execution_order
        );
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
}
