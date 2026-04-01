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

impl RegistryReconciler {
    /// Compile a Rust source package directory to a cdylib.
    ///
    /// Runs `cargo build --lib` in `source_dir` (using `--release` in release
    /// builds and `--debug` in debug builds so the wire format matches), then
    /// returns the path to the compiled library.
    pub(super) async fn compile_source_package(
        source_dir: &Path,
    ) -> Result<PathBuf, RegistryError> {
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
