/*
 *  Copyright 2026 Colliery Software
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

//! Build execution: unpack source → cargo (or language-appropriate) build →
//! return the compiled cdylib bytes. Called per-claim from the compiler's
//! main loop.

use std::path::{Path, PathBuf};

use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use tempfile::TempDir;
use tracing::{debug, info, warn};

use crate::config::CompilerConfig;

/// Result of a single build attempt.
pub enum BuildOutcome {
    Success(Vec<u8>),
    Failed(String),
}

/// Execute a build for the given package id.
///
/// Fetches source bytes from the registry, unpacks them, runs the
/// language-appropriate build step, and returns the produced cdylib bytes
/// (empty for pure-Python packages) or an error tail.
pub async fn execute_build(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_id: uuid::Uuid,
    config: &CompilerConfig,
) -> BuildOutcome {
    match run_build(registry, package_id, config).await {
        Ok(bytes) => BuildOutcome::Success(bytes),
        Err(e) => BuildOutcome::Failed(e),
    }
}

async fn run_build(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_id: uuid::Uuid,
    config: &CompilerConfig,
) -> Result<Vec<u8>, String> {
    let (meta, source_bytes) = registry
        .get_source_for_build(package_id)
        .await
        .map_err(|e| format!("failed to load source for {package_id}: {e}"))?
        .ok_or_else(|| format!("package {package_id} disappeared between claim and build"))?;

    let tmp_root = config.tmp_root_or_default();
    std::fs::create_dir_all(&tmp_root)
        .map_err(|e| format!("failed to ensure tmp_root {}: {e}", tmp_root.display()))?;

    let work =
        TempDir::new_in(&tmp_root).map_err(|e| format!("failed to create build tmpdir: {e}"))?;

    let archive_path = work.path().join("pkg.cloacina");
    std::fs::write(&archive_path, &source_bytes)
        .map_err(|e| format!("failed to stage archive: {e}"))?;

    let extract_dir = work.path().join("source");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("failed to prepare extract dir: {e}"))?;

    let archive_path_inner = archive_path.clone();
    let extract_dir_inner = extract_dir.clone();
    let source_dir = tokio::task::spawn_blocking(move || {
        fidius_core::package::unpack_package(&archive_path_inner, &extract_dir_inner)
    })
    .await
    .map_err(|e| format!("unpack task panicked: {e}"))?
    .map_err(|e| format!("fidius_core::unpack_package failed: {e}"))?;

    let manifest = load_manifest(&source_dir)?;
    let language = manifest_language(&manifest);

    info!(
        %package_id,
        package_name = %meta.package_name,
        version = %meta.version,
        language = %language,
        "starting build"
    );

    let bytes = match language.as_str() {
        "python" => {
            debug!("pure-Python package: skipping cargo build, using empty artifact");
            Vec::new()
        }
        _ => cargo_build(&source_dir, config)?,
    };

    info!(
        %package_id,
        artifact_bytes = bytes.len(),
        "build succeeded"
    );
    Ok(bytes)
}

fn load_manifest(source_dir: &Path) -> Result<toml::Value, String> {
    let manifest_path = source_dir.join("package.toml");
    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {e}", manifest_path.display()))?;
    let value: toml::Value = toml::from_str(&raw)
        .map_err(|e| format!("failed to parse {}: {e}", manifest_path.display()))?;
    Ok(value)
}

fn manifest_language(manifest: &toml::Value) -> String {
    manifest
        .get("package")
        .and_then(|p| p.get("language"))
        .and_then(|v| v.as_str())
        .unwrap_or("rust")
        .to_ascii_lowercase()
}

fn cargo_build(source_dir: &Path, config: &CompilerConfig) -> Result<Vec<u8>, String> {
    const MAX_ERR: usize = 64 * 1024;

    let mut cmd = std::process::Command::new("cargo");
    cmd.args(&config.cargo_flags).current_dir(source_dir);
    // Share a cargo target cache across builds so transitive deps
    // (cloacina-workflow, cloacina-macros, tokio, …) compile once and are
    // reused by every subsequent package. Without this, every build is a
    // cold ~120-crate compile.
    if let Some(target_dir) = &config.cargo_target_dir {
        std::fs::create_dir_all(target_dir).map_err(|e| {
            format!(
                "failed to create cargo_target_dir {}: {e}",
                target_dir.display()
            )
        })?;
        cmd.env("CARGO_TARGET_DIR", target_dir);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to spawn cargo: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail = if stderr.len() > MAX_ERR {
            let start = stderr.len() - MAX_ERR;
            stderr[start..].to_string()
        } else {
            stderr.to_string()
        };
        warn!(status = ?output.status, "cargo build failed");
        return Err(format!("cargo build failed:\n{tail}"));
    }

    let target_subdir = profile_for_flags(&config.cargo_flags);
    let target_root = config
        .cargo_target_dir
        .clone()
        .unwrap_or_else(|| source_dir.join("target"));
    let target_dir = target_root.join(target_subdir);
    // With a shared cargo_target_dir, multiple packages' cdylibs coexist.
    // Match on the Cargo.toml [package].name to pick the right one.
    let pkg_name = read_cargo_package_name(source_dir)?;
    let lib_path = find_cdylib(&target_dir, &pkg_name)?;
    let bytes = std::fs::read(&lib_path).map_err(|e| {
        format!(
            "failed to read compiled library {}: {e}",
            lib_path.display()
        )
    })?;

    Ok(bytes)
}

fn profile_for_flags(flags: &[String]) -> &'static str {
    if flags.iter().any(|f| f == "--release") {
        "release"
    } else {
        "debug"
    }
}

fn find_cdylib(target_dir: &Path, pkg_name: &str) -> Result<PathBuf, String> {
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    // Cargo normalizes `-` to `_` in the emitted libfoo.dylib.
    let normalized = pkg_name.replace('-', "_");
    let expected = format!("lib{}.{}", normalized, ext);

    let candidate = target_dir.join(&expected);
    if candidate.exists() {
        return Ok(candidate);
    }

    Err(format!(
        "expected {} in {} (built package: {})",
        expected,
        target_dir.display(),
        pkg_name
    ))
}

fn read_cargo_package_name(source_dir: &Path) -> Result<String, String> {
    let cargo_toml = source_dir.join("Cargo.toml");
    let raw = std::fs::read_to_string(&cargo_toml)
        .map_err(|e| format!("failed to read {}: {e}", cargo_toml.display()))?;
    let value: toml::Value = toml::from_str(&raw)
        .map_err(|e| format!("failed to parse {}: {e}", cargo_toml.display()))?;
    value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("[package].name missing in {}", cargo_toml.display()))
}
