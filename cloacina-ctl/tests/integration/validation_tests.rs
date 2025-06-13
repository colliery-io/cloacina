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

use cloacina_ctl::validation::*;
use std::fs;
use tempfile::TempDir;

fn create_test_project(cargo_toml_content: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create Cargo.toml
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml_content)
        .expect("Failed to write Cargo.toml");

    // Create src directory
    fs::create_dir(temp_dir.path().join("src")).expect("Failed to create src directory");

    // Create lib.rs
    fs::write(temp_dir.path().join("src/lib.rs"), "// Test library")
        .expect("Failed to write lib.rs");

    temp_dir
}

#[test]
fn test_valid_cargo_toml_with_cdylib() {
    let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
"#;

    let temp_dir = create_test_project(cargo_toml);
    let result = validate_cargo_toml(&temp_dir.path().to_path_buf());

    assert!(
        result.is_ok(),
        "Should accept valid cdylib configuration: {:?}",
        result.err()
    );
}

#[test]
fn test_valid_cargo_toml_with_multiple_crate_types() {
    let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["lib", "cdylib"]
"#;

    let temp_dir = create_test_project(cargo_toml);
    let result = validate_cargo_toml(&temp_dir.path().to_path_buf());

    assert!(
        result.is_ok(),
        "Should accept cdylib among multiple crate types"
    );
}

#[test]
fn test_missing_lib_section() {
    let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"
"#;

    let temp_dir = create_test_project(cargo_toml);
    let result = validate_cargo_toml(&temp_dir.path().to_path_buf());

    assert!(result.is_err(), "Should reject missing [lib] section");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("[lib]"),
        "Error should mention missing [lib] section"
    );
}

#[test]
fn test_missing_crate_type() {
    let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
name = "test_workflow"
"#;

    let temp_dir = create_test_project(cargo_toml);
    let result = validate_cargo_toml(&temp_dir.path().to_path_buf());

    assert!(result.is_err(), "Should reject missing crate-type");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("crate-type"),
        "Error should mention missing crate-type"
    );
}

#[test]
fn test_wrong_crate_type() {
    let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["lib"]
"#;

    let temp_dir = create_test_project(cargo_toml);
    let result = validate_cargo_toml(&temp_dir.path().to_path_buf());

    assert!(result.is_err(), "Should reject non-cdylib crate-type");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("cdylib"),
        "Error should mention required cdylib"
    );
}

#[test]
fn test_missing_package_section() {
    let cargo_toml = r#"
[lib]
crate-type = ["cdylib"]
"#;

    let temp_dir = create_test_project(cargo_toml);
    let result = validate_cargo_toml(&temp_dir.path().to_path_buf());

    assert!(result.is_err(), "Should reject missing [package] section");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("package"),
        "Error should mention missing [package] section"
    );
}

#[test]
fn test_rust_crate_structure_validation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Test missing directory
    let nonexistent = temp_dir.path().join("nonexistent");
    let result = validate_rust_crate_structure(&nonexistent);
    assert!(result.is_err(), "Should reject nonexistent directory");

    // Test missing Cargo.toml
    let empty_dir = TempDir::new().expect("Failed to create temp directory");
    let result = validate_rust_crate_structure(&empty_dir.path().to_path_buf());
    assert!(
        result.is_err(),
        "Should reject directory without Cargo.toml"
    );

    // Test valid structure (just needs Cargo.toml)
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"",
    )
    .expect("Failed to write Cargo.toml");
    let result = validate_rust_crate_structure(&temp_dir.path().to_path_buf());
    assert!(result.is_ok(), "Should accept valid crate with Cargo.toml");
}

#[test]
fn test_cloacina_compatibility_valid() {
    let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cloacina = "0.2.0-alpha.5"
cloacina-macros = "0.2.0-alpha.5"
"#;

    let temp_dir = create_test_project(cargo_toml);
    let cargo_toml = validate_cargo_toml(&temp_dir.path().to_path_buf()).unwrap();
    let result = validate_cloacina_compatibility(&cargo_toml);

    assert!(
        result.is_ok(),
        "Should accept matching cloacina version: {:?}",
        result.err()
    );
}

#[test]
fn test_cloacina_compatibility_missing() {
    let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = "1.0"
"#;

    let temp_dir = create_test_project(cargo_toml);
    let cargo_toml = validate_cargo_toml(&temp_dir.path().to_path_buf()).unwrap();
    let result = validate_cloacina_compatibility(&cargo_toml);

    assert!(result.is_err(), "Should reject missing cloacina dependency");
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Missing 'cloacina' dependency"));
}

#[test]
fn test_packaged_workflow_presence() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"",
    )
    .expect("Failed to write Cargo.toml");

    // Create src directory
    fs::create_dir(temp_dir.path().join("src")).expect("Failed to create src directory");

    // Create lib.rs with packaged_workflow macro
    fs::write(
        temp_dir.path().join("src/lib.rs"),
        "#[packaged_workflow]\nmod my_workflow {}",
    )
    .expect("Failed to write lib.rs");

    let result = validate_packaged_workflow_presence(&temp_dir.path().to_path_buf());
    assert!(result.is_ok(), "Should find packaged_workflow macro");
}

#[test]
fn test_packaged_workflow_missing() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"",
    )
    .expect("Failed to write Cargo.toml");

    // Create src directory
    fs::create_dir(temp_dir.path().join("src")).expect("Failed to create src directory");

    // Create lib.rs without packaged_workflow macro
    fs::write(temp_dir.path().join("src/lib.rs"), "// No macro here")
        .expect("Failed to write lib.rs");

    let result = validate_packaged_workflow_presence(&temp_dir.path().to_path_buf());
    assert!(
        result.is_err(),
        "Should reject missing packaged_workflow macro"
    );
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("#[packaged_workflow]"));
}
