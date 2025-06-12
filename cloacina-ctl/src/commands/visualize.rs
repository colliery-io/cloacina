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

use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::archive::extract_manifest_from_package;
use crate::cli::Cli;
use crate::utils::{should_print, LogLevel};
use crate::visualization::{generate_ascii_visualization, generate_dot_visualization};

pub fn visualize_package(
    package_path: PathBuf,
    details: bool,
    layout: String,
    format: String,
    cli: &Cli,
) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        println!("Visualizing package: {:?}", package_path);
    }

    // Step 1: Validate package file exists
    if !package_path.exists() {
        bail!("Package file does not exist: {:?}", package_path);
    }

    if !package_path.is_file() {
        bail!("Package path is not a file: {:?}", package_path);
    }

    // Step 2: Extract manifest from package
    let manifest = extract_manifest_from_package(&package_path)?;

    // Step 3: Generate visualization based on format
    match format.as_str() {
        "ascii" => generate_ascii_visualization(&manifest, &layout, details, cli)?,
        "dot" => generate_dot_visualization(&manifest, cli)?,
        _ => bail!("Unsupported format: {}. Use 'ascii' or 'dot'", format),
    }

    Ok(())
}
