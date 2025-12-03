/*
 *  Copyright 2025 Colliery Software
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
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

pub use archive::create_package_archive;
pub use compile::compile_workflow;
pub use debug::{debug_package, extract_manifest_from_package, DebugResult, TaskDebugInfo};
pub use manifest::generate_manifest;
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

    Ok(())
}
