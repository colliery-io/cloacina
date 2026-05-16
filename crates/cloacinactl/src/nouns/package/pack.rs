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
    // CLOACI-T-0596 / API-05: `--sign` is part of the public surface but
    // the CLI doesn't yet drive `cloacina::security::package_signer`.
    // Previously this silently ignored the flag with an `eprintln!` —
    // operators thought they were producing signed packages. Now we
    // fail-hard so the gap is visible. When the signing path lands,
    // replace this with the actual side-car generation.
    if let Some(key_path) = sign {
        return Err(CliError::UserError(format!(
            "--sign {} is not yet implemented — package signing is tracked under I-0103. \
             Remove --sign and re-run, or wait for the signing-side completion.",
            key_path.display()
        )));
    }

    if !dir.join("package.toml").exists() {
        return Err(CliError::UserError(format!(
            "{} has no package.toml — not a cloacina package source",
            dir.display()
        )));
    }

    let produced = fidius_core::package::pack_package(dir, out)
        .map_err(|e| CliError::UserError(format!("pack_package failed: {e}")))?;

    println!("{}", produced.path.display());
    Ok(())
}
