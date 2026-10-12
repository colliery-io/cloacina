---
title: "cloacina-computation-graph"
description: "Core types for Cloacina computation graph plugins."
sidebar:
  order: 18
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Core types for Cloacina computation graph plugins.

This crate contains the types that packaged computation graph cdylibs need
at compile time. It is the computation-graph equivalent of `cloacina-workflow`
— a thin crate that avoids pulling in the full engine.
The `#[computation_graph]` macro expands into code that references types from
this crate. Embedded-mode users get these types re-exported from `cloacina`.

## Structs

### <span id="struct.SourceName"></span>`struct SourceName`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`

Identifies an accumulator source by name.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `String` |  |

#### Methods

##### <span id="SourceName.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="SourceName.as_str"></span>`as_str`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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





### <span id="struct.InputCache"></span>`struct InputCache`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

The input cache holds the last-seen serialized boundary per source.

The reactor's receiver task updates this cache continuously. The executor
takes a snapshot before calling the compiled graph function.
Serialization format: bincode (compact binary). The FFI packaging bridge
converts bincode→JSON at the boundary for plugin compatibility.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `entries` | `HashMap < SourceName , Vec < u8 > >` |  |

#### Methods

##### <span id="InputCache.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.update"></span>`update`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.get"></span>`get`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.has"></span>`has`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.get_raw"></span>`get_raw`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.snapshot"></span>`snapshot`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.len"></span>`len`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.is_empty"></span>`is_empty`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.replace_all"></span>`replace_all`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.sources"></span>`sources`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.entries_raw"></span>`entries_raw`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="InputCache.entries_as_json"></span>`entries_as_json`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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





### <span id="struct.ComputationGraphRegistration"></span>`struct ComputationGraphRegistration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Metadata about a registered computation graph.

`accumulator_names` and `reaction_mode` are the canonical fields consumed
by the packaging FFI and the reconciler. Bundled-form graphs populate
these from the local declaration; split-form graphs mirror the
referenced reactor's declaration. Trigger-less graphs carry empty
`accumulator_names` and `trigger_reactor = None`.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `graph_fn` | `CompiledGraphFn` | The compiled graph function. |
| `trigger_reactor` | `Option < String >` | Name of the reactor this graph is bound to, if any. `None` for
trigger-less graphs (T-02/T-03 invoke these directly from workflow
tasks or Python tasks). |
| `accumulator_names` | `Vec < String >` | Accumulator names. For split-form graphs this mirrors the reactor's
accumulators; for trigger-less graphs it is empty. |
| `reaction_mode` | `String` | Reaction mode: `"when_any"`, `"when_all"`, or `"none"` for
trigger-less graphs. |



### <span id="struct.ReactorConstructorRef"></span>`struct ReactorConstructorRef`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

A reference to a packaged WASM reactor **constructor** carried by a reactor declaration (CLOACI-T-0830).

When present, the reactor's firing decision is delegated to the named WASM
constructor's `evaluate` (installed on the live `Reactor` via
`with_evaluator`), REPLACING the built-in `WhenAny`/`WhenAll` dirty-flag
criteria. The fields mirror the `constructor!(...)` consumer surface added for
task/trigger in CLOACI-T-0829, so a reactor constructor is authored/resolved
the same way:
* `from` — the provider package, `"name[@version]"`, resolved against the
T-0829 provider search path (`CLOACINA_PROVIDER_PATH` / `./providers` /
the process override);
* `constructor` — which constructor inside that provider (validated against
the provider's `constructor.json` `name`);
* `config` — the author's `config = { name = value }` kwargs as
`(name, value)` pairs in WRITTEN order, bound BY NAME at load.
Resolution happens in the CG scheduler (`load_reactor`) behind the
`constructors-wasm` feature; this type itself is always compiled (it carries
only `String`/`serde_json::Value`, no wasmtime types).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `from` | `String` | Provider package reference, `"name[@version]"`. |
| `constructor` | `String` | The constructor's `constructor.json` name inside the provider. |
| `config` | `Vec < (String , serde_json :: Value) >` | Author config as `(name, value)` pairs in written order; bound by name. |
| `grants` | `Vec < (String , Vec < String >) >` | Tenant capability grants as raw `(kind, patterns)` pairs (kinds: `http`,
`tcp`, `fs`, `env`) — the `grants = { .. }` literal from the reactor's
consumer surface (CLOACI-T-0834). Carried opaquely (like `config`) and
translated to a fidius `EgressPolicy` + capability allow-list at load.
Empty ⇒ default-closed (the constructor reaches nothing). |
| `runtime` | `Option < String >` | CLOACI-T-0920: the author's optional trust-tier PIN — the
`runtime = "wasm"|"native"` literal from the consumer surface, or `None` when
unpinned. Carried as a `String` (validated at macro-expansion time, re-parsed
fail-closed at load) so this crate stays free of any dependency on the
constructor contract crate. When pinned, the load fails unless the RESOLVED
provider's runtime matches; `"native"` additionally acknowledges that `grants`
are advisory-only on that tier. |



### <span id="struct.AccumulatorSpec"></span>`struct AccumulatorSpec`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

The authored KIND + config of one accumulator a reactor consumes (CLOACI-T-0839). Carried on [`ReactorRegistration`] so runtime-walking load paths (notably packaged Python, where `@cloaca.state_accumulator` is the only declaration) can spawn the right accumulator runtime instead of defaulting to passthrough. `config` is the same String-keyed map the manifest override channel uses (e.g. `capacity` for state accumulators).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulator_type` | `String` | "passthrough" | "stream" | "polling" | "batch" | "state". |
| `config` | `std :: collections :: HashMap < String , String >` |  |



### <span id="struct.ReactorRegistration"></span>`struct ReactorRegistration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Runtime-side description of a reactor.

Populated by the `#[reactor]` macro's emitted inventory entry.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulator_names` | `Vec < String >` |  |
| `accumulator_specs` | `Vec < AccumulatorSpec >` | Authored accumulator kinds/configs for (a subset of) `accumulator_names`
(CLOACI-T-0839). Empty on paths where the kind travels out-of-band
(Rust packaged reactors carry it in the FFI metadata); populated by the
Python `@cloaca.reactor` path from the module's accumulator decorators.
Consumers fall back to manifest overrides, then passthrough. |
| `reaction_mode` | `ReactionMode` |  |
| `constructor` | `Option < ReactorConstructorRef >` | Optional packaged WASM reactor-constructor reference (CLOACI-T-0830).
`Some(..)` makes the WASM guest's `evaluate` the reactor's firing
criteria; `None` is the native dirty-flag reactor. Defaults to `None`
so every pre-existing construction site stays valid. |



## Enums

### <span id="enum.GraphResult"></span>`enum GraphResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Result of executing a compiled computation graph.

#### Variants

- **`Completed`** - Graph executed to completion. Contains terminal node outputs.
- **`Error`** - Graph execution failed.



### <span id="enum.GraphError"></span>`enum GraphError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during graph execution.

#### Variants

- **`Serialization`**
- **`Deserialization`**
- **`MissingInput`**
- **`NodeExecution`**
- **`Execution`**



### <span id="enum.ReactionMode"></span>`enum ReactionMode`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


How a reactor decides when to fire.

#### Variants

- **`WhenAny`** - Fire as soon as any one accumulator has new input.
- **`WhenAll`** - Fire only when every accumulator has new input since the last firing.



## Functions

### <span id="fn.serialize"></span>`fn serialize`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn serialize < T : Serialize > (value : & T) -> Result < Vec < u8 > , GraphError >
```

Serialize a value to bincode bytes.

Bincode is used for all internal wire formats (boundary channels,
checkpoint persistence, accumulator-to-reactor messaging).

<details>
<summary>Source</summary>

```rust
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, GraphError> {
    bincode::serialize(value).map_err(|e| GraphError::Serialization(e.to_string()))
}
```

</details>



### <span id="fn.deserialize"></span>`fn deserialize`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn deserialize < T : DeserializeOwned > (bytes : & [u8]) -> Result < T , GraphError >
```

Deserialize bincode bytes to a value.

<details>
<summary>Source</summary>

```rust
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, GraphError> {
    bincode::deserialize(bytes).map_err(|e| GraphError::Deserialization(e.to_string()))
}
```

</details>



### <span id="fn.hex_encode"></span>`fn hex_encode`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


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



