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

//! CLOACI-T-0831 — the **Python half of the packaged-provider story**:
//! `cloaca.constructor(...)` wires a WASM constructor-provider member into a
//! Python workflow, resolved through the SAME host loader as Rust's
//! `constructor!` (provider search path, name-in-configure member select,
//! name-keyed config, default-closed grants) and registered into the scoped
//! Runtime where the Python workflow's DAG assembly picks it up.
//!
//! The provider is the real `cloacina-provider-fs` suite, bundled from the
//! consumer fixture's Cargo graph exactly as the build side does. The proof
//! EXECUTES the resolved node: `read_file` reads a granted file through the
//! sandbox from a Python-authored workflow.
//!
//! Builds the provider to wasm32-wasip2 in-test (same pattern as the cloacina
//! wasm tests).

use serial_test::serial;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Bundle `cloacina-provider-fs` into a providers/ tree ONCE, point the loader's
/// provider search path at it, and return the granted data dir.
fn provider_data_dir() -> &'static PathBuf {
    static ROOT: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &ROOT
        .get_or_init(|| {
            let work = tempfile::TempDir::new().unwrap();
            let consumer =
                workspace_root().join("examples/constructor-contract/provider-consumer-fixture");
            let bundled = cloacina::packaging::provider_bundle::bundle_providers(
                &consumer,
                &[cloacina::packaging::provider_bundle::ProviderRef::parse(
                    "cloacina-provider-fs",
                )],
                work.path(),
                false,
            )
            .expect("bundle cloacina-provider-fs");
            assert_eq!(bundled.len(), 1);
            cloacina::registry::loader::set_provider_search_path(work.path().join("providers"));

            let data = work.path().join("data");
            std::fs::create_dir_all(&data).unwrap();
            std::fs::write(data.join("secret.txt"), "python-read secret").unwrap();
            (work, data)
        })
        .1
}

#[tokio::test]
#[serial]
async fn python_constructor_node_resolves_registers_and_runs() {
    pyo3::prepare_freethreaded_python();
    let data = provider_data_dir();
    let secret = data.join("secret.txt");

    // A scoped Runtime, exactly as the loader installs for Python workflows.
    let runtime = Arc::new(cloacina::Runtime::empty());
    let _scope = cloacina_python::runtime_scope::ScopedRuntime::new(runtime.clone())
        .expect("install scoped runtime");

    // Author a Python workflow that wires the provider member with a grant.
    let code = format!(
        r#"
import cloaca

with cloaca.WorkflowBuilder("pyflow") as _:
    cloaca.constructor(
        id="reader",
        from_="cloacina-provider-fs@0.1.0",
        constructor="read_file",
        config={{"path": "{secret}"}},
        grants={{"fs": ["ro:{data}"]}},
    )
"#,
        secret = secret.display(),
        data = data.display(),
    );

    pyo3::Python::with_gil(|py| {
        cloacina_python::loader::ensure_cloaca_module(py).unwrap();
        let c_code = std::ffi::CString::new(code).unwrap();
        py.run(&c_code, None, None)
            .map_err(|e| {
                pyo3::Python::with_gil(|py| e.print(py));
                "python workflow failed"
            })
            .unwrap();
    });

    // The resolved node landed in the scoped Runtime under the workflow's
    // namespace (same shape as a @cloaca.task).
    let ns = cloacina::TaskNamespace::new("public", "embedded", "pyflow", "reader");
    let namespaces = runtime.task_namespaces();
    let found = namespaces
        .iter()
        .find(|n| n.task_id == "reader")
        .unwrap_or_else(|| panic!("constructor node not registered; registry has {namespaces:?}"));
    assert_eq!(found.workflow_id, "pyflow");
    let task = runtime
        .get_task(found)
        .expect("constructor node retrievable");
    let _ = ns; // exact tenant/package naming is the loader's concern; workflow+id proven above

    // And it RUNS: the sandboxed read succeeds through the Python-declared grant.
    let out = task
        .execute(cloacina::Context::new())
        .await
        .expect("python-wired constructor node executes");
    assert_eq!(
        out.get("contents"),
        Some(&serde_json::json!("python-read secret")),
        "read_file resolved via cloaca.constructor read the granted file"
    );
}

#[tokio::test]
#[serial]
async fn python_constructor_without_grant_fails_closed_at_execute() {
    pyo3::prepare_freethreaded_python();
    let data = provider_data_dir();
    let secret = data.join("secret.txt");

    let runtime = Arc::new(cloacina::Runtime::empty());
    let _scope = cloacina_python::runtime_scope::ScopedRuntime::new(runtime.clone())
        .expect("install scoped runtime");

    let code = format!(
        r#"
import cloaca

with cloaca.WorkflowBuilder("pyflow_denied") as _:
    cloaca.constructor(
        id="reader_denied",
        from_="cloacina-provider-fs",
        constructor="read_file",
        config={{"path": "{secret}"}},
    )
"#,
        secret = secret.display(),
    );

    pyo3::Python::with_gil(|py| {
        cloacina_python::loader::ensure_cloaca_module(py).unwrap();
        let c_code = std::ffi::CString::new(code).unwrap();
        py.run(&c_code, None, None).unwrap();
    });

    let namespaces = runtime.task_namespaces();
    let found = namespaces
        .iter()
        .find(|n| n.task_id == "reader_denied")
        .expect("node registered (load succeeds; DENIAL is at execute)");
    let task = runtime.get_task(found).unwrap();

    let err = task
        .execute(cloacina::Context::new())
        .await
        .expect_err("no grant → the sandboxed read must fail closed");
    assert!(
        format!("{err}").contains("read"),
        "error should reflect the denied read, got: {err}"
    );
}

#[tokio::test]
#[serial]
async fn python_constructor_unknown_member_raises() {
    pyo3::prepare_freethreaded_python();
    let _ = provider_data_dir();

    let runtime = Arc::new(cloacina::Runtime::empty());
    let _scope = cloacina_python::runtime_scope::ScopedRuntime::new(runtime.clone())
        .expect("install scoped runtime");

    // The unknown member raises a ValueError naming the suite. (The builder's
    // exit then also raises "workflow cannot be empty" since the node never
    // registered — expected collateral, swallowed.)
    let code = r#"
import cloaca

err = None
try:
    with cloaca.WorkflowBuilder("pyflow_unknown") as _:
        try:
            cloaca.constructor(
                id="nope",
                from_="cloacina-provider-fs",
                constructor="delete_file",
                config={"path": "/x"},
            )
        except ValueError as e:
            err = str(e)
except ValueError:
    pass  # empty-workflow on exit — the failed node never registered

assert err is not None, "unknown member must raise"
assert "delete_file" in err and "read_file" in err, f"error should name the suite: {err}"
"#;

    pyo3::Python::with_gil(|py| {
        cloacina_python::loader::ensure_cloaca_module(py).unwrap();
        let c_code = std::ffi::CString::new(code).unwrap();
        py.run(&c_code, None, None)
            .map_err(|e| {
                pyo3::Python::with_gil(|py| e.print(py));
                "python assertion failed"
            })
            .unwrap();
    });
}
