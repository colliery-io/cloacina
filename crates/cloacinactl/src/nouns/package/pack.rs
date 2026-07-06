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

use std::path::{Path, PathBuf};

use super::manifest::{self, PackageLanguage};
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

    let produced = pack_to(dir, out)?;
    println!("{}", produced.display());
    Ok(())
}

/// Validate the package source and pack it into a `.cloacina` archive, returning
/// the produced archive path. Shared by `pack` and `publish`.
///
/// CLOACI-T-0665: branches on `[metadata].language`. Python packages are
/// validated for the server-expected `workflow/` layout up front so a
/// mis-laid-out module fails here rather than at upload. (Reading the manifest
/// through `CloacinaMetadata` also rejects `package_type` / `[[metadata.triggers]]`.)
pub fn pack_to(dir: &Path, out: Option<&Path>) -> Result<PathBuf, CliError> {
    let meta = manifest::read_metadata(dir)?;
    let lang = manifest::language(&meta)?;
    match lang {
        PackageLanguage::Python => manifest::validate_python_layout(dir, &meta)?,
        PackageLanguage::Rust => manifest::validate_rust_layout(dir)?,
    }
    manifest::lint_footguns(dir, lang, &meta)?;

    // CLOACI-T-0735: fidius's pack re-parses the manifest with its strict
    // header schema, and the archive carries package.toml verbatim. When the
    // on-disk manifest is MINIMAL (resolver-defaulted), stage a copy with the
    // RESOLVED manifest so (a) fidius accepts it and (b) every produced
    // archive carries the fully-resolved form — consumers never depend on
    // resolution having happened.
    let raw = std::fs::read_to_string(dir.join("package.toml"))
        .map_err(|e| CliError::UserError(format!("read package.toml: {e}")))?;
    let resolved = cloacina_workflow_plugin::manifest::resolve_manifest_str(&raw, dir)
        .map_err(CliError::UserError)?;
    let raw_value: toml::Value = toml::from_str(&raw)
        .map_err(|e| CliError::UserError(format!("invalid package.toml: {e}")))?;

    let produced = if raw_value == resolved {
        fidius_core::package::pack_package(dir, out)
            .map_err(|e| CliError::UserError(format!("pack_package failed: {e}")))?
    } else {
        let stage =
            tempfile::tempdir().map_err(|e| CliError::UserError(format!("staging dir: {e}")))?;
        let stage_dir = stage.path().join(
            dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "package".to_string()),
        );
        copy_package_tree(dir, &stage_dir)?;
        std::fs::write(
            stage_dir.join("package.toml"),
            toml::to_string_pretty(&resolved)
                .map_err(|e| CliError::UserError(format!("serialize manifest: {e}")))?,
        )
        .map_err(|e| CliError::UserError(format!("write resolved manifest: {e}")))?;
        // Default output is the CURRENT directory (same as the unstaged
        // path), so staging doesn't change where the archive lands.
        fidius_core::package::pack_package(&stage_dir, out)
            .map_err(|e| CliError::UserError(format!("pack_package failed: {e}")))?
    };

    Ok(produced.path)
}

/// Recursive copy of a package source tree into `dst`, skipping build
/// output/VCS dirs that don't belong in an archive (`target`, `.git`,
/// `node_modules`, `__pycache__`) and any prior archives.
fn copy_package_tree(src: &Path, dst: &Path) -> Result<(), CliError> {
    std::fs::create_dir_all(dst).map_err(|e| CliError::UserError(format!("staging copy: {e}")))?;
    for entry in
        std::fs::read_dir(src).map_err(|e| CliError::UserError(format!("staging copy: {e}")))?
    {
        let entry = entry.map_err(|e| CliError::UserError(format!("staging copy: {e}")))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if matches!(
            name_str.as_ref(),
            "target" | ".git" | "node_modules" | "__pycache__"
        ) || name_str.ends_with(".cloacina")
        {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_package_tree(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| CliError::UserError(format!("staging copy {name_str}: {e}")))?;
        }
    }
    Ok(())
}
