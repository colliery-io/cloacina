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
        cloacina_workflow_plugin::unpack_limit::unpack_package(&archive_path, &extract_dir)
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Failed to unpack source archive: {e}"),
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
///
/// `vendored_set` is the archive of the dependencies a compiler vendored for
/// this package and this platform (CLOACINA-T-1073). It is unpacked into
/// `vendor/<platform>/` of the staged package, and its record takes the place
/// of `[metadata.vendored]`. A package that pins dependencies in
/// `requirements.lock`, ships no `vendor/` folder and gets no set is refused:
/// its imports would fail.
pub fn extract_python_package(
    archive_data: &[u8],
    vendored_set: Option<&[u8]>,
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
        cloacina_workflow_plugin::unpack_limit::unpack_package(&archive_path, &extract_dir)
            .map_err(|e| LoaderError::FileSystem {
                path: archive_path.display().to_string(),
                error: format!("Failed to unpack source archive: {e}"),
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

    // CLOACINA-T-1073: the dependencies a compiler vendored go into
    // `vendor/<platform>/` before the folder is chosen.
    let platform = cloacina_workflow_plugin::python_env::host_platform();
    let mut manifest = manifest;
    if let Some(record) = stage_vendored_set(
        &manifest.package.name,
        &source_dir,
        vendored_set,
        staging_dir,
        &platform,
    )
    .map_err(|reason| LoaderError::Validation { reason })?
    {
        manifest.metadata.vendored = Some(record);
    }

    // CLOACINA-T-1048: the Python version and the platform of this process
    // decide whether the package loads here, and from which vendor folder.
    let vendor_dir = resolve_python_env(
        &manifest.package.name,
        &source_dir,
        &manifest.metadata,
        &crate::interpreter_version(),
        &platform,
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

/// Put the dependencies a compiler vendored into a staged package
/// (CLOACINA-T-1073).
///
/// - With a set: it is unpacked into `<package_root>/vendor/`, under the
///   archive limits of a package, and its record is returned.
/// - With no set: a package that pins dependencies in `requirements.lock` and
///   ships no `vendor/` folder is refused. `Ok(None)` otherwise.
pub fn stage_vendored_set(
    package_name: &str,
    package_root: &Path,
    vendored_set: Option<&[u8]>,
    staging_dir: &Path,
    platform: &str,
) -> Result<Option<cloacina_workflow_plugin::python_env::Vendored>, String> {
    use cloacina_workflow_plugin::{python_env, python_vendor};

    let Some(set) = vendored_set else {
        if package_root.join("vendor").is_dir() {
            return Ok(None);
        }
        let pinned = std::fs::read_to_string(package_root.join(python_env::LOCK_FILE))
            .map(|lock| python_vendor::lock_has_pins(&lock))
            .unwrap_or(false);
        if pinned {
            return Err(format!(
                "package `{package_name}` pins dependencies in {} and has no vendored \
                 dependencies for this platform ({platform}). The compiler vendors them: add \
                 `{platform}` to its `--python-platform` and publish the package again, or \
                 ship them in the package with `cloacinactl package build --vendor`",
                python_env::LOCK_FILE
            ));
        }
        return Ok(None);
    };

    let archive = staging_dir.join(format!("{}.vendor", uuid::Uuid::new_v4()));
    std::fs::write(&archive, set).map_err(|e| format!("write {}: {e}", archive.display()))?;
    let scratch = staging_dir.join(uuid::Uuid::new_v4().to_string());
    let result = std::fs::create_dir_all(&scratch)
        .map_err(|e| format!("create {}: {e}", scratch.display()))
        .and_then(|()| {
            python_vendor::unpack_vendor_artifact(
                &archive,
                package_root,
                &scratch,
                &cloacina_workflow_plugin::unpack_limit::unpack_options(),
            )
        });
    let _ = std::fs::remove_file(&archive);
    let _ = std::fs::remove_dir_all(&scratch);
    result
        .map(Some)
        .map_err(|e| format!("package `{package_name}`: {e}"))
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

    const LOCK: &str = "six==1.17.0 \\\n    --hash=sha256:4721f391ed90541fddacab5acf947aa0d3dc7d27b2e1e8eda2be8970586c3274\n";

    /// A vendored set for `platform`, as a compiler stores it.
    fn vendored_set(tmp: &Path, platform: &str, python: &str) -> Vec<u8> {
        use cloacina_workflow_plugin::{python_env, python_vendor};
        let scratch = tempfile::TempDir::new_in(tmp).unwrap().keep();
        let target = scratch.join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("six.py"), "VALUE = 1\n").unwrap();
        let work = scratch.join("work");
        std::fs::create_dir_all(&work).unwrap();
        let out = scratch.join("vendored.set");
        python_vendor::pack_vendor_artifact(
            &work,
            "etl",
            "1.0.0",
            &python_env::Vendored {
                python_version: python.to_string(),
                platforms: vec![platform.to_string()],
                native: false,
                lock_file: Some(python_env::LOCK_FILE.to_string()),
            },
            &target,
            &out,
        )
        .unwrap();
        std::fs::read(&out).unwrap()
    }

    #[test]
    fn test_vendored_set_is_staged_into_the_platform_folder() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("etl-1.0.0");
        std::fs::create_dir_all(root.join("workflow")).unwrap();
        std::fs::write(root.join("requirements.lock"), LOCK).unwrap();
        let staging = tmp.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let set = vendored_set(tmp.path(), "x86_64-linux", "3.11");

        let record = stage_vendored_set("etl", &root, Some(&set), &staging, "x86_64-linux")
            .unwrap()
            .unwrap();
        assert_eq!(record.platforms, vec!["x86_64-linux".to_string()]);
        assert!(root.join("vendor/x86_64-linux/six.py").is_file());
        // Nothing is left in the staging folder.
        assert_eq!(std::fs::read_dir(&staging).unwrap().count(), 0);

        // The record takes the place of `[metadata.vendored]`: the folder is
        // chosen, and the Python version is checked.
        let mut meta = python_metadata("");
        meta.vendored = Some(record);
        assert_eq!(
            resolve_python_env("etl", &root, &meta, "3.11.9", "x86_64-linux").unwrap(),
            root.join("vendor/x86_64-linux")
        );
        let err = resolve_python_env("etl", &root, &meta, "3.12.1", "x86_64-linux").unwrap_err();
        assert!(err.contains("vendored for Python 3.11"), "{err}");
    }

    #[test]
    fn test_pinned_dependencies_with_no_vendored_set_are_refused() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("etl-1.0.0");
        std::fs::create_dir_all(root.join("workflow")).unwrap();
        std::fs::write(root.join("requirements.lock"), LOCK).unwrap();
        let err = stage_vendored_set("etl", &root, None, tmp.path(), "aarch64-macos").unwrap_err();
        assert!(err.contains("package `etl`"), "{err}");
        assert!(err.contains("(aarch64-macos)"), "{err}");
        assert!(err.contains("--python-platform"), "{err}");
        assert!(err.contains("--vendor"), "{err}");

        // A lock file with no pins, or no lock file, needs no set.
        std::fs::write(root.join("requirements.lock"), "# nothing\n").unwrap();
        assert_eq!(
            stage_vendored_set("etl", &root, None, tmp.path(), "aarch64-macos").unwrap(),
            None
        );
        std::fs::remove_file(root.join("requirements.lock")).unwrap();
        assert_eq!(
            stage_vendored_set("etl", &root, None, tmp.path(), "aarch64-macos").unwrap(),
            None
        );
    }

    #[test]
    fn test_package_with_its_own_vendor_folder_is_left_as_it_is() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("etl-1.0.0");
        std::fs::create_dir_all(root.join("vendor/x86_64-linux")).unwrap();
        std::fs::write(root.join("vendor/x86_64-linux/six.py"), "VALUE = 0\n").unwrap();
        std::fs::write(root.join("requirements.lock"), LOCK).unwrap();
        // No set: the folder the package ships is used.
        assert_eq!(
            stage_vendored_set("etl", &root, None, tmp.path(), "x86_64-linux").unwrap(),
            None
        );
        // A set on top of it is refused, and the folder is not changed.
        let set = vendored_set(tmp.path(), "x86_64-linux", "3.11");
        let staging = tmp.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let err =
            stage_vendored_set("etl", &root, Some(&set), &staging, "x86_64-linux").unwrap_err();
        assert!(err.contains("ships a vendor/ folder"), "{err}");
        assert_eq!(
            std::fs::read_to_string(root.join("vendor/x86_64-linux/six.py")).unwrap(),
            "VALUE = 0\n"
        );
    }

    #[test]
    fn test_bytes_that_are_not_a_vendored_set_are_refused() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("etl-1.0.0");
        std::fs::create_dir_all(root.join("workflow")).unwrap();
        let staging = tmp.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let err = stage_vendored_set(
            "etl",
            &root,
            Some(b"not an archive"),
            &staging,
            "x86_64-linux",
        )
        .unwrap_err();
        assert!(err.contains("package `etl`"), "{err}");
        assert!(err.contains("cannot be unpacked"), "{err}");
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
        let extracted = extract_python_package(&archive_data, None, staging.path()).unwrap();

        assert!(extracted.root_dir.exists());
        assert!(extracted.workflow_dir.exists());
        assert_eq!(extracted.entry_module, "workflow.tasks");
        assert_eq!(extracted.package_name, "test-py");
        assert_eq!(extracted.workflow_name, "test_workflow");
    }

    /// CLOACINA-T-1073: a package with a lock file and no `vendor/` folder
    /// loads with the set the compiler stored for this platform, and is
    /// refused without one.
    #[test]
    fn test_extract_with_the_vendored_set_of_this_platform() {
        use cloacina_workflow_plugin::python_env;
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("requirements.lock"), LOCK).unwrap();
        let archive_path = create_python_source_package(&pkg_dir, "locked-py", true);
        let archive_data = std::fs::read(&archive_path).unwrap();

        let staging = TempDir::new().unwrap();
        let err = extract_python_package(&archive_data, None, staging.path()).unwrap_err();
        assert!(
            err.to_string()
                .contains("no vendored dependencies for this platform"),
            "{err}"
        );

        let platform = python_env::host_platform();
        let (major, minor) = python_env::major_minor(&crate::interpreter_version()).unwrap();
        let set = vendored_set(tmp.path(), &platform, &format!("{major}.{minor}"));
        let staging = TempDir::new().unwrap();
        let extracted = extract_python_package(&archive_data, Some(&set), staging.path()).unwrap();
        assert_eq!(
            extracted.vendor_dir,
            extracted.root_dir.join("vendor").join(&platform)
        );
        assert!(extracted.vendor_dir.join("six.py").is_file());

        // A set for another Python version is refused, as a prebuilt one is.
        let other = vendored_set(tmp.path(), &platform, "3.1");
        let staging = TempDir::new().unwrap();
        let err = extract_python_package(&archive_data, Some(&other), staging.path()).unwrap_err();
        assert!(err.to_string().contains("vendored for Python 3.1"), "{err}");
    }

    #[test]
    fn test_extract_missing_workflow_dir() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let archive_path = create_python_source_package(&pkg_dir, "no-workflow", false);
        let archive_data = std::fs::read(&archive_path).unwrap();

        let staging = TempDir::new().unwrap();
        let err = extract_python_package(&archive_data, None, staging.path()).unwrap_err();
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
        let err = extract_python_package(&archive_data, None, staging.path()).unwrap_err();
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
        let err = extract_python_package(&archive_data, None, staging.path()).unwrap_err();
        assert!(matches!(err, LoaderError::WrongLanguage { .. }));
    }
}
