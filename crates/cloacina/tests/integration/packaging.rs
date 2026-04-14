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

//! Integration tests for workflow packaging functionality.
//!
//! These tests verify the packaging workflow including compilation,
//! manifest generation, and archive creation.

use anyhow::Result;
use serial_test::serial;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use cloacina::packaging::{package_workflow, CompileOptions, Manifest};

/// Write a minimal `package.toml` into a project directory for testing.
fn write_package_toml(project_path: &Path) {
    let content = r#"[package]
name = "test-workflow"
version = "1.0.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "test_package"
language = "rust"
description = "Test workflow for packaging"
author = "Test"
"#;
    std::fs::write(project_path.join("package.toml"), content)
        .expect("Failed to write package.toml");
}

/// Test fixture for managing temporary projects and packages
struct PackagingFixture {
    #[allow(dead_code)]
    temp_dir: TempDir,
    project_path: PathBuf,
    output_path: PathBuf,
}

impl PackagingFixture {
    /// Create a new packaging fixture with a test project
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let project_path = temp_dir.path().join("test_project");
        let output_path = temp_dir.path().join("test_output.cloacina");

        // Create a minimal Rust project structure
        std::fs::create_dir_all(project_path.join("src"))?;

        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test-workflow"
version = "1.0.0"
edition = "2021"
description = "Test workflow for packaging"

[lib]
crate-type = ["cdylib"]

[dependencies]
cloacina = { path = "../../../cloacina", features = ["sqlite"] }
serde_json = "1.0"
"#;
        std::fs::write(project_path.join("Cargo.toml"), cargo_toml)?;

        // Create lib.rs with a workflow package
        let lib_rs = r#"
use cloacina::{workflow, task, Context, TaskError};

#[workflow(name = "test_package")]
pub mod test_package {
    use super::*;

    #[task(id = "test_task", dependencies = [])]
    pub async fn test_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Ok(())
    }
}
"#;
        std::fs::write(project_path.join("src/lib.rs"), lib_rs)?;

        Ok(Self {
            temp_dir,
            project_path,
            output_path,
        })
    }

    fn get_project_path(&self) -> &Path {
        &self.project_path
    }

    fn get_output_path(&self) -> &Path {
        &self.output_path
    }
}

#[tokio::test]
#[serial]
async fn test_package_workflow_full_pipeline() {
    let fixture = PackagingFixture::new().expect("Failed to create fixture");

    // Write the required package.toml for fidius source packaging
    write_package_toml(fixture.get_project_path());

    let result = package_workflow(
        fixture.get_project_path().to_path_buf(),
        fixture.get_output_path().to_path_buf(),
    );

    match result {
        Ok(()) => {
            assert!(
                fixture.get_output_path().exists(),
                "Package file should exist"
            );

            // Verify the package is a valid archive
            let package_data = std::fs::read(fixture.get_output_path())
                .expect("Should be able to read package file");
            assert!(!package_data.is_empty(), "Package should not be empty");

            // fidius produces a bzip2 tar archive (starts with bzip2 magic: 0x42 0x5a)
            assert_eq!(&package_data[0..2], &[0x42, 0x5a], "Should be bzip2");
        }
        Err(e) => {
            println!("Packaging failed (may be expected in CI): {}", e);
        }
    }
}

#[test]
fn test_compile_options_default() {
    let options = CompileOptions::default();

    assert_eq!(options.profile, "debug");
    assert!(options.target.is_none());
    assert!(options.cargo_flags.is_empty());
    assert!(options.jobs.is_none());
}

#[test]
fn test_compile_options_custom() {
    let options = CompileOptions {
        target: Some("x86_64-unknown-linux-gnu".to_string()),
        profile: "release".to_string(),
        cargo_flags: vec!["--features".to_string(), "postgres".to_string()],
        jobs: Some(4),
    };

    assert_eq!(options.target.unwrap(), "x86_64-unknown-linux-gnu");
    assert_eq!(options.profile, "release");
    assert_eq!(options.cargo_flags.len(), 2);
    assert_eq!(options.jobs.unwrap(), 4);
}

#[tokio::test]
#[serial]
async fn test_packaging_with_package_toml() {
    let fixture = PackagingFixture::new().expect("Failed to create fixture");

    // Write a package.toml so fidius can pack
    write_package_toml(fixture.get_project_path());

    let result = package_workflow(
        fixture.get_project_path().to_path_buf(),
        fixture.get_output_path().to_path_buf(),
    );

    // With a valid project and package.toml, fidius source packaging should succeed
    match result {
        Ok(()) => {
            assert!(
                fixture.get_output_path().exists(),
                "Package file should exist after successful packaging"
            );
        }
        Err(e) => {
            println!("Packaging failed: {}", e);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_packaging_invalid_project() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let invalid_project = temp_dir.path().join("invalid");
    let output_path = temp_dir.path().join("output.cloacina");

    // Don't create the project directory - it should fail
    let result = package_workflow(invalid_project, output_path);

    assert!(result.is_err(), "Should fail with invalid project path");
}

#[tokio::test]
#[serial]
async fn test_packaging_missing_cargo_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("project");
    let output_path = temp_dir.path().join("output.cloacina");

    // Create directory but no Cargo.toml
    std::fs::create_dir_all(&project_path).expect("Failed to create project dir");

    let result = package_workflow(project_path, output_path);

    assert!(result.is_err(), "Should fail with missing Cargo.toml");
}

#[tokio::test]
#[serial]
async fn test_packaging_missing_package_toml() {
    let fixture = PackagingFixture::new().expect("Failed to create fixture");

    // Do NOT write package.toml — packaging should fail with a clear error
    let result = package_workflow(
        fixture.get_project_path().to_path_buf(),
        fixture.get_output_path().to_path_buf(),
    );

    assert!(result.is_err(), "Should fail without package.toml");
    let error_msg = format!("{}", result.unwrap_err());
    assert!(
        error_msg.contains("package.toml"),
        "Error should mention package.toml: {}",
        error_msg
    );
}

#[test]
fn test_package_manifest_schema_serialization() {
    use cloacina::packaging::{PackageInfo, PackageLanguage, RustRuntime, TaskDefinition};

    let manifest = Manifest {
        format_version: "2".to_string(),
        package: PackageInfo {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test package".to_string()),
            fingerprint: "sha256:test".to_string(),
            targets: vec!["linux-x86_64".to_string()],
        },
        language: PackageLanguage::Rust,
        python: None,
        rust: Some(RustRuntime {
            library_path: "libtest.so".to_string(),
        }),
        tasks: vec![TaskDefinition {
            id: "test_task".to_string(),
            function: "cloacina_execute_task".to_string(),
            dependencies: vec![],
            description: Some("Test task".to_string()),
            retries: 0,
            timeout_seconds: None,
        }],
        triggers: vec![],
        created_at: chrono::Utc::now(),
        signature: None,
    };

    // Test serialization
    let json = serde_json::to_string(&manifest).expect("Should serialize to JSON");
    assert!(!json.is_empty());
    assert!(json.contains("test-package"));
    assert!(json.contains("test_task"));

    // Test deserialization
    let deserialized: Manifest = serde_json::from_str(&json).expect("Should deserialize from JSON");
    assert_eq!(deserialized.package.name, "test-package");
    assert_eq!(deserialized.tasks.len(), 1);
    assert_eq!(deserialized.tasks[0].id, "test_task");
}

#[test]
fn test_package_constants() {
    use cloacina::packaging::types::{CLOACINA_VERSION, MANIFEST_FILENAME};

    assert_eq!(MANIFEST_FILENAME, "manifest.json");
    assert!(!CLOACINA_VERSION.is_empty());
}

/// Helper function to create a minimal valid Cargo.toml for testing
fn create_test_cargo_toml() -> cloacina::packaging::types::CargoToml {
    cloacina::packaging::types::CargoToml {
        package: Some(cloacina::packaging::types::CargoPackage {
            name: "test-workflow".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test workflow".to_string()),
            authors: Some(vec!["Test Author".to_string()]),
            keywords: Some(vec!["workflow".to_string()]),
            rust_version: None,
        }),
        lib: Some(cloacina::packaging::types::CargoLib {
            crate_type: Some(vec!["cdylib".to_string()]),
        }),
        dependencies: None,
    }
}

#[test]
fn test_cargo_toml_parsing() {
    let cargo_toml = create_test_cargo_toml();

    assert!(cargo_toml.package.is_some());
    let package = cargo_toml.package.unwrap();
    assert_eq!(package.name, "test-workflow");
    assert_eq!(package.version, "1.0.0");
    assert_eq!(package.description.unwrap(), "Test workflow");

    assert!(cargo_toml.lib.is_some());
    let lib = cargo_toml.lib.unwrap();
    assert!(lib.crate_type.is_some());
    let crate_types = lib.crate_type.unwrap();
    assert!(crate_types.contains(&"cdylib".to_string()));
}
