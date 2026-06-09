# cloacina::computation_graph::packaging_bridge <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Bridge from FFI-loaded package metadata to ComputationGraphScheduler types.

Converts `GraphPackageMetadata` + library data into `ComputationGraphDeclaration`
with `AccumulatorFactory` implementations and a `CompiledGraphFn` that calls
`execute_graph()` via fidius FFI.

## Structs

### `cloacina::computation_graph::packaging_bridge::LoadedGraphPlugin`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


A persistent handle to a loaded FFI graph plugin.

Loaded once from library bytes, kept alive for the lifetime of the graph.
The `PluginHandle` is behind a `Mutex` because fidius calls are synchronous
and must not be invoked concurrently.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `handle` | `std :: sync :: Mutex < fidius_host :: PluginHandle >` |  |
| `_temp_dir` | `tempfile :: TempDir` |  |

#### Methods

##### `load` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn load (library_data : & [u8]) -> Result < Self , String >
```

Load a graph plugin from library bytes. The library is written to a temp file, loaded via fidius, and kept resident for reuse.

<details>
<summary>Source</summary>

```rust
    fn load(library_data: &[u8]) -> Result<Self, String> {
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
```

</details>



##### `execute_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn execute_graph (& self , request : GraphExecutionRequest ,) -> Result < cloacina_workflow_plugin :: GraphExecutionResult , String >
```

Call execute_graph on the loaded plugin.

<details>
<summary>Source</summary>

```rust
    fn execute_graph(
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
```

</details>





### `cloacina::computation_graph::packaging_bridge::PassthroughAccumulatorFactory`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A generic passthrough accumulator factory for FFI-loaded packages.

All packaged accumulators are passthrough at the host level — they receive
serialized events via WebSocket/socket and forward them to the reactor.
The actual processing logic lives inside the FFI plugin's `execute_graph()`.



### `cloacina::computation_graph::packaging_bridge::GenericPassthroughAccumulator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>




### `cloacina::computation_graph::packaging_bridge::StreamBackendAccumulatorFactory`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A stream-backed accumulator factory for FFI-loaded packages.

Creates a passthrough accumulator with a `KafkaEventSource` that pulls raw
bytes from a Kafka topic. The event source runs on its own task via
`accumulator_runtime_with_source`. The socket channel remains available for
out-of-band WebSocket pushes.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `config` | `std :: collections :: HashMap < String , String >` | Stream backend config from the package metadata. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (config : std :: collections :: HashMap < String , String >) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(config: std::collections::HashMap<String, String>) -> Self {
        Self { config }
    }
```

</details>





### `cloacina::computation_graph::packaging_bridge::KafkaEventSource`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


EventSource that reads raw bytes from a Kafka topic.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `broker_var` | `String` |  |
| `topic` | `String` |  |
| `group` | `String` |  |
| `extra` | `std :: collections :: HashMap < String , String >` |  |
| `name` | `String` |  |



## Functions

### `cloacina::computation_graph::packaging_bridge::call_get_reactor_metadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn call_get_reactor_metadata (handle : & fidius_host :: PluginHandle ,) -> Result < Vec < cloacina_workflow_plugin :: ReactorPackageMetadata > , String >
```

Call `get_reactor_metadata` (method index 4) on a loaded fidius plugin.

I-0102 / T-B: this is the host-side bridge that consumes the unified
`cloacina::package!()` shell's reactor metadata. Plugins built before
trait v2 (or per-macro `_ffi` blocks emitting empty stubs) return either
`CallError::NotImplemented { bit }` or `Ok(vec![])` — both translate to
"package declares no reactors" and the reconciler skips the reactor
dispatch step for that package.

<details>
<summary>Source</summary>

```rust
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
```

</details>



### `cloacina::computation_graph::packaging_bridge::call_get_trigger_metadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn call_get_trigger_metadata (handle : & fidius_host :: PluginHandle ,) -> Result < Vec < cloacina_workflow_plugin :: TriggerPackageMetadata > , String >
```

Call `get_trigger_metadata` (method index 5) on a loaded fidius plugin.

I-0102 / T-B: same NotImplemented fallback as `call_get_reactor_metadata`.
The reconciler routes cron-shaped entries (cron_expression present) to the
cron scheduler and the rest to the runtime trigger registry.

<details>
<summary>Source</summary>

```rust
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
```

</details>



### `cloacina::computation_graph::packaging_bridge::build_declaration_from_ffi`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn build_declaration_from_ffi (graph_meta : & GraphPackageMetadata , library_data : Vec < u8 > ,) -> ComputationGraphDeclaration
```

Convert FFI graph metadata + library data into a `ComputationGraphDeclaration` that the `ComputationGraphScheduler` can load.

The library is loaded once here and the handle is kept alive in the
`CompiledGraphFn` closure for reuse on every reactor fire.

<details>
<summary>Source</summary>

```rust
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
            let factory: Arc<dyn AccumulatorFactory> = match acc_entry.accumulator_type.as_str() {
                "stream" => Arc::new(StreamBackendAccumulatorFactory::new(
                    acc_entry.config.clone(),
                )),
                _ => Arc::new(PassthroughAccumulatorFactory),
            };
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
    }
}
```

</details>



### `cloacina::computation_graph::packaging_bridge::execute_graph_via_ffi`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn execute_graph_via_ffi (plugin : & Arc < LoadedGraphPlugin > , cache : & InputCache) -> GraphResult
```

Execute a computation graph via FFI using the pre-loaded plugin handle.

<details>
<summary>Source</summary>

```rust
async fn execute_graph_via_ffi(plugin: &Arc<LoadedGraphPlugin>, cache: &InputCache) -> GraphResult {
    let cache_snapshot = cache.snapshot();

    // Recover raw bytes from bincode wire format, then interpret as UTF-8 JSON
    // for the FFI boundary. The passthrough accumulator stores raw event bytes
    // (typically JSON from WebSocket) which are bincode-serialized as Vec<u8>.
    let mut ffi_cache: HashMap<String, String> = HashMap::new();
    for source_name in cache_snapshot.sources() {
        if let Some(raw_bytes) = cache_snapshot.get_raw(source_name.as_str()) {
            // Wire format is bincode(Vec<u8>) — recover the original bytes
            match bincode::deserialize::<Vec<u8>>(raw_bytes) {
                Ok(original_bytes) => {
                    // Original bytes are JSON from WebSocket — convert to string
                    let json_str = String::from_utf8(original_bytes).unwrap_or_else(|e| {
                        tracing::warn!(
                            source = source_name.as_str(),
                            "cache entry is not valid UTF-8, hex-encoding: {}",
                            e
                        );
                        // Fall back to hex encoding for non-UTF-8 data
                        raw_bytes.iter().map(|b| format!("{:02x}", b)).collect()
                    });
                    ffi_cache.insert(source_name.as_str().to_string(), json_str);
                }
                Err(e) => {
                    return GraphResult::error(GraphError::Serialization(format!(
                        "Failed to deserialize cache entry '{}' for FFI: {}",
                        source_name.as_str(),
                        e
                    )));
                }
            }
        }
    }

    let request = GraphExecutionRequest { cache: ffi_cache };

    // FFI call is synchronous — run in a blocking task
    let plugin = plugin.clone();
    let result = tokio::task::spawn_blocking(move || plugin.execute_graph(request)).await;

    match result {
        Ok(Ok(ffi_result)) => {
            if ffi_result.success {
                let outputs: Vec<Box<dyn std::any::Any + Send>> =
                    if let Some(json_outputs) = ffi_result.terminal_outputs_json {
                        json_outputs
                            .into_iter()
                            .filter_map(|json_str| {
                                serde_json::from_str::<serde_json::Value>(&json_str)
                                    .ok()
                                    .map(|v| Box::new(v) as Box<dyn std::any::Any + Send>)
                            })
                            .collect()
                    } else {
                        vec![]
                    };
                GraphResult::completed(outputs)
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
```

</details>



### `cloacina::computation_graph::packaging_bridge::dispatch_runtime_reactors_into_scheduler`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn dispatch_runtime_reactors_into_scheduler (runtime : & crate :: Runtime , scheduler : & super :: scheduler :: ComputationGraphScheduler , accumulator_overrides : & [cloacina_workflow_plugin :: types :: AccumulatorConfig] , tenant_id : Option < String > ,) -> Result < Vec < String > , String >
```

Dispatch every reactor registered in `runtime` into `scheduler` via `scheduler.load_reactor`. Idempotent on `(reactor_name, contract)` — callable repeatedly without spawning duplicate reactors.

This is the runtime-side glue that makes a reactor declaration in any
package "just work" without a co-located CG subscriber. The reconciler
drives this once per package load, after the language-specific loader
has populated the runtime's reactor registry. Accumulator factories
come from optional `package.toml`-style overrides (passthrough/stream)
with passthrough as the default.
Returns the names of reactors that were dispatched (newly loaded plus
idempotent re-loads). Errors short-circuit and surface to the caller —
package loading is fail-fast under the I-0101 lifecycle model.

<details>
<summary>Source</summary>

```rust
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
                let factory: Arc<dyn AccumulatorFactory> = match accumulator_overrides
                    .iter()
                    .find(|cfg| &cfg.name == acc_name)
                {
                    Some(override_cfg) => match override_cfg.accumulator_type.as_str() {
                        "stream" => Arc::new(StreamBackendAccumulatorFactory::new(
                            override_cfg.config.clone(),
                        )),
                        _ => Arc::new(PassthroughAccumulatorFactory),
                    },
                    None => Arc::new(PassthroughAccumulatorFactory),
                };
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
            )
            .await?;

        tracing::info!(reactor = %name, "package-declared reactor loaded into scheduler");
        dispatched.push(name);
    }
    Ok(dispatched)
}
```

</details>



### `cloacina::computation_graph::packaging_bridge::dispatch_package_reactors_into_scheduler`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn dispatch_package_reactors_into_scheduler (reactor_metadata : & [cloacina_workflow_plugin :: ReactorPackageMetadata] , scheduler : & super :: scheduler :: ComputationGraphScheduler , accumulator_overrides : & [cloacina_workflow_plugin :: types :: AccumulatorConfig] , tenant_id : Option < String > ,) -> Result < Vec < String > , String >
```

Dispatch reactors declared by a packaged Rust cdylib (T-B / I-0102).

Consumes `Vec<ReactorPackageMetadata>` produced by the unified
`cloacina::package!()` shell's `get_reactor_metadata` and registers each
reactor with the `ComputationGraphScheduler`. Mirrors the shape of
`dispatch_runtime_reactors_into_scheduler` (which serves the Python
path) so the reconciler's reactor step looks identical between
languages.
`accumulator_overrides` is the manifest's `[metadata].accumulators`
table — kept as input until T-E removes manifest-side accumulator
overrides entirely. Today it shadows FFI-default `passthrough` with
`stream` configurations.

<details>
<summary>Source</summary>

```rust
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
                let factory: Arc<dyn AccumulatorFactory> = match accumulator_overrides
                    .iter()
                    .find(|cfg| cfg.name == acc.name)
                {
                    Some(override_cfg) => match override_cfg.accumulator_type.as_str() {
                        "stream" => Arc::new(StreamBackendAccumulatorFactory::new(
                            override_cfg.config.clone(),
                        )),
                        _ => Arc::new(PassthroughAccumulatorFactory),
                    },
                    None => match acc.accumulator_type.as_str() {
                        "stream" => {
                            Arc::new(StreamBackendAccumulatorFactory::new(acc.config.clone()))
                        }
                        _ => Arc::new(PassthroughAccumulatorFactory),
                    },
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
```

</details>
