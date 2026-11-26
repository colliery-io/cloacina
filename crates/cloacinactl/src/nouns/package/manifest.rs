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

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use cloacina_workflow_plugin::CloacinaMetadata;

use crate::shared::error::CliError;

/// The package's source language, read from `[metadata].language`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageLanguage {
    Rust,
    Python,
}

/// Read and validate `<dir>/package.toml` against the closed `CloacinaMetadata`
/// schema. Surfaces schema errors (unknown keys such as `package_type`, a
/// `[[metadata.triggers]]` table, missing `language`) as a user-facing CLI
/// error. Returns the full manifest (`[package]` header + `[metadata]`).
pub fn read_manifest(
    dir: &Path,
) -> Result<fidius_core::package::PackageManifest<CloacinaMetadata>, CliError> {
    if !dir.join("package.toml").exists() {
        return Err(CliError::UserError(format!(
            "{} has no package.toml — not a cloacina package source",
            dir.display()
        )));
    }
    // CLOACI-T-0735: the shared resolver defaults the constant fidius header
    // triple and infers language/entry_module from layout — a minimal
    // manifest (name + version + workflow_name) is valid from here on.
    cloacina_workflow_plugin::manifest::load_resolved_manifest(dir).map_err(CliError::UserError)
}

/// Read just the `[metadata]` table. See [`read_manifest`].
pub fn read_metadata(dir: &Path) -> Result<CloacinaMetadata, CliError> {
    Ok(read_manifest(dir)?.metadata)
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

/// Check the Python environment of a Python package (CLOACINA-T-1048), as
/// `validate`, `pack` and `publish` do before the server sees it:
///
/// - `requires_python` allows the Python that runs the package, and the
///   dependencies are vendored for that Python. That Python is
///   `python_version` when given, else the version in `[metadata.vendored]`,
///   else the Python of the official agent image.
/// - Each platform of `[metadata.vendored]` has its `vendor/<platform>/`
///   folder, and a record that says "no compiled code" is true.
pub fn validate_python_env(
    dir: &Path,
    package: &str,
    meta: &CloacinaMetadata,
    python_version: Option<&str>,
) -> Result<(), CliError> {
    use cloacina_workflow_plugin::python_env;

    let (python, runner) = match (python_version, meta.vendored.as_ref()) {
        (Some(v), _) => (v.to_string(), "the target (--python-version)"),
        (None, Some(v)) => (v.python_version.clone(), "the build target"),
        (None, None) => (
            python_env::DEFAULT_PYTHON_VERSION.to_string(),
            "the official agent image",
        ),
    };
    python_env::check_python(
        package,
        meta.requires_python.as_deref(),
        meta.vendored.as_ref(),
        &python,
        runner,
    )
    .map_err(CliError::UserError)?;

    if let Some(v) = &meta.vendored {
        for platform in &v.platforms {
            let folder = dir.join("vendor").join(platform);
            if !folder.is_dir() {
                return Err(CliError::UserError(format!(
                    "package.toml records vendored dependencies for {platform}, and \
                     vendor/{platform}/ does not exist. Run `cloacinactl package build` again."
                )));
            }
            if !v.native
                && let Some(file) = python_env::native_files(&folder, 1).first()
            {
                return Err(CliError::UserError(format!(
                    "package.toml records no compiled code in vendor/, and \
                     vendor/{platform}/{file} is a compiled file. Run `cloacinactl \
                     package build` again."
                )));
            }
        }
    }
    if dir.join("vendor").is_dir() {
        let limits = fidius_core::package::UnpackOptions::default();
        check_unpacked_size(dir, limits.max_decompressed, u64::from(limits.max_entries))?;
    }
    Ok(())
}

/// Refuse a package source that is larger than an archive can be unpacked
/// to. The server and the loader unpack an archive with these limits, so a
/// larger package would be refused after the upload.
fn check_unpacked_size(dir: &Path, max_bytes: u64, max_entries: u64) -> Result<(), CliError> {
    fn walk(dir: &Path, bytes: &mut u64, entries: &mut u64) {
        let Ok(read) = fs::read_dir(dir) else { return };
        for entry in read.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(
                name.as_ref(),
                "target" | ".git" | "node_modules" | "__pycache__"
            ) || name.ends_with(".cloacina")
            {
                continue;
            }
            *entries += 1;
            match entry.metadata() {
                Ok(m) if m.is_dir() => walk(&entry.path(), bytes, entries),
                Ok(m) => *bytes += m.len(),
                Err(_) => {}
            }
        }
    }
    let (mut bytes, mut entries) = (0u64, 0u64);
    walk(dir, &mut bytes, &mut entries);
    let mib = |b: u64| format!("{:.1} MiB", b as f64 / (1024.0 * 1024.0));
    if bytes > max_bytes {
        return Err(CliError::UserError(format!(
            "the package is {} unpacked, and a package archive can hold {} unpacked. \
             Vendor for fewer platforms (`cloacinactl package build --arch`), or remove \
             dependencies.",
            mib(bytes),
            mib(max_bytes)
        )));
    }
    if entries > max_entries {
        return Err(CliError::UserError(format!(
            "the package has {entries} files and folders, and a package archive can hold \
             {max_entries}. Vendor for fewer platforms (`cloacinactl package build --arch`), \
             or remove dependencies."
        )));
    }
    Ok(())
}

/// Validate that a Rust package's source is present: a `Cargo.toml` and a
/// `src/lib.rs` (the server compiles `src/` into a cdylib at load). A missing
/// `build.rs` (which calls `cloacina_build::configure()`) is surfaced as a
/// warning rather than a hard error.
pub fn validate_rust_layout(dir: &Path) -> Result<(), CliError> {
    if !dir.join("Cargo.toml").exists() {
        return Err(CliError::UserError(format!(
            "rust package is missing Cargo.toml ({} not found)",
            dir.join("Cargo.toml").display()
        )));
    }
    if !dir.join("src/lib.rs").exists() {
        return Err(CliError::UserError(format!(
            "rust package is missing src/lib.rs ({} not found) — packaged workflows \
             are compiled as a cdylib from src/lib.rs",
            dir.join("src/lib.rs").display()
        )));
    }
    // CLOACI-T-0737: build.rs is no longer part of the package shell — the
    // compiler injects the build wiring; no warning for its absence.
    Ok(())
}

/// Author-time footgun lints (CLOACI-T-0680). Run by `validate` and `pack` to
/// catch the mistakes that otherwise surface only at upload/load:
/// - an unrewritten `__WORKSPACE__` placeholder in `Cargo.toml`;
/// - a computation-graph package that doesn't declare `graph_name`;
/// - a cron trigger also listed in `#[workflow(triggers = [...])]` (cron triggers
///   bind via `on`, not via the workflow's poll-trigger subscription list).
pub fn lint_footguns(
    dir: &Path,
    lang: PackageLanguage,
    meta: &CloacinaMetadata,
) -> Result<(), CliError> {
    match lang {
        PackageLanguage::Rust => lint_rust_source(dir, meta),
        PackageLanguage::Python => lint_python_source(dir, meta),
    }
}

fn lint_rust_source(dir: &Path, meta: &CloacinaMetadata) -> Result<(), CliError> {
    if let Ok(cargo) = fs::read_to_string(dir.join("Cargo.toml")) {
        if cargo.contains("__WORKSPACE__") {
            return Err(CliError::UserError(
                "Cargo.toml contains an unrewritten `__WORKSPACE__` path placeholder — \
                 replace it with a published crate version (or a real path) before packing."
                    .to_string(),
            ));
        }
    }

    let lib = match fs::read_to_string(dir.join("src/lib.rs")) {
        Ok(s) => s,
        // A missing src/lib.rs is reported by validate_rust_layout, not here.
        Err(_) => return Ok(()),
    };

    if lib.contains("computation_graph(") && meta.graph_name.is_none() {
        return Err(CliError::UserError(
            "src/lib.rs defines a #[computation_graph] but [metadata].graph_name is unset — \
             a computation-graph package must declare graph_name."
                .to_string(),
        ));
    }

    let subscribed = workflow_trigger_names(&lib);
    for name in cron_trigger_names(&lib) {
        if subscribed.contains(&name) {
            return Err(CliError::UserError(format!(
                "cron trigger `{name}` is also listed in a #[workflow(triggers = [...])] — \
                 cron triggers bind to their workflow via `on` and must not be subscribed there. \
                 Remove `{name}` from the workflow's triggers list."
            )));
        }
    }
    Ok(())
}

fn lint_python_source(dir: &Path, meta: &CloacinaMetadata) -> Result<(), CliError> {
    let entry = match meta.entry_module.as_deref() {
        Some(e) => e,
        None => return Ok(()),
    };
    let mut base = dir.join("workflow");
    for part in entry.split('.') {
        base = base.join(part);
    }
    let src = [base.with_extension("py"), base.join("__init__.py")]
        .iter()
        .find_map(|p| fs::read_to_string(p).ok());
    let src = match src {
        Some(s) => s,
        None => return Ok(()),
    };

    let is_cg = src.contains("ComputationGraphBuilder")
        || src.contains("cloaca.reactor")
        || src.contains("@cloaca.reactor");
    if is_cg && meta.graph_name.is_none() {
        return Err(CliError::UserError(
            "the entry module uses ComputationGraphBuilder/@cloaca.reactor but \
             [metadata].graph_name is unset — a computation-graph package must declare graph_name."
                .to_string(),
        ));
    }
    Ok(())
}

/// Names of triggers declared with a `cron = "..."` argument. The name is the
/// explicit `name = "..."` attribute argument if present, else the decorated
/// function's identifier.
fn cron_trigger_names(src: &str) -> Vec<String> {
    attr_invocations(src, "trigger")
        .into_iter()
        .filter(|(args, _)| kv_value(args, "cron").is_some())
        .filter_map(|(args, fn_name)| kv_value(&args, "name").or(fn_name))
        .collect()
}

/// Trigger names listed in any `#[workflow(triggers = [...])]` attribute.
fn workflow_trigger_names(src: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    for (args, _) in attr_invocations(src, "workflow") {
        if let Some(list) = array_value(&args, "triggers") {
            for s in quoted_strings(&list) {
                set.insert(s);
            }
        }
    }
    set
}

/// Find `#[<attr>(...)]` / `#[cloacina_macros::<attr>(...)]` invocations, returning
/// each one's argument text and the identifier of the `fn` that follows it.
fn attr_invocations(src: &str, attr: &str) -> Vec<(String, Option<String>)> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    for pat in [format!("#[{attr}("), format!("#[cloacina_macros::{attr}(")] {
        let mut from = 0;
        while let Some(rel) = src[from..].find(pat.as_str()) {
            let open = from + rel + pat.len() - 1; // index of the '('
            let mut depth = 0usize;
            let mut close = None;
            let mut i = open;
            while i < bytes.len() {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            let close = match close {
                Some(c) => c,
                None => break,
            };
            let args = src[open + 1..close].to_string();
            let after = src[close..]
                .find(']')
                .map(|d| close + d + 1)
                .unwrap_or(close);
            let fn_name = src[after..].find("fn ").and_then(|p| {
                let start = after + p + 3;
                let ident: String = src[start..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                (!ident.is_empty()).then_some(ident)
            });
            out.push((args, fn_name));
            from = close + 1;
        }
    }
    out
}

/// Value of a `key = "..."` argument within an attribute's args, word-bounded so
/// `name` doesn't match `filename`.
fn kv_value(args: &str, key: &str) -> Option<String> {
    let mut from = 0;
    while let Some(rel) = args[from..].find(key) {
        let idx = from + rel;
        let prev_ok = idx == 0
            || !args[..idx]
                .chars()
                .next_back()
                .map(|c| c.is_alphanumeric() || c == '_')
                .unwrap_or(false);
        let after = &args[idx + key.len()..];
        if prev_ok {
            if let Some(eq) = after.find('=') {
                let tail = &after[eq + 1..];
                if let Some(q1) = tail.find('"') {
                    if let Some(q2) = tail[q1 + 1..].find('"') {
                        return Some(tail[q1 + 1..q1 + 1 + q2].to_string());
                    }
                }
            }
        }
        from = idx + key.len();
    }
    None
}

/// The `[...]` text of a `key = [...]` argument.
fn array_value(args: &str, key: &str) -> Option<String> {
    let idx = args.find(key)?;
    let after = &args[idx + key.len()..];
    let eq = after.find('=')?;
    let lb = after[eq..].find('[')? + eq;
    let rb = after[lb..].find(']')? + lb;
    Some(after[lb + 1..rb].to_string())
}

/// All double-quoted string literals in `s`.
fn quoted_strings(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(q1) = rest.find('"') {
        let tail = &rest[q1 + 1..];
        if let Some(q2) = tail.find('"') {
            out.push(tail[..q2].to_string());
            rest = &tail[q2 + 1..];
        } else {
            break;
        }
    }
    out
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
            providers: Default::default(),
            vendored: None,
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

    #[test]
    fn package_over_the_unpack_limits_is_refused() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("vendor/x86_64-linux")).unwrap();
        fs::write(
            tmp.path().join("vendor/x86_64-linux/a.py"),
            vec![b'x'; 2048],
        )
        .unwrap();
        fs::write(tmp.path().join("vendor/x86_64-linux/b.py"), b"x").unwrap();
        // Build output and caches do not count.
        fs::create_dir_all(tmp.path().join("target")).unwrap();
        fs::write(tmp.path().join("target/big"), vec![b'x'; 8192]).unwrap();

        check_unpacked_size(tmp.path(), 4096, 100).unwrap();
        let err = format!(
            "{:?}",
            check_unpacked_size(tmp.path(), 1024, 100).unwrap_err()
        );
        assert!(err.contains("unpacked"), "{err}");
        assert!(err.contains("--arch"), "{err}");
        // vendor, x86_64-linux, a.py, b.py = 4 entries.
        check_unpacked_size(tmp.path(), 4096, 4).unwrap();
        let err = format!(
            "{:?}",
            check_unpacked_size(tmp.path(), 4096, 3).unwrap_err()
        );
        assert!(err.contains("4 files and folders"), "{err}");
    }

    // ---- footgun lints (CLOACI-T-0680) ----

    fn meta(lang: &str, graph_name: Option<&str>, entry: Option<&str>) -> CloacinaMetadata {
        CloacinaMetadata {
            workflow_name: Some("demo".to_string()),
            graph_name: graph_name.map(str::to_string),
            language: lang.to_string(),
            description: None,
            author: None,
            requires_python: None,
            entry_module: entry.map(str::to_string),
            reaction_mode: None,
            input_strategy: None,
            accumulators: Vec::new(),
            providers: Default::default(),
            vendored: None,
        }
    }

    fn write_rust(dir: &Path, cargo: &str, lib: &str) {
        fs::write(dir.join("Cargo.toml"), cargo).unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/lib.rs"), lib).unwrap();
    }

    #[test]
    fn lint_rejects_unrewritten_workspace_placeholder() {
        let tmp = TempDir::new().unwrap();
        write_rust(
            tmp.path(),
            "cloacina-macros = { path = \"__WORKSPACE__/crates/cloacina-macros\" }",
            "// nothing",
        );
        let err = lint_footguns(tmp.path(), PackageLanguage::Rust, &meta("rust", None, None))
            .unwrap_err();
        assert!(format!("{err:?}").contains("__WORKSPACE__"));
    }

    #[test]
    fn lint_rust_cg_requires_graph_name() {
        let tmp = TempDir::new().unwrap();
        write_rust(
            tmp.path(),
            "[package]",
            "#[cloacina_macros::computation_graph(trigger = reactor(\"r\"))]\npub mod g {}",
        );
        let err = lint_footguns(tmp.path(), PackageLanguage::Rust, &meta("rust", None, None))
            .unwrap_err();
        assert!(format!("{err:?}").contains("graph_name"));
        // declaring graph_name clears it
        lint_footguns(
            tmp.path(),
            PackageLanguage::Rust,
            &meta("rust", Some("g"), None),
        )
        .unwrap();
    }

    #[test]
    fn lint_rejects_cron_trigger_in_workflow_triggers_list() {
        let tmp = TempDir::new().unwrap();
        write_rust(
            tmp.path(),
            "[package]",
            "#[trigger(on = \"wf\", cron = \"0 0 * * * *\")]\npub async fn nightly() {}\n\n\
             #[workflow(name = \"wf\", triggers = [\"nightly\"])]\npub mod wf {}",
        );
        let err = lint_footguns(tmp.path(), PackageLanguage::Rust, &meta("rust", None, None))
            .unwrap_err();
        assert!(format!("{err:?}").contains("nightly"));
    }

    #[test]
    fn lint_allows_cron_trigger_bound_via_on() {
        let tmp = TempDir::new().unwrap();
        write_rust(
            tmp.path(),
            "[package]",
            "#[trigger(on = \"wf\", cron = \"0 0 * * * *\")]\npub async fn nightly() {}\n\n\
             #[workflow(name = \"wf\")]\npub mod wf {}",
        );
        lint_footguns(tmp.path(), PackageLanguage::Rust, &meta("rust", None, None)).unwrap();
    }

    #[test]
    fn lint_python_cg_requires_graph_name() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("workflow/g");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(
            mod_dir.join("graph.py"),
            "import cloaca\nwith cloaca.ComputationGraphBuilder(\"g\"):\n    pass\n",
        )
        .unwrap();
        let err = lint_footguns(
            tmp.path(),
            PackageLanguage::Python,
            &meta("python", None, Some("g.graph")),
        )
        .unwrap_err();
        assert!(format!("{err:?}").contains("graph_name"));
    }
}
