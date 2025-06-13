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
use semver::{Version, VersionReq};
use std::fs;
use std::path::PathBuf;

use crate::manifest::{CargoToml, DependencySpec, CLOACINA_VERSION};

pub fn validate_cloacina_compatibility(cargo_toml: &CargoToml) -> Result<()> {
    let dependencies = cargo_toml
        .dependencies
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No dependencies found in Cargo.toml"))?;

    // Check for cloacina dependency
    let cloacina_dep = dependencies.get("cloacina").ok_or_else(|| {
        anyhow::anyhow!(
            "Missing 'cloacina' dependency. Add:\n\n[dependencies]\ncloacina = \"{}\"",
            CLOACINA_VERSION
        )
    })?;

    // Extract version requirement
    let version_req = match cloacina_dep {
        DependencySpec::Simple(version) => version.clone(),
        DependencySpec::Detailed { version, path, .. } => {
            match (version, path) {
                (Some(v), _) => v.clone(),
                (None, Some(_)) => {
                    // Path dependency - assume it's compatible with current version
                    println!("Using path dependency for cloacina (assuming compatible)");
                    format!(">= {}", CLOACINA_VERSION)
                }
                (None, None) => bail!("cloacina dependency must specify either version or path"),
            }
        }
    };

    // Parse current cloacina version
    let current_version = Version::parse(CLOACINA_VERSION).with_context(|| {
        format!(
            "Failed to parse current cloacina version: {}",
            CLOACINA_VERSION
        )
    })?;

    // Parse dependency version requirement
    let version_req = VersionReq::parse(&version_req).with_context(|| {
        format!(
            "Failed to parse cloacina dependency version: {}",
            version_req
        )
    })?;

    // Check if current version satisfies requirement
    if !version_req.matches(&current_version) {
        bail!(
            "cloacina version mismatch. Project requires: {}, but cloacina-ctl is version: {}",
            version_req,
            current_version
        );
    }

    // Additional semver compatibility check for runtime
    // A 0.2.x build will run on 0.3.0 runtime (forward compatible on minor versions)
    println!(
        "cloacina dependency found: {} (compatible with {})",
        version_req, current_version
    );

    // Check for cloacina-macros dependency
    if let Some(macros_dep) = dependencies.get("cloacina-macros") {
        let macros_version = match macros_dep {
            DependencySpec::Simple(version) => version.clone(),
            DependencySpec::Detailed { version, .. } => {
                version.as_ref().unwrap_or(&"*".to_string()).clone()
            }
        };
        println!("cloacina-macros dependency found: {}", macros_version);
    }

    Ok(())
}

pub fn validate_packaged_workflow_presence(project_path: &PathBuf) -> Result<()> {
    let src_path = project_path.join("src");

    // Regex to find #[packaged_workflow] macro usage (with or without attributes, including multiline)
    let packaged_workflow_regex =
        Regex::new(r"(?s)#\[packaged_workflow(?:\([^)]*\))?\]").expect("Failed to compile regex");

    let mut found_macro = false;

    // Walk through all .rs files in src directory
    for entry in std::fs::read_dir(&src_path)
        .with_context(|| format!("Failed to read src directory: {:?}", src_path))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;

            if packaged_workflow_regex.is_match(&content) {
                found_macro = true;
                println!(
                    "Found #[packaged_workflow] macro in: {:?}",
                    path.file_name().unwrap()
                );
                break;
            }
        }
    }

    if !found_macro {
        bail!(
            "No #[packaged_workflow] macro found in source files.\n\n\
            Make sure at least one module is annotated with #[packaged_workflow]:\n\n\
            #[packaged_workflow]\n\
            mod my_workflow {{\n\
                // workflow tasks here\n\
            }}"
        );
    }

    Ok(())
}

pub fn check_cloacina_version_compatibility(package_version: &str) -> String {
    // Parse the package's cloacina version
    let package_ver = match Version::parse(package_version) {
        Ok(v) => v,
        Err(_) => return "unknown".to_string(),
    };

    // Parse current cloacina version
    let current_ver = match Version::parse(CLOACINA_VERSION) {
        Ok(v) => v,
        Err(_) => return "unknown".to_string(),
    };

    // Check compatibility using semver rules
    if package_ver.major != current_ver.major {
        "incompatible (major version mismatch)".to_string()
    } else if package_ver.minor == current_ver.minor {
        "compatible".to_string()
    } else if package_ver.minor < current_ver.minor {
        "compatible (forward compatible)".to_string()
    } else {
        "incompatible (requires newer runtime)".to_string()
    }
}
