---
title: "cloacina-computation-graph"
description: "Core types for Cloacina computation graph plugins."
editUrl: false
sidebar:
  order: 3
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Core types for Cloacina computation graph plugins.

This crate contains the types that packaged computation graph cdylibs need
at compile time. It is the computation-graph equivalent of `cloacina-workflow`
— a thin crate that avoids pulling in the full engine.

The `#[computation_graph]` macro expands into code that references types from
this crate. Embedded-mode users get these types re-exported from `cloacina`.

## Re-exports

- `serde_json` (`serde_json`)

## Structs

### <span id="struct.SourceName"></span><code style="font-size: 0.85em">struct SourceName</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`

Identifies an accumulator source by name.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `String` |  |

#### Methods

##### <span id="SourceName.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(name: impl Into<String>) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
```

</details>

##### <span id="SourceName.as_str"></span><code>as_​str</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_str(&self) -> &str
```

<details>
<summary>Source</summary>

```rust
    pub fn as_str(&self) -> &str {
        &self.0
    }
```

</details>

### <span id="struct.InputCache"></span><code style="font-size: 0.85em">struct InputCache</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

The input cache holds the last-seen serialized boundary per source.

The reactor's receiver task updates this cache continuously. The executor
takes a snapshot before calling the compiled graph function.

Serialization format: bincode (compact binary). The FFI packaging bridge
converts bincode→JSON at the boundary for plugin compatibility.

#### Methods

##### <span id="InputCache.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
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

##### <span id="InputCache.update"></span><code>update</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn update(&mut self, source: SourceName, bytes: Vec<u8>)
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

##### <span id="InputCache.get"></span><code>get</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get<T: DeserializeOwned>(
    &self,
    name: &str,
) -> Option<Result<T, GraphError>>
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

##### <span id="InputCache.has"></span><code>has</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn has(&self, name: &str) -> bool
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

##### <span id="InputCache.get_raw"></span><code>get_​raw</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_raw(&self, name: &str) -> Option<&[u8]>
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

##### <span id="InputCache.snapshot"></span><code>snapshot</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn snapshot(&self) -> InputCache
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

##### <span id="InputCache.len"></span><code>len</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn len(&self) -> usize
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

##### <span id="InputCache.is_empty"></span><code>is_​empty</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_empty(&self) -> bool
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

##### <span id="InputCache.replace_all"></span><code>replace_​all</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn replace_all(&mut self, other: InputCache)
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

##### <span id="InputCache.sources"></span><code>sources</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn sources(&self) -> Vec<&SourceName>
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

##### <span id="InputCache.entries_raw"></span><code>entries_​raw</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn entries_raw(&self) -> &HashMap<SourceName, Vec<u8>>
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

##### <span id="InputCache.entries_as_json"></span><code>entries_​as_​json</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn entries_as_json(&self) -> HashMap<String, String>
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

### <span id="struct.ComputationGraphRegistration"></span><code style="font-size: 0.85em">struct ComputationGraphRegistration</code>

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
| `trigger_reactor` | `Option<String>` | Name of the reactor this graph is bound to, if any. `None` for trigger-less graphs (T-02/T-03 invoke these directly from workflow tasks or Python tasks). |
| `accumulator_names` | `Vec<String>` | Accumulator names. For split-form graphs this mirrors the reactor's accumulators; for trigger-less graphs it is empty. |
| `reaction_mode` | `String` | Reaction mode: `"when_any"`, `"when_all"`, or `"none"` for trigger-less graphs. |

### <span id="struct.ReactorConstructorRef"></span><code style="font-size: 0.85em">struct ReactorConstructorRef</code>

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
| `config` | `Vec<(String, serde_json::Value)>` | Author config as `(name, value)` pairs in written order; bound by name. |
| `grants` | `Vec<(String, Vec<String>)>` | Tenant capability grants as raw `(kind, patterns)` pairs (kinds: `http`, `tcp`, `fs`, `env`) — the `grants = { .. }` literal from the reactor's consumer surface (CLOACI-T-0834). Carried opaquely (like `config`) and translated to a fidius `EgressPolicy` + capability allow-list at load. Empty ⇒ default-closed (the constructor reaches nothing). |
| `runtime` | `Option<String>` | CLOACI-T-0920: the author's optional trust-tier PIN — the `runtime = "wasm"\|"native"` literal from the consumer surface, or `None` when unpinned. Carried as a `String` (validated at macro-expansion time, re-parsed fail-closed at load) so this crate stays free of any dependency on the constructor contract crate. When pinned, the load fails unless the RESOLVED provider's runtime matches; `"native"` additionally acknowledges that `grants` are advisory-only on that tier. |

### <span id="struct.AccumulatorSpec"></span><code style="font-size: 0.85em">struct AccumulatorSpec</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

The authored KIND + config of one accumulator a reactor consumes (CLOACI-T-0839). Carried on [`ReactorRegistration`](../../rust/cloacina-computation-graph/#struct.ReactorRegistration) so runtime-walking load paths (notably packaged Python, where `@cloaca.state_accumulator` is the only declaration) can spawn the right accumulator runtime instead of defaulting to passthrough. `config` is the same String-keyed map the manifest override channel uses (e.g. `capacity` for state accumulators).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulator_type` | `String` | "passthrough" \| "stream" \| "polling" \| "batch" \| "state". |
| `config` | `std::collections::HashMap<String, String>` |  |

### <span id="struct.ReactorRegistration"></span><code style="font-size: 0.85em">struct ReactorRegistration</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Runtime-side description of a reactor.

Populated by the `#[reactor]` macro's emitted inventory entry.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulator_names` | `Vec<String>` |  |
| `accumulator_specs` | `Vec<AccumulatorSpec>` | Authored accumulator kinds/configs for (a subset of) `accumulator_names` (CLOACI-T-0839). Empty on paths where the kind travels out-of-band (Rust packaged reactors carry it in the FFI metadata); populated by the Python `@cloaca.reactor` path from the module's accumulator decorators. Consumers fall back to manifest overrides, then passthrough. |
| `reaction_mode` | `ReactionMode` |  |
| `constructor` | `Option<ReactorConstructorRef>` | Optional packaged WASM reactor-constructor reference (CLOACI-T-0830). `Some(..)` makes the WASM guest's `evaluate` the reactor's firing criteria; `None` is the native dirty-flag reactor. Defaults to `None` so every pre-existing construction site stays valid. |

## Enums

### <span id="enum.GraphResult"></span><code style="font-size: 0.85em">enum GraphResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Result of executing a compiled computation graph.

#### Variants

- **`Completed`** - Graph executed to completion. Contains terminal node outputs.
- **`Error`** - Graph execution failed.

#### Methods

##### <span id="GraphResult.completed"></span><code>completed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn completed(outputs: Vec<Box<dyn Any + Send>>) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn completed(outputs: Vec<Box<dyn Any + Send>>) -> Self {
        Self::Completed {
            outputs,
            outputs_json: Vec::new(),
        }
    }
```

</details>

##### <span id="GraphResult.completed_with_json"></span><code>completed_​with_​json</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn completed_with_json(
    outputs: Vec<Box<dyn Any + Send>>,
    outputs_json: Vec<serde_json::Value>,
) -> Self
```

Completed with the terminal outputs also captured as JSON (CLOACI-T-0775).

<details>
<summary>Source</summary>

```rust
    pub fn completed_with_json(
        outputs: Vec<Box<dyn Any + Send>>,
        outputs_json: Vec<serde_json::Value>,
    ) -> Self {
        Self::Completed {
            outputs,
            outputs_json,
        }
    }
```

</details>

##### <span id="GraphResult.error"></span><code>error</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn error(err: GraphError) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn error(err: GraphError) -> Self {
        Self::Error(err)
    }
```

</details>

##### <span id="GraphResult.is_completed"></span><code>is_​completed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_completed(&self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }
```

</details>

##### <span id="GraphResult.is_error"></span><code>is_​error</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_error(&self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
```

</details>

### <span id="enum.GraphError"></span><code style="font-size: 0.85em">enum GraphError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during graph execution.

#### Variants

- **`Serialization`**
- **`Deserialization`**
- **`MissingInput`**
- **`NodeExecution`**
- **`Execution`**

### <span id="enum.ReactionMode"></span><code style="font-size: 0.85em">enum ReactionMode</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

How a reactor decides when to fire.

#### Variants

- **`WhenAny`** - Fire as soon as any one accumulator has new input.
- **`WhenAll`** - Fire only when every accumulator has new input since the last firing.

#### Methods

##### <span id="ReactionMode.as_str"></span><code>as_​str</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-const" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">const</span>

```rust
const fn as_str(&self) -> &'static str
```

<details>
<summary>Source</summary>

```rust
    pub const fn as_str(&self) -> &'static str {
        match self {
            ReactionMode::WhenAny => "when_any",
            ReactionMode::WhenAll => "when_all",
        }
    }
```

</details>

## Traits

### <span id="trait.Reactor"></span><code style="font-size: 0.85em">trait Reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Compile-time handle for a reactor declaration.

Implemented by the unit struct emitted by `#[reactor]`. The constants are
readable at `const` context, which the `#[computation_graph]` macro uses to
const-check the graph's entry accumulators against the reactor's
accumulator set.

### <span id="trait.Graph"></span><code style="font-size: 0.85em">trait Graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Compile-time handle for a computation graph declaration.

Implemented by the `__CGHandle_<modname>` unit struct emitted by
`#[computation_graph]`. Lets other macros (notably `#[task(invokes =
computation_graph(H))]`) reference a graph by type path and const-check
invariants like trigger-less-ness at compile time.

## Functions

### <span id="fn.serialize"></span><code style="font-size: 0.85em">fn serialize</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, GraphError>
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

### <span id="fn.deserialize"></span><code style="font-size: 0.85em">fn deserialize</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn deserialize<T: DeserializeOwned>(
    bytes: &[u8],
) -> Result<T, GraphError>
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

## Type aliases

| Name | Type | Description |
|------|------|-------------|
| <span id="type.CompiledGraphFn"></span>`CompiledGraphFn` | `Arc<dyn Fn(InputCache) -> Pin<Box<dyn Future<Output = GraphResult> + Send>> + Send + Sync>` | Type alias for the compiled graph function. |
| <span id="type.ComputationGraphConstructor"></span>`ComputationGraphConstructor` | `Box<dyn Fn() -> ComputationGraphRegistration + Send + Sync>` |  |
| <span id="type.ReactorConstructor"></span>`ReactorConstructor` | `Box<dyn Fn() -> ReactorRegistration + Send + Sync>` |  |
