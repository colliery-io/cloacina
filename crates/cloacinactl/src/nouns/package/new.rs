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
//! source tree (CLOACI-T-0678 / T-0680, I-0119).
//!
//! Emits the server-accepted layout so `package pack` + upload work without
//! hand-assembly. `--kind` selects the package shape:
//! - `workflow` — `@cloaca.task` / `#[workflow]` tasks.
//! - `graph` — a computation graph (`ComputationGraphBuilder` / `#[computation_graph]`).
//! - `cron` — a workflow fired by a cron `#[trigger(on, cron)]` (Rust only;
//!   Python has no cron trigger — it uses poll `@cloaca.trigger`).
//!
//! Python packages use bare decorators (the loader builds the workflow/graph
//! context from `workflow_name`/`graph_name`); Rust packages depend on the
//! published `cloacina-*` crates, not the in-repo path deps.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScaffoldKind {
    Workflow,
    Graph,
    Cron,
}

pub fn run(
    name: &str,
    lang: ScaffoldLang,
    kind: ScaffoldKind,
    path: Option<&Path>,
) -> Result<(), CliError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CliError::UserError(
            "package name must not be empty".to_string(),
        ));
    }
    // The Python module / Rust workflow identifier derived from the package
    // name: hyphens aren't valid identifiers, so map them to underscores.
    let module = name.replace('-', "_");

    // Python has no cron trigger — poll triggers only.
    if lang == ScaffoldLang::Python && kind == ScaffoldKind::Cron {
        return Err(CliError::UserError(
            "cron triggers are Rust-only. Python packages use poll triggers — add a \
             `@cloaca.trigger(name=..., poll_interval=...)` to a `--kind workflow` package."
                .to_string(),
        ));
    }

    let dir = path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(name));

    if dir.exists()
        && fs::read_dir(&dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        return Err(CliError::UserError(format!(
            "{} already exists and is not empty",
            dir.display()
        )));
    }

    match lang {
        ScaffoldLang::Python => scaffold_python(&dir, name, &module, kind)?,
        ScaffoldLang::Rust => scaffold_rust(&dir, name, &module, kind)?,
    }

    println!(
        "created {} {} package at {}",
        lang_label(lang),
        kind_label(kind),
        dir.display()
    );
    println!("next: cloacinactl package validate {}", dir.display());
    Ok(())
}

fn lang_label(lang: ScaffoldLang) -> &'static str {
    match lang {
        ScaffoldLang::Python => "python",
        ScaffoldLang::Rust => "rust",
    }
}

fn kind_label(kind: ScaffoldKind) -> &'static str {
    match kind {
        ScaffoldKind::Workflow => "workflow",
        ScaffoldKind::Graph => "graph",
        ScaffoldKind::Cron => "cron",
    }
}

/// `my_graph` -> `MyGraph` for a reactor class/type name.
fn pascal_case(module: &str) -> String {
    module
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn write(path: &Path, contents: &str) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(CliError::Io)?;
    }
    fs::write(path, contents).map_err(CliError::Io)
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

fn scaffold_python(
    dir: &Path,
    name: &str,
    module: &str,
    kind: ScaffoldKind,
) -> Result<(), CliError> {
    match kind {
        ScaffoldKind::Workflow => scaffold_python_workflow(dir, name, module),
        ScaffoldKind::Graph => scaffold_python_graph(dir, name, module),
        ScaffoldKind::Cron => unreachable!("python cron rejected in run()"),
    }
}

fn scaffold_python_workflow(dir: &Path, name: &str, module: &str) -> Result<(), CliError> {
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
    write(&dir.join(format!("workflow/{module}/__init__.py")), "")?;
    write(&dir.join(format!("workflow/{module}/tasks.py")), tasks_py)?;
    Ok(())
}

fn scaffold_python_graph(dir: &Path, name: &str, module: &str) -> Result<(), CliError> {
    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "python"
graph_name = "{module}"
entry_module = "{module}.graph"
description = "{name} computation graph"
requires_python = ">=3.10"
reaction_mode = "when_any"
input_strategy = "latest"
"#
    );

    let reactor_cls = format!("{}Reactor", pascal_case(module));
    let graph_py = format!(
        r#"import cloaca


# Accumulators receive boundary events and feed the graph's input nodes.
@cloaca.passthrough_accumulator
def input_stream(event):
    return event


# The reactor names the graph's accumulators and the firing criteria.
@cloaca.reactor(
    name="{module}",
    accumulators=["input_stream"],
    mode="when_any",
)
class {reactor_cls}:
    pass


# Bare ComputationGraphBuilder context — the loader binds the graph to
# graph_name from package.toml. @cloaca.node functions register at import.
with cloaca.ComputationGraphBuilder(
    "{module}",
    reactor={reactor_cls},
    graph={{
        "decide": {{
            "inputs": ["input_stream"],
            "routes": {{"Go": "act", "Stop": "halt"}},
        }},
        "act": {{}},
        "halt": {{}},
    }},
) as builder:

    @cloaca.node
    def decide(input_stream):
        if input_stream and input_stream.get("value", 0) > 0:
            return ("Go", {{"value": input_stream["value"]}})
        return ("Stop", {{"reason": "no positive value"}})

    @cloaca.node
    def act(signal):
        return {{"acted": True, "value": signal["value"]}}

    @cloaca.node
    def halt(reason):
        return {{"halted": True, "reason": reason["reason"]}}
"#
    );

    write(&dir.join("package.toml"), &package_toml)?;
    write(&dir.join(format!("workflow/{module}/__init__.py")), "")?;
    write(&dir.join(format!("workflow/{module}/graph.py")), &graph_py)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Rust
// ---------------------------------------------------------------------------

fn scaffold_rust(dir: &Path, name: &str, module: &str, kind: ScaffoldKind) -> Result<(), CliError> {
    write(&dir.join("Cargo.toml"), &rust_cargo_toml(name))?;
    write(
        &dir.join("build.rs"),
        "fn main() {\n    cloacina_build::configure();\n}\n",
    )?;
    match kind {
        ScaffoldKind::Workflow => {
            write(
                &dir.join("package.toml"),
                &rust_package_toml(name, module, false),
            )?;
            write(&dir.join("src/lib.rs"), &rust_workflow_lib(name, module))?;
        }
        ScaffoldKind::Cron => {
            write(
                &dir.join("package.toml"),
                &rust_package_toml(name, module, false),
            )?;
            write(&dir.join("src/lib.rs"), &rust_cron_lib(name, module))?;
        }
        ScaffoldKind::Graph => {
            write(
                &dir.join("package.toml"),
                &rust_graph_package_toml(name, module),
            )?;
            write(&dir.join("src/lib.rs"), &rust_graph_lib(module))?;
        }
    }
    Ok(())
}

fn rust_cargo_toml(name: &str) -> String {
    format!(
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
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
async-trait = "0.1"
futures = "0.3"

[build-dependencies]
cloacina-build = "{ver}"
"#,
        ver = CLOACINA_CRATE_VERSION
    )
}

fn rust_package_toml(name: &str, module: &str, _graph: bool) -> String {
    format!(
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
    )
}

fn rust_graph_package_toml(name: &str, module: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "rust"
graph_name = "{module}"
description = "{name} computation graph"
reaction_mode = "when_any"
input_strategy = "latest"
"#
    )
}

fn rust_workflow_lib(name: &str, module: &str) -> String {
    format!(
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
    )
}

fn rust_cron_lib(name: &str, module: &str) -> String {
    // A cron trigger binds to its workflow via `on` and is driven by the cron
    // scheduler — it is NOT listed in `#[workflow(triggers = [...])]` (that list
    // is for poll-trigger subscriptions). `package validate` enforces this.
    format!(
        r#"use cloacina_macros::{{task, trigger, workflow}};
use cloacina_workflow::{{Context, TaskError}};

cloacina_workflow_plugin::package!();

// Fires `{module}` every 5 minutes (6-field cron, leading seconds).
#[trigger(on = "{module}", cron = "0 */5 * * * *")]
pub async fn {module}_cron() {{}}

#[workflow(name = "{module}", description = "{name} workflow")]
pub mod {module}_wf {{
    use super::*;

    #[task(id = "step", dependencies = [])]
    pub async fn step(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {{
        context.insert("ran", serde_json::json!(true))?;
        Ok(())
    }}
}}
"#
    )
}

fn rust_graph_lib(module: &str) -> String {
    let reactor = format!("{}Reactor", pascal_case(module));
    format!(
        r#"use serde::{{Deserialize, Serialize}};

cloacina_workflow_plugin::package!();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {{
    pub value: f64,
}}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoSignal {{
    pub value: f64,
}}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopReason {{
    pub reason: String,
}}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActResult {{
    pub acted: bool,
}}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaltResult {{
    pub halted: bool,
}}

#[cloacina_macros::reactor(
    name = "{module}_reactor",
    accumulators = [input_stream],
    criteria = when_any(input_stream),
)]
pub struct {reactor};

#[cloacina_macros::computation_graph(
    trigger = reactor("{module}_reactor"),
    graph = {{
        decide(input_stream) => {{
            Go -> act,
            Stop -> halt,
        }},
    }}
)]
pub mod {module} {{
    use super::*;

    #[derive(Debug, Clone)]
    pub enum Decision {{
        Go(GoSignal),
        Stop(StopReason),
    }}

    pub async fn decide(input_stream: Option<&Input>) -> Decision {{
        match input_stream {{
            Some(i) if i.value > 0.0 => Decision::Go(GoSignal {{ value: i.value }}),
            _ => Decision::Stop(StopReason {{
                reason: "no positive value".to_string(),
            }}),
        }}
    }}

    pub async fn act(signal: &GoSignal) -> ActResult {{
        ActResult {{ acted: signal.value > 0.0 }}
    }}

    pub async fn halt(_reason: &StopReason) -> HaltResult {{
        HaltResult {{ halted: true }}
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn python_workflow_scaffold_canonical_layout() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("data-pipeline");
        run(
            "data-pipeline",
            ScaffoldLang::Python,
            ScaffoldKind::Workflow,
            Some(&dir),
        )
        .unwrap();

        assert!(dir.join("workflow/data_pipeline/__init__.py").exists());
        let tasks = fs::read_to_string(dir.join("workflow/data_pipeline/tasks.py")).unwrap();
        assert!(tasks.contains("@cloaca.task"));
        assert!(!tasks.contains("WorkflowBuilder"));

        let manifest = fs::read_to_string(dir.join("package.toml")).unwrap();
        assert!(manifest.contains("workflow_name = \"data_pipeline\""));
        assert!(manifest.contains("entry_module = \"data_pipeline.tasks\""));
    }

    #[test]
    fn python_graph_scaffold_canonical_layout() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-graph");
        run(
            "my-graph",
            ScaffoldLang::Python,
            ScaffoldKind::Graph,
            Some(&dir),
        )
        .unwrap();

        let graph = fs::read_to_string(dir.join("workflow/my_graph/graph.py")).unwrap();
        assert!(graph.contains("ComputationGraphBuilder"));
        assert!(graph.contains("@cloaca.reactor"));
        assert!(graph.contains("class MyGraphReactor"));

        let manifest = fs::read_to_string(dir.join("package.toml")).unwrap();
        assert!(manifest.contains("graph_name = \"my_graph\""));
        assert!(manifest.contains("entry_module = \"my_graph.graph\""));
        assert!(!manifest.contains("workflow_name"));
    }

    #[test]
    fn rust_workflow_scaffold_canonical_layout() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-workflow");
        run(
            "my-workflow",
            ScaffoldLang::Rust,
            ScaffoldKind::Workflow,
            Some(&dir),
        )
        .unwrap();

        let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(cargo.contains("cloacina-workflow = { version = \"0.7\""));
        assert!(!cargo.contains("__WORKSPACE__"));
        assert!(!cargo.contains("path ="));

        let lib = fs::read_to_string(dir.join("src/lib.rs")).unwrap();
        assert!(lib.contains("cloacina_workflow_plugin::package!()"));
        assert!(lib.contains("pub mod my_workflow_wf"));
    }

    #[test]
    fn rust_cron_scaffold_binds_via_on_not_triggers_list() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("nightly");
        run(
            "nightly",
            ScaffoldLang::Rust,
            ScaffoldKind::Cron,
            Some(&dir),
        )
        .unwrap();

        let lib = fs::read_to_string(dir.join("src/lib.rs")).unwrap();
        assert!(lib.contains("#[trigger(on = \"nightly\", cron ="));
        // The cron trigger must NOT be in the workflow's triggers list.
        assert!(!lib.contains("triggers = ["));
    }

    #[test]
    fn rust_graph_scaffold_sets_graph_name_and_macros() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-graph");
        run(
            "my-graph",
            ScaffoldLang::Rust,
            ScaffoldKind::Graph,
            Some(&dir),
        )
        .unwrap();

        let manifest = fs::read_to_string(dir.join("package.toml")).unwrap();
        assert!(manifest.contains("graph_name = \"my_graph\""));
        assert!(!manifest.contains("workflow_name"));

        let lib = fs::read_to_string(dir.join("src/lib.rs")).unwrap();
        assert!(lib.contains("cloacina_macros::reactor"));
        assert!(lib.contains("cloacina_macros::computation_graph"));
        assert!(lib.contains("pub struct MyGraphReactor"));
    }

    #[test]
    fn python_cron_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let err = run(
            "x",
            ScaffoldLang::Python,
            ScaffoldKind::Cron,
            Some(tmp.path()),
        )
        .unwrap_err();
        assert!(format!("{err:?}").contains("Rust-only"));
    }

    #[test]
    fn refuses_non_empty_existing_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("taken");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("something"), b"x").unwrap();

        let err = run(
            "taken",
            ScaffoldLang::Python,
            ScaffoldKind::Workflow,
            Some(&dir),
        )
        .unwrap_err();
        assert!(format!("{err:?}").contains("not empty"));
    }

    #[test]
    fn rejects_empty_name() {
        let tmp = TempDir::new().unwrap();
        let err = run(
            "  ",
            ScaffoldLang::Python,
            ScaffoldKind::Workflow,
            Some(tmp.path()),
        )
        .unwrap_err();
        assert!(format!("{err:?}").contains("must not be empty"));
    }
}
