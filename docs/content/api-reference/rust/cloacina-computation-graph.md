# cloacina-computation-graph <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Core types for Cloacina computation graph plugins.

This crate contains the types that packaged computation graph cdylibs need
at compile time. It is the computation-graph equivalent of `cloacina-workflow`
— a thin crate that avoids pulling in the full engine.
The `#[computation_graph]` macro expands into code that references types from
this crate. Embedded-mode users get these types re-exported from `cloacina`.

## Structs

### `cloacina-computation-graph::SourceName`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`

Identifies an accumulator source by name.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `String` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (name : impl Into < String >) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
```

</details>



##### `as_str` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn as_str (& self) -> & str
```

<details>
<summary>Source</summary>

```rust
    pub fn as_str(&self) -> &str {
        &self.0
    }
```

</details>





### `cloacina-computation-graph::InputCache`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

The input cache holds the last-seen serialized boundary per source.

The reactor's receiver task updates this cache continuously. The executor
takes a snapshot before calling the compiled graph function.
Serialization format:
- **Release builds**: bincode (compact binary, fast)
- **Debug builds**: JSON (human-readable, inspectable)

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `entries` | `HashMap < SourceName , Vec < u8 > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
```

</details>



##### `update` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn update (& mut self , source : SourceName , bytes : Vec < u8 >)
```

Update the cached value for a source.

<details>
<summary>Source</summary>

```rust
    pub fn update(&mut self, source: SourceName, bytes: Vec<u8>) {
        self.entries.insert(source, bytes);
    }
```

</details>



##### `get` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get < T : DeserializeOwned > (& self , name : & str) -> Option < Result < T , GraphError > >
```

Get and deserialize a cached value by source name.

<details>
<summary>Source</summary>

```rust
    pub fn get<T: DeserializeOwned>(&self, name: &str) -> Option<Result<T, GraphError>> {
        let bytes = self.entries.get(&SourceName::new(name))?;
        Some(deserialize::<T>(bytes))
    }
```

</details>



##### `has` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn has (& self , name : & str) -> bool
```

Check if a source has an entry in the cache.

<details>
<summary>Source</summary>

```rust
    pub fn has(&self, name: &str) -> bool {
        self.entries.contains_key(&SourceName::new(name))
    }
```

</details>



##### `get_raw` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_raw (& self , name : & str) -> Option < & [u8] >
```

Get the raw bytes for a source.

<details>
<summary>Source</summary>

```rust
    pub fn get_raw(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .get(&SourceName::new(name))
            .map(|v| v.as_slice())
    }
```

</details>



##### `snapshot` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn snapshot (& self) -> InputCache
```

Create a snapshot (clone) of the cache.

<details>
<summary>Source</summary>

```rust
    pub fn snapshot(&self) -> InputCache {
        self.clone()
    }
```

</details>



##### `len` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn len (& self) -> usize
```

Number of sources in the cache.

<details>
<summary>Source</summary>

```rust
    pub fn len(&self) -> usize {
        self.entries.len()
    }
```

</details>



##### `is_empty` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_empty (& self) -> bool
```

Whether the cache is empty.

<details>
<summary>Source</summary>

```rust
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
```

</details>



##### `replace_all` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn replace_all (& mut self , other : InputCache)
```

Replace all entries.

<details>
<summary>Source</summary>

```rust
    pub fn replace_all(&mut self, other: InputCache) {
        self.entries = other.entries;
    }
```

</details>



##### `sources` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn sources (& self) -> Vec < & SourceName >
```

List all source names in the cache.

<details>
<summary>Source</summary>

```rust
    pub fn sources(&self) -> Vec<&SourceName> {
        self.entries.keys().collect()
    }
```

</details>



##### `entries_raw` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn entries_raw (& self) -> & HashMap < SourceName , Vec < u8 > >
```

Get a reference to the raw entries map.

<details>
<summary>Source</summary>

```rust
    pub fn entries_raw(&self) -> &HashMap<SourceName, Vec<u8>> {
        &self.entries
    }
```

</details>



##### `entries_as_json` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn entries_as_json (& self) -> HashMap < String , String >
```

Return entries as a JSON-friendly map.

<details>
<summary>Source</summary>

```rust
    pub fn entries_as_json(&self) -> HashMap<String, String> {
        self.entries
            .iter()
            .map(|(name, bytes)| {
                let value = if cfg!(debug_assertions) {
                    serde_json::from_slice::<serde_json::Value>(bytes)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|_| hex_encode(bytes))
                } else {
                    hex_encode(bytes)
                };
                (name.as_str().to_string(), value)
            })
            .collect()
    }
```

</details>





### `cloacina-computation-graph::ComputationGraphRegistration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Metadata about a registered computation graph.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `graph_fn` | `CompiledGraphFn` | The compiled graph function. |
| `accumulator_names` | `Vec < String >` | Accumulator names declared in the graph topology. |
| `reaction_mode` | `String` | Reaction mode: "when_any" or "when_all". |



## Enums

### `cloacina-computation-graph::GraphResult` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Result of executing a compiled computation graph.

#### Variants

- **`Completed`** - Graph executed to completion. Contains terminal node outputs.
- **`Error`** - Graph execution failed.



### `cloacina-computation-graph::GraphError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during graph execution.

#### Variants

- **`Serialization`**
- **`Deserialization`**
- **`MissingInput`**
- **`NodeExecution`**
- **`Execution`**



## Functions

### `cloacina-computation-graph::serialize`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn serialize < T : Serialize > (value : & T) -> Result < Vec < u8 > , GraphError >
```

Serialize a value to bytes using the build-profile-appropriate format.

- Release: bincode (fast, compact)
- Debug: JSON (readable, inspectable in logs)

<details>
<summary>Source</summary>

```rust
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, GraphError> {
    #[cfg(debug_assertions)]
    {
        serde_json::to_vec(value).map_err(|e| GraphError::Serialization(e.to_string()))
    }
    #[cfg(not(debug_assertions))]
    {
        bincode::serialize(value).map_err(|e| GraphError::Serialization(e.to_string()))
    }
}
```

</details>



### `cloacina-computation-graph::deserialize`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn deserialize < T : DeserializeOwned > (bytes : & [u8]) -> Result < T , GraphError >
```

Deserialize bytes to a value using the build-profile-appropriate format.

<details>
<summary>Source</summary>

```rust
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, GraphError> {
    #[cfg(debug_assertions)]
    {
        serde_json::from_slice(bytes).map_err(|e| GraphError::Deserialization(e.to_string()))
    }
    #[cfg(not(debug_assertions))]
    {
        bincode::deserialize(bytes).map_err(|e| GraphError::Deserialization(e.to_string()))
    }
}
```

</details>



### `cloacina-computation-graph::hex_encode`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn hex_encode (bytes : & [u8]) -> String
```

<details>
<summary>Source</summary>

```rust
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
```

</details>



### `cloacina-computation-graph::register_computation_graph_constructor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_computation_graph_constructor < F > (graph_name : String , constructor : F) where F : Fn () -> ComputationGraphRegistration + Send + Sync + 'static ,
```

Register a computation graph constructor in the global registry.

<details>
<summary>Source</summary>

```rust
pub fn register_computation_graph_constructor<F>(graph_name: String, constructor: F)
where
    F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
{
    let mut registry = GLOBAL_COMPUTATION_GRAPH_REGISTRY.write();
    registry.insert(graph_name.clone(), Box::new(constructor));
    tracing::debug!("Registered computation graph constructor: {}", graph_name);
}
```

</details>



### `cloacina-computation-graph::global_computation_graph_registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn global_computation_graph_registry () -> GlobalComputationGraphRegistry
```

Get the global computation graph registry.

<details>
<summary>Source</summary>

```rust
pub fn global_computation_graph_registry() -> GlobalComputationGraphRegistry {
    GLOBAL_COMPUTATION_GRAPH_REGISTRY.clone()
}
```

</details>



### `cloacina-computation-graph::list_registered_graphs`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn list_registered_graphs () -> Vec < String >
```

List all registered computation graph names.

<details>
<summary>Source</summary>

```rust
pub fn list_registered_graphs() -> Vec<String> {
    let registry = GLOBAL_COMPUTATION_GRAPH_REGISTRY.read();
    registry.keys().cloned().collect()
}
```

</details>



### `cloacina-computation-graph::deregister_computation_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn deregister_computation_graph (graph_name : & str)
```

Remove a computation graph from the global registry.

<details>
<summary>Source</summary>

```rust
pub fn deregister_computation_graph(graph_name: &str) {
    let mut registry = GLOBAL_COMPUTATION_GRAPH_REGISTRY.write();
    registry.remove(graph_name);
    tracing::debug!("Deregistered computation graph constructor: {}", graph_name);
}
```

</details>
