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

use crate::shared::error::CliError;

pub fn run(dir: &Path, out: Option<&Path>, sign: Option<&Path>) -> Result<(), CliError> {
    if !dir.join("package.toml").exists() {
        return Err(CliError::UserError(format!(
            "{} has no package.toml — not a cloacina package source",
            dir.display()
        )));
    }

    let produced = fidius_core::package::pack_package(dir, out)
        .map_err(|e| CliError::UserError(format!("pack_package failed: {e}")))?;

    if let Some(key_path) = sign {
        // Signature support lives behind cloacina::security::package_signer. It
        // produces a sidecar .sig file at <archive>.sig.
        eprintln!(
            "note: signing via {} — detached sig side-car not yet wired in T-0514 (tracked under \
             spec Open Items).",
            key_path.display()
        );
    }

    println!("{}", produced.path.display());
    Ok(())
}
