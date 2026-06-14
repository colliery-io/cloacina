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

//! `cloacinactl package new <name>` — scaffold a canonical `.cloacina` package
//! source tree (CLOACI-T-0678 / I-0119).
//!
//! Emits the server-accepted layout so `package pack` + upload work without
//! hand-assembly: Python gets `package.toml` + `workflow/<module>/` with bare
//! `@cloaca.task` decorators (the loader names the workflow from
//! `workflow_name`); Rust gets `Cargo.toml` + `build.rs` + `src/lib.rs` wired to
//! the published `cloacina-*` crates.

use std::fs;
use std::path::{Path, PathBuf};

use clap::ValueEnum;

use crate::shared::error::CliError;

/// Version pin for the generated Rust package's `cloacina-*` dependencies.
const CLOACINA_CRATE_VERSION: &str = "0.7";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScaffoldLang {
    Python,
    Rust,
}

pub fn run(name: &str, lang: ScaffoldLang, path: Option<&Path>) -> Result<(), CliError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CliError::UserError("package name must not be empty".to_string()));
    }
    // The Python module / Rust workflow identifier derived from the package
    // name: hyphens aren't valid identifiers, so map them to underscores.
    let module = name.replace('-', "_");

    let dir = path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(name));

    if dir.exists() && fs::read_dir(&dir).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(CliError::UserError(format!(
            "{} already exists and is not empty",
            dir.display()
        )));
    }

    match lang {
        ScaffoldLang::Python => scaffold_python(&dir, name, &module)?,
        ScaffoldLang::Rust => scaffold_rust(&dir, name, &module)?,
    }

    println!("created {} package at {}", lang_label(lang), dir.display());
    println!("next: cloacinactl package pack {}", dir.display());
    Ok(())
}

fn lang_label(lang: ScaffoldLang) -> &'static str {
    match lang {
        ScaffoldLang::Python => "python",
        ScaffoldLang::Rust => "rust",
    }
}

fn write(path: &Path, contents: &str) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(CliError::Io)?;
    }
    fs::write(path, contents).map_err(CliError::Io)
}

fn scaffold_python(dir: &Path, name: &str, module: &str) -> Result<(), CliError> {
    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "python"
workflow_name = "{module}"
entry_module = "{module}.tasks"
description = "{name} workflow"
requires_python = ">=3.10"
"#
    );

    // Bare @cloaca.task decorators — the packaged loader builds the workflow
    // context from workflow_name before importing this module, so do NOT wrap
    // these in a WorkflowBuilder (that is for in-process runs only).
    let tasks_py = r#"import cloaca


@cloaca.task(id="hello", dependencies=[])
def hello(context):
    context.set("hello", "world")
    return context


@cloaca.task(id="goodbye", dependencies=["hello"])
def goodbye(context):
    context.set("done", True)
    return context
"#;

    write(&dir.join("package.toml"), &package_toml)?;
    write(
        &dir.join(format!("workflow/{module}/__init__.py")),
        "",
    )?;
    write(&dir.join(format!("workflow/{module}/tasks.py")), tasks_py)?;
    Ok(())
}

fn scaffold_rust(dir: &Path, name: &str, module: &str) -> Result<(), CliError> {
    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "rust"
workflow_name = "{module}"
description = "{name} workflow"
"#
    );

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["packaged"]
packaged = []

[dependencies]
cloacina-macros = "{ver}"
cloacina-computation-graph = "{ver}"
cloacina-workflow = {{ version = "{ver}", features = ["packaged"] }}
cloacina-workflow-plugin = "{ver}"
serde_json = "1.0"
async-trait = "0.1"
futures = "0.3"

[build-dependencies]
cloacina-build = "{ver}"
"#,
        ver = CLOACINA_CRATE_VERSION
    );

    let build_rs = "fn main() {\n    cloacina_build::configure();\n}\n";

    let lib_rs = format!(
        r#"use cloacina_macros::{{task, workflow}};
use cloacina_workflow::{{Context, TaskError}};

cloacina_workflow_plugin::package!();

#[workflow(name = "{module}", description = "{name} workflow")]
pub mod {module}_wf {{
    use super::*;

    #[task(id = "hello", dependencies = [])]
    pub async fn hello(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {{
        context.insert("hello", serde_json::json!("world"))?;
        Ok(())
    }}

    #[task(id = "goodbye", dependencies = ["hello"])]
    pub async fn goodbye(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {{
        context.insert("done", serde_json::json!(true))?;
        Ok(())
    }}
}}
"#
    );

    write(&dir.join("package.toml"), &package_toml)?;
    write(&dir.join("Cargo.toml"), &cargo_toml)?;
    write(&dir.join("build.rs"), build_rs)?;
    write(&dir.join("src/lib.rs"), &lib_rs)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn python_scaffold_creates_canonical_layout() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("data-pipeline");
        run("data-pipeline", ScaffoldLang::Python, Some(&dir)).unwrap();

        assert!(dir.join("package.toml").exists());
        assert!(dir.join("workflow/data_pipeline/__init__.py").exists());
        let tasks = fs::read_to_string(dir.join("workflow/data_pipeline/tasks.py")).unwrap();
        assert!(tasks.contains("@cloaca.task"));
        assert!(!tasks.contains("WorkflowBuilder"));

        let manifest = fs::read_to_string(dir.join("package.toml")).unwrap();
        assert!(manifest.contains("language = \"python\""));
        assert!(manifest.contains("workflow_name = \"data_pipeline\""));
        assert!(manifest.contains("entry_module = \"data_pipeline.tasks\""));
    }

    #[test]
    fn rust_scaffold_creates_canonical_layout() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-workflow");
        run("my-workflow", ScaffoldLang::Rust, Some(&dir)).unwrap();

        assert!(dir.join("package.toml").exists());
        assert!(dir.join("Cargo.toml").exists());
        assert!(dir.join("build.rs").exists());
        assert!(dir.join("src/lib.rs").exists());

        let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(cargo.contains("crate-type = [\"cdylib\", \"rlib\"]"));
        assert!(cargo.contains("cloacina-workflow = { version = \"0.7\""));
        assert!(!cargo.contains("__WORKSPACE__"));
        assert!(!cargo.contains("path ="));

        let lib = fs::read_to_string(dir.join("src/lib.rs")).unwrap();
        assert!(lib.contains("cloacina_workflow_plugin::package!()"));
        assert!(lib.contains("pub mod my_workflow_wf"));
    }

    #[test]
    fn refuses_non_empty_existing_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("taken");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("something"), b"x").unwrap();

        let err = run("taken", ScaffoldLang::Python, Some(&dir)).unwrap_err();
        assert!(format!("{err:?}").contains("not empty"));
    }

    #[test]
    fn rejects_empty_name() {
        let tmp = TempDir::new().unwrap();
        let err = run("  ", ScaffoldLang::Python, Some(tmp.path())).unwrap_err();
        assert!(format!("{err:?}").contains("must not be empty"));
    }
}
