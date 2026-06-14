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

//! `cloacinactl package validate <path>` — check a package against the
//! canonical format without uploading (CLOACI-T-0679 / I-0119).
//!
//! Accepts either a source directory or a packed `.cloacina` archive. Surfaces
//! the same problems the server would reject — closed `[metadata]` schema
//! (unknown keys, `package_type`, `[[metadata.triggers]]`, missing `language`),
//! plus the language-specific layout (`workflow/` + `entry_module` for Python;
//! `Cargo.toml` + `src/lib.rs` for Rust) — so authors catch them locally.

use std::path::Path;

use tempfile::TempDir;

use super::manifest::{self, PackageLanguage};
use crate::shared::error::CliError;

pub fn run(path: &Path) -> Result<(), CliError> {
    if !path.exists() {
        return Err(CliError::UserError(format!("{} does not exist", path.display())));
    }

    // A directory is validated in place; anything else is treated as a packed
    // archive and unpacked into a temp dir first. The TempDir is held until the
    // end of the function so the extracted tree outlives the checks.
    let _staging: Option<TempDir>;
    let dir = if path.is_dir() {
        _staging = None;
        path.to_path_buf()
    } else {
        let tmp = TempDir::new().map_err(CliError::Io)?;
        let extracted =
            fidius_core::package::unpack_package(path, tmp.path()).map_err(|e| {
                CliError::UserError(format!("failed to unpack {}: {e}", path.display()))
            })?;
        _staging = Some(tmp);
        extracted
    };

    let manifest = manifest::read_manifest(&dir)?;
    let lang = manifest::language(&manifest.metadata)?;
    match lang {
        PackageLanguage::Python => manifest::validate_python_layout(&dir, &manifest.metadata)?,
        PackageLanguage::Rust => manifest::validate_rust_layout(&dir)?,
    }
    manifest::lint_footguns(&dir, lang, &manifest.metadata)?;

    let lang_str = match lang {
        PackageLanguage::Python => "python",
        PackageLanguage::Rust => "rust",
    };
    println!(
        "ok: {} {} ({}) — {} package is valid",
        manifest.package.name,
        manifest.package.version,
        lang_str,
        if path.is_dir() { "source" } else { "archive" }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_python_package(dir: &Path, entry_under_workflow: bool) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("package.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "python"
workflow_name = "demo"
entry_module = "demo.tasks"
"#,
        )
        .unwrap();
        let mod_dir = if entry_under_workflow {
            dir.join("workflow/demo")
        } else {
            dir.join("demo")
        };
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(mod_dir.join("__init__.py"), "").unwrap();
        fs::write(mod_dir.join("tasks.py"), "import cloaca\n").unwrap();
    }

    #[test]
    fn validates_good_python_source_dir() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_python_package(&pkg, true);
        run(&pkg).unwrap();
    }

    #[test]
    fn rejects_python_without_workflow_dir() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_python_package(&pkg, false); // module at top level, no workflow/
        let err = run(&pkg).unwrap_err();
        assert!(format!("{err:?}").contains("workflow/"));
    }

    #[test]
    fn rejects_package_type_unknown_key() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("package.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "python"
workflow_name = "demo"
entry_module = "demo.tasks"
package_type = "workflow"
"#,
        )
        .unwrap();
        let err = run(&pkg).unwrap_err();
        assert!(format!("{err:?}").contains("package_type") || format!("{err:?}").contains("unknown"));
    }

    #[test]
    fn rejects_missing_path() {
        let tmp = TempDir::new().unwrap();
        let err = run(&tmp.path().join("nope")).unwrap_err();
        assert!(format!("{err:?}").contains("does not exist"));
    }
}
