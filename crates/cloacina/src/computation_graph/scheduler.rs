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

use super::accumulator::{health_channel, shutdown_signal, AccumulatorHealth, CheckpointHandle};
use super::reactor::{
    reactor_health_channel, CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor,
    ReactorHandle,
};
use super::registry::{AccumulatorAuthPolicy, EndpointRegistry, ReactorAuthPolicy};
use super::types::{GraphResult, InputCache, SourceName};

/// Declaration of a computation graph to be loaded by the Reactive Scheduler.
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
    pub criteria: ReactionCriteria,
    /// Input strategy (latest / sequential).
    pub strategy: InputStrategy,
    /// The compiled graph function.
    pub graph_fn: CompiledGraphFn,
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

            // Pass 2: log per-subscriber errors. No short-circuit; the reactor
            // treats this as one firing regardless of how many subscribers
            // succeeded.
            for (graph_name, result) in results {
                if let GraphResult::Error(e) = result {
                    tracing::error!(
                        reactor = %reactor_name,
                        graph = %graph_name,
                        "subscriber graph failed: {}",
                        e
                    );
                }
            }
            GraphResult::completed(vec![])
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
    /// Manual command sender, kept here so the supervisor's restart path
    /// can re-register the same channel under the same keys without going
    /// back through `register_reactor` from scratch.
    manual_tx: mpsc::Sender<super::reactor::ManualCommand>,
    /// Per-component consecutive failure count.
    failure_counts: HashMap<String, u32>,
    /// Timestamp of last successful operation per component (for failure count reset).
    last_success: HashMap<String, std::time::Instant>,
}

/// Maximum consecutive failures before a component is permanently abandoned.
const MAX_RECOVERY_ATTEMPTS: u32 = 5;

/// Base delay for exponential backoff (doubles on each failure, capped at 60s).
const BACKOFF_BASE_SECS: u64 = 1;

/// Maximum backoff delay.
const BACKOFF_MAX_SECS: u64 = 60;

/// Duration of successful operation before failure counter resets.
const SUCCESS_RESET_SECS: u64 = 60;

/// The Reactive Scheduler.
pub struct ComputationGraphScheduler {
    /// Endpoint registry for WebSocket routing.
    registry: EndpointRegistry,
    /// Running reactors, keyed by reactor name. Each reactor owns a subscriber
    /// map that may contain one or more graphs sharing this reactor instance.
    reactors: Arc<RwLock<HashMap<String, RunningGraph>>>,
    /// Maps graph_name → reactor_name so external operations that take a
    /// graph_name (`unload_graph`, `list_graphs`) can find the reactor that
    /// hosts it.
    graph_to_reactor: Arc<RwLock<HashMap<String, String>>>,
    /// DAL handle for persistence. None in embedded/test mode.
    dal: Option<crate::dal::unified::DAL>,
}

impl ComputationGraphScheduler {
    pub fn new(registry: EndpointRegistry) -> Self {
        Self {
            registry,
            reactors: Arc::new(RwLock::new(HashMap::new())),
            graph_to_reactor: Arc::new(RwLock::new(HashMap::new())),
            dal: None,
        }
    }

    /// Create a scheduler with DAL support for persistence and health tracking.
    pub fn with_dal(registry: EndpointRegistry, dal: crate::dal::unified::DAL) -> Self {
        Self {
            registry,
            reactors: Arc::new(RwLock::new(HashMap::new())),
            graph_to_reactor: Arc::new(RwLock::new(HashMap::new())),
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
    pub async fn load_reactor(
        &self,
        reactor_name: String,
        accumulators: Vec<AccumulatorDeclaration>,
        criteria: ReactionCriteria,
        strategy: InputStrategy,
        tenant_id: Option<String>,
        register_aliases: Vec<String>,
    ) -> Result<(), String> {
        // Idempotent path: matching contract → no-op.
        {
            let reactors = self.reactors.read().await;
            if let Some(existing) = reactors.get(&reactor_name) {
                let probe = ComputationGraphDeclaration {
                    name: reactor_name.clone(),
                    accumulators: accumulators.clone(),
                    reactor: ReactorDeclaration {
                        criteria: criteria.clone(),
                        strategy: strategy.clone(),
                        graph_fn: dummy_graph_fn(),
                    },
                    tenant_id: tenant_id.clone(),
                    reactor_name: Some(reactor_name.clone()),
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
        let mut accumulator_handles = Vec::new();
        let mut acc_health_rxs: Vec<(
            String,
            watch::Receiver<super::accumulator::AccumulatorHealth>,
        )> = Vec::new();
        for acc_decl in &accumulators {
            let (health_tx, health_rx) = health_channel();
            acc_health_rxs.push((acc_decl.name.clone(), health_rx.clone()));

            let spawn_config = AccumulatorSpawnConfig {
                dal: self.dal.clone(),
                health_tx: Some(health_tx),
                graph_name: reactor_name.clone(),
            };

            let (socket_tx, handle) = acc_decl.factory.spawn(
                acc_decl.name.clone(),
                boundary_tx.clone(),
                shutdown_rx.clone(),
                spawn_config,
            );

            self.registry
                .register_accumulator(acc_decl.name.clone(), socket_tx)
                .await;
            self.registry
                .register_accumulator_health(acc_decl.name.clone(), health_rx)
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
        .with_accumulator_health(acc_health_rxs);

        if let Some(ref dal) = self.dal {
            reactor = reactor.with_dal(dal.clone());
        }

        let reactor_shared = reactor.handle();

        // Register reactor under its name + any aliases. Both keys point at
        // the same manual channel + handle.
        let mut endpoint_registry_keys = vec![reactor_name.clone()];
        self.registry
            .register_reactor(
                reactor_name.clone(),
                manual_tx.clone(),
                reactor_shared.clone(),
            )
            .await;
        for alias in &register_aliases {
            if alias != &reactor_name {
                self.registry
                    .register_reactor(alias.clone(), manual_tx.clone(), reactor_shared.clone())
                    .await;
                endpoint_registry_keys.push(alias.clone());
            }
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
                .set_accumulator_policy(acc_decl.name.clone(), acc_policy.clone())
                .await;
        }
        for key in &endpoint_registry_keys {
            self.registry
                .set_reactor_policy(key.clone(), reactor_policy.clone())
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
            },
            tenant_id,
            reactor_name: Some(reactor_name.clone()),
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
            subscribers,
            endpoint_registry_keys,
            manual_tx,
            failure_counts: HashMap::new(),
            last_success: HashMap::new(),
        };

        self.reactors.write().await.insert(reactor_name, running);
        Ok(())
    }

    /// Bind a graph as an additional subscriber on an already-loaded reactor.
    ///
    /// The reactor must have been loaded first (via [`load_reactor`] or
    /// transitively via [`load_graph`]); this entry point doesn't spawn
    /// reactors. Returns an error if the reactor isn't loaded or if a graph
    /// with the same name is already bound somewhere.
    pub async fn bind_graph_to_reactor(
        &self,
        graph_name: String,
        reactor_name: String,
        graph_fn: CompiledGraphFn,
    ) -> Result<(), String> {
        {
            let g2r = self.graph_to_reactor.read().await;
            if g2r.contains_key(&graph_name) {
                return Err(format!("graph '{}' already loaded", graph_name));
            }
        }

        {
            let reactors = self.reactors.read().await;
            let existing = reactors
                .get(&reactor_name)
                .ok_or_else(|| format!("reactor '{}' is not loaded", reactor_name))?;
            existing
                .subscribers
                .write()
                .await
                .insert(graph_name.clone(), graph_fn);
        }
        self.graph_to_reactor
            .write()
            .await
            .insert(graph_name.clone(), reactor_name.clone());

        info!(
            graph = %graph_name,
            reactor = %reactor_name,
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

        // Pre-check: reject re-loading the same graph regardless of which
        // reactor it was bound to. (load_reactor + bind_graph_to_reactor
        // would catch this too, but doing it here keeps the error message
        // precise.)
        {
            let g2r = self.graph_to_reactor.read().await;
            if g2r.contains_key(&name) {
                return Err(format!("graph '{}' already loaded", name));
            }
        }

        // Load (or join) the reactor. We register the graph's name as an
        // alias so `cloacinactl reactor force-fire <graph>` keeps working
        // for bundled-form callers and for the first graph that names a
        // shared reactor (T-0544 M2 surface promise).
        self.load_reactor(
            reactor_name.clone(),
            decl.accumulators.clone(),
            decl.reactor.criteria.clone(),
            decl.reactor.strategy.clone(),
            decl.tenant_id.clone(),
            vec![name.clone()],
        )
        .await?;

        self.bind_graph_to_reactor(name, reactor_name, decl.reactor.graph_fn)
            .await
    }

    /// Load a computation graph that references a reactor declaration by
    /// value (split form, from `#[computation_graph(trigger = reactor(T))]`).
    ///
    /// This spawns a fresh reactor instance tied to this graph, using the
    /// criteria + accumulator list carried by `reactor`, and binds `graph_fn`
    /// as the firing callback. The reactor is registered in the endpoint
    /// registry under `graph_name` (not the reactor's own name), which keeps
    /// parity with today's bundled-form operational surface. Sharing a
    /// single reactor instance across multiple graphs is a later step
    /// (T-01b) — for now, "split form" means "the user declared the reactor
    /// separately and referenced it by type path," and the linkage still
    /// gets one reactor instance per graph.
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
            },
            tenant_id,
            // Split-form callers carry an explicit reactor identity. Multiple
            // graphs naming the same reactor here share one reactor instance
            // (T-0544 fan-out).
            reactor_name: Some(reactor.name.clone()),
        };

        self.load_graph(decl).await
    }

    /// Unbind a graph from its reactor without affecting the reactor itself.
    ///
    /// The graph stops being a subscriber but the reactor (and its
    /// accumulators) keeps running, ready for new subscribers. This is the
    /// honest lifecycle primitive — reactors are independent units; binding
    /// and unbinding subscribers is decoupled from reactor teardown.
    pub async fn unbind_graph_from_reactor(&self, name: &str) -> Result<String, String> {
        let reactor_name = {
            let mut g2r = self.graph_to_reactor.write().await;
            g2r.remove(name)
                .ok_or_else(|| format!("graph '{}' not loaded", name))?
        };

        let remaining = {
            let reactors = self.reactors.read().await;
            if let Some(running) = reactors.get(&reactor_name) {
                let mut subs = running.subscribers.write().await;
                subs.remove(name);
                subs.len()
            } else {
                // graph_to_reactor pointed at a missing reactor — surface as
                // an error rather than silently no-oping.
                return Err(format!(
                    "graph '{}' was bound to reactor '{}' but the reactor is not loaded",
                    name, reactor_name
                ));
            }
        };

        info!(
            graph = %name,
            reactor = %reactor_name,
            remaining_subscribers = remaining,
            "graph unbound from reactor"
        );
        Ok(reactor_name)
    }

    /// Tear down a reactor and its accumulators. Rejects if the reactor has
    /// any bound subscribers — operators must unbind subscribers first. This
    /// is the lifecycle guard that makes "reactors as independent units"
    /// safe: a reactor never disappears out from under a graph that's still
    /// declaring it as an upstream.
    pub async fn unload_reactor(&self, reactor_name: &str) -> Result<(), String> {
        // Snapshot subscribers under read lock so we can build a precise
        // error message if any remain.
        let subscriber_names: Vec<String> = {
            let reactors = self.reactors.read().await;
            match reactors.get(reactor_name) {
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
                .remove(reactor_name)
                .ok_or_else(|| format!("reactor '{}' not loaded", reactor_name))?
        };

        let _ = running.shutdown_tx.send(true);
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), running.reactor_handle).await;

        for (acc_name, handle) in running.accumulator_handles {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
            self.registry.deregister_accumulator(&acc_name).await;
        }

        // Deregister every endpoint-registry key the reactor was registered
        // under (its own name + any back-compat aliases for bundled-form
        // callers).
        for key in &running.endpoint_registry_keys {
            self.registry.deregister_reactor(key).await;
        }

        info!(reactor = %reactor_name, "reactor unloaded");
        Ok(())
    }

    /// Backward-compat convenience: unbind the graph from its reactor and,
    /// if it was the last subscriber, also tear down the reactor. This
    /// preserves today's 1:1 reactor-per-graph callers (a single
    /// `unload_graph(name)` removes everything the matching `load_graph`
    /// brought in). For independent reactor lifecycles, prefer
    /// [`unbind_graph_from_reactor`] + explicit [`unload_reactor`].
    pub async fn unload_graph(&self, name: &str) -> Result<(), String> {
        let reactor_name = self.unbind_graph_from_reactor(name).await?;

        // If subscribers are now empty, tear down the reactor for back-compat
        // with bundled-form callers.
        let now_empty = {
            let reactors = self.reactors.read().await;
            match reactors.get(&reactor_name) {
                Some(running) => running.subscribers.read().await.is_empty(),
                None => false,
            }
        };
        if now_empty {
            self.unload_reactor(&reactor_name).await?;
        }
        info!(graph = %name, reactor = %reactor_name, "computation graph unloaded");
        Ok(())
    }

    /// Snapshot the accumulator names of a loaded reactor, in declaration
    /// order. Returns `None` if the reactor isn't loaded. Used by the
    /// reconciler to pre-validate cross-package subscriber bindings against
    /// the upstream reactor's contract before calling [`load_graph`].
    pub async fn reactor_accumulator_names(&self, reactor_name: &str) -> Option<Vec<String>> {
        let reactors = self.reactors.read().await;
        reactors.get(reactor_name).map(|running| {
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
        g2r.iter()
            .filter_map(|(graph_name, reactor_name)| {
                reactors.get(reactor_name).map(|running| GraphStatus {
                    name: graph_name.clone(),
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
                })
            })
            .collect()
    }

    /// Check all graphs for crashed tasks and restart them.
    ///
    /// Individual accumulators are restarted in-place without tearing down the
    /// reactor. Reactor crashes trigger a full-graph restart. Failure counting
    /// with exponential backoff prevents infinite restart loops.
    pub async fn check_and_restart_failed(&self) -> usize {
        let mut restarted = 0;
        let mut graphs = self.reactors.write().await;
        let now = std::time::Instant::now();

        for (graph_name, running) in graphs.iter_mut() {
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
                let reactor_key = format!("{}::reactor", graph_name);
                let failures = running
                    .failure_counts
                    .entry(reactor_key.clone())
                    .or_insert(0);
                *failures += 1;

                if *failures > MAX_RECOVERY_ATTEMPTS {
                    error!(
                        graph = %graph_name,
                        failures = *failures,
                        "reactor permanently failed — circuit breaker open"
                    );
                    continue;
                }

                let backoff_secs =
                    (BACKOFF_BASE_SECS * 2u64.pow(*failures - 1)).min(BACKOFF_MAX_SECS);
                let backoff = std::time::Duration::from_secs(backoff_secs);
                warn!(
                    graph = %graph_name,
                    attempt = *failures,
                    backoff_secs = backoff_secs,
                    "reactor crashed, restarting (full graph restart)"
                );

                // Record recovery event and wait for backoff
                self.record_recovery_event(&reactor_key, *failures, backoff_secs)
                    .await;
                tokio::time::sleep(backoff).await;

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

                let mut new_acc_handles = Vec::new();
                let mut restart_acc_health_rxs: Vec<(
                    String,
                    watch::Receiver<super::accumulator::AccumulatorHealth>,
                )> = Vec::new();
                for acc_decl in &running.declaration.accumulators {
                    let (health_tx, health_rx) = health_channel();
                    restart_acc_health_rxs.push((acc_decl.name.clone(), health_rx.clone()));
                    let spawn_config = AccumulatorSpawnConfig {
                        dal: self.dal.clone(),
                        health_tx: Some(health_tx),
                        graph_name: graph_name.clone(),
                    };
                    let (socket_tx, handle) = acc_decl.factory.spawn(
                        acc_decl.name.clone(),
                        boundary_tx.clone(),
                        shutdown_rx.clone(),
                        spawn_config,
                    );
                    self.registry
                        .register_accumulator(acc_decl.name.clone(), socket_tx)
                        .await;
                    self.registry
                        .register_accumulator_health(acc_decl.name.clone(), health_rx)
                        .await;
                    new_acc_handles.push((acc_decl.name.clone(), handle));
                }

                let (manual_tx, manual_rx) = mpsc::channel(64);
                let (reactor_health_tx, reactor_health_rx) = reactor_health_channel();
                // Reuse the same subscriber map across restart so subscribers
                // bound mid-life don't get dropped when the reactor restarts.
                let restart_dispatcher =
                    make_subscriber_dispatcher(graph_name.clone(), running.subscribers.clone());
                let mut reactor = Reactor::new(
                    restart_dispatcher,
                    running.declaration.reactor.criteria.clone(),
                    running.declaration.reactor.strategy.clone(),
                    boundary_rx,
                    manual_rx,
                    shutdown_rx,
                )
                .with_graph_name(graph_name.clone())
                .with_health(reactor_health_tx)
                .with_expected_sources(expected_sources)
                .with_accumulator_health(restart_acc_health_rxs);
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
                running.manual_tx = manual_tx.clone();
                for key in &running.endpoint_registry_keys {
                    self.registry
                        .register_reactor(key.clone(), manual_tx.clone(), reactor_shared.clone())
                        .await;
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
                        .set_accumulator_policy(acc_decl.name.clone(), restart_acc_policy.clone())
                        .await;
                }
                for key in &running.endpoint_registry_keys {
                    self.registry
                        .set_reactor_policy(key.clone(), restart_reactor_policy.clone())
                        .await;
                }

                running.shutdown_tx = shutdown_tx;
                running.shutdown_rx = stored_shutdown_rx;
                running.boundary_tx = stored_boundary_tx;
                running.accumulator_handles = new_acc_handles;
                running.reactor_handle = reactor_handle;
                running.reactor_shared = reactor_shared;
                running.reactor_health_rx = Some(reactor_health_rx);
                running.last_success.insert(reactor_key, now);

                restarted += 1;
                info!(graph = %graph_name, "reactor restarted successfully");
            } else {
                // Check individual accumulators — restart them in-place
                let mut new_handles = Vec::new();
                let mut changed = false;

                for (acc_name, handle) in running.accumulator_handles.drain(..) {
                    if handle.is_finished() {
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
                            // Don't add handle back — accumulator is abandoned
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

                        // Record recovery event and wait for backoff
                        self.record_recovery_event(&acc_key, *failures, backoff_secs)
                            .await;
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;

                        // Find the declaration for this accumulator
                        if let Some(acc_decl) = running
                            .declaration
                            .accumulators
                            .iter()
                            .find(|d| d.name == *acc_name)
                        {
                            // Re-spawn with existing boundary_tx and shutdown_rx
                            let (health_tx, health_rx) = health_channel();
                            let spawn_config = AccumulatorSpawnConfig {
                                dal: self.dal.clone(),
                                health_tx: Some(health_tx),
                                graph_name: graph_name.clone(),
                            };
                            let (socket_tx, new_handle) = acc_decl.factory.spawn(
                                acc_name.clone(),
                                running.boundary_tx.clone(),
                                running.shutdown_rx.clone(),
                                spawn_config,
                            );

                            // Re-register socket, health, and auth policy in endpoint registry
                            self.registry
                                .register_accumulator(acc_name.clone(), socket_tx)
                                .await;
                            self.registry
                                .register_accumulator_health(acc_name.clone(), health_rx)
                                .await;
                            let ind_acc_policy = match &running.declaration.tenant_id {
                                Some(tid) => AccumulatorAuthPolicy::for_tenant(tid),
                                None => AccumulatorAuthPolicy::allow_all(),
                            };
                            self.registry
                                .set_accumulator_policy(acc_name.clone(), ind_acc_policy)
                                .await;

                            running.last_success.insert(acc_key, now);
                            let restarted_name = acc_name.clone();
                            new_handles.push((acc_name, new_handle));
                            restarted += 1;
                            changed = true;

                            info!(
                                graph = %graph_name,
                                accumulator = %restarted_name,
                                "accumulator restarted individually"
                            );
                        } else {
                            let lost_name = acc_name.clone();
                            error!(
                                graph = %graph_name,
                                accumulator = %lost_name,
                                "cannot restart: declaration not found"
                            );
                        }
                    } else {
                        new_handles.push((acc_name, handle));
                    }
                }

                running.accumulator_handles = new_handles;

                if changed {
                    // Mark accumulators that are still running as successful
                    for (acc_name, _) in &running.accumulator_handles {
                        let acc_key = format!("{}::{}", graph_name, acc_name);
                        running.last_success.entry(acc_key).or_insert(now);
                    }
                }
            }
        }

        restarted
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
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::debug!("supervision loop shutting down");
                        break;
                    }
                }
            }
        })
    }

    /// Record a recovery event in the DAL (best-effort, logs on failure).
    async fn record_recovery_event(&self, component: &str, attempt: u32, backoff_secs: u64) {
        let dal = match &self.dal {
            Some(d) => d,
            None => return,
        };
        use crate::database::universal_types::UniversalUuid;
        use crate::models::recovery_event::NewRecoveryEvent;
        let event = NewRecoveryEvent {
            workflow_execution_id: UniversalUuid::new_v4(),
            task_execution_id: None,
            recovery_type: "graph_component_restart".to_string(),
            details: Some(format!(
                "component={}, attempt={}, backoff={}s",
                component, attempt, backoff_secs
            )),
        };
        if let Err(e) = dal.recovery_event().create(event).await {
            warn!(component = %component, "failed to record recovery event: {}", e);
        }
    }

    /// Graceful shutdown of all graphs.
    pub async fn shutdown_all(&self) {
        let graph_names: Vec<String> = {
            let g2r = self.graph_to_reactor.read().await;
            g2r.keys().cloned().collect()
        };

        for name in graph_names {
            if let Err(e) = self.unload_graph(&name).await {
                warn!(graph = %name, error = %e, "failed to unload graph during shutdown");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computation_graph::accumulator::{
        accumulator_runtime, Accumulator, AccumulatorContext, AccumulatorRuntimeConfig,
        BoundarySender,
    };
    use crate::computation_graph::types::{serialize, GraphResult, InputCache};
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

            let sender = BoundarySender::new(boundary_tx, SourceName::new(&name));
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
            },
            tenant_id: None,
            reactor_name: None,
        };

        scheduler.load_graph(decl).await.unwrap();

        // Push event via registry (simulating WebSocket push)
        let event = TestEvent { value: 42.0 };
        let bytes = serde_json::to_vec(&event).unwrap();
        registry.send_to_accumulator("alpha", bytes).await.unwrap();

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
            },
            tenant_id: None,
            reactor_name: None,
        };

        scheduler.load_graph(decl).await.unwrap();

        // Verify registered
        assert_eq!(registry.accumulator_count("alpha").await, 1);
        assert!(registry
            .list_reactors()
            .await
            .contains(&"test_graph".to_string()));

        // Unload
        scheduler.unload_graph("test_graph").await.unwrap();

        // Verify deregistered
        assert_eq!(registry.accumulator_count("alpha").await, 0);
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
            },
            tenant_id: None,
            reactor_name: None,
        };

        scheduler.load_graph(decl.clone()).await.unwrap();
        let err = scheduler.load_graph(decl).await.unwrap_err();
        assert!(err.contains("already loaded"));

        scheduler.shutdown_all().await;
    }
}
