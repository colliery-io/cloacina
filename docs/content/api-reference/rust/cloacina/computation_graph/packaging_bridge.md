# cloacina::computation_graph::packaging_bridge <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Bridge from FFI-loaded package metadata to ReactiveScheduler types.

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

Call execute_graph (method index 3) on the loaded plugin.

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
            .call_method(3, &(request,))
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

Creates a passthrough accumulator with a background task that reads from a
`StreamBackend` and pushes events into the accumulator's socket channel.
The accumulator itself is still passthrough — the stream reader feeds it.

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





## Functions

### `cloacina::computation_graph::packaging_bridge::build_declaration_from_ffi`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn build_declaration_from_ffi (graph_meta : & GraphPackageMetadata , library_data : Vec < u8 > ,) -> ComputationGraphDeclaration
```

Convert FFI graph metadata + library data into a `ComputationGraphDeclaration` that the `ReactiveScheduler` can load.

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

    // Serialize cache entries to JSON for the FFI boundary
    let mut ffi_cache: HashMap<String, String> = HashMap::new();
    for source_name in cache_snapshot.sources() {
        if let Some(raw_bytes) = cache_snapshot.get_raw(source_name.as_str()) {
            #[cfg(debug_assertions)]
            {
                let json_str = String::from_utf8_lossy(raw_bytes).to_string();
                ffi_cache.insert(source_name.as_str().to_string(), json_str);
            }
            #[cfg(not(debug_assertions))]
            {
                match bincode::deserialize::<serde_json::Value>(raw_bytes) {
                    Ok(val) => {
                        let json_str = serde_json::to_string(&val).unwrap_or_default();
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
