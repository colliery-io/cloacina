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

pub fn run(path: &Path, python_version: Option<&str>) -> Result<(), CliError> {
    if !path.exists() {
        return Err(CliError::UserError(format!(
            "{} does not exist",
            path.display()
        )));
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
        let extracted = cloacina_workflow_plugin::unpack_limit::unpack_package(path, tmp.path())
            .map_err(|e| {
                CliError::UserError(format!("failed to unpack {}: {e}", path.display()))
            })?;
        _staging = Some(tmp);
        extracted
    };

    let manifest = manifest::read_manifest(&dir)?;
    let lang = manifest::language(&manifest.metadata)?;
    match lang {
        PackageLanguage::Python => {
            manifest::validate_python_layout(&dir, &manifest.metadata)?;
            manifest::validate_python_env(
                &dir,
                &manifest.package.name,
                &manifest.metadata,
                python_version,
            )?;
            // CLOACINA-T-1073: dependencies ship as a lock file for the
            // compiler, or in vendor/.
            super::vendor::check_dependencies_ship(&dir, &manifest.package.name)?;
        }
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
        run(&pkg, None).unwrap();
    }

    #[test]
    fn rejects_python_without_workflow_dir() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_python_package(&pkg, false); // module at top level, no workflow/
        let err = run(&pkg, None).unwrap_err();
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
        let err = run(&pkg, None).unwrap_err();
        assert!(
            format!("{err:?}").contains("package_type") || format!("{err:?}").contains("unknown")
        );
    }

    fn add_metadata(pkg: &Path, lines: &str) {
        let mut text = fs::read_to_string(pkg.join("package.toml")).unwrap();
        text.push_str(lines);
        fs::write(pkg.join("package.toml"), text).unwrap();
    }

    #[test]
    fn rejects_requires_python_that_excludes_the_agent_python() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_python_package(&pkg, true);
        add_metadata(&pkg, "requires_python = \">=3.12\"\n");
        // Default: the Python of the official agent image, 3.11.
        let err = format!("{:?}", run(&pkg, None).unwrap_err());
        assert!(err.contains("requires Python >=3.12"), "{err}");
        assert!(err.contains("3.11"), "{err}");
        // The same package is valid for agents that run 3.12.
        run(&pkg, Some("3.12")).unwrap();
    }

    #[test]
    fn rejects_dependencies_vendored_for_another_python() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_python_package(&pkg, true);
        fs::create_dir_all(pkg.join("vendor/x86_64-linux")).unwrap();
        add_metadata(
            &pkg,
            "\n[metadata.vendored]\npython_version = \"3.11\"\nplatforms = [\"x86_64-linux\"]\nnative = true\n",
        );
        run(&pkg, None).unwrap();
        run(&pkg, Some("3.11")).unwrap();
        let err = format!("{:?}", run(&pkg, Some("3.12")).unwrap_err());
        assert!(err.contains("vendored for Python 3.11"), "{err}");
        assert!(err.contains("3.12"), "{err}");
    }

    #[test]
    fn rejects_a_record_whose_platform_folder_is_missing() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_python_package(&pkg, true);
        fs::create_dir_all(pkg.join("vendor/x86_64-linux")).unwrap();
        add_metadata(
            &pkg,
            "\n[metadata.vendored]\npython_version = \"3.11\"\nplatforms = [\"aarch64-linux\", \"x86_64-linux\"]\n",
        );
        let err = format!("{:?}", run(&pkg, None).unwrap_err());
        assert!(err.contains("vendor/aarch64-linux"), "{err}");
    }

    #[test]
    fn rejects_missing_path() {
        let tmp = TempDir::new().unwrap();
        let err = run(&tmp.path().join("nope"), None).unwrap_err();
        assert!(format!("{err:?}").contains("does not exist"));
    }
}
