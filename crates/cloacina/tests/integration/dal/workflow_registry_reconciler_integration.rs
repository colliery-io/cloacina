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

//! Integration tests for the end-to-end workflow: register package via DAL → load via reconciler

use crate::fixtures::get_or_init_fixture;
use cloacina::packaging::{create_package_archive, generate_manifest, CargoToml, CompileResult};
use cloacina::registry::traits::WorkflowRegistry;
use serial_test::serial;
use std::sync::OnceLock;
use tempfile::TempDir;
use uuid::Uuid;

/// Cached test package data.
///
/// This is created from pre-built .so files (built by angreal before tests).
/// No subprocess is spawned - only the .so is loaded to extract metadata.
static TEST_PACKAGE: OnceLock<Vec<u8>> = OnceLock::new();

/// Get the cached test package, creating it from pre-built .so if necessary.
///
/// IMPORTANT: The .so file must be pre-built before running tests.
/// Run `angreal cloacina integration` which pre-builds the packages,
/// or manually run `cargo build --release -p simple-packaged-demo`.
fn get_test_package() -> Vec<u8> {
    TEST_PACKAGE
        .get_or_init(|| create_package_from_prebuilt_so())
        .clone()
}

/// Create a package from pre-built .so file without spawning cargo.
///
/// This function:
/// 1. Finds the pre-built .so file in the example's target/release directory
/// 2. Generates the manifest by loading the .so (no subprocess)
/// 3. Creates the .cloacina archive
fn create_package_from_prebuilt_so() -> Vec<u8> {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let unique_id = Uuid::new_v4().to_string();

    // Find the workspace root (crates/cloacina -> crates -> project root)
    let cargo_manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_path = std::path::PathBuf::from(&cargo_manifest_dir);
    let workspace_root = workspace_path
        .parent()
        .expect("Should have parent directory (crates)")
        .parent()
        .expect("Should have parent directory (project root)");
    let project_path = workspace_root.join("examples/features/simple-packaged");

    if !project_path.exists() {
        panic!("Project path does not exist: {}", project_path.display());
    }

    // Find pre-built .so file
    let so_path = find_prebuilt_library(&project_path).expect(
        "Pre-built .so not found. Run `cargo build --release -p simple-packaged-demo` first.",
    );

    // Read and parse Cargo.toml
    let cargo_toml_path = project_path.join("Cargo.toml");
    let cargo_toml_content =
        std::fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");
    let cargo_toml: CargoToml =
        toml::from_str(&cargo_toml_content).expect("Failed to parse Cargo.toml");

    // Generate manifest by loading the .so (no subprocess spawned)
    let manifest = generate_manifest(&cargo_toml, &so_path, &None, &project_path)
        .expect("Failed to generate manifest");

    // Create temp file for .so copy (archive needs the path)
    let temp_so_path = temp_dir.path().join(so_path.file_name().unwrap());
    std::fs::copy(&so_path, &temp_so_path).expect("Failed to copy .so file");

    let compile_result = CompileResult {
        so_path: temp_so_path,
        manifest,
    };

    // Create package archive
    let package_path = temp_dir
        .path()
        .join(format!("test_package_{}.cloacina", unique_id));
    create_package_archive(&compile_result, &package_path)
        .expect("Failed to create package archive");

    // Read and return the package data
    std::fs::read(&package_path).expect("Failed to read package file")
}

/// Find the pre-built library in the project's target directory.
fn find_prebuilt_library(project_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let target_dir = project_path.join("target/release");

    if !target_dir.exists() {
        return None;
    }

    // Look for .so (Linux) or .dylib (macOS)
    let extensions = if cfg!(target_os = "macos") {
        vec!["dylib"]
    } else {
        vec!["so"]
    };

    for ext in extensions {
        for entry in std::fs::read_dir(&target_dir).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some(ext) {
                // Skip files that look like dependency artifacts
                let filename = path.file_name()?.to_str()?;
                if !filename.contains("-") || filename.starts_with("lib") {
                    return Some(path);
                }
            }
        }
    }

    None
}

#[tokio::test]
#[serial]
async fn test_dal_register_then_reconciler_load() {
    // IMPORTANT: Get test package BEFORE initializing database to avoid SIGSEGV
    println!("Step 1: Create test package");
    let package_data = get_test_package();
    println!("Package created: {} bytes", package_data.len());

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    println!("🔧 Step 2: Register package using DAL system");
    let dal = fixture.get_dal();
    let storage = fixture.create_storage();
    let mut registry_dal = dal.workflow_registry(storage);

    let package_id = registry_dal
        .register_workflow_package(package_data.clone())
        .await
        .expect("Failed to register workflow package");

    println!("✅ Package registered with DAL ID: {}", package_id);

    println!("🔧 Step 3: Verify package is listed");
    let packages = registry_dal
        .list_packages()
        .await
        .expect("Failed to list packages");
    assert!(!packages.is_empty(), "Should have at least one package");

    let our_package = packages.iter().find(|p| p.package_name == "simple_demo");
    assert!(our_package.is_some(), "Should find our registered package");
    let our_package = our_package.unwrap();

    println!(
        "✅ Package found in list: {} v{}",
        our_package.package_name, our_package.version
    );

    println!("🔧 Step 4: Try to retrieve package by ID (DAL method)");
    let retrieved_by_id = registry_dal
        .get_workflow_package_by_id(package_id)
        .await
        .expect("Failed to get workflow package by ID");

    assert!(
        retrieved_by_id.is_some(),
        "Should be able to retrieve package by ID"
    );
    let (metadata, binary_data) = retrieved_by_id.unwrap();
    assert_eq!(metadata.package_name, "simple_demo");
    assert_eq!(binary_data, package_data);

    println!("✅ Package retrieved by ID successfully");

    println!("🔧 Step 5: Try to retrieve package by name/version (DAL method)");
    let retrieved_by_name = registry_dal
        .get_workflow_package_by_name(&our_package.package_name, &our_package.version)
        .await
        .expect("Failed to get workflow package by name");

    assert!(
        retrieved_by_name.is_some(),
        "Should be able to retrieve package by name"
    );
    let (metadata2, binary_data2) = retrieved_by_name.unwrap();
    assert_eq!(metadata2.package_name, metadata.package_name);
    assert_eq!(metadata2.version, metadata.version);
    assert_eq!(binary_data2, binary_data);

    println!("✅ Package retrieved by name/version successfully");

    println!("🔧 Step 6: Try to load package via WorkflowRegistry trait (reconciler method)");
    let loaded_workflow = registry_dal
        .get_workflow(&our_package.package_name, &our_package.version)
        .await
        .expect("Failed to get workflow via trait method");

    assert!(
        loaded_workflow.is_some(),
        "Should be able to load workflow via trait"
    );
    let loaded_workflow = loaded_workflow.unwrap();
    assert_eq!(loaded_workflow.package_data, package_data);

    println!("✅ Package loaded via WorkflowRegistry trait successfully");

    println!("🎉 All steps completed successfully - DAL registration → reconciler loading works!");
}

#[tokio::test]
#[serial]
async fn test_dal_register_then_get_workflow_package_by_id_failure_case() {
    // IMPORTANT: Get test package BEFORE initializing database to avoid SIGSEGV
    println!("Step 1: Create test package");
    let package_data = get_test_package();

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    println!("🔧 Step 2: Register package using DAL system");
    let dal = fixture.get_dal();
    let storage = fixture.create_storage();
    let mut registry_dal = dal.workflow_registry(storage);

    let package_id = registry_dal
        .register_workflow_package(package_data)
        .await
        .expect("Failed to register workflow package");

    println!("✅ Package registered with DAL ID: {}", package_id);

    println!("🔧 Step 3: Directly test get_workflow_package_by_id to reproduce the failure");
    match registry_dal.get_workflow_package_by_id(package_id).await {
        Ok(Some((metadata, binary_data))) => {
            println!(
                "✅ SUCCESS: Retrieved package {} v{} with {} bytes of binary data",
                metadata.package_name,
                metadata.version,
                binary_data.len()
            );
        }
        Ok(None) => {
            panic!("❌ UNEXPECTED: Package not found by ID");
        }
        Err(e) => {
            println!("❌ FAILURE: {}", e);
            panic!("Failed to retrieve package by ID: {}", e);
        }
    }
}
