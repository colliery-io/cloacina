---
title: "cloacina::computation_graph::scheduler"
description: "Computation graph scheduler — spawns, supervises, and shuts down accumulator/reactor tasks from computation graph declarations."
editUrl: false
sidebar:
  order: 50
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Computation graph scheduler — spawns, supervises, and shuts down accumulator/reactor tasks from computation graph declarations.

The companion to the Unified Scheduler for the computation graph
primitive. Receives declarations from the reconciler, wires channels,
spawns tokio tasks, registers endpoints, and restarts tasks on panic.

## Structs

### <span id="struct.ComputationGraphDeclaration"></span><code style="font-size: 0.85em">struct ComputationGraphDeclaration</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Declaration of a computation graph to be loaded by the [`ComputationGraphScheduler`](../../../../rust/cloacina/computation_graph/scheduler/#struct.ComputationGraphScheduler).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Unique name for this computation graph. |
| `accumulators` | `Vec<AccumulatorDeclaration>` | Accumulator declarations. |
| `reactor` | `ReactorDeclaration` | Reactor declaration. |
| `tenant_id` | `Option<String>` | Tenant that owns this graph (None = global/public). |
| `reactor_name` | `Option<String>` | Explicit reactor name. When `Some(name)`, multiple graph declarations referencing the same reactor name share a single reactor instance — the second `load_graph` call with a matching contract is idempotent on the reactor and just binds the new graph as an additional subscriber. `None` (today's bundled-form default) synthesizes a per-graph reactor name (`__Reactor_<graph_name>`) to preserve the 1:1 reactor-per-graph behavior callers expect. |
| `topology` | `Option<String>` | Serialized node/edge topology JSON for this graph (from the package's FFI metadata), retained so the health API can surface the CG DAG. `None` for packages predating topology emission. (CLOACI-T-0673) |

### <span id="struct.AccumulatorDeclaration"></span><code style="font-size: 0.85em">struct AccumulatorDeclaration</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Declaration for a single accumulator.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Accumulator name (used as WebSocket endpoint name). |
| `factory` | `Arc<dyn AccumulatorFactory>` | Factory that creates the accumulator instance. |

### <span id="struct.AccumulatorSpawnConfig"></span><code style="font-size: 0.85em">struct AccumulatorSpawnConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Configuration passed to [`AccumulatorFactory::spawn`](../../../../rust/cloacina/computation_graph/scheduler/#AccumulatorFactory.spawn) for resilience wiring.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `Option<crate::dal::unified::DAL>` | DAL handle for checkpoint persistence. None in embedded/test mode. |
| `health_tx` | `Option<watch::Sender<AccumulatorHealth>>` | Health state reporter. None when health tracking is not needed. |
| `graph_name` | `String` | Graph name (used as key for checkpoint persistence). |
| `freshness` | `super::accumulator::FreshnessHandle` | Shared freshness probe for the accumulator's BoundarySender (CLOACI-T-0765). The factory builds the sender via `BoundarySender::with_freshness` so the registry can report events_total + last-event for this source. |

### <span id="struct.ReactorDeclaration"></span><code style="font-size: 0.85em">struct ReactorDeclaration</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Declaration for the reactor.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `criteria` | `ReactionCriteria` | Reaction criteria (when_any / when_all).<br>Ignored at runtime when `constructor` is `Some(..)` — a reactor constructor's WASM `evaluate` replaces the dirty-flag criteria. |
| `strategy` | `InputStrategy` | Input strategy (latest / sequential). |
| `graph_fn` | `CompiledGraphFn` | The compiled graph function. |
| `constructor` | `Option<cloacina_computation_graph::ReactorConstructorRef>` | Optional packaged WASM reactor-constructor reference (CLOACI-T-0830).<br>`Some(..)` makes the named constructor's WASM `evaluate` the reactor's firing decision: [`load_reactor`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.load_reactor) resolves it against the T-0829 provider search path and installs it via [`Reactor::with_evaluator`](../../../../rust/cloacina/computation_graph/reactor/#Reactor.with_evaluator), replacing the built-in `criteria`. `None` (the default for every existing path) is the native dirty-flag reactor. |

### <span id="struct.GraphStatus"></span><code style="font-size: 0.85em">struct GraphStatus</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Status of a managed computation graph.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulators` | `Vec<String>` |  |
| `paused` | `bool` |  |
| `running` | `bool` |  |
| `health` | `Option<super::reactor::ReactorHealth>` | Reactor health state machine value. None if health tracking is not configured. |
| `tenant_id` | `Option<String>` | Tenant scope of the graph at load time. `None` for single-tenant or admin-owned graphs. CLOACI-T-0579: surfaced so per-tenant health endpoints can filter by caller authorization. |
| `topology` | `Option<String>` | Serialized node/edge topology JSON for this graph, so the health API can render the CG DAG. `None` for graphs predating topology emission. (CLOACI-T-0673) |
| `reactor` | `Option<String>` | Name of the reactor this graph is bound to (the trigger that fires it). `None` only if no reactor identity was recorded. (CLOACI-T-0673 follow-up) |
| `reaction_mode` | `String` | Reaction mode of the bound reactor: `"when_any"` \| `"when_all"`. |
| `input_strategy` | `String` | Input strategy of the bound reactor: `"latest"` \| `"sequential"`. |
| `fires` | `u64` | Total graph fires since load (live reactor counter, WS-10). |
| `last_fire_unix_ms` | `Option<i64>` | Unix-epoch millis of the last fire; `None` if it hasn't fired yet. |

### <span id="struct.ReactorStatus"></span><code style="font-size: 0.85em">struct ReactorStatus</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Status of a managed reactor (CLOACI-T-0742). Reactors are first-class: a reactor is loaded (`load_reactor`) and graphs bind to it afterward (`bind_graph_to_reactor`), so a reactor can be running with **no graph bound**. `list_graphs` is graph-first and would omit such a reactor; this is reactor-first, sourced directly from the `reactors` map.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Reactor name (the `reactors` map key). |
| `accumulators` | `Vec<String>` | Accumulators this reactor consumes, in declaration order. |
| `reaction_mode` | `String` | Firing criteria: `"when_any"` \| `"when_all"`. |
| `input_strategy` | `String` | Input strategy: `"latest"` \| `"sequential"`. |
| `bound_graphs` | `Vec<String>` | Graphs bound to this reactor (empty if the reactor has no graph yet). |
| `paused` | `bool` |  |
| `running` | `bool` |  |
| `health` | `Option<super::reactor::ReactorHealth>` | Reactor health state machine value. None if health tracking isn't configured. |
| `tenant_id` | `Option<String>` | Tenant scope at load time. `None` for single-tenant / admin-owned reactors. |
| `fires` | `u64` | Total fires since load (live reactor counter, WS-10). |
| `last_fire_unix_ms` | `Option<i64>` | Unix-epoch millis of the last fire; `None` if it hasn't fired yet. |

### <span id="struct.ComputationGraphScheduler"></span><code style="font-size: 0.85em">struct ComputationGraphScheduler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

The computation graph scheduler: loads reactors and computation graphs, supervises them, and routes operational commands to running instances.

#### Methods

##### <span id="ComputationGraphScheduler.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(registry: EndpointRegistry) -> Self
```

<details>
<summary>Source</summary>

```rust
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
            ownership: None,
            foreign_reactors: Arc::new(RwLock::new(std::collections::HashSet::new())),
            foreign_accumulators: Arc::new(RwLock::new(HashMap::new())),
            foreign_pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }
```

</details>

##### <span id="ComputationGraphScheduler.with_dal"></span><code>with_​dal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_dal(
    registry: EndpointRegistry,
    dal: crate::dal::unified::DAL,
) -> Self
```

Create a scheduler with DAL support for persistence and health tracking.

<details>
<summary>Source</summary>

```rust
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
            ownership: None,
            foreign_reactors: Arc::new(RwLock::new(std::collections::HashSet::new())),
            foreign_accumulators: Arc::new(RwLock::new(HashMap::new())),
            foreign_pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }
```

</details>

##### <span id="ComputationGraphScheduler.set_graph_executor"></span><code>set_​graph_​executor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_graph_executor(
    &self,
    executor: Arc<dyn super::graph_executor::GraphExecutor>,
)
```

Load and start a reactor with no subscribers.

Idempotent on `(reactor_name, contract)`: if a reactor with this name
is already running and the contract matches (accumulators, criteria,
strategy, tenant_id), this returns `Ok(())` without spawning anything.
A mismatched contract returns a precise error.

`register_aliases` lets the caller register additional endpoint-registry
keys pointing at this reactor's manual command channel — used by
[`load_graph`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.load_graph) to alias the first graph's name for back-compat with
today's `cloacinactl reactor force-fire <graph>` operator surface.
Direct callers (e.g. T-0545's reconciler routing for reactor-only
packages) typically pass `&[]` and address the reactor by its name.

Subscribers are bound separately via [`bind_graph_to_reactor`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.bind_graph_to_reactor).
Swap the graph executor firings run through (CLOACI-T-0722). Takes
effect for reactors spawned/restarted AFTER the call — the server sets
this once at startup, before any packages load.

<details>
<summary>Source</summary>

```rust
    pub async fn set_graph_executor(
        &self,
        executor: Arc<dyn super::graph_executor::GraphExecutor>,
    ) {
        *self.graph_executor.write().await = executor;
    }
```

</details>

##### <span id="ComputationGraphScheduler.load_reactor"></span><code>load_​reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_reactor(
    &self,
    reactor_name: String,
    accumulators: Vec<AccumulatorDeclaration>,
    criteria: ReactionCriteria,
    strategy: InputStrategy,
    tenant_id: Option<String>,
    register_aliases: Vec<String>,
    constructor: Option<cloacina_computation_graph::ReactorConstructorRef>,
) -> Result<(), String>
```

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.load_reactor_in"></span><code>load_​reactor_​in</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_reactor_in(
    &self,
    reactor_name: String,
    accumulators: Vec<AccumulatorDeclaration>,
    criteria: ReactionCriteria,
    strategy: InputStrategy,
    tenant_id: Option<String>,
    register_aliases: Vec<String>,
    constructor: Option<cloacina_computation_graph::ReactorConstructorRef>,
    provider_root: Option<&std::path::Path>,
) -> Result<(), String>
```

`load_reactor` resolving the reactor's constructor ref against an EXPLICIT provider tree (CLOACI-T-0925): `provider_root` is where the owning package's bundled providers were staged. `None` keeps the ambient behavior for embedded/test callers.

<details>
<summary>Source</summary>

```rust
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

        // CLOACI-T-0851: claim cross-replica ownership BEFORE spawning anything.
        //
        // Placed here, alongside the other "resolve what can fail before we
        // spawn" work, for the same reason: losing the claim after the reactor
        // and its accumulators are running would mean tearing down a live
        // reactor, and a partially-wired teardown is how endpoints get left
        // registered for something that is no longer running.
        //
        // `ownership == None` — embedded, sqlite, single replica — skips this
        // entirely and the path below is byte-for-byte what it was.
        //
        // NOT acquiring the lock is a normal outcome, not an error: another
        // replica owns this reactor. The load still SUCCEEDS, because the
        // package is legitimately present on this replica; it simply does not
        // run the reactor here. Returning an error instead would make a
        // correctly-functioning multi-replica deployment look like a failed
        // deployment on every replica but one.
        if let Some(ownership) = self.ownership.as_ref() {
            let id = super::reactor_ownership::ReactorId::from(&reactor_key);
            match ownership.claim(&id).await {
                Ok(true) => {
                    tracing::info!(reactor = %reactor_key, "reactor ownership claimed");
                }
                Ok(false) => {
                    tracing::info!(
                        reactor = %reactor_key,
                        "reactor owned by another replica; not starting it here"
                    );
                    self.foreign_reactors
                        .write()
                        .await
                        .insert(reactor_key.clone());
                    // Record which FOREIGN reactor each of this graph's
                    // accumulators belongs to. The inject edge needs this: an
                    // inject names an ACCUMULATOR, but owner addresses are
                    // published per REACTOR, and only the declaration — which
                    // we have right here and the routes never see — links the
                    // two. Without this map a non-owner cannot compute where to
                    // redirect, and every miss would fall back to the outbox.
                    {
                        let mut fa = self.foreign_accumulators.write().await;
                        for acc in &accumulators {
                            fa.insert(
                                TenantKey::new(tenant_id.as_deref(), &acc.name),
                                reactor_key.clone(),
                            );
                        }
                    }
                    // Stash everything needed to COMPLETE this load later.
                    // Losing a claim is not the end of the story: when the
                    // owner dies its lock auto-releases, and the watchdog tick
                    // attempts takeover (`try_takeover_foreign_reactors`).
                    // Without this stash a loser could never become the owner
                    // — observed live in the k8s lane: killing the owner left
                    // the reactor unclaimed forever, because nothing on the
                    // surviving replica ever retried.
                    self.foreign_pending.write().await.insert(
                        reactor_key.clone(),
                        PendingForeignLoad {
                            reactor_name,
                            accumulators,
                            criteria,
                            strategy,
                            tenant_id,
                            register_aliases,
                            constructor,
                            provider_root,
                            pending_binds: Vec::new(),
                        },
                    );
                    return Ok(());
                }
                Err(e) => {
                    // Fail the load rather than starting an unowned reactor. If
                    // we cannot reach Postgres to claim, we equally cannot know
                    // that nobody else holds it, and starting anyway is the
                    // split-brain this mechanism exists to prevent.
                    return Err(format!(
                        "reactor '{reactor_key}': could not determine ownership: {e}"
                    ));
                }
            }
        }

        self.spawn_reactor_claimed(
            reactor_name,
            accumulators,
            criteria,
            strategy,
            tenant_id,
            register_aliases,
            constructor,
            provider_root,
        )
        .await
    }
```

</details>

##### <span id="ComputationGraphScheduler.bind_graph_to_reactor"></span><code>bind_​graph_​to_​reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn bind_graph_to_reactor(
    &self,
    graph_name: String,
    reactor_name: String,
    scope: TenantScope<'_>,
    graph_fn: CompiledGraphFn,
) -> Result<(), String>
```

Bind a graph as an additional subscriber on an already-loaded reactor.

The reactor must have been loaded first (via [`load_reactor`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.load_reactor) or
transitively via [`load_graph`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.load_graph)); this entry point doesn't spawn
reactors. Returns an error if the reactor isn't loaded or if a graph
with the same name is already bound somewhere.

CLOACI-T-0924: `scope` is the *binding tenant*. The graph is claimed
under `scope`'s own key, while the upstream reactor is **resolved**
within `scope` — own tenant first, then the untenanted (embedded /
pre-multi-tenancy) reactor. A tenant can therefore subscribe to an
untenanted upstream, but never to another tenant's.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.load_graph"></span><code>load_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_graph(
    &self) -> Result<(,
    decl: ComputationGraphDeclaration) -> Result<(,
) -> Result<(), String>
```

Load and start a computation graph.

After T-0545 M1 this is a thin wrapper over [`load_reactor`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.load_reactor) +
[`bind_graph_to_reactor`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.bind_graph_to_reactor). It exists so today's bundled-form callers
(every existing test, every package built before reactor-only
packages) keep their contract: one call resolves both the reactor's
lifecycle and the graph's subscription. Independent-reactor consumers
(the reconciler post-T-0545) call the explicit pair directly.

<details>
<summary>Source</summary>

```rust
    pub async fn load_graph(&self, decl: ComputationGraphDeclaration) -> Result<(), String> {
        self.load_graph_in(decl, None).await
    }
```

</details>

##### <span id="ComputationGraphScheduler.load_graph_in"></span><code>load_​graph_​in</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_graph_in(
    &self,
    decl: ComputationGraphDeclaration,
    provider_root: Option<&std::path::Path>,
) -> Result<(), String>
```

`load_graph` resolving the declaration's reactor constructor against an EXPLICIT provider tree (CLOACI-T-0925) — the directory the reconciler staged for the package that declared the graph.

<details>
<summary>Source</summary>

```rust
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

        // CLOACI-T-0851: if another replica owns this reactor, nothing was
        // started here, so there is nothing to bind to. Binding would fail with
        // "reactor not loaded" — technically true, but it would report a
        // correctly-functioning multi-replica deployment as a broken load on
        // every replica except the owner.
        {
            let key = TenantKey::new(decl.tenant_id.as_deref(), &reactor_name);
            if self.foreign_reactors.read().await.contains(&key) {
                tracing::debug!(
                    reactor = %key,
                    graph = %name,
                    "graph loaded but its reactor is owned by another replica; not binding here"
                );
                // Stash the bind for takeover: if this replica later claims
                // the reactor, it must replay this subscription or the taken-
                // over reactor fires into nothing (`CompiledGraphFn` is an
                // Arc, so storing it is a refcount bump, not a copy).
                if let Some(pending) = self.foreign_pending.write().await.get_mut(&key) {
                    pending
                        .pending_binds
                        .push((name.to_string(), decl.reactor.graph_fn));
                }
                return Ok(());
            }
        }

        self.bind_graph_to_reactor(name, reactor_name, scope, decl.reactor.graph_fn)
            .await
    }
```

</details>

##### <span id="ComputationGraphScheduler.load_graph_split"></span><code>load_​graph_​split</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_graph_split(
    &self,
    graph_name: String,
    graph_fn: CompiledGraphFn,
    reactor: &cloacina_computation_graph::ReactorRegistration,
    accumulators: Vec<AccumulatorDeclaration>,
    tenant_id: Option<String>,
) -> Result<(), String>
```

Load a computation graph that references a reactor declaration by value (split form, from `#[computation_graph(trigger = reactor(T))]`).

This spawns a fresh reactor instance tied to this graph, using the
criteria + accumulator list carried by `reactor`, and binds `graph_fn`
as the firing callback.

**Test-only convenience API.** Production reconciler code does NOT
call this — the `RegistryReconciler` calls `load_reactor` followed
by `bind_graph_to_reactor` directly so the reactor identity is
explicit at every step. This helper exists for integration tests in
`crates/cloacina/tests/integration/computation_graph.rs` that exercise
the split-form lifecycle. (T-0556 audit confirmed zero non-test
callers.)

`input_strategy` defaults to [`InputStrategy::Latest`](../../../../rust/cloacina/computation_graph/reactor/#enum.InputStrategy).

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.unbind_graph_from_reactor"></span><code>unbind_​graph_​from_​reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unbind_graph_from_reactor(
    &self,
    name: &str,
    scope: TenantScope<'_>,
) -> Result<TenantKey, String>
```

Unbind a graph from its reactor without affecting the reactor itself.

The graph stops being a subscriber but the reactor (and its
accumulators) keeps running, ready for new subscribers. This is the
honest lifecycle primitive — reactors are independent units; binding
and unbinding subscribers is decoupled from reactor teardown.

CLOACI-T-0924: `name` is resolved within `scope` (own tenant, then the
untenanted entry), so a caller can only unbind a graph it can see.
Returns the full [`TenantKey`](../../../../rust/cloacina/tenant_scope/#struct.TenantKey) of the reactor the graph was bound to.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.unload_reactor"></span><code>unload_​reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unload_reactor(
    &self,
    reactor_name: &str,
    scope: TenantScope<'_>,
) -> Result<(), String>
```

Tear down a reactor and its accumulators. Rejects if the reactor has any bound subscribers — operators must unbind subscribers first. This is the lifecycle guard that makes "reactors as independent units" safe: a reactor never disappears out from under a graph that's still declaring it as an upstream.

CLOACI-T-0924: `reactor_name` is resolved within `scope`. A tenant can
only tear down its own reactor (or an untenanted one it can already
address); another tenant's same-named reactor is simply "not loaded".

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.set_ownership"></span><code>set_​ownership</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_ownership(
    &mut self,
    ownership: Arc<dyn super::reactor_ownership::ReactorOwnership>,
)
```

Install cross-replica reactor ownership (CLOACI-T-0851).

Only `cloacina-server` under multi-replica postgres should call this.
Leaving it unset keeps embedded/sqlite/single-replica behaviour exactly
as it was.

<details>
<summary>Source</summary>

```rust
    pub fn set_ownership(
        &mut self,
        ownership: Arc<dyn super::reactor_ownership::ReactorOwnership>,
    ) {
        self.ownership = Some(ownership);
    }
```

</details>

##### <span id="ComputationGraphScheduler.has_ownership_coordination"></span><code>has_​ownership_​coordination</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn has_ownership_coordination(&self) -> bool
```

Whether cross-replica ownership is in force.

<details>
<summary>Source</summary>

```rust
    pub fn has_ownership_coordination(&self) -> bool {
        self.ownership.is_some()
    }
```

</details>

##### <span id="ComputationGraphScheduler.foreign_reactor_for_accumulator"></span><code>foreign_​reactor_​for_​accumulator</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn foreign_reactor_for_accumulator(
    &self,
    tenant_id: Option<&str>,
    accumulator: &str,
) -> Option<crate::TenantKey>
```

If `accumulator` belongs to a reactor another replica owns, return that reactor's key so the caller can look up the owner's address and redirect. `None` means "not known to be foreign" — the accumulator is local, or nonexistent; the caller distinguishes those the way it always has.

<details>
<summary>Source</summary>

```rust
    pub async fn foreign_reactor_for_accumulator(
        &self,
        tenant_id: Option<&str>,
        accumulator: &str,
    ) -> Option<crate::TenantKey> {
        let fa = self.foreign_accumulators.read().await;
        if let Some(hit) = fa.get(&TenantKey::new(tenant_id, accumulator)) {
            return Some(hit.clone());
        }
        // Untenanted caller (admin/bootstrap keys carry no tenant): resolve by
        // name across tenants when UNIQUE, mirroring the endpoint registry's
        // Global-scope semantics. Found live on a real cluster: the bootstrap
        // key (tenant_id=None) injected into an accumulator whose reactor was
        // loaded under Some("public") — the exact-key lookup missed, so the
        // redirect never happened and an admin inject at a non-owner 404'd
        // while the same inject at the owner returned 200. Ambiguity stays
        // None: silently picking one tenant's reactor would misroute another's
        // events, which is worse than the outbox fallback.
        if tenant_id.is_none() {
            let mut matches = fa.iter().filter(|(k, _)| k.name == accumulator);
            if let Some((_, reactor_key)) = matches.next() {
                if matches.next().is_none() {
                    return Some(reactor_key.clone());
                }
            }
        }
        None
    }
```

</details>

##### <span id="ComputationGraphScheduler.ownership_watchdog_tick"></span><code>ownership_​watchdog_​tick</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn ownership_watchdog_tick(
    &self,
    watchdog: &mut super::reactor_ownership::OwnershipWatchdog,
) -> Vec<super::reactor_ownership::ReactorId>
```

One watchdog tick: verify what we believe we own, and act on the verdict.

Returns the reactors halted (empty when all is well, or when ownership
coordination is not installed). Callers drive this on a timer; it is a
single tick rather than a loop so the cadence, and the test, stay in the
caller's hands.

See [`ADR CLOACI-A-0012`] Amendment 1 for why a long-held lock needs
verifying at all, and `OwnershipWatchdog` for why sustained inability to
verify is treated as loss rather than as health.

<details>
<summary>Source</summary>

```rust
    pub async fn ownership_watchdog_tick(
        &self,
        watchdog: &mut super::reactor_ownership::OwnershipWatchdog,
    ) -> Vec<super::reactor_ownership::ReactorId> {
        use super::reactor_ownership::WatchdogAction;

        let Some(ownership) = self.ownership.as_ref() else {
            return Vec::new();
        };

        let check = ownership.verify().await;
        let believed = ownership.believed_owned().await;

        let halted = match watchdog.observe(check, &believed) {
            WatchdogAction::Continue => Vec::new(),
            WatchdogAction::StopReactors(lost) => {
                tracing::warn!(
                    count = lost.len(),
                    "ownership lost for reactors; halting them"
                );
                self.halt_unowned_reactors(&lost).await
            }
            WatchdogAction::StopAllPresumedLost(all) => {
                tracing::error!(
                    count = all.len(),
                    "ownership UNVERIFIABLE for too long — presuming loss and halting every \
                     reactor this replica believed it owned"
                );
                self.halt_unowned_reactors(&all).await
            }
        };

        // TAKEOVER: the other half of failover. The halt path above handles
        // "we lost a reactor we were running"; this handles "someone else's
        // reactor is now claimable". A dead owner's session drops and Postgres
        // releases its locks — but nothing else ever retries the claim, so
        // without this pass a reactor whose owner died stays unclaimed FOREVER
        // (observed live: kill the owner, and no survivor claimed within 90s,
        // because no survivor ever tried).
        self.try_takeover_foreign_reactors().await;

        halted
    }
```

</details>

##### <span id="ComputationGraphScheduler.halt_unowned_reactors"></span><code>halt_​unowned_​reactors</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn halt_unowned_reactors(
    &self,
    lost: &[super::reactor_ownership::ReactorId],
) -> Vec<super::reactor_ownership::ReactorId>
```

Stop reactors this replica has lost ownership of (CLOACI-T-0851 / [`ADR CLOACI-A-0012`] Amendment 1).

Returns the reactors actually stopped — a subset, because a reactor may
already have been unloaded between the liveness check and this call.

**Deliberately bypasses `unload_reactor`'s subscriber guard.** That guard
exists so a reactor never disappears under a graph still declaring it
upstream, which is right for an operator-initiated unload. It is wrong
here: having lost the lock, another replica may already be running this
reactor, and refusing to stop because a subscriber remains would leave
two live copies double-processing the same stream. A stopped reactor is
recoverable by re-claiming; silent double-processing is not.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.unload_graph"></span><code>unload_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unload_graph(
    &self) -> Result<(,
    name: &str) -> Result<(,
    scope: TenantScope<'_>) -> Result<(,
) -> Result<(), String>
```

Backward-compat convenience: unbind the graph from its reactor and, if it was the last subscriber, also tear down the reactor. This preserves today's 1:1 reactor-per-graph callers (a single `unload_graph(name)` removes everything the matching `load_graph` brought in). For independent reactor lifecycles, prefer [`unbind_graph_from_reactor`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.unbind_graph_from_reactor) + explicit [`unload_reactor`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.unload_reactor).

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.reactor_accumulator_names"></span><code>reactor_​accumulator_​names</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn reactor_accumulator_names(
    &self,
    reactor_name: &str,
    scope: TenantScope<'_>,
) -> Option<Vec<String>>
```

Snapshot the accumulator names of a loaded reactor, in declaration order. Returns `None` if the reactor isn't loaded. Used by the reconciler to pre-validate cross-package subscriber bindings against the upstream reactor's contract before calling [`load_graph`](../../../../rust/cloacina/computation_graph/scheduler/#ComputationGraphScheduler.load_graph).

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.list_graphs"></span><code>list_​graphs</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_graphs(&self) -> Vec<GraphStatus>
```

List all loaded computation graphs with status. Emits one entry per graph; multiple graphs sharing a reactor each get a status reflecting the same reactor's running state.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.list_reactors"></span><code>list_​reactors</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_reactors(&self) -> Vec<ReactorStatus>
```

List all loaded reactors with status (CLOACI-T-0742). Reactor-first: one entry per reactor in the `reactors` map, **including reactors with no graph bound** (which `list_graphs` omits, since it iterates `graph_to_reactor`). `bound_graphs` is the reverse lookup over that map.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.check_and_restart_failed"></span><code>check_​and_​restart_​failed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn check_and_restart_failed(&self) -> usize
```

Check all graphs for crashed tasks and restart them.

Individual accumulators are restarted in-place without tearing down the
reactor. Reactor crashes trigger a full-graph restart. Failure counting
with exponential backoff prevents infinite restart loops.

Runs in three phases (CLOACI-T-0915): crash detection and
failure-count bookkeeping happen under the reactors write lock; the
recovery-event write and the exponential-backoff sleep happen with
the lock RELEASED so list/health readers (`list_graphs`,
`/v1/health/graphs`, package loads) stay responsive during a restart
storm; each restart then re-acquires the lock and re-validates that
the component is still down before acting.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.start_supervision"></span><code>start_​supervision</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn start_supervision(
    self: &Arc<Self>,
    mut shutdown_rx: watch::Receiver<bool>,
    check_interval: std::time::Duration,
) -> JoinHandle<()>
```

Start a background supervision loop that checks for crashed tasks.

Returns a `JoinHandle` for the supervision task.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.emit_health_metrics"></span><code>emit_​health_​metrics</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn emit_health_metrics(&self)
```

Walk every loaded graph and emit the current `cloacina_component_health` gauge for its reactor and accumulators.

Health values are derived from the existing watch channels
(`ReactorHealth`, `AccumulatorHealth`) projected onto the bounded
`state` label vocabulary via `as_state_label()`. Called once per
supervision tick so the gauge tracks the state machine without
requiring an event-driven emitter wired into every health-write
site.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="ComputationGraphScheduler.shutdown_all"></span><code>shutdown_​all</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn shutdown_all(&self)
```

Graceful shutdown of all graphs.

<details>
<summary>Source</summary>

```rust
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
```

</details>

## Traits

### <span id="trait.AccumulatorFactory"></span><code style="font-size: 0.85em">trait AccumulatorFactory</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Factory trait for creating accumulator instances.

We can't clone trait objects, so we use a factory that produces them.

#### Methods

##### <span id="AccumulatorFactory.spawn"></span><code>spawn</code>

```rust
fn spawn(
    &self,
    name: String,
    boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
    shutdown_rx: watch::Receiver<bool>,
    config: AccumulatorSpawnConfig,
) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>)
```

Create a new accumulator instance and its runtime components.

Returns:
- socket_tx: sender for the accumulator's socket channel
- join_handle: spawned task handle

<details>
<summary>Source</summary>

```rust
    fn spawn(
```

</details>
