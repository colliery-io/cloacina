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

//! Shared `package.toml` reading for the package verbs (CLOACI-T-0665).
//!
//! Cloacina declares a package's language in `[metadata].language` (the closed
//! `CloacinaMetadata` schema), not in fidius's `[package].runtime` field. The
//! `build` / `pack` / `publish` verbs branch on that to decide whether to invoke
//! `cargo` (Rust) or simply archive the source tree (Python). Parsing through
//! `CloacinaMetadata` also rejects `package_type` / `[[metadata.triggers]]` at
//! pack time (`#[serde(deny_unknown_fields)]`) rather than at server upload.

use std::path::Path;

use cloacina_workflow_plugin::CloacinaMetadata;

use crate::shared::error::CliError;

/// The package's source language, read from `[metadata].language`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageLanguage {
    Rust,
    Python,
}

/// Read and validate `[metadata]` from `<dir>/package.toml` against the closed
/// `CloacinaMetadata` schema. Surfaces schema errors (unknown keys such as
/// `package_type`, a `[[metadata.triggers]]` table, missing `language`) as a
/// user-facing CLI error.
pub fn read_metadata(dir: &Path) -> Result<CloacinaMetadata, CliError> {
    if !dir.join("package.toml").exists() {
        return Err(CliError::UserError(format!(
            "{} has no package.toml — not a cloacina package source",
            dir.display()
        )));
    }
    let manifest = fidius_core::package::load_manifest::<CloacinaMetadata>(dir)
        .map_err(|e| CliError::UserError(format!("invalid package.toml: {e}")))?;
    Ok(manifest.metadata)
}

/// Resolve the package language from `[metadata].language`.
pub fn language(meta: &CloacinaMetadata) -> Result<PackageLanguage, CliError> {
    match meta.language.as_str() {
        "rust" => Ok(PackageLanguage::Rust),
        "python" => Ok(PackageLanguage::Python),
        other => Err(CliError::UserError(format!(
            "unknown [metadata].language \"{other}\" — expected \"rust\" or \"python\""
        ))),
    }
}

/// Read `package.toml` and return its language in one step.
pub fn read_language(dir: &Path) -> Result<PackageLanguage, CliError> {
    language(&read_metadata(dir)?)
}

/// Validate that a Python package's source is laid out the way the server
/// loader expects: a `workflow/` directory at the package root, with the
/// `entry_module` dotted path resolving to a module under it. Catches the
/// "top-level module" footgun (`Missing workflow source directory` at upload)
/// at pack time instead.
pub fn validate_python_layout(dir: &Path, meta: &CloacinaMetadata) -> Result<(), CliError> {
    let entry = meta.entry_module.as_deref().ok_or_else(|| {
        CliError::UserError(
            "python package requires `entry_module` in [metadata] (dotted path relative to workflow/)"
                .to_string(),
        )
    })?;

    let workflow = dir.join("workflow");
    if !workflow.is_dir() {
        return Err(CliError::UserError(format!(
            "python package is missing its `workflow/` source directory ({} not found). \
             The module tree must live under workflow/ — a top-level module is rejected \
             by the server loader (\"Missing workflow source directory\").",
            workflow.display()
        )));
    }

    // `entry_module` is a dotted path relative to workflow/, e.g.
    // "data_pipeline.tasks" -> workflow/data_pipeline/tasks.py (a module) or
    // workflow/data_pipeline/tasks/__init__.py (a package).
    let mut base = workflow.clone();
    for part in entry.split('.') {
        base = base.join(part);
    }
    let as_module = base.with_extension("py");
    let as_package = base.join("__init__.py");
    if as_module.exists() || as_package.exists() {
        return Ok(());
    }

    Err(CliError::UserError(format!(
        "entry_module \"{entry}\" does not resolve under workflow/ — expected {} or {}. \
         entry_module is a dotted path relative to workflow/.",
        as_module.display(),
        as_package.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn py_meta(entry: Option<&str>) -> CloacinaMetadata {
        CloacinaMetadata {
            workflow_name: Some("data_pipeline".to_string()),
            graph_name: None,
            language: "python".to_string(),
            description: None,
            author: None,
            requires_python: None,
            entry_module: entry.map(str::to_string),
            reaction_mode: None,
            input_strategy: None,
            accumulators: Vec::new(),
        }
    }

    #[test]
    fn language_parses_known_values() {
        let mut m = py_meta(None);
        assert_eq!(language(&m).unwrap(), PackageLanguage::Python);
        m.language = "rust".to_string();
        assert_eq!(language(&m).unwrap(), PackageLanguage::Rust);
        m.language = "node".to_string();
        assert!(language(&m).is_err());
    }

    #[test]
    fn python_layout_ok_when_module_file_present() {
        let tmp = TempDir::new().unwrap();
        let module = tmp.path().join("workflow/data_pipeline");
        fs::create_dir_all(&module).unwrap();
        fs::write(module.join("tasks.py"), b"# tasks").unwrap();

        let meta = py_meta(Some("data_pipeline.tasks"));
        validate_python_layout(tmp.path(), &meta).unwrap();
    }

    #[test]
    fn python_layout_ok_when_package_init_present() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("workflow/data_pipeline");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("__init__.py"), b"# pkg").unwrap();

        // entry_module points at the package itself
        let meta = py_meta(Some("data_pipeline"));
        validate_python_layout(tmp.path(), &meta).unwrap();
    }

    #[test]
    fn python_layout_rejects_missing_workflow_dir() {
        let tmp = TempDir::new().unwrap();
        // top-level module layout — the documented footgun
        let module = tmp.path().join("data_pipeline");
        fs::create_dir_all(&module).unwrap();
        fs::write(module.join("tasks.py"), b"# tasks").unwrap();

        let meta = py_meta(Some("data_pipeline.tasks"));
        let err = validate_python_layout(tmp.path(), &meta).unwrap_err();
        assert!(format!("{err:?}").contains("workflow/"));
    }

    #[test]
    fn python_layout_rejects_missing_entry_module() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("workflow")).unwrap();
        let meta = py_meta(None);
        let err = validate_python_layout(tmp.path(), &meta).unwrap_err();
        assert!(format!("{err:?}").contains("entry_module"));
    }

    #[test]
    fn python_layout_rejects_unresolvable_entry_module() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("workflow/data_pipeline")).unwrap();
        // entry points at a module file that doesn't exist
        let meta = py_meta(Some("data_pipeline.missing"));
        let err = validate_python_layout(tmp.path(), &meta).unwrap_err();
        assert!(format!("{err:?}").contains("does not resolve"));
    }
}
