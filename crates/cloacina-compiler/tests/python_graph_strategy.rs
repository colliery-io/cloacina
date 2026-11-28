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

//! The input strategy of a Python graph package (CLOACINA-T-1135).
//!
//! A Python graph whose manifest says `input_strategy = "sequential"` loads
//! and runs sequential in process, and the document the compiler stores for
//! it says the same. This test imports the package in its own process, so it
//! is a test binary of its own: an import leaves the module in `sys.modules`,
//! which the read of the same fixture in `describe.rs` must not see.
//!
//! Run with `--features sqlite`.

#![cfg(all(unix, feature = "sqlite"))]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use cloacina::Runtime;
use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::registry::{PackageLoadMode, ReconcilerConfig, RegistryReconciler};
use cloacina_compiler::{BuildOutcome, BuildRlimits, CompilerConfig};

type Registry = WorkflowRegistryImpl<UnifiedRegistryStorage>;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn compiler_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cloacina-compiler"))
}

/// Copy a fixture folder of `examples/fixtures`, with `from` replaced by `to`
/// in the file `file` (a path in the fixture).
fn stage_fixture_replacing(fixture: &str, file: &str, from: &str, to: &str) -> tempfile::TempDir {
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
    let path = stage.path().join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{file} has {from:?}");
    std::fs::write(&path, text.replace(from, to)).unwrap();
    stage
}

fn pack_dir(dir: &Path) -> Vec<u8> {
    let out = tempfile::TempDir::new().expect("tempdir");
    let archive = out.path().join("package.cloacina");
    fidius_core::package::pack_package(dir, Some(&archive)).expect("pack the package");
    std::fs::read(&archive).expect("read the archive")
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
        cargo_target_dir: None,
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

/// Register `archive`, build it through the compiler (the describe child)
/// and record the build with its document.
async fn build_and_record(
    registry: &mut Registry,
    archive: Vec<u8>,
    config: &CompilerConfig,
) -> uuid::Uuid {
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

/// A Python graph whose manifest says `sequential` loads in process with
/// that strategy, on the graph and on the reactor of the package it binds
/// to. The stored document of the package says `sequential` too.
#[tokio::test]
async fn a_python_graph_with_a_sequential_manifest_runs_sequential_in_process() {
    let stage = stage_fixture_replacing(
        "demo-py-graph",
        "package.toml",
        "input_strategy = \"latest\"",
        "input_strategy = \"sequential\"",
    );
    let home = tempfile::TempDir::new().unwrap();
    let url = format!(
        "file:t1135_strategy_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    let db = cloacina::database::Database::new(&url, "", 5);
    db.run_migrations().await.expect("migrations");
    let mut registry =
        WorkflowRegistryImpl::new(UnifiedRegistryStorage::new(db.clone()), db).expect("registry");
    let id = build_and_record(
        &mut registry,
        pack_dir(stage.path()),
        &test_config(home.path()),
    )
    .await;

    let document = registry
        .get_package_description(id)
        .await
        .unwrap()
        .expect("the row")
        .document
        .expect("a document");
    let described = document.graph.as_ref().expect("the graph");
    assert_eq!(
        described.input_strategy, "sequential",
        "the stored document"
    );

    cloacina::python_runtime::register_python_runtime(Arc::new(
        cloacina_python::CloacinaPythonRuntime,
    ));
    let config = ReconcilerConfig {
        default_tenant_id: "public".to_string(),
        package_load_mode: PackageLoadMode::InProcess,
        ..ReconcilerConfig::default()
    };
    let (_tx, rx) = tokio::sync::watch::channel(false);
    let scheduler = Arc::new(ComputationGraphScheduler::new(EndpointRegistry::new()));
    let reconciler = RegistryReconciler::new(Arc::new(registry), config, rx)
        .expect("create the reconciler")
        .with_runtime(Arc::new(Runtime::empty()))
        .with_graph_scheduler(scheduler.clone());
    let result = reconciler.reconcile().await.expect("reconcile");
    assert_eq!(
        result.packages_loaded,
        vec![id],
        "{:?}",
        result.packages_failed
    );

    let graphs = scheduler.list_graphs().await;
    let loaded = graphs
        .iter()
        .find(|g| g.name == "demo_py_graph")
        .unwrap_or_else(|| panic!("the graph is loaded in process: {graphs:?}"));
    assert!(loaded.running);
    assert_eq!(loaded.reactor.as_deref(), Some("demo_py_graph_rx"));
    assert_eq!(loaded.input_strategy, "sequential", "in process");
    assert_eq!(loaded.input_strategy, described.input_strategy);
    scheduler.shutdown_all().await;
}
