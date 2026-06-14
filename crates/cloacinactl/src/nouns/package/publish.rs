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
use tempfile::TempDir;

use crate::shared::error::CliError;
use crate::GlobalOpts;

pub async fn run(
    globals: &GlobalOpts,
    dir: &Path,
    release: bool,
    sign: Option<&Path>,
) -> Result<(), CliError> {
    // CLOACI-T-0596 / API-05: `--sign` not yet implemented; fail-hard
    // before doing the build so the operator catches the gap. See
    // package/pack.rs for the matching error message.
    if let Some(key_path) = sign {
        return Err(CliError::UserError(format!(
            "--sign {} is not yet implemented — package signing is tracked under I-0103. \
             Remove --sign and re-run, or wait for the signing-side completion.",
            key_path.display()
        )));
    }

    // Rust: cargo build. Python: no-op (nothing to compile). build::run
    // branches on [metadata].language (CLOACI-T-0665).
    super::build::run(dir, release)?;

    let tmp = TempDir::new().map_err(CliError::Io)?;
    let pkg_path = tmp.path().join("package.cloacina");
    // pack_to validates the Python layout before archiving.
    let produced = super::pack::pack_to(dir, Some(&pkg_path))?;

    super::upload::run(globals, &produced).await
}
