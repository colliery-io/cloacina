/*
 *  Copyright 2026 Colliery Software
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

use std::path::Path;
use std::process::Command;

use super::manifest::{self, PackageLanguage};
use super::vendor::VendorArgs;
use crate::shared::error::CliError;

pub fn run(dir: &Path, release: bool, vendor: &VendorArgs) -> Result<(), CliError> {
    // A Python package has nothing to compile. Its build step vendors the
    // dependencies that pyproject.toml declares into vendor/<platform>/
    // (CLOACINA-T-1048). `publish` calls this, so a publish vendors too.
    if manifest::read_language(dir)? == PackageLanguage::Python {
        super::vendor::run(dir, vendor)?;
        println!(
            "built {} (python: nothing to compile; use `package pack` to archive it)",
            dir.display()
        );
        return Ok(());
    }

    if !dir.join("Cargo.toml").exists() {
        return Err(CliError::UserError(format!(
            "{} has no Cargo.toml",
            dir.display()
        )));
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("build").current_dir(dir);
    if release {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .map_err(|e| CliError::UserError(format!("failed to spawn cargo: {e}")))?;

    if !status.success() {
        return Err(CliError::UserError(format!(
            "cargo build exited with status {status}"
        )));
    }

    let profile = if release { "release" } else { "debug" };
    println!("built {} in {} profile", dir.display(), profile);
    Ok(())
}
