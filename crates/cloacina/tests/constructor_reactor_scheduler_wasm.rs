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

//! CLOACI-T-0830 — end-to-end: a reactor authored as a WASM **constructor** fires
//! via the CG SCHEDULER's package-load path, not just via a direct
//! `Reactor::with_evaluator(...)` harness (that direct seam is proven in
//! `constructor_reactor_wasm.rs`).
//!
//! Proves the gap T-0830 closed:
//!   1. a `ComputationGraphDeclaration` carries a `ReactorConstructorRef`
//!      (`from`/`constructor`/`config`) on its `ReactorDeclaration`;
//!   2. `ComputationGraphScheduler::load_graph` resolves that ref against the
//!      T-0829 provider search path, binds `config` BY NAME, and installs the WASM
//!      guest's `evaluate` as the reactor's firing decider (`with_evaluator`);
//!   3. boundaries pushed through the normal accumulator → reactor path fire the
//!      graph ONLY when the WASM guest says so (the gate config decides), with the
//!      built-in WhenAny/WhenAll criteria fully replaced.
//!
//! Reuses the `reactor-constructor-fixture` (a `#[constructor(kind = reactor)]`
//! gate: fire when boundary `x.value >= gate`).
//!
//! Feature-gated (`constructors-wasm`, which pulls wasmtime). Excluded from the
//! default build.
#![cfg(feature = "constructors-wasm")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use cloacina::cloacina_computation_graph::ReactorConstructorRef;
use cloacina::computation_graph::reactor::{CompiledGraphFn, InputStrategy, ReactionCriteria};
use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::{
    AccumulatorDeclaration, AccumulatorFactory, AccumulatorSpawnConfig,
    ComputationGraphDeclaration, ComputationGraphScheduler, ReactorDeclaration,
};
use cloacina::computation_graph::types::{GraphResult, InputCache, SourceName};
use cloacina::computation_graph::Accumulator;
use cloacina::registry::loader::constructor_loader::{
    clear_provider_search_path, set_provider_search_path,
};

// ---------------------------------------------------------------------------
// Fixture build + staging (mirrors constructor_reactor_wasm.rs)
// ---------------------------------------------------------------------------

const PROVIDER_PACKAGE: &str = "reactor-constructor-pkg";

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/reactor-constructor-fixture")
}

fn component() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let fixture = fixture_dir();
        let status = Command::new("cargo")
            .args(["build", "--lib", "--target", "wasm32-wasip2", "--release"])
            .current_dir(&fixture)
            .status()
            .expect("spawn cargo build --target wasm32-wasip2");
        assert!(
            status.success(),
            "reactor-constructor-fixture wasm build failed"
        );
        std::fs::read(fixture.join("target/wasm32-wasip2/release/reactor_constructor_fixture.wasm"))
            .expect("read built wasm component")
    })
}

fn macro_manifest_json() -> &'static str {
    static JSON: OnceLock<String> = OnceLock::new();
    JSON.get_or_init(|| {
        let out = Command::new("cargo")
            .args(["run", "--quiet", "--bin", "emit_manifest"])
            .current_dir(fixture_dir())
            .output()
            .expect("spawn cargo run --bin emit_manifest");
        assert!(
            out.status.success(),
            "emit_manifest failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).expect("emit_manifest stdout is UTF-8")
    })
}

/// Stage the built component + sidecar manifest as a provider package under
/// `root/reactor-constructor-pkg/`, the layout the provider search path expects.
fn stage(root: &Path) {
    let dir = root.join(PROVIDER_PACKAGE);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("reactor_constructor_fixture.wasm"), component()).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nname = \"reactor-constructor-pkg\"\nversion = \"0.1.0\"\n\
         interface = \"reactor-constructor\"\ninterface_version = 1\nruntime = \"wasm\"\n\n\
         [metadata]\ncategory = \"constructor\"\n\n\
         [wasm]\ncomponent = \"reactor_constructor_fixture.wasm\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("constructor.json"), macro_manifest_json()).unwrap();
}

// ---------------------------------------------------------------------------
// A minimal passthrough accumulator factory: raw event bytes → boundary frame.
// The reactor's `should_fire` decodes `bincode(json_bytes)` boundary frames, so
// emitting the raw JSON bytes is exactly what the WASM `evaluate` expects.
// ---------------------------------------------------------------------------

struct PassthroughFactory;

impl AccumulatorFactory for PassthroughFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        use cloacina::computation_graph::accumulator::{
            accumulator_runtime, AccumulatorContext, AccumulatorRuntimeConfig, BoundarySender,
        };

        let (socket_tx, socket_rx) = mpsc::channel(64);

        struct Passthrough;
        #[async_trait::async_trait]
        impl Accumulator for Passthrough {
            type Output = Vec<u8>;
            fn process(&mut self, event: Vec<u8>) -> Option<Vec<u8>> {
                Some(event)
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

/// Drive a graph whose reactor is a WASM `#[constructor(kind = reactor)]` gate,
/// loaded + fired entirely through the scheduler. `gate` is bound BY NAME via the
/// declaration's `config`; the graph fires only when the guest's `evaluate` says so.
#[tokio::test]
async fn reactor_constructor_fires_via_scheduler() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());
    set_provider_search_path(tmp.path());

    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry.clone());

    let fire_count = Arc::new(AtomicU32::new(0));
    let fire_count_inner = fire_count.clone();
    let graph_fn: CompiledGraphFn = Arc::new(move |_cache: InputCache| {
        let fc = fire_count_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            GraphResult::completed(vec![])
        })
    });

    let decl = ComputationGraphDeclaration {
        name: "t0830_reactor_constructor_graph".to_string(),
        accumulators: vec![AccumulatorDeclaration {
            name: "x".to_string(),
            factory: Arc::new(PassthroughFactory),
        }],
        reactor: ReactorDeclaration {
            // criteria is vestigial here — the WASM evaluate replaces it.
            criteria: ReactionCriteria::WhenAny,
            strategy: InputStrategy::Latest,
            graph_fn,
            // The T-0830 reactor-constructor reference: gate = 5.0, bound BY NAME.
            constructor: Some(ReactorConstructorRef {
                from: PROVIDER_PACKAGE.to_string(),
                constructor: "gate".to_string(),
                config: vec![("gate".to_string(), serde_json::json!(5.0))],
            }),
        },
        tenant_id: None,
        reactor_name: None,
        topology: None,
    };

    scheduler
        .load_graph(decl)
        .await
        .expect("load_graph with reactor constructor should resolve + install the WASM evaluator");

    // Below the gate (1.0) → the WASM guest holds → no fire.
    registry
        .send_to_accumulator(
            "x",
            serde_json::to_vec(&serde_json::json!({ "value": 1.0 })).unwrap(),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(
        fire_count.load(Ordering::SeqCst),
        0,
        "below-gate boundary must NOT fire the graph (WASM evaluate gates it)"
    );

    // Above the gate (9.0) → the WASM guest fires → graph runs via the scheduler.
    registry
        .send_to_accumulator(
            "x",
            serde_json::to_vec(&serde_json::json!({ "value": 9.0 })).unwrap(),
        )
        .await
        .unwrap();

    let mut fired = false;
    for _ in 0..50 {
        if fire_count.load(Ordering::SeqCst) >= 1 {
            fired = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        fired,
        "above-gate boundary should fire the graph via the scheduler-installed WASM evaluator"
    );

    clear_provider_search_path();
}

/// A reactor constructor whose declared name doesn't match the provider's
/// `constructor.json` name fails the load closed — the author asked for a
/// constructor the provider does not carry.
#[tokio::test]
async fn reactor_constructor_name_mismatch_fails_closed() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());
    set_provider_search_path(tmp.path());

    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry.clone());

    let graph_fn: CompiledGraphFn =
        Arc::new(|_c: InputCache| Box::pin(async move { GraphResult::completed(vec![]) }));

    let decl = ComputationGraphDeclaration {
        name: "t0830_mismatch_graph".to_string(),
        accumulators: vec![AccumulatorDeclaration {
            name: "x".to_string(),
            factory: Arc::new(PassthroughFactory),
        }],
        reactor: ReactorDeclaration {
            criteria: ReactionCriteria::WhenAny,
            strategy: InputStrategy::Latest,
            graph_fn,
            constructor: Some(ReactorConstructorRef {
                from: PROVIDER_PACKAGE.to_string(),
                constructor: "not_the_gate".to_string(),
                config: vec![("gate".to_string(), serde_json::json!(5.0))],
            }),
        },
        tenant_id: None,
        reactor_name: None,
        topology: None,
    };

    let err = scheduler
        .load_graph(decl)
        .await
        .expect_err("a constructor-name mismatch must fail the load closed");
    assert!(
        err.contains("not_the_gate") || err.contains("gate"),
        "error should name the requested/resolved constructor, got: {err}"
    );

    clear_provider_search_path();
}
