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
pub mod manifest_v2;
pub mod platform;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

pub use archive::create_package_archive;
pub use compile::compile_workflow;
pub use debug::{debug_package, extract_manifest_from_package, DebugResult, TaskDebugInfo};
pub use manifest::generate_manifest;
pub use manifest_v2::{
    ManifestV2, ManifestValidationError, PackageInfoV2, PackageLanguage, PythonRuntime,
    RustRuntime, TaskDefinitionV2,
};
pub use platform::{detect_current_platform, SUPPORTED_TARGETS};
pub use types::CompileOptions;
pub use types::{CargoToml, CompileResult, PackageManifest};

use anyhow::Result;
use std::path::PathBuf;

/// High-level function to package a workflow project.
///
/// This function performs the complete packaging pipeline:
/// 1. Validates the project structure and dependencies
/// 2. Compiles the workflow to a dynamic library
/// 3. Generates the package manifest
/// 4. Creates the final package archive
pub fn package_workflow(
    project_path: PathBuf,
    output_path: PathBuf,
    options: CompileOptions,
) -> Result<()> {
    // Step 1: Compile the workflow project
    let temp_so = tempfile::NamedTempFile::new()?;
    let temp_so_path = temp_so.path().to_path_buf();

    let compile_result = compile_workflow(project_path, temp_so_path, options)?;

    // Step 2: Create the package archive
    create_package_archive(&compile_result, &output_path)?;

    // Step 3: Validate the package (including FFI smoke test)
    validate_built_package(&output_path)?;

    Ok(())
}

/// Validate a built package by running it through the full PackageValidator
/// pipeline, including the FFI smoke test.
fn validate_built_package(package_path: &PathBuf) -> Result<()> {
    use crate::registry::loader::validator::PackageValidator;

    let package_data = std::fs::read(package_path)
        .map_err(|e| anyhow::anyhow!("Failed to read built package: {}", e))?;

    let validator = PackageValidator::new()
        .map_err(|e| anyhow::anyhow!("Failed to create validator: {}", e))?;

    // Run validation in a tokio runtime (the FFI smoke test is async)
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow::anyhow!("Failed to create runtime for validation: {}", e))?;

    let result = rt
        .block_on(validator.validate_package(&package_data, None))
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    if !result.is_valid {
        let errors = result.errors.join("\n  - ");
        anyhow::bail!(
            "Package validation failed:\n  - {}\n\n\
             See https://docs.cloacina.dev/explanation/packaged-workflow-validation/ \
             for troubleshooting guidance.",
            errors
        );
    }

    if !result.warnings.is_empty() {
        for warning in &result.warnings {
            tracing::warn!("Package validation warning: {}", warning);
        }
    }

    tracing::info!("Package validation passed (FFI smoke test OK)");
    Ok(())
}
