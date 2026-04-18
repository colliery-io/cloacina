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

use crate::shared::error::CliError;

pub fn run(dir: &Path, release: bool) -> Result<(), CliError> {
    if !dir.join("Cargo.toml").exists() {
        return Err(CliError::UserError(format!(
            "{} has no Cargo.toml",
            dir.display()
        )));
    }
    if !dir.join("package.toml").exists() {
        return Err(CliError::UserError(format!(
            "{} has no package.toml — not a cloacina package source",
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
