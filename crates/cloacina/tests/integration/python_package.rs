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

//! Integration tests for Python workflow package loading.
//!
//! These tests build fidius source packages (bzip2 tar + package.toml) in a
//! temp directory, then exercise the server-side python loader to verify the
//! full round-trip: pack → detect → extract → validate.

use tempfile::TempDir;

use cloacina::packaging::{
    Manifest, ManifestValidationError, PackageInfo, PackageLanguage, PythonRuntime, RustRuntime,
    TaskDefinition,
};
use cloacina::registry::loader::{detect_package_kind, extract_python_package, PackageKind};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a fidius source package directory for a Python workflow.
fn create_python_source_dir(
    dir: &std::path::Path,
    name: &str,
    version: &str,
    entry_module: &str,
    include_workflow: bool,
) {
    // package.toml
    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "{version}"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "{name}"
language = "python"
description = "Test Python workflow"
requires_python = ">=3.10"
entry_module = "{entry_module}"
"#
    );
    std::fs::write(dir.join("package.toml"), package_toml).unwrap();

    if include_workflow {
        std::fs::create_dir_all(dir.join("workflow")).unwrap();
        std::fs::write(dir.join("workflow/__init__.py"), "# workflow init\n").unwrap();
        std::fs::write(
            dir.join("workflow/tasks.py"),
            "def process(ctx): return ctx\n",
        )
        .unwrap();
    }

    std::fs::create_dir_all(dir.join("vendor")).unwrap();
}

/// Create a fidius source package directory for a Rust workflow.
fn create_rust_source_dir(dir: &std::path::Path, name: &str, version: &str) {
    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "{version}"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "{name}"
language = "rust"
"#
    );
    std::fs::write(dir.join("package.toml"), package_toml).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "// placeholder\n").unwrap();
}

/// Pack a source directory into a `.cloacina` archive and return the bytes.
fn pack_to_bytes(source_dir: &std::path::Path, output_dir: &std::path::Path) -> Vec<u8> {
    let output = output_dir.join(format!(
        "{}.cloacina",
        source_dir.file_name().unwrap().to_str().unwrap()
    ));
    fidius_core::package::pack_package(source_dir, Some(&output)).unwrap();
    std::fs::read(&output).unwrap()
}

// ---------------------------------------------------------------------------
// Tests — detect_package_kind
// ---------------------------------------------------------------------------

#[test]
fn detect_package_kind_identifies_python() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("data-pipeline");
    std::fs::create_dir_all(&src).unwrap();
    create_python_source_dir(&src, "data-pipeline", "1.0.0", "workflow.tasks", true);
    let archive = pack_to_bytes(&src, tmp.path());

    let kind = detect_package_kind(&archive).unwrap();
    assert!(matches!(kind, PackageKind::Python { .. }));
}

#[test]
fn detect_package_kind_identifies_rust() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("rust-workflow");
    std::fs::create_dir_all(&src).unwrap();
    create_rust_source_dir(&src, "rust-workflow", "0.1.0");
    let archive = pack_to_bytes(&src, tmp.path());

    let kind = detect_package_kind(&archive).unwrap();
    assert!(matches!(kind, PackageKind::Rust { .. }));
}

// ---------------------------------------------------------------------------
// Tests — extract_python_package
// ---------------------------------------------------------------------------

#[test]
fn extract_python_package_full_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("data-pipeline");
    std::fs::create_dir_all(&src).unwrap();
    create_python_source_dir(&src, "data-pipeline", "1.0.0", "workflow.tasks", true);
    let archive = pack_to_bytes(&src, tmp.path());

    let staging = TempDir::new().unwrap();
    let extracted = extract_python_package(&archive, staging.path()).unwrap();

    // Verify directory structure
    assert!(extracted.root_dir.exists());
    assert!(extracted.workflow_dir.exists());

    // Verify metadata was parsed correctly
    assert_eq!(extracted.package_name, "data-pipeline");
    assert_eq!(extracted.version, "1.0.0");
    assert_eq!(extracted.entry_module, "workflow.tasks");
    assert_eq!(extracted.workflow_name, "data-pipeline");

    // Verify files were extracted
    assert!(extracted.workflow_dir.join("tasks.py").exists());
}

#[test]
fn extract_rejects_rust_archive() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("rust-pkg");
    std::fs::create_dir_all(&src).unwrap();
    create_rust_source_dir(&src, "rust-pkg", "0.1.0");
    let archive = pack_to_bytes(&src, tmp.path());

    let staging = TempDir::new().unwrap();
    let err = extract_python_package(&archive, staging.path()).unwrap_err();
    assert!(
        format!("{:?}", err).contains("WrongLanguage"),
        "Expected WrongLanguage error, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Tests — manifest_schema validation (schema logic, not archive format)
// ---------------------------------------------------------------------------

fn make_python_manifest() -> Manifest {
    Manifest {
        format_version: "2".to_string(),
        package: PackageInfo {
            name: "data-pipeline-example".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Example Python workflow".to_string()),
            fingerprint: "sha256:abc123".to_string(),
            targets: vec!["linux-x86_64".to_string()],
        },
        language: PackageLanguage::Python,
        python: Some(PythonRuntime {
            requires_python: ">=3.10".to_string(),
            entry_module: "workflow.tasks".to_string(),
        }),
        rust: None,
        tasks: vec![
            TaskDefinition {
                id: "fetch-data".to_string(),
                function: "workflow.tasks:fetch_data".to_string(),
                dependencies: vec![],
                description: None,
                retries: 0,
                timeout_seconds: None,
            },
            TaskDefinition {
                id: "validate-data".to_string(),
                function: "workflow.tasks:validate_data".to_string(),
                dependencies: vec!["fetch-data".to_string()],
                description: None,
                retries: 0,
                timeout_seconds: None,
            },
        ],
        triggers: vec![],
        created_at: chrono::Utc::now(),
        signature: None,
    }
}

#[test]
fn manifest_validates_task_dependency_references() {
    let mut manifest = make_python_manifest();
    manifest.tasks[1].dependencies = vec!["nonexistent-task".to_string()];

    let err = manifest.validate().unwrap_err();
    assert!(
        matches!(err, ManifestValidationError::InvalidDependency { .. }),
        "Expected InvalidDependency error, got: {err:?}"
    );
}

#[test]
fn manifest_validates_duplicate_task_ids() {
    let mut manifest = make_python_manifest();
    manifest.tasks[1].id = "fetch-data".to_string();

    let err = manifest.validate().unwrap_err();
    assert!(
        matches!(err, ManifestValidationError::DuplicateTaskId { .. }),
        "Expected DuplicateTaskId error, got: {err:?}"
    );
}

#[test]
fn manifest_validates_python_function_path_format() {
    let mut manifest = make_python_manifest();
    manifest.tasks[0].function = "missing_colon_separator".to_string();

    let err = manifest.validate().unwrap_err();
    assert!(
        matches!(err, ManifestValidationError::InvalidFunctionPath { .. }),
        "Expected InvalidFunctionPath error, got: {err:?}"
    );
}
