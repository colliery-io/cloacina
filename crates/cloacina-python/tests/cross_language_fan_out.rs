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

//! T-0544 M5 — Cross-language cross-package fan-out.
//!
//! Two "packages" — one Rust-shaped (a hand-built `GraphPackageMetadata`
//! plus a `CompiledGraphFn`, run through `build_declaration_from_ffi`) and
//! one Python-shaped (a `@cloaca.reactor` class + `ComputationGraphBuilder`
//! driven through `build_python_graph_declaration`) — both name reactor
//! `"shared_rx"`. Loaded into one scheduler, M2's idempotent path collapses
//! them onto a single reactor instance. One event into the accumulator,
//! both subscribers fire.
//!
//! This is the runtime-side proof that the M5 wire-format change
//! (`GraphPackageMetadata.trigger_reactor` + Python
//! `PythonGraphExecutor.reactor_name`) makes shared-reactor binding visible
//! to packages without introducing a special "reactor package" shape — the
//! reactor declaration in any package "just works."

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use pyo3::ffi::c_str;
use pyo3::prelude::*;

use cloacina::computation_graph::accumulator::{
    accumulator_runtime, Accumulator, AccumulatorContext, AccumulatorRuntimeConfig, BoundarySender,
};
use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::{
    AccumulatorDeclaration, AccumulatorFactory, AccumulatorSpawnConfig, ComputationGraphScheduler,
};
use cloacina::computation_graph::types::{GraphResult, InputCache, SourceName};
use cloacina_workflow_plugin::{AccumulatorDeclarationEntry, GraphPackageMetadata};

use cloacina_python::computation_graph::{
    self as py_cg, build_python_graph_declaration, PyComputationGraphBuilder,
};
use cloacina_python::reactor as py_reactor;
use cloacina_python::runtime_scope::ScopedRuntime;

/// Minimal passthrough accumulator factory shared by both subscribers.
/// Mirrors the `TestAccumulatorFactory` used in the cloacina integration
/// tests but lives here so this crate can drive it directly.
struct PassthroughTestFactory;

impl AccumulatorFactory for PassthroughTestFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: tokio::sync::mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: tokio::sync::watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (
        tokio::sync::mpsc::Sender<Vec<u8>>,
        tokio::task::JoinHandle<()>,
    ) {
        let (socket_tx, socket_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

        struct Passthrough;

        #[async_trait::async_trait]
        impl Accumulator for Passthrough {
            type Output = serde_json::Value;
            fn process(&mut self, event: Vec<u8>) -> Option<serde_json::Value> {
                serde_json::from_slice(&event).ok()
            }
        }

        let sender = BoundarySender::new(boundary_tx, SourceName::new(&name));
        let ctx = AccumulatorContext {
            output: sender,
            name: name.clone(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: config.health_tx,
        };
        let handle = tokio::spawn(accumulator_runtime(
            Passthrough,
            ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
        ));
        (socket_tx, handle)
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_language_fan_out_via_shared_reactor_name() {
    pyo3::prepare_freethreaded_python();

    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry.clone());

    let rust_fires = Arc::new(AtomicU32::new(0));
    let py_fires = Arc::new(AtomicU32::new(0));

    // ---- Rust-shaped declaration via build_declaration_from_ffi ----
    let rust_inner = rust_fires.clone();
    let rust_graph_fn: cloacina_computation_graph::CompiledGraphFn =
        Arc::new(move |_cache: InputCache| {
            let fc = rust_inner.clone();
            Box::pin(async move {
                fc.fetch_add(1, Ordering::SeqCst);
                GraphResult::completed(vec![])
            })
        });
    let rust_meta = GraphPackageMetadata {
        graph_name: "rust_g".to_string(),
        package_name: "rust-pkg".to_string(),
        reaction_mode: "when_any".to_string(),
        input_strategy: "latest".to_string(),
        accumulators: vec![AccumulatorDeclarationEntry {
            name: "alpha".to_string(),
            accumulator_type: "passthrough".to_string(),
            config: std::collections::HashMap::new(),
        }],
        // The M5 wire-format change: this package opts the graph into the
        // shared-reactor binding by naming "shared_rx".
        trigger_reactor: Some("shared_rx".to_string()),
    };
    // We don't need the FFI plugin path here — `build_declaration_from_ffi`
    // gracefully degrades when the library bytes don't load (it returns a
    // fn that errors). Replace `decl.reactor.graph_fn` with our local
    // counter-incrementing fn so the reactor invokes it on firing.
    let mut rust_decl = cloacina::computation_graph::packaging_bridge::build_declaration_from_ffi(
        &rust_meta,
        vec![],
    );
    rust_decl.reactor.graph_fn = rust_graph_fn;
    rust_decl.accumulators = vec![AccumulatorDeclaration {
        name: "alpha".to_string(),
        factory: Arc::new(PassthroughTestFactory),
    }];

    assert_eq!(rust_decl.reactor_name.as_deref(), Some("shared_rx"));

    // ---- Python-shaped declaration via @cloaca.reactor + builder ----
    // Install a scoped runtime so @cloaca.reactor's registration finds a
    // Runtime to attach to (decorator side effect).
    let rt = Arc::new(cloacina::Runtime::empty());
    let _scope = ScopedRuntime::new(rt.clone()).unwrap();

    Python::with_gil(|py| {
        let locals = pyo3::types::PyDict::new(py);
        locals
            .set_item("node", pyo3::wrap_pyfunction!(py_cg::node, py).unwrap())
            .unwrap();
        locals
            .set_item(
                "ComputationGraphBuilder",
                py.get_type::<PyComputationGraphBuilder>(),
            )
            .unwrap();
        locals
            .set_item(
                "reactor",
                pyo3::wrap_pyfunction!(py_reactor::reactor, py).unwrap(),
            )
            .unwrap();

        py.run(
            c_str!(
                r#"
@reactor(name="shared_rx", accumulators=["alpha"], mode="when_any")
class SharedRx: pass

with ComputationGraphBuilder("py_g", reactor=SharedRx, graph={
    "entry": {"inputs": ["alpha"]},
}) as builder:

    @node
    def entry(alpha):
        return {"py_terminal": True}
"#
            ),
            None,
            Some(&locals),
        )
        .unwrap();
    });

    let mut py_decl = build_python_graph_declaration("py_g", None, &[])
        .expect("python build_python_graph_declaration should produce a declaration");
    assert_eq!(py_decl.reactor_name.as_deref(), Some("shared_rx"));

    // Replace the python graph_fn with a counter-incrementing wrapper so we
    // can observe firings without depending on the executor's serializer.
    let original_py_fn = py_decl.reactor.graph_fn.clone();
    let py_inner = py_fires.clone();
    py_decl.reactor.graph_fn = Arc::new(move |cache: InputCache| {
        let fc = py_inner.clone();
        let inner = original_py_fn.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            // Drive the underlying Python executor too — proves the bridge
            // doesn't blow up under fan-out, even though we don't read its
            // terminal outputs in this test.
            let _ = inner(cache).await;
            GraphResult::completed(vec![])
        })
    });
    py_decl.accumulators = vec![AccumulatorDeclaration {
        name: "alpha".to_string(),
        factory: Arc::new(PassthroughTestFactory),
    }];

    // ---- Load both into one scheduler ----
    scheduler
        .load_graph(rust_decl)
        .await
        .expect("rust graph loads");
    scheduler
        .load_graph(py_decl)
        .await
        .expect("python graph binds to the same shared_rx reactor (M2 idempotent path)");

    // Both graphs should appear in list_graphs.
    let mut listed: Vec<String> = scheduler
        .list_graphs()
        .await
        .into_iter()
        .map(|s| s.name)
        .collect();
    listed.sort();
    assert_eq!(listed, vec!["py_g".to_string(), "rust_g".to_string()]);

    // ---- One event → both subscribers fire ----
    registry
        .send_to_accumulator(
            "alpha",
            serde_json::to_vec(&serde_json::json!({ "value": 42.0 })).unwrap(),
        )
        .await
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < deadline
        && (rust_fires.load(Ordering::SeqCst) == 0 || py_fires.load(Ordering::SeqCst) == 0)
    {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(
        rust_fires.load(Ordering::SeqCst),
        1,
        "rust subscriber should have fired exactly once"
    );
    assert_eq!(
        py_fires.load(Ordering::SeqCst),
        1,
        "python subscriber should have fired exactly once"
    );

    // Cleanup — explicit unbind + unload_reactor, exercising the M4 lifecycle
    // primitives.
    scheduler.unbind_graph_from_reactor("rust_g").await.unwrap();
    scheduler.unbind_graph_from_reactor("py_g").await.unwrap();
    scheduler.unload_reactor("shared_rx").await.unwrap();
}
