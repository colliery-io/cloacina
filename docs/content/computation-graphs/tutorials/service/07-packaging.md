---
title: "07 - Packaging a Computation Graph"
description: "Compile a computation graph as a cdylib plugin, upload it to the server, and watch the reconciler load it"
weight: 10
---

In this tutorial you'll take a computation graph from Rust source code all the way to a running graph loaded inside the Cloacina server. You'll build it as a shared library, package it into a `.cloacina` source archive, upload it via the REST API, and verify that the reconciler compiles and loads it automatically.

## What you'll learn

- The directory layout and `package.toml` fields for a computation graph package
- The `Cargo.toml` configuration for `cdylib` output
- How to write a minimal single-accumulator graph with `#[computation_graph]`
- Packaging the source into a `.cloacina` archive and uploading via `POST /tenants/public/workflows`
- Polling the health endpoints to confirm the graph is live

## Prerequisites

- Completion of the library tutorial [07 - Your First Computation Graph]({{< ref "/computation-graphs/tutorials/library/07-computation-graph/" >}})
- The Cloacina server running and reachable (see the workflow service tutorials for server setup)
- A valid PAK token (bootstrap key or one created via `POST /auth/keys`)
- Rust toolchain installed (`rustc`, `cargo`)
- `curl` and `tar` available in your shell

## Time estimate

20–30 minutes (most of which is waiting for the first Rust compile)

---

## Background: how packaged graphs work

A computation graph package is a Rust crate compiled as a `cdylib`. The server's reconciler watches for newly uploaded `.cloacina` archives, extracts the source, compiles it, and loads the resulting shared library via fidius FFI. Once loaded, the graph's accumulators and reactor are registered with the `ReactiveScheduler` and start accepting events.

The key distinction from a packaged workflow: the graph plugin exposes an `execute_graph()` FFI method that receives a serialized `InputCache` snapshot and returns the terminal node outputs. The host server owns all accumulator channels and the reactor loop — your plugin only contains the pure computation logic.

---

## Step 1: Create the project directory

```bash
mkdir my-price-signal
cd my-price-signal
```

## Step 2: Write `package.toml`

`package.toml` is the Cloacina package manifest. It tells the reconciler that this is a computation graph, not a workflow.

```toml
[package]
name = "my-price-signal"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
package_type = ["computation_graph"]
graph_name = "price_signal"
language = "rust"
description = "Compute a mid-price signal from order book snapshots"
reaction_mode = "when_any"
input_strategy = "latest"
```

The required `[metadata]` fields for computation graphs:

| Field | Required | Meaning |
|---|---|---|
| `package_type` | Yes | Must include `"computation_graph"` |
| `graph_name` | Yes | Identifier used for accumulator and reactor names |
| `language` | Yes | `"rust"` — tells the reconciler how to compile |
| `reaction_mode` | Yes | `"when_any"` or `"when_all"` |
| `input_strategy` | Yes | `"latest"` (use most recent value per source) or `"sequential"` |

## Step 3: Write `Cargo.toml`

```toml
[package]
name = "my-price-signal"
version = "0.1.0"
edition = "2021"

[workspace]

[features]
default = ["packaged"]
packaged = []

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-computation-graph = "0.3"
cloacina-macros = "0.3"
cloacina-workflow-plugin = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
tokio = { version = "1.0", features = ["full"] }

[build-dependencies]
cloacina-build = "0.3"
```

{{< hint type=info title="Why both cdylib and rlib?" >}}
`cdylib` produces the shared library (`.so`/`.dylib`/`.dll`) that the server loads at runtime. `rlib` lets you run `cargo test` against the crate — tests cannot link against a `cdylib` directly.
{{< /hint >}}

## Step 4: Write `build.rs`

`cloacina-build` generates the FFI glue that fidius needs to call your `execute_graph()` function.

```rust
fn main() {
    cloacina_build::configure();
}
```

## Step 5: Write `src/lib.rs`

Create a minimal graph: a single `orderbook` accumulator drives a `compute_signal` entry node which produces a `PriceSignal` terminal output.

```rust
use serde::{Deserialize, Serialize};

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

// --- Computation graph ---

#[cloacina_macros::computation_graph(
    react = when_any(orderbook),
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

## Step 6: Build the shared library locally (optional verification)

Before packaging, verify the crate compiles:

```bash
cargo build --lib
```

On success you'll see the shared library in:

```
target/debug/libmy_price_signal.dylib   # macOS
target/debug/libmy_price_signal.so      # Linux
target/debug/my_price_signal.dll        # Windows
```

You don't need to ship this file — the server compiles from source.

## Step 7: Create the source archive

The server expects a `.cloacina` file, which is a bz2-compressed tar archive. The archive must have a top-level directory named `{package-name}-{version}/` containing all source files.

```bash
cd ..   # go one level above my-price-signal/
tar -cjf my-price-signal.cloacina \
  --transform 's,^my-price-signal,my-price-signal-0.1.0,' \
  my-price-signal/package.toml \
  my-price-signal/Cargo.toml \
  my-price-signal/build.rs \
  my-price-signal/src/lib.rs
```

Verify the archive structure:

```bash
tar -tjf my-price-signal.cloacina
```

Expected output:
```
my-price-signal-0.1.0/package.toml
my-price-signal-0.1.0/Cargo.toml
my-price-signal-0.1.0/build.rs
my-price-signal-0.1.0/src/lib.rs
```

{{< hint type=warning title="Archive structure matters" >}}
The reconciler expects a single top-level directory named `{name}-{version}`. If the paths inside the archive don't match this layout, the extract step will fail and the package will be rejected.
{{< /hint >}}

## Step 8: Upload the package

Set your server base URL and PAK token:

```bash
BASE_URL="http://localhost:8080"
TOKEN="clk_your_bootstrap_or_api_key_here"
```

Upload via multipart form:

```bash
curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "${BASE_URL}/tenants/public/workflows" \
  -H "Authorization: Bearer ${TOKEN}" \
  -F "file=@my-price-signal.cloacina;type=application/octet-stream"
```

Expected response (HTTP 201):

```json
{
  "id": "a1b2c3d4-...",
  "name": "my-price-signal",
  "version": "0.1.0",
  "status": "pending"
}
```

The `status: "pending"` means the reconciler has accepted the archive and queued the compile job.

## Step 9: Wait for the reconciler to compile and load

The first Rust compile of a new package typically takes 60–120 seconds. The reconciler runs `cargo build --lib` with the Cloacina workspace available as a path dependency, then loads the resulting shared library into the server process.

Poll the reactor health endpoint until your graph appears:

```bash
# Poll every 5 seconds for up to 2 minutes
for i in $(seq 1 24); do
  echo "--- attempt $i ---"
  curl -s "${BASE_URL}/v1/health/reactors" \
    -H "Authorization: Bearer ${TOKEN}" | \
    python3 -m json.tool
  sleep 5
done
```

While compiling you'll see an empty reactor list:

```json
{ "reactors": [] }
```

Once loaded:

```json
{
  "reactors": [
    {
      "name": "price_signal",
      "health": { "state": "running" },
      "accumulators": ["orderbook"],
      "paused": false
    }
  ]
}
```

## Step 10: Check accumulator health

```bash
curl -s "${BASE_URL}/v1/health/accumulators" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -m json.tool
```

Expected:

```json
{
  "accumulators": [
    {
      "name": "orderbook",
      "status": "healthy"
    }
  ]
}
```

If the accumulator is `"healthy"` and the reactor is `"running"`, your packaged computation graph is live and ready to receive events.

---

## How the reconciler compiles your package

When the server receives a `.cloacina` source package with `package_type = ["computation_graph"]`, the reconciler:

1. Extracts the archive to a temporary build directory
2. Injects a `[patch.crates-io]` section into `Cargo.toml` so path dependencies resolve to the server's bundled Cloacina version
3. Runs `cargo build --lib --release` (or `--debug` depending on server mode)
4. Calls `build_declaration_from_ffi()` to convert the `GraphPackageMetadata` returned by the FFI plugin into a `ComputationGraphDeclaration`
5. Calls `ReactiveScheduler::load_graph()` to spawn the accumulator tasks and reactor loop

The FFI boundary uses JSON (debug builds) or bincode (release builds) for the `InputCache` snapshot passed to `execute_graph()`.

---

## Troubleshooting

**HTTP 400 on upload**: The archive is malformed. Check that the top-level directory matches `{name}-{version}` and that `package.toml` is present.

**Graph never appears in `/v1/health/reactors`**: Check the server logs. Look for `cargo build` errors — the most common cause is a version mismatch in `Cargo.toml`. Make sure `cloacina-computation-graph`, `cloacina-macros`, `cloacina-workflow-plugin`, and `cloacina-build` all use the same version.

**Accumulator shows `"unhealthy"`**: The accumulator task crashed, usually due to a deserialization failure on the first event. Check that the event payload you send matches the boundary type (`OrderBook` in this example).

---

## Next steps

Now that your graph is deployed and running, the next step is to push events into it:

- [**Tutorial 08: WebSocket Event Injection**]({{< ref "/computation-graphs/tutorials/service/08-websocket-events/" >}}) — push events to the `orderbook` accumulator over a WebSocket connection
- [**Tutorial 09: Kafka-Sourced Computation Graphs**]({{< ref "/computation-graphs/tutorials/service/09-kafka-stream/" >}}) — drive accumulators from a Kafka topic instead of WebSocket
