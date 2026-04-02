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

//! Source package compilation — unpacks a bzip2 tar source archive and compiles
//! it to a cdylib using `cargo build`.

use std::path::{Path, PathBuf};
use tracing::debug;

use super::RegistryReconciler;
use crate::registry::error::RegistryError;

/// Cloacina crates whose path dependencies should be rewritten to host paths
/// in debug builds. Maps crate name → subpath from workspace root.
#[cfg(debug_assertions)]
const HOST_CRATES: &[(&str, &str)] = &[
    ("cloacina", "crates/cloacina"),
    ("cloacina-macros", "crates/cloacina-macros"),
    ("cloacina-workflow", "crates/cloacina-workflow"),
    (
        "cloacina-workflow-plugin",
        "crates/cloacina-workflow-plugin",
    ),
    ("cloacina-build", "crates/cloacina-build"),
];

/// Returns the host workspace root, derived from `CARGO_MANIFEST_DIR` at compile time.
/// `CARGO_MANIFEST_DIR` for the cloacina crate is `<root>/crates/cloacina`.
#[cfg(debug_assertions)]
fn host_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR should have parent (crates/)")
        .parent()
        .expect("crates/ should have parent (workspace root)")
        .to_path_buf()
}

/// Rewrite path dependencies in an extracted source package's Cargo.toml
/// to point to the host's workspace crates. Debug builds only.
///
/// This solves the chicken-and-egg problem: source packages need cloacina
/// crates to compile, but we can't publish them to crates.io before testing.
/// In debug mode, we inject the host's local workspace paths so everything
/// resolves without requiring published crates.
#[cfg(debug_assertions)]
fn rewrite_host_dependencies(source_dir: &Path) -> Result<(), RegistryError> {
    let cargo_toml_path = source_dir.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml_path).map_err(|e| {
        RegistryError::RegistrationFailed {
            message: format!(
                "Failed to read Cargo.toml at {}: {}",
                cargo_toml_path.display(),
                e
            ),
        }
    })?;

    let mut doc: toml::Value =
        content
            .parse::<toml::Value>()
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to parse Cargo.toml: {}", e),
            })?;

    let workspace_root = host_workspace_root();
    let dep_tables = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut modified = false;

    for table_name in &dep_tables {
        if let Some(table) = doc.get_mut(table_name).and_then(|v| v.as_table_mut()) {
            for &(crate_name, crate_subpath) in HOST_CRATES {
                if let Some(dep_value) = table.get_mut(crate_name) {
                    let abs_path = workspace_root.join(crate_subpath);
                    let abs_path_str = abs_path.to_string_lossy().to_string();

                    match dep_value {
                        toml::Value::Table(dep_table) => {
                            dep_table.insert("path".to_string(), toml::Value::String(abs_path_str));
                            modified = true;
                        }
                        toml::Value::String(_version) => {
                            let mut dep_table = toml::map::Map::new();
                            dep_table.insert("version".to_string(), dep_value.clone());
                            dep_table.insert("path".to_string(), toml::Value::String(abs_path_str));
                            *dep_value = toml::Value::Table(dep_table);
                            modified = true;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Ensure bare [workspace] exists to prevent parent workspace lookup
    if doc.get("workspace").is_none() {
        if let Some(table) = doc.as_table_mut() {
            table.insert(
                "workspace".to_string(),
                toml::Value::Table(toml::map::Map::new()),
            );
            modified = true;
        }
    }

    if modified {
        let new_content =
            toml::to_string_pretty(&doc).map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to serialize modified Cargo.toml: {}", e),
            })?;

        std::fs::write(&cargo_toml_path, new_content).map_err(|e| {
            RegistryError::RegistrationFailed {
                message: format!("Failed to write modified Cargo.toml: {}", e),
            }
        })?;

        debug!(
            "Rewrote host dependencies in {} (workspace root: {})",
            cargo_toml_path.display(),
            workspace_root.display()
        );
    }

    Ok(())
}

impl RegistryReconciler {
    /// Compile a Rust source package directory to a cdylib.
    ///
    /// Runs `cargo build --lib` in `source_dir` (using `--release` in release
    /// builds and `--debug` in debug builds so the wire format matches), then
    /// returns the path to the compiled library.
    ///
    /// In debug builds, path dependencies on cloacina crates are rewritten to
    /// point to the host's workspace. This enables testing source packages
    /// before crates are published to crates.io.
    pub(super) async fn compile_source_package(
        source_dir: &Path,
    ) -> Result<PathBuf, RegistryError> {
        // In debug builds, inject host workspace paths so path deps resolve.
        // Compiled out entirely in release builds.
        #[cfg(debug_assertions)]
        rewrite_host_dependencies(source_dir)?;

        // Mirror the host's build profile so the fidius wire format (JSON in
        // debug, bincode in release) matches between host and dylib.
        let profile_args: &[&str] = if cfg!(debug_assertions) {
            &["build", "--lib"]
        } else {
            &["build", "--lib", "--release"]
        };

        debug!(
            "Compiling source package at {} with args: {:?}",
            source_dir.display(),
            profile_args
        );

        let output = tokio::process::Command::new("cargo")
            .args(profile_args)
            .current_dir(source_dir)
            .output()
            .await
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to invoke cargo: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RegistryError::RegistrationFailed {
                message: format!("Compilation failed:\n{}", stderr),
            });
        }

        let target_subdir = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };

        let target_dir = source_dir.join("target").join(target_subdir);
        Self::find_compiled_library(&target_dir)
    }

    /// Search `target_dir` for the cdylib produced by `cargo build --lib`.
    ///
    /// Looks for a file whose name starts with `lib`, has the platform extension
    /// (`.dylib` on macOS, `.so` on Linux), and contains no `-` in the name
    /// (excluding Cargo hash-suffixed artifacts).
    fn find_compiled_library(target_dir: &Path) -> Result<PathBuf, RegistryError> {
        let ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        let entries =
            std::fs::read_dir(target_dir).map_err(|e| RegistryError::RegistrationFailed {
                message: format!(
                    "Failed to read target directory {}: {}",
                    target_dir.display(),
                    e
                ),
            })?;

        for entry in entries {
            let entry = entry.map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to read directory entry: {}", e),
            })?;

            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some(ext) {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                // Cargo hash-suffixed dylibs contain a `-` (e.g. `libfoo-abc123.dylib`)
                if name.starts_with("lib") && !name.contains('-') {
                    debug!("Found compiled library: {}", path.display());
                    return Ok(path);
                }
            }
        }

        Err(RegistryError::RegistrationFailed {
            message: format!(
                "No compiled library (.{}) found in {}",
                ext,
                target_dir.display()
            ),
        })
    }
}
