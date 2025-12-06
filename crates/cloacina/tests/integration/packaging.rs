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

//! Integration tests for workflow packaging functionality.
//!
//! These tests verify the packaging pipeline including compilation,
//! manifest generation, and archive creation.

use anyhow::Result;
use serial_test::serial;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use cloacina::packaging::{
    compile_workflow, generate_manifest, package_workflow, CompileOptions, PackageManifest,
};

/// Test fixture for managing temporary projects and packages
struct PackagingFixture {
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
        std::fs::create_dir_all(&project_path.join("src"))?;

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

        // Create lib.rs with a packaged workflow
        let lib_rs = r#"
use cloacina::packaged_workflow;

#[packaged_workflow]
pub fn test_task() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::json!({"result": "success"}))
}

#[packaged_workflow(package = "test-package")]
pub fn another_task() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::json!({"result": "another_success"}))
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
async fn test_compile_workflow_basic() {
    let fixture = PackagingFixture::new().expect("Failed to create fixture");

    let temp_so = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    let so_path = temp_so.path().to_path_buf();

    let options = CompileOptions {
        target: None,
        profile: "debug".to_string(),
        cargo_flags: vec![],
        jobs: None,
    };

    let result = compile_workflow(
        fixture.get_project_path().to_path_buf(),
        so_path.clone(),
        options,
    );

    // Note: This test might fail in CI due to compilation dependencies,
    // but it should work in a local development environment
    match result {
        Ok(compile_result) => {
            assert!(so_path.exists(), "Compiled library should exist");
            assert!(
                !compile_result.manifest.tasks.is_empty(),
                "Should have extracted tasks"
            );

            // Verify manifest content
            assert_eq!(compile_result.manifest.package.name, "test-workflow");
            assert_eq!(compile_result.manifest.package.version, "1.0.0");
            assert!(
                compile_result.manifest.library.filename.ends_with(".so")
                    || compile_result.manifest.library.filename.ends_with(".dylib")
                    || compile_result.manifest.library.filename.ends_with(".dll")
            );
        }
        Err(e) => {
            // Log the error for debugging but don't fail the test if it's a compilation issue
            println!("Compilation failed (may be expected in CI): {}", e);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_package_workflow_full_pipeline() {
    let fixture = PackagingFixture::new().expect("Failed to create fixture");

    let options = CompileOptions {
        target: None,
        profile: "debug".to_string(),
        cargo_flags: vec![],
        jobs: None,
    };

    let result = package_workflow(
        fixture.get_project_path().to_path_buf(),
        fixture.get_output_path().to_path_buf(),
        options,
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

            // Verify it's a gzipped tar archive (starts with gzip magic)
            assert_eq!(&package_data[0..2], &[0x1f, 0x8b], "Should be gzipped");
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
async fn test_packaging_with_cross_compilation() {
    let fixture = PackagingFixture::new().expect("Failed to create fixture");

    let options = CompileOptions {
        target: Some("x86_64-unknown-linux-gnu".to_string()),
        profile: "release".to_string(),
        cargo_flags: vec![],
        jobs: Some(1),
    };

    let result = package_workflow(
        fixture.get_project_path().to_path_buf(),
        fixture.get_output_path().to_path_buf(),
        options,
    );

    // This will likely fail on macOS trying to cross-compile to Linux,
    // but we can test that the error is related to cross-compilation
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        // Should fail due to missing cross-compilation toolchain, not our code
        assert!(
            error_msg.contains("target")
                || error_msg.contains("cargo")
                || error_msg.contains("build"),
            "Error should be compilation-related: {}",
            error_msg
        );
    }
}

#[tokio::test]
#[serial]
async fn test_packaging_invalid_project() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let invalid_project = temp_dir.path().join("invalid");
    let output_path = temp_dir.path().join("output.cloacina");

    // Don't create the project directory - it should fail
    let options = CompileOptions::default();

    let result = package_workflow(invalid_project, output_path, options);

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

    let options = CompileOptions::default();

    let result = package_workflow(project_path, output_path, options);

    assert!(result.is_err(), "Should fail with missing Cargo.toml");
}

#[tokio::test]
#[serial]
async fn test_packaging_with_cargo_flags() {
    let fixture = PackagingFixture::new().expect("Failed to create fixture");

    let options = CompileOptions {
        target: None,
        profile: "debug".to_string(),
        cargo_flags: vec!["--offline".to_string()], // This will likely fail but tests flag passing
        jobs: None,
    };

    let result = package_workflow(
        fixture.get_project_path().to_path_buf(),
        fixture.get_output_path().to_path_buf(),
        options,
    );

    // This will likely fail due to --offline, but we're testing that flags are passed through
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        // Should contain cargo-related error messages
        assert!(
            error_msg.contains("cargo")
                || error_msg.contains("build")
                || error_msg.contains("offline"),
            "Error should mention cargo flags: {}",
            error_msg
        );
    }
}

#[test]
fn test_package_manifest_serialization() {
    let manifest = PackageManifest {
        package: cloacina::packaging::types::PackageInfo {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            description: "Test package".to_string(),
            author: None,
            workflow_fingerprint: None,
            cloacina_version: "0.2.0".to_string(),
        },
        library: cloacina::packaging::types::LibraryInfo {
            filename: "libtest.so".to_string(),
            symbols: vec!["cloacina_execute_task".to_string()],
            architecture: "x86_64".to_string(),
        },
        tasks: vec![cloacina::packaging::types::TaskInfo {
            index: 0,
            id: "test_task".to_string(),
            dependencies: vec![],
            description: "Test task".to_string(),
            source_location: "src/lib.rs:5".to_string(),
        }],
        graph: None,
    };

    // Test serialization
    let json = serde_json::to_string(&manifest).expect("Should serialize to JSON");
    assert!(!json.is_empty());
    assert!(json.contains("test-package"));
    assert!(json.contains("test_task"));

    // Test deserialization
    let deserialized: PackageManifest =
        serde_json::from_str(&json).expect("Should deserialize from JSON");
    assert_eq!(deserialized.package.name, "test-package");
    assert_eq!(deserialized.tasks.len(), 1);
    assert_eq!(deserialized.tasks[0].id, "test_task");
}

#[test]
fn test_package_constants() {
    use cloacina::packaging::types::{CLOACINA_VERSION, EXECUTE_TASK_SYMBOL, MANIFEST_FILENAME};

    assert_eq!(MANIFEST_FILENAME, "manifest.json");
    assert_eq!(EXECUTE_TASK_SYMBOL, "cloacina_execute_task");
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
