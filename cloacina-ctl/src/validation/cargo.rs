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
use std::fs;
use std::path::PathBuf;

use crate::manifest::{CargoToml, CLOACINA_VERSION};

pub fn validate_cargo_toml(project_path: &PathBuf) -> Result<CargoToml> {
    let cargo_toml_path = project_path.join("Cargo.toml");

    // Read and parse Cargo.toml
    let content = fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Failed to read Cargo.toml at {:?}", cargo_toml_path))?;

    let cargo_toml: CargoToml = toml::from_str(&content)
        .with_context(|| format!("Failed to parse Cargo.toml at {:?}", cargo_toml_path))?;

    // Check for package section
    let package = cargo_toml
        .package
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml missing [package] section"))?;

    println!("Package: {} v{}", package.name, package.version);

    // Check for lib section with cdylib crate-type
    let lib = cargo_toml.lib.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Cargo.toml missing [lib] section. Add:\n\n[lib]\ncrate-type = [\"cdylib\"]\n"
        )
    })?;

    let crate_types = lib.crate_type.as_ref()
        .ok_or_else(|| anyhow::anyhow!(
            "Cargo.toml [lib] section missing crate-type. Add:\n\n[lib]\ncrate-type = [\"cdylib\"]\n"
        ))?;

    if !crate_types.contains(&"cdylib".to_string()) {
        bail!(
            "Cargo.toml [lib] crate-type must include \"cdylib\". Current: {:?}\n\n\
            Add or update:\n\n[lib]\ncrate-type = [\"cdylib\"]\n",
            crate_types
        );
    }

    println!("Found cdylib crate-type: {:?}", crate_types);

    Ok(cargo_toml)
}
