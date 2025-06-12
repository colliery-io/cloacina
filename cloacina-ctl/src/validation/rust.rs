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
use regex::Regex;
use semver::Version;
use std::path::PathBuf;

use crate::manifest::CargoToml;

pub fn validate_rust_crate_structure(project_path: &PathBuf) -> Result<()> {
    // Check if project path exists and is a directory
    if !project_path.exists() {
        bail!("Project path does not exist: {:?}", project_path);
    }

    if !project_path.is_dir() {
        bail!("Project path is not a directory: {:?}", project_path);
    }

    // Check for Cargo.toml - the only requirement for a valid Rust crate
    let cargo_toml_path = project_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        bail!(
            "Cargo.toml not found in project directory: {:?}",
            project_path
        );
    }

    // Let cargo handle validation of the actual source structure during build

    Ok(())
}

pub fn validate_rust_version_compatibility(cargo_toml: &CargoToml) -> Result<()> {
    // Get Rust version from rustc
    let rustc_output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .context("Failed to run rustc --version. Is Rust installed?")?;

    let rustc_version_str =
        String::from_utf8(rustc_output.stdout).context("Failed to parse rustc version output")?;

    // Parse rustc version (e.g., "rustc 1.75.0 (82e1608df 2023-12-21)")
    let rustc_version_regex =
        Regex::new(r"rustc (\d+\.\d+\.\d+)").expect("Failed to compile regex");

    let rustc_version = rustc_version_regex
        .captures(&rustc_version_str)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract version from rustc output"))?;

    println!("Current Rust version: {}", rustc_version);

    // Check if package specifies rust-version
    if let Some(package) = &cargo_toml.package {
        if let Some(required_rust_version) = &package.rust_version {
            // Compare versions
            let current =
                Version::parse(rustc_version).context("Failed to parse current Rust version")?;
            let required = Version::parse(required_rust_version)
                .context("Failed to parse required Rust version")?;

            if current < required {
                bail!(
                    "Rust version mismatch. Project requires: {}, but current version is: {}",
                    required_rust_version,
                    rustc_version
                );
            }

            println!(
                "Rust version {} satisfies requirement: {}",
                rustc_version, required_rust_version
            );
        }
    }

    Ok(())
}
