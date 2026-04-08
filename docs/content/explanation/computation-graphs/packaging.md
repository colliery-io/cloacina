---
title: "Packaging & FFI"
weight: 30
---

# Packaging & FFI — How Packaged Graphs Work

Embedded computation graphs — graphs defined directly in your application's Rust code using the `#[computation_graph]` macro — are linked at compile time. There is no FFI, no dynamic loading, no plugin system. The graph is just a function.

Packaged computation graphs are different. They are compiled separately into a shared library (`.dylib` / `.so` / `.dll`), uploaded to Cloacina's registry, and loaded at runtime by the reconciler. This document explains why a separate crate exists, how the FFI boundary works, and what the reconciler does to turn a package archive into a running reactor.

## Why a Separate Crate

The `cloacina` crate is the full engine. It includes the DAL, the workflow scheduler, the API server infrastructure, PyO3 bindings, and everything else. Compiled as a dependency, it adds significant weight.

A packaged computation graph compiles into a shared library that the host loads at runtime. If that library had to link against the full `cloacina` crate, it would be ~60MB per plugin. Every graph deployed to the cluster would carry the entire engine as baggage.

`cloacina-computation-graph` solves this. It is a thin types crate — ~2.8MB — containing only what the graph code actually needs at compile time:

- `SourceName`, `InputCache`, `GraphResult`, `GraphError` — the data types
- `CompiledGraphFn` — the type alias for the compiled graph function
- `serialize` / `deserialize` — the profile-aware wire format helpers
- The global computation graph registry — for embedded-mode auto-registration via `#[ctor]`

The `#[computation_graph]` macro expands into code that references types from this crate. Packaged graph authors depend on `cloacina-computation-graph`. Embedded-mode users get the same types re-exported from `cloacina` directly — there is no difference in API.

## The FFI Boundary: fidius

Cloacina uses [fidius](https://github.com/colliery-software/fidius) as its plugin system. fidius provides a stable ABI for calling methods on loaded plugins by index. Graph plugins expose three methods:

| Method index | Name | What it does |
|---|---|---|
| 0 | `get_task_metadata` | Returns task metadata (for workflow packages) |
| 1 | `get_workflow_metadata` | Returns workflow metadata (for workflow packages) |
| 2 | `get_graph_metadata` | Returns `GraphPackageMetadata` — graph name, reaction mode, input strategy, accumulator list |
| 3 | `execute_graph` | Executes the computation graph with a provided `GraphExecutionRequest` (serialized cache) |

`GraphPackageMetadata` is the FFI handshake. It tells the host everything needed to wire up the graph without the host knowing anything about the graph's internal types:

```rust
pub struct GraphPackageMetadata {
    pub graph_name: String,
    pub package_name: String,
    pub reaction_mode: String,   // "when_any" or "when_all"
    pub input_strategy: String,  // "latest" or "sequential"
    pub accumulators: Vec<AccumulatorDeclarationEntry>,
}

pub struct AccumulatorDeclarationEntry {
    pub name: String,
    pub accumulator_type: String,  // "passthrough" or "stream"
    pub config: HashMap<String, String>,
}
```

The accumulator list tells the host what sources the graph expects. The host creates one accumulator per entry, wires them to the reactor, and passes the assembled cache to `execute_graph` when the reactor fires.

Wire format at the FFI boundary uses JSON in debug builds and bincode in release builds — the same profile-aware pattern used throughout the boundary channel. At the FFI call itself, `execute_graph` receives `GraphExecutionRequest { cache: HashMap<String, String> }` — the cache serialized to JSON strings, regardless of build profile. This ensures the FFI boundary is always inspectable and avoids bincode compatibility issues across separately-compiled binaries.

## Load-Once: The LoadedGraphPlugin

A critical design decision: the shared library is loaded exactly once, when the package is registered. It is not dlopen'd on every graph execution.

`LoadedGraphPlugin` is the in-process handle that keeps the library resident:

```rust
struct LoadedGraphPlugin {
    handle: std::sync::Mutex<fidius_host::PluginHandle>,
    _temp_dir: tempfile::TempDir,  // keeps the dylib file alive on disk
}
```

The library bytes are written to a temp directory (platform-appropriate extension: `.dylib`, `.so`, `.dll`), then loaded via `fidius_host::loader::load_library`. The `TempDir` is held in the struct — when the `LoadedGraphPlugin` is dropped, the temp dir is cleaned up and the file is deleted. Until then, the library stays on disk and in memory.

The `PluginHandle` is behind a `std::sync::Mutex` because fidius method calls are synchronous and must not be invoked concurrently. When the reactor fires, `execute_graph` is called via `spawn_blocking` to avoid blocking the async runtime, then the result is sent back to the caller.

The `LoadedGraphPlugin` is wrapped in `Arc` and moved into the `CompiledGraphFn` closure, so every reactor fire uses the same loaded library:

```rust
let plugin = Arc::new(plugin);
let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
    let plugin = plugin.clone();
    Box::pin(async move { execute_graph_via_ffi(&plugin, &cache).await })
});
```

## The dlclose Problem

Why not reload the library on each execution? Or unload it when the package is unloaded?

Rust's `inventory` crate (used internally for type registration) maintains a linked list of registered items. When a cdylib is dynamically loaded, `inventory` items from that library are appended to the global linked list. When the library is `dlclose`'d (unloaded), those list entries become dangling pointers — the memory they point to is freed, but the list still references it. Any subsequent iteration of the registry causes undefined behavior.

This is not a fidius bug or a Cloacina bug — it is a fundamental constraint of `dlclose` with Rust's global state patterns. The solution is: never `dlclose` a library that registered global state. Cloacina keeps the `PluginHandle` alive for the lifetime of the graph, which is the lifetime of the `LoadedGraphPlugin`, which is held by the `CompiledGraphFn` closure, which is held by the reactor.

When a package is unloaded (via the reconciler), the reactor is shut down and the `CompiledGraphFn` is dropped. The `Arc<LoadedGraphPlugin>` reference count goes to zero, the struct is dropped, and the `TempDir` cleans up — but the library has been unloaded gracefully, after all references to it are gone, rather than having the rug pulled out from under it.

## The Reconciler Flow

When a package is uploaded to the registry, the reconciler performs these steps:

```
Package uploaded
      │
      ▼
1. Write archive to temp file
2. Unpack via fidius_core::package::unpack_package
3. Load package.toml → CloacinaMetadata
4. Compile source with `cargo build --lib`
5. Read compiled library bytes
6. register_package_tasks (workflow plugin path)
7. register_package_workflows (workflow plugin path)
8. Detect: has_computation_graph()?
      │
      ├── YES
      │     │
      │     ▼
      │   Call get_graph_metadata (FFI method 2)
      │   Merge manifest accumulator config into FFI defaults
      │   build_declaration_from_ffi(graph_meta, library_data)
      │     → LoadedGraphPlugin::load(library_data)
      │     → Create AccumulatorFactories per accumulator entry
      │     → Assemble ComputationGraphDeclaration
      │   scheduler.load_graph(declaration)
      │     → Spawn accumulators + reactor
      │
      └── NO
            → Done
```

The key step is `build_declaration_from_ffi`, implemented in `packaging_bridge.rs`. It takes the FFI metadata and the library bytes and produces a `ComputationGraphDeclaration` that the `ReactiveScheduler` can consume. This is where the `LoadedGraphPlugin` is created — the library is loaded here, and the handle lives inside the `CompiledGraphFn` closure for the lifetime of the reactor.

## Manifest Accumulator Config

The FFI metadata declares accumulators with their type and default config. The `package.toml` manifest can override these defaults — this is how the same compiled graph package can be configured for different environments without recompilation.

The reconciler merges them in `loading.rs`:

```rust
for manifest_acc in &cloacina_manifest.metadata.accumulators {
    if let Some(ffi_acc) = graph_meta.accumulators.iter_mut()
        .find(|a| a.name == manifest_acc.name)
    {
        ffi_acc.accumulator_type = manifest_acc.accumulator_type.clone();
        ffi_acc.config = manifest_acc.config.clone();
    }
}
```

A `package.toml` might look like:

```toml
[package]
name = "market-maker"
version = "1.0.0"

[metadata]
language = "rust"
type = "computation_graph"
graph_name = "market_maker"

[[metadata.accumulators]]
name = "orderbook"
accumulator_type = "stream"
[metadata.accumulators.config]
topic = "prod.market.orderbook"
group = "market-maker-prod"
```

This allows the graph code to declare `accumulator_type = "passthrough"` as a development default (push events manually for testing) while the deployed manifest overrides it to `stream` with the production Kafka topic and consumer group. The Kafka broker URL itself comes from `KAFKA_BROKER_URL` at runtime, not from the manifest, so it is not embedded in the package.

## Accumulator Types in Packaged Graphs

At the host level, all packaged accumulators are one of two types:

**Passthrough** (`PassthroughAccumulatorFactory`): creates a `GenericPassthroughAccumulator` that forwards `serde_json::Value` events directly to the reactor. The actual event type is opaque to the host — the graph plugin decodes it inside `execute_graph`.

**Stream** (`StreamBackendAccumulatorFactory`): creates the same passthrough accumulator at the host level, but also spawns a background task that reads from a `StreamBackend` (e.g., Kafka) and feeds raw message bytes into the accumulator's socket channel. The passthrough accumulator then forwards those bytes to the reactor.

In both cases, the host's accumulator is a dumb byte forwarder. All type-aware processing happens inside the compiled graph plugin via `execute_graph`. This is the right abstraction: the host cannot know the graph's types (they were compiled separately), so it should not try to process them.

## Python Computation Graphs

Python computation graphs follow a different path. Instead of FFI via fidius, they are loaded via PyO3. The reconciler extracts the Python package, imports the entry module, and the `@computation_graph` decorator registers the graph's executor in the same global registry as Rust graphs. The reactive scheduler then handles them identically.

The Python path bypasses the `LoadedGraphPlugin` / fidius mechanism entirely. Graph execution happens in the Python interpreter (via `spawn_blocking` to avoid blocking the async runtime), with the cache passed across the PyO3 boundary as a `HashMap<String, String>`.

## Further Reading

- [Architecture]({{< ref "architecture" >}}) — the process model and where the reactive scheduler lives
- [Accumulator Design]({{< ref "accumulator-design" >}}) — how accumulators work and what the FFI accumulators are doing
- [Performance Characteristics]({{< ref "performance" >}}) — the overhead cost of the FFI boundary
