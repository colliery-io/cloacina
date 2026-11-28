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

//! A packaged workflow with a declared priority runs end to end at that
//! priority (CLOACINA-T-1123), on SQLite, in process.
//!
//! The fixture `examples/fixtures/priority-rust` declares its workflow at
//! priority 2, its task `urgent` at 0 and its task `plain` at none. It is
//! built here with `cargo build`, registered and marked built in the
//! registry, loaded by the reconciler (which sets the workflow's priority
//! from the package's task metadata), planned by the scheduler and run by the
//! in-process executor. With no priority authority (the embedded use) a
//! declared priority is given as declared.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
use cloacina::dal::{DAL, UnifiedRegistryStorage};
use cloacina::dispatcher::{DefaultDispatcher, Dispatcher};
use cloacina::execution_planner::TaskScheduler;
use cloacina::executor::{ExecutorConfig, ThreadTaskExecutor};
use cloacina::registry::reconciler::{ReconcilerConfig, RegistryReconciler};
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::{Context, Database, Runtime, TaskRegistry, UniversalUuid};

const WORKFLOW: &str = "packaged_priority_workflow";

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fixtures/priority-rust")
}

/// Build the fixture once per test process and give the path of its library.
fn fixture_library() -> &'static Path {
    static LIBRARY: OnceLock<PathBuf> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let dir = fixture_dir();
        let target = dir.join("target");
        let status = std::process::Command::new(env!("CARGO"))
            .arg("build")
            .current_dir(&dir)
            .env("CARGO_TARGET_DIR", &target)
            .status()
            .expect("start cargo build of the priority-rust fixture");
        assert!(status.success(), "the priority-rust fixture builds");
        let ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };
        let path = target.join("debug").join(format!("libpriority_rust.{ext}"));
        assert!(path.exists(), "built library at {}", path.display());
        path
    })
}

/// The fixture's source package, as an upload sends it.
fn fixture_archive() -> Vec<u8> {
    let temp_dir = tempfile::TempDir::new().expect("tempdir");
    let archive = temp_dir
        .path()
        .join(format!("priority-rust-{}.cloacina", uuid::Uuid::new_v4()));
    fidius_core::package::pack_package(&fixture_dir(), Some(&archive)).expect("pack the fixture");
    std::fs::read(&archive).expect("read the archive")
}

#[tokio::test]
async fn sqlite_a_packaged_workflow_runs_at_the_priority_it_declares() {
    let dylib = std::fs::read(fixture_library()).expect("read the library");
    let archive = fixture_archive();

    // A file, not memory: the pool may close every connection while the
    // run waits, and a shared in-memory database ends with its last one.
    let db_dir = tempfile::TempDir::new().expect("tempdir");
    let url = format!("sqlite://{}", db_dir.path().join("priority.db").display());
    let db = Database::new(&url, "", 5);
    db.run_migrations().await.expect("migrations");
    let dal = DAL::new(db.clone());

    // Publish: register the package and record its build.
    let mut writer = WorkflowRegistryImpl::new(UnifiedRegistryStorage::new(db.clone()), db.clone())
        .expect("registry");
    let package_id = writer
        .register_workflow_package(archive)
        .await
        .expect("register the package");
    writer.claim_next_build().await.expect("claim the build");
    writer
        .mark_build_success(package_id, dylib)
        .await
        .expect("mark the package built");

    // Load: the reconciler puts the workflow and its tasks in the runtime.
    let runtime = Arc::new(Runtime::empty());
    let reader: Arc<dyn WorkflowRegistry> = Arc::new(
        WorkflowRegistryImpl::new(UnifiedRegistryStorage::new(db.clone()), db.clone())
            .expect("registry"),
    );
    let graphs = Arc::new(ComputationGraphScheduler::new(EndpointRegistry::new()));
    let (_reconciler_stop, reconciler_stopped) = tokio::sync::watch::channel(false);
    let reconciler = RegistryReconciler::new(
        reader,
        ReconcilerConfig {
            default_tenant_id: "public".to_string(),
            ..ReconcilerConfig::default()
        },
        reconciler_stopped,
    )
    .expect("reconciler")
    .with_runtime(runtime.clone())
    .with_graph_scheduler(graphs.clone());
    let loaded = reconciler.reconcile().await.expect("reconcile");
    assert!(
        loaded.packages_failed.is_empty(),
        "{:?}",
        loaded.packages_failed
    );
    let workflow = runtime
        .get_workflow(WORKFLOW)
        .expect("the workflow is loaded");
    assert_eq!(
        workflow.priority(),
        Some(2),
        "the priority the package declares"
    );

    // Run: the scheduler plans the run, the in-process executor runs it.
    let executor = Arc::new(ThreadTaskExecutor::with_runtime_and_registry(
        db.clone(),
        Arc::new(TaskRegistry::new()),
        runtime.clone(),
        ExecutorConfig {
            max_concurrent_tasks: 1,
            enable_claiming: true,
            ..ExecutorConfig::default()
        },
    ));
    let dispatcher = Arc::new(DefaultDispatcher::new(dal.clone(), "default"));
    dispatcher.register_executor("default", executor);
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let scheduler = Arc::new(
        TaskScheduler::with_poll_interval(db.clone(), Duration::from_millis(50))
            .await
            .expect("scheduler")
            .with_runtime(runtime.clone())
            .with_dispatcher(dispatcher)
            .with_shutdown(stopped),
    );
    let id = scheduler
        .schedule_workflow_execution(WORKFLOW, Context::new())
        .await
        .expect("schedule a run");
    let id = UniversalUuid(id);

    // The planner wrote the declared priorities on the rows.
    let execution = dal.workflow_execution().get_by_id(id).await.expect("run");
    assert_eq!((execution.priority_asked, execution.priority), (Some(2), 2));
    let task = |rows: &[cloacina::models::task_execution::TaskExecution], name: &str| {
        rows.iter()
            .find(|t| t.task_name.ends_with(&format!("::{name}")))
            .unwrap_or_else(|| panic!("a row for {name}: {rows:?}"))
            .clone()
    };
    let rows = dal
        .task_execution()
        .get_all_tasks_for_workflow(id)
        .await
        .expect("task rows");
    let urgent = task(&rows, "urgent");
    let plain = task(&rows, "plain");
    assert_eq!(
        (
            urgent.workflow_priority,
            urgent.task_priority,
            urgent.task_priority_asked
        ),
        (2, 0, Some(0))
    );
    assert_eq!(
        (
            plain.workflow_priority,
            plain.task_priority,
            plain.task_priority_asked
        ),
        (2, 2, None)
    );

    let looping = {
        let scheduler = scheduler.clone();
        tokio::spawn(async move { scheduler.run_scheduling_loop().await })
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let status = loop {
        let status = dal
            .workflow_execution()
            .get_by_id(id)
            .await
            .expect("run")
            .status;
        if matches!(status.as_str(), "Completed" | "Failed" | "Cancelled")
            || tokio::time::Instant::now() >= deadline
        {
            break status;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    let _ = stop.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(10), looping).await;
    graphs.shutdown_all().await;
    // Keep the loaded library for the process.
    std::mem::forget(reconciler);

    assert_eq!(status, "Completed");
    let rows = dal
        .task_execution()
        .get_all_tasks_for_workflow(id)
        .await
        .expect("task rows");
    let urgent = task(&rows, "urgent");
    let plain = task(&rows, "plain");
    assert_eq!(
        (urgent.status.as_str(), plain.status.as_str()),
        ("Completed", "Completed")
    );
    // One slot: the more important task of the run went first.
    assert!(
        urgent.started_at.expect("urgent started").0 <= plain.started_at.expect("plain started").0,
        "urgent at {:?}, plain at {:?}",
        urgent.started_at,
        plain.started_at
    );
}
