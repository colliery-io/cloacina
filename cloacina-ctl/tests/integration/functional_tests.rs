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

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn create_test_workflow_project(temp_dir: &TempDir) -> PathBuf {
    let project_path = temp_dir.path().join("test_workflow");
    fs::create_dir_all(&project_path).expect("Failed to create project directory");

    // Get the workspace root directory
    let workspace_root = std::env::current_dir()
        .expect("Failed to get current directory")
        .canonicalize()
        .expect("Failed to canonicalize path");

    eprintln!("Current directory: {:?}", workspace_root);

    // The tests are run from cloacina-ctl subdirectory, so we need to go up one level
    let workspace_root = if workspace_root.ends_with("cloacina-ctl") {
        workspace_root.parent().unwrap().to_path_buf()
    } else {
        workspace_root
    };

    // Create Cargo.toml with absolute paths
    let cargo_toml = format!(
        r#"
[package]
name = "test_workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cloacina = {{ path = "{}/cloacina", default-features = false, features = ["macros", "sqlite"] }}
cloacina-macros = {{ path = "{}/cloacina-macros" }}
serde_json = "1.0"
tokio = {{ version = "1", features = ["rt", "macros"] }}
chrono = "0.4"
async-trait = "0.1"
ctor = "0.2"
"#,
        workspace_root.display(),
        workspace_root.display()
    );
    fs::write(project_path.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Create src directory
    let src_dir = project_path.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src directory");

    // Create lib.rs with a simple workflow
    let lib_rs = r#"
use cloacina::{Context, TaskError};
use cloacina_macros::{packaged_workflow, task};

#[packaged_workflow(
    package = "test_workflow",
    version = "0.1.0",
    description = "Test workflow for functional tests"
)]
pub mod test_workflow {
    use super::*;

    #[task(
        id = "first_task",
        dependencies = []
    )]
    pub async fn first_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("Executing first task");
        Ok(())
    }

    #[task(
        id = "second_task",
        dependencies = ["first_task"]
    )]
    pub async fn second_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("Executing second task");
        Ok(())
    }
}
"#;
    fs::write(src_dir.join("lib.rs"), lib_rs).expect("Failed to write lib.rs");

    project_path
}

#[test]
fn test_compile_command_functional() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = create_test_workflow_project(&temp_dir);
    let output_path = temp_dir.path().join("test_workflow.so");

    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--bin")
        .arg("cloacina-ctl")
        .arg("--")
        .arg("compile")
        .arg(&project_path)
        .arg("--output")
        .arg(&output_path);

    let output = cmd
        .output()
        .expect("Failed to execute cloacina-ctl compile");

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("cloacina-ctl compile command failed");
    }

    assert!(output_path.exists(), "Compiled .so file should exist");

    let metadata = fs::metadata(&output_path).expect("Failed to get .so metadata");
    assert!(metadata.len() > 0, "Compiled .so file should not be empty");
}

#[test]
fn test_package_command_functional() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = create_test_workflow_project(&temp_dir);
    let package_path = temp_dir.path().join("test_workflow.cloacina");

    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--bin")
        .arg("cloacina-ctl")
        .arg("--")
        .arg("package")
        .arg(&project_path)
        .arg("--output")
        .arg(&package_path);

    let output = cmd
        .output()
        .expect("Failed to execute cloacina-ctl package");

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("cloacina-ctl package command failed");
    }

    assert!(package_path.exists(), "Package file should exist");

    let metadata = fs::metadata(&package_path).expect("Failed to get package metadata");
    assert!(metadata.len() > 0, "Package file should not be empty");
}

#[test]
fn test_inspect_command_functional() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = create_test_workflow_project(&temp_dir);
    let package_path = temp_dir.path().join("test_workflow.cloacina");

    // First, create a package
    let mut package_cmd = Command::new("cargo");
    package_cmd
        .arg("run")
        .arg("--bin")
        .arg("cloacina-ctl")
        .arg("--")
        .arg("package")
        .arg(&project_path)
        .arg("--output")
        .arg(&package_path);

    let package_output = package_cmd
        .output()
        .expect("Failed to execute cloacina-ctl package");
    if !package_output.status.success() {
        eprintln!("Package creation failed");
        eprintln!(
            "STDOUT: {}",
            String::from_utf8_lossy(&package_output.stdout)
        );
        eprintln!(
            "STDERR: {}",
            String::from_utf8_lossy(&package_output.stderr)
        );
        panic!("Failed to create package for inspect test");
    }

    // Then inspect it
    let mut inspect_cmd = Command::new("cargo");
    inspect_cmd
        .arg("run")
        .arg("--bin")
        .arg("cloacina-ctl")
        .arg("--")
        .arg("inspect")
        .arg(&package_path)
        .arg("--format")
        .arg("json");

    let inspect_output = inspect_cmd
        .output()
        .expect("Failed to execute cloacina-ctl inspect");

    if !inspect_output.status.success() {
        eprintln!(
            "STDOUT: {}",
            String::from_utf8_lossy(&inspect_output.stdout)
        );
        eprintln!(
            "STDERR: {}",
            String::from_utf8_lossy(&inspect_output.stderr)
        );
        panic!("cloacina-ctl inspect command failed");
    }

    let stdout = String::from_utf8_lossy(&inspect_output.stdout);

    // Debug: print the actual output to understand what we're getting
    if !stdout.trim().starts_with('{') && !stdout.trim().starts_with('[') {
        eprintln!("DEBUG - Inspect stdout: '{}'", stdout);
        eprintln!(
            "DEBUG - Inspect stderr: '{}'",
            String::from_utf8_lossy(&inspect_output.stderr)
        );
    }

    assert!(
        stdout.contains("test_workflow"),
        "Output should contain workflow name"
    );
    assert!(
        stdout.contains("first_task"),
        "Output should contain first task"
    );
    assert!(
        stdout.contains("second_task"),
        "Output should contain second task"
    );

    // Try to extract JSON from the output (might have logging prefixes)
    let json_part = if let Some(start) = stdout.find('{') {
        &stdout[start..]
    } else if let Some(start) = stdout.find('[') {
        &stdout[start..]
    } else {
        stdout.trim()
    };

    // Verify it's valid JSON
    let _: serde_json::Value =
        serde_json::from_str(json_part).expect("Inspect output should be valid JSON");
}

#[test]
fn test_debug_list_command_functional() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = create_test_workflow_project(&temp_dir);
    let package_path = temp_dir.path().join("test_workflow.cloacina");

    // First, create a package
    let mut package_cmd = Command::new("cargo");
    package_cmd
        .arg("run")
        .arg("--bin")
        .arg("cloacina-ctl")
        .arg("--")
        .arg("package")
        .arg(&project_path)
        .arg("--output")
        .arg(&package_path);

    let package_output = package_cmd
        .output()
        .expect("Failed to execute cloacina-ctl package");
    if !package_output.status.success() {
        eprintln!("Package creation failed");
        eprintln!(
            "STDOUT: {}",
            String::from_utf8_lossy(&package_output.stdout)
        );
        eprintln!(
            "STDERR: {}",
            String::from_utf8_lossy(&package_output.stderr)
        );
        panic!("Failed to create package for debug list test");
    }

    // Then list tasks
    let mut debug_cmd = Command::new("cargo");
    debug_cmd
        .arg("run")
        .arg("--bin")
        .arg("cloacina-ctl")
        .arg("--")
        .arg("debug")
        .arg(&package_path)
        .arg("list");

    let debug_output = debug_cmd
        .output()
        .expect("Failed to execute cloacina-ctl debug list");

    if !debug_output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&debug_output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&debug_output.stderr));
        panic!("cloacina-ctl debug list command failed");
    }

    let stdout = String::from_utf8_lossy(&debug_output.stdout);
    assert!(
        stdout.contains("first_task"),
        "Debug list should contain first task"
    );
    assert!(
        stdout.contains("second_task"),
        "Debug list should contain second task"
    );
    assert!(
        stdout.contains("0:") || stdout.contains("1:"),
        "Debug list should show task indices"
    );
}

#[test]
fn test_invalid_project_path() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let nonexistent_path = temp_dir.path().join("nonexistent");
    let output_path = temp_dir.path().join("output.so");

    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--bin")
        .arg("cloacina-ctl")
        .arg("--")
        .arg("compile")
        .arg(&nonexistent_path)
        .arg("--output")
        .arg(&output_path);

    let output = cmd
        .output()
        .expect("Failed to execute cloacina-ctl compile");

    assert!(
        !output.status.success(),
        "Command should fail for invalid project path"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("error") || stderr.to_lowercase().contains("failed"),
        "Error output should indicate failure"
    );
}

#[test]
fn test_help_command() {
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--bin")
        .arg("cloacina-ctl")
        .arg("--")
        .arg("--help");

    let output = cmd.output().expect("Failed to execute cloacina-ctl --help");

    assert!(output.status.success(), "Help command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cloacina-ctl"),
        "Help should contain program name"
    );
    assert!(
        stdout.contains("compile"),
        "Help should mention compile command"
    );
    assert!(
        stdout.contains("package"),
        "Help should mention package command"
    );
    assert!(
        stdout.contains("inspect"),
        "Help should mention inspect command"
    );
    assert!(
        stdout.contains("visualize"),
        "Help should mention visualize command"
    );
    assert!(
        stdout.contains("debug"),
        "Help should mention debug command"
    );
}
