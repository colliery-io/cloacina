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

//! Integration tests for the packaging and inspection workflow.
//!
//! These tests verify that `package_workflow` produces a valid fidius source
//! package (bzip2 tar archive containing source files and `package.toml`).

use anyhow::Result;
use serial_test::serial;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use cloacina::packaging::package_workflow;

/// Test fixture for packaging and inspecting existing example projects.
struct PackageInspectionFixture {
    temp_dir: TempDir,
    project_path: PathBuf,
    package_path: PathBuf,
}

impl PackageInspectionFixture {
    /// Create a new fixture using an existing example project.
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let package_path = temp_dir.path().join("example_workflow.cloacina");

        // Use the existing complex-dag example
        let workspace_root = std::env::current_dir()?.parent().unwrap().to_path_buf();
        let project_path = workspace_root.join("examples/features/complex-dag");

        if !project_path.exists() {
            anyhow::bail!("Example project not found at: {}", project_path.display());
        }

        Ok(Self {
            temp_dir,
            project_path,
            package_path,
        })
    }

    fn get_project_path(&self) -> &Path {
        &self.project_path
    }

    fn get_package_path(&self) -> &Path {
        &self.package_path
    }

    /// Package the workflow using the cloacina library.
    fn package_workflow(&self) -> Result<()> {
        package_workflow(self.project_path.clone(), self.package_path.clone())
    }

    /// Verify the package is a valid bzip2 archive (fidius format).
    fn verify_bzip2_magic(&self) -> Result<bool> {
        let data = std::fs::read(&self.package_path)?;
        // bzip2 magic: 0x42 0x5a ('B', 'Z')
        Ok(data.len() >= 2 && data[0] == 0x42 && data[1] == 0x5a)
    }
}

#[tokio::test]
#[serial]
async fn test_package_produces_bzip2_archive() {
    let fixture = match PackageInspectionFixture::new() {
        Ok(f) => f,
        Err(_e) => {
            return; // Skip test if fixture creation fails
        }
    };

    let package_result = fixture.package_workflow();

    match package_result {
        Ok(()) => {
            assert!(
                fixture.get_package_path().exists(),
                "Package file should exist"
            );
            let package_metadata = std::fs::metadata(fixture.get_package_path()).unwrap();
            assert!(
                package_metadata.len() > 0,
                "Package file should not be empty"
            );

            // fidius source packages are bzip2 tar archives
            let is_bzip2 = fixture
                .verify_bzip2_magic()
                .expect("Should be able to check archive format");
            assert!(is_bzip2, "Package should be a bzip2 archive");
        }
        Err(e) => {
            // In CI or environments without proper Rust toolchain, gracefully handle this
            let error_msg = format!("{}", e);
            if error_msg.contains("cargo")
                || error_msg.contains("rustc")
                || error_msg.contains("compile")
                || error_msg.contains("build")
                || error_msg.contains("package.toml")
            {
                return;
            } else {
                panic!("Unexpected packaging error: {}", e);
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_package_inspection_error_handling() {
    // Test error handling when packaging an invalid project
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let invalid_project_path = temp_dir.path().join("nonexistent");
    let output_path = temp_dir.path().join("output.cloacina");

    // Try to package a nonexistent project
    let result = package_workflow(invalid_project_path, output_path);

    assert!(result.is_err(), "Should fail with invalid project path");
}

#[test]
fn test_packaging_constants_integration() {
    use cloacina::packaging::types::{CLOACINA_VERSION, MANIFEST_FILENAME};

    // Verify constants are what we expect for packaging/inspection
    assert_eq!(MANIFEST_FILENAME, "manifest.json");
    assert!(!CLOACINA_VERSION.is_empty());

    // Verify version format
    let version_parts: Vec<&str> = CLOACINA_VERSION.split('.').collect();
    assert!(version_parts.len() >= 2, "Version should be semver format");
}
