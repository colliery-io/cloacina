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

//! Computation graph scheduler — spawns, supervises, and shuts down
//! accumulator/reactor tasks from computation graph declarations.
//!
//! The companion to the Unified Scheduler for the computation graph
//! primitive. Receives declarations from the reconciler, wires channels,
//! spawns tokio tasks, registers endpoints, and restarts tasks on panic.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use super::accumulator::{health_channel, shutdown_signal, AccumulatorHealth};
use super::reactor::{
    reactor_health_channel, CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor,
    ReactorFireDecider, ReactorHandle,
};
use super::registry::{AccumulatorAuthPolicy, EndpointRegistry, ReactorAuthPolicy};
use super::types::{GraphResult, InputCache, SourceName};
use crate::tenant_scope::{resolve_tenant_key, TenantKey, TenantScope};

/// Declaration of a computation graph to be loaded by the [`ComputationGraphScheduler`].
#[derive(Clone)]
pub struct ComputationGraphDeclaration {
    /// Unique name for this computation graph.
    pub name: String,
    /// Accumulator declarations.
    pub accumulators: Vec<AccumulatorDeclaration>,
    /// Reactor declaration.
    pub reactor: ReactorDeclaration,
    /// Tenant that owns this graph (None = global/public).
    pub tenant_id: Option<String>,
    /// Explicit reactor name. When `Some(name)`, multiple graph declarations
    /// referencing the same reactor name share a single reactor instance —
    /// the second `load_graph` call with a matching contract is idempotent
    /// on the reactor and just binds the new graph as an additional
    /// subscriber. `None` (today's bundled-form default) synthesizes a
    /// per-graph reactor name (`__Reactor_<graph_name>`) to preserve the
    /// 1:1 reactor-per-graph behavior callers expect.
    pub reactor_name: Option<String>,
    /// Serialized node/edge topology JSON for this graph (from the package's
    /// FFI metadata), retained so the health API can surface the CG DAG.
    /// `None` for packages predating topology emission. (CLOACI-T-0673)
    pub topology: Option<String>,
}

/// Declaration for a single accumulator.
#[derive(Clone)]
pub struct AccumulatorDeclaration {
    /// Accumulator name (used as WebSocket endpoint name).
    pub name: String,
    /// Factory that creates the accumulator instance.
    pub factory: Arc<dyn AccumulatorFactory>,
}

/// Configuration passed to [`AccumulatorFactory::spawn`] for resilience wiring.
pub struct AccumulatorSpawnConfig {
    /// DAL handle for checkpoint persistence. None in embedded/test mode.
    pub dal: Option<crate::dal::unified::DAL>,
    /// Health state reporter. None when health tracking is not needed.
    pub health_tx: Option<watch::Sender<AccumulatorHealth>>,
    /// Graph name (used as key for checkpoint persistence).
    pub graph_name: String,
    /// Shared freshness probe for the accumulator's BoundarySender (CLOACI-T-0765).
    /// The factory builds the sender via `BoundarySender::with_freshness` so the
    /// registry can report events_total + last-event for this source.
    pub freshness: super::accumulator::FreshnessHandle,
}

/// Factory trait for creating accumulator instances.
///
/// We can't clone trait objects, so we use a factory that produces them.
pub trait AccumulatorFactory: Send + Sync {
    /// Create a new accumulator instance and its runtime components.
    ///
    /// Returns:
    /// - socket_tx: sender for the accumulator's socket channel
    /// - join_handle: spawned task handle
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>);
}

/// Declaration for the reactor.
#[derive(Clone)]
pub struct ReactorDeclaration {
    /// Reaction criteria (when_any / when_all).
    ///
    /// Ignored at runtime when `constructor` is `Some(..)` — a reactor
    /// constructor's WASM `evaluate` replaces the dirty-flag criteria.
    pub criteria: ReactionCriteria,
    /// Input strategy (latest / sequential).
    pub strategy: InputStrategy,
    /// The compiled graph function.
    pub graph_fn: CompiledGraphFn,
    /// Optional packaged WASM reactor-constructor reference (CLOACI-T-0830).
    ///
    /// `Some(..)` makes the named constructor's WASM `evaluate` the reactor's
    /// firing decision: [`load_reactor`](ComputationGraphScheduler::load_reactor)
    /// resolves it against the T-0829 provider search path and installs it via
    /// [`Reactor::with_evaluator`], replacing the built-in `criteria`. `None`
    /// (the default for every existing path) is the native dirty-flag reactor.
    pub constructor: Option<cloacina_computation_graph::ReactorConstructorRef>,
}

/// Status of a managed computation graph.
#[derive(Debug, Clone)]
pub struct GraphStatus {
    pub name: String,
    pub accumulators: Vec<String>,
    pub paused: bool,
    pub running: bool,
    /// Reactor health state machine value. None if health tracking is not configured.
    pub health: Option<super::reactor::ReactorHealth>,
    /// Tenant scope of the graph at load time. `None` for single-tenant or
    /// admin-owned graphs. CLOACI-T-0579: surfaced so per-tenant health
    /// endpoints can filter by caller authorization.
    pub tenant_id: Option<String>,
    /// Serialized node/edge topology JSON for this graph, so the health API can
    /// render the CG DAG. `None` for graphs predating topology emission. (CLOACI-T-0673)
    pub topology: Option<String>,
    /// Name of the reactor this graph is bound to (the trigger that fires it).
    /// `None` only if no reactor identity was recorded. (CLOACI-T-0673 follow-up)
    pub reactor: Option<String>,
    /// Reaction mode of the bound reactor: `"when_any"` | `"when_all"`.
    pub reaction_mode: String,
    /// Input strategy of the bound reactor: `"latest"` | `"sequential"`.
    pub input_strategy: String,
    /// Total graph fires since load (live reactor counter, WS-10).
    pub fires: u64,
    /// Unix-epoch millis of the last fire; `None` if it hasn't fired yet.
    pub last_fire_unix_ms: Option<i64>,
}

/// Status of a managed reactor (CLOACI-T-0742). Reactors are first-class: a
/// reactor is loaded (`load_reactor`) and graphs bind to it afterward
/// (`bind_graph_to_reactor`), so a reactor can be running with **no graph
/// bound**. `list_graphs` is graph-first and would omit such a reactor; this is
/// reactor-first, sourced directly from the `reactors` map.
#[derive(Debug, Clone)]
pub struct ReactorStatus {
    /// Reactor name (the `reactors` map key).
    pub name: String,
    /// Accumulators this reactor consumes, in declaration order.
    pub accumulators: Vec<String>,
    /// Firing criteria: `"when_any"` | `"when_all"`.
    pub reaction_mode: String,
    /// Input strategy: `"latest"` | `"sequential"`.
    pub input_strategy: String,
    /// Graphs bound to this reactor (empty if the reactor has no graph yet).
    pub bound_graphs: Vec<String>,
    pub paused: bool,
    pub running: bool,
    /// Reactor health state machine value. None if health tracking isn't configured.
    pub health: Option<super::reactor::ReactorHealth>,
    /// Tenant scope at load time. `None` for single-tenant / admin-owned reactors.
    pub tenant_id: Option<String>,
    /// Total fires since load (live reactor counter, WS-10).
    pub fires: u64,
    /// Unix-epoch millis of the last fire; `None` if it hasn't fired yet.
    pub last_fire_unix_ms: Option<i64>,
}

/// Validate that two declarations targeting the same reactor name agree on
/// the reactor's contract. Mismatches are operator-facing errors, not silent
/// no-ops — the second package may have shipped with a different
/// accumulator set or firing criteria, and binding to the existing reactor
/// would silently drop those expectations.
fn check_reactor_contract_matches(
    existing: &ComputationGraphDeclaration,
    new: &ComputationGraphDeclaration,
) -> Result<(), String> {
    let existing_accs: Vec<&str> = existing
        .accumulators
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    let new_accs: Vec<&str> = new.accumulators.iter().map(|a| a.name.as_str()).collect();
    if existing_accs != new_accs {
        return Err(format!(
            "accumulator set differs (existing: {:?}, new: {:?})",
            existing_accs, new_accs
        ));
    }
    if existing.reactor.criteria != new.reactor.criteria {
        return Err("reaction criteria differ".to_string());
    }
    if existing.reactor.strategy != new.reactor.strategy {
        return Err("input strategy differs".to_string());
    }
    if existing.tenant_id != new.tenant_id {
        return Err(format!(
            "tenant ownership differs (existing: {:?}, new: {:?})",
            existing.tenant_id, new.tenant_id
        ));
    }
    Ok(())
}

/// Placeholder `CompiledGraphFn` used inside the synthetic anchoring
/// declaration that backs a reactor in `RunningGraph.declaration`. Never
/// invoked — the reactor's dispatcher walks the subscribers map instead.
fn dummy_graph_fn() -> CompiledGraphFn {
    Arc::new(|_cache: InputCache| Box::pin(async move { GraphResult::completed(vec![]) }))
}

/// Resolve a [`ReactorConstructorRef`](cloacina_computation_graph::ReactorConstructorRef)
/// into a live firing decider (CLOACI-T-0830).
///
/// `None` ref → `None` decider (the native dirty-flag reactor). `Some(ref)` loads
/// the named WASM reactor constructor through the T-0829 provider seam — resolving
/// `from` against the provider search path, binding `config` BY NAME, and validating
/// the resolved `constructor` name — and returns it as an
/// `Arc<dyn ReactorFireDecider>` ready for [`Reactor::with_evaluator`]. The load is
/// blocking (builds a `PluginHost`, loads + configures the wasmtime component), so it
/// runs on `spawn_blocking`.
///
/// Behind the default-OFF `constructors-wasm` feature: a ref present in a build that
/// lacks the feature fails closed with a clear error rather than silently ignoring the
/// author's firing logic.
/// `provider_root` (CLOACI-T-0925) is the provider tree the owning package's
/// bundled providers were staged into. `None` falls back to the ambient
/// [`crate::registry::loader::provider_search_path`] — the embedded/test path.
/// A multi-tenant host MUST pass `Some(..)`: this resolution runs on a
/// `spawn_blocking` thread, so an ambient read can observe whatever another
/// tenant's concurrent load left behind.
async fn resolve_reactor_evaluator(
    constructor: &Option<cloacina_computation_graph::ReactorConstructorRef>,
    provider_root: Option<std::path::PathBuf>,
) -> Result<Option<Arc<dyn ReactorFireDecider>>, String> {
    let Some(cref) = constructor else {
        return Ok(None);
    };
    let _ = &provider_root;

    #[cfg(feature = "constructors-wasm")]
    {
        let cref = cref.clone();
        let search_path =
            provider_root.unwrap_or_else(crate::registry::loader::provider_search_path);
        let decider = tokio::task::spawn_blocking(move || {
            let grants = crate::registry::loader::grants::GrantSpec::from_pairs(cref.grants);
            // CLOACI-T-0920: re-parse the author's `runtime = ".."` pin fail-closed
            // (the ref carries it as a String to keep the CG crate contract-free).
            let pin = cref
                .runtime
                .as_deref()
                .map(|lit| {
                    crate::registry::loader::parse_runtime_pin(
                        &format!("reactor constructor '{}'", cref.constructor),
                        lit,
                    )
                })
                .transpose()?;
            crate::registry::loader::constructor_loader::load_reactor_constructor_node_pinned_in(
                &search_path,
                &cref.from,
                &cref.constructor,
                cref.config,
                grants,
                pin,
            )
        })
        .await
        .map_err(|e| format!("reactor constructor load task join failed: {e}"))?
        .map_err(|e| format!("reactor constructor load failed: {e}"))?;
        Ok(Some(decider))
    }

    #[cfg(not(feature = "constructors-wasm"))]
    {
        Err(format!(
            "reactor declares constructor '{}' from provider '{}', but this build lacks \
             the 'constructors-wasm' feature required to load WASM reactor constructors",
            cref.constructor, cref.from
        ))
    }
}

/// Subscribers bound to a single reactor instance.
///
/// Today every reactor has exactly one subscriber (the bundled-form graph
/// whose declaration brought the reactor into existence). T-0544 adds the
/// scaffolding for N subscribers; M2 wires the cross-package binding path so
/// multiple graph declarations naming the same reactor share a single instance.
type ReactorSubscribers = Arc<RwLock<HashMap<String, CompiledGraphFn>>>;

/// Build the dispatcher [`CompiledGraphFn`] handed to [`Reactor::new`].
///
/// On firing, walks the current subscriber map and runs every subscriber
/// concurrently via `futures::future::join_all`. Slow subscribers don't
/// block fast ones; per-subscriber errors are logged but do not short-
/// circuit siblings — the reactor sees one `GraphResult::Completed` per
/// firing regardless of subscriber count, matching today's per-reactor
/// fire-counter accounting.
fn make_subscriber_dispatcher(
    reactor_name: String,
    subscribers: ReactorSubscribers,
) -> CompiledGraphFn {
    Arc::new(move |cache: InputCache| {
        let reactor_name = reactor_name.clone();
        let subscribers = subscribers.clone();
        Box::pin(async move {
            let snapshot: Vec<(String, CompiledGraphFn)> = subscribers
                .read()
                .await
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            // Pass 1: kick off all subscriber invocations concurrently.
            let futures = snapshot.into_iter().map(|(graph_name, graph_fn)| {
                let cache = cache.clone();
                async move {
                    let result = graph_fn(cache).await;
                    (graph_name, result)
                }
            });
            let results = futures::future::join_all(futures).await;

            // Pass 2: log per-subscriber errors + aggregate their terminal
            // outputs (CLOACI-T-0775) so the reactor records per-fire outputs.
            // No short-circuit; the reactor treats this as one firing regardless
            // of how many subscribers succeeded.
            let mut outputs_json: Vec<serde_json::Value> = Vec::new();
            for (graph_name, result) in results {
                match result {
                    GraphResult::Error(e) => {
                        tracing::error!(
                            reactor = %reactor_name,
                            graph = %graph_name,
                            "subscriber graph failed: {}",
                            e
                        );
                    }
                    GraphResult::Completed {
                        outputs_json: oj, ..
                    } => outputs_json.extend(oj),
                }
            }
            GraphResult::completed_with_json(vec![], outputs_json)
        })
    })
}

/// State for a running computation graph.
struct RunningGraph {
    /// Shutdown signal sender.
    shutdown_tx: watch::Sender<bool>,
    /// Shutdown signal receiver (cloneable, for re-spawning accumulators).
    shutdown_rx: watch::Receiver<bool>,
    /// Boundary channel sender (shared by all accumulators, for re-spawning).
    boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
    /// Accumulator task handles.
    accumulator_handles: Vec<(String, JoinHandle<()>)>,
    /// Reactor task handle.
    reactor_handle: JoinHandle<()>,
    /// Reactor handle for pause/resume queries.
    reactor_shared: ReactorHandle,
    /// Reactor health receiver for status reporting.
    reactor_health_rx: Option<watch::Receiver<super::reactor::ReactorHealth>>,
    /// Declaration (for restarts).
    declaration: ComputationGraphDeclaration,
    /// CLOACI-T-0921: the endpoint-registry ownership identity every
    /// accumulator/reactor of this graph is registered under. Carried on the
    /// running graph so the restart and unload paths re-register / deregister
    /// under exactly the same `(tenant, name)` keys they claimed at load.
    owner: super::registry::EndpointOwner,
    /// Subscribers bound to this reactor. May contain one or many graphs
    /// after T-0544 fan-out.
    subscribers: ReactorSubscribers,
    /// Endpoint-registry keys this reactor is registered under. Always
    /// includes the reactor's name; bundled/split callers via `load_graph`
    /// also register the first graph's name as an alias for back-compat
    /// with `cloacinactl reactor force-fire <graph>` (T-0544 M2 surface).
    /// All keys are deregistered when the reactor is unloaded and
    /// re-registered after a restart.
    endpoint_registry_keys: Vec<String>,
    /// Per-component consecutive failure count.
    failure_counts: HashMap<String, u32>,
    /// Timestamp of last successful operation per component (for failure count reset).
    last_success: HashMap<String, std::time::Instant>,
    /// Resolved reactor-constructor firing decider (CLOACI-T-0830). `Some(..)`
    /// when this reactor was loaded with a [`ReactorConstructorRef`]: the WASM
    /// `evaluate` was resolved ONCE at load and is shared (it is `Send + Sync`)
    /// so the supervisor reuses it on restart instead of re-loading the
    /// component. `None` is the native dirty-flag reactor.
    evaluator: Option<Arc<dyn ReactorFireDecider>>,
}

/// Maximum consecutive failures before a component is permanently abandoned.
const MAX_RECOVERY_ATTEMPTS: u32 = 5;

/// All possible label values for the `state` label on
/// `cloacina_component_health`. Used by the supervisor to ensure exactly
/// one state is `1` per (graph, component) by zeroing every other label
/// value on each tick. Keep in sync with the docs / decomposition in
/// I-0099.
const COMPONENT_HEALTH_STATES: &[&str] = &["healthy", "degraded", "starting", "stopped", "crashed"];

/// Emit the `cloacina_component_health` gauge for a single component, setting
/// `current` to `1.0` and every other label value in
/// [`COMPONENT_HEALTH_STATES`] to `0.0`. Centralizing this here keeps the
/// "exactly one state per (graph, component)" invariant from drifting as
/// new emit sites are added.
fn emit_component_health(graph: &str, component: &str, current: &'static str) {
    for state in COMPONENT_HEALTH_STATES {
        let value = if *state == current { 1.0 } else { 0.0 };
        metrics::gauge!(
            "cloacina_component_health",
            "graph" => graph.to_string(),
            "component" => component.to_string(),
            "state" => *state,
        )
        .set(value);
    }
}

/// Classify a finished [`JoinHandle`] result into the bounded `reason`
/// label values for `cloacina_supervisor_restarts_total`.
///
/// Only `panic` and `error` are observable from the supervisor; the
/// `shutdown_timeout` variant is emitted by the graceful-shutdown path
/// (I-0099 / T-0585).
fn classify_join_result(result: Result<(), tokio::task::JoinError>) -> &'static str {
    match result {
        Ok(_) => "error",
        Err(e) if e.is_panic() => "panic",
        Err(_) => "error",
    }
}

/// Base delay for exponential backoff (doubles on each failure, capped at 60s).
const BACKOFF_BASE_SECS: u64 = 1;

/// Maximum backoff delay.
const BACKOFF_MAX_SECS: u64 = 60;

/// Duration of successful operation before failure counter resets.
const SUCCESS_RESET_SECS: u64 = 60;

/// A restart decided in phase 1 of
/// [`ComputationGraphScheduler::check_and_restart_failed`] (under the
/// reactors write lock) and executed afterwards with the lock released
/// (CLOACI-T-0915): the backoff sleep and recovery-event write must never
/// happen while the lock is held, or every list/health reader blocks behind
/// them for up to [`BACKOFF_MAX_SECS`] during a restart storm.
enum PlannedRestart {
    /// The reactor task exited — full-graph restart (new channels,
    /// re-spawned accumulators + reactor).
    Reactor {
        /// Full `(tenant, name)` key of the reactor to restart
        /// (CLOACI-T-0924).
        reactor_key: TenantKey,
        /// `"{reactor}::reactor"` — recovery-event component key.
        component_key: String,
        /// Failure count at detection time (drives the recovery event).
        attempt: u32,
        backoff_secs: u64,
        /// The finished handle, taken in phase 1 so phase 2 can classify
        /// panic-vs-error without holding the lock.
        dead: JoinHandle<()>,
    },
    /// An accumulator task exited — in-place respawn.
    Accumulator {
        reactor_key: TenantKey,
        acc_name: String,
        component_key: String,
        attempt: u32,
        backoff_secs: u64,
        dead: JoinHandle<()>,
    },
}

/// The computation graph scheduler: loads reactors and computation graphs,
/// supervises them, and routes operational commands to running instances.
pub struct ComputationGraphScheduler {
    /// Endpoint registry for WebSocket routing.
    registry: EndpointRegistry,
    /// Where reactor firings execute (CLOACI-T-0722): in-process by default;
    /// the server swaps in its fleet executor under `--default-executor
    /// fleet`. Applied to every reactor spawned (and re-spawned) after set.
    graph_executor: Arc<RwLock<Arc<dyn super::graph_executor::GraphExecutor>>>,
    /// Running reactors, keyed by `(tenant, reactor name)`. Each reactor owns a
    /// subscriber map that may contain one or more graphs sharing this reactor
    /// instance.
    ///
    /// CLOACI-T-0924: ONE scheduler `Arc` serves every tenant
    /// (`cloacina-server`'s `TenantRunnerCache` installs the same instance on
    /// every per-tenant runner), so the tenant has to be in the key — bare
    /// names let two tenants' same-named reactors collide in-process.
    /// `tenant_id: None` is the embedded/untenanted entry.
    reactors: Arc<RwLock<HashMap<TenantKey, RunningGraph>>>,
    /// Maps graph key → reactor key so external operations that take a
    /// graph_name (`unload_graph`, `list_graphs`) can find the reactor that
    /// hosts it. The *value* is a full key, not a name, so a subscriber in one
    /// tenant that bound to an untenanted upstream reactor still points at the
    /// exact reactor entry it bound to.
    graph_to_reactor: Arc<RwLock<HashMap<TenantKey, TenantKey>>>,
    /// Maps graph key → serialized node/edge topology JSON, captured from the
    /// declaration at load so the health API can surface the CG DAG without
    /// digging through the synthetic per-reactor anchor declaration.
    /// (CLOACI-T-0673)
    graph_topologies: Arc<RwLock<HashMap<TenantKey, String>>>,
    /// DAL handle for persistence. None in embedded/test mode.
    dal: Option<crate::dal::unified::DAL>,
}

impl ComputationGraphScheduler {
    pub fn new(registry: EndpointRegistry) -> Self {
        Self {
            registry,
            graph_executor: Arc::new(RwLock::new(
                super::graph_executor::in_process_graph_executor(),
            )),
            reactors: Arc::new(RwLock::new(HashMap::new())),
            graph_to_reactor: Arc::new(RwLock::new(HashMap::new())),
            graph_topologies: Arc::new(RwLock::new(HashMap::new())),
            dal: None,
        }
    }

    /// Create a scheduler with DAL support for persistence and health tracking.
    pub fn with_dal(registry: EndpointRegistry, dal: crate::dal::unified::DAL) -> Self {
        Self {
            registry,
            graph_executor: Arc::new(RwLock::new(
                super::graph_executor::in_process_graph_executor(),
            )),
            reactors: Arc::new(RwLock::new(HashMap::new())),
            graph_to_reactor: Arc::new(RwLock::new(HashMap::new())),
            graph_topologies: Arc::new(RwLock::new(HashMap::new())),
            dal: Some(dal),
        }
    }

    /// Load and start a reactor with no subscribers.
    ///
    /// Idempotent on `(reactor_name, contract)`: if a reactor with this name
    /// is already running and the contract matches (accumulators, criteria,
    /// strategy, tenant_id), this returns `Ok(())` without spawning anything.
    /// A mismatched contract returns a precise error.
    ///
    /// `register_aliases` lets the caller register additional endpoint-registry
    /// keys pointing at this reactor's manual command channel — used by
    /// [`load_graph`] to alias the first graph's name for back-compat with
    /// today's `cloacinactl reactor force-fire <graph>` operator surface.
    /// Direct callers (e.g. T-0545's reconciler routing for reactor-only
    /// packages) typically pass `&[]` and address the reactor by its name.
    ///
    /// Subscribers are bound separately via [`bind_graph_to_reactor`].
    /// Swap the graph executor firings run through (CLOACI-T-0722). Takes
    /// effect for reactors spawned/restarted AFTER the call — the server sets
    /// this once at startup, before any packages load.
    pub async fn set_graph_executor(
        &self,
        executor: Arc<dyn super::graph_executor::GraphExecutor>,
    ) {
        *self.graph_executor.write().await = executor;
    }

    pub async fn load_reactor(
        &self,
        reactor_name: String,
        accumulators: Vec<AccumulatorDeclaration>,
        criteria: ReactionCriteria,
        strategy: InputStrategy,
        tenant_id: Option<String>,
        register_aliases: Vec<String>,
        // CLOACI-T-0830: optional packaged reactor-constructor reference. When
        // `Some(..)`, the named WASM constructor's `evaluate` becomes the
        // reactor's firing decider (via `Reactor::with_evaluator`), replacing
        // the `criteria`. Resolved once here and reused across restarts.
        constructor: Option<cloacina_computation_graph::ReactorConstructorRef>,
    ) -> Result<(), String> {
        self.load_reactor_in(
            reactor_name,
            accumulators,
            criteria,
            strategy,
            tenant_id,
            register_aliases,
            constructor,
            None,
        )
        .await
    }

    /// [`load_reactor`](Self::load_reactor) resolving the reactor's constructor
    /// ref against an EXPLICIT provider tree (CLOACI-T-0925): `provider_root` is
    /// where the owning package's bundled providers were staged. `None` keeps the
    /// ambient behavior for embedded/test callers.
    #[allow(clippy::too_many_arguments)]
    pub async fn load_reactor_in(
        &self,
        reactor_name: String,
        accumulators: Vec<AccumulatorDeclaration>,
        criteria: ReactionCriteria,
        strategy: InputStrategy,
        tenant_id: Option<String>,
        register_aliases: Vec<String>,
        constructor: Option<cloacina_computation_graph::ReactorConstructorRef>,
        provider_root: Option<&std::path::Path>,
    ) -> Result<(), String> {
        let provider_root = provider_root.map(|p| p.to_path_buf());

        // CLOACI-T-0924: a load is a CLAIM, so it addresses the caller's own
        // `(tenant, name)` key exactly — never the untenanted fallback. A
        // tenant loading `R` gets its own `R` even if an untenanted `R` is
        // already running; with `tenant_id: None` (embedded) the key IS the
        // bare name, so this path is byte-for-byte what it was.
        let reactor_key = TenantKey::new(tenant_id.as_deref(), &reactor_name);

        // Idempotent path: matching contract → no-op.
        {
            let reactors = self.reactors.read().await;
            if let Some(existing) = reactors.get(&reactor_key) {
                let probe = ComputationGraphDeclaration {
                    name: reactor_name.clone(),
                    accumulators: accumulators.clone(),
                    reactor: ReactorDeclaration {
                        criteria: criteria.clone(),
                        strategy: strategy.clone(),
                        graph_fn: dummy_graph_fn(),
                        constructor: constructor.clone(),
                    },
                    tenant_id: tenant_id.clone(),
                    reactor_name: Some(reactor_name.clone()),
                    topology: None,
                };
                if let Err(e) = check_reactor_contract_matches(&existing.declaration, &probe) {
                    return Err(format!(
                        "reactor '{}' is already loaded with a different contract: {}",
                        reactor_name, e
                    ));
                }
                return Ok(());
            }
        }

        // CLOACI-T-0830: resolve the reactor-constructor reference (if any) into a
        // live firing decider BEFORE we spawn anything, so a bad constructor ref
        // fails the load cleanly instead of leaving a half-wired reactor running.
        // The resolved decider is reused on restart (stored on `RunningGraph`).
        let evaluator = resolve_reactor_evaluator(&constructor, provider_root).await?;

        // CLOACI-T-0921: every endpoint this reactor registers is keyed by
        // `(tenant, name)` and stamped with this owner, so a same-named
        // endpoint in another tenant is a separate entry and a same-named
        // endpoint owned by another reactor in the SAME tenant is rejected.
        let owner = super::registry::EndpointOwner::new(
            tenant_id.clone(),
            // Package provenance is not threaded into `load_reactor` today; the
            // reactor name is the discriminator. See CLOACI-T-0921 deferrals.
            None,
            reactor_name.clone(),
        );

        let (shutdown_tx, shutdown_rx) = shutdown_signal();
        let stored_shutdown_rx = shutdown_rx.clone();

        // Create boundary channel (all accumulators → reactor)
        let (boundary_tx, boundary_rx) = mpsc::channel(256);
        let stored_boundary_tx = boundary_tx.clone();

        // Collect expected source names for WhenAll seeding
        let expected_sources: Vec<SourceName> = accumulators
            .iter()
            .map(|a| SourceName::new(&a.name))
            .collect();

        // Spawn accumulators with health and DAL wiring
        let mut accumulator_handles: Vec<(String, JoinHandle<()>)> = Vec::new();
        let mut acc_health_rxs: Vec<(
            String,
            watch::Receiver<super::accumulator::AccumulatorHealth>,
        )> = Vec::new();
        for acc_decl in &accumulators {
            let (health_tx, health_rx) = health_channel();
            acc_health_rxs.push((acc_decl.name.clone(), health_rx.clone()));

            let freshness = super::accumulator::FreshnessHandle::new();
            let spawn_config = AccumulatorSpawnConfig {
                dal: self.dal.clone(),
                health_tx: Some(health_tx),
                graph_name: reactor_name.clone(),
                freshness: freshness.clone(),
            };

            let (socket_tx, handle) = acc_decl.factory.spawn(
                acc_decl.name.clone(),
                boundary_tx.clone(),
                shutdown_rx.clone(),
                spawn_config,
            );

            // CLOACI-T-0921: a name already claimed by a DIFFERENT owner in
            // this tenant is a load-time rejection. Tear down what we already
            // spawned so a rejected load leaves nothing half-wired behind.
            if let Err(e) = self
                .registry
                .register_accumulator(&owner, acc_decl.name.clone(), socket_tx)
                .await
            {
                let _ = shutdown_tx.send(true);
                for (spawned, _) in &accumulator_handles {
                    self.registry.deregister_accumulator(&owner, spawned).await;
                }
                return Err(format!(
                    "reactor '{}' cannot be loaded: {}",
                    reactor_name, e
                ));
            }
            self.registry
                .register_accumulator_health(&owner, acc_decl.name.clone(), health_rx)
                .await;
            self.registry
                .register_accumulator_freshness(&owner, acc_decl.name.clone(), freshness)
                .await;
            // CLOACI-I-0128 follow-up: self-register discoverability metadata
            // (the reactor this accumulator feeds + owning tenant) so the
            // discovery API can surface the relationship, not just the name.
            self.registry
                .register_accumulator_meta(
                    &owner,
                    acc_decl.name.clone(),
                    super::registry::AccumulatorDescriptor {
                        reactor: reactor_name.clone(),
                        tenant_id: tenant_id.clone(),
                    },
                )
                .await;

            accumulator_handles.push((acc_decl.name.clone(), handle));
        }

        // Manual command channel + reactor health channel
        let (manual_tx, manual_rx) = mpsc::channel(64);
        let (reactor_health_tx, reactor_health_rx) = reactor_health_channel();

        // Empty subscribers map; subscribers bind via `bind_graph_to_reactor`
        // after load_reactor returns. The dispatcher walks the (currently
        // empty) map and returns Completed — the reactor still fires-and-
        // counts even with zero subscribers.
        let subscribers: ReactorSubscribers = Arc::new(RwLock::new(HashMap::new()));
        let dispatcher = make_subscriber_dispatcher(reactor_name.clone(), subscribers.clone());

        let mut reactor = Reactor::new(
            dispatcher,
            criteria.clone(),
            strategy.clone(),
            boundary_rx,
            manual_rx,
            shutdown_rx,
        )
        .with_graph_name(reactor_name.clone())
        .with_health(reactor_health_tx)
        .with_expected_sources(expected_sources)
        .with_accumulator_health(acc_health_rxs)
        .with_tenant_id(tenant_id.clone())
        .with_graph_executor(self.graph_executor.read().await.clone());

        // CLOACI-T-0830: a resolved reactor-constructor decider replaces the
        // built-in WhenAny/WhenAll criteria — the WASM guest's `evaluate` decides
        // firing. The native path leaves `evaluator` as `None` (unchanged).
        if let Some(ref ev) = evaluator {
            reactor = reactor.with_evaluator(ev.clone());
        }

        if let Some(ref dal) = self.dal {
            reactor = reactor.with_dal(dal.clone());
        }

        let reactor_shared = reactor.handle();

        // Register reactor under its name + any aliases. Both keys point at
        // the same manual channel + handle.
        let mut endpoint_registry_keys = vec![reactor_name.clone()];
        let mut registration_failure: Option<String> = None;
        if let Err(e) = self
            .registry
            .register_reactor(
                &owner,
                reactor_name.clone(),
                manual_tx.clone(),
                reactor_shared.clone(),
            )
            .await
        {
            registration_failure = Some(e.to_string());
        }
        if registration_failure.is_none() {
            for alias in &register_aliases {
                if alias != &reactor_name {
                    if let Err(e) = self
                        .registry
                        .register_reactor(
                            &owner,
                            alias.clone(),
                            manual_tx.clone(),
                            reactor_shared.clone(),
                        )
                        .await
                    {
                        registration_failure = Some(e.to_string());
                        break;
                    }
                    endpoint_registry_keys.push(alias.clone());
                }
            }
        }
        // CLOACI-T-0921: unwind the whole load if any endpoint name was
        // already claimed by another owner in this tenant.
        if let Some(e) = registration_failure {
            let _ = shutdown_tx.send(true);
            for key in &endpoint_registry_keys {
                self.registry.deregister_reactor(&owner, key).await;
            }
            for (spawned, _) in &accumulator_handles {
                self.registry.deregister_accumulator(&owner, spawned).await;
            }
            return Err(format!(
                "reactor '{}' cannot be loaded: {}",
                reactor_name, e
            ));
        }

        // Set auth policies based on package tenant ownership.
        let acc_policy = match &tenant_id {
            Some(tid) => AccumulatorAuthPolicy::for_tenant(tid),
            None => AccumulatorAuthPolicy::allow_all(),
        };
        let reactor_policy = match &tenant_id {
            Some(tid) => ReactorAuthPolicy::for_tenant(tid),
            None => ReactorAuthPolicy::allow_all(),
        };
        for acc_decl in &accumulators {
            self.registry
                .set_accumulator_policy(&owner, acc_decl.name.clone(), acc_policy.clone())
                .await;
        }
        for key in &endpoint_registry_keys {
            self.registry
                .set_reactor_policy(&owner, key.clone(), reactor_policy.clone())
                .await;
        }

        let reactor_handle = tokio::spawn(reactor.run());

        info!(reactor = %reactor_name, "reactor loaded and running");

        // Synthetic anchoring declaration. Contract fields (accumulators,
        // criteria, strategy, tenant_id) are read on the idempotent path and
        // by the supervisor's restart logic. `name` carries the reactor's
        // name for logging/restart purposes.
        let anchor = ComputationGraphDeclaration {
            name: reactor_name.clone(),
            accumulators,
            reactor: ReactorDeclaration {
                criteria,
                strategy,
                graph_fn: dummy_graph_fn(),
                // Preserve the constructor ref on the anchor for fidelity; the
                // restart path reuses the already-resolved `evaluator` rather
                // than re-resolving from this, but keeping it keeps the anchor an
                // honest record of how the reactor was declared (CLOACI-T-0830).
                constructor,
            },
            tenant_id,
            reactor_name: Some(reactor_name.clone()),
            topology: None,
        };

        let running = RunningGraph {
            shutdown_tx,
            shutdown_rx: stored_shutdown_rx,
            boundary_tx: stored_boundary_tx,
            accumulator_handles,
            reactor_handle,
            reactor_shared,
            reactor_health_rx: Some(reactor_health_rx),
            declaration: anchor,
            owner,
            subscribers,
            endpoint_registry_keys,
            failure_counts: HashMap::new(),
            last_success: HashMap::new(),
            evaluator,
        };

        self.reactors.write().await.insert(reactor_key, running);
        Ok(())
    }

    /// Bind a graph as an additional subscriber on an already-loaded reactor.
    ///
    /// The reactor must have been loaded first (via [`load_reactor`] or
    /// transitively via [`load_graph`]); this entry point doesn't spawn
    /// reactors. Returns an error if the reactor isn't loaded or if a graph
    /// with the same name is already bound somewhere.
    ///
    /// CLOACI-T-0924: `scope` is the *binding tenant*. The graph is claimed
    /// under `scope`'s own key, while the upstream reactor is **resolved**
    /// within `scope` — own tenant first, then the untenanted (embedded /
    /// pre-multi-tenancy) reactor. A tenant can therefore subscribe to an
    /// untenanted upstream, but never to another tenant's.
    pub async fn bind_graph_to_reactor(
        &self,
        graph_name: String,
        reactor_name: String,
        scope: TenantScope<'_>,
        graph_fn: CompiledGraphFn,
    ) -> Result<(), String> {
        let graph_key = scope.own_key(&graph_name);
        {
            let g2r = self.graph_to_reactor.read().await;
            if g2r.contains_key(&graph_key) {
                return Err(format!("graph '{}' already loaded", graph_name));
            }
        }

        let reactor_key = {
            let reactors = self.reactors.read().await;
            let reactor_key = resolve_tenant_key(&*reactors, scope, &reactor_name)
                .map_err(|_| format!("reactor '{}' is not loaded", reactor_name))?;
            let existing = reactors
                .get(&reactor_key)
                .ok_or_else(|| format!("reactor '{}' is not loaded", reactor_name))?;
            let mut subs = existing.subscribers.write().await;
            // The per-reactor subscriber map is keyed by bare graph name (the
            // dispatcher labels results with it). The `graph_to_reactor`
            // pre-check above already rejects a same-tenant duplicate, so a
            // name that is still present here means a DIFFERENT tenant bound a
            // same-named graph to this same (necessarily untenanted) reactor.
            // Refuse loudly rather than silently replacing their graph_fn.
            if subs.contains_key(&graph_name) {
                return Err(format!(
                    "graph '{}' is already bound to reactor '{}' by another tenant; \
                     rename the graph or load a tenant-scoped reactor",
                    graph_name, reactor_name
                ));
            }
            subs.insert(graph_name.clone(), graph_fn);
            drop(subs);
            reactor_key
        };
        self.graph_to_reactor
            .write()
            .await
            .insert(graph_key, reactor_key);

        info!(
            graph = %graph_name,
            reactor = %reactor_name,
            tenant = %scope.tenant_id.unwrap_or("<untenanted>"),
            "graph bound to reactor"
        );
        Ok(())
    }

    /// Load and start a computation graph.
    ///
    /// After T-0545 M1 this is a thin wrapper over [`load_reactor`] +
    /// [`bind_graph_to_reactor`]. It exists so today's bundled-form callers
    /// (every existing test, every package built before reactor-only
    /// packages) keep their contract: one call resolves both the reactor's
    /// lifecycle and the graph's subscription. Independent-reactor consumers
    /// (the reconciler post-T-0545) call the explicit pair directly.
    pub async fn load_graph(&self, decl: ComputationGraphDeclaration) -> Result<(), String> {
        self.load_graph_in(decl, None).await
    }

    /// [`load_graph`](Self::load_graph) resolving the declaration's reactor
    /// constructor against an EXPLICIT provider tree (CLOACI-T-0925) — the
    /// directory the reconciler staged for the package that declared the graph.
    pub async fn load_graph_in(
        &self,
        decl: ComputationGraphDeclaration,
        provider_root: Option<&std::path::Path>,
    ) -> Result<(), String> {
        let name = decl.name.clone();
        // Resolve the reactor identity. `Some(...)` from a split-form caller
        // (T-0544 M2: cross-package fan-out) lets multiple graphs share a
        // reactor by name. `None` (today's bundled-form path) synthesizes a
        // per-graph reactor name preserving the 1:1 reactor-per-graph
        // behavior.
        let reactor_name = decl
            .reactor_name
            .clone()
            .unwrap_or_else(|| format!("__Reactor_{}", name));
        // CLOACI-T-0924: the declaration's tenant is the scope for everything
        // this load claims. `None` (embedded / bundled-form tests) keeps the
        // untenanted keys the pre-T-0924 code used.
        let scope = TenantScope::of(decl.tenant_id.as_deref());
        let graph_key = scope.own_key(&name);

        // Pre-check: reject re-loading the same graph regardless of which
        // reactor it was bound to. (load_reactor + bind_graph_to_reactor
        // would catch this too, but doing it here keeps the error message
        // precise.) Scoped to this tenant — another tenant's same-named graph
        // is a different entry entirely.
        {
            let g2r = self.graph_to_reactor.read().await;
            if g2r.contains_key(&graph_key) {
                return Err(format!("graph '{}' already loaded", name));
            }
        }

        // Capture this graph's node/edge topology so the health API can render
        // its DAG. Keyed by graph name; cleaned up in `unload_graph`. Safe to
        // record here — `list_graphs`/`get_graph` only read it for graphs that
        // are also in `graph_to_reactor`, so a failed load below never leaks a
        // visible entry. (CLOACI-T-0673)
        if let Some(topology) = decl.topology.clone() {
            self.graph_topologies
                .write()
                .await
                .insert(graph_key.clone(), topology);
        }

        // Cross-package subscriber path: when the named reactor is
        // already loaded by an earlier package and this declaration's
        // accumulators is empty, the package is binding to an upstream
        // reactor it does not own. Skip `load_reactor` (its idempotent
        // contract check would reject the empty-vs-populated mismatch)
        // and bind directly. The publisher's accumulator factories
        // remain authoritative; the subscriber just adds itself to the
        // subscribers map.
        if decl.reactor_name.is_some() && decl.accumulators.is_empty() {
            let already_loaded = {
                let reactors = self.reactors.read().await;
                resolve_tenant_key(&*reactors, scope, &reactor_name).is_ok()
            };
            if already_loaded {
                return self
                    .bind_graph_to_reactor(name, reactor_name, scope, decl.reactor.graph_fn)
                    .await;
            }
        }

        // Load (or join) the reactor. We register the graph's name as an
        // alias so `cloacinactl reactor force-fire <graph>` keeps working
        // for bundled-form callers and for the first graph that names a
        // shared reactor (T-0544 M2 surface promise).
        self.load_reactor_in(
            reactor_name.clone(),
            decl.accumulators.clone(),
            decl.reactor.criteria.clone(),
            decl.reactor.strategy.clone(),
            decl.tenant_id.clone(),
            vec![name.clone()],
            decl.reactor.constructor.clone(),
            provider_root,
        )
        .await?;

        self.bind_graph_to_reactor(name, reactor_name, scope, decl.reactor.graph_fn)
            .await
    }

    /// Load a computation graph that references a reactor declaration by
    /// value (split form, from `#[computation_graph(trigger = reactor(T))]`).
    ///
    /// This spawns a fresh reactor instance tied to this graph, using the
    /// criteria + accumulator list carried by `reactor`, and binds `graph_fn`
    /// as the firing callback.
    ///
    /// **Test-only convenience API.** Production reconciler code does NOT
    /// call this — the `RegistryReconciler` calls `load_reactor` followed
    /// by `bind_graph_to_reactor` directly so the reactor identity is
    /// explicit at every step. This helper exists for integration tests in
    /// `crates/cloacina/tests/integration/computation_graph.rs` that exercise
    /// the split-form lifecycle. (T-0556 audit confirmed zero non-test
    /// callers.)
    ///
    /// `input_strategy` defaults to [`InputStrategy::Latest`].
    pub async fn load_graph_split(
        &self,
        graph_name: String,
        graph_fn: CompiledGraphFn,
        reactor: &cloacina_computation_graph::ReactorRegistration,
        accumulators: Vec<AccumulatorDeclaration>,
        tenant_id: Option<String>,
    ) -> Result<(), String> {
        // Validate: every accumulator named in the reactor declaration must
        // have an `AccumulatorDeclaration` supplied.
        let supplied: std::collections::HashSet<&str> =
            accumulators.iter().map(|a| a.name.as_str()).collect();
        for name in &reactor.accumulator_names {
            if !supplied.contains(name.as_str()) {
                return Err(format!(
                    "reactor '{}' declares accumulator '{}' but no AccumulatorDeclaration was \
                     supplied for it",
                    reactor.name, name
                ));
            }
        }

        let decl = ComputationGraphDeclaration {
            name: graph_name,
            accumulators,
            reactor: ReactorDeclaration {
                criteria: reactor.reaction_mode.into(),
                strategy: InputStrategy::Latest,
                graph_fn,
                // Split-form (`#[computation_graph(trigger = reactor(T))]`) does
                // not author WASM reactor constructors — native firing only.
                constructor: None,
            },
            tenant_id,
            // Split-form callers carry an explicit reactor identity. Multiple
            // graphs naming the same reactor here share one reactor instance
            // (T-0544 fan-out).
            reactor_name: Some(reactor.name.clone()),
            topology: None,
        };

        self.load_graph(decl).await
    }

    /// Unbind a graph from its reactor without affecting the reactor itself.
    ///
    /// The graph stops being a subscriber but the reactor (and its
    /// accumulators) keeps running, ready for new subscribers. This is the
    /// honest lifecycle primitive — reactors are independent units; binding
    /// and unbinding subscribers is decoupled from reactor teardown.
    ///
    /// CLOACI-T-0924: `name` is resolved within `scope` (own tenant, then the
    /// untenanted entry), so a caller can only unbind a graph it can see.
    /// Returns the full [`TenantKey`] of the reactor the graph was bound to.
    pub async fn unbind_graph_from_reactor(
        &self,
        name: &str,
        scope: TenantScope<'_>,
    ) -> Result<TenantKey, String> {
        let reactor_key = {
            let mut g2r = self.graph_to_reactor.write().await;
            let graph_key = resolve_tenant_key(&*g2r, scope, name)
                .map_err(|_| format!("graph '{}' not loaded", name))?;
            // Drop the cached topology for this graph. (CLOACI-T-0673)
            self.graph_topologies.write().await.remove(&graph_key);
            g2r.remove(&graph_key)
                .ok_or_else(|| format!("graph '{}' not loaded", name))?
        };

        let remaining = {
            let reactors = self.reactors.read().await;
            if let Some(running) = reactors.get(&reactor_key) {
                let mut subs = running.subscribers.write().await;
                subs.remove(name);
                subs.len()
            } else {
                // graph_to_reactor pointed at a missing reactor — surface as
                // an error rather than silently no-oping.
                return Err(format!(
                    "graph '{}' was bound to reactor '{}' but the reactor is not loaded",
                    name, reactor_key.name
                ));
            }
        };

        info!(
            graph = %name,
            reactor = %reactor_key,
            remaining_subscribers = remaining,
            "graph unbound from reactor"
        );
        Ok(reactor_key)
    }

    /// Tear down a reactor and its accumulators. Rejects if the reactor has
    /// any bound subscribers — operators must unbind subscribers first. This
    /// is the lifecycle guard that makes "reactors as independent units"
    /// safe: a reactor never disappears out from under a graph that's still
    /// declaring it as an upstream.
    ///
    /// CLOACI-T-0924: `reactor_name` is resolved within `scope`. A tenant can
    /// only tear down its own reactor (or an untenanted one it can already
    /// address); another tenant's same-named reactor is simply "not loaded".
    pub async fn unload_reactor(
        &self,
        reactor_name: &str,
        scope: TenantScope<'_>,
    ) -> Result<(), String> {
        // Snapshot subscribers under read lock so we can build a precise
        // error message if any remain.
        let reactor_key = {
            let reactors = self.reactors.read().await;
            resolve_tenant_key(&*reactors, scope, reactor_name)
                .map_err(|_| format!("reactor '{}' not loaded", reactor_name))?
        };
        let subscriber_names: Vec<String> = {
            let reactors = self.reactors.read().await;
            match reactors.get(&reactor_key) {
                Some(running) => running.subscribers.read().await.keys().cloned().collect(),
                None => return Err(format!("reactor '{}' not loaded", reactor_name)),
            }
        };
        if !subscriber_names.is_empty() {
            return Err(format!(
                "reactor '{}' has {} bound subscriber(s): {:?}; unbind them first",
                reactor_name,
                subscriber_names.len(),
                subscriber_names
            ));
        }

        let running = {
            let mut reactors = self.reactors.write().await;
            reactors
                .remove(&reactor_key)
                .ok_or_else(|| format!("reactor '{}' not loaded", reactor_name))?
        };

        self.teardown_running(running, reactor_name).await;
        Ok(())
    }

    /// Stop a reactor that has ALREADY been removed from the `reactors` map,
    /// releasing everything it owned.
    ///
    /// Extracted so ownership-loss halts (CLOACI-T-0851) and ordinary unloads
    /// share ONE teardown. A second copy would be free to drift — and the way
    /// it would drift is by forgetting a deregistration, leaving a stopped
    /// reactor still advertised in the endpoint registry.
    ///
    /// The caller owns the policy decision (may this reactor be torn down?);
    /// this performs it unconditionally.
    async fn teardown_running(&self, running: RunningGraph, reactor_name: &str) {
        // Capture graph names for health-metric "stopped" emission. Use the
        // endpoint-registry keys (which include the reactor's own name and
        // back-compat graph aliases) so every graph the reactor served sees
        // a stop signal in the gauge.
        let graph_labels: Vec<String> = running.endpoint_registry_keys.clone();

        let _ = running.shutdown_tx.send(true);
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), running.reactor_handle).await;
        for label in &graph_labels {
            emit_component_health(label, "reactor", "stopped");
        }

        for (acc_name, handle) in running.accumulator_handles {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
            for label in &graph_labels {
                emit_component_health(label, &acc_name, "stopped");
            }
            self.registry
                .deregister_accumulator(&running.owner, &acc_name)
                .await;
        }

        // Deregister every endpoint-registry key the reactor was registered
        // under (its own name + any back-compat aliases for bundled-form
        // callers). CLOACI-T-0921: owner-scoped, so unloading this package
        // never tears down another tenant's/package's same-named endpoint.
        for key in &running.endpoint_registry_keys {
            self.registry.deregister_reactor(&running.owner, key).await;
        }

        info!(reactor = %reactor_name, "reactor unloaded");
    }

    /// Stop reactors this replica has lost ownership of (CLOACI-T-0851 /
    /// [`ADR CLOACI-A-0012`] Amendment 1).
    ///
    /// Returns the reactors actually stopped — a subset, because a reactor may
    /// already have been unloaded between the liveness check and this call.
    ///
    /// **Deliberately bypasses `unload_reactor`'s subscriber guard.** That guard
    /// exists so a reactor never disappears under a graph still declaring it
    /// upstream, which is right for an operator-initiated unload. It is wrong
    /// here: having lost the lock, another replica may already be running this
    /// reactor, and refusing to stop because a subscriber remains would leave
    /// two live copies double-processing the same stream. A stopped reactor is
    /// recoverable by re-claiming; silent double-processing is not.
    pub async fn halt_unowned_reactors(
        &self,
        lost: &[super::reactor_ownership::ReactorId],
    ) -> Vec<super::reactor_ownership::ReactorId> {
        let mut stopped = Vec::new();
        for id in lost {
            let key: crate::TenantKey = id.into();
            let running = {
                let mut reactors = self.reactors.write().await;
                reactors.remove(&key)
            };
            match running {
                Some(running) => {
                    tracing::warn!(
                        reactor = %key,
                        "ownership lost — halting reactor locally; another replica may own it"
                    );
                    self.teardown_running(running, &key.name).await;
                    stopped.push(id.clone());
                }
                None => {
                    // Not an error: it may have been unloaded normally between
                    // the check and here. Logged so a puzzling "lost ownership
                    // but nothing stopped" is explicable rather than silent.
                    tracing::debug!(
                        reactor = %key,
                        "ownership lost for a reactor that is no longer loaded here"
                    );
                }
            }
        }
        stopped
    }

    /// Backward-compat convenience: unbind the graph from its reactor and,
    /// if it was the last subscriber, also tear down the reactor. This
    /// preserves today's 1:1 reactor-per-graph callers (a single
    /// `unload_graph(name)` removes everything the matching `load_graph`
    /// brought in). For independent reactor lifecycles, prefer
    /// [`unbind_graph_from_reactor`] + explicit [`unload_reactor`].
    pub async fn unload_graph(&self, name: &str, scope: TenantScope<'_>) -> Result<(), String> {
        let reactor_key = self.unbind_graph_from_reactor(name, scope).await?;

        // If subscribers are now empty, tear down the reactor for back-compat
        // with bundled-form callers.
        let now_empty = {
            let reactors = self.reactors.read().await;
            match reactors.get(&reactor_key) {
                Some(running) => running.subscribers.read().await.is_empty(),
                None => false,
            }
        };
        if now_empty {
            // Address the reactor in ITS own scope — a tenant graph bound to
            // an untenanted upstream must still resolve back to that exact
            // entry, not to a same-named one in the caller's tenant.
            self.unload_reactor(
                &reactor_key.name,
                TenantScope::of(reactor_key.tenant_id.as_deref()),
            )
            .await?;
        }
        info!(graph = %name, reactor = %reactor_key, "computation graph unloaded");
        Ok(())
    }

    /// Snapshot the accumulator names of a loaded reactor, in declaration
    /// order. Returns `None` if the reactor isn't loaded. Used by the
    /// reconciler to pre-validate cross-package subscriber bindings against
    /// the upstream reactor's contract before calling [`load_graph`].
    pub async fn reactor_accumulator_names(
        &self,
        reactor_name: &str,
        scope: TenantScope<'_>,
    ) -> Option<Vec<String>> {
        let reactors = self.reactors.read().await;
        let key = resolve_tenant_key(&*reactors, scope, reactor_name).ok()?;
        reactors.get(&key).map(|running| {
            running
                .accumulator_handles
                .iter()
                .map(|(n, _)| n.clone())
                .collect()
        })
    }

    /// List all loaded computation graphs with status. Emits one entry per
    /// graph; multiple graphs sharing a reactor each get a status reflecting
    /// the same reactor's running state.
    pub async fn list_graphs(&self) -> Vec<GraphStatus> {
        let g2r = self.graph_to_reactor.read().await;
        let reactors = self.reactors.read().await;
        let topologies = self.graph_topologies.read().await;
        g2r.iter()
            .filter_map(|(graph_key, reactor_key)| {
                reactors.get(reactor_key).map(|running| GraphStatus {
                    name: graph_key.name.clone(),
                    accumulators: running
                        .accumulator_handles
                        .iter()
                        .map(|(n, _)| n.clone())
                        .collect(),
                    paused: running.reactor_shared.is_paused(),
                    running: !running.reactor_handle.is_finished(),
                    health: running
                        .reactor_health_rx
                        .as_ref()
                        .map(|rx| rx.borrow().clone()),
                    // CLOACI-T-0924: the key is now the authority on tenancy —
                    // it is what isolation is enforced on. (It matches the
                    // declaration's `tenant_id`, which set it at load.)
                    tenant_id: graph_key.tenant_id.clone(),
                    topology: topologies.get(graph_key).cloned(),
                    reactor: running.declaration.reactor_name.clone(),
                    reaction_mode: match running.declaration.reactor.criteria {
                        ReactionCriteria::WhenAny => "when_any".to_string(),
                        ReactionCriteria::WhenAll => "when_all".to_string(),
                    },
                    input_strategy: match running.declaration.reactor.strategy {
                        InputStrategy::Latest => "latest".to_string(),
                        InputStrategy::Sequential => "sequential".to_string(),
                    },
                    fires: running.reactor_shared.stats().0,
                    last_fire_unix_ms: running.reactor_shared.stats().1,
                })
            })
            .collect()
    }

    /// List all loaded reactors with status (CLOACI-T-0742). Reactor-first: one
    /// entry per reactor in the `reactors` map, **including reactors with no
    /// graph bound** (which `list_graphs` omits, since it iterates
    /// `graph_to_reactor`). `bound_graphs` is the reverse lookup over that map.
    pub async fn list_reactors(&self) -> Vec<ReactorStatus> {
        let reactors = self.reactors.read().await;
        let g2r = self.graph_to_reactor.read().await;
        reactors
            .iter()
            .map(|(reactor_key, running)| {
                let bound_graphs: Vec<String> = g2r
                    .iter()
                    .filter(|(_, r)| *r == reactor_key)
                    .map(|(g, _)| g.name.clone())
                    .collect();
                let (fires, last_fire_unix_ms) = running.reactor_shared.stats();
                ReactorStatus {
                    name: reactor_key.name.clone(),
                    accumulators: running
                        .accumulator_handles
                        .iter()
                        .map(|(n, _)| n.clone())
                        .collect(),
                    reaction_mode: match running.declaration.reactor.criteria {
                        ReactionCriteria::WhenAny => "when_any".to_string(),
                        ReactionCriteria::WhenAll => "when_all".to_string(),
                    },
                    input_strategy: match running.declaration.reactor.strategy {
                        InputStrategy::Latest => "latest".to_string(),
                        InputStrategy::Sequential => "sequential".to_string(),
                    },
                    bound_graphs,
                    paused: running.reactor_shared.is_paused(),
                    running: !running.reactor_handle.is_finished(),
                    health: running
                        .reactor_health_rx
                        .as_ref()
                        .map(|rx| rx.borrow().clone()),
                    // CLOACI-T-0924: tenancy comes from the key.
                    tenant_id: reactor_key.tenant_id.clone(),
                    fires,
                    last_fire_unix_ms,
                }
            })
            .collect()
    }

    /// Check all graphs for crashed tasks and restart them.
    ///
    /// Individual accumulators are restarted in-place without tearing down the
    /// reactor. Reactor crashes trigger a full-graph restart. Failure counting
    /// with exponential backoff prevents infinite restart loops.
    ///
    /// Runs in three phases (CLOACI-T-0915): crash detection and
    /// failure-count bookkeeping happen under the reactors write lock; the
    /// recovery-event write and the exponential-backoff sleep happen with
    /// the lock RELEASED so list/health readers (`list_graphs`,
    /// `/v1/health/graphs`, package loads) stay responsive during a restart
    /// storm; each restart then re-acquires the lock and re-validates that
    /// the component is still down before acting.
    pub async fn check_and_restart_failed(&self) -> usize {
        let mut restarted = 0;
        let now = std::time::Instant::now();

        // Phase 1 — under the write lock: reset success bookkeeping, detect
        // crashed components, take their dead handles, and collect a restart
        // plan. NO sleeps and NO DB writes happen while the lock is held.
        let plans: Vec<PlannedRestart> = {
            let mut graphs = self.reactors.write().await;
            let mut plans = Vec::new();

            for (reactor_key, running) in graphs.iter_mut() {
                // Metric/log labels stay bare names (CLOACI-T-0924 keeps the
                // `cloacina_component_health` label vocabulary unchanged); the
                // restart plan carries the full key so phase 3 re-finds the
                // exact entry.
                let graph_name = reactor_key.name.as_str();
                // Reset failure counts for components that have been running successfully
                let success_threshold = std::time::Duration::from_secs(SUCCESS_RESET_SECS);
                let names_to_reset: Vec<String> = running
                    .last_success
                    .iter()
                    .filter(|(_, ts)| now.duration_since(**ts) >= success_threshold)
                    .map(|(name, _)| name.clone())
                    .collect();
                for name in names_to_reset {
                    running.failure_counts.remove(&name);
                    running.last_success.remove(&name);
                }

                // Check reactor
                if running.reactor_handle.is_finished() {
                    let component_key = format!("{}::reactor", graph_name);
                    let failures = running
                        .failure_counts
                        .entry(component_key.clone())
                        .or_insert(0);
                    *failures += 1;

                    // Take ownership of the finished handle so phase 2 can
                    // inspect the JoinError (panic vs ordinary exit) without
                    // blocking. The dummy stays in place until the phase-3
                    // restart swaps in the re-spawned reactor.
                    let dead =
                        std::mem::replace(&mut running.reactor_handle, tokio::spawn(async {}));

                    if *failures > MAX_RECOVERY_ATTEMPTS {
                        error!(
                            graph = %graph_name,
                            failures = *failures,
                            "reactor permanently failed — circuit breaker open"
                        );
                        emit_component_health(graph_name, "reactor", "crashed");
                        drop(dead);
                        continue;
                    }

                    let backoff_secs =
                        (BACKOFF_BASE_SECS * 2u64.pow(*failures - 1)).min(BACKOFF_MAX_SECS);
                    warn!(
                        graph = %graph_name,
                        attempt = *failures,
                        backoff_secs = backoff_secs,
                        "reactor crashed, restarting (full graph restart)"
                    );

                    plans.push(PlannedRestart::Reactor {
                        reactor_key: reactor_key.clone(),
                        component_key,
                        attempt: *failures,
                        backoff_secs,
                        dead,
                    });
                } else {
                    // Check individual accumulators — plan in-place restarts.
                    // Circuit-broken accumulators are dropped (abandoned)
                    // right here; crashed-but-recoverable ones get a dummy
                    // swapped into their slot and a plan entry.
                    let mut idx = 0;
                    while idx < running.accumulator_handles.len() {
                        if !running.accumulator_handles[idx].1.is_finished() {
                            idx += 1;
                            continue;
                        }
                        let acc_name = running.accumulator_handles[idx].0.clone();
                        let acc_key = format!("{}::{}", graph_name, acc_name);
                        let failures = running.failure_counts.entry(acc_key.clone()).or_insert(0);
                        *failures += 1;

                        if *failures > MAX_RECOVERY_ATTEMPTS {
                            error!(
                                graph = %graph_name,
                                accumulator = %acc_name,
                                failures = *failures,
                                "accumulator permanently failed — circuit breaker open"
                            );
                            emit_component_health(graph_name, &acc_name, "crashed");
                            // Remove the slot — accumulator is abandoned.
                            running.accumulator_handles.remove(idx);
                            continue;
                        }

                        let backoff_secs =
                            (BACKOFF_BASE_SECS * 2u64.pow(*failures - 1)).min(BACKOFF_MAX_SECS);
                        warn!(
                            graph = %graph_name,
                            accumulator = %acc_name,
                            attempt = *failures,
                            backoff_secs = backoff_secs,
                            "accumulator crashed, restarting individually"
                        );

                        // Take the finished handle; the dummy stays in the
                        // slot until the phase-3 respawn replaces it.
                        let dead = std::mem::replace(
                            &mut running.accumulator_handles[idx].1,
                            tokio::spawn(async {}),
                        );
                        plans.push(PlannedRestart::Accumulator {
                            reactor_key: reactor_key.clone(),
                            acc_name,
                            component_key: acc_key,
                            attempt: *failures,
                            backoff_secs,
                            dead,
                        });
                        idx += 1;
                    }
                }
            }

            plans
        };

        // Phases 2 and 3 — the reactors lock is NOT held here. Record the
        // recovery event and sleep out the backoff unlocked (serial, matching
        // the pre-T-0915 pacing), then re-acquire the lock per restart and
        // re-validate before acting.
        for plan in plans {
            match plan {
                PlannedRestart::Reactor {
                    reactor_key,
                    component_key,
                    attempt,
                    backoff_secs,
                    dead,
                } => {
                    // Non-blocking: the handle already finished in phase 1.
                    let reason = classify_join_result(dead.await);
                    self.record_recovery_event(&component_key, attempt, backoff_secs)
                        .await;
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                    if self
                        .restart_reactor_after_backoff(&reactor_key, reason, now)
                        .await
                    {
                        restarted += 1;
                    }
                }
                PlannedRestart::Accumulator {
                    reactor_key,
                    acc_name,
                    component_key,
                    attempt,
                    backoff_secs,
                    dead,
                } => {
                    let reason = classify_join_result(dead.await);
                    self.record_recovery_event(&component_key, attempt, backoff_secs)
                        .await;
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                    if self
                        .restart_accumulator_after_backoff(&reactor_key, &acc_name, reason, now)
                        .await
                    {
                        restarted += 1;
                    }
                }
            }
        }

        restarted
    }

    /// Phase 3 of [`check_and_restart_failed`]: after the unlocked backoff,
    /// re-acquire the write lock and perform the full-graph restart.
    ///
    /// Re-validates before acting — while the lock was released the reactor
    /// may have been unloaded, or replaced with a live task by another path
    /// (e.g. unload + reload). Returns whether a restart actually happened.
    async fn restart_reactor_after_backoff(
        &self,
        reactor_key: &TenantKey,
        reason: &'static str,
        now: std::time::Instant,
    ) -> bool {
        // Labels/log fields stay bare names; the lookup uses the full key.
        let reactor_name = reactor_key.name.as_str();
        let mut graphs = self.reactors.write().await;
        let Some(running) = graphs.get_mut(reactor_key) else {
            info!(
                graph = %reactor_name,
                "reactor unloaded during restart backoff — skipping restart"
            );
            return false;
        };
        // Phase 1 swapped a finished dummy into `reactor_handle`. A live
        // (unfinished) handle means another path already brought a reactor
        // up under this name — don't stomp it.
        if !running.reactor_handle.is_finished() {
            info!(
                graph = %reactor_name,
                "reactor already replaced during restart backoff — skipping restart"
            );
            return false;
        }

        // Full graph restart: new channels, re-spawn everything
        let (shutdown_tx, shutdown_rx) = shutdown_signal();
        let stored_shutdown_rx = shutdown_rx.clone();
        let (boundary_tx, boundary_rx) = mpsc::channel(256);
        let stored_boundary_tx = boundary_tx.clone();

        let expected_sources: Vec<SourceName> = running
            .declaration
            .accumulators
            .iter()
            .map(|a| SourceName::new(&a.name))
            .collect();

        // CLOACI-T-0921: re-register under the identity claimed at load.
        let owner = running.owner.clone();

        let mut new_acc_handles = Vec::new();
        let mut restart_acc_health_rxs: Vec<(
            String,
            watch::Receiver<super::accumulator::AccumulatorHealth>,
        )> = Vec::new();
        for acc_decl in &running.declaration.accumulators {
            let (health_tx, health_rx) = health_channel();
            restart_acc_health_rxs.push((acc_decl.name.clone(), health_rx.clone()));
            let freshness = super::accumulator::FreshnessHandle::new();
            let spawn_config = AccumulatorSpawnConfig {
                dal: self.dal.clone(),
                health_tx: Some(health_tx),
                graph_name: reactor_name.to_string(),
                freshness: freshness.clone(),
            };
            let (socket_tx, handle) = acc_decl.factory.spawn(
                acc_decl.name.clone(),
                boundary_tx.clone(),
                shutdown_rx.clone(),
                spawn_config,
            );
            // CLOACI-T-0921: the restart re-registers under the SAME owner it
            // claimed at load, so this always matches and never conflicts.
            if let Err(e) = self
                .registry
                .register_accumulator(&owner, acc_decl.name.clone(), socket_tx)
                .await
            {
                warn!(
                    graph = %reactor_name,
                    accumulator = %acc_decl.name,
                    error = %e,
                    "accumulator re-registration rejected on restart"
                );
            }
            self.registry
                .register_accumulator_health(&owner, acc_decl.name.clone(), health_rx)
                .await;
            self.registry
                .register_accumulator_freshness(&owner, acc_decl.name.clone(), freshness)
                .await;
            // CLOACI-I-0128 follow-up: re-register discoverability meta
            // on the restart path too (graph + tenant).
            self.registry
                .register_accumulator_meta(
                    &owner,
                    acc_decl.name.clone(),
                    super::registry::AccumulatorDescriptor {
                        reactor: reactor_name.to_string(),
                        tenant_id: running.declaration.tenant_id.clone(),
                    },
                )
                .await;
            new_acc_handles.push((acc_decl.name.clone(), handle));
        }

        let (manual_tx, manual_rx) = mpsc::channel(64);
        let (reactor_health_tx, reactor_health_rx) = reactor_health_channel();
        // Reuse the same subscriber map across restart so subscribers
        // bound mid-life don't get dropped when the reactor restarts.
        let restart_dispatcher =
            make_subscriber_dispatcher(reactor_name.to_string(), running.subscribers.clone());
        let mut reactor = Reactor::new(
            restart_dispatcher,
            running.declaration.reactor.criteria.clone(),
            running.declaration.reactor.strategy.clone(),
            boundary_rx,
            manual_rx,
            shutdown_rx,
        )
        .with_graph_name(reactor_name.to_string())
        .with_health(reactor_health_tx)
        .with_expected_sources(expected_sources)
        .with_accumulator_health(restart_acc_health_rxs)
        .with_tenant_id(running.declaration.tenant_id.clone())
        .with_graph_executor(self.graph_executor.read().await.clone());
        // CLOACI-T-0830: re-install the reactor-constructor decider on
        // restart. The decider was resolved once at load and is shared
        // (`Arc<dyn ReactorFireDecider>`, `Send + Sync`), so the restart
        // reuses it rather than re-loading the WASM component.
        if let Some(ref ev) = running.evaluator {
            reactor = reactor.with_evaluator(ev.clone());
        }
        if let Some(ref dal) = self.dal {
            reactor = reactor.with_dal(dal.clone());
        }
        let reactor_shared = reactor.handle();
        let reactor_handle = tokio::spawn(reactor.run());

        // Re-register every endpoint-registry key the reactor was
        // originally registered under (its own name + any back-compat
        // aliases for bundled-form callers; T-0545 M1 stores these
        // explicitly on RunningGraph instead of recovering from
        // declaration.name).
        for key in &running.endpoint_registry_keys {
            if let Err(e) = self
                .registry
                .register_reactor(
                    &owner,
                    key.clone(),
                    manual_tx.clone(),
                    reactor_shared.clone(),
                )
                .await
            {
                warn!(
                    graph = %reactor_name,
                    key = %key,
                    error = %e,
                    "reactor re-registration rejected on restart"
                );
            }
        }

        // Re-set auth policies after restart
        let restart_acc_policy = match &running.declaration.tenant_id {
            Some(tid) => AccumulatorAuthPolicy::for_tenant(tid),
            None => AccumulatorAuthPolicy::allow_all(),
        };
        let restart_reactor_policy = match &running.declaration.tenant_id {
            Some(tid) => ReactorAuthPolicy::for_tenant(tid),
            None => ReactorAuthPolicy::allow_all(),
        };
        for acc_decl in &running.declaration.accumulators {
            self.registry
                .set_accumulator_policy(&owner, acc_decl.name.clone(), restart_acc_policy.clone())
                .await;
        }
        for key in &running.endpoint_registry_keys {
            self.registry
                .set_reactor_policy(&owner, key.clone(), restart_reactor_policy.clone())
                .await;
        }

        running.shutdown_tx = shutdown_tx;
        running.shutdown_rx = stored_shutdown_rx;
        running.boundary_tx = stored_boundary_tx;
        running.accumulator_handles = new_acc_handles;
        running.reactor_handle = reactor_handle;
        running.reactor_shared = reactor_shared;
        running.reactor_health_rx = Some(reactor_health_rx);
        running
            .last_success
            .insert(format!("{}::reactor", reactor_name), now);

        metrics::counter!(
            "cloacina_supervisor_restarts_total",
            "graph" => reactor_name.to_string(),
            "component" => "reactor",
            "reason" => reason,
        )
        .increment(1);
        emit_component_health(reactor_name, "reactor", "starting");
        info!(graph = %reactor_name, "reactor restarted successfully");
        true
    }

    /// Phase 3 of [`check_and_restart_failed`]: after the unlocked backoff,
    /// re-acquire the write lock and respawn a single accumulator in place.
    ///
    /// Re-validates before acting — while the lock was released the graph
    /// may have been unloaded, or the slot replaced by a full-graph restart.
    /// Returns whether a restart actually happened.
    async fn restart_accumulator_after_backoff(
        &self,
        reactor_key: &TenantKey,
        acc_name: &str,
        reason: &'static str,
        now: std::time::Instant,
    ) -> bool {
        let reactor_name = reactor_key.name.as_str();
        let mut graphs = self.reactors.write().await;
        let Some(running) = graphs.get_mut(reactor_key) else {
            info!(
                graph = %reactor_name,
                accumulator = %acc_name,
                "graph unloaded during restart backoff — skipping accumulator restart"
            );
            return false;
        };
        // Phase 1 left a finished dummy in this slot. A missing slot or a
        // live handle means another path (unload, full-graph restart)
        // already handled it — don't stomp.
        let Some(slot) = running
            .accumulator_handles
            .iter()
            .position(|(n, h)| n.as_str() == acc_name && h.is_finished())
        else {
            info!(
                graph = %reactor_name,
                accumulator = %acc_name,
                "accumulator replaced or removed during restart backoff — skipping restart"
            );
            return false;
        };

        // Find the declaration for this accumulator
        let Some(factory) = running
            .declaration
            .accumulators
            .iter()
            .find(|d| d.name == acc_name)
            .map(|d| d.factory.clone())
        else {
            error!(
                graph = %reactor_name,
                accumulator = %acc_name,
                "cannot restart: declaration not found"
            );
            running.accumulator_handles.remove(slot);
            return false;
        };

        // Re-spawn with the CURRENT boundary_tx and shutdown_rx — the
        // re-validation above guarantees the slot is still the dead one,
        // and reading the live fields keeps the respawn wired to whatever
        // channels the graph has now.
        let (health_tx, health_rx) = health_channel();
        let freshness = super::accumulator::FreshnessHandle::new();
        let spawn_config = AccumulatorSpawnConfig {
            dal: self.dal.clone(),
            health_tx: Some(health_tx),
            graph_name: reactor_name.to_string(),
            freshness: freshness.clone(),
        };
        let (socket_tx, new_handle) = factory.spawn(
            acc_name.to_string(),
            running.boundary_tx.clone(),
            running.shutdown_rx.clone(),
            spawn_config,
        );

        // Re-register socket, health, and auth policy in endpoint registry
        // under the identity claimed at load (CLOACI-T-0921).
        let owner = running.owner.clone();
        if let Err(e) = self
            .registry
            .register_accumulator(&owner, acc_name.to_string(), socket_tx)
            .await
        {
            warn!(
                graph = %reactor_name,
                accumulator = %acc_name,
                error = %e,
                "accumulator re-registration rejected on individual restart"
            );
        }
        self.registry
            .register_accumulator_health(&owner, acc_name.to_string(), health_rx)
            .await;
        self.registry
            .register_accumulator_freshness(&owner, acc_name.to_string(), freshness)
            .await;
        let ind_acc_policy = match &running.declaration.tenant_id {
            Some(tid) => AccumulatorAuthPolicy::for_tenant(tid),
            None => AccumulatorAuthPolicy::allow_all(),
        };
        self.registry
            .set_accumulator_policy(&owner, acc_name.to_string(), ind_acc_policy)
            .await;

        running
            .last_success
            .insert(format!("{}::{}", reactor_name, acc_name), now);
        running.accumulator_handles[slot].1 = new_handle;
        metrics::counter!(
            "cloacina_supervisor_restarts_total",
            "graph" => reactor_name.to_string(),
            "component" => acc_name.to_string(),
            "reason" => reason,
        )
        .increment(1);
        emit_component_health(reactor_name, acc_name, "starting");

        info!(
            graph = %reactor_name,
            accumulator = %acc_name,
            "accumulator restarted individually"
        );

        // Mark accumulators that are still running as successful
        for (name, _) in &running.accumulator_handles {
            let key = format!("{}::{}", reactor_name, name);
            running.last_success.entry(key).or_insert(now);
        }
        true
    }

    /// Start a background supervision loop that checks for crashed tasks.
    ///
    /// Returns a `JoinHandle` for the supervision task.
    pub fn start_supervision(
        self: &Arc<Self>,
        mut shutdown_rx: watch::Receiver<bool>,
        check_interval: std::time::Duration,
    ) -> JoinHandle<()> {
        let scheduler = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            interval.tick().await; // skip first immediate tick

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let restarted = scheduler.check_and_restart_failed().await;
                        if restarted > 0 {
                            info!("supervision check: restarted {} tasks", restarted);
                        }
                        scheduler.emit_health_metrics().await;
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::debug!("supervision loop shutting down");
                        break;
                    }
                }
            }
        })
    }

    /// Walk every loaded graph and emit the current
    /// `cloacina_component_health` gauge for its reactor and accumulators.
    ///
    /// Health values are derived from the existing watch channels
    /// (`ReactorHealth`, `AccumulatorHealth`) projected onto the bounded
    /// `state` label vocabulary via `as_state_label()`. Called once per
    /// supervision tick so the gauge tracks the state machine without
    /// requiring an event-driven emitter wired into every health-write
    /// site.
    pub async fn emit_health_metrics(&self) {
        let reactors = self.reactors.read().await;
        for (reactor_key, running) in reactors.iter() {
            let graph_labels = if running.endpoint_registry_keys.is_empty() {
                vec![reactor_key.name.clone()]
            } else {
                running.endpoint_registry_keys.clone()
            };

            let reactor_state = running
                .reactor_health_rx
                .as_ref()
                .map(|rx| rx.borrow().as_state_label())
                .unwrap_or("healthy");
            for label in &graph_labels {
                emit_component_health(label, "reactor", reactor_state);
            }

            for (acc_name, _) in &running.accumulator_handles {
                let acc_state = self
                    .registry
                    .get_accumulator_health(
                        acc_name,
                        super::registry::EndpointScope::of(running.owner.tenant_id.as_deref()),
                    )
                    .await
                    .map(|h| h.as_state_label())
                    .unwrap_or("healthy");
                for label in &graph_labels {
                    emit_component_health(label, acc_name, acc_state);
                }
            }
        }
    }

    /// Record a recovery event in the DAL (best-effort, logs on failure).
    async fn record_recovery_event(&self, component: &str, attempt: u32, backoff_secs: u64) {
        let dal = match &self.dal {
            Some(d) => d,
            None => return,
        };
        use crate::database::universal_types::UniversalUuid;
        use crate::models::recovery_event::NewRecoveryEvent;
        // `recovery_events.details` carries a CHECK (details::json IS NOT NULL)
        // in the postgres DDL — details MUST be valid JSON text. The previous
        // `component=…, attempt=…` plain string failed that check on every
        // graph-component restart (observed live in the T-0907 kafka lane).
        let event = NewRecoveryEvent {
            workflow_execution_id: UniversalUuid::new_v4(),
            task_execution_id: None,
            recovery_type: "graph_component_restart".to_string(),
            details: Some(
                serde_json::json!({
                    "component": component,
                    "attempt": attempt,
                    "backoff_secs": backoff_secs,
                })
                .to_string(),
            ),
        };
        if let Err(e) = dal.recovery_event().create(event).await {
            warn!(component = %component, "failed to record recovery event: {}", e);
        }
    }

    /// Graceful shutdown of all graphs.
    pub async fn shutdown_all(&self) {
        let graph_keys: Vec<TenantKey> = {
            let g2r = self.graph_to_reactor.read().await;
            g2r.keys().cloned().collect()
        };

        for key in graph_keys {
            // Address each graph in its OWN tenant scope so shutdown reaches
            // every tenant's graphs, not just the untenanted ones.
            if let Err(e) = self
                .unload_graph(&key.name, TenantScope::of(key.tenant_id.as_deref()))
                .await
            {
                warn!(graph = %key, error = %e, "failed to unload graph during shutdown");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computation_graph::accumulator::{
        accumulator_runtime, Accumulator, AccumulatorContext, AccumulatorRuntimeConfig,
        BoundarySender, CheckpointHandle,
    };
    use crate::computation_graph::types::{GraphResult, InputCache};
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicU32, Ordering};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        value: f64,
    }

    /// A simple passthrough accumulator for testing.
    struct TestAccumulatorFactory;

    impl AccumulatorFactory for TestAccumulatorFactory {
        fn spawn(
            &self,
            name: String,
            boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
            shutdown_rx: watch::Receiver<bool>,
            config: AccumulatorSpawnConfig,
        ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
            let (socket_tx, socket_rx) = mpsc::channel(64);

            struct Passthrough;

            #[async_trait::async_trait]
            impl Accumulator for Passthrough {
                type Output = TestEvent;
                fn process(&mut self, event: Vec<u8>) -> Option<TestEvent> {
                    serde_json::from_slice(&event).ok()
                }
            }

            let checkpoint = config
                .dal
                .map(|dal| CheckpointHandle::new(dal, config.graph_name.clone(), name.clone()));

            let sender = BoundarySender::with_freshness(
                boundary_tx,
                SourceName::new(&name),
                config.freshness.clone(),
            );
            let ctx = AccumulatorContext {
                output: sender,
                name: name.clone(),
                shutdown: shutdown_rx,
                checkpoint,
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

    #[tokio::test]
    async fn test_load_graph_push_event_fires() {
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
            name: "test_graph".to_string(),
            accumulators: vec![AccumulatorDeclaration {
                name: "alpha".to_string(),
                factory: Arc::new(TestAccumulatorFactory),
            }],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn,
                constructor: None,
            },
            tenant_id: None,
            reactor_name: None,
            topology: None,
        };

        scheduler.load_graph(decl).await.unwrap();

        // Push event via registry (simulating WebSocket push)
        let event = TestEvent { value: 42.0 };
        let bytes = serde_json::to_vec(&event).unwrap();
        registry
            .send_to_accumulator(
                "alpha",
                crate::computation_graph::registry::EndpointScope::untenanted(),
                bytes,
            )
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert_eq!(fire_count.load(Ordering::SeqCst), 1, "graph should fire");

        // List graphs
        let graphs = scheduler.list_graphs().await;
        assert_eq!(graphs.len(), 1);
        assert_eq!(graphs[0].name, "test_graph");
        assert!(!graphs[0].paused);

        scheduler.shutdown_all().await;
    }

    #[tokio::test]
    async fn test_unload_graph_deregisters() {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        let graph_fn: CompiledGraphFn =
            Arc::new(|_cache: InputCache| Box::pin(async { GraphResult::completed(vec![]) }));

        let decl = ComputationGraphDeclaration {
            name: "test_graph".to_string(),
            accumulators: vec![AccumulatorDeclaration {
                name: "alpha".to_string(),
                factory: Arc::new(TestAccumulatorFactory),
            }],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn,
                constructor: None,
            },
            tenant_id: None,
            reactor_name: None,
            topology: None,
        };

        scheduler.load_graph(decl).await.unwrap();

        // Verify registered
        assert_eq!(
            registry
                .accumulator_count(
                    "alpha",
                    crate::computation_graph::registry::EndpointScope::untenanted()
                )
                .await,
            1
        );
        assert!(registry
            .list_reactors()
            .await
            .contains(&"test_graph".to_string()));

        // Unload
        scheduler
            .unload_graph("test_graph", TenantScope::untenanted())
            .await
            .unwrap();

        // Verify deregistered
        assert_eq!(
            registry
                .accumulator_count(
                    "alpha",
                    crate::computation_graph::registry::EndpointScope::untenanted()
                )
                .await,
            0
        );
        assert!(registry.list_reactors().await.is_empty());
    }

    #[tokio::test]
    async fn test_duplicate_load_rejected() {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        let graph_fn: CompiledGraphFn =
            Arc::new(|_cache: InputCache| Box::pin(async { GraphResult::completed(vec![]) }));

        let decl = ComputationGraphDeclaration {
            name: "dup".to_string(),
            accumulators: vec![],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn,
                constructor: None,
            },
            tenant_id: None,
            reactor_name: None,
            topology: None,
        };

        scheduler.load_graph(decl.clone()).await.unwrap();
        let err = scheduler.load_graph(decl).await.unwrap_err();
        assert!(err.contains("already loaded"));

        scheduler.shutdown_all().await;
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0924: tenant keying of reactors / graph_to_reactor / topologies
    // -----------------------------------------------------------------------

    fn nop_graph_fn() -> CompiledGraphFn {
        Arc::new(|_cache: InputCache| Box::pin(async { GraphResult::completed(vec![]) }))
    }

    fn tenant_decl(
        graph: &str,
        reactor: &str,
        tenant: Option<&str>,
    ) -> ComputationGraphDeclaration {
        ComputationGraphDeclaration {
            name: graph.to_string(),
            accumulators: vec![AccumulatorDeclaration {
                name: format!("{}_acc", graph),
                factory: Arc::new(TestAccumulatorFactory),
            }],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn: nop_graph_fn(),
                constructor: None,
            },
            tenant_id: tenant.map(|t| t.to_string()),
            reactor_name: Some(reactor.to_string()),
            topology: Some(format!("{{\"graph\":\"{}\"}}", graph)),
        }
    }

    /// CLOACI-T-0851: losing ownership must stop the reactor EVEN THOUGH a
    /// subscriber still declares it upstream. `unload_reactor` refuses in that
    /// situation by design; the halt path must not, because the alternative is
    /// two replicas running the same reactor.
    #[tokio::test]
    async fn halt_unowned_stops_a_reactor_that_unload_would_refuse() {
        use super::super::reactor_ownership::ReactorId;

        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());
        scheduler
            .load_graph(tenant_decl("pipeline", "rx", Some("acme")))
            .await
            .expect("graph should load");
        assert_eq!(scheduler.reactors.read().await.len(), 1);

        // Precondition: the ordinary unload path refuses while a subscriber
        // remains. If this ever stops being true the test below proves less
        // than it claims, so assert it rather than assume it.
        let refused = scheduler
            .unload_reactor("rx", TenantScope::tenant("acme"))
            .await;
        assert!(
            refused.is_err(),
            "unload_reactor should refuse while a subscriber remains; got {refused:?}"
        );
        assert_eq!(scheduler.reactors.read().await.len(), 1, "still loaded");

        let stopped = scheduler
            .halt_unowned_reactors(&[ReactorId::new(Some("acme"), "rx")])
            .await;

        assert_eq!(stopped.len(), 1, "the lost reactor must be stopped");
        assert!(
            scheduler.reactors.read().await.is_empty(),
            "halted reactor must be gone from the map"
        );

        scheduler.shutdown_all().await;
    }

    /// A reactor may be unloaded normally between the liveness check and the
    /// halt. That must be a quiet no-op, not a panic or a spurious "stopped".
    #[tokio::test]
    async fn halt_unowned_is_a_noop_for_a_reactor_that_is_not_loaded() {
        use super::super::reactor_ownership::ReactorId;

        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        let stopped = scheduler
            .halt_unowned_reactors(&[ReactorId::new(Some("acme"), "never_loaded")])
            .await;
        assert!(stopped.is_empty(), "nothing was loaded, so nothing stopped");
    }

    /// Halting one tenant's reactor must not touch another tenant's same-named
    /// one — the ownership key and the scheduler key must agree.
    #[tokio::test]
    async fn halt_unowned_is_tenant_scoped() {
        use super::super::reactor_ownership::ReactorId;

        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());
        for tenant in ["acme", "globex"] {
            scheduler
                .load_graph(tenant_decl("pipeline", "rx", Some(tenant)))
                .await
                .unwrap_or_else(|e| panic!("tenant {tenant} should load: {e}"));
        }
        assert_eq!(scheduler.reactors.read().await.len(), 2);

        let stopped = scheduler
            .halt_unowned_reactors(&[ReactorId::new(Some("acme"), "rx")])
            .await;
        assert_eq!(stopped.len(), 1);

        let remaining = scheduler.list_reactors().await;
        assert_eq!(remaining.len(), 1, "globex's reactor must survive");
        assert_eq!(remaining[0].tenant_id.as_deref(), Some("globex"));

        scheduler.shutdown_all().await;
    }

    /// Two tenants load a graph AND a reactor under the SAME names on ONE
    /// shared scheduler. Both survive; neither is overwritten.
    #[tokio::test]
    async fn two_tenants_same_graph_and_reactor_names_coexist() {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        for tenant in ["acme", "globex"] {
            scheduler
                .load_graph(tenant_decl("pipeline", "rx", Some(tenant)))
                .await
                .unwrap_or_else(|e| panic!("tenant {tenant} should load its own graph: {e}"));
        }

        assert_eq!(scheduler.reactors.read().await.len(), 2);
        assert_eq!(scheduler.graph_to_reactor.read().await.len(), 2);
        assert_eq!(scheduler.graph_topologies.read().await.len(), 2);

        let graphs = scheduler.list_graphs().await;
        assert_eq!(graphs.len(), 2);
        let mut tenants: Vec<Option<String>> = graphs.iter().map(|g| g.tenant_id.clone()).collect();
        tenants.sort();
        assert_eq!(
            tenants,
            vec![Some("acme".to_string()), Some("globex".to_string())]
        );
        assert!(graphs.iter().all(|g| g.name == "pipeline"));

        let reactors = scheduler.list_reactors().await;
        assert_eq!(reactors.len(), 2);
        assert!(reactors.iter().all(|r| r.name == "rx"));

        scheduler.shutdown_all().await;
    }

    /// Unloading one tenant's graph leaves the other tenant's graph and
    /// reactor running.
    #[tokio::test]
    async fn unload_graph_is_tenant_scoped() {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        for tenant in ["acme", "globex"] {
            scheduler
                .load_graph(tenant_decl("pipeline", "rx", Some(tenant)))
                .await
                .unwrap();
        }

        scheduler
            .unload_graph("pipeline", TenantScope::tenant("acme"))
            .await
            .expect("acme unloads its own graph");

        // Only globex's entries remain — and they are globex's.
        let reactors = scheduler.list_reactors().await;
        assert_eq!(reactors.len(), 1);
        assert_eq!(reactors[0].tenant_id.as_deref(), Some("globex"));
        let graphs = scheduler.list_graphs().await;
        assert_eq!(graphs.len(), 1);
        assert_eq!(graphs[0].tenant_id.as_deref(), Some("globex"));
        assert_eq!(scheduler.graph_topologies.read().await.len(), 1);

        scheduler.shutdown_all().await;
    }

    /// A tenant cannot see, unbind, or tear down another tenant's reactor.
    #[tokio::test]
    async fn other_tenants_reactors_are_unreachable() {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        scheduler
            .load_graph(tenant_decl("pipeline", "rx", Some("acme")))
            .await
            .unwrap();

        let outsider = TenantScope::tenant("globex");
        assert!(scheduler
            .reactor_accumulator_names("rx", outsider)
            .await
            .is_none());
        assert!(scheduler.unload_reactor("rx", outsider).await.is_err());
        assert!(scheduler
            .unbind_graph_from_reactor("pipeline", outsider)
            .await
            .is_err());
        assert!(scheduler
            .bind_graph_to_reactor(
                "intruder".to_string(),
                "rx".to_string(),
                outsider,
                nop_graph_fn(),
            )
            .await
            .is_err());

        // The owner's entries are untouched.
        assert!(scheduler
            .reactor_accumulator_names("rx", TenantScope::tenant("acme"))
            .await
            .is_some());
        assert_eq!(scheduler.reactors.read().await.len(), 1);

        scheduler.shutdown_all().await;
    }

    /// EMBEDDED COMPATIBILITY: with `tenant_id: None` everywhere, keys are bare
    /// names and the whole lifecycle behaves exactly as it did pre-T-0924 —
    /// including a tenant view resolving the untenanted reactor via fallback.
    #[tokio::test]
    async fn untenanted_lifecycle_is_unchanged_and_globally_addressable() {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        scheduler
            .load_graph(tenant_decl("pipeline", "rx", None))
            .await
            .unwrap();

        let key = scheduler
            .reactors
            .read()
            .await
            .keys()
            .next()
            .unwrap()
            .clone();
        assert_eq!(key, TenantKey::new(None, "rx"));

        // Untenanted callers address it by bare name, as always…
        assert!(scheduler
            .reactor_accumulator_names("rx", TenantScope::untenanted())
            .await
            .is_some());
        // …and a tenant-scoped caller reaches it through the untenanted
        // fallback, which is how an embedded/inventory reactor stays usable
        // once a deployment grows tenants.
        assert!(scheduler
            .reactor_accumulator_names("rx", TenantScope::tenant("acme"))
            .await
            .is_some());

        scheduler
            .unload_graph("pipeline", TenantScope::untenanted())
            .await
            .unwrap();
        assert!(scheduler.reactors.read().await.is_empty());
        assert!(scheduler.graph_to_reactor.read().await.is_empty());
        assert!(scheduler.graph_topologies.read().await.is_empty());
    }

    /// A tenant may subscribe to an untenanted (embedded) upstream reactor,
    /// and `unload_graph` follows the binding back to that exact entry rather
    /// than looking for a same-named reactor in the subscriber's own tenant.
    #[tokio::test]
    async fn tenant_graph_can_bind_untenanted_upstream() {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        scheduler
            .load_reactor(
                "upstream".to_string(),
                vec![AccumulatorDeclaration {
                    name: "alpha".to_string(),
                    factory: Arc::new(TestAccumulatorFactory),
                }],
                ReactionCriteria::WhenAny,
                InputStrategy::Latest,
                None,
                vec![],
                None,
            )
            .await
            .unwrap();

        scheduler
            .bind_graph_to_reactor(
                "subscriber".to_string(),
                "upstream".to_string(),
                TenantScope::tenant("acme"),
                nop_graph_fn(),
            )
            .await
            .expect("a tenant may subscribe to an untenanted upstream");

        // The binding records the UNTENANTED reactor key…
        let bound = scheduler
            .graph_to_reactor
            .read()
            .await
            .get(&TenantKey::new(Some("acme"), "subscriber"))
            .cloned();
        assert_eq!(bound, Some(TenantKey::new(None, "upstream")));

        // …and unloading the subscriber (its last subscriber) tears down that
        // exact reactor.
        scheduler
            .unload_graph("subscriber", TenantScope::tenant("acme"))
            .await
            .unwrap();
        assert!(scheduler.reactors.read().await.is_empty());
    }

    /// Same tenant, same graph name from two packages is still "already
    /// loaded" — the loud same-tenant collision the ticket asks for.
    #[tokio::test]
    async fn same_tenant_duplicate_graph_is_rejected() {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());

        scheduler
            .load_graph(tenant_decl("pipeline", "rx_a", Some("acme")))
            .await
            .unwrap();
        let err = scheduler
            .load_graph(tenant_decl("pipeline", "rx_b", Some("acme")))
            .await
            .expect_err("a second package in the same tenant must not silently replace");
        assert!(err.contains("already loaded"), "{err}");

        scheduler.shutdown_all().await;
    }
}
