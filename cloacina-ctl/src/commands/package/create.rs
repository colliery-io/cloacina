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

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::archive::create_package_archive;
use crate::cli::Cli;
use crate::commands::compile_workflow;
use crate::utils::{should_print, LogLevel};

pub fn package_workflow(
    project_path: PathBuf,
    output: PathBuf,
    target: Option<String>,
    profile: String,
    cargo_flags: Vec<String>,
    cli: &Cli,
) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        println!("Packaging workflow project: {:?}", project_path);
    }

    // Step 1: Use compile_workflow to get .so and manifest
    let temp_so =
        tempfile::NamedTempFile::new().context("Failed to create temporary file for .so")?;
    let temp_so_path = temp_so.path().to_path_buf();

    let compile_result = compile_workflow(
        project_path,
        temp_so_path,
        target,
        profile,
        cargo_flags,
        cli,
    )?;

    // Step 2: Create package archive
    create_package_archive(&compile_result, &output, cli)?;

    if should_print(cli, LogLevel::Info) {
        println!("Package created successfully: {:?}", output);
    }

    Ok(())
}
