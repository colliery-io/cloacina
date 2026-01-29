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
//! These tests build `.cloacina` archives in memory using the v2 manifest
//! schema, then exercise the server-side python loader to verify the full
//! round-trip: archive → peek → detect → extract → validate.

use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;
use tempfile::TempDir;

use cloacina::packaging::{
    ManifestV2, ManifestValidationError, PackageInfoV2, PackageLanguage, PythonRuntime,
    RustRuntime, TaskDefinitionV2,
};
use cloacina::registry::loader::{
    detect_package_kind, extract_python_package, peek_manifest, PackageKind,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `.cloacina` archive in memory with realistic structure.
fn build_archive(manifest: &ManifestV2, workflow_files: &[(&str, &[u8])]) -> Vec<u8> {
    let buf = Vec::new();
    let enc = GzEncoder::new(buf, Compression::fast());
    let mut builder = Builder::new(enc);

    // manifest.json
    let manifest_json = serde_json::to_vec_pretty(manifest).unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_json.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "manifest.json", manifest_json.as_slice())
        .unwrap();

    // workflow files
    for (path, content) in workflow_files {
        let mut h = tar::Header::new_gnu();
        h.set_size(content.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        builder.append_data(&mut h, *path, *content).unwrap();
    }

    // vendor/ directory marker
    let mut dh = tar::Header::new_gnu();
    dh.set_size(0);
    dh.set_entry_type(tar::EntryType::Directory);
    dh.set_mode(0o755);
    dh.set_cksum();
    builder
        .append_data(&mut dh, "vendor/", &[] as &[u8])
        .unwrap();

    let enc = builder.into_inner().unwrap();
    enc.finish().unwrap()
}

/// Create a manifest matching the example data-pipeline project.
fn data_pipeline_manifest() -> ManifestV2 {
    ManifestV2 {
        format_version: "2".to_string(),
        package: PackageInfoV2 {
            name: "data-pipeline-example".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Example Python workflow for Cloacina".to_string()),
            fingerprint: "sha256:abc123".to_string(),
            targets: vec!["linux-x86_64".to_string(), "macos-arm64".to_string()],
        },
        language: PackageLanguage::Python,
        python: Some(PythonRuntime {
            requires_python: ">=3.11".to_string(),
            entry_module: "data_pipeline.tasks".to_string(),
        }),
        rust: None,
        tasks: vec![
            TaskDefinitionV2 {
                id: "fetch-data".to_string(),
                function: "data_pipeline.tasks:fetch_data".to_string(),
                dependencies: vec![],
                description: Some("Fetch data from external source".to_string()),
                retries: 0,
                timeout_seconds: None,
            },
            TaskDefinitionV2 {
                id: "validate-data".to_string(),
                function: "data_pipeline.tasks:validate_data".to_string(),
                dependencies: vec!["fetch-data".to_string()],
                description: Some("Validate raw data".to_string()),
                retries: 0,
                timeout_seconds: None,
            },
            TaskDefinitionV2 {
                id: "aggregate-data".to_string(),
                function: "data_pipeline.tasks:aggregate_data".to_string(),
                dependencies: vec!["validate-data".to_string()],
                description: Some("Compute summary statistics".to_string()),
                retries: 0,
                timeout_seconds: None,
            },
            TaskDefinitionV2 {
                id: "generate-report".to_string(),
                function: "data_pipeline.tasks:generate_report".to_string(),
                dependencies: vec!["aggregate-data".to_string()],
                description: Some("Produce summary report".to_string()),
                retries: 0,
                timeout_seconds: None,
            },
        ],
        created_at: Utc::now(),
        signature: None,
    }
}

/// Workflow source files for the data-pipeline example.
fn data_pipeline_files() -> Vec<(&'static str, &'static [u8])> {
    vec![
        (
            "workflow/data_pipeline/__init__.py",
            b"# Data Pipeline Example\n",
        ),
        (
            "workflow/data_pipeline/tasks.py",
            b"from cloaca import task\n\n@task(id=\"fetch-data\", dependencies=[])\ndef fetch_data(context):\n    return context\n",
        ),
    ]
}

// ---------------------------------------------------------------------------
// Tests — archive round-trip
// ---------------------------------------------------------------------------

#[test]
fn peek_manifest_returns_correct_metadata() {
    let manifest = data_pipeline_manifest();
    let archive = build_archive(&manifest, &data_pipeline_files());

    let peeked = peek_manifest(&archive).unwrap();
    assert_eq!(peeked.package.name, "data-pipeline-example");
    assert_eq!(peeked.package.version, "1.0.0");
    assert_eq!(peeked.language, PackageLanguage::Python);
    assert_eq!(peeked.tasks.len(), 4);
}

#[test]
fn detect_package_kind_identifies_python() {
    let manifest = data_pipeline_manifest();
    let archive = build_archive(&manifest, &data_pipeline_files());

    let kind = detect_package_kind(&archive).unwrap();
    assert!(matches!(kind, PackageKind::Python(_)));
}

#[test]
fn detect_package_kind_identifies_rust() {
    let mut manifest = data_pipeline_manifest();
    manifest.language = PackageLanguage::Rust;
    manifest.python = None;
    manifest.rust = Some(RustRuntime {
        library_path: "lib/libworkflow.so".to_string(),
    });
    manifest.tasks[0].function = "execute_task".to_string();
    manifest.tasks[1].function = "execute_task".to_string();
    manifest.tasks[2].function = "execute_task".to_string();
    manifest.tasks[3].function = "execute_task".to_string();

    let archive = build_archive(&manifest, &data_pipeline_files());
    let kind = detect_package_kind(&archive).unwrap();
    assert!(matches!(kind, PackageKind::Rust(_)));
}

#[test]
fn extract_python_package_full_roundtrip() {
    let manifest = data_pipeline_manifest();
    let archive = build_archive(&manifest, &data_pipeline_files());
    let staging = TempDir::new().unwrap();

    let extracted = extract_python_package(&archive, staging.path()).unwrap();

    // Verify directory structure
    assert!(extracted.root_dir.exists());
    assert!(extracted.workflow_dir.exists());
    assert!(extracted.vendor_dir.parent().is_some());

    // Verify manifest was parsed correctly
    assert_eq!(extracted.manifest.package.name, "data-pipeline-example");
    assert_eq!(extracted.manifest.tasks.len(), 4);
    assert_eq!(extracted.entry_module, "data_pipeline.tasks");

    // Verify files were extracted
    assert!(extracted
        .workflow_dir
        .join("data_pipeline/__init__.py")
        .exists());
    assert!(extracted
        .workflow_dir
        .join("data_pipeline/tasks.py")
        .exists());
}

#[test]
fn extract_rejects_rust_archive() {
    let mut manifest = data_pipeline_manifest();
    manifest.language = PackageLanguage::Rust;
    manifest.python = None;
    manifest.rust = Some(RustRuntime {
        library_path: "lib/libworkflow.so".to_string(),
    });
    manifest
        .tasks
        .iter_mut()
        .for_each(|t| t.function = "ffi_entry".to_string());

    let archive = build_archive(&manifest, &data_pipeline_files());
    let staging = TempDir::new().unwrap();

    let err = extract_python_package(&archive, staging.path()).unwrap_err();
    assert!(
        err.to_string().contains("Wrong language")
            || err.to_string().contains("wrong language")
            || format!("{:?}", err).contains("WrongLanguage"),
        "Expected WrongLanguage error, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Tests — manifest validation
// ---------------------------------------------------------------------------

#[test]
fn manifest_validates_task_dependency_references() {
    let mut manifest = data_pipeline_manifest();
    manifest.tasks[1].dependencies = vec!["nonexistent-task".to_string()];

    let err = manifest.validate().unwrap_err();
    assert!(
        matches!(err, ManifestValidationError::InvalidDependency { .. }),
        "Expected InvalidDependency error, got: {err:?}"
    );
}

#[test]
fn manifest_validates_duplicate_task_ids() {
    let mut manifest = data_pipeline_manifest();
    manifest.tasks[1].id = "fetch-data".to_string();

    let err = manifest.validate().unwrap_err();
    assert!(
        matches!(err, ManifestValidationError::DuplicateTaskId { .. }),
        "Expected DuplicateTaskId error, got: {err:?}"
    );
}

#[test]
fn manifest_validates_python_function_path_format() {
    let mut manifest = data_pipeline_manifest();
    manifest.tasks[0].function = "missing_colon_separator".to_string();

    let err = manifest.validate().unwrap_err();
    assert!(
        matches!(err, ManifestValidationError::InvalidFunctionPath { .. }),
        "Expected InvalidFunctionPath error, got: {err:?}"
    );
}
