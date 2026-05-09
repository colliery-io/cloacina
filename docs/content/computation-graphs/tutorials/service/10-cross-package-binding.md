---
title: "10 - Cross-Package Reactor Binding"
description: "Build a publisher cdylib that owns a reactor, then bind a subscriber CG from a separate cdylib to it. Cross-package, cross-tenant, hot-reloadable."
weight: 13
---

In this tutorial you'll build two packaged crates that cooperate
across the FFI boundary: **`pricing-publisher`** owns a reactor
(`PriceReactor`), and **`pricing-subscriber`** declares a computation
graph that binds to that reactor as its upstream. You'll deploy them
to a running `cloacina-server`, watch the reconciler load both
packages and wire them together, then practice safe unload ordering.

By the end you'll understand the split-form reactor model, why
cross-package binding works, and what the bound-subscriber guard is
protecting against.

## What you'll learn

- The difference between bundled-form and split-form computation
  graphs (and why split form enables this pattern).
- How `#[reactor(...)]` declares a reactor as a unit struct.
- How `#[computation_graph(trigger = reactor(...))]` binds a graph
  in a *separate* cdylib to a reactor in another cdylib.
- How the reconciler resolves the cross-package reference and what
  ordering constraints it enforces.
- How the bound-subscriber guard rejects unsafe unloads, and the
  correct unload order.

## Prerequisites

- Completion of [07 - Packaging a Computation Graph]({{< ref "/computation-graphs/tutorials/service/07-packaging" >}}).
  You need the basic packaged-CG flow under your fingers before
  attempting cross-package binding.
- A running `cloacina-server` from [01 - Deploy a Server]({{< ref "/platform/tutorials/01-deploy-a-server" >}}).
- A tenant + tenant-scoped key configured (a profile like
  `acme-prod` from the deployment tutorial).

## Time estimate

30–45 minutes.

---

## Background: split-form reactors

In the bundled form (the default for the `#[computation_graph]`
macro), a reactor and the graph that subscribes to it are declared
together in a single module. There's a 1:1 mapping; loading the
package creates one reactor with one subscriber, and unloading the
package tears both down together.

The split form decouples the two. A reactor is declared standalone
with `#[reactor(...)]`. One or more graphs declare an upstream
binding via `#[computation_graph(trigger = reactor(MyReactor),
...)]`. The N graphs can live in the same package as the reactor,
in different packages, or even in different tenants — the
reconciler resolves the `reactor(MyReactor)` reference by name at
load time.

This unlocks a real architectural pattern: a publisher package owns
a reactor that emits events of broad interest (e.g., a price feed,
a signal stream, a normalized event bus), and any number of
subscriber packages bind their own graphs to consume from it.

## Step 1: Build the publisher package

```bash
cargo new --lib pricing-publisher
cd pricing-publisher
```

### `Cargo.toml`

```toml
[package]
name = "pricing-publisher"
version = "0.1.0"
edition = "2021"

[features]
default = ["packaged"]
packaged = []

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-macros = "0.6.1"
cloacina-workflow = { version = "0.6.1", features = ["packaged"] }
cloacina-workflow-plugin = "0.6.1"
async-trait = "0.1"
serde_json = "1.0"

[build-dependencies]
cloacina-build = "0.6.1"
```

### `build.rs`

```rust
fn main() {
    cloacina_build::configure();
}
```

### `package.toml`

```toml
[package]
name = "pricing-publisher"
version = "0.1.0"
description = "Publishes a normalized price stream via a split-form reactor"
```

### `src/lib.rs`

```rust
use cloacina_workflow::*;
use cloacina_workflow_plugin::*;

// The reactor declaration. This is a unit struct with ACCUMULATORS
// + REACTION_MODE consts. The macro emits a ReactorEntry inventory
// submission so the reconciler can discover it.
#[reactor(
    name = "PriceReactor",
    accumulators = ["raw_prices", "normalized_prices"],
    criteria = when_any
)]
pub struct PriceReactor;

// Accumulators that feed the reactor. Both are passthrough — the
// upstream producer (a Kafka stream or a WebSocket pusher) feeds
// them; the reactor fires when any new boundary arrives.
#[passthrough_accumulator]
pub async fn raw_prices(value: serde_json::Value) -> serde_json::Value {
    value
}

#[passthrough_accumulator]
pub async fn normalized_prices(value: serde_json::Value) -> serde_json::Value {
    value
}

// The unified plugin shell. Emits the FFI vtable for fidius-host.
#[cfg(feature = "packaged")]
cloacina::package!();
```

### Build and pack

```bash
cloacinactl package build .
cloacinactl package pack .
# pricing-publisher-0.1.0.cloacina
```

### Upload

```bash
cloacinactl package upload pricing-publisher-0.1.0.cloacina --tenant acme
```

Watch the server log: you'll see step 3 of the reconciler pipeline
register `PriceReactor` (and its two accumulators) into the
computation-graph scheduler. The reactor is now live and listening
for boundary events on `raw_prices` and `normalized_prices`.

Verify:

```bash
cloacinactl graph list --tenant acme
# Includes "PriceReactor" with its accumulators
```

## Step 2: Build the subscriber package

```bash
cd ..
cargo new --lib pricing-subscriber
cd pricing-subscriber
```

### `Cargo.toml`, `build.rs`, `package.toml`

Same shape as the publisher. Different `name`/`version`.

### `src/lib.rs`

```rust
use cloacina_workflow::*;
use cloacina_workflow_plugin::*;

// This is the cross-package binding. The macro references
// `reactor(PriceReactor)` by name — but PriceReactor is declared
// in the publisher package, NOT here. The reconciler resolves the
// reference at load time by looking up "PriceReactor" in the host
// runtime's reactor registry.
//
// The accumulator names ("raw_prices", "normalized_prices") MUST
// be a subset of the reactor's declared accumulators. The macro
// validates this at compile time within a single crate, but
// across packages the reconciler enforces it at load time and
// rejects the load with a clear error if the binding is invalid.
#[computation_graph(
    react = when_any,
    graph = {
        score: { inputs: ["raw_prices", "normalized_prices"], next: "publish" },
        publish: {},
    },
    trigger = reactor(PriceReactor),
)]
pub mod price_consumer {
    use super::*;

    pub async fn score(
        raw: serde_json::Value,
        normalized: serde_json::Value,
    ) -> ScoreOutput {
        // Compute a derived signal from the two upstream feeds.
        ScoreOutput {
            ratio: extract_ratio(&raw, &normalized),
        }
    }

    pub async fn publish(scored: ScoreOutput) -> serde_json::Value {
        // Terminal node — output is collected by the host.
        serde_json::json!({"signal": scored.ratio})
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoreOutput {
    pub ratio: f64,
}

fn extract_ratio(raw: &serde_json::Value, normalized: &serde_json::Value) -> f64 {
    // Stand-in business logic.
    0.0
}

#[cfg(feature = "packaged")]
cloacina::package!();
```

### Build, pack, upload

```bash
cloacinactl package build .
cloacinactl package pack .
cloacinactl package upload pricing-subscriber-0.1.0.cloacina --tenant acme
```

This time, watch the server log carefully. The reconciler will:

1. Run steps 1–4 for the subscriber package (no triggers, no
   reactors, no trigger-less CGs in this package — all skip
   cleanly).
2. Run step 5 — reactor-bound computation graphs. The macro emitted
   metadata declaring the binding to `PriceReactor`. The
   scheduler's `load_graph()` looks up the reactor by name in its
   own registry, finds it (because the publisher loaded it in
   step 3 of *that* package's pipeline), and binds the new graph
   to the existing reactor.

If the publisher had not been loaded, you'd see this error:

```text
Error loading reactor-bound CG 'price_consumer':
  reactor 'PriceReactor' not loaded
```

The reconciler refuses to bind to an absent reactor. Operators must
load the publisher first (or the reconciler will retry on the next
poll once the publisher arrives).

Verify the subscriber is bound:

```bash
cloacinactl graph status PriceReactor --tenant acme
# Shows subscribers: 1 (price_consumer)
```

## Step 3: Drive an event through

The accumulators are passthrough; you can push events via the
WebSocket interface (see [Tutorial 08]({{< ref "/computation-graphs/tutorials/service/08-websocket-events" >}})
for the full WS protocol).

The recipe below uses
[`websocat`](https://github.com/vi/websocat) (`cargo install
websocat` or `brew install websocat`) and `jq`. If you don't have
them, the WebSocket tutorial walks through alternatives.

> The reactor and the subscriber graph must be loaded in the
> **same tenant**. Cross-tenant binding is not supported; the
> reactor lookup happens in the tenant's `Runtime` registry.

```bash
# Acquire a single-use WebSocket ticket.
TICKET=$(curl -s -X POST http://127.0.0.1:8080/v1/auth/ws-ticket \
    -H "Authorization: Bearer $ACME_KEY" \
    | jq -r .ticket)

# Connect to raw_prices and push an event.
echo '{"symbol":"BTCUSD","price":42000}' | \
    websocat "ws://127.0.0.1:8080/v1/ws/accumulator/raw_prices?token=$TICKET"
```

The reactor fires `when_any` (the moment any accumulator gets a new
boundary), invokes the subscriber's `price_consumer` graph with both
boundaries, and the `publish` terminal node's output is collected.

Check the metrics:

```bash
curl -s http://127.0.0.1:8080/metrics | grep -E 'cloacina_(graph|reactor)'
```

## Step 4: The bound-subscriber guard

Now try to unload the publisher *first*. The reconciler will reject
it:

```bash
cloacinactl package delete <publisher-id> --tenant acme
```

```text
Error: reactor 'PriceReactor' has 1 bound subscriber(s):
       ['price_consumer']; unbind them first
```

This is the bound-subscriber guard. Tearing down the reactor while
the subscriber's CG still depends on it would leave the subscriber
with no upstream and dangling references.

The correct unload order is **subscribers first**, then publisher:

```bash
cloacinactl package delete <subscriber-id> --tenant acme
# OK: reactor PriceReactor now has 0 subscribers

cloacinactl package delete <publisher-id> --tenant acme
# OK: reactor PriceReactor torn down
```

The reconciler does **not** auto-cascade. It surfaces the
rejection so operators can decide whether unloading the
subscribers is genuinely the right move (vs. realizing the
publisher unload was a mistake and aborting).

For the full unload-ordering recipe — including recovery from
partial unloads — see [Safely Unload a Package]({{< ref "/platform/how-to-guides/safely-unload-a-package" >}}).

## What you've built

- A two-cdylib pattern where a reactor in one package is consumed
  by a graph in another.
- Working knowledge of split-form vs bundled-form reactors.
- Hands-on familiarity with the cross-package reactor name lookup
  and the bound-subscriber guard.
- The mental model for designing a Cloacina deployment around
  shared upstream signals.

## Where to go next

- [Reactor Lifecycle]({{< ref "/computation-graphs/explanation/reactor-lifecycle" >}})
  — the dual-layer reactor teardown (scheduler-side + Runtime-side
  constructor cleanup) and why both arms exist.
- [Reconciler Pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}})
  — the six-step ordering and why step 3 (reactors) must precede
  step 5 (reactor-bound CGs).
- [Trigger-less Computation Graphs]({{< ref "/computation-graphs/explanation/trigger-less-graphs" >}})
  — the *other* CG model: graphs invoked by workflow tasks rather
  than by reactor events.
- [Safely Unload a Package]({{< ref "/platform/how-to-guides/safely-unload-a-package" >}})
  — the full unload-ordering recipe, including recovery from
  partial unloads.
