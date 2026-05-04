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
#[allow(clippy::module_inception)]
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
