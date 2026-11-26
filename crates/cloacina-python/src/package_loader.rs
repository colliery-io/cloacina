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
//! Extracts `.cloacina` source archives containing Python workflow packages,
//! validates `package.toml` with `CloacinaMetadata`, and prepares the package
//! for task execution via PyO3.

use std::path::{Path, PathBuf};

use cloacina::registry::error::LoaderError;

/// An extracted Python package ready for task execution.
#[derive(Debug, Clone)]
pub struct ExtractedPythonPackage {
    /// Root directory containing the extracted source.
    pub root_dir: PathBuf,
    /// Path to the `vendor/` directory (added to `sys.path`).
    pub vendor_dir: PathBuf,
    /// Path to the `workflow/` directory (added to `sys.path`).
    pub workflow_dir: PathBuf,
    /// Entry module to import tasks from (e.g., `"workflow.tasks"`).
    pub entry_module: String,
    /// Package name from `package.toml`.
    pub package_name: String,
    /// Package version from `package.toml`.
    pub version: String,
    /// Workflow name from metadata.
    pub workflow_name: String,
}

/// Result of detecting the package language from a source archive.
pub enum PackageKind {
    /// Python workflow package.
    Python {
        workflow_name: String,
        package_name: String,
        version: String,
    },
    /// Rust dynamic-library package.
    Rust {
        workflow_name: String,
        package_name: String,
        version: String,
    },
}

/// Detect the package kind (Python or Rust) from a `.cloacina` source archive.
///
/// Unpacks the archive to a temp dir, reads `package.toml`, and checks
/// the `language` field in `CloacinaMetadata`.
pub fn detect_package_kind(archive_data: &[u8]) -> Result<PackageKind, LoaderError> {
    let tmp = tempfile::TempDir::new().map_err(|e| LoaderError::FileSystem {
        path: "tempdir".to_string(),
        error: e.to_string(),
    })?;

    let archive_path = tmp.path().join("pkg.cloacina");
    std::fs::write(&archive_path, archive_data).map_err(|e| LoaderError::FileSystem {
        path: archive_path.display().to_string(),
        error: e.to_string(),
    })?;

    let extract_dir = tmp.path().join("extract");
    std::fs::create_dir_all(&extract_dir).map_err(|e| LoaderError::FileSystem {
        path: extract_dir.display().to_string(),
        error: e.to_string(),
    })?;

    let source_dir =
        fidius_core::package::unpack_package(&archive_path, &extract_dir).map_err(|e| {
            LoaderError::MetadataExtraction {
                reason: format!("Failed to unpack source archive: {e}"),
            }
        })?;

    let manifest =
        fidius_core::package::load_manifest::<cloacina_workflow_plugin::CloacinaMetadata>(
            &source_dir,
        )
        .map_err(|e| LoaderError::ManifestParse {
            reason: format!("Failed to parse package.toml: {e}"),
        })?;

    let pkg = &manifest.package;
    let meta = &manifest.metadata;

    let wf_name = meta
        .effective_workflow_name()
        .unwrap_or("unknown")
        .to_string();

    match meta.language.as_str() {
        "python" => Ok(PackageKind::Python {
            workflow_name: wf_name,
            package_name: pkg.name.clone(),
            version: pkg.version.clone(),
        }),
        _ => Ok(PackageKind::Rust {
            workflow_name: wf_name,
            package_name: pkg.name.clone(),
            version: pkg.version.clone(),
        }),
    }
}

/// Extract a Python workflow package from a `.cloacina` source archive.
///
/// The archive is unpacked via fidius into a sub-directory of *staging_dir*.
/// The returned [`ExtractedPythonPackage`] contains paths to the
/// workflow source and vendored dependencies.
pub fn extract_python_package(
    archive_data: &[u8],
    staging_dir: &Path,
) -> Result<ExtractedPythonPackage, LoaderError> {
    // Write archive to staging dir
    let archive_path = staging_dir.join(format!("{}.cloacina", uuid::Uuid::new_v4()));
    std::fs::write(&archive_path, archive_data).map_err(|e| LoaderError::FileSystem {
        path: archive_path.display().to_string(),
        error: e.to_string(),
    })?;

    // Unpack via fidius
    let extract_dir = staging_dir.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&extract_dir).map_err(|e| LoaderError::FileSystem {
        path: extract_dir.display().to_string(),
        error: e.to_string(),
    })?;

    let source_dir =
        fidius_core::package::unpack_package(&archive_path, &extract_dir).map_err(|e| {
            LoaderError::FileSystem {
                path: archive_path.display().to_string(),
                error: format!("Failed to unpack source archive: {e}"),
            }
        })?;

    // Read package.toml
    let manifest =
        fidius_core::package::load_manifest::<cloacina_workflow_plugin::CloacinaMetadata>(
            &source_dir,
        )
        .map_err(|e| LoaderError::ManifestParse {
            reason: format!("Failed to parse package.toml: {e}"),
        })?;

    // Validate language
    if manifest.metadata.language != "python" {
        return Err(LoaderError::WrongLanguage {
            expected: "python".to_string(),
            actual: manifest.metadata.language.clone(),
        });
    }

    let entry_module = manifest
        .metadata
        .entry_module
        .as_ref()
        .ok_or(LoaderError::MissingPythonConfig)?
        .clone();

    let workflow_dir = source_dir.join("workflow");

    // Workflow directory is required
    if !workflow_dir.exists() {
        return Err(LoaderError::MissingSourceDir);
    }

    // CLOACINA-T-1048: the Python version and the platform of this process
    // decide whether the package loads here, and from which vendor folder.
    let vendor_dir = resolve_python_env(
        &manifest.package.name,
        &source_dir,
        &manifest.metadata,
        &crate::interpreter_version(),
        &cloacina_workflow_plugin::python_env::host_platform(),
    )
    .map_err(|reason| LoaderError::Validation { reason })?;

    // Clean up archive file
    let _ = std::fs::remove_file(&archive_path);

    Ok(ExtractedPythonPackage {
        root_dir: source_dir,
        vendor_dir,
        workflow_dir,
        entry_module,
        package_name: manifest.package.name,
        version: manifest.package.version,
        workflow_name: manifest
            .metadata
            .effective_workflow_name()
            .unwrap_or("unknown")
            .to_string(),
    })
}

/// Check a Python package against the Python version and the platform that
/// load it, and choose its vendor folder (CLOACINA-T-1048).
///
/// - `requires_python` must allow `python`, and dependencies that
///   `cloacinactl package build` vendored must be for the same `major.minor`.
/// - The vendor folder is `vendor/<platform>/` when the package has one. A
///   package with compiled dependencies for other platforms only is refused.
///
/// Returns the folder to put on `sys.path`. For a package with no vendored
/// dependencies it is `vendor/`, which does not have to exist.
pub fn resolve_python_env(
    package_name: &str,
    package_root: &Path,
    metadata: &cloacina_workflow_plugin::CloacinaMetadata,
    python: &str,
    platform: &str,
) -> Result<PathBuf, String> {
    use cloacina_workflow_plugin::python_env;

    python_env::check_python(
        package_name,
        metadata.requires_python.as_deref(),
        metadata.vendored.as_ref(),
        python,
        "this host",
    )?;
    let chosen = python_env::select_vendor_dir(
        package_name,
        package_root,
        metadata.vendored.as_ref(),
        platform,
    )?;
    Ok(chosen.unwrap_or_else(|| package_root.join("vendor")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn python_metadata(extra: &str) -> cloacina_workflow_plugin::CloacinaMetadata {
        toml::from_str(&format!(
            "workflow_name = \"w\"\nlanguage = \"python\"\nentry_module = \"w.tasks\"\n{extra}"
        ))
        .unwrap()
    }

    #[test]
    fn test_env_refuses_requires_python_the_interpreter_does_not_meet() {
        let tmp = TempDir::new().unwrap();
        let meta = python_metadata("requires_python = \">=3.12\"\n");
        let err =
            resolve_python_env("etl", tmp.path(), &meta, "3.11.9", "x86_64-linux").unwrap_err();
        assert!(err.contains("`etl`"), "{err}");
        assert!(err.contains(">=3.12"), "{err}");
        assert!(err.contains("3.11.9"), "{err}");
        // The same package loads on 3.12, with the flat vendor/ path.
        assert_eq!(
            resolve_python_env("etl", tmp.path(), &meta, "3.12.1", "x86_64-linux").unwrap(),
            tmp.path().join("vendor")
        );
    }

    #[test]
    fn test_env_refuses_dependencies_vendored_for_another_python() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("vendor/x86_64-linux")).unwrap();
        let meta = python_metadata(
            "[vendored]\npython_version = \"3.11\"\nplatforms = [\"x86_64-linux\"]\nnative = true\n",
        );
        let err =
            resolve_python_env("etl", tmp.path(), &meta, "3.12.1", "x86_64-linux").unwrap_err();
        assert!(err.contains("vendored for Python 3.11"), "{err}");
    }

    #[test]
    fn test_env_picks_the_folder_of_this_platform() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("vendor/x86_64-linux")).unwrap();
        std::fs::create_dir_all(tmp.path().join("vendor/aarch64-linux")).unwrap();
        let meta = python_metadata(
            "[vendored]\npython_version = \"3.11\"\nplatforms = [\"aarch64-linux\", \"x86_64-linux\"]\nnative = true\n",
        );
        assert_eq!(
            resolve_python_env("etl", tmp.path(), &meta, "3.11.2", "aarch64-linux").unwrap(),
            tmp.path().join("vendor/aarch64-linux")
        );
    }

    #[test]
    fn test_env_refuses_native_dependencies_for_another_platform_only() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("vendor/x86_64-linux")).unwrap();
        let meta = python_metadata(
            "[vendored]\npython_version = \"3.11\"\nplatforms = [\"x86_64-linux\"]\nnative = true\n",
        );
        let err =
            resolve_python_env("etl", tmp.path(), &meta, "3.11.2", "aarch64-linux").unwrap_err();
        assert!(
            err.contains("native dependencies vendored for x86_64-linux"),
            "{err}"
        );
        assert!(err.contains("aarch64-linux"), "{err}");
    }

    #[test]
    fn test_interpreter_version_is_a_version() {
        let v = crate::interpreter_version();
        assert!(
            cloacina_workflow_plugin::python_env::major_minor(&v).is_ok(),
            "{v}"
        );
    }

    /// Create a fidius source package directory for a Python workflow.
    fn create_python_source_package(
        dir: &Path,
        name: &str,
        include_workflow: bool,
    ) -> std::path::PathBuf {
        // package.toml
        let package_toml = format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "test_workflow"
language = "python"
description = "Test Python workflow"
requires_python = ">=3.10"
entry_module = "workflow.tasks"
"#
        );
        std::fs::write(dir.join("package.toml"), package_toml).unwrap();

        // workflow/ directory
        if include_workflow {
            std::fs::create_dir_all(dir.join("workflow")).unwrap();
            std::fs::write(dir.join("workflow/__init__.py"), "# workflow init\n").unwrap();
            std::fs::write(
                dir.join("workflow/tasks.py"),
                "def hello(ctx): return ctx\n",
            )
            .unwrap();
        }

        // vendor/ directory
        std::fs::create_dir_all(dir.join("vendor")).unwrap();

        // Pack it
        let output = dir.parent().unwrap().join(format!("{name}-0.1.0.cloacina"));
        fidius_core::package::pack_package(dir, Some(&output)).unwrap();
        output
    }

    #[test]
    fn test_detect_package_kind_python() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let archive_path = create_python_source_package(&pkg_dir, "test-py", true);
        let archive_data = std::fs::read(&archive_path).unwrap();

        let kind = detect_package_kind(&archive_data).unwrap();
        assert!(matches!(kind, PackageKind::Python { .. }));
    }

    #[test]
    fn test_extract_python_package() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let archive_path = create_python_source_package(&pkg_dir, "test-py", true);
        let archive_data = std::fs::read(&archive_path).unwrap();

        let staging = TempDir::new().unwrap();
        let extracted = extract_python_package(&archive_data, staging.path()).unwrap();

        assert!(extracted.root_dir.exists());
        assert!(extracted.workflow_dir.exists());
        assert_eq!(extracted.entry_module, "workflow.tasks");
        assert_eq!(extracted.package_name, "test-py");
        assert_eq!(extracted.workflow_name, "test_workflow");
    }

    #[test]
    fn test_extract_missing_workflow_dir() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let archive_path = create_python_source_package(&pkg_dir, "no-workflow", false);
        let archive_data = std::fs::read(&archive_path).unwrap();

        let staging = TempDir::new().unwrap();
        let err = extract_python_package(&archive_data, staging.path()).unwrap_err();
        assert!(matches!(err, LoaderError::MissingSourceDir));
    }

    /// The checks run on the real load path: an archive whose
    /// `requires_python` no interpreter meets is refused at extract.
    #[test]
    fn test_extract_refuses_requires_python_the_interpreter_does_not_meet() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("pkg");
        std::fs::create_dir_all(pkg_dir.join("workflow")).unwrap();
        std::fs::write(pkg_dir.join("workflow/tasks.py"), "").unwrap();
        std::fs::write(
            pkg_dir.join("package.toml"),
            r#"[package]
name = "too-new"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "w"
language = "python"
requires_python = ">=99.0"
entry_module = "tasks"
"#,
        )
        .unwrap();
        let output = tmp.path().join("too-new-0.1.0.cloacina");
        fidius_core::package::pack_package(&pkg_dir, Some(&output)).unwrap();
        let archive_data = std::fs::read(&output).unwrap();

        let staging = TempDir::new().unwrap();
        let err = extract_python_package(&archive_data, staging.path()).unwrap_err();
        let text = err.to_string();
        assert!(matches!(err, LoaderError::Validation { .. }), "{text}");
        assert!(text.contains("`too-new` requires Python >=99.0"), "{text}");
        assert!(text.contains("this host runs Python"), "{text}");
    }

    #[test]
    fn test_wrong_language_rejected() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        // Create a Rust package.toml but try to extract as Python
        let package_toml = r#"[package]
name = "rust-pkg"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "rust_workflow"
language = "rust"
"#;
        std::fs::write(pkg_dir.join("package.toml"), package_toml).unwrap();
        std::fs::create_dir_all(pkg_dir.join("src")).unwrap();
        std::fs::write(pkg_dir.join("src/lib.rs"), "// placeholder").unwrap();

        let output = tmp.path().join("rust-pkg-0.1.0.cloacina");
        fidius_core::package::pack_package(&pkg_dir, Some(&output)).unwrap();
        let archive_data = std::fs::read(&output).unwrap();

        let staging = TempDir::new().unwrap();
        let err = extract_python_package(&archive_data, staging.path()).unwrap_err();
        assert!(matches!(err, LoaderError::WrongLanguage { .. }));
    }
}
