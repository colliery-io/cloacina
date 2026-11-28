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

//! The describe step: a child process reads a built package's metadata
//! (CLOACINA-T-1074, CLOACINA-A-0019 decision 6).
//!
//! A package's metadata is in its code. Opening a Rust library runs its
//! load-time code, and importing a Python package runs the module. The
//! compiler process holds a database URL, so it does neither. It starts a
//! child, the child reads the metadata and writes one
//! [`PackageDescription`] document to a file, and the compiler stores the
//! document.
//!
//! Two halves in this file:
//!
//! - [`describe`], the compiler's half: it writes the inputs into the build's
//!   scratch folder, starts the child and reads the document back.
//! - [`run_child`], the child's half: what `cloacina-compiler describe …`
//!   runs.
//!
//! The child is the compiler binary itself, started again in describe mode,
//! so it opens a library with the same fidius version and build profile the
//! library was built for. It is a build child like `cargo` and `uv`: made by
//! `cloacina::packaging::build_child::build_child_command` with a confined
//! policy, so it gets a cleared environment (no database URL), its own
//! process group, the resource limits and the time limit `--describe-timeout-s`.
//!
//! The document goes to a file, not to the child's stdout: the package's code
//! can print.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cloacina::packaging::build_child::{
    ChildError, ChildPolicy, ChildRlimits, ChildStep, build_child_command, run_build_child,
};
use cloacina::registry::package_description::PackageDescription;

use crate::config::CompilerConfig;

/// The first argument that puts the compiler binary in describe mode.
pub const DESCRIBE_MODE: &str = "describe";

/// The tenant the child registers a Python package under. The document holds
/// no tenant: task ids are local, and templates keep `{tenant}`.
const DESCRIBE_TENANT: &str = "public";

/// The longest text of the child's stderr that goes into an error.
const MAX_ERR: usize = 16 * 1024;

/// What the child reads.
pub(crate) enum DescribeInput<'a> {
    /// A built Rust library.
    Rust {
        /// The bytes of the cdylib.
        library: &'a [u8],
    },
    /// A Python package.
    Python {
        /// The uploaded archive.
        archive: &'a [u8],
        /// The dependencies vendored for the compiler's own platform, when
        /// the registry holds a set for it.
        vendored_set: Option<&'a [u8]>,
        /// The package's bundled constructor providers: name and archive.
        providers: &'a [(String, Vec<u8>)],
    },
}

/// The confined policy of the describe child: the compiler's resource limits
/// and the describe time limit. Nothing is set by name except the unpack
/// limit, when the compiler runs with one that is not the default.
pub(crate) fn describe_policy(config: &CompilerConfig) -> ChildPolicy {
    let rlimits = ChildRlimits {
        cpu_s: config.build_rlimits.cpu_s,
        mem_bytes: config.build_rlimits.mem_bytes,
        files: config.build_rlimits.files,
        procs: config.build_rlimits.procs,
    };
    let mut policy = ChildPolicy::confined(config.describe_timeout, rlimits);
    if let Some(limit) = cloacina_workflow_plugin::unpack_limit::child_env_value() {
        policy = policy.env(
            cloacina_workflow_plugin::unpack_limit::MAX_UNPACKED_BYTES_ENV,
            limit,
        );
    }
    policy
}

/// Describe a package in a child process. `work` is the scratch folder of the
/// build. An `Err` is the reason, as the publisher reads it.
pub(crate) async fn describe(
    input: DescribeInput<'_>,
    work: &Path,
    config: &CompilerConfig,
) -> Result<PackageDescription, String> {
    let dir = work.join(format!("describe-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let out = dir.join("description.json");

    let mut args: Vec<std::ffi::OsString> = vec![DESCRIBE_MODE.into()];
    let write = |path: &Path, bytes: &[u8]| {
        std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
    };
    match input {
        DescribeInput::Rust { library } => {
            let path = dir.join(format!(
                "package.{}",
                cloacina::registry::loader::package_loader::get_library_extension()
            ));
            write(&path, library)?;
            args.push("--library".into());
            args.push(path.into());
        }
        DescribeInput::Python {
            archive,
            vendored_set,
            providers,
        } => {
            let path = dir.join("package.cloacina");
            write(&path, archive)?;
            args.push("--python-archive".into());
            args.push(path.into());
            if let Some(set) = vendored_set {
                let path = dir.join("vendored.set");
                write(&path, set)?;
                args.push("--vendored-set".into());
                args.push(path.into());
            }
            if !providers.is_empty() {
                let providers_dir = dir.join("providers");
                std::fs::create_dir_all(&providers_dir)
                    .map_err(|e| format!("create {}: {e}", providers_dir.display()))?;
                for (index, (_name, bytes)) in providers.iter().enumerate() {
                    write(&providers_dir.join(format!("{index}.cloacina")), bytes)?;
                }
                args.push("--providers-dir".into());
                args.push(providers_dir.into());
            }
        }
    }
    args.push("--out".into());
    args.push(out.clone().into());

    let policy = describe_policy(config);
    let program = config.describe_program_or_default()?;
    let mut cmd = build_child_command(&program.to_string_lossy(), &policy, ChildStep::Build);
    cmd.args(&args).current_dir(&dir);

    let limit = config.describe_timeout;
    let run_policy = policy.clone();
    let ran = tokio::task::spawn_blocking(move || run_build_child(cmd, &run_policy, true))
        .await
        .map_err(|e| format!("the describe step panicked: {e}"))?;
    let result = match ran {
        Ok(output) if output.status.success() => std::fs::read(&out)
            .map_err(|e| format!("the describe child wrote no document: {e}"))
            .and_then(|bytes| {
                serde_json::from_slice::<PackageDescription>(&bytes).map_err(|e| {
                    format!("the describe child wrote a document that does not parse: {e}")
                })
            }),
        Ok(output) => Err(format!(
            "the package could not be described ({}): {}",
            output.status,
            tail(&String::from_utf8_lossy(&output.stderr))
        )),
        Err(ChildError::TimedOut(_)) => Err(format!(
            "the describe step timed out after {limit:?} (--describe-timeout-s): reading the \
             package's metadata runs its load-time code, or its import for a Python package, \
             and that code did not finish"
        )),
        Err(e) => Err(format!(
            "the describe child `{}` could not be run: {e}",
            program.display()
        )),
    };
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// The end of a long error text.
fn tail(text: &str) -> String {
    let text = text.trim();
    if text.len() <= MAX_ERR {
        return text.to_string();
    }
    let mut start = text.len() - MAX_ERR;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_string()
}

// ---------------------------------------------------------------------------
// The child's half
// ---------------------------------------------------------------------------

/// Arguments of `cloacina-compiler describe`.
#[derive(clap::Parser, Debug)]
#[command(name = "cloacina-compiler describe")]
#[command(about = "Read the metadata of one built package and write it as a document")]
struct DescribeArgs {
    /// A built Rust library to open.
    #[arg(long, conflicts_with = "python_archive")]
    library: Option<PathBuf>,

    /// A Python package archive to import.
    #[arg(long)]
    python_archive: Option<PathBuf>,

    /// The vendored dependencies of the Python package for this platform.
    #[arg(long, requires = "python_archive")]
    vendored_set: Option<PathBuf>,

    /// A folder with the archives of the package's bundled providers.
    #[arg(long, requires = "python_archive")]
    providers_dir: Option<PathBuf>,

    /// The file the document is written to.
    #[arg(long)]
    out: PathBuf,
}

/// Run describe mode: `args` are the program's arguments after the mode word.
/// Returns the process exit code. An error goes to stderr, where the compiler
/// reads it.
pub fn run_child(args: Vec<std::ffi::OsString>) -> i32 {
    use clap::Parser;

    let mut argv: Vec<std::ffi::OsString> = vec!["cloacina-compiler describe".into()];
    argv.extend(args);
    let args = match DescribeArgs::try_parse_from(argv) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let described = match (&args.library, &args.python_archive) {
        (Some(library), None) => describe_rust(library),
        (None, Some(archive)) => describe_python(
            archive,
            args.vendored_set.as_deref(),
            args.providers_dir.as_deref(),
        ),
        _ => Err("give one of --library and --python-archive".to_string()),
    };
    let written = described.and_then(|doc| {
        let json = serde_json::to_vec(&doc).map_err(|e| format!("serialize the document: {e}"))?;
        std::fs::write(&args.out, json).map_err(|e| format!("write {}: {e}", args.out.display()))
    });
    match written {
        Ok(()) => 0,
        Err(reason) => {
            eprintln!("{reason}");
            1
        }
    }
}

fn describe_rust(library: &Path) -> Result<PackageDescription, String> {
    cloacina::registry::PackageLoader::describe_library(library).map_err(|e| e.to_string())
}

fn describe_python(
    archive: &Path,
    vendored_set: Option<&Path>,
    providers_dir: Option<&Path>,
) -> Result<PackageDescription, String> {
    use cloacina::python_runtime::PythonRuntime;

    let read =
        |path: &Path| std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()));
    let archive_data = read(archive)?;
    let vendored = vendored_set.map(read).transpose()?;

    let scratch = tempfile::TempDir::new().map_err(|e| format!("create a scratch folder: {e}"))?;
    let source_dir = cloacina_workflow_plugin::unpack_limit::unpack_package(
        archive,
        &scratch.path().join("source"),
    )
    .map_err(|e| format!("unpack the package archive: {e}"))?;
    let manifest = cloacina_workflow_plugin::manifest::load_resolved_manifest(&source_dir)
        .map_err(|e| format!("load package.toml: {e}"))?;
    if manifest.metadata.language != "python" {
        return Err(format!(
            "--python-archive is for a Python package; this one is `{}`",
            manifest.metadata.language
        ));
    }
    let package_name = manifest.package.name.clone();

    // `cloaca.constructor(...)` resolves a bundled provider during the
    // import, as it does when a server or an agent loads the package.
    let provider_root = match providers_dir {
        Some(dir) => Some(stage_providers(dir, &scratch.path().join("providers"))?),
        None => None,
    };
    let _provider_scope =
        cloacina::registry::loader::ScopedProviderSearch::for_staged_root(provider_root);

    // An empty runtime: what it holds after the import is this package's.
    let runtime = Arc::new(cloacina::Runtime::empty());
    let python = cloacina_python::CloacinaPythonRuntime;
    let python_version = python.python_version();

    // CLOACINA-T-1078: what each `cloaca.constructor(...)` declares goes into
    // the document; this process imports one package only.
    cloacina_python::constructor::record_declarations();
    let mut doc = if !manifest.metadata.has_computation_graph() {
        let loaded = python.load_workflow_package(
            &archive_data,
            vendored.as_deref(),
            &scratch.path().join("python-staging"),
            DESCRIBE_TENANT,
            &runtime,
        )?;
        PackageDescription::from_python_runtime(
            &runtime,
            &package_name,
            &loaded.workflow_name,
            &loaded.task_namespaces,
            &manifest.metadata,
            python_version,
        )
    } else {
        let (Some(graph_name), Some(entry_module)) = (
            manifest.metadata.graph_name.as_deref(),
            manifest.metadata.entry_module.as_deref(),
        ) else {
            return Err(
                "a Python computation graph package needs graph_name and entry_module in \
                 package.toml"
                    .to_string(),
            );
        };
        let mut declaration = python.load_cg_package(
            &archive_data,
            vendored.as_deref(),
            &scratch.path().join("python-cg-staging"),
            DESCRIBE_TENANT,
            graph_name,
            entry_module,
            &manifest.metadata.accumulators,
            &runtime,
        )?;
        // CLOACINA-T-1135: the manifest's input strategy, as the server's
        // in-process load applies it.
        if let Some(declaration) = declaration.as_mut() {
            cloacina::computation_graph::packaging_bridge::apply_manifest_input_strategy(
                declaration,
                manifest.metadata.input_strategy.as_deref(),
            );
        }
        let mut doc = PackageDescription::from_python_runtime(
            &runtime,
            &package_name,
            "",
            &[],
            &manifest.metadata,
            python_version,
        );
        // The declaration is what a server gives its graph scheduler: the
        // graph's data comes from it where the runtime walk has none.
        if let Some(declaration) = declaration {
            add_python_graph_declaration(
                &mut doc,
                &declaration,
                &package_name,
                &manifest.metadata.accumulators,
            );
        }
        doc
    };
    doc.constructors = cloacina_python::constructor::take_declarations();
    Ok(doc)
}

/// Put the data of a Python graph declaration into the document: the graph
/// entry when the runtime walk gave none, the strategy the declaration runs
/// with, and the node and edge topology.
fn add_python_graph_declaration(
    doc: &mut PackageDescription,
    declaration: &cloacina::computation_graph::scheduler::ComputationGraphDeclaration,
    package_name: &str,
    accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
) {
    use cloacina::computation_graph::reactor::{InputStrategy, ReactionCriteria};
    use cloacina_workflow_plugin::types::AccumulatorDeclarationEntry;

    // CLOACINA-T-1078: the authored kinds, as the graph's reactor entry has
    // them.
    let reactor_entry = declaration
        .reactor_name
        .as_deref()
        .and_then(|name| doc.reactors.iter().find(|r| r.name == name))
        .cloned();
    let graph = doc.graph.get_or_insert_with(|| {
        let accumulators = declaration
            .accumulators
            .iter()
            .map(|accumulator| {
                // The manifest entry of the accumulator, then the kind its
                // reactor was authored with, then a passthrough.
                let (accumulator_type, config) = accumulator_overrides
                    .iter()
                    .find(|cfg| cfg.name == accumulator.name)
                    .map(|cfg| (cfg.accumulator_type.clone(), cfg.config.clone()))
                    .or_else(|| {
                        reactor_entry.as_ref().and_then(|r| {
                            r.accumulators
                                .iter()
                                .find(|a| a.name == accumulator.name)
                                .map(|a| (a.accumulator_type.clone(), a.config.clone()))
                        })
                    })
                    .unwrap_or_else(|| ("passthrough".to_string(), Default::default()));
                AccumulatorDeclarationEntry {
                    name: accumulator.name.clone(),
                    accumulator_type,
                    config,
                }
            })
            .collect();
        cloacina_workflow_plugin::GraphPackageMetadata {
            graph_name: declaration.name.clone(),
            package_name: package_name.to_string(),
            reaction_mode: match declaration.reactor.criteria {
                ReactionCriteria::WhenAll => "when_all".to_string(),
                _ => "when_any".to_string(),
            },
            input_strategy: "latest".to_string(),
            accumulators,
            trigger_reactor: declaration.reactor_name.clone(),
            graph_data_json: None,
        }
    });
    graph.input_strategy = match declaration.reactor.strategy {
        InputStrategy::Sequential => "sequential".to_string(),
        InputStrategy::Latest => "latest".to_string(),
    };
    if graph.graph_data_json.is_none() {
        graph.graph_data_json = declaration.topology.clone();
    }
}

/// Unpack each provider archive of `dir` into `root`, the provider search
/// path of the import.
fn stage_providers(dir: &Path, root: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(root).map_err(|e| format!("create the providers folder: {e}"))?;
    let root = root.to_path_buf();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("cloacina") {
            continue;
        }
        cloacina::registry::loader::unpack_provider_archive(&path, &root, &[])
            .map_err(|e| format!("unpack a bundled provider: {e}"))?;
    }
    Ok(root)
}
