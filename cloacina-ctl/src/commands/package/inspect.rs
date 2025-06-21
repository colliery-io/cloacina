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

use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::archive::extract_manifest_from_package;
use crate::cli::Cli;
use crate::manifest::PackageManifest;
use crate::utils::{should_print, LogLevel};
use crate::validation::check_cloacina_version_compatibility;

pub fn inspect_package(package_path: PathBuf, format: String, cli: &Cli) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        println!("Inspecting package: {:?}", package_path);
    }

    // Step 1: Validate package file exists
    if !package_path.exists() {
        bail!("Package file does not exist: {:?}", package_path);
    }

    if !package_path.is_file() {
        bail!("Package path is not a file: {:?}", package_path);
    }

    // Step 2: Extract manifest.json from package
    let manifest = extract_manifest_from_package(&package_path)?;

    // Step 3: Output based on format
    match format.as_str() {
        "json" => output_manifest_json(&manifest, cli)?,
        "human" => output_manifest_human(&manifest, &package_path, cli)?,
        _ => bail!("Unsupported format: {}. Use 'json' or 'human'", format),
    }

    Ok(())
}

fn output_manifest_json(manifest: &PackageManifest, cli: &Cli) -> Result<()> {
    let json_output =
        serde_json::to_string_pretty(manifest).context("Failed to serialize manifest to JSON")?;

    if should_print(cli, LogLevel::Info) {
        println!("{}", json_output);
    }

    Ok(())
}

fn output_manifest_human(
    manifest: &PackageManifest,
    package_path: &PathBuf,
    cli: &Cli,
) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        println!("Package Information:");
        println!("  File: {}", package_path.display());
        println!("  Package: {}", manifest.package.name);
        println!("  Version: {}", manifest.package.version);
        println!("  Description: {}", manifest.package.description);
        let compatibility =
            check_cloacina_version_compatibility(&manifest.package.cloacina_version);
        println!(
            "  Cloacina Version: {} ({})",
            manifest.package.cloacina_version, compatibility
        );
        println!();

        println!("Library:");
        println!("  File: {}", manifest.library.filename);
        println!("  Architecture: {}", manifest.library.architecture);
        println!("  Symbols: {:?}", manifest.library.symbols);
        println!();

        if manifest.tasks.is_empty() {
            println!("Tasks: None defined");
        } else {
            println!("Tasks ({}):", manifest.tasks.len());
            for task in &manifest.tasks {
                println!("  {}: {}", task.index, task.id);
                if !task.dependencies.is_empty() {
                    println!("     Dependencies: {:?}", task.dependencies);
                } else {
                    println!("     Dependencies: []");
                }
                if !task.source_location.is_empty() {
                    println!("     Source: {}", task.source_location);
                }
                println!();
            }
        }

        if !manifest.execution_order.is_empty() {
            println!("Execution Order: {}", manifest.execution_order.join(" → "));
        } else {
            println!("Execution Order: Not defined");
        }
    }

    Ok(())
}
