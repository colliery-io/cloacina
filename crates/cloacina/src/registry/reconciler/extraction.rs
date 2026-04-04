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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // find_compiled_library tests
    // -----------------------------------------------------------------------

    #[test]
    fn find_compiled_library_finds_dylib_on_macos() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path();

        // Create a valid library file (no dash, starts with "lib")
        let ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };
        let lib_name = format!("libmyworkflow.{}", ext);
        std::fs::write(target_dir.join(&lib_name), b"fake library data").unwrap();

        let result = RegistryReconciler::find_compiled_library(target_dir);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let path = result.unwrap();
        assert!(path.ends_with(&lib_name));
    }

    #[test]
    fn find_compiled_library_ignores_hash_suffixed_artifacts() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path();

        let ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        // Hash-suffixed artifact (contains a dash) should be ignored
        let hashed = format!("libmyworkflow-abc123.{}", ext);
        std::fs::write(target_dir.join(&hashed), b"hashed artifact").unwrap();

        let result = RegistryReconciler::find_compiled_library(target_dir);
        assert!(result.is_err(), "Should not find hash-suffixed library");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("No compiled library"),
            "Error should mention no library found: {}",
            err_msg
        );
    }

    #[test]
    fn find_compiled_library_ignores_wrong_extension() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path();

        // A .a static library should be ignored
        std::fs::write(target_dir.join("libmyworkflow.a"), b"static lib").unwrap();
        // A .rlib should be ignored
        std::fs::write(target_dir.join("libmyworkflow.rlib"), b"rlib").unwrap();

        let result = RegistryReconciler::find_compiled_library(target_dir);
        assert!(result.is_err());
    }

    #[test]
    fn find_compiled_library_ignores_non_lib_prefix() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path();

        let ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        // File without "lib" prefix should be ignored
        std::fs::write(
            target_dir.join(format!("myworkflow.{}", ext)),
            b"no lib prefix",
        )
        .unwrap();

        let result = RegistryReconciler::find_compiled_library(target_dir);
        assert!(result.is_err());
    }

    #[test]
    fn find_compiled_library_empty_directory() {
        let tmp = TempDir::new().unwrap();
        let result = RegistryReconciler::find_compiled_library(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn find_compiled_library_nonexistent_directory() {
        let result =
            RegistryReconciler::find_compiled_library(std::path::Path::new("/nonexistent/path"));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Failed to read target directory"));
    }

    #[test]
    fn find_compiled_library_prefers_first_matching() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path();

        let ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        // Two valid libraries — should find at least one
        std::fs::write(target_dir.join(format!("libalpha.{}", ext)), b"alpha").unwrap();
        std::fs::write(target_dir.join(format!("libbeta.{}", ext)), b"beta").unwrap();

        let result = RegistryReconciler::find_compiled_library(target_dir);
        assert!(result.is_ok());
        let path = result.unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("lib"));
        assert!(name.ends_with(ext));
    }

    // -----------------------------------------------------------------------
    // rewrite_host_dependencies tests (debug builds only)
    // -----------------------------------------------------------------------

    #[cfg(debug_assertions)]
    #[test]
    fn rewrite_host_dependencies_adds_path_to_string_dep() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = r#"[package]
name = "test-pkg"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cloacina = "0.1.0"
serde = "1.0"
"#;
        std::fs::write(tmp.path().join("Cargo.toml"), cargo_toml).unwrap();

        rewrite_host_dependencies(tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        let doc: toml::Value = content.parse().unwrap();

        // cloacina dep should now be a table with both version and path
        let cloacina_dep = doc["dependencies"]["cloacina"].as_table().unwrap();
        assert!(cloacina_dep.contains_key("path"));
        assert!(cloacina_dep.contains_key("version"));
        let path_val = cloacina_dep["path"].as_str().unwrap();
        assert!(
            path_val.contains("crates/cloacina"),
            "Path should point to workspace crate: {}",
            path_val
        );

        // serde should be untouched (still a string)
        let serde_dep = &doc["dependencies"]["serde"];
        assert!(
            serde_dep.is_str(),
            "Non-cloacina deps should stay unchanged"
        );

        // workspace key should be inserted
        assert!(doc.get("workspace").is_some());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn rewrite_host_dependencies_adds_path_to_table_dep() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = r#"[package]
name = "test-pkg"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina-workflow = { version = "0.1.0", features = ["macros"] }
"#;
        std::fs::write(tmp.path().join("Cargo.toml"), cargo_toml).unwrap();

        rewrite_host_dependencies(tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        let doc: toml::Value = content.parse().unwrap();

        let dep = doc["dependencies"]["cloacina-workflow"].as_table().unwrap();
        assert!(dep.contains_key("path"));
        assert!(dep.contains_key("version"));
        // features should still be there
        assert!(dep.contains_key("features"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn rewrite_host_dependencies_preserves_existing_workspace() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = r#"[package]
name = "test-pkg"
version = "0.1.0"
edition = "2021"

[workspace]
members = []

[dependencies]
serde = "1.0"
"#;
        std::fs::write(tmp.path().join("Cargo.toml"), cargo_toml).unwrap();

        rewrite_host_dependencies(tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        let doc: toml::Value = content.parse().unwrap();

        // workspace should still exist and have members
        let ws = doc["workspace"].as_table().unwrap();
        assert!(ws.contains_key("members"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn rewrite_host_dependencies_no_cloacina_deps_is_noop() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = r#"[package]
name = "test-pkg"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
"#;
        std::fs::write(tmp.path().join("Cargo.toml"), cargo_toml).unwrap();

        rewrite_host_dependencies(tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        let doc: toml::Value = content.parse().unwrap();

        // serde should still be a string
        assert!(doc["dependencies"]["serde"].is_str());
        // workspace should be added even if no cloacina deps
        assert!(doc.get("workspace").is_some());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn rewrite_host_dependencies_missing_cargo_toml_errors() {
        let tmp = TempDir::new().unwrap();
        let result = rewrite_host_dependencies(tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Failed to read Cargo.toml"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn rewrite_host_dependencies_invalid_toml_errors() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "this is not valid toml {{{").unwrap();
        let result = rewrite_host_dependencies(tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Failed to parse Cargo.toml"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn rewrite_host_dependencies_handles_dev_and_build_deps() {
        let tmp = TempDir::new().unwrap();
        let cargo_toml = r#"[package]
name = "test-pkg"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"

[dev-dependencies]
cloacina = "0.1.0"

[build-dependencies]
cloacina-build = "0.1.0"
"#;
        std::fs::write(tmp.path().join("Cargo.toml"), cargo_toml).unwrap();

        rewrite_host_dependencies(tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        let doc: toml::Value = content.parse().unwrap();

        // dev-dependencies cloacina should have path
        let dev_dep = doc["dev-dependencies"]["cloacina"].as_table().unwrap();
        assert!(dev_dep.contains_key("path"));

        // build-dependencies cloacina-build should have path
        let build_dep = doc["build-dependencies"]["cloacina-build"]
            .as_table()
            .unwrap();
        assert!(build_dep.contains_key("path"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn host_workspace_root_returns_valid_path() {
        let root = host_workspace_root();
        // The workspace root should contain a Cargo.toml
        assert!(
            root.join("Cargo.toml").exists(),
            "Workspace root {} should contain Cargo.toml",
            root.display()
        );
    }
}
