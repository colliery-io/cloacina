/*
 *  Copyright 2025-2026 Colliery Software
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

//! T-0545 M2 — Python "reactor library" package routing.
//!
//! A Python module declares only `@cloaca.reactor` classes (no `@task`, no
//! `ComputationGraphBuilder`). The decorator side-effect populates the scoped
//! Runtime's reactor registry. The new
//! [`dispatch_runtime_reactors_into_scheduler`] helper walks that registry
//! and dispatches each entry into a `ComputationGraphScheduler` via
//! `load_reactor` — making the reactor instance actually live without
//! requiring a co-located CG subscriber. This is the runtime-side proof
//! that "a reactor declared anywhere just works."
//!
//! Loader integration (calling this helper from inside
//! `import_and_register_python_workflow_named` / the reconciler) lands in
//! T-0545 M3 alongside the Rust packaged path.

use std::sync::Arc;

use pyo3::ffi::c_str;
use pyo3::prelude::*;

use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::ComputationGraphScheduler;

use cloacina_python::reactor as py_reactor;
use cloacina_python::runtime_scope::ScopedRuntime;

#[tokio::test(flavor = "multi_thread")]
async fn test_python_reactor_library_dispatches_into_scheduler() {
    pyo3::prepare_freethreaded_python();

    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry.clone());

    let rt = Arc::new(cloacina::Runtime::empty());
    let _scope = ScopedRuntime::new(rt.clone()).unwrap();

    // ---- Drive a "reactor library" Python module: decorator only ----
    Python::with_gil(|py| {
        let locals = pyo3::types::PyDict::new(py);
        locals
            .set_item(
                "reactor",
                pyo3::wrap_pyfunction!(py_reactor::reactor, py).unwrap(),
            )
            .unwrap();

        py.run(
            c_str!(
                r#"
@reactor(name="lib_rx_a", accumulators=["alpha"], mode="when_any")
class LibRxA: pass

@reactor(name="lib_rx_b", accumulators=["beta", "gamma"], mode="when_all")
class LibRxB: pass
"#
            ),
            None,
            Some(&locals),
        )
        .unwrap();
    });

    // Both reactors registered in the runtime; nothing in the scheduler yet.
    let mut runtime_names = rt.reactor_names();
    runtime_names.sort();
    assert_eq!(
        runtime_names,
        vec!["lib_rx_a".to_string(), "lib_rx_b".to_string()]
    );
    assert!(scheduler.list_graphs().await.is_empty());
    assert!(registry.get_reactor_handle("lib_rx_a").await.is_none());

    // ---- Dispatch into the scheduler ----
    let dispatched =
        py_reactor::dispatch_runtime_reactors_into_scheduler(&rt, &scheduler, &[], None)
            .await
            .expect("dispatch should succeed");
    let mut dispatched_sorted = dispatched.clone();
    dispatched_sorted.sort();
    assert_eq!(
        dispatched_sorted,
        vec!["lib_rx_a".to_string(), "lib_rx_b".to_string()]
    );

    // Both reactors are now addressable in the endpoint registry under their
    // own names (no aliasing — `register_aliases: vec![]`).
    assert!(
        registry.get_reactor_handle("lib_rx_a").await.is_some(),
        "lib_rx_a should be registered in the endpoint registry"
    );
    assert!(
        registry.get_reactor_handle("lib_rx_b").await.is_some(),
        "lib_rx_b should be registered in the endpoint registry"
    );

    // No graphs bound yet.
    assert!(scheduler.list_graphs().await.is_empty());

    // ---- Idempotent re-dispatch ----
    py_reactor::dispatch_runtime_reactors_into_scheduler(&rt, &scheduler, &[], None)
        .await
        .expect("idempotent re-dispatch should succeed");

    // ---- Cleanup ----
    scheduler.unload_reactor("lib_rx_a").await.unwrap();
    scheduler.unload_reactor("lib_rx_b").await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_python_reactor_library_then_bind_graph() {
    use std::sync::atomic::{AtomicU32, Ordering};

    use cloacina::computation_graph::types::InputCache;
    use cloacina_computation_graph::CompiledGraphFn;

    pyo3::prepare_freethreaded_python();
    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry.clone());

    let rt = Arc::new(cloacina::Runtime::empty());
    let _scope = ScopedRuntime::new(rt.clone()).unwrap();

    // ---- Step 1: a Python "reactor library" module brings up a reactor
    // ----  that has no co-located CG subscriber.
    Python::with_gil(|py| {
        let locals = pyo3::types::PyDict::new(py);
        locals
            .set_item(
                "reactor",
                pyo3::wrap_pyfunction!(py_reactor::reactor, py).unwrap(),
            )
            .unwrap();
        py.run(
            c_str!(
                r#"
@reactor(name="cross_pkg_rx", accumulators=["alpha"], mode="when_any")
class CrossPkgRx: pass
"#
            ),
            None,
            Some(&locals),
        )
        .unwrap();
    });
    py_reactor::dispatch_runtime_reactors_into_scheduler(&rt, &scheduler, &[], None)
        .await
        .unwrap();

    // ---- Step 2: a separate caller binds a graph to the reactor by name
    // ----  via the M1 explicit API. Mimics what the reconciler does when a
    // ----  later CG package references a reactor that's already loaded.
    let fires = Arc::new(AtomicU32::new(0));
    let fires_inner = fires.clone();
    let graph_fn: CompiledGraphFn = Arc::new(move |_cache: InputCache| {
        let fc = fires_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            cloacina::computation_graph::GraphResult::completed(vec![])
        })
    });
    scheduler
        .bind_graph_to_reactor("late_g".to_string(), "cross_pkg_rx".to_string(), graph_fn)
        .await
        .expect("bind should succeed against the Python-declared reactor");

    // ---- Step 3: pushing an event fires the late-bound graph.
    registry
        .send_to_accumulator(
            "alpha",
            serde_json::to_vec(&serde_json::json!({ "value": 7.0 })).unwrap(),
        )
        .await
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline && fires.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(
        fires.load(Ordering::SeqCst),
        1,
        "late-bound graph should fire"
    );

    scheduler.unbind_graph_from_reactor("late_g").await.unwrap();
    scheduler.unload_reactor("cross_pkg_rx").await.unwrap();
}
