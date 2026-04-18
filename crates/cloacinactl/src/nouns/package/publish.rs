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
    super::build::run(dir, release)?;

    let tmp = TempDir::new().map_err(CliError::Io)?;
    let pkg_path = tmp.path().join("package.cloacina");
    let produced = fidius_core::package::pack_package(dir, Some(&pkg_path))
        .map_err(|e| CliError::UserError(format!("pack_package failed: {e}")))?;

    if let Some(key_path) = sign {
        eprintln!(
            "note: signing via {} — detached sig side-car not yet wired in T-0514.",
            key_path.display()
        );
    }

    super::upload::run(globals, &produced.path).await
}
