---
title: "04 — Packaging a Computation Graph"
description: "Author a computation graph package, upload it to the server, and watch it compile and load"
weight: 14
aliases:
  - "/computation-graphs/tutorials/service/07-packaging/"

---

In this tutorial you'll take a computation graph from Rust source code all the way to a running graph loaded inside the Cloacina server. You'll author it from the canonical scaffold, pack it into a `.cloacina` source archive, upload it via the REST API, and verify that the compiler service builds it and the reconciler loads it.

## What you'll learn

- The directory layout and `package.toml` fields for a computation graph package
- How to write a minimal single-accumulator graph with `#[computation_graph]`
- Packing the source into a `.cloacina` archive and uploading via `POST /v1/tenants/public/workflows`
- Polling the health endpoints to confirm the graph is live

## Prerequisites

- Completion of the library tutorial [07 - Your First Computation Graph]({{< ref "/embed/tutorials/10-computation-graph/" >}})
- The Cloacina server running and reachable, **with a `cloacina-compiler` service attached to the same database** (Rust packages stay `pending` forever without one — see [03 — Packaged Workflows]({{< ref "/service/tutorials/03-packaged-workflows" >}}))
- A valid API key (bootstrap key or one created via the key endpoints)
- Rust toolchain installed (`rustc`, `cargo`)
- `curl` available in your shell

## Time estimate

20–30 minutes (most of which is waiting for the first Rust compile)

---

## Background: how packaged graphs work

A `.cloacina` package is a **source archive** (tar + bzip2). After upload, the separate `cloacina-compiler` service claims the pending package, compiles the crate to a shared library, and writes the result back to the database; the server's reconciler then loads the library via fidius FFI. Once loaded, the graph's accumulators and reactor are registered with the `ComputationGraphScheduler` and start accepting events.

The key distinction from a packaged workflow: the graph plugin exposes graph-execution FFI methods that receive a serialized input-cache snapshot and return the terminal node outputs. The host server owns all accumulator channels and the reactor loop — your plugin only contains the pure computation logic.

---

## Step 1: Scaffold the package

```bash
cloacinactl package new my-price-signal --lang rust --kind graph
cd my-price-signal
```

This emits the canonical graph package layout:

```
my-price-signal/
├── Cargo.toml
├── package.toml
└── src/
    └── lib.rs
```

## Step 2: The `package.toml` manifest

The scaffolded manifest identifies the package and declares the graph's firing behavior:

```toml
[package]
name = "my-price-signal"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "rust"
graph_name = "my_price_signal"
description = "my-price-signal computation graph"
reaction_mode = "when_any"
input_strategy = "latest"
```

The `[metadata]` fields for computation graph packages:

| Field | Required | Meaning |
|---|---|---|
| `graph_name` | Yes | Identifier used for accumulator and reactor names |
| `language` | Yes | `"rust"` — tells the compiler service how to build |
| `description` | No | Human-readable package description |
| `reaction_mode` | No | Firing criteria: `"when_any"` or `"when_all"` |
| `input_strategy` | No | `"latest"` or `"sequential"` |

**Note**: Earlier versions accepted `package_type = ["computation_graph"]` and `[[triggers]]` stanzas in `[metadata]`. Both are now hard-rejected at load time — package classification flows through the FFI metadata the compiled plugin reports, and trigger declarations live on macros in the source.

## Step 3: The `Cargo.toml`

The scaffold's `Cargo.toml` carries only dependencies. Add the two graph crates (`cloacina-macros` for the graph/reactor attribute macros, `cloacina-computation-graph` for the runtime types) so the full dependency list reads:

```toml
[package]
name = "my-price-signal"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina-workflow = { version = "0.11", features = ["packaged", "macros"] }
cloacina-workflow-plugin = "0.11"
cloacina-macros = "0.11"
cloacina-computation-graph = "0.11"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

{{< hint type=important title="No build wiring — the compiler injects it" >}}
There is no `[lib] crate-type`, no `[features]` section, and no `build.rs` — the `cloacina-compiler` service injects `crate-type = ["cdylib", "rlib"]` and the `packaged` feature at build time. Older documentation showed graph packages declaring these plus a `cloacina-build` build-dependency; that model is retired.
{{< /hint >}}

## Step 4: Write `src/lib.rs`

Replace the scaffolded example with a minimal graph: a single `orderbook` accumulator drives a `compute_signal` entry node which produces a `PriceSignal` terminal output.

```rust
use serde::{Deserialize, Serialize};

// One invocation per package — emits the FFI plugin shell the server
// loads at runtime.
cloacina_workflow_plugin::package!();

// --- Boundary types ---

/// Input from the orderbook accumulator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub best_bid: f64,
    pub best_ask: f64,
}

/// Terminal output of the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceSignal {
    pub mid_price: f64,
    pub spread: f64,
}

// --- Reactor: publishes the orderbook accumulator ---

#[cloacina_macros::reactor(
    name = "price_signal_rx",
    accumulators = [orderbook],
    criteria = when_any(orderbook),
)]
pub struct PriceSignalReactor;

// --- Computation graph (reactor-bound) ---

#[cloacina_macros::computation_graph(
    trigger = reactor("price_signal_rx"),
    graph = {
        compute_signal(orderbook) -> emit,
    }
)]
pub mod price_signal {
    use super::*;

    /// Entry node: receives an order book snapshot and computes the mid-price.
    pub async fn compute_signal(orderbook: Option<&OrderBook>) -> PriceSignal {
        match orderbook {
            Some(ob) => PriceSignal {
                mid_price: (ob.best_bid + ob.best_ask) / 2.0,
                spread: ob.best_ask - ob.best_bid,
            },
            None => PriceSignal {
                mid_price: 0.0,
                spread: 0.0,
            },
        }
    }

    /// Terminal node: receives the computed signal and logs it.
    pub async fn emit(signal: &PriceSignal) -> String {
        format!(
            "mid={:.4} spread={:.4}",
            signal.mid_price, signal.spread
        )
    }
}
```

The topology `compute_signal(orderbook) -> emit` means:

- `compute_signal` is an **entry node** — it reads from the `orderbook` accumulator (by receiving `Option<&OrderBook>`)
- `emit` is a **terminal node** — it receives the output of `compute_signal` and its return value is the final graph output
- The reactor fires when the `orderbook` accumulator delivers a new value (`when_any`)

Update `package.toml` so `graph_name = "price_signal"` matches the module name.

## Step 5: Check it compiles locally (optional)

```bash
cloacinactl package build .
```

This runs a plain `cargo build` to catch errors before upload. You won't get a shared library out of it — the cdylib is produced **server-side** by the compiler service, which injects the crate-type at build time.

## Step 6: Validate and pack

```bash
cd ..
cloacinactl package validate my-price-signal
cloacinactl package pack my-price-signal
# my-price-signal/my-price-signal.cloacina
```

`pack` produces the `.cloacina` source archive with the layout the server expects — no hand-rolled `tar` invocations needed.

## Step 7: Upload the package

Set your server base URL and API key:

```bash
BASE_URL="http://localhost:8080"
TOKEN="clk_your_bootstrap_or_api_key_here"
```

Upload via multipart form — the multipart field **must** be named `file`:

```bash
curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "${BASE_URL}/v1/tenants/public/workflows" \
  -H "Authorization: Bearer ${TOKEN}" \
  -F "file=@my-price-signal/my-price-signal.cloacina;type=application/octet-stream"
```

Expected response (HTTP 201):

```json
{
  "package_id": "a1b2c3d4-...",
  "tenant_id": "public"
}
```

(The CLI equivalent is `cloacinactl package upload my-price-signal/my-price-signal.cloacina`.)

The package row lands with `build_status = pending` — the compiler service picks it up from there.

## Step 8: Wait for the compile and load

The first Rust compile of a new package typically takes 60–120 seconds. The compiler service claims the pending row, injects the build wiring, runs the cargo build, and writes back success; the server's reconciler then loads the shared library.

Check compiler progress:

```bash
cloacinactl compiler status
```

Poll the graph health endpoint until your graph appears:

```bash
# Poll every 5 seconds for up to 2 minutes
for i in $(seq 1 24); do
  echo "--- attempt $i ---"
  curl -s "${BASE_URL}/v1/health/graphs" \
    -H "Authorization: Bearer ${TOKEN}" | \
    python3 -m json.tool
  sleep 5
done
```

While compiling you'll see an empty graph list:

```json
{ "items": [], "total": 0 }
```

Once loaded:

```json
{
  "items": [
    {
      "name": "price_signal",
      "health": { "state": "running" },
      "accumulators": ["orderbook"],
      "paused": false
    }
  ],
  "total": 1
}
```

## Step 9: Check accumulator health

```bash
curl -s "${BASE_URL}/v1/health/accumulators" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -m json.tool
```

Expected (abridged):

```json
{
  "items": [
    {
      "name": "orderbook",
      "state": "live",
      "reactor": "price_signal_rx"
    }
  ],
  "total": 1
}
```

If the accumulator is `"live"` and the graph is `"running"`, your packaged computation graph is ready to receive events. (See [Monitoring Computation Graph Health]({{< ref "/engine/computation-graphs/how-to/computation-graph-health" >}}) for the full state set.)

---

## How your package gets compiled and loaded

When you upload a `.cloacina` source package:

1. The server stores it in `workflow_packages` with `build_status = pending`
2. The `cloacina-compiler` service (a separate process polling the same database) claims the row, extracts the source, injects the cdylib crate-type + `packaged` feature, and runs the cargo build
3. On success the compiled shared library is written back and `build_status` becomes `success`
4. The server's registry reconciler picks up the built package, loads it via fidius FFI, converts the reported graph metadata into a `ComputationGraphDeclaration`, and calls `ComputationGraphScheduler::load_graph()` to spawn the accumulator tasks and reactor loop

The fidius FFI boundary always uses **bincode** for serialized values — there is no debug/release wire-format split.

---

## Troubleshooting

**HTTP 400 on upload**: The archive is malformed. Re-produce it with `cloacinactl package pack` and check `package.toml` is present at the archive root.

**Package stays `pending` forever**: No `cloacina-compiler` service is running against the server's database. Start one (`cloacinactl compiler start --database-url ...`).

**Graph never appears in `/v1/health/graphs`**: Check the compiler and server logs. Look for `cargo build` errors — the most common cause is a version mismatch in `Cargo.toml`. Make sure `cloacina-workflow`, `cloacina-workflow-plugin`, `cloacina-macros`, and `cloacina-computation-graph` all use the same version.

**Accumulator shows a degraded state**: The accumulator task crashed, usually due to a deserialization failure on the first event. Check that the event payload you send matches the boundary type (`OrderBook` in this example).

---

## Next steps

Now that your graph is deployed and running, the next step is to push events into it:

- [**Tutorial 05: WebSocket Event Injection**]({{< ref "/service/tutorials/05-websocket-events/" >}}) — push events to the `orderbook` accumulator over a WebSocket connection
- [**Tutorial 06: Kafka-Sourced Computation Graphs**]({{< ref "/service/tutorials/06-kafka-stream/" >}}) — drive accumulators from a Kafka topic instead of WebSocket
