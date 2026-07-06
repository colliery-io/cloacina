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

//! The ONE `package.toml` resolver (CLOACI-T-0735).
//!
//! A minimal cloacina manifest is `name` + `version` (+ `workflow_name`):
//! the constant fidius header triple and the layout-implied metadata are
//! defaulted/inferred here, so authors stop copying ceremony. Every parse
//! site — cloacinactl (validate/pack/publish), the compiler, and the
//! server-side registry/reconciler loaders — MUST go through this resolver;
//! per-site parsing is how the `language` drift bug (CLOACI-T-0666) happened.
//!
//! Resolution rules (explicit values always win):
//! - `[package].interface`          → `"cloacina-workflow-plugin"`
//! - `[package].interface_version`  → `1`
//! - `[package].extension`          → `"cloacina"`
//! - `[metadata].language`          → inferred from layout: `Cargo.toml` +
//!   `src/lib.rs` ⇒ `"rust"`; a `workflow/` dir ⇒ `"python"`. Both or
//!   neither present ⇒ error (ambiguity is never guessed — T-0666).
//! - `[metadata].entry_module` (python, absent) → `<name_with_underscores>.tasks`

use std::path::Path;

use crate::CloacinaMetadata;
use fidius_core::package::PackageManifest;

/// Default fidius header values for cloacina workflow packages.
pub const DEFAULT_INTERFACE: &str = "cloacina-workflow-plugin";
pub const DEFAULT_INTERFACE_VERSION: i64 = 1;
pub const DEFAULT_EXTENSION: &str = "cloacina";

/// Read `<dir>/package.toml`, apply the cloacina defaults/inference above,
/// and deserialize the RESOLVED document into the typed manifest.
pub fn load_resolved_manifest(dir: &Path) -> Result<PackageManifest<CloacinaMetadata>, String> {
    let raw = std::fs::read_to_string(dir.join("package.toml"))
        .map_err(|e| format!("read package.toml: {e}"))?;
    let resolved = resolve_manifest_str(&raw, dir)?;
    // Round-trip through fidius's loader semantics: deserialize the patched
    // document with the same typed schema every site expects.
    let manifest: PackageManifest<CloacinaMetadata> =
        toml::from_str(&toml::to_string(&resolved).map_err(|e| e.to_string())?)
            .map_err(|e| format!("invalid package.toml: {e}"))?;
    manifest
        .validate_runtime()
        .map_err(|e| format!("invalid package.toml: {e}"))?;
    Ok(manifest)
}

/// Apply defaults/inference to a parsed manifest document. `dir` is the
/// package source root used for layout inference; explicit values are never
/// overwritten.
pub fn resolve_manifest_str(raw: &str, dir: &Path) -> Result<toml::Value, String> {
    let mut doc: toml::Value =
        toml::from_str(raw).map_err(|e| format!("invalid package.toml: {e}"))?;

    let root = doc
        .as_table_mut()
        .ok_or_else(|| "package.toml root must be a table".to_string())?;

    // [package] header: constant triple.
    let pkg = root
        .entry("package")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .ok_or_else(|| "[package] must be a table".to_string())?;
    pkg.entry("interface")
        .or_insert_with(|| toml::Value::String(DEFAULT_INTERFACE.into()));
    pkg.entry("interface_version")
        .or_insert(toml::Value::Integer(DEFAULT_INTERFACE_VERSION));
    pkg.entry("extension")
        .or_insert_with(|| toml::Value::String(DEFAULT_EXTENSION.into()));
    let package_name = pkg.get("name").and_then(|v| v.as_str()).map(str::to_string);

    // [metadata]: language inference + entry_module convention.
    let meta = root
        .entry("metadata")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .ok_or_else(|| "[metadata] must be a table".to_string())?;

    if !meta.contains_key("language") {
        let looks_rust = dir.join("Cargo.toml").exists() && dir.join("src/lib.rs").exists();
        let looks_python = dir.join("workflow").is_dir();
        let lang = match (looks_rust, looks_python) {
            (true, false) => "rust",
            (false, true) => "python",
            (true, true) => {
                return Err(
                    "cannot infer [metadata].language: the layout has BOTH a Rust crate \
                     (Cargo.toml + src/lib.rs) and a Python workflow/ dir — declare \
                     language explicitly"
                        .to_string(),
                )
            }
            (false, false) => {
                return Err(
                    "cannot infer [metadata].language: no Cargo.toml + src/lib.rs (Rust) \
                     and no workflow/ dir (Python) — declare language explicitly or fix \
                     the layout"
                        .to_string(),
                )
            }
        };
        meta.insert("language".into(), toml::Value::String(lang.into()));
    }

    let is_python = meta
        .get("language")
        .and_then(|v| v.as_str())
        .map(|l| l.eq_ignore_ascii_case("python"))
        .unwrap_or(false);
    if is_python && !meta.contains_key("entry_module") {
        if let Some(name) = package_name {
            let module = name.replace('-', "_");
            meta.insert(
                "entry_module".into(),
                toml::Value::String(format!("{module}.tasks")),
            );
        }
    }

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scaffold_python(dir: &Path, module: &str) {
        std::fs::create_dir_all(dir.join("workflow").join(module)).unwrap();
    }

    #[test]
    fn minimal_python_manifest_resolves() {
        let tmp = tempfile::tempdir().unwrap();
        scaffold_python(tmp.path(), "sync_file");
        std::fs::write(
            tmp.path().join("package.toml"),
            r#"[package]
name = "sync-file"
version = "0.1.0"

[metadata]
workflow_name = "sync_file"
"#,
        )
        .unwrap();

        let m = load_resolved_manifest(tmp.path()).unwrap();
        assert_eq!(m.package.interface, DEFAULT_INTERFACE);
        assert_eq!(m.package.interface_version, 1u32);
        assert_eq!(m.package.extension.as_deref(), Some(DEFAULT_EXTENSION));
        assert_eq!(m.metadata.language, "python");
        assert_eq!(m.metadata.entry_module.as_deref(), Some("sync_file.tasks"));
    }

    #[test]
    fn minimal_rust_manifest_resolves() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\n").unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), "").unwrap();
        std::fs::write(
            tmp.path().join("package.toml"),
            r#"[package]
name = "etl"
version = "0.1.0"

[metadata]
workflow_name = "etl"
"#,
        )
        .unwrap();

        let m = load_resolved_manifest(tmp.path()).unwrap();
        assert_eq!(m.metadata.language, "rust");
        assert!(m.metadata.entry_module.is_none()); // rust: no convention default
    }

    #[test]
    fn explicit_values_always_win() {
        let tmp = tempfile::tempdir().unwrap();
        scaffold_python(tmp.path(), "x");
        std::fs::write(
            tmp.path().join("package.toml"),
            r#"[package]
name = "x"
version = "0.1.0"
interface = "custom"
interface_version = 9
extension = "zip"

[metadata]
language = "python"
entry_module = "x.graph"
workflow_name = "x"
"#,
        )
        .unwrap();

        let m = load_resolved_manifest(tmp.path()).unwrap();
        assert_eq!(m.package.interface, "custom");
        assert_eq!(m.package.interface_version, 9);
        assert_eq!(m.package.extension.as_deref(), Some("zip"));
        assert_eq!(m.metadata.entry_module.as_deref(), Some("x.graph"));
    }

    #[test]
    fn ambiguous_layout_errors() {
        let tmp = tempfile::tempdir().unwrap();
        scaffold_python(tmp.path(), "x");
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), "").unwrap();
        std::fs::write(
            tmp.path().join("package.toml"),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let err = load_resolved_manifest(tmp.path()).unwrap_err();
        assert!(err.contains("cannot infer"), "{err}");
    }
}
