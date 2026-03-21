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

//! Pure Rust builder for Python `.cloacina` packages.
//!
//! Replaces the Python `cloaca build` CLI. Produces the same archive format
//! without requiring `pip install cloaca`:
//!
//! 1. Parse `pyproject.toml` for package metadata and `[tool.cloaca]` config
//! 2. Copy workflow source tree to staging
//! 3. Vendor dependencies via `uv` subprocess
//! 4. Write `manifest.json` (tasks discovered at registration time via PyO3)
//! 5. Create tar.gz archive with SHA256 fingerprint

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use tar::Builder;

use super::manifest_v2::{ManifestV2, PackageInfoV2, PackageLanguage, PythonRuntime};
use super::platform::detect_current_platform;

/// Configuration for building a Python package.
pub struct PythonBuildConfig {
    /// Project directory containing `pyproject.toml`
    pub project_dir: PathBuf,
    /// Output directory for the `.cloacina` archive
    pub output_dir: PathBuf,
    /// Target platforms (default: current platform)
    pub targets: Vec<String>,
    /// Show verbose output
    pub verbose: bool,
}

/// Result of a successful Python package build.
pub struct PythonBuildResult {
    /// Path to the created `.cloacina` archive
    pub archive_path: PathBuf,
    /// Package name
    pub package_name: String,
    /// Package version
    pub version: String,
    /// SHA256 fingerprint
    pub fingerprint: String,
}

/// Parsed `pyproject.toml` fields relevant to package building.
#[derive(Debug)]
struct PyProjectConfig {
    name: String,
    version: String,
    description: Option<String>,
    requires_python: String,
    entry_module: String,
    has_dependencies: bool,
}

/// Build a Python `.cloacina` package from a project directory.
pub fn build_python_package(config: &PythonBuildConfig) -> anyhow::Result<PythonBuildResult> {
    // 1. Parse pyproject.toml
    let pyproject = parse_pyproject(&config.project_dir)?;

    // 2. Determine targets
    let targets = if config.targets.is_empty() {
        vec![detect_current_platform().to_string()]
    } else {
        config.targets.clone()
    };

    // 3. Stage build directory
    let build_dir = config.project_dir.join(".cloaca_build");
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)?;
    }
    std::fs::create_dir_all(&build_dir)?;

    // Copy workflow source
    let workflow_dir = build_dir.join("workflow");
    copy_workflow_source(&config.project_dir, &workflow_dir, &pyproject.entry_module)?;

    // 4. Vendor dependencies
    let vendor_dir = build_dir.join("vendor");
    std::fs::create_dir_all(&vendor_dir)?;
    let lock_file = if pyproject.has_dependencies {
        vendor_dependencies(&config.project_dir, &vendor_dir, &targets, config.verbose)?
    } else {
        if config.verbose {
            tracing::info!("No dependencies to vendor");
        }
        None
    };

    // 5. Write manifest (tasks array empty — discovered at registration via PyO3)
    let manifest = ManifestV2 {
        format_version: "2".to_string(),
        package: PackageInfoV2 {
            name: pyproject.name.clone(),
            version: pyproject.version.clone(),
            description: pyproject.description.clone(),
            fingerprint: String::new(), // Filled after archive creation
            targets: targets.clone(),
        },
        language: PackageLanguage::Python,
        python: Some(PythonRuntime {
            requires_python: pyproject.requires_python.clone(),
            entry_module: pyproject.entry_module.clone(),
        }),
        rust: None,
        tasks: vec![], // Discovered at registration time
        created_at: Utc::now(),
        signature: None,
    };

    let manifest_path = build_dir.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, &manifest_json)?;

    // 6. Create archive (first pass — without fingerprint)
    std::fs::create_dir_all(&config.output_dir)?;
    let archive_name = format!("{}-{}.cloacina", pyproject.name, pyproject.version);
    let archive_path = config.output_dir.join(&archive_name);

    create_archive(
        &archive_path,
        &manifest_path,
        &workflow_dir,
        &vendor_dir,
        lock_file.as_deref(),
    )?;

    // 7. Compute fingerprint and rewrite
    let fingerprint = format!("sha256:{}", compute_sha256(&archive_path)?);

    let mut manifest_with_fp = manifest;
    manifest_with_fp.package.fingerprint = fingerprint.clone();
    let manifest_json = serde_json::to_string_pretty(&manifest_with_fp)?;
    std::fs::write(&manifest_path, &manifest_json)?;

    // Recreate archive with fingerprinted manifest
    create_archive(
        &archive_path,
        &manifest_path,
        &workflow_dir,
        &vendor_dir,
        lock_file.as_deref(),
    )?;

    // 8. Clean up build dir
    std::fs::remove_dir_all(&build_dir)?;

    tracing::info!(
        "Created: {} (fingerprint: {})",
        archive_path.display(),
        fingerprint
    );

    Ok(PythonBuildResult {
        archive_path,
        package_name: pyproject.name,
        version: pyproject.version,
        fingerprint,
    })
}

/// Parse `pyproject.toml` for package metadata and `[tool.cloaca]` config.
fn parse_pyproject(project_dir: &Path) -> anyhow::Result<PyProjectConfig> {
    let pyproject_path = project_dir.join("pyproject.toml");
    if !pyproject_path.exists() {
        anyhow::bail!("pyproject.toml not found in {}", project_dir.display());
    }

    let content = std::fs::read_to_string(&pyproject_path)?;
    let table: toml::Table = content.parse()?;

    // [project] section
    let project = table
        .get("project")
        .and_then(|v| v.as_table())
        .ok_or_else(|| anyhow::anyhow!("Missing [project] section in pyproject.toml"))?;

    let name = project
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing project.name in pyproject.toml"))?
        .to_string();

    let version = project
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing project.version in pyproject.toml"))?
        .to_string();

    let description = project
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let requires_python = project
        .get("requires-python")
        .and_then(|v| v.as_str())
        .unwrap_or(">=3.10")
        .to_string();

    let has_dependencies = project
        .get("dependencies")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);

    // [tool.cloaca] section
    let cloaca_config = table
        .get("tool")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("cloaca"))
        .and_then(|v| v.as_table())
        .ok_or_else(|| anyhow::anyhow!("Missing [tool.cloaca] section in pyproject.toml"))?;

    let entry_module = cloaca_config
        .get("entry_module")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing tool.cloaca.entry_module in pyproject.toml"))?
        .to_string();

    Ok(PyProjectConfig {
        name,
        version,
        description,
        requires_python,
        entry_module,
        has_dependencies,
    })
}

/// Copy the workflow source tree (entry module's top-level package) to staging.
fn copy_workflow_source(project_dir: &Path, dest: &Path, entry_module: &str) -> anyhow::Result<()> {
    let top_package = entry_module
        .split('.')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid entry_module: {}", entry_module))?;

    let src = project_dir.join(top_package);

    if src.is_dir() {
        let dest_pkg = dest.join(top_package);
        copy_dir_recursive(&src, &dest_pkg)?;
    } else {
        // Single-file module
        let src_file = project_dir.join(format!("{}.py", top_package));
        std::fs::create_dir_all(dest)?;
        if src_file.exists() {
            std::fs::copy(&src_file, dest.join(src_file.file_name().unwrap()))?;
        } else {
            anyhow::bail!(
                "Entry module source not found: neither {} nor {}.py exists",
                src.display(),
                top_package
            );
        }
    }

    Ok(())
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            // Skip __pycache__ and hidden directories
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "__pycache__" || name_str.starts_with('.') {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// UV platform mapping (matches Python vendoring.py)
fn uv_platform(target: &str) -> anyhow::Result<&'static str> {
    match target {
        "linux-x86_64" => Ok("x86_64-unknown-linux-gnu"),
        "linux-arm64" => Ok("aarch64-unknown-linux-gnu"),
        "macos-x86_64" => Ok("x86_64-apple-darwin"),
        "macos-arm64" => Ok("aarch64-apple-darwin"),
        _ => anyhow::bail!("No uv platform mapping for target: {}", target),
    }
}

/// Vendor dependencies using `uv` subprocess.
///
/// Returns the path to the generated requirements.lock file, if any.
fn vendor_dependencies(
    project_dir: &Path,
    vendor_dir: &Path,
    targets: &[String],
    verbose: bool,
) -> anyhow::Result<Option<PathBuf>> {
    // Check uv is available
    let uv_check = Command::new("uv").arg("--version").output();
    match uv_check {
        Err(_) => anyhow::bail!(
            "uv is not installed. Install with: curl -LsSf https://astral.sh/uv/install.sh | sh"
        ),
        Ok(output) if !output.status.success() => {
            anyhow::bail!("uv --version failed")
        }
        Ok(output) => {
            if verbose {
                let version = String::from_utf8_lossy(&output.stdout);
                tracing::info!("Using {}", version.trim());
            }
        }
    }

    let pyproject_path = project_dir.join("pyproject.toml");
    let target = &targets[0];
    let platform = uv_platform(target)?;

    // 1. Resolve dependencies
    let lock_path = tempfile::NamedTempFile::new()?
        .into_temp_path()
        .to_path_buf();
    let resolve = Command::new("uv")
        .args([
            "pip",
            "compile",
            pyproject_path.to_str().unwrap(),
            "--output-file",
            lock_path.to_str().unwrap(),
            "--python-version",
            "3.11",
            "--python-platform",
            platform,
            "--generate-hashes",
        ])
        .output()?;

    if !resolve.status.success() {
        let stderr = String::from_utf8_lossy(&resolve.stderr);
        anyhow::bail!("uv pip compile failed:\n{}", stderr);
    }

    // Check if there are any resolved deps
    let lock_content = std::fs::read_to_string(&lock_path)?;
    let has_deps = lock_content
        .lines()
        .any(|l| !l.trim().is_empty() && !l.starts_with('#'));

    if !has_deps {
        return Ok(None);
    }

    // 2. Download wheels
    let wheel_dir = tempfile::TempDir::new()?;
    let download = Command::new("uv")
        .args([
            "pip",
            "download",
            "-r",
            lock_path.to_str().unwrap(),
            "--dest",
            wheel_dir.path().to_str().unwrap(),
            "--python-platform",
            platform,
            "--python-version",
            "3.11",
            "--only-binary",
            ":all:",
        ])
        .output()?;

    if !download.status.success() {
        let stderr = String::from_utf8_lossy(&download.stderr);
        anyhow::bail!("uv pip download failed:\n{}", stderr);
    }

    // 3. Extract wheels into vendor dir
    for entry in std::fs::read_dir(wheel_dir.path())? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("whl") {
            extract_wheel(&path, vendor_dir)?;
        }
    }

    // 4. Copy lock file
    let final_lock = vendor_dir.parent().unwrap().join("requirements.lock");
    std::fs::copy(&lock_path, &final_lock)?;

    Ok(Some(final_lock))
}

/// Extract a .whl (zip) file into the vendor directory.
fn extract_wheel(whl_path: &Path, vendor_dir: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(whl_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let out_path = vendor_dir.join(entry.mangled_name());

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }

    Ok(())
}

/// Create a `.cloacina` tar.gz archive.
fn create_archive(
    archive_path: &Path,
    manifest_path: &Path,
    workflow_dir: &Path,
    vendor_dir: &Path,
    lock_file: Option<&Path>,
) -> anyhow::Result<()> {
    let file = std::fs::File::create(archive_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(enc);

    // manifest.json at top level
    builder.append_path_with_name(manifest_path, "manifest.json")?;

    // workflow/ directory
    builder.append_dir_all("workflow", workflow_dir)?;

    // vendor/ directory
    builder.append_dir_all("vendor", vendor_dir)?;

    // requirements.lock (if exists)
    if let Some(lock) = lock_file {
        if lock.exists() {
            builder.append_path_with_name(lock, "requirements.lock")?;
        }
    }

    let enc = builder.into_inner()?;
    enc.finish()?;

    Ok(())
}

/// Compute SHA256 hex digest of a file.
fn compute_sha256(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_project(dir: &Path) {
        // pyproject.toml
        std::fs::write(
            dir.join("pyproject.toml"),
            r#"[project]
name = "test-workflow"
version = "0.1.0"
description = "Test workflow"
requires-python = ">=3.10"
dependencies = []

[tool.cloaca]
entry_module = "workflow.tasks"
"#,
        )
        .unwrap();

        // workflow/tasks.py
        let pkg_dir = dir.join("workflow");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("__init__.py"), "# workflow package\n").unwrap();
        std::fs::write(
            pkg_dir.join("tasks.py"),
            r#"import cloaca

@cloaca.task(id="hello")
def hello(ctx):
    ctx.set("greeting", "world")
    return ctx
"#,
        )
        .unwrap();
    }

    #[test]
    fn test_parse_pyproject() {
        let dir = TempDir::new().unwrap();
        create_test_project(dir.path());

        let config = parse_pyproject(dir.path()).unwrap();
        assert_eq!(config.name, "test-workflow");
        assert_eq!(config.version, "0.1.0");
        assert_eq!(config.entry_module, "workflow.tasks");
        assert!(!config.has_dependencies);
    }

    #[test]
    fn test_parse_pyproject_missing_cloaca() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"x\"\nversion = \"1\"\n",
        )
        .unwrap();

        let err = parse_pyproject(dir.path()).unwrap_err();
        assert!(err.to_string().contains("[tool.cloaca]"));
    }

    #[test]
    fn test_copy_workflow_source() {
        let dir = TempDir::new().unwrap();
        create_test_project(dir.path());

        let dest = dir.path().join("staging").join("workflow");
        copy_workflow_source(dir.path(), &dest, "workflow.tasks").unwrap();

        assert!(dest.join("workflow").join("__init__.py").exists());
        assert!(dest.join("workflow").join("tasks.py").exists());
    }

    #[test]
    fn test_build_python_package_no_deps() {
        let dir = TempDir::new().unwrap();
        create_test_project(dir.path());

        let output_dir = dir.path().join("output");
        let config = PythonBuildConfig {
            project_dir: dir.path().to_path_buf(),
            output_dir: output_dir.clone(),
            targets: vec!["macos-arm64".to_string()],
            verbose: false,
        };

        let result = build_python_package(&config).unwrap();

        assert_eq!(result.package_name, "test-workflow");
        assert_eq!(result.version, "0.1.0");
        assert!(result.archive_path.exists());
        assert!(result.fingerprint.starts_with("sha256:"));

        // Verify archive contents
        let data = std::fs::read(&result.archive_path).unwrap();
        let manifest = crate::registry::loader::python_loader::peek_manifest(&data).unwrap();
        assert_eq!(manifest.package.name, "test-workflow");
        assert_eq!(manifest.language, PackageLanguage::Python);
        assert!(manifest.package.fingerprint.starts_with("sha256:"));
    }
}
