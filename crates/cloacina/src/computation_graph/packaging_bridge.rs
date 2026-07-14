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

//! Bridge from FFI-loaded package metadata to ComputationGraphScheduler types.
//!
//! Converts `GraphPackageMetadata` + library data into `ComputationGraphDeclaration`
//! with `AccumulatorFactory` implementations and a `CompiledGraphFn` that calls
//! `execute_graph()` via fidius FFI.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use cloacina_workflow_plugin::{GraphExecutionRequest, GraphPackageMetadata};

use super::accumulator::{
    accumulator_runtime, accumulator_runtime_with_source, batch_accumulator_runtime, flush_signal,
    state_accumulator_runtime, AccumulatorContext, AccumulatorRuntimeConfig, BatchAccumulator,
    BatchAccumulatorConfig, BoundarySender, StateAccumulator,
};
use super::reactor::{CompiledGraphFn, InputStrategy, ReactionCriteria};
use super::scheduler::{
    AccumulatorDeclaration, AccumulatorFactory, AccumulatorSpawnConfig,
    ComputationGraphDeclaration, ReactorDeclaration,
};
use super::types::{GraphError, GraphResult, InputCache, SourceName};

/// A persistent handle to a loaded FFI graph plugin.
///
/// Loaded once from library bytes, kept alive for the lifetime of the graph.
/// The `PluginHandle` is behind a `Mutex` because fidius calls are synchronous
/// and must not be invoked concurrently.
pub struct LoadedGraphPlugin {
    handle: std::sync::Mutex<fidius_host::PluginHandle>,
    // Keep the temp dir alive so the dylib file isn't deleted while loaded
    _temp_dir: tempfile::TempDir,
}

// Safety: fidius PluginHandle wraps a libloading::Library which is Send.
// We serialize access via Mutex so concurrent calls are safe.
unsafe impl Send for LoadedGraphPlugin {}
unsafe impl Sync for LoadedGraphPlugin {}

impl LoadedGraphPlugin {
    /// Load a graph plugin from library bytes. The library is written to a temp
    /// file, loaded via fidius, and kept resident for reuse. Public so the
    /// execution agent can run whole-graph firings (CLOACI-T-0722).
    pub fn load(library_data: &[u8]) -> Result<Self, String> {
        let temp_dir =
            tempfile::TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let library_extension = if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        let temp_path = temp_dir
            .path()
            .join(format!("graph_plugin.{}", library_extension));
        std::fs::write(&temp_path, library_data)
            .map_err(|e| format!("Failed to write library: {}", e))?;

        let loaded = fidius_host::loader::load_library(&temp_path)
            .map_err(|e| format!("Failed to load library: {}", e))?;

        let plugin = loaded
            .plugins
            .into_iter()
            .next()
            .ok_or_else(|| "No plugins in library".to_string())?;

        let handle = fidius_host::PluginHandle::from_loaded(plugin);

        Ok(Self {
            handle: std::sync::Mutex::new(handle),
            _temp_dir: temp_dir,
        })
    }

    /// Call execute_graph on the loaded plugin.
    pub fn execute_graph(
        &self,
        request: GraphExecutionRequest,
    ) -> Result<cloacina_workflow_plugin::GraphExecutionResult, String> {
        let handle = self
            .handle
            .lock()
            .map_err(|e| format!("Plugin mutex poisoned: {}", e))?;
        handle
            .call_method(METHOD_EXECUTE_GRAPH, &(request,))
            .map_err(|e| format!("execute_graph FFI call failed: {}", e))
    }
}

/// Method index constants for the version-2 `CloacinaPlugin` trait. The
/// canonical definitions live alongside the trait in
/// `cloacina-workflow-plugin`; we re-export them here so existing
/// `crate::computation_graph::packaging_bridge::METHOD_*` consumers don't
/// have to change their import paths.
pub use cloacina_workflow_plugin::{
    METHOD_EXECUTE_GRAPH, METHOD_EXECUTE_TASK, METHOD_GET_CONSTRUCTOR_METADATA,
    METHOD_GET_GRAPH_METADATA, METHOD_GET_REACTOR_METADATA, METHOD_GET_TASK_METADATA,
    METHOD_GET_TRIGGERLESS_GRAPH_METADATA, METHOD_GET_TRIGGER_METADATA,
    METHOD_INVOKE_TRIGGERLESS_GRAPH, METHOD_INVOKE_TRIGGER_POLL,
};

/// Call `get_reactor_metadata` (method index 4) on a loaded fidius plugin.
///
/// I-0102 / T-B: this is the host-side bridge that consumes the unified
/// `cloacina::package!()` shell's reactor metadata. Plugins built before
/// trait v2 (or per-macro `_ffi` blocks emitting empty stubs) return either
/// `CallError::NotImplemented { bit }` or `Ok(vec![])` — both translate to
/// "package declares no reactors" and the reconciler skips the reactor
/// dispatch step for that package.
pub fn call_get_reactor_metadata(
    handle: &fidius_host::PluginHandle,
) -> Result<Vec<cloacina_workflow_plugin::ReactorPackageMetadata>, String> {
    match handle.call_method::<(), Vec<cloacina_workflow_plugin::ReactorPackageMetadata>>(
        METHOD_GET_REACTOR_METADATA,
        &(),
    ) {
        Ok(metadata) => Ok(metadata),
        Err(fidius_host::CallError::NotImplemented { .. }) => Ok(Vec::new()),
        Err(e) => Err(format!("get_reactor_metadata FFI call failed: {}", e)),
    }
}

/// Call `get_constructor_metadata` (method index 10) on a loaded fidius plugin
/// (CLOACI-T-0832). Returns the packaged workflow's declared `constructor!(...)`
/// nodes for the host to resolve + inject. Plugins built before trait v4 return
/// `CallError::NotImplemented` → `Ok(vec![])` ("package declares no constructor
/// nodes"), so older packages keep loading unchanged.
pub fn call_get_constructor_metadata(
    handle: &fidius_host::PluginHandle,
) -> Result<Vec<cloacina_workflow_plugin::ConstructorPackageMetadata>, String> {
    match handle.call_method::<(), Vec<cloacina_workflow_plugin::ConstructorPackageMetadata>>(
        METHOD_GET_CONSTRUCTOR_METADATA,
        &(),
    ) {
        Ok(metadata) => Ok(metadata),
        Err(fidius_host::CallError::NotImplemented { .. }) => Ok(Vec::new()),
        Err(e) => Err(format!("get_constructor_metadata FFI call failed: {}", e)),
    }
}

/// Call `get_trigger_metadata` (method index 5) on a loaded fidius plugin.
///
/// I-0102 / T-B: same NotImplemented fallback as `call_get_reactor_metadata`.
/// The reconciler routes cron-shaped entries (cron_expression present) to the
/// cron scheduler and the rest to the runtime trigger registry.
pub fn call_get_trigger_metadata(
    handle: &fidius_host::PluginHandle,
) -> Result<Vec<cloacina_workflow_plugin::TriggerPackageMetadata>, String> {
    match handle.call_method::<(), Vec<cloacina_workflow_plugin::TriggerPackageMetadata>>(
        METHOD_GET_TRIGGER_METADATA,
        &(),
    ) {
        Ok(metadata) => Ok(metadata),
        Err(fidius_host::CallError::NotImplemented { .. }) => Ok(Vec::new()),
        Err(e) => Err(format!("get_trigger_metadata FFI call failed: {}", e)),
    }
}

/// Convert FFI graph metadata + library data into a `ComputationGraphDeclaration`
/// that the `ComputationGraphScheduler` can load.
///
/// The library is loaded once here and the handle is kept alive in the
/// `CompiledGraphFn` closure for reuse on every reactor fire.
pub fn build_declaration_from_ffi(
    graph_meta: &GraphPackageMetadata,
    library_data: Vec<u8>,
) -> ComputationGraphDeclaration {
    let criteria = match graph_meta.reaction_mode.as_str() {
        "when_all" => ReactionCriteria::WhenAll,
        _ => ReactionCriteria::WhenAny,
    };

    let strategy = match graph_meta.input_strategy.as_str() {
        "sequential" => InputStrategy::Sequential,
        _ => InputStrategy::Latest,
    };

    // Load the library once and keep the handle for reuse.
    // If loading fails (e.g., in tests with fake data), the graph function
    // returns an error on every call instead of panicking at construction.
    let graph_fn: CompiledGraphFn = match LoadedGraphPlugin::load(&library_data) {
        Ok(plugin) => {
            let plugin = Arc::new(plugin);
            Arc::new(move |cache: InputCache| {
                let plugin = plugin.clone();
                Box::pin(async move { execute_graph_via_ffi(&plugin, &cache).await })
            })
        }
        Err(e) => {
            let error_msg = format!("Graph plugin library failed to load: {}", e);
            tracing::warn!("{}", error_msg);
            Arc::new(move |_cache: InputCache| {
                let msg = error_msg.clone();
                Box::pin(async move { GraphResult::error(GraphError::Execution(msg)) })
            })
        }
    };

    // Create accumulator factories from FFI metadata
    let accumulators = graph_meta
        .accumulators
        .iter()
        .map(|acc_entry| {
            let factory = accumulator_factory_for(&acc_entry.accumulator_type, &acc_entry.config);
            AccumulatorDeclaration {
                name: acc_entry.name.clone(),
                factory,
            }
        })
        .collect();

    ComputationGraphDeclaration {
        name: graph_meta.graph_name.clone(),
        accumulators,
        reactor: ReactorDeclaration {
            criteria,
            strategy,
            graph_fn,
            // CLOACI-T-0830: the FFI/cdylib packaged path doesn't yet carry a
            // reactor-constructor reference through `GraphPackageMetadata`
            // (deferred — see `dispatch_package_reactors_into_scheduler`). Native
            // dirty-flag firing only for this path.
            constructor: None,
        },
        tenant_id: None, // Set by the reconciler based on package ownership
        // Propagate the explicit reactor name from the FFI metadata
        // (T-0544 M5). `Some(name)` opts the graph into shared-reactor
        // binding — packages built from `#[computation_graph(trigger =
        // reactor(R))]` now plumb R's name all the way to the scheduler,
        // so two packages naming the same reactor share one runtime
        // instance via M2's idempotent path. `None` (today's bundled-form
        // default and pre-M5 packages via `#[serde(default)]`) keeps the
        // synthesized per-graph reactor name and 1:1 lifecycle.
        reactor_name: graph_meta.trigger_reactor.clone(),
        topology: graph_meta.graph_data_json.clone(),
    }
}

/// Execute a computation graph via FFI using the pre-loaded plugin handle.
/// Convert an [`InputCache`] snapshot into the FFI/wire cache shape
/// (source name → UTF-8 JSON string) — the same conversion the in-process FFI
/// call performs, shared with the fleet path so a dispatched firing carries an
/// agent-ready cache (CLOACI-T-0722). Boundary frames are `bincode(Vec<u8>)`
/// of raw event JSON; non-UTF-8 payloads are hex-encoded.
pub fn input_cache_to_ffi_cache(cache: &InputCache) -> Result<HashMap<String, String>, String> {
    let cache_snapshot = cache.snapshot();
    let mut ffi_cache: HashMap<String, String> = HashMap::new();
    for source_name in cache_snapshot.sources() {
        if let Some(raw_bytes) = cache_snapshot.get_raw(source_name.as_str()) {
            match bincode::deserialize::<Vec<u8>>(raw_bytes) {
                Ok(original_bytes) => {
                    let json_str = String::from_utf8(original_bytes).unwrap_or_else(|e| {
                        tracing::warn!(
                            source = source_name.as_str(),
                            "cache entry is not valid UTF-8, hex-encoding: {}",
                            e
                        );
                        raw_bytes.iter().map(|b| format!("{:02x}", b)).collect()
                    });
                    ffi_cache.insert(source_name.as_str().to_string(), json_str);
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to deserialize cache entry '{}' for FFI: {}",
                        source_name.as_str(),
                        e
                    ));
                }
            }
        }
    }
    Ok(ffi_cache)
}

async fn execute_graph_via_ffi(plugin: &Arc<LoadedGraphPlugin>, cache: &InputCache) -> GraphResult {
    let ffi_cache = match input_cache_to_ffi_cache(cache) {
        Ok(c) => c,
        Err(e) => return GraphResult::error(GraphError::Serialization(e)),
    };

    let request = GraphExecutionRequest { cache: ffi_cache };

    // FFI call is synchronous — run in a blocking task
    let plugin = plugin.clone();
    let result = tokio::task::spawn_blocking(move || plugin.execute_graph(request)).await;

    match result {
        Ok(Ok(ffi_result)) => {
            if ffi_result.success {
                // CLOACI-T-0775: keep the terminal outputs as JSON (for the
                // per-fire output history) in addition to the type-erased boxes.
                let outputs_json: Vec<serde_json::Value> = ffi_result
                    .terminal_outputs_json
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|json_str| {
                        serde_json::from_str::<serde_json::Value>(&json_str).ok()
                    })
                    .collect();
                let outputs: Vec<Box<dyn std::any::Any + Send>> = outputs_json
                    .iter()
                    .cloned()
                    .map(|v| Box::new(v) as Box<dyn std::any::Any + Send>)
                    .collect();
                GraphResult::completed_with_json(outputs, outputs_json)
            } else {
                let error_msg = ffi_result
                    .error
                    .unwrap_or_else(|| "unknown FFI execution error".to_string());
                GraphResult::error(GraphError::NodeExecution(error_msg))
            }
        }
        Ok(Err(e)) => GraphResult::error(GraphError::NodeExecution(format!(
            "FFI execute_graph call failed: {}",
            e
        ))),
        Err(join_err) => GraphResult::error(GraphError::NodeExecution(format!(
            "FFI execute_graph panicked: {}",
            join_err
        ))),
    }
}

/// A generic passthrough accumulator factory for FFI-loaded packages.
///
/// All packaged accumulators are passthrough at the host level — they receive
/// serialized events via WebSocket/socket and forward them to the reactor.
/// The actual processing logic lives inside the FFI plugin's `execute_graph()`.
pub struct PassthroughAccumulatorFactory;

struct GenericPassthroughAccumulator;

#[async_trait::async_trait]
impl super::Accumulator for GenericPassthroughAccumulator {
    type Output = Vec<u8>;

    fn process(&mut self, event: Vec<u8>) -> Option<Vec<u8>> {
        Some(event)
    }
}

impl AccumulatorFactory for PassthroughAccumulatorFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        let (socket_tx, socket_rx) = mpsc::channel(64);

        let checkpoint = config.dal.map(|dal| {
            super::accumulator::CheckpointHandle::new(dal, config.graph_name.clone(), name.clone())
        });

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
            GenericPassthroughAccumulator,
            ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
        ));

        (socket_tx, handle)
    }
}

/// A stream-backed accumulator factory for FFI-loaded packages.
///
/// Creates a passthrough accumulator with a `KafkaEventSource` that pulls raw
/// bytes from a Kafka topic. The event source runs on its own task via
/// `accumulator_runtime_with_source`. The socket channel remains available for
/// out-of-band WebSocket pushes.
pub struct StreamBackendAccumulatorFactory {
    /// Stream backend config from the package metadata.
    config: std::collections::HashMap<String, String>,
}

impl StreamBackendAccumulatorFactory {
    pub fn new(config: std::collections::HashMap<String, String>) -> Self {
        Self { config }
    }
}

/// EventSource that reads raw bytes from a Kafka topic.
#[cfg(feature = "kafka")]
struct KafkaEventSource {
    broker_var: String,
    topic: String,
    group: String,
    extra: std::collections::HashMap<String, String>,
    name: String,
}

#[cfg(feature = "kafka")]
#[async_trait::async_trait]
impl super::accumulator::EventSource for KafkaEventSource {
    async fn run(
        self,
        events: mpsc::Sender<Vec<u8>>,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<(), super::accumulator::AccumulatorError> {
        // The broker config may be a `{{ VAR }}` template (e.g.
        // `{{ KAFKA_BROKER }}` → CLOACINA_VAR_KAFKA_BROKER) or a literal
        // (`kafka:9092`). `resolve_template` handles both — it strips the
        // braces/whitespace and resolves the var, and passes plain values
        // through unchanged. (Previously this passed the raw `{{ KAFKA_BROKER }}`
        // string to `var()`, so the Kafka accumulator never initialized —
        // CLOACI-I-0124 / WS-11.)
        let broker_url = crate::var::resolve_template(&self.broker_var).map_err(|missing| {
            let names = missing
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            super::accumulator::AccumulatorError::Init(format!(
                "cannot resolve broker var '{}': {}",
                self.broker_var, names
            ))
        })?;

        let stream_config = super::stream_backend::StreamConfig {
            broker_url,
            topic: self.topic,
            group: self.group,
            extra: self.extra,
        };

        use super::stream_backend::StreamBackend as _;
        let mut backend = super::stream_backend::kafka::KafkaStreamBackend::connect(&stream_config)
            .await
            .map_err(|e| {
                super::accumulator::AccumulatorError::Init(format!("Kafka connect failed: {}", e))
            })?;

        tracing::info!(accumulator = %self.name, "Kafka event source started");
        loop {
            tokio::select! {
                result = backend.recv() => {
                    match result {
                        Ok(msg) => {
                            tracing::debug!(
                                accumulator = %self.name,
                                offset = msg.offset,
                                bytes = msg.payload.len(),
                                "Kafka message received"
                            );
                            if events.send(msg.payload).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(accumulator = %self.name, "Kafka recv error: {}", e);
                        }
                    }
                }
                _ = shutdown.changed() => {
                    tracing::debug!(accumulator = %self.name, "Kafka event source shutting down");
                    break;
                }
            }
        }
        Ok(())
    }
}

impl AccumulatorFactory for StreamBackendAccumulatorFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        let (socket_tx, socket_rx) = mpsc::channel(1024);

        let checkpoint = config.dal.map(|dal| {
            super::accumulator::CheckpointHandle::new(dal, config.graph_name.clone(), name.clone())
        });

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

        let topic = self.config.get("topic").cloned().unwrap_or_default();
        let group = self
            .config
            .get("group")
            .cloned()
            .unwrap_or_else(|| format!("{}_group", name));
        let broker_var = self
            .config
            .get("broker")
            .cloned()
            .expect("stream accumulator config must include 'broker' key");
        let extra_config: std::collections::HashMap<String, String> = self
            .config
            .iter()
            .filter(|(k, _)| !["topic", "group", "backend", "broker"].contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        #[cfg(feature = "kafka")]
        let handle = {
            let source = KafkaEventSource {
                broker_var,
                topic,
                group,
                extra: extra_config,
                name: name.clone(),
            };
            tokio::spawn(accumulator_runtime_with_source(
                GenericPassthroughAccumulator,
                ctx,
                socket_rx,
                AccumulatorRuntimeConfig::default(),
                source,
            ))
        };

        #[cfg(not(feature = "kafka"))]
        let handle = {
            let _ = (topic, group, extra_config, broker_var);
            tracing::error!(accumulator = %name, "stream accumulator requires 'kafka' feature");
            tokio::spawn(accumulator_runtime(
                GenericPassthroughAccumulator,
                ctx,
                socket_rx,
                AccumulatorRuntimeConfig::default(),
            ))
        };

        (socket_tx, handle)
    }
}

/// A state-backed accumulator factory for FFI-loaded / Python packages.
///
/// Spawns `state_accumulator_runtime::<serde_json::Value>` with a bounded
/// `VecDeque` of the given capacity. This is the host-side wiring for
/// `@cloaca.state_accumulator(capacity=N)` (and Rust's
/// `#[state_accumulator(capacity=…)]`): values pushed over the socket are
/// buffered, persisted to the DAL on every write, and the full list is emitted
/// back as the boundary so the graph can feed its own state on the next fire.
///
/// Capacity semantics (see `StateAccumulator`):
/// - `> 0`: bounded — evicts oldest when at capacity
/// - `< 0`: unbounded — grows without limit
/// - `0`:  write-only sink — no history emitted back
pub struct StateAccumulatorFactory {
    capacity: i32,
}

impl StateAccumulatorFactory {
    pub fn new(capacity: i32) -> Self {
        Self { capacity }
    }
}

impl AccumulatorFactory for StateAccumulatorFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        let (socket_tx, socket_rx) = mpsc::channel(1024);

        let checkpoint = config.dal.map(|dal| {
            super::accumulator::CheckpointHandle::new(dal, config.graph_name.clone(), name.clone())
        });

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

        let acc = StateAccumulator::<serde_json::Value>::new(self.capacity);
        let handle = tokio::spawn(state_accumulator_runtime(acc, ctx, socket_rx));

        (socket_tx, handle)
    }
}

/// Parse a state accumulator's capacity from its String-keyed config map.
/// Defaults to `0` (write-only sink) when absent or unparsable.
fn state_capacity_from_config(config: &std::collections::HashMap<String, String>) -> i32 {
    config
        .get("capacity")
        .and_then(|c| c.parse::<i32>().ok())
        .unwrap_or(0)
}

/// A generic, list-collecting batch accumulator for the packaged path
/// (CLOACI-T-0896). Socket events arrive as JSON bytes (the same wire the
/// passthrough/state accumulators receive); on flush we emit the whole batch as
/// a JSON array, so the boundary matches the shape the FFI cache expects
/// (`bincode(Vec<u8>)` of JSON — see `input_cache_to_ffi_cache`). This mirrors
/// what `state_window_frame` does for the state accumulator.
struct JsonListBatchAccumulator;

impl BatchAccumulator for JsonListBatchAccumulator {
    type Output = Vec<u8>;

    fn process_batch(&mut self, events: Vec<Vec<u8>>) -> Option<Vec<u8>> {
        let list: Vec<serde_json::Value> = events
            .iter()
            .filter_map(|e| serde_json::from_slice(e).ok())
            .collect();
        if list.is_empty() {
            return None;
        }
        serde_json::to_vec(&list).ok()
    }
}

/// Packaged batch-accumulator factory (CLOACI-T-0896): buffers socket events and
/// flushes the whole buffer as one boundary on the flush interval or when the
/// buffer fills. Mirrors `StateAccumulatorFactory` — socket-driven, so it fits
/// the existing spawn contract without any FFI change.
pub struct BatchAccumulatorFactory {
    flush_interval: Option<std::time::Duration>,
    max_buffer_size: Option<usize>,
}

impl BatchAccumulatorFactory {
    pub fn new(
        flush_interval: Option<std::time::Duration>,
        max_buffer_size: Option<usize>,
    ) -> Self {
        Self {
            flush_interval,
            max_buffer_size,
        }
    }
}

impl AccumulatorFactory for BatchAccumulatorFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        let (socket_tx, socket_rx) = mpsc::channel(1024);
        let (flush_tx, flush_rx) = flush_signal();

        let checkpoint = config.dal.map(|dal| {
            super::accumulator::CheckpointHandle::new(dal, config.graph_name.clone(), name.clone())
        });
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
        let batch_cfg = BatchAccumulatorConfig {
            flush_interval: self.flush_interval,
            max_buffer_size: self.max_buffer_size,
        };
        let handle = tokio::spawn(async move {
            // The packaged path has no external reactor-driven flusher, so
            // flushes come from the timer / size threshold. Hold `flush_tx` for
            // the runtime's lifetime so `flush_rx` stays open (its select arm
            // simply never fires) rather than closing and busy-spinning.
            let _flush_tx = flush_tx;
            batch_accumulator_runtime(
                JsonListBatchAccumulator,
                ctx,
                socket_rx,
                flush_rx,
                batch_cfg,
            )
            .await;
        });
        (socket_tx, handle)
    }
}

/// Parse a batch accumulator's `flush_interval` (e.g. `"1s"`, `"500ms"`) and
/// `max_buffer_size` from its String-keyed config map. Absent/unparsable →
/// `None` (the runtime treats each as an optional flush trigger).
fn batch_config_from_config(
    config: &std::collections::HashMap<String, String>,
) -> (Option<std::time::Duration>, Option<usize>) {
    let flush_interval = config
        .get("flush_interval")
        .and_then(|s| crate::packaging::manifest_schema::parse_duration_str(s).ok());
    let max_buffer_size = config
        .get("max_buffer_size")
        .and_then(|s| s.parse::<usize>().ok());
    (flush_interval, max_buffer_size)
}

/// Central accumulator-factory dispatch for the packaged path (CLOACI-T-0896).
/// Every packaged-reactor loader (Python runtime registration, Rust cdylib
/// metadata, manifest overrides) resolves the same set of kinds here, and an
/// unknown type WARNs loudly + falls back to passthrough instead of silently
/// degrading — the whole point of T-0896.
/// A poll closure for a packaged polling accumulator: invoked on each interval,
/// it returns `Some(json_bytes)` to emit a boundary or `None` to skip. The
/// closure does the blocking, GIL-taking work itself — cloacina runs it via
/// `spawn_blocking` off the async executor. The Python layer supplies these
/// (each wraps the registered Python poll fn); there is NO FFI accumulator-invoke
/// method, and none is needed — the poll fn lives in-process. (CLOACI-T-0896)
pub type PollClosure = Arc<dyn Fn() -> Option<Vec<u8>> + Send + Sync>;

/// Resolves the poll closure for a polling accumulator by name (returns `None`
/// when the name isn't registered). Installed once by the Python extension.
type PollingClosureBuilder = Box<dyn Fn(&str) -> Option<PollClosure> + Send + Sync>;

static POLLING_CLOSURE_BUILDER: std::sync::OnceLock<PollingClosureBuilder> =
    std::sync::OnceLock::new();

/// Register the polling-accumulator poll-closure resolver for the packaged path
/// (CLOACI-T-0896). Idempotent — the first registration wins. The Python
/// extension calls this at module install so a packaged polling accumulator
/// drives its Python poll fn on the configured interval. A pure-Rust host that
/// never installs one gets a loud passthrough fallback for polling accumulators.
pub fn register_polling_accumulator_builder(builder: PollingClosureBuilder) {
    let _ = POLLING_CLOSURE_BUILDER.set(builder);
}

/// A [`PollingAccumulator`] driven by an injected [`PollClosure`] (the Python
/// poll fn). `poll()` runs the closure on a blocking thread so the GIL work
/// never blocks the async executor — the same discipline `PythonTriggerWrapper`
/// uses for poll triggers. (CLOACI-T-0896)
struct ClosurePollingAccumulator {
    poll_fn: PollClosure,
    interval: std::time::Duration,
}

#[async_trait::async_trait]
impl super::accumulator::PollingAccumulator for ClosurePollingAccumulator {
    type Output = Vec<u8>;

    async fn poll(&mut self) -> Option<Vec<u8>> {
        let f = self.poll_fn.clone();
        tokio::task::spawn_blocking(move || f())
            .await
            .ok()
            .flatten()
    }

    fn interval(&self) -> std::time::Duration {
        self.interval
    }
}

/// Packaged polling-accumulator factory (CLOACI-T-0896). Resolves the poll
/// closure by name at spawn time via the registered builder, then runs
/// `polling_accumulator_runtime` on the configured interval. If no closure is
/// registered for the name, the accumulator simply never emits (logged) rather
/// than failing the load.
pub struct PollingAccumulatorFactory {
    interval: std::time::Duration,
}

impl PollingAccumulatorFactory {
    pub fn new(interval: std::time::Duration) -> Self {
        Self { interval }
    }
}

impl AccumulatorFactory for PollingAccumulatorFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        let (socket_tx, socket_rx) = mpsc::channel(1024);

        let poll_fn = POLLING_CLOSURE_BUILDER
            .get()
            .and_then(|builder| builder(&name));
        if poll_fn.is_none() {
            tracing::warn!(
                accumulator = %name,
                "no poll closure registered for polling accumulator — it will \
                 never emit (CLOACI-T-0896)"
            );
        }
        // No registered closure → a no-op poller that always returns None.
        let poll_fn: PollClosure = poll_fn.unwrap_or_else(|| Arc::new(|| None));

        let checkpoint = config.dal.map(|dal| {
            super::accumulator::CheckpointHandle::new(dal, config.graph_name.clone(), name.clone())
        });
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
        let poller = ClosurePollingAccumulator {
            poll_fn,
            interval: self.interval,
        };
        let handle = tokio::spawn(super::accumulator::polling_accumulator_runtime(
            poller, ctx, socket_rx,
        ));
        (socket_tx, handle)
    }
}

/// Parse a polling accumulator's `interval` (e.g. `"2s"`) from config; defaults
/// to 5s when absent/unparsable.
fn polling_interval_from_config(
    config: &std::collections::HashMap<String, String>,
) -> std::time::Duration {
    config
        .get("interval")
        .and_then(|s| crate::packaging::manifest_schema::parse_duration_str(s).ok())
        .unwrap_or_else(|| std::time::Duration::from_secs(5))
}

fn accumulator_factory_for(
    acc_type: &str,
    config: &std::collections::HashMap<String, String>,
) -> Arc<dyn AccumulatorFactory> {
    match acc_type {
        "stream" => Arc::new(StreamBackendAccumulatorFactory::new(config.clone())),
        "state" => Arc::new(StateAccumulatorFactory::new(state_capacity_from_config(
            config,
        ))),
        "batch" => {
            let (flush_interval, max_buffer_size) = batch_config_from_config(config);
            Arc::new(BatchAccumulatorFactory::new(
                flush_interval,
                max_buffer_size,
            ))
        }
        "polling" => Arc::new(PollingAccumulatorFactory::new(
            polling_interval_from_config(config),
        )),
        "passthrough" => Arc::new(PassthroughAccumulatorFactory),
        other => {
            tracing::warn!(
                accumulator_type = %other,
                "unknown accumulator type in packaged graph — falling back to \
                 passthrough (CLOACI-T-0896); firing will be per-event, not the \
                 declared behavior"
            );
            Arc::new(PassthroughAccumulatorFactory)
        }
    }
}

// ---------------------------------------------------------------------------
// T-0545 M3a: dispatch reactors registered in a Runtime into a scheduler
// ---------------------------------------------------------------------------

/// Dispatch every reactor registered in `runtime` into `scheduler` via
/// `scheduler.load_reactor`. Idempotent on `(reactor_name, contract)` —
/// callable repeatedly without spawning duplicate reactors.
///
/// This is the runtime-side glue that makes a reactor declaration in any
/// package "just work" without a co-located CG subscriber. The reconciler
/// drives this once per package load, after the language-specific loader
/// has populated the runtime's reactor registry. Accumulator factories
/// come from optional `package.toml`-style overrides (passthrough/stream)
/// with passthrough as the default.
///
/// Returns the names of reactors that were dispatched (newly loaded plus
/// idempotent re-loads). Errors short-circuit and surface to the caller —
/// package loading is fail-fast under the I-0101 lifecycle model.
pub async fn dispatch_runtime_reactors_into_scheduler(
    runtime: &crate::Runtime,
    scheduler: &super::scheduler::ComputationGraphScheduler,
    accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
    tenant_id: Option<String>,
) -> Result<Vec<String>, String> {
    let mut dispatched = Vec::new();
    for name in runtime.reactor_names() {
        let registration = match runtime.get_reactor(&name) {
            Some(r) => r,
            None => continue,
        };

        let accumulators: Vec<AccumulatorDeclaration> = registration
            .accumulator_names
            .iter()
            .map(|acc_name| {
                // CLOACI-T-0839 precedence: manifest override (deployment wins)
                // → authored spec carried on the registration → passthrough.
                // The authored-spec fallback closes the gap where a
                // runtime-registered reactor's state/stream accumulators
                // silently degraded to passthrough (this site only had names).
                let (acc_type, acc_config) = match accumulator_overrides
                    .iter()
                    .find(|cfg| &cfg.name == acc_name)
                {
                    Some(cfg) => (cfg.accumulator_type.clone(), cfg.config.clone()),
                    None => match registration
                        .accumulator_specs
                        .iter()
                        .find(|spec| &spec.name == acc_name)
                    {
                        Some(spec) => (spec.accumulator_type.clone(), spec.config.clone()),
                        None => ("passthrough".to_string(), Default::default()),
                    },
                };
                let factory = accumulator_factory_for(&acc_type, &acc_config);
                AccumulatorDeclaration {
                    name: acc_name.clone(),
                    factory,
                }
            })
            .collect();

        let criteria = registration.reaction_mode.into();
        let strategy = InputStrategy::Latest;

        scheduler
            .load_reactor(
                name.clone(),
                accumulators,
                criteria,
                strategy,
                tenant_id.clone(),
                vec![],
                // CLOACI-T-0830: carry the reactor-constructor reference from the
                // runtime registration (populated by `#[reactor(from=.., …)]`)
                // into the scheduler, which resolves + installs the WASM
                // `evaluate` as the reactor's firing decider.
                registration.constructor.clone(),
            )
            .await?;

        tracing::info!(reactor = %name, "package-declared reactor loaded into scheduler");
        dispatched.push(name);
    }
    Ok(dispatched)
}

/// Dispatch reactors declared by a packaged Rust cdylib (T-B / I-0102).
///
/// Consumes `Vec<ReactorPackageMetadata>` produced by the unified
/// `cloacina::package!()` shell's `get_reactor_metadata` and registers each
/// reactor with the `ComputationGraphScheduler`. Mirrors the shape of
/// `dispatch_runtime_reactors_into_scheduler` (which serves the Python
/// path) so the reconciler's reactor step looks identical between
/// languages.
///
/// `accumulator_overrides` is the manifest's `[metadata].accumulators`
/// table — kept as input until T-E removes manifest-side accumulator
/// overrides entirely. Today it shadows FFI-default `passthrough` with
/// `stream` configurations.
pub async fn dispatch_package_reactors_into_scheduler(
    reactor_metadata: &[cloacina_workflow_plugin::ReactorPackageMetadata],
    scheduler: &super::scheduler::ComputationGraphScheduler,
    accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
    tenant_id: Option<String>,
) -> Result<Vec<String>, String> {
    use cloacina_computation_graph::ReactionMode;

    let mut dispatched = Vec::new();
    for meta in reactor_metadata {
        let accumulators: Vec<AccumulatorDeclaration> = meta
            .accumulators
            .iter()
            .map(|acc| {
                let factory = match accumulator_overrides
                    .iter()
                    .find(|cfg| cfg.name == acc.name)
                {
                    Some(override_cfg) => accumulator_factory_for(
                        &override_cfg.accumulator_type,
                        &override_cfg.config,
                    ),
                    None => accumulator_factory_for(&acc.accumulator_type, &acc.config),
                };
                AccumulatorDeclaration {
                    name: acc.name.clone(),
                    factory,
                }
            })
            .collect();

        let criteria = match meta.reaction_mode.as_str() {
            "when_all" => ReactionMode::WhenAll.into(),
            _ => ReactionMode::WhenAny.into(),
        };
        let strategy = InputStrategy::Latest;

        scheduler
            .load_reactor(
                meta.name.clone(),
                accumulators,
                criteria,
                strategy,
                tenant_id.clone(),
                vec![],
                // CLOACI-T-0830: threading a reactor-constructor reference through
                // the FFI `ReactorPackageMetadata` shape is deferred (it needs new
                // serialized fields + signing). Rust cdylib packages dispatch as
                // native dirty-flag reactors for now.
                None,
            )
            .await?;

        tracing::info!(
            reactor = %meta.name,
            package = %meta.package_name,
            "package-declared reactor loaded into scheduler (via get_reactor_metadata)"
        );
        dispatched.push(meta.name.clone());
    }
    Ok(dispatched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_workflow_plugin::AccumulatorDeclarationEntry;

    #[test]
    fn test_build_declaration_from_ffi_metadata() {
        let meta = GraphPackageMetadata {
            graph_name: "test_graph".to_string(),
            package_name: "test-pkg".to_string(),
            reaction_mode: "when_any".to_string(),
            input_strategy: "latest".to_string(),
            accumulators: vec![
                AccumulatorDeclarationEntry {
                    name: "alpha".to_string(),
                    accumulator_type: "passthrough".to_string(),
                    config: HashMap::new(),
                },
                AccumulatorDeclarationEntry {
                    name: "beta".to_string(),
                    accumulator_type: "stream".to_string(),
                    config: [("topic".to_string(), "test.topic".to_string())]
                        .into_iter()
                        .collect(),
                },
            ],
            trigger_reactor: None,
            graph_data_json: None,
        };

        let decl = build_declaration_from_ffi(&meta, vec![0u8; 100]);

        assert_eq!(decl.name, "test_graph");
        assert_eq!(decl.accumulators.len(), 2);
        assert_eq!(decl.accumulators[0].name, "alpha");
        assert_eq!(decl.accumulators[1].name, "beta");
    }

    #[test]
    fn test_reaction_mode_parsing() {
        let meta_any = GraphPackageMetadata {
            graph_name: "g".to_string(),
            package_name: "p".to_string(),
            reaction_mode: "when_any".to_string(),
            input_strategy: "latest".to_string(),
            accumulators: vec![],
            trigger_reactor: None,
            graph_data_json: None,
        };
        let decl_any = build_declaration_from_ffi(&meta_any, vec![]);
        assert!(matches!(
            decl_any.reactor.criteria,
            ReactionCriteria::WhenAny
        ));

        let meta_all = GraphPackageMetadata {
            graph_name: "g".to_string(),
            package_name: "p".to_string(),
            reaction_mode: "when_all".to_string(),
            input_strategy: "sequential".to_string(),
            accumulators: vec![],
            trigger_reactor: None,
            graph_data_json: None,
        };
        let decl_all = build_declaration_from_ffi(&meta_all, vec![]);
        assert!(matches!(
            decl_all.reactor.criteria,
            ReactionCriteria::WhenAll
        ));
        assert!(matches!(
            decl_all.reactor.strategy,
            InputStrategy::Sequential
        ));
    }
}
