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

use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::CargoToml;

/// Validate that the project has a valid Rust crate structure
pub fn validate_rust_crate_structure(project_path: &PathBuf) -> Result<()> {
    let cargo_toml_path = project_path.join("Cargo.toml");

    if !cargo_toml_path.exists() {
        bail!(
            "Cargo.toml not found in project directory: {:?}",
            project_path
        );
    }

    let src_dir = project_path.join("src");
    if !src_dir.exists() {
        bail!(
            "src directory not found in project directory: {:?}",
            project_path
        );
    }

    Ok(())
}

/// Parse and validate Cargo.toml
pub fn validate_cargo_toml(project_path: &Path) -> Result<CargoToml> {
    let cargo_toml_path = project_path.join("Cargo.toml");
    let cargo_toml_content = fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Failed to read Cargo.toml: {:?}", cargo_toml_path))?;

    let cargo_toml: CargoToml =
        toml::from_str(&cargo_toml_content).context("Failed to parse Cargo.toml")?;

    // Validate that it's configured as a cdylib
    if let Some(lib) = &cargo_toml.lib {
        if let Some(crate_types) = &lib.crate_type {
            if !crate_types.contains(&"cdylib".to_string()) {
                bail!(
                    "Cargo.toml must specify crate-type = [\"cdylib\"] in [lib] section for workflow compilation"
                );
            }
        } else {
            bail!("Cargo.toml must specify crate-type = [\"cdylib\"] in [lib] section");
        }
    } else {
        bail!("Cargo.toml must have a [lib] section with crate-type = [\"cdylib\"]");
    }

    Ok(cargo_toml)
}

/// Validate cloacina dependency compatibility
pub fn validate_cloacina_compatibility(cargo_toml: &CargoToml) -> Result<()> {
    let dependencies = cargo_toml
        .dependencies
        .as_ref()
        .ok_or_else(|| anyhow!("No dependencies section found in Cargo.toml"))?;

    // Check if cloacina is a dependency
    let cloacina_dep = dependencies
        .get("cloacina")
        .ok_or_else(|| anyhow!("cloacina must be a dependency"))?;

    // For now, just validate that cloacina is present
    // In the future, we could add version compatibility checks
    match cloacina_dep {
        toml::Value::String(_) => Ok(()),
        toml::Value::Table(_) => Ok(()),
        _ => bail!("Invalid cloacina dependency specification"),
    }
}

/// Check for packaged_workflow macros in the source code
pub fn validate_packaged_workflow_presence(project_path: &Path) -> Result<()> {
    let src_dir = project_path.join("src");
    let lib_rs = src_dir.join("lib.rs");
    let main_rs = src_dir.join("main.rs");

    // Check lib.rs first, then main.rs if lib.rs doesn't exist
    let source_file = if lib_rs.exists() {
        lib_rs
    } else if main_rs.exists() {
        main_rs
    } else {
        return Err(anyhow!("Neither src/lib.rs nor src/main.rs found"));
    };

    let source_content = fs::read_to_string(&source_file)
        .with_context(|| format!("Failed to read source file: {:?}", source_file))?;

    // Look for #[packaged_workflow] macro usage - just detect the beginning
    // This regex matches:
    // - #[packaged_workflow]
    // - #[packaged_workflow(
    // We don't need to match the entire macro, just verify it exists
    let workflow_regex = Regex::new(r"#\[\s*packaged_workflow\s*[\]\(]")
        .context("Failed to compile regex for packaged_workflow detection")?;

    if !workflow_regex.is_match(&source_content) {
        // Try a more permissive search for debugging
        let simple_regex = Regex::new(r"packaged_workflow")
            .context("Failed to compile simple regex for packaged_workflow detection")?;

        if simple_regex.is_match(&source_content) {
            // Found the text but not in macro format
            bail!(
                "Found 'packaged_workflow' text in {:?} but not in proper macro format. \
                Ensure you use #[packaged_workflow] or #[packaged_workflow(...)] syntax.",
                source_file
            );
        } else {
            bail!(
                "No #[packaged_workflow] macro found in {:?}. \
                Workflows must use the #[packaged_workflow] macro to be packageable.",
                source_file
            );
        }
    }

    Ok(())
}

/// Validate Rust version compatibility
pub fn validate_rust_version_compatibility(cargo_toml: &CargoToml) -> Result<()> {
    if let Some(package) = &cargo_toml.package {
        if let Some(rust_version) = &package.rust_version {
            // Parse the required Rust version
            let required_version = semver::Version::parse(rust_version)
                .with_context(|| format!("Invalid rust-version format: {}", rust_version))?;

            // For now, just validate that it's a reasonable version (1.70+)
            let min_version =
                semver::Version::parse("1.70.0").context("Failed to parse minimum Rust version")?;

            if required_version < min_version {
                bail!(
                    "Rust version {} is too old. Minimum supported version is {}",
                    required_version,
                    min_version
                );
            }
        }
    }

    Ok(())
}
