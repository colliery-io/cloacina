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

//! CLOACI-T-0919 — packaged-Python import edges.
//!
//! Two structural defects, one test binary:
//!
//! 1. A module-scope infinite loop used to abandon a GIL-holding thread, which
//!    disabled the single embedded interpreter process-wide. The loader now
//!    injects an async exception into the hung thread and only declares the
//!    runtime wedged if that fails.
//! 2. The `@task` namespace source was a process-global stack, so two
//!    concurrent package loads could silently namespace each other's tasks. It
//!    is now thread-local, like the registration target.
//!
//! **Not testable here:** a *true C-level* hang (a native extension that never
//! returns to the eval loop) is what the wedged path exists for, and there is
//! no portable way to manufacture one from a test — it would require shipping
//! a hostile native module per platform, and a failure would leave the test
//! binary permanently stuck. The wedged plumbing is covered instead by the
//! flag unit test in `cloacina::python_runtime` and the `/ready` test in
//! `cloacina-server`. What IS covered here is the interruption ladder, which
//! is the path a pure-Python hang actually takes.

use std::sync::Arc;
use tempfile::TempDir;

use cloacina_python::loader::import_and_register_python_workflow_named;

/// Lay out a single-module package the way `extract_python_package` does:
/// `workflow_dir` holds the source directly, `vendor_dir` is empty.
fn stage_package(
    dir: &TempDir,
    module: &str,
    source: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let workflow_dir = dir.path().join(format!("{module}_src"));
    let vendor_dir = dir.path().join(format!("{module}_vendor"));
    std::fs::create_dir_all(&workflow_dir).unwrap();
    std::fs::create_dir_all(&vendor_dir).unwrap();
    std::fs::write(workflow_dir.join(format!("{module}.py")), source).unwrap();
    (workflow_dir, vendor_dir)
}

const GOOD_MODULE: &str = r#"
import cloaca

@cloaca.task(id="ok_task", dependencies=[])
def ok_task(context):
    return context
"#;

/// A hostile package: the module body never returns. This is a PURE-PYTHON
/// loop, which is the interesting case — CPython's eval breaker still hands
/// the GIL around every few milliseconds, so `PyThreadState_SetAsyncExc` can
/// land and unwind it.
const HOSTILE_MODULE: &str = r#"
import cloaca

while True:
    pass

@cloaca.task(id="never_registered", dependencies=[])
def never_registered(context):
    return context
"#;

/// The whole point of the ladder: a hostile package fails ITS OWN load through
/// the interrupt path, and the interpreter is still usable afterwards.
#[test]
#[serial_test::serial(python_import)]
fn hostile_module_scope_loop_is_interrupted_and_subsystem_survives() {
    pyo3::prepare_freethreaded_python();

    // Shorten the deadline — the 60s default is not a test-shaped number.
    std::env::set_var(cloacina_python::import_guard::IMPORT_TIMEOUT_ENV, "2");

    let dir = TempDir::new().unwrap();
    let (hostile_src, hostile_vendor) = stage_package(&dir, "hostile_pkg", HOSTILE_MODULE);

    let started = std::time::Instant::now();
    let rt = Arc::new(cloacina::Runtime::empty());
    let result = import_and_register_python_workflow_named(
        &hostile_src,
        &hostile_vendor,
        "hostile_pkg",
        "hostile-pkg",
        "hostile_wf",
        "public",
        rt,
    );

    let err = result.expect_err("a module-scope infinite loop must fail the load");
    let message = err.to_string();
    assert!(
        message.contains("interrupted"),
        "the load must fail via the INTERRUPT path, not by abandoning the thread \
         (got: {message})"
    );
    assert!(
        !cloacina::python_runtime::is_python_runtime_wedged(),
        "a successfully interrupted import must NOT wedge the runtime — \
         reason: {:?}",
        cloacina::python_runtime::python_runtime_wedged_reason()
    );
    // 2s deadline + one 5s grace window; anything near the 60s default would
    // mean the env knob was ignored.
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "the configurable deadline was not honoured (took {:?})",
        started.elapsed()
    );

    // The floor requirement: the interpreter is healthy, so the NEXT package
    // loads normally. Before T-0919 this hung forever.
    let good_dir = TempDir::new().unwrap();
    let (good_src, good_vendor) = stage_package(&good_dir, "after_hostile_pkg", GOOD_MODULE);
    let rt2 = Arc::new(cloacina::Runtime::empty());
    let namespaces = import_and_register_python_workflow_named(
        &good_src,
        &good_vendor,
        "after_hostile_pkg",
        "after-hostile-pkg",
        "after_hostile_wf",
        "public",
        rt2,
    )
    .expect("the Python subsystem must still work after an interrupted import");

    assert!(
        namespaces.iter().any(|ns| ns.task_id == "ok_task"),
        "the follow-up package must register its task (got {:?})",
        namespaces.iter().map(|n| n.to_string()).collect::<Vec<_>>()
    );

    std::env::remove_var(cloacina_python::import_guard::IMPORT_TIMEOUT_ENV);
}

/// CLOACI-T-0919 item 2: the `@task` namespace source is thread-local, so two
/// packages loading CONCURRENTLY namespace their own tasks. With the old
/// process-global stack the two imports could interleave their pushes and
/// register tasks under each other's tenant/package — silently, with no error.
#[test]
#[serial_test::serial(python_import)]
fn concurrent_loads_from_two_threads_namespace_their_own_tasks() {
    pyo3::prepare_freethreaded_python();

    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    let (src_a, vendor_a) = stage_package(&dir_a, "concurrent_pkg_a", GOOD_MODULE);
    let (src_b, vendor_b) = stage_package(&dir_b, "concurrent_pkg_b", GOOD_MODULE);

    let rt_a = Arc::new(cloacina::Runtime::empty());
    let rt_b = Arc::new(cloacina::Runtime::empty());

    let a = {
        let rt_a = rt_a.clone();
        std::thread::spawn(move || {
            import_and_register_python_workflow_named(
                &src_a,
                &vendor_a,
                "concurrent_pkg_a",
                "pkg-a",
                "wf_a",
                "tenant_a",
                rt_a,
            )
        })
    };
    let b = {
        let rt_b = rt_b.clone();
        std::thread::spawn(move || {
            import_and_register_python_workflow_named(
                &src_b,
                &vendor_b,
                "concurrent_pkg_b",
                "pkg-b",
                "wf_b",
                "tenant_b",
                rt_b,
            )
        })
    };

    let ns_a = a.join().unwrap().expect("package A must load");
    let ns_b = b.join().unwrap().expect("package B must load");

    for ns in &ns_a {
        assert_eq!(ns.tenant_id, "tenant_a", "A's task leaked a foreign tenant");
        assert_eq!(
            ns.package_name, "pkg-a",
            "A's task leaked a foreign package"
        );
        assert_eq!(ns.workflow_id, "wf_a");
    }
    for ns in &ns_b {
        assert_eq!(ns.tenant_id, "tenant_b", "B's task leaked a foreign tenant");
        assert_eq!(
            ns.package_name, "pkg-b",
            "B's task leaked a foreign package"
        );
        assert_eq!(ns.workflow_id, "wf_b");
    }
    assert_eq!(ns_a.len(), 1, "A registered exactly its own task");
    assert_eq!(ns_b.len(), 1, "B registered exactly its own task");

    // And the runtimes only ever saw their own tasks.
    assert!(rt_a
        .get_task(&cloacina::TaskNamespace::new(
            "tenant_a", "pkg-a", "wf_a", "ok_task"
        ))
        .is_some());
    assert!(rt_b
        .get_task(&cloacina::TaskNamespace::new(
            "tenant_b", "pkg-b", "wf_b", "ok_task"
        ))
        .is_some());
}

/// The other half of item 2's acceptance: evaluating a decorator on a thread
/// that never entered the workflow scope is a DETERMINISTIC hard error naming
/// the cross-thread cause — never a silent registration under whatever
/// namespace some other thread happened to be holding.
#[test]
#[serial_test::serial(python_import)]
fn decorating_from_a_foreign_thread_hard_errors() {
    use cloacina_python::task::{
        current_workflow_context, pop_workflow_context, push_workflow_context,
    };
    use cloacina_python::workflow_context::PyWorkflowContext;

    pyo3::prepare_freethreaded_python();

    // This thread enters a workflow scope...
    push_workflow_context(PyWorkflowContext::new("tenant_x", "pkg-x", "wf_x"));
    assert_eq!(
        current_workflow_context().unwrap().as_components().0,
        "tenant_x"
    );

    // ...and a DIFFERENT thread tries to register into it.
    let foreign = std::thread::spawn(|| {
        let err = current_workflow_context()
            .expect_err("a foreign thread must not inherit this thread's namespace");
        err.to_string()
    })
    .join()
    .unwrap();

    assert!(
        foreign.contains("DIFFERENT thread"),
        "the error must name the cross-thread cause (got: {foreign})"
    );

    pop_workflow_context();

    // With no context anywhere, the message goes back to the ordinary
    // "you forgot the context manager" guidance.
    let plain = std::thread::spawn(|| current_workflow_context().unwrap_err().to_string())
        .join()
        .unwrap();
    assert!(
        plain.contains("WorkflowBuilder context manager"),
        "without an active context the generic message applies (got: {plain})"
    );
}
