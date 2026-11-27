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

//! The describe child of the compiler (CLOACINA-T-1074).
//!
//! These tests run the built `cloacina-compiler` binary in describe mode:
//!
//! - the document of a Rust fixture and of Python fixtures equals, field by
//!   field, what the in-process load reads today;
//! - a build describes the package in a child: a library whose load-time code
//!   writes its process id is never opened by the compiler (this test) process;
//! - the child's environment has no database URL;
//! - a describe past its time limit fails the build with the reason;
//! - a package built before the document existed gets one from its stored
//!   build, and one that cannot be described is marked with the reason.
//!
//! The registry tests use SQLite: run with `--features sqlite`.

#![cfg(unix)]

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use cloacina::Runtime;
use cloacina::python_runtime::PythonRuntime;
use cloacina::registry::package_description::{
    PACKAGE_DESCRIPTION_VERSION, PackageDescription, python_load_view,
};
use cloacina::registry::{PackageLoader, TaskRegistrar};
use serial_test::serial;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// The built compiler: the program of the describe child.
fn compiler_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cloacina-compiler"))
}

/// One cargo target folder for every fixture these tests build.
fn fixture_target_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("t1074-fixture-target")
}

fn library_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

/// Copy a Rust fixture of `examples/fixtures` into a new folder, with the
/// `__WORKSPACE__` placeholder of a template fixture replaced by the path of
/// this workspace, and `extra_lib` added to the end of `src/lib.rs`.
fn stage_rust_fixture(fixture: &str, extra_lib: &str) -> (tempfile::TempDir, PathBuf) {
    let root = workspace_root();
    let source = root.join("examples/fixtures").join(fixture);
    let stage = tempfile::TempDir::new().expect("tempdir");
    let pkg = stage.path().join(fixture);
    std::fs::create_dir_all(pkg.join("src")).unwrap();
    let cargo = std::fs::read_to_string(source.join("Cargo.toml"))
        .unwrap()
        .replace("__WORKSPACE__", &root.display().to_string());
    std::fs::write(pkg.join("Cargo.toml"), cargo).unwrap();
    for file in ["build.rs", "package.toml"] {
        if source.join(file).exists() {
            std::fs::copy(source.join(file), pkg.join(file)).unwrap();
        }
    }
    let mut lib = std::fs::read_to_string(source.join("src/lib.rs")).unwrap();
    lib.push_str(extra_lib);
    std::fs::write(pkg.join("src/lib.rs"), lib).unwrap();
    (stage, pkg)
}

/// Build a Rust fixture of `examples/fixtures` and return its library.
fn fixture_library(fixture: &str) -> Vec<u8> {
    let in_place = workspace_root().join("examples/fixtures").join(fixture);
    let is_template = std::fs::read_to_string(in_place.join("Cargo.toml"))
        .expect("fixture Cargo.toml")
        .contains("__WORKSPACE__");
    // A template fixture is built from a copy with the placeholder replaced.
    let staged = is_template.then(|| stage_rust_fixture(fixture, ""));
    let dir = match &staged {
        Some((_keep, pkg)) => pkg.clone(),
        None => in_place,
    };
    let status = Command::new("cargo")
        .args(["build", "--lib"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", fixture_target_dir())
        .status()
        .expect("spawn cargo build for the fixture");
    assert!(status.success(), "build of fixture {fixture} failed");
    let path = fixture_target_dir().join("debug").join(format!(
        "lib{}.{}",
        fixture.replace('-', "_"),
        library_extension()
    ));
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Pack a fixture of `examples/fixtures` and return the archive bytes.
fn fixture_archive(fixture: &str) -> Vec<u8> {
    let dir = workspace_root().join("examples/fixtures").join(fixture);
    pack_dir(&dir)
}

fn pack_dir(dir: &Path) -> Vec<u8> {
    let out = tempfile::TempDir::new().expect("tempdir");
    let archive = out.path().join("package.cloacina");
    fidius_core::package::pack_package(dir, Some(&archive)).expect("pack the package");
    std::fs::read(&archive).expect("read the archive")
}

/// Run the compiler in describe mode with a cleared environment, as the
/// compiler runs it, and return the document.
fn describe_in_child(input_flag: &str, input: &Path) -> PackageDescription {
    let out_dir = tempfile::TempDir::new().expect("tempdir");
    let out = out_dir.path().join("description.json");
    let mut cmd = Command::new(compiler_bin());
    cmd.env_clear();
    for key in ["PATH", "HOME", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            cmd.env(key, value);
        }
    }
    let output = cmd
        .arg("describe")
        .arg(input_flag)
        .arg(input)
        .arg("--out")
        .arg(&out)
        .output()
        .expect("spawn the describe child");
    assert!(
        output.status.success(),
        "describe child failed ({}): {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(&out).expect("the child wrote the document");
    serde_json::from_slice(&bytes).expect("the document parses")
}

fn json<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).expect("serialize")
}

/// Sort a list of wire entries by their `name`, as JSON.
fn by_name<T: serde::Serialize>(entries: &[T]) -> Vec<serde_json::Value> {
    let mut values: Vec<serde_json::Value> = entries.iter().map(json).collect();
    values.sort_by_key(|v| v["name"].as_str().unwrap_or_default().to_string());
    values
}

// ---------------------------------------------------------------------------
// The document equals today's in-process read
// ---------------------------------------------------------------------------

/// For a Rust library: each field of the child's document equals what the
/// package loader and the task registrar read by opening the library in this
/// process, which is what the compiler and the reconciler do today.
#[tokio::test]
#[serial]
async fn rust_document_equals_the_in_process_read() {
    let mut documents = BTreeMap::new();
    for fixture in ["mixed-rust", "fleet-pools-rust"] {
        let library = fixture_library(fixture);
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(format!("package.{}", library_extension()));
        std::fs::write(&path, &library).unwrap();

        let doc = describe_in_child("--library", &path);
        assert_eq!(doc.format_version, PACKAGE_DESCRIPTION_VERSION);
        assert_eq!(doc.language, "rust");
        assert!(doc.python.is_none());

        // What the registry stores today (`extract_metadata`).
        let loader = PackageLoader::new().unwrap();
        let legacy = loader.extract_metadata(&library).await.unwrap();
        assert_eq!(doc.package_name, legacy.package_name);
        assert_eq!(doc.workflow_name, legacy.workflow_name);
        assert_eq!(doc.description, legacy.description);
        assert_eq!(doc.author, legacy.author);
        assert_eq!(doc.architecture, legacy.architecture);
        assert_eq!(doc.graph_data, legacy.graph_data);
        assert_eq!(doc.workflow_triggers, legacy.workflow_triggers);
        assert_eq!(doc.declared_params, legacy.declared_params);
        assert_eq!(doc.declared_surfaces, legacy.declared_surfaces);

        // What the reconciler's load steps read today.
        let triggers = loader.extract_trigger_metadata(&library).await.unwrap();
        let reactors = loader.extract_reactor_metadata(&library).await.unwrap();
        let graph = loader.extract_graph_metadata(&library).await.unwrap();
        let triggerless = loader
            .extract_triggerless_graph_metadata(&library)
            .await
            .unwrap();
        let constructors = loader.extract_constructor_metadata(&library).await.unwrap();
        assert_eq!(json(&doc.triggers), json(&triggers));
        assert_eq!(json(&doc.reactors), json(&reactors));
        assert_eq!(json(&doc.graph), json(&graph));
        assert_eq!(json(&doc.triggerless_graphs), json(&triggerless));
        assert_eq!(json(&doc.constructors), json(&constructors));

        // The tasks, as the task registrar loads them into a runtime: this is
        // where the scheduler and the fleet read trigger rules, the retry
        // policy, the time limit and the placement today.
        let runtime = Arc::new(Runtime::empty());
        let registrar = TaskRegistrar::new().unwrap();
        let namespaces = registrar
            .register_package_tasks("t1074", &library, &legacy, Some("public"), &runtime)
            .await
            .unwrap();
        assert_eq!(doc.tasks.len(), legacy.tasks.len());
        assert_eq!(doc.tasks.len(), namespaces.len());
        for (task, old) in doc.tasks.iter().zip(&legacy.tasks) {
            assert_eq!(task.index, old.index);
            assert_eq!(task.id, old.local_id);
            assert_eq!(task.namespaced_id_template, old.namespaced_id_template);
            assert_eq!(task.dependencies, old.dependencies);
            assert_eq!(task.description, old.description);
            assert_eq!(task.source_location, old.source_location);

            let ns = namespaces
                .iter()
                .find(|ns| ns.task_id == task.id)
                .expect("the registrar loaded the task");
            let loaded = runtime.get_task(ns).expect("task in the runtime");
            let loaded_deps: Vec<String> = loaded
                .dependencies()
                .iter()
                .map(|d| d.task_id.clone())
                .collect();
            assert_eq!(task.dependencies, loaded_deps, "{}", task.id);
            assert_eq!(task.trigger_rules, loaded.trigger_rules(), "{}", task.id);
            assert_eq!(task.retry_policy, loaded.retry_policy(), "{}", task.id);
            assert_eq!(
                task.timeout_seconds,
                loaded.timeout().map(|d| d.as_secs()),
                "{}",
                task.id
            );
            assert_eq!(
                task.requires_handle,
                loaded.requires_handle(),
                "{}",
                task.id
            );
            assert_eq!(task.placement(), loaded.placement(), "{}", task.id);
        }
        documents.insert(fixture, doc);
    }

    // The fixtures are not empty where the comparison above could pass on two
    // empty values.
    let mixed = &documents["mixed-rust"];
    assert_eq!(mixed.workflow_name, "mixed_workflow");
    assert_eq!(mixed.workflow_triggers, vec!["mixed_trigger"]);
    assert_eq!(mixed.triggers.len(), 1);
    assert_eq!(mixed.triggers[0].name, "mixed_trigger");
    assert_eq!(mixed.triggers[0].workflow_name, "mixed_workflow");
    assert_eq!(mixed.triggers[0].poll_interval, "5000ms");
    assert!(mixed.triggers[0].cron_expression.is_none());
    assert_eq!(mixed.reactors.len(), 1);
    assert_eq!(mixed.reactors[0].name, "mixed_reactor");
    assert_eq!(mixed.reactors[0].accumulators[0].name, "alpha");
    let graph = mixed.graph.as_ref().expect("the reactor-bound graph");
    assert_eq!(graph.trigger_reactor.as_deref(), Some("mixed_reactor"));
    let params: Vec<&str> = mixed
        .declared_params
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(params, vec!["signal_id", "window_ms"]);
    assert!(!mixed.declared_surfaces.is_empty());
    assert_eq!(mixed.tasks.len(), 1);
    assert!(mixed.graph_data.is_some());

    let pools = &documents["fleet-pools-rust"];
    let big = pools.tasks.iter().find(|t| t.id == "big").expect("big");
    assert_eq!(big.pool.as_deref(), Some("large"));
    assert_eq!(big.cpu.as_deref(), Some("1"));
    assert_eq!(big.memory.as_deref(), Some("24Gi"));
    assert_eq!(big.retry_policy.max_attempts, 0);
    let small = pools.tasks.iter().find(|t| t.id == "small").expect("small");
    assert!(small.placement().is_default());
}

/// For Python packages: each field of the child's document equals what the
/// reconciler reads today, after it imported the package in its own process.
#[test]
#[serial]
fn python_document_equals_the_in_process_read() {
    let mut documents = BTreeMap::new();
    for fixture in ["demo-py-workflow", "demo-py-cron"] {
        let archive = fixture_archive(fixture);
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("package.cloacina");
        std::fs::write(&path, &archive).unwrap();

        let doc = describe_in_child("--python-archive", &path);
        assert_eq!(doc.format_version, PACKAGE_DESCRIPTION_VERSION);
        assert_eq!(doc.language, "python");
        assert_eq!(doc.package_name, fixture);

        // Today: the server imports the package into its runtime and reads
        // the runtime back.
        let source = cloacina_workflow_plugin::unpack_limit::unpack_package(
            &path,
            &dir.path().join("source"),
        )
        .unwrap();
        let manifest = cloacina_workflow_plugin::manifest::load_resolved_manifest(&source).unwrap();
        let runtime = Arc::new(Runtime::empty());
        let python = cloacina_python::CloacinaPythonRuntime;
        let loaded = python
            .load_workflow_package(
                &archive,
                None,
                &dir.path().join("staging"),
                "public",
                &runtime,
            )
            .unwrap();
        let none = HashSet::new();
        let view = python_load_view(
            &runtime,
            fixture,
            &none,
            &none,
            &none,
            &manifest.metadata.accumulators,
        );

        assert_eq!(doc.workflow_name, loaded.workflow_name);
        assert_eq!(doc.description, manifest.metadata.description);
        assert_eq!(doc.author, manifest.metadata.author);
        assert_eq!(by_name(&doc.triggers), by_name(&view.triggers));
        assert_eq!(by_name(&doc.reactors), by_name(&view.reactors));
        assert_eq!(json(&doc.graph), json(&view.graph));
        let python_part = doc.python.as_ref().expect("the Python part");
        assert_eq!(python_part.entry_module, manifest.metadata.entry_module);
        assert_eq!(
            python_part.requires_python,
            manifest.metadata.requires_python
        );
        assert_eq!(python_part.python_version, python.python_version());

        // The task list: what `persist_task_graph` writes at load today, and
        // what the scheduler and the fleet read from the loaded task.
        assert_eq!(doc.tasks.len(), loaded.tasks.len());
        assert_eq!(doc.tasks.len(), loaded.task_namespaces.len());
        for node in &loaded.tasks {
            let task = doc
                .tasks
                .iter()
                .find(|t| t.id == node.id)
                .unwrap_or_else(|| panic!("task {} is not in the document", node.id));
            assert_eq!(task.dependencies, node.dependencies, "{}", node.id);
            let ns = loaded
                .task_namespaces
                .iter()
                .find(|ns| ns.task_id == node.id)
                .unwrap();
            let in_process = runtime.get_task(ns).expect("task in the runtime");
            assert_eq!(
                task.trigger_rules,
                in_process.trigger_rules(),
                "{}",
                node.id
            );
            assert_eq!(task.retry_policy, in_process.retry_policy(), "{}", node.id);
            assert_eq!(
                task.timeout_seconds,
                in_process.timeout().map(|d| d.as_secs()),
                "{}",
                node.id
            );
            assert_eq!(
                task.requires_handle,
                in_process.requires_handle(),
                "{}",
                node.id
            );
            assert_eq!(task.placement(), in_process.placement(), "{}", node.id);
            assert_eq!(
                task.namespaced_id_template,
                format!(
                    "{{tenant}}::{}::{}::{}",
                    fixture, loaded.workflow_name, node.id
                )
            );
        }
        documents.insert(fixture, doc);
    }

    let workflow = &documents["demo-py-workflow"];
    assert_eq!(workflow.workflow_name, "demo_py_workflow");
    // The document lists a Python package's tasks by id, with the index in
    // that order: the import reports them in no fixed order.
    let ids: Vec<&str> = workflow.tasks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["finish", "prepare", "transform", "validate"]);
    let indexes: Vec<u32> = workflow.tasks.iter().map(|t| t.index).collect();
    assert_eq!(indexes, vec![0, 1, 2, 3]);
    let finish = workflow.tasks.iter().find(|t| t.id == "finish").unwrap();
    let mut deps = finish.dependencies.clone();
    deps.sort();
    assert_eq!(deps, vec!["transform", "validate"]);
    assert!(workflow.triggers.is_empty());
    assert!(workflow.graph_data.is_some());

    let cron = &documents["demo-py-cron"];
    assert_eq!(cron.triggers.len(), 1);
    assert_eq!(cron.triggers[0].name, "demo_py_cron_trigger");
    assert_eq!(cron.triggers[0].workflow_name, "demo_py_cron_workflow");
    assert_eq!(
        cron.triggers[0].cron_expression.as_deref(),
        Some("*/15 * * * * *")
    );
    let audit = cron.tasks.iter().find(|t| t.id == "py_audit").unwrap();
    assert_ne!(
        audit.trigger_rules,
        serde_json::json!({"type": "Always"}),
        "py_audit has a trigger rule"
    );
}

/// For a Python computation graph package: the document's reactor and graph
/// equal what the reconciler reads today, the runtime walk and the graph
/// declaration it gives its scheduler.
#[test]
#[serial]
fn python_graph_document_equals_the_in_process_read() {
    let fixture = "demo-py-graph";
    let archive = fixture_archive(fixture);
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("package.cloacina");
    std::fs::write(&path, &archive).unwrap();

    let doc = describe_in_child("--python-archive", &path);
    assert_eq!(doc.language, "python");
    assert!(doc.tasks.is_empty());
    assert!(doc.workflow_name.is_empty());

    let source =
        cloacina_workflow_plugin::unpack_limit::unpack_package(&path, &dir.path().join("source"))
            .unwrap();
    let manifest = cloacina_workflow_plugin::manifest::load_resolved_manifest(&source).unwrap();
    let runtime = Arc::new(Runtime::empty());
    let python = cloacina_python::CloacinaPythonRuntime;
    let declaration = python
        .load_cg_package(
            &archive,
            None,
            &dir.path().join("staging"),
            "public",
            manifest.metadata.graph_name.as_deref().unwrap(),
            manifest.metadata.entry_module.as_deref().unwrap(),
            &manifest.metadata.accumulators,
            &runtime,
        )
        .unwrap()
        .expect("the graph declaration");
    let none = HashSet::new();
    let view = python_load_view(
        &runtime,
        fixture,
        &none,
        &none,
        &none,
        &manifest.metadata.accumulators,
    );

    assert_eq!(by_name(&doc.reactors), by_name(&view.reactors));
    assert_eq!(by_name(&doc.triggers), by_name(&view.triggers));
    assert_eq!(doc.reactors.len(), 1);
    assert_eq!(doc.reactors[0].name, "demo_py_graph_rx");
    assert_eq!(doc.reactors[0].reaction_mode, "when_any");
    assert_eq!(doc.reactors[0].accumulators[0].name, "py_alpha");

    let graph = doc.graph.as_ref().expect("the graph");
    assert_eq!(graph.graph_name, declaration.name);
    assert_eq!(graph.graph_name, "demo_py_graph");
    assert_eq!(graph.trigger_reactor, declaration.reactor_name);
    assert_eq!(graph.trigger_reactor.as_deref(), Some("demo_py_graph_rx"));
    assert_eq!(graph.input_strategy, "latest");
    let declared: Vec<&str> = declaration
        .accumulators
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    let described: Vec<&str> = graph.accumulators.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(described, declared);
    // The topology of the declaration, as JSON (key order aside).
    let topology = |text: &Option<String>| -> serde_json::Value {
        serde_json::from_str(text.as_deref().expect("a topology")).expect("topology JSON")
    };
    assert_eq!(
        topology(&graph.graph_data_json),
        topology(&declaration.topology)
    );
    // Where the runtime walk has the graph, the rest of the entry is its.
    if let Some(walked) = &view.graph {
        assert_eq!(graph.graph_name, walked.graph_name);
        assert_eq!(graph.reaction_mode, walked.reaction_mode);
        assert_eq!(json(&graph.accumulators), json(&walked.accumulators));
        assert_eq!(graph.trigger_reactor, walked.trigger_reactor);
    }
}

// ---------------------------------------------------------------------------
// Through the compiler's build and registry (SQLite)
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
mod through_the_compiler {
    use super::*;
    use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
    use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
    use cloacina_compiler::{BuildOutcome, BuildRlimits, CompilerConfig, describe_pass};
    use std::os::unix::fs::PermissionsExt;

    type Registry = WorkflowRegistryImpl<UnifiedRegistryStorage>;

    async fn sqlite_registry() -> Registry {
        let url = format!(
            "file:t1074_describe_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = cloacina::database::Database::new(&url, "", 5);
        db.run_migrations().await.expect("migrations");
        let storage = UnifiedRegistryStorage::new(db.clone());
        WorkflowRegistryImpl::new(storage, db).expect("registry")
    }

    fn test_config(home: &Path) -> CompilerConfig {
        CompilerConfig {
            home: home.to_path_buf(),
            bind: "127.0.0.1:0".parse().unwrap(),
            metrics_bind: None,
            database_url: "unused-in-this-test".to_string(),
            tenant_schema: None,
            build_target: None,
            build_target_package: None,
            verbose: false,
            poll_interval: Duration::from_secs(1),
            heartbeat_interval: Duration::from_secs(5),
            stale_threshold: Duration::from_secs(30),
            sweep_interval: Duration::from_secs(15),
            cargo_flags: vec!["build".to_string(), "--lib".to_string()],
            tmp_root: Some(home.join("build-tmp")),
            cargo_target_dir: Some(fixture_target_dir()),
            build_timeout: Duration::from_secs(1200),
            vendor_dir: None,
            dev_workspace: None,
            build_rlimits: BuildRlimits {
                cpu_s: 1200,
                mem_bytes: 16 * 1024 * 1024 * 1024,
                files: 4096,
                procs: 4096,
            },
            compiler_instance_id: cloacina::UniversalUuid::new_v4(),
            log_retention_days: 7,
            python: Default::default(),
            describe_timeout: Duration::from_secs(120),
            describe_program: Some(compiler_bin()),
        }
    }

    /// A stand-in for the describe program: a shell script.
    fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Register an archive and claim its build. Returns the package id.
    async fn register_and_claim(registry: &mut Registry, archive: Vec<u8>) -> uuid::Uuid {
        let id = registry
            .register_workflow_package(archive)
            .await
            .expect("register the package");
        let claim = registry
            .claim_next_build()
            .await
            .expect("claim")
            .expect("a pending build");
        assert_eq!(claim.id, id);
        id
    }

    /// The compiler fixture, with load-time code that appends the id of the
    /// process that opened the library to `mark`.
    fn marking_package(mark: &Path) -> Vec<u8> {
        let load_time_code = format!(
            r#"
// Load-time code: runs in each process that opens this library.
#[used]
#[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__mod_init_func"))]
#[cfg_attr(not(target_os = "macos"), unsafe(link_section = ".init_array"))]
static T1074_MARK: extern "C" fn() = {{
    extern "C" fn mark() {{
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open({mark:?})
        {{
            let _ = writeln!(file, "{{}}", std::process::id());
        }}
    }}
    mark
}};
"#,
            mark = mark.display().to_string()
        );
        let (_stage, pkg) = stage_rust_fixture("compiler-happy-rust", &load_time_code);
        pack_dir(&pkg)
    }

    /// A build describes the package in a child process and stores the
    /// document. The library's load-time code leaves the id of each process
    /// that opened it: this process, which is the compiler here, is not one.
    #[tokio::test]
    #[serial]
    async fn build_describes_in_a_child_and_stores_the_document() {
        let home = tempfile::TempDir::new().unwrap();
        let mark = home.path().join("opened-by.txt");
        let mut registry = sqlite_registry().await;
        let id = register_and_claim(&mut registry, marking_package(&mark)).await;
        let config = test_config(home.path());

        let outcome = cloacina_compiler::execute_build(&registry, id, &config).await;
        let BuildOutcome::Success {
            artifact,
            task_docs,
            declared_params,
            declared_surfaces,
            document,
        } = outcome
        else {
            match outcome {
                BuildOutcome::Failed(reason) => panic!("the build failed: {reason}"),
                BuildOutcome::TimedOut { elapsed } => panic!("the build timed out: {elapsed:?}"),
                BuildOutcome::Success { .. } => unreachable!(),
            }
        };
        let document = document.expect("the build gives a document");
        assert!(!artifact.is_empty());
        assert_eq!(document.language, "rust");
        assert_eq!(document.workflow_name, "compiler_happy_workflow");
        assert_eq!(document.tasks.len(), 1);
        assert_eq!(document.tasks[0].id, "noop");
        assert_eq!(document.tasks[0].retry_policy.max_attempts, 0);

        registry
            .mark_build_success_described(
                id,
                artifact,
                &document,
                task_docs,
                declared_params,
                declared_surfaces,
            )
            .await
            .expect("record the build");

        // A process opened the library, and it was not this one.
        let opened_by = std::fs::read_to_string(&mark).expect("the library was opened");
        let pids: Vec<u32> = opened_by
            .lines()
            .map(|l| l.trim().parse().expect("a process id"))
            .collect();
        assert!(!pids.is_empty(), "no process opened the library");
        assert!(
            !pids.contains(&std::process::id()),
            "the compiler process ({}) opened the built library: {pids:?}",
            std::process::id()
        );

        // The document is stored, and the row's old metadata was filled from
        // it without opening the library.
        let stored = registry
            .get_package_description(id)
            .await
            .unwrap()
            .expect("the row");
        assert_eq!(stored.version, Some(PACKAGE_DESCRIPTION_VERSION));
        assert!(stored.describe_error.is_none());
        let stored_doc = stored.document.expect("the stored document");
        assert_eq!(json(&stored_doc), json(&document));

        let inspected = registry
            .inspect_package_by_id(id)
            .await
            .unwrap()
            .expect("the row");
        assert_eq!(inspected.build_status, "success");
        assert_eq!(inspected.metadata.workflow_name, "compiler_happy_workflow");
        assert_eq!(inspected.metadata.tasks, vec!["noop"]);
        assert!(inspected.describe_error.is_none());
        assert!(
            registry
                .list_undescribed_packages(10)
                .await
                .unwrap()
                .is_empty()
        );

        let after = std::fs::read_to_string(&mark).unwrap();
        assert!(
            !after
                .lines()
                .any(|l| l.trim() == std::process::id().to_string()),
            "recording the build opened the library in the compiler process"
        );
    }

    /// The describe child is started through the child-process helper: its
    /// environment is cleared. The stand-in prints the environment it got.
    #[tokio::test]
    #[serial]
    async fn describe_child_has_no_database_url() {
        use cloacina::packaging::build_child::CHILD_ENV_ALLOWLIST;

        let home = tempfile::TempDir::new().unwrap();
        let mut registry = sqlite_registry().await;
        let id = register_and_claim(&mut registry, fixture_archive("demo-py-workflow")).await;
        let mut config = test_config(home.path());
        config.describe_program = Some(script(home.path(), "print-env", "env >&2\nexit 1"));

        let secrets = [
            (
                "DATABASE_URL",
                "postgres://user:secret@db.example.invalid/app",
            ),
            (
                "CLOACINA_DATABASE_URL",
                "postgres://user:secret@db.example.invalid/app",
            ),
            ("CLOACINA_SECRET_KEK", "kek"),
            ("CLOACINA_BOOTSTRAP_KEY", "bootstrap"),
            ("CLOACINA_T_1074_MARKER", "parent-only"),
        ];
        for (key, value) in secrets {
            // SAFETY: serial test; no other thread of this test reads the env.
            unsafe { std::env::set_var(key, value) };
        }
        let outcome = cloacina_compiler::execute_build(&registry, id, &config).await;
        for (key, _) in secrets {
            // SAFETY: as above.
            unsafe { std::env::remove_var(key) };
        }

        let BuildOutcome::Failed(reason) = outcome else {
            panic!("the stand-in exits 1, so the build must fail");
        };
        assert!(reason.contains("could not be described"), "{reason}");
        let printed: Vec<(&str, &str)> = reason
            .lines()
            .filter_map(|line| line.split_once('='))
            .filter(|(name, _)| {
                !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
            .collect();
        assert!(
            printed.iter().any(|(name, _)| *name == "PATH"),
            "the child's environment was not printed: {reason}"
        );
        // What the shell of the stand-in sets itself.
        let shell_own = ["PWD", "OLDPWD", "SHLVL", "_"];
        for (name, value) in &printed {
            assert!(
                !name.contains("DATABASE") && !value.contains("secret@"),
                "a database URL reached the describe child: {name}"
            );
            assert!(
                CHILD_ENV_ALLOWLIST.contains(name)
                    || shell_own.contains(name)
                    || *name == cloacina_workflow_plugin::unpack_limit::MAX_UNPACKED_BYTES_ENV,
                "{name} is in the describe child and on no list:\n{reason}"
            );
        }
        for (key, _) in secrets {
            assert!(
                !printed.iter().any(|(name, _)| name == &key),
                "{key} reached the describe child"
            );
        }
    }

    /// A describe that passes its time limit is killed, and the build fails
    /// with that reason. The row then reads `failed` with the error text.
    #[tokio::test]
    #[serial]
    async fn describe_past_its_time_limit_fails_the_build() {
        let home = tempfile::TempDir::new().unwrap();
        let mut registry = sqlite_registry().await;
        let id = register_and_claim(&mut registry, fixture_archive("demo-py-workflow")).await;
        let mut config = test_config(home.path());
        config.describe_program = Some(script(home.path(), "sleeper", "sleep 60"));
        config.describe_timeout = Duration::from_secs(1);

        let started = std::time::Instant::now();
        let outcome = cloacina_compiler::execute_build(&registry, id, &config).await;
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(30),
            "the describe was not killed at its limit ({elapsed:?})"
        );
        let BuildOutcome::Failed(reason) = outcome else {
            panic!("a describe past its limit must fail the build, not leave it for a retry");
        };
        assert!(reason.contains("timed out"), "{reason}");
        assert!(reason.contains("--describe-timeout-s"), "{reason}");

        registry.mark_build_failed(id, &reason).await.unwrap();
        let inspected = registry
            .inspect_package_by_id(id)
            .await
            .unwrap()
            .expect("the row");
        assert_eq!(inspected.build_status, "failed");
        assert!(
            inspected
                .build_error
                .as_deref()
                .unwrap_or_default()
                .contains("timed out")
        );
    }

    /// A Python package goes through the build with the real describe child:
    /// the document has its task list at build time, and the source parse
    /// (params, docs) is in it.
    #[tokio::test]
    #[serial]
    async fn python_build_stores_the_document_with_the_task_list() {
        let home = tempfile::TempDir::new().unwrap();
        let mut registry = sqlite_registry().await;
        let id = register_and_claim(&mut registry, fixture_archive("demo-py-workflow")).await;
        let config = test_config(home.path());

        let outcome = cloacina_compiler::execute_build(&registry, id, &config).await;
        let BuildOutcome::Success {
            artifact,
            task_docs,
            declared_params,
            declared_surfaces,
            document,
        } = outcome
        else {
            match outcome {
                BuildOutcome::Failed(reason) => panic!("the build failed: {reason}"),
                _ => panic!("the build did not finish"),
            }
        };
        assert!(artifact.is_empty(), "a Python package has no library");
        let document = document.expect("the build gives a document");
        assert_eq!(document.language, "python");
        assert_eq!(document.tasks.len(), 4);
        let params: Vec<&str> = document
            .declared_params
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(params, vec!["source_id", "batch_size"]);
        let prepare = document.tasks.iter().find(|t| t.id == "prepare").unwrap();
        assert!(
            prepare.doc_what.is_some(),
            "the docstring is in the document"
        );

        registry
            .mark_build_success_described(
                id,
                artifact,
                &document,
                task_docs,
                declared_params,
                declared_surfaces,
            )
            .await
            .unwrap();
        let stored = registry.get_package_description(id).await.unwrap().unwrap();
        assert_eq!(stored.document.expect("stored").tasks.len(), 4);
    }

    /// Packages built before the document existed are described from their
    /// stored build, with no publish and no build, and stay in service. One
    /// that cannot be described is marked with the reason and not tried again.
    #[tokio::test]
    #[serial]
    async fn packages_built_before_are_described_by_the_pass() {
        let home = tempfile::TempDir::new().unwrap();
        let mut registry = sqlite_registry().await;
        let config = test_config(home.path());

        // A Python package, a Rust package and a package whose stored library
        // is not a library. Each is recorded the way a compiler before this
        // change recorded it: no document.
        let python_id =
            register_and_claim(&mut registry, fixture_archive("demo-py-workflow")).await;
        registry
            .mark_build_success(python_id, Vec::new())
            .await
            .unwrap();

        let rust_library = fixture_library("mixed-rust");
        let rust_id = register_and_claim(&mut registry, fixture_archive("mixed-rust")).await;
        registry
            .mark_build_success(rust_id, rust_library)
            .await
            .unwrap();

        let broken_id =
            register_and_claim(&mut registry, fixture_archive("fleet-pools-rust")).await;
        registry
            .mark_build_success(broken_id, b"this is not a library".to_vec())
            .await
            .unwrap();

        let undescribed = registry.list_undescribed_packages(10).await.unwrap();
        assert_eq!(undescribed.len(), 3);
        assert_eq!(registry.build_queue_stats().await.unwrap().undescribed, 3);
        for id in [python_id, rust_id, broken_id] {
            let stored = registry.get_package_description(id).await.unwrap().unwrap();
            assert!(stored.document.is_none() && stored.version.is_none());
        }

        assert_eq!(describe_pass(&registry, &config).await, 3);

        let python = registry
            .get_package_description(python_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            python.describe_error.is_none(),
            "{:?}",
            python.describe_error
        );
        let python_doc = python.document.expect("the Python document");
        assert_eq!(python_doc.tasks.len(), 4);
        assert_eq!(python_doc.declared_params.len(), 2);

        let rust = registry
            .get_package_description(rust_id)
            .await
            .unwrap()
            .unwrap();
        assert!(rust.describe_error.is_none(), "{:?}", rust.describe_error);
        let rust_doc = rust.document.expect("the Rust document");
        assert_eq!(rust_doc.workflow_name, "mixed_workflow");
        assert_eq!(rust_doc.triggers.len(), 1);
        assert_eq!(rust_doc.reactors.len(), 1);

        let broken = registry
            .get_package_description(broken_id)
            .await
            .unwrap()
            .unwrap();
        assert!(broken.document.is_none());
        assert_eq!(broken.version, Some(PACKAGE_DESCRIPTION_VERSION));
        let reason = broken.describe_error.expect("the reason");
        assert!(reason.contains("could not be described"), "{reason}");

        // Nothing left service, and the mark is where the API reads it.
        for id in [python_id, rust_id, broken_id] {
            let inspected = registry.inspect_package_by_id(id).await.unwrap().unwrap();
            assert_eq!(inspected.build_status, "success");
            assert_eq!(inspected.describe_error.is_some(), id == broken_id);
        }
        let versions = registry
            .list_package_versions("fleet-pools-rust")
            .await
            .unwrap();
        assert_eq!(versions.len(), 1);
        assert!(versions[0].active);
        assert!(versions[0].describe_error.is_some());

        // The failed one is not tried again by the same compiler version.
        assert!(
            registry
                .list_undescribed_packages(10)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(registry.build_queue_stats().await.unwrap().undescribed, 0);
        assert_eq!(describe_pass(&registry, &config).await, 0);
    }

    // -----------------------------------------------------------------------
    // CLOACINA-T-1075: the server loads from the document only
    // -----------------------------------------------------------------------

    use cloacina::registry::{
        CronWorkflowRegistrar, PackageLoadMode, ReconcilerConfig, RegistryReconciler,
    };

    /// A cron registrar that records each schedule.
    #[derive(Default)]
    struct RecordedSchedules {
        cron: std::sync::Mutex<Vec<(String, String)>>,
        /// CLOACINA-T-1077: poll rows, as (trigger, workflow, interval ms).
        poll: std::sync::Mutex<Vec<(String, String, i32)>>,
    }

    #[async_trait::async_trait]
    impl CronWorkflowRegistrar for RecordedSchedules {
        async fn register_cron_workflow(
            &self,
            workflow_name: &str,
            cron_expression: &str,
            _timezone: &str,
        ) -> Result<String, String> {
            let mut cron = self.cron.lock().unwrap();
            cron.push((workflow_name.to_string(), cron_expression.to_string()));
            Ok(format!("schedule-{}", cron.len()))
        }
        async fn unregister_cron_workflow(&self, _schedule_id: &str) -> Result<(), String> {
            Ok(())
        }
        async fn register_poll_trigger(
            &self,
            trigger_name: &str,
            workflow_name: &str,
            poll_interval_ms: i32,
            _allow_concurrent: bool,
        ) -> Result<String, String> {
            let mut poll = self.poll.lock().unwrap();
            poll.push((
                trigger_name.to_string(),
                workflow_name.to_string(),
                poll_interval_ms,
            ));
            Ok(format!("poll-{}", poll.len()))
        }
    }

    /// A reconciler of the tenant `public` over `registry`, in `mode`, with
    /// its own empty runtime.
    fn reconciler(
        registry: Arc<Registry>,
        mode: PackageLoadMode,
        schedules: Arc<RecordedSchedules>,
    ) -> (RegistryReconciler, Arc<Runtime>) {
        reconciler_with_polls(
            registry,
            mode,
            schedules,
            cloacina::trigger::empty_poll_dispatcher_slot(),
        )
    }

    /// [`reconciler`], with the slot its poll triggers send their polls to.
    fn reconciler_with_polls(
        registry: Arc<Registry>,
        mode: PackageLoadMode,
        schedules: Arc<RecordedSchedules>,
        polls: cloacina::trigger::PollDispatcherSlot,
    ) -> (RegistryReconciler, Arc<Runtime>) {
        let config = ReconcilerConfig {
            default_tenant_id: "public".to_string(),
            package_load_mode: mode,
            ..ReconcilerConfig::default()
        };
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let runtime = Arc::new(Runtime::empty());
        let mut reconciler = RegistryReconciler::new(registry, config, rx)
            .expect("create the reconciler")
            .with_runtime(runtime.clone())
            .with_cron_registrar(schedules);
        reconciler.set_poll_dispatcher_slot(polls);
        (reconciler, runtime)
    }

    /// The process ids in a mark file. An absent file has none.
    fn marks(path: &Path) -> Vec<u32> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim().parse().expect("a process id"))
            .collect()
    }

    /// Build `archive` through the compiler (the describe child) and record
    /// the build with its document.
    async fn build_and_record(
        registry: &mut Registry,
        archive: Vec<u8>,
        config: &CompilerConfig,
    ) -> uuid::Uuid {
        let id = register_and_claim(registry, archive).await;
        let outcome = cloacina_compiler::execute_build(registry, id, config).await;
        let BuildOutcome::Success {
            artifact,
            task_docs,
            declared_params,
            declared_surfaces,
            document,
        } = outcome
        else {
            match outcome {
                BuildOutcome::Failed(reason) => panic!("the build failed: {reason}"),
                _ => panic!("the build did not finish"),
            }
        };
        let document = document.expect("the build gives a document");
        registry
            .mark_build_success_described(
                id,
                artifact,
                &document,
                task_docs,
                declared_params,
                declared_surfaces,
            )
            .await
            .expect("record the build");
        id
    }

    /// Copy a fixture folder of `examples/fixtures`, with `extra` added to the
    /// end of the file `module` (a path in the fixture).
    fn stage_fixture_with(fixture: &str, module: &str, extra: &str) -> tempfile::TempDir {
        fn copy_dir(from: &Path, to: &Path) {
            std::fs::create_dir_all(to).unwrap();
            for entry in std::fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let target = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy_dir(&entry.path(), &target);
                } else {
                    std::fs::copy(entry.path(), &target).unwrap();
                }
            }
        }
        let stage = tempfile::TempDir::new().unwrap();
        copy_dir(
            &workspace_root().join("examples/fixtures").join(fixture),
            stage.path(),
        );
        let file = stage.path().join(module);
        let mut text = std::fs::read_to_string(&file).unwrap();
        text.push_str(extra);
        std::fs::write(&file, text).unwrap();
        stage
    }

    /// With the metadata-only load, the reconciler in this process never
    /// opens a tenant's library: a library whose load-time code appends the
    /// process id holds the describe child's id and never this one. Its
    /// tasks and workflow are registered from the document. With the
    /// in-process load (single trust), the same package is opened here, as
    /// today.
    #[tokio::test]
    #[serial]
    async fn metadata_only_load_never_opens_the_library() {
        let home = tempfile::TempDir::new().unwrap();
        let mark = home.path().join("opened-by.txt");
        let config = test_config(home.path());
        let mut registry = sqlite_registry().await;
        let id = build_and_record(&mut registry, marking_package(&mark), &config).await;
        let described_by = marks(&mark);
        assert!(!described_by.is_empty(), "the describe child opened it");
        assert!(!described_by.contains(&std::process::id()));
        let registry = Arc::new(registry);

        let (metadata_only, runtime) = reconciler(
            registry.clone(),
            PackageLoadMode::MetadataOnly,
            Arc::default(),
        );
        let result = metadata_only.reconcile().await.expect("reconcile");
        assert_eq!(
            result.packages_loaded,
            vec![id],
            "{:?}",
            result.packages_failed
        );
        assert!(
            !marks(&mark).contains(&std::process::id()),
            "the server process opened the tenant's library"
        );
        let ns = cloacina::TaskNamespace::new(
            "public",
            "compiler-happy-rust",
            "compiler_happy_workflow",
            "noop",
        );
        let task = runtime.get_task(&ns).expect("the task, from the document");
        assert_eq!(task.retry_policy().max_attempts, 0);
        assert!(runtime.get_workflow("compiler_happy_workflow").is_some());

        // Single trust: today's in-process load opens it in this process.
        let (in_process, runtime) =
            reconciler(registry.clone(), PackageLoadMode::InProcess, Arc::default());
        let result = in_process.reconcile().await.expect("reconcile");
        assert_eq!(
            result.packages_loaded,
            vec![id],
            "{:?}",
            result.packages_failed
        );
        assert!(
            marks(&mark).contains(&std::process::id()),
            "the in-process load opens the library here"
        );
        assert!(runtime.get_task(&ns).is_some());
    }

    /// With the metadata-only load, the reconciler never imports a tenant's
    /// Python package, also with a Python runtime in the process: a module
    /// whose import appends the process id holds only the describe child's
    /// id. The tasks, the workflow and the cron schedule come from the
    /// document. With the in-process load the module is imported here.
    #[tokio::test]
    #[serial]
    async fn metadata_only_load_never_imports_the_python_package() {
        let home = tempfile::TempDir::new().unwrap();
        let mark = home.path().join("imported-by.txt");
        let import_mark = format!(
            "\n\nimport os as _t1075_os\nwith open({:?}, \"a\") as _t1075_f:\n    \
             _t1075_f.write(f\"{{_t1075_os.getpid()}}\\n\")\n",
            mark.display().to_string()
        );
        let stage = stage_fixture_with(
            "demo-py-cron",
            "workflow/demo_py_cron/tasks.py",
            &import_mark,
        );
        let config = test_config(home.path());
        let mut registry = sqlite_registry().await;
        let id = build_and_record(&mut registry, pack_dir(stage.path()), &config).await;
        let described_by = marks(&mark);
        assert!(!described_by.is_empty(), "the describe child imported it");
        assert!(!described_by.contains(&std::process::id()));
        let registry = Arc::new(registry);
        cloacina::python_runtime::register_python_runtime(Arc::new(
            cloacina_python::CloacinaPythonRuntime,
        ));

        let schedules = Arc::new(RecordedSchedules::default());
        let (metadata_only, runtime) = reconciler(
            registry.clone(),
            PackageLoadMode::MetadataOnly,
            schedules.clone(),
        );
        let result = metadata_only.reconcile().await.expect("reconcile");
        assert_eq!(
            result.packages_loaded,
            vec![id],
            "{:?}",
            result.packages_failed
        );
        assert!(
            !marks(&mark).contains(&std::process::id()),
            "the server process imported the tenant's package"
        );
        let workflow = runtime
            .get_workflow("demo_py_cron_workflow")
            .expect("the workflow, from the document");
        assert_eq!(workflow.get_task_ids().len(), 4);
        let audit = runtime
            .get_task(&cloacina::TaskNamespace::new(
                "public",
                "demo-py-cron",
                "demo_py_cron_workflow",
                "py_audit",
            ))
            .expect("py_audit");
        assert_ne!(audit.trigger_rules(), serde_json::json!({"type": "Always"}));
        assert_eq!(
            schedules.cron.lock().unwrap().as_slice(),
            &[(
                "demo_py_cron_workflow".to_string(),
                "*/15 * * * * *".to_string()
            )]
        );

        // Single trust: today's in-process load imports it in this process.
        let (in_process, _runtime) =
            reconciler(registry.clone(), PackageLoadMode::InProcess, Arc::default());
        let result = in_process.reconcile().await.expect("reconcile");
        assert_eq!(
            result.packages_loaded,
            vec![id],
            "{:?}",
            result.packages_failed
        );
        assert!(
            marks(&mark).contains(&std::process::id()),
            "the in-process load imports the package here"
        );
    }

    /// A built package with no document (a build of a compiler before the
    /// document) does not load in the metadata-only mode, and its row is not
    /// changed; the in-process load still loads it.
    #[tokio::test]
    #[serial]
    async fn a_package_with_no_document_does_not_load_from_metadata() {
        let mut registry = sqlite_registry().await;
        let id = register_and_claim(&mut registry, fixture_archive("demo-py-workflow")).await;
        registry.mark_build_success(id, Vec::new()).await.unwrap();
        let registry = Arc::new(registry);

        let (metadata_only, runtime) = reconciler(
            registry.clone(),
            PackageLoadMode::MetadataOnly,
            Arc::default(),
        );
        let result = metadata_only.reconcile().await.expect("reconcile");
        assert!(result.packages_loaded.is_empty());
        assert_eq!(result.packages_failed.len(), 1);
        assert!(
            result.packages_failed[0]
                .1
                .contains("no metadata document yet"),
            "{:?}",
            result.packages_failed
        );
        assert!(runtime.task_namespaces().is_empty());
        let inspected = registry.inspect_package_by_id(id).await.unwrap().unwrap();
        assert_eq!(inspected.build_status, "success");
    }

    // -----------------------------------------------------------------------
    // CLOACINA-T-1077: a poll trigger's code runs on the agent
    // -----------------------------------------------------------------------

    use cloacina::cloacina_workflow::{TriggerError, TriggerResult};
    use cloacina::trigger::{PollDispatcher, PollRequest};

    /// Stands in for the agent fleet: it records each poll and answers fire
    /// with `{"from": "agent"}`. It runs no trigger code.
    #[derive(Default)]
    struct FleetStandIn {
        polls: std::sync::Mutex<Vec<PollRequest>>,
    }

    #[async_trait::async_trait]
    impl PollDispatcher for FleetStandIn {
        async fn poll(&self, request: &PollRequest) -> Result<TriggerResult, TriggerError> {
            self.polls.lock().unwrap().push(request.clone());
            let mut ctx = cloacina::Context::new();
            ctx.insert("from", serde_json::json!("agent")).unwrap();
            Ok(TriggerResult::Fire(Some(ctx)))
        }
        fn wait(&self) -> Duration {
            Duration::from_secs(40)
        }
    }

    /// What an agent does with the answer of a poll: the outcome it reports
    /// and the server reads back.
    fn answer_as_the_server_reads_it(result: Result<TriggerResult, TriggerError>) -> TriggerResult {
        cloacina::fleet::poll_result_from_outcome(cloacina::fleet::poll_outcome(result))
            .expect("a poll answer")
    }

    /// A Rust package with a poll trigger loads on a server that loads from
    /// metadata only. The server's scheduler polls the trigger through the
    /// dispatcher, and the trigger's code never runs in this process: the
    /// poll appends the process id to a file, and the file has none. The
    /// agent's path then opens the build and polls it: the code runs there,
    /// and the answer is fire with the trigger's context.
    #[tokio::test]
    #[serial]
    async fn a_rust_poll_trigger_runs_on_the_agent_never_in_the_server() {
        let home = tempfile::TempDir::new().unwrap();
        let mark = home.path().join("polled-by.txt");
        let poll_code = format!(
            r#"
// CLOACINA-T-1077: a poll trigger that leaves the id of the process it runs in.
#[cloacina_macros::trigger(on = "compiler_happy_workflow", poll_interval = "7s")]
pub async fn t1077_marking_poll() -> Result<cloacina_workflow::TriggerResult, cloacina_workflow::TriggerError> {{
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open({mark:?}) {{
        let _ = writeln!(file, "{{}}", std::process::id());
    }}
    let mut ctx = cloacina_workflow::Context::new();
    ctx.insert("polled_by", serde_json::json!(std::process::id()))
        .map_err(cloacina_workflow::TriggerError::from)?;
    Ok(cloacina_workflow::TriggerResult::Fire(Some(ctx)))
}}
"#,
            mark = mark.display().to_string()
        );
        let (_stage, pkg) = stage_rust_fixture("compiler-happy-rust", &poll_code);
        let config = test_config(home.path());
        let mut registry = sqlite_registry().await;
        let id = build_and_record(&mut registry, pack_dir(&pkg), &config).await;
        // The build this test just made; copied before another test builds.
        let built = fixture_target_dir()
            .join("debug")
            .join(format!("libcompiler_happy_rust.{}", library_extension()));
        let library = home
            .path()
            .join(format!("agent-copy.{}", library_extension()));
        std::fs::copy(&built, &library).expect("copy the built library");
        assert!(marks(&mark).is_empty(), "the describe child does not poll");
        let registry = Arc::new(registry);

        let slot = cloacina::trigger::empty_poll_dispatcher_slot();
        let stand_in = Arc::new(FleetStandIn::default());
        *slot.write().unwrap() = Some(stand_in.clone());
        let schedules = Arc::new(RecordedSchedules::default());
        let (metadata_only, runtime) = reconciler_with_polls(
            registry.clone(),
            PackageLoadMode::MetadataOnly,
            schedules.clone(),
            slot,
        );
        let result = metadata_only.reconcile().await.expect("reconcile");
        assert_eq!(
            result.packages_loaded,
            vec![id],
            "the refusal of a poll trigger is gone: {:?}",
            result.packages_failed
        );
        // The interval comes from the stored document.
        assert_eq!(
            schedules.poll.lock().unwrap().as_slice(),
            &[(
                "t1077_marking_poll".to_string(),
                "compiler_happy_workflow".to_string(),
                7_000
            )]
        );

        // The scheduler's poll, on the server: it goes to the fleet.
        let trigger = runtime
            .get_trigger("t1077_marking_poll")
            .expect("the trigger, from the document");
        for _ in 0..3 {
            let answer = trigger.poll().await.expect("an answer");
            let ctx = answer.into_context().expect("fire with a context");
            assert_eq!(ctx.get("from"), Some(&serde_json::json!("agent")));
        }
        assert_eq!(stand_in.polls.lock().unwrap().len(), 3);
        assert_eq!(
            stand_in.polls.lock().unwrap()[0].package_name,
            "compiler-happy-rust"
        );
        assert!(
            !marks(&mark).contains(&std::process::id()),
            "the server process ran the poll trigger's code"
        );
        assert!(marks(&mark).is_empty());

        // The agent's path: open the build, poll the trigger.
        let on_agent = cloacina::registry::loader::ffi_trigger::FfiTriggerImpl::open(
            &library,
            "t1077_marking_poll",
            Duration::from_secs(7),
            false,
        )
        .expect("open the build as the agent does");
        let ctx = answer_as_the_server_reads_it(
            cloacina::cloacina_workflow::Trigger::poll(&on_agent).await,
        )
        .into_context()
        .expect("fire with the trigger's context");
        assert_eq!(
            ctx.get("polled_by"),
            Some(&serde_json::json!(std::process::id()))
        );
        assert_eq!(marks(&mark), vec![std::process::id()]);
    }

    /// The same for a Python package: the poll trigger loads from the
    /// document, the server's poll goes to the fleet, the module is never
    /// imported here; the agent's path imports the package into a runtime,
    /// polls it, and the answer is fire with the trigger's context.
    #[tokio::test]
    #[serial]
    async fn a_python_poll_trigger_runs_on_the_agent_never_in_the_server() {
        let home = tempfile::TempDir::new().unwrap();
        let mark = home.path().join("polled-by.txt");
        let poll_code = format!(
            "\n\nimport os as _t1077_os\n\n\
             @cloaca.trigger(on=\"demo_py_cron_workflow\", poll_interval=\"9s\")\n\
             def t1077_marking_poll():\n    \
             with open({mark:?}, \"a\") as _t1077_f:\n        \
             _t1077_f.write(f\"{{_t1077_os.getpid()}}\\n\")\n    \
             return cloaca.TriggerResult.fire(cloaca.Context({{\"polled_by\": _t1077_os.getpid()}}))\n",
            mark = mark.display().to_string()
        );
        let stage =
            stage_fixture_with("demo-py-cron", "workflow/demo_py_cron/tasks.py", &poll_code);
        let archive = pack_dir(stage.path());
        let config = test_config(home.path());
        let mut registry = sqlite_registry().await;
        let id = build_and_record(&mut registry, archive.clone(), &config).await;
        assert!(marks(&mark).is_empty(), "the describe child does not poll");
        let registry = Arc::new(registry);
        cloacina::python_runtime::register_python_runtime(Arc::new(
            cloacina_python::CloacinaPythonRuntime,
        ));

        let slot = cloacina::trigger::empty_poll_dispatcher_slot();
        let stand_in = Arc::new(FleetStandIn::default());
        *slot.write().unwrap() = Some(stand_in.clone());
        let schedules = Arc::new(RecordedSchedules::default());
        let (metadata_only, runtime) = reconciler_with_polls(
            registry.clone(),
            PackageLoadMode::MetadataOnly,
            schedules.clone(),
            slot,
        );
        let result = metadata_only.reconcile().await.expect("reconcile");
        assert_eq!(
            result.packages_loaded,
            vec![id],
            "the refusal of a poll trigger is gone: {:?}",
            result.packages_failed
        );
        assert_eq!(
            schedules.poll.lock().unwrap().as_slice(),
            &[(
                "t1077_marking_poll".to_string(),
                "demo_py_cron_workflow".to_string(),
                9_000
            )]
        );
        let trigger = runtime
            .get_trigger("t1077_marking_poll")
            .expect("the trigger, from the document");
        assert!(trigger.poll().await.expect("an answer").should_fire());
        assert_eq!(stand_in.polls.lock().unwrap().len(), 1);
        assert!(
            marks(&mark).is_empty(),
            "the server process ran the poll trigger's code"
        );

        // The agent's path: import into a runtime of the build, poll.
        let py = cloacina::python_runtime::python_runtime().expect("a Python runtime");
        let agent_runtime = Arc::new(Runtime::new());
        let staging = home.path().join("agent-pysrc");
        std::fs::create_dir_all(&staging).unwrap();
        let load_runtime = agent_runtime.clone();
        tokio::task::spawn_blocking(move || {
            py.load_workflow_package(&archive, None, &staging, "public", &load_runtime)
                .map(|_| ())
        })
        .await
        .unwrap()
        .expect("import the package as the agent does");
        let on_agent = agent_runtime
            .get_trigger("t1077_marking_poll")
            .expect("the import registers the trigger");
        let ctx = answer_as_the_server_reads_it(on_agent.poll().await)
            .into_context()
            .expect("fire with the trigger's context");
        assert_eq!(
            ctx.get("polled_by"),
            Some(&serde_json::json!(std::process::id()))
        );
        assert_eq!(marks(&mark), vec![std::process::id()]);
    }
}
