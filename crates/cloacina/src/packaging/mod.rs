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

//! Workflow packaging functionality for creating distributable workflow packages.
//!
//! This module provides the core library functions for compiling and packaging
//! workflow projects into distributable archives. These functions can be used
//! by CLI tools, tests, or other applications that need to package workflows.

pub mod archive;
pub mod compile;
pub mod debug;
pub mod manifest;
pub mod manifest_schema;
pub mod platform;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

pub use archive::create_package_archive;
pub use compile::compile_workflow;
pub use debug::{debug_package, extract_manifest_from_package, DebugResult, TaskDebugInfo};
pub use manifest::generate_manifest;
pub use manifest_schema::{
    Manifest, ManifestValidationError, PackageInfo, PackageLanguage, PythonRuntime, RustRuntime,
    TaskDefinition, TriggerDefinition,
};
pub use platform::{detect_current_platform, SUPPORTED_TARGETS};
pub use types::{CargoToml, CompileOptions, CompileResult};

use anyhow::{bail, Result};
use std::path::PathBuf;

/// High-level function to package a workflow project using fidius source packaging.
///
/// This function performs the packaging pipeline:
/// 1. Validates the project structure (Cargo.toml, src/, cdylib crate type)
/// 2. Verifies that a `package.toml` exists in the project directory
/// 3. Calls `fidius_core::package::pack_package` to create the bzip2 tar archive
pub fn package_workflow(project_path: PathBuf, output_path: PathBuf) -> Result<()> {
    // Step 1: Validate the project structure
    validation::validate_rust_crate_structure(&project_path)?;
    let cargo_toml = validation::validate_cargo_toml(&project_path)?;
    validation::validate_cloacina_compatibility(&cargo_toml)?;
    validation::validate_packaged_workflow_presence(&project_path)?;

    // Step 2: Verify package.toml exists
    let package_toml_path = project_path.join("package.toml");
    if !package_toml_path.exists() {
        bail!(
            "package.toml not found in project directory: {:?}. \
            Create a package.toml with [package] name, version, interface, interface_version, \
            and extension = \"cloacina\" fields.",
            project_path
        );
    }

    // Step 3: Pack the source package using fidius
    fidius_core::package::pack_package(&project_path, Some(&output_path))
        .map_err(|e| anyhow::anyhow!("Failed to pack package: {}", e))?;

    Ok(())
}
