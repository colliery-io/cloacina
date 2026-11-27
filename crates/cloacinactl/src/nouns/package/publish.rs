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

use crate::GlobalOpts;
use crate::shared::error::CliError;

pub async fn run(
    globals: &GlobalOpts,
    dir: &Path,
    release: bool,
    sign: Option<&Path>,
    vendor: &super::vendor::VendorArgs,
) -> Result<(), CliError> {
    // A key file that cannot sign stops the command before the build.
    if let Some(key_file) = sign {
        super::sign::check_signing_key(key_file)?;
    }

    // Rust: cargo build. Python: lock the dependencies, and with --vendor
    // vendor them (CLOACINA-T-1048, CLOACINA-T-1073).
    // build::run branches on [metadata].language (CLOACI-T-0665).
    super::build::run(dir, release, vendor)?;

    let tmp = TempDir::new().map_err(CliError::Io)?;
    let pkg_path = tmp.path().join("package.cloacina");
    // pack_to validates the Python layout before archiving.
    let produced = super::pack::pack_to(dir, Some(&pkg_path))?;

    // CLOACINA-T-1062: `--sign` signs the archive that was just built, on
    // this machine, and the signature goes with the upload. With no `--sign`
    // the upload has no signature.
    let signature = match sign {
        Some(key_file) => Some(super::sign::UploadSignature::Made(
            super::sign::sign_archive(&produced, key_file)?,
        )),
        None => None,
    };
    super::upload::run(globals, &produced, signature).await
}
