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

//! The log lines of a packaged Rust task reach the host's log (CLOACINA-T-1099).
//!
//! The fixture `examples/fixtures/task-logs-rust` is built here with `cargo
//! build`, loaded through the task registrar (the loader the agent, its
//! worker, the server and the embedded runner use) and run by the in-process
//! executor on SQLite. The host's subscriber is a text subscriber with an
//! INFO filter, set as the default of every thread of the test's runtime, so
//! it also sees the blocking thread that calls into the library.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use cloacina::dal::DAL;
use cloacina::dispatcher::ExecutionStatus;
use cloacina::executor::{ExecutorConfig, ThreadTaskExecutor};
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::models::workflow_execution::NewWorkflowExecution;
use cloacina::registry::loader::TaskRegistrar;
use cloacina::registry::loader::package_loader::PackageMetadata;
use cloacina::{Database, Runtime, TaskExecutor, TaskReadyEvent, TaskRegistry};

const TASK_NAME: &str = "public::task-logs-rust::task_logs_workflow::speak";

/// Text log lines, written by the host's subscriber.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Capture {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
    type Writer = Capture;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fixtures/task-logs-rust")
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
            .expect("start cargo build of the task-logs-rust fixture");
        assert!(status.success(), "the task-logs-rust fixture builds");
        let ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };
        let path = target
            .join("debug")
            .join(format!("libtask_logs_rust.{ext}"));
        assert!(path.exists(), "built library at {}", path.display());
        path
    })
}

fn metadata() -> PackageMetadata {
    PackageMetadata {
        package_name: "task-logs-rust".into(),
        workflow_name: "task_logs_workflow".into(),
        version: "0.1.0".into(),
        description: None,
        author: None,
        tasks: vec![],
        graph_data: None,
        architecture: String::new(),
        symbols: vec![],
        workflow_triggers: vec![],
        declared_params: vec![],
        declared_surfaces: vec![],
        task_docs: Default::default(),
        python_platforms: None,
    }
}

/// Load `library` through the task registrar, run its task `speak` once in
/// the in-process executor on SQLite, and give the host's log lines and the
/// id of the run.
fn run_packaged_task(library: &Path) -> (String, String) {
    let capture = Capture::default();
    let make_subscriber = {
        let capture = capture.clone();
        move || {
            tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(capture.clone())
                .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
                .finish()
        }
    };
    let dispatch = tracing::Dispatch::new(make_subscriber());
    let thread_dispatch = dispatch.clone();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .on_thread_start(move || {
            // The default of each runtime thread, the blocking ones too, for
            // the life of the thread.
            std::mem::forget(tracing::dispatcher::set_default(&thread_dispatch));
        })
        .build()
        .unwrap();

    let bytes = std::fs::read(library).unwrap();
    let run_id = tracing::dispatcher::with_default(&dispatch, || {
        rt.block_on(async {
            let url = format!(
                "file:packaged_task_logs_{}?mode=memory&cache=shared",
                uuid::Uuid::new_v4()
            );
            let db = Database::new(&url, "", 5);
            db.run_migrations().await.expect("migrations");
            let dal = DAL::new(db.clone());

            let runtime = Arc::new(Runtime::empty());
            let registrar = TaskRegistrar::new().unwrap();
            let registered = registrar
                .register_package_tasks(
                    "task-logs-rust",
                    &bytes,
                    &metadata(),
                    Some("public"),
                    &runtime,
                )
                .await
                .expect("the package registers");
            assert_eq!(
                registered.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                vec![TASK_NAME.to_string()]
            );

            let exec = ThreadTaskExecutor::with_runtime_and_registry(
                db,
                Arc::new(TaskRegistry::new()),
                runtime,
                ExecutorConfig {
                    enable_claiming: false,
                    ..ExecutorConfig::default()
                },
            );
            let run = dal
                .workflow_execution()
                .create(NewWorkflowExecution {
                    workflow_name: "task_logs_workflow".into(),
                    workflow_version: "0.1.0".into(),
                    status: "Running".into(),
                    context_id: None,
                })
                .await
                .unwrap();
            let task = dal
                .task_execution()
                .create(NewTaskExecution {
                    workflow_execution_id: run.id,
                    task_name: TASK_NAME.into(),
                    status: "Ready".into(),
                    attempt: 1,
                    max_attempts: 1,
                    trigger_rules: r#"{"type":"Always"}"#.into(),
                    task_configuration: "{}".into(),
                })
                .await
                .unwrap();

            let result = TaskExecutor::execute(
                &exec,
                TaskReadyEvent::new(task.id, run.id, TASK_NAME.to_string(), 1),
            )
            .await
            .unwrap();
            assert_eq!(result.status, ExecutionStatus::Completed, "{result:?}");
            // Keep the registrar (and so the loaded library) for the process.
            std::mem::forget(registrar);
            run.id.to_string()
        })
    });
    rt.shutdown_background();
    (capture.text(), run_id)
}

/// The ids of the task's span, as the text format writes them.
fn span_ids(run_id: &str) -> String {
    format!(
        "task{{tenant=public workflow=task_logs_workflow execution_id={run_id} \
         task={TASK_NAME} attempt=1}}"
    )
}

fn line_with<'a>(log: &'a str, text: &str) -> Option<&'a str> {
    log.lines().find(|l| l.contains(text))
}

#[test]
fn sqlite_a_packaged_tasks_tracing_and_log_lines_reach_the_host_log_with_the_task_ids() {
    let (log, run_id) = run_packaged_task(fixture_library());
    let ids = span_ids(&run_id);

    let line = line_with(&log, "packaged tracing line")
        .unwrap_or_else(|| panic!("the task's tracing line is in the host's log:\n{log}"));
    println!("{line}");
    assert!(line.contains(" INFO "), "level kept: {line}");
    assert!(line.contains(&ids), "the task's ids: {line}");
    assert!(line.contains("task_logs::speak"), "target kept: {line}");
    assert!(line.contains("rows=3"), "field kept: {line}");

    let line = line_with(&log, "packaged log line")
        .unwrap_or_else(|| panic!("the task's log-crate line is in the host's log:\n{log}"));
    println!("{line}");
    assert!(line.contains(" WARN "), "level kept: {line}");
    assert!(line.contains(&ids), "the task's ids: {line}");
    assert!(line.contains("task_logs::legacy"), "target kept: {line}");

    assert!(
        line_with(&log, "packaged debug line").is_none(),
        "a DEBUG line is dropped by the host's INFO filter:\n{log}"
    );
}

/// A package built on a host that does not bind the log interface (an older
/// host) runs its task and writes nothing to the host's log, as before.
#[test]
fn sqlite_a_package_on_a_host_that_binds_no_log_interface_still_runs() {
    use cloacina_workflow_plugin::{
        METHOD_EXECUTE_TASK, TaskExecutionRequest, TaskExecutionResult,
    };

    // A copy at its own path is its own image of the library, with its own
    // bind cells.
    let dir = tempfile::TempDir::new().unwrap();
    let copy = dir.path().join(
        fixture_library()
            .file_name()
            .expect("the library has a file name"),
    );
    std::fs::copy(fixture_library(), &copy).unwrap();
    let loaded = fidius_host::loader::load_library(&copy).expect("the library loads");
    let plugin = loaded.plugins.into_iter().next().expect("one plugin");
    let handle = fidius_host::PluginHandle::from_loaded(plugin);
    let result: TaskExecutionResult = handle
        .call_method(
            METHOD_EXECUTE_TASK,
            &(TaskExecutionRequest {
                task_name: "speak".into(),
                context_json: "{}".into(),
                resolved_secrets: Default::default(),
                task_execution_id: String::new(),
            },),
        )
        .expect("execute_task");
    assert!(result.success, "{:?}", result.error);
    // Keep the library loaded for the process.
    std::mem::forget(handle);
    std::mem::forget(dir);
}

/// CLOACINA-T-1144: a Rust poll trigger opened as the agent opens it
/// (`FfiTriggerImpl::open`) writes its `tracing` line in the host's log, in
/// the span of the poll.
#[test]
fn sqlite_a_rust_poll_triggers_tracing_line_reaches_the_host_log_with_the_poll_ids() {
    use cloacina::cloacina_workflow::Trigger;
    use cloacina::registry::loader::ffi_trigger::FfiTriggerImpl;

    // A copy at its own path is its own image of the library, with its own
    // bind cells, as a build the agent fetched.
    let dir = tempfile::TempDir::new().unwrap();
    let copy = dir.path().join(
        fixture_library()
            .file_name()
            .expect("the library has a file name"),
    );
    std::fs::copy(fixture_library(), &copy).unwrap();

    let capture = Capture::default();
    let dispatch = tracing::Dispatch::new(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(capture.clone())
            .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
            .finish(),
    );
    let thread_dispatch = dispatch.clone();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .on_thread_start(move || {
            std::mem::forget(tracing::dispatcher::set_default(&thread_dispatch));
        })
        .build()
        .unwrap();

    tracing::dispatcher::with_default(&dispatch, || {
        rt.block_on(async {
            let trigger = FfiTriggerImpl::open(
                &copy,
                "task_logs_poll",
                std::time::Duration::from_secs(60),
                false,
            )
            .expect("the library opens");
            let span =
                tracing::info_span!("poll", poll_id = %"p-1144", trigger = %"task_logs_poll");
            let answer = tracing::Instrument::instrument(trigger.poll(), span)
                .await
                .expect("the poll answers");
            assert!(!answer.should_fire(), "the fixture skips");
            // Keep the library loaded for the process.
            std::mem::forget(trigger);
        })
    });
    rt.shutdown_background();
    std::mem::forget(dir);

    let log = capture.text();
    let line = line_with(&log, "packaged poll line")
        .unwrap_or_else(|| panic!("the poll's tracing line is in the host's log:\n{log}"));
    println!("{line}");
    assert!(line.contains(" INFO "), "level kept: {line}");
    assert!(line.contains("task_logs::poll"), "target kept: {line}");
    assert!(
        line.contains("poll{poll_id=p-1144 trigger=task_logs_poll}"),
        "the poll's ids: {line}"
    );
}

/// A package built before this change still loads and runs. Run by hand:
/// build the fixture at a commit before CLOACINA-T-1099 and give the path
/// of its library in `CLOACINA_T1099_OLD_LIBRARY`.
#[test]
#[ignore = "needs a library built before CLOACINA-T-1099; see the doc comment"]
fn sqlite_a_package_built_before_the_change_still_runs() {
    let path = PathBuf::from(
        std::env::var("CLOACINA_T1099_OLD_LIBRARY").expect("CLOACINA_T1099_OLD_LIBRARY is set"),
    );
    let (log, _) = run_packaged_task(&path);
    assert!(
        line_with(&log, "packaged tracing line").is_none(),
        "an older package's tracing line does not reach the host:\n{log}"
    );
    println!("{log}");
}
