---
title: "cloacina::computation_graph::embedded"
description: "Embedded computation-graph runtime builder (CLOACI-T-0738)."
sidebar:
  order: 57
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Embedded computation-graph runtime builder (CLOACI-T-0738).

Replaces the ~60-line hand-wired `main()` block (four channels, an
`AccumulatorContext` full of `None`s, a `CompiledGraphFn` closure, an
unused `manual_rx`, two `tokio::spawn`s) that embedded CG examples used to
copy-paste. The production scheduler already does all of that wiring in
`load_graph`; this is the embedded-friendly face of the same machinery:
```rust
let graph = EmbeddedGraph::spawn(my_graph_declaration()).await?;
graph.push("prices", &serde_json::json!({"symbol": "X", "px": 42.0})).await?;
// ... later
graph.shutdown().await;
```
Manual wiring still works — this is additive.

## Structs

### <span id="struct.EmbeddedGraph"></span>`struct EmbeddedGraph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


A running embedded computation graph: accumulators spawned, reactor live, events pushed via [`push`](Self::push). Dropping the value does NOT stop the graph — call [`shutdown`](Self::shutdown).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `scheduler` | `ComputationGraphScheduler` |  |
| `registry` | `EndpointRegistry` |  |
| `graph_name` | `String` |  |
| `tenant_id` | `Option < String >` | Tenant the declaration was loaded under (`None` for the usual
untenanted embedded graph). CLOACI-T-0921: endpoint lookups are
`(tenant, name)`-scoped, so pushes must carry the same scope the
scheduler registered under. |

#### Methods

##### <span id="EmbeddedGraph.spawn"></span>`spawn`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn spawn (decl : ComputationGraphDeclaration) -> Result < Self , String >
```

Wire and spawn `decl` (accumulators + reactor + compiled graph fn) — the whole block embedded examples used to hand-write.

<details>
<summary>Source</summary>

```rust
    pub async fn spawn(decl: ComputationGraphDeclaration) -> Result<Self, String> {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());
        let graph_name = decl.name.clone();
        let tenant_id = decl.tenant_id.clone();
        scheduler.load_graph(decl).await?;
        Ok(Self {
            scheduler,
            registry,
            graph_name,
            tenant_id,
        })
    }
```

</details>



##### <span id="EmbeddedGraph.push"></span>`push`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn push (& self , accumulator : & str , event : & impl Serialize) -> Result < () , String >
```

Push a JSON-serializable event into an accumulator by name (the same raw-JSON socket contract the server's WS/REST injection uses).

<details>
<summary>Source</summary>

```rust
    pub async fn push(&self, accumulator: &str, event: &impl Serialize) -> Result<(), String> {
        let bytes = serde_json::to_vec(event).map_err(|e| e.to_string())?;
        self.push_raw(accumulator, bytes).await
    }
```

</details>



##### <span id="EmbeddedGraph.push_raw"></span>`push_raw`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn push_raw (& self , accumulator : & str , bytes : Vec < u8 >) -> Result < () , String >
```

Push pre-encoded raw event bytes into an accumulator by name.

<details>
<summary>Source</summary>

```rust
    pub async fn push_raw(&self, accumulator: &str, bytes: Vec<u8>) -> Result<(), String> {
        self.registry
            .send_to_accumulator(
                accumulator,
                super::registry::EndpointScope::of(self.tenant_id.as_deref()),
                bytes,
            )
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
```

</details>



##### <span id="EmbeddedGraph.graph_name"></span>`graph_name`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn graph_name (& self) -> & str
```

The graph's name (== reactor name for self-reactor declarations).

<details>
<summary>Source</summary>

```rust
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }
```

</details>



##### <span id="EmbeddedGraph.scheduler"></span>`scheduler`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn scheduler (& self) -> & ComputationGraphScheduler
```

Escape hatch: the underlying scheduler, for anything the lean surface doesn't cover (manual force-fire, health, additional graphs).

<details>
<summary>Source</summary>

```rust
    pub fn scheduler(&self) -> &ComputationGraphScheduler {
        &self.scheduler
    }
```

</details>



##### <span id="EmbeddedGraph.registry"></span>`registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn registry (& self) -> & EndpointRegistry
```

Escape hatch: the endpoint registry (reactor handles, health).

<details>
<summary>Source</summary>

```rust
    pub fn registry(&self) -> &EndpointRegistry {
        &self.registry
    }
```

</details>



##### <span id="EmbeddedGraph.shutdown"></span>`shutdown`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn shutdown (self)
```

Stop the reactor and accumulators.

<details>
<summary>Source</summary>

```rust
    pub async fn shutdown(self) {
        self.scheduler.shutdown_all().await;
    }
```

</details>





