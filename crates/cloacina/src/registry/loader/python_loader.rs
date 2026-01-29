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

//! Server-side Python package loader.
//!
//! Extracts `.cloacina` archives containing Python workflow packages,
//! validates manifest v2 format, and prepares the package for task
//! execution via PyO3.

use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::packaging::manifest_v2::{ManifestV2, PackageLanguage};
use crate::registry::error::LoaderError;

/// An extracted Python package ready for task execution.
#[derive(Debug, Clone)]
pub struct ExtractedPythonPackage {
    /// Root directory containing the extracted archive.
    pub root_dir: PathBuf,
    /// Path to the `vendor/` directory (added to `sys.path`).
    pub vendor_dir: PathBuf,
    /// Path to the `workflow/` directory (added to `sys.path`).
    pub workflow_dir: PathBuf,
    /// Entry module to import tasks from (e.g., `"workflow.tasks"`).
    pub entry_module: String,
    /// Parsed manifest.
    pub manifest: ManifestV2,
}

/// Result of peeking at a manifest inside an archive.
pub enum PackageKind {
    /// Python workflow package.
    Python(ManifestV2),
    /// Rust dynamic-library package.
    Rust(ManifestV2),
}

/// Peek at the manifest inside a `.cloacina` archive without full extraction.
pub fn peek_manifest(archive_data: &[u8]) -> Result<ManifestV2, LoaderError> {
    let cursor = std::io::Cursor::new(archive_data);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);

    for entry_result in archive.entries().map_err(|e| LoaderError::FileSystem {
        path: "archive".to_string(),
        error: format!("Failed to read archive entries: {e}"),
    })? {
        let mut entry = entry_result.map_err(|e| LoaderError::FileSystem {
            path: "archive".to_string(),
            error: format!("Failed to read archive entry: {e}"),
        })?;

        let path = entry.path().map_err(|e| LoaderError::FileSystem {
            path: "archive".to_string(),
            error: format!("Failed to get entry path: {e}"),
        })?;

        if path.file_name() == Some("manifest.json".as_ref()) {
            let mut data = Vec::new();
            entry
                .read_to_end(&mut data)
                .map_err(|e| LoaderError::FileSystem {
                    path: "manifest.json".to_string(),
                    error: format!("Failed to read manifest: {e}"),
                })?;
            let manifest: ManifestV2 =
                serde_json::from_slice(&data).map_err(|e| LoaderError::ManifestParse {
                    reason: e.to_string(),
                })?;
            return Ok(manifest);
        }
    }

    Err(LoaderError::MissingManifest)
}

/// Determine the package kind (Python or Rust) from archive data.
pub fn detect_package_kind(archive_data: &[u8]) -> Result<PackageKind, LoaderError> {
    let manifest = peek_manifest(archive_data)?;
    match manifest.language {
        PackageLanguage::Python => Ok(PackageKind::Python(manifest)),
        PackageLanguage::Rust => Ok(PackageKind::Rust(manifest)),
    }
}

/// Extract a Python workflow package from a `.cloacina` archive.
///
/// The archive is extracted into a sub-directory of *staging_dir*.
/// The returned [`ExtractedPythonPackage`] contains paths to the
/// workflow source and vendored dependencies.
pub fn extract_python_package(
    archive_data: &[u8],
    staging_dir: &Path,
) -> Result<ExtractedPythonPackage, LoaderError> {
    // Create a unique staging sub-directory
    let package_dir = staging_dir.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&package_dir).map_err(|e| LoaderError::FileSystem {
        path: package_dir.display().to_string(),
        error: e.to_string(),
    })?;

    // Extract tar.gz
    let cursor = std::io::Cursor::new(archive_data);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    archive
        .unpack(&package_dir)
        .map_err(|e| LoaderError::FileSystem {
            path: package_dir.display().to_string(),
            error: format!("Failed to extract archive: {e}"),
        })?;

    // Read manifest
    let manifest_path = package_dir.join("manifest.json");
    let manifest_data = std::fs::read(&manifest_path).map_err(|e| LoaderError::FileSystem {
        path: manifest_path.display().to_string(),
        error: e.to_string(),
    })?;
    let manifest: ManifestV2 =
        serde_json::from_slice(&manifest_data).map_err(|e| LoaderError::ManifestParse {
            reason: e.to_string(),
        })?;

    // Validate language
    if manifest.language != PackageLanguage::Python {
        return Err(LoaderError::WrongLanguage {
            expected: "python".to_string(),
            actual: "rust".to_string(),
        });
    }

    let python_config = manifest
        .python
        .as_ref()
        .ok_or(LoaderError::MissingPythonConfig)?;

    let vendor_dir = package_dir.join("vendor");
    let workflow_dir = package_dir.join("workflow");

    // Workflow directory is required
    if !workflow_dir.exists() {
        return Err(LoaderError::MissingSourceDir);
    }

    Ok(ExtractedPythonPackage {
        root_dir: package_dir,
        vendor_dir,
        workflow_dir,
        entry_module: python_config.entry_module.clone(),
        manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packaging::manifest_v2::{PackageInfoV2, PythonRuntime, TaskDefinitionV2};
    use chrono::Utc;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tar::Builder;
    use tempfile::TempDir;

    /// Build a minimal Python `.cloacina` archive in memory.
    fn build_test_archive(manifest: &ManifestV2, include_workflow: bool) -> Vec<u8> {
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

        // workflow/ directory with a dummy __init__.py
        if include_workflow {
            let init_py = b"# workflow init\n";
            let mut h = tar::Header::new_gnu();
            h.set_size(init_py.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            builder
                .append_data(&mut h, "workflow/__init__.py", init_py.as_slice())
                .unwrap();
        }

        // vendor/ directory (empty marker)
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

    fn make_test_manifest() -> ManifestV2 {
        ManifestV2 {
            format_version: "2".to_string(),
            package: PackageInfoV2 {
                name: "test-workflow".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                fingerprint: "sha256:test".to_string(),
                targets: vec!["linux-x86_64".to_string()],
            },
            language: PackageLanguage::Python,
            python: Some(PythonRuntime {
                requires_python: ">=3.10".to_string(),
                entry_module: "workflow.tasks".to_string(),
            }),
            rust: None,
            tasks: vec![TaskDefinitionV2 {
                id: "hello".to_string(),
                function: "workflow.tasks:hello".to_string(),
                dependencies: vec![],
                description: Some("Say hello".to_string()),
                retries: 0,
                timeout_seconds: None,
            }],
            created_at: Utc::now(),
            signature: None,
        }
    }

    #[test]
    fn test_peek_manifest() {
        let manifest = make_test_manifest();
        let archive = build_test_archive(&manifest, true);

        let peeked = peek_manifest(&archive).unwrap();
        assert_eq!(peeked.package.name, "test-workflow");
        assert_eq!(peeked.language, PackageLanguage::Python);
    }

    #[test]
    fn test_detect_package_kind_python() {
        let manifest = make_test_manifest();
        let archive = build_test_archive(&manifest, true);

        let kind = detect_package_kind(&archive).unwrap();
        assert!(matches!(kind, PackageKind::Python(_)));
    }

    #[test]
    fn test_extract_python_package() {
        let manifest = make_test_manifest();
        let archive = build_test_archive(&manifest, true);
        let staging = TempDir::new().unwrap();

        let extracted = extract_python_package(&archive, staging.path()).unwrap();

        assert!(extracted.root_dir.exists());
        assert!(extracted.workflow_dir.exists());
        assert_eq!(extracted.entry_module, "workflow.tasks");
        assert_eq!(extracted.manifest.package.name, "test-workflow");
    }

    #[test]
    fn test_extract_missing_workflow_dir() {
        let manifest = make_test_manifest();
        let archive = build_test_archive(&manifest, false);
        let staging = TempDir::new().unwrap();

        let err = extract_python_package(&archive, staging.path()).unwrap_err();
        assert!(matches!(err, LoaderError::MissingSourceDir));
    }

    #[test]
    fn test_peek_manifest_missing() {
        // Build an archive with no manifest.json
        let buf = Vec::new();
        let enc = GzEncoder::new(buf, Compression::fast());
        let mut builder = Builder::new(enc);
        let data = b"some file";
        let mut h = tar::Header::new_gnu();
        h.set_size(data.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        builder
            .append_data(&mut h, "other.txt", data.as_slice())
            .unwrap();
        let enc = builder.into_inner().unwrap();
        let archive_data = enc.finish().unwrap();

        let err = peek_manifest(&archive_data).unwrap_err();
        assert!(matches!(err, LoaderError::MissingManifest));
    }

    #[test]
    fn test_wrong_language() {
        use crate::packaging::manifest_v2::RustRuntime;

        let mut manifest = make_test_manifest();
        manifest.language = PackageLanguage::Rust;
        manifest.python = None;
        manifest.rust = Some(RustRuntime {
            library_path: "lib/libworkflow.so".to_string(),
        });
        // Rust function path doesn't need colon
        manifest.tasks[0].function = "cloacina_execute_task".to_string();

        let archive = build_test_archive(&manifest, true);
        let staging = TempDir::new().unwrap();

        let err = extract_python_package(&archive, staging.path()).unwrap_err();
        assert!(matches!(err, LoaderError::WrongLanguage { .. }));
    }
}
