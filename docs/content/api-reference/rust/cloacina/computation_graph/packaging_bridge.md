# cloacina::computation_graph::packaging_bridge <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Bridge from FFI-loaded package metadata to ComputationGraphScheduler types.

Converts `GraphPackageMetadata` + library data into `ComputationGraphDeclaration`
with `AccumulatorFactory` implementations and a `CompiledGraphFn` that calls
`execute_graph()` via fidius FFI.

## Structs

### `cloacina::computation_graph::packaging_bridge::LoadedGraphPlugin`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

##### `load` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn load (library_data : & [u8]) -> Result < Self , String >
```

Load a graph plugin from library bytes. The library is written to a temp file, loaded via fidius, and kept resident for reuse. Public so the execution agent can run whole-graph firings (CLOACI-T-0722).

<details>
<summary>Source</summary>

```rust
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
```

</details>



##### `execute_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn execute_graph (& self , request : GraphExecutionRequest ,) -> Result < cloacina_workflow_plugin :: GraphExecutionResult , String >
```

Call execute_graph on the loaded plugin.

<details>
<summary>Source</summary>

```rust
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




### `cloacina::computation_graph::packaging_bridge::ProviderStreamAccumulatorFactory`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A PROVIDER-backed stream accumulator factory (CLOACI-T-0907): the `stream` accumulator's source comes from a constructor provider the package bundles (e.g. `cloacina-provider-kafka`), not from host-compiled backend code.

Selected by `accumulator_factory_for` when the accumulator's config carries a
`provider` key:
```toml
[[metadata.accumulators]]
name = "ticks"
accumulator_type = "stream"
[metadata.accumulators.config]
provider = "cloacina-provider-kafka"   # routing: which bundled provider
constructor = "kafka_source"           # routing: which member (default = provider's convention)
broker = "{{ KAFKA_BROKER }}"          # member #[config] (name-keyed, templated)
topic = "tour.ticks"
group = "cg-feature-tour-group"
```
The provider resolves from the process-wide provider search path (the
`providers/` tree the reconciler unpacks bundled providers into); its member's
`source` is driven via fidius `call_streaming` and drained onto the boundary
channel by `ProviderStreamSource` (T-0904). Load failure is LOUD: an ERROR log
+ health `Disconnected` — never a silent passthrough (CLOACI-T-0898 item 3).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `config` | `std :: collections :: HashMap < String , String >` | Full accumulator config; `provider`/`constructor` are routing keys, the
rest are the member's `#[config]` values (may be `{{ VAR }}` templates). |

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





### `cloacina::computation_graph::packaging_bridge::StateAccumulatorFactory`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A state-backed accumulator factory for FFI-loaded / Python packages.

Spawns `state_accumulator_runtime::<serde_json::Value>` with a bounded
`VecDeque` of the given capacity. This is the host-side wiring for
`@cloaca.state_accumulator(capacity=N)` (and Rust's
`#[state_accumulator(capacity=…)]`): values pushed over the socket are
buffered, persisted to the DAL on every write, and the full list is emitted
back as the boundary so the graph can feed its own state on the next fire.
Capacity semantics (see `StateAccumulator`):
- `> 0`: bounded — evicts oldest when at capacity
- `< 0`: unbounded — grows without limit
- `0`:  write-only sink — no history emitted back

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `capacity` | `i32` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (capacity : i32) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(capacity: i32) -> Self {
        Self { capacity }
    }
```

</details>





### `cloacina::computation_graph::packaging_bridge::JsonListBatchAccumulator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


A generic, list-collecting batch accumulator for the packaged path (CLOACI-T-0896). Socket events arrive as JSON bytes (the same wire the passthrough/state accumulators receive); on flush we emit the whole batch as a JSON array, so the boundary matches the shape the FFI cache expects (`bincode(Vec<u8>)` of JSON — see `input_cache_to_ffi_cache`). This mirrors what `state_window_frame` does for the state accumulator.



### `cloacina::computation_graph::packaging_bridge::BatchAccumulatorFactory`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Packaged batch-accumulator factory (CLOACI-T-0896): buffers socket events and flushes the whole buffer as one boundary on the flush interval or when the buffer fills. Mirrors `StateAccumulatorFactory` — socket-driven, so it fits the existing spawn contract without any FFI change.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `flush_interval` | `Option < std :: time :: Duration >` |  |
| `max_buffer_size` | `Option < usize >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (flush_interval : Option < std :: time :: Duration > , max_buffer_size : Option < usize > ,) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(
        flush_interval: Option<std::time::Duration>,
        max_buffer_size: Option<usize>,
    ) -> Self {
        Self {
            flush_interval,
            max_buffer_size,
        }
    }
```

</details>





### `cloacina::computation_graph::packaging_bridge::ClosurePollingAccumulator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


A [`PollingAccumulator`] driven by an injected [`PollClosure`] (the Python poll fn). `poll()` runs the closure on a blocking thread so the GIL work never blocks the async executor — the same discipline `PythonTriggerWrapper` uses for poll triggers. (CLOACI-T-0896)

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `poll_fn` | `PollClosure` |  |
| `interval` | `std :: time :: Duration` |  |



### `cloacina::computation_graph::packaging_bridge::PollingAccumulatorFactory`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Packaged polling-accumulator factory (CLOACI-T-0896). Resolves the poll closure by name at spawn time via the registered builder, then runs `polling_accumulator_runtime` on the configured interval. If no closure is registered for the name, the accumulator simply never emits (logged) rather than failing the load.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `interval` | `std :: time :: Duration` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (interval : std :: time :: Duration) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(interval: std::time::Duration) -> Self {
        Self { interval }
    }
```

</details>





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



### `cloacina::computation_graph::packaging_bridge::call_get_constructor_metadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn call_get_constructor_metadata (handle : & fidius_host :: PluginHandle ,) -> Result < Vec < cloacina_workflow_plugin :: ConstructorPackageMetadata > , String >
```

Call `get_constructor_metadata` (method index 10) on a loaded fidius plugin (CLOACI-T-0832). Returns the packaged workflow's declared `constructor!(...)` nodes for the host to resolve + inject. Plugins built before trait v4 return `CallError::NotImplemented` → `Ok(vec![])` ("package declares no constructor nodes"), so older packages keep loading unchanged.

<details>
<summary>Source</summary>

```rust
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
```

</details>



### `cloacina::computation_graph::packaging_bridge::input_cache_to_ffi_cache`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn input_cache_to_ffi_cache (cache : & InputCache) -> Result < HashMap < String , String > , String >
```

Execute a computation graph via FFI using the pre-loaded plugin handle. Convert an [`InputCache`] snapshot into the FFI/wire cache shape (source name → UTF-8 JSON string) — the same conversion the in-process FFI call performs, shared with the fleet path so a dispatched firing carries an agent-ready cache (CLOACI-T-0722). Boundary frames are `bincode(Vec<u8>)` of raw event JSON; non-UTF-8 payloads are hex-encoded.

<details>
<summary>Source</summary>

```rust
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
```

</details>



### `cloacina::computation_graph::packaging_bridge::execute_graph_via_ffi`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn execute_graph_via_ffi (plugin : & Arc < LoadedGraphPlugin > , cache : & InputCache) -> GraphResult
```

<details>
<summary>Source</summary>

```rust
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
```

</details>



### `cloacina::computation_graph::packaging_bridge::state_capacity_from_config`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn state_capacity_from_config (config : & std :: collections :: HashMap < String , String >) -> i32
```

Parse a state accumulator's capacity from its String-keyed config map. Defaults to `0` (write-only sink) when absent or unparsable.

<details>
<summary>Source</summary>

```rust
fn state_capacity_from_config(config: &std::collections::HashMap<String, String>) -> i32 {
    config
        .get("capacity")
        .and_then(|c| c.parse::<i32>().ok())
        .unwrap_or(0)
}
```

</details>



### `cloacina::computation_graph::packaging_bridge::batch_config_from_config`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn batch_config_from_config (config : & std :: collections :: HashMap < String , String > ,) -> (Option < std :: time :: Duration > , Option < usize >)
```

Parse a batch accumulator's `flush_interval` (e.g. `"1s"`, `"500ms"`) and `max_buffer_size` from its String-keyed config map. Absent/unparsable → `None` (the runtime treats each as an optional flush trigger).

<details>
<summary>Source</summary>

```rust
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
```

</details>



### `cloacina::computation_graph::packaging_bridge::register_polling_accumulator_builder`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_polling_accumulator_builder (builder : PollingClosureBuilder)
```

Register the polling-accumulator poll-closure resolver for the packaged path (CLOACI-T-0896). Idempotent — the first registration wins. The Python extension calls this at module install so a packaged polling accumulator drives its Python poll fn on the configured interval. A pure-Rust host that never installs one gets a loud passthrough fallback for polling accumulators.

<details>
<summary>Source</summary>

```rust
pub fn register_polling_accumulator_builder(builder: PollingClosureBuilder) {
    let _ = POLLING_CLOSURE_BUILDER.set(builder);
}
```

</details>



### `cloacina::computation_graph::packaging_bridge::polling_interval_from_config`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn polling_interval_from_config (config : & std :: collections :: HashMap < String , String > ,) -> std :: time :: Duration
```

Parse a polling accumulator's `interval` (e.g. `"2s"`) from config; defaults to 5s when absent/unparsable.

<details>
<summary>Source</summary>

```rust
fn polling_interval_from_config(
    config: &std::collections::HashMap<String, String>,
) -> std::time::Duration {
    config
        .get("interval")
        .and_then(|s| crate::packaging::manifest_schema::parse_duration_str(s).ok())
        .unwrap_or_else(|| std::time::Duration::from_secs(5))
}
```

</details>



### `cloacina::computation_graph::packaging_bridge::accumulator_factory_for`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn accumulator_factory_for (acc_type : & str , config : & std :: collections :: HashMap < String , String > ,) -> Arc < dyn AccumulatorFactory >
```

<details>
<summary>Source</summary>

```rust
fn accumulator_factory_for(
    acc_type: &str,
    config: &std::collections::HashMap<String, String>,
) -> Arc<dyn AccumulatorFactory> {
    match acc_type {
        // CLOACI-T-0898/T-0907: a stream accumulator's source ALWAYS comes from
        // a bundled constructor provider (`provider`/`constructor` config keys —
        // e.g. cloacina-provider-kafka). The host-compiled kafka backend is
        // gone; a declaration without a `provider` key fails LOUDLY at spawn
        // (ERROR + health Disconnected), never a silent passthrough.
        "stream" => Arc::new(ProviderStreamAccumulatorFactory::new(config.clone())),
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
```

</details>
