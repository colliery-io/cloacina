---
title: "Consume a Constructor Provider"
description: "Wire a provider member into a packaged workflow — Rust or Python — and let the compiler bundle it hermetically."
weight: 20
---

# Consume a Constructor Provider

This guide covers the **consumer** side: referencing a provider member from a
packaged workflow so it ships, loads, and runs with no operator staging. For
authoring the provider itself, see
[Author a Constructor Provider]({{< ref "author-a-provider.md" >}}).

The contract in one sentence: **declare the provider as a dependency, reference
a member by name, and the compiler bundles everything the deployed package
needs.**

## Rust: a Cargo dependency + `constructor!`

The provider is an ordinary Cargo dependency of the workflow crate:

```toml
[dependencies]
cloacina-provider-fs = "0.1"           # crates.io, or { path = ... } / { git = ... }
```

Inside a `#[workflow]`, a `constructor!(...)` node instantiates one member:

```rust,ignore
#[workflow(name = "constructor_demo")]
pub mod constructor_demo {
    use super::*;

    constructor!(
        id = "reader",
        from = "cloacina-provider-fs@0.1.0",
        constructor = "read_file",
        config = { path = "/etc/hostname" },
        grants = { fs = ["ro:/etc"] },
    );

    #[task(id = "summarize", dependencies = ["reader"])]
    pub async fn summarize(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let contents = context.get("contents").and_then(|v| v.as_str()).unwrap_or("").to_string();
        context.insert("bytes", serde_json::json!(contents.len()))?;
        Ok(())
    }
}
```

At build time the compiler scans the source for `constructor!` /
`#[reactor(... from = ...)]` references, resolves each named provider from the
crate's **resolved Cargo graph**, builds it to a `wasm32-wasip2` component, and
bundles the packed provider into the workflow package. A referenced provider
that is **not** a dependency (or whose pinned version the graph doesn't
provide) fails the build — never silently at load.

## Python: `[metadata.providers]` + `cloaca.constructor`

Python packages have no Cargo manifest, so the provider declaration lives in
`package.toml` — this section is **authoritative** (the only source of provider
dependencies for a Python package):

```toml
[metadata.providers]
cloacina-provider-fs = "0.1.0"
# or detailed specs, same shapes as Cargo dependencies:
# cloacina-provider-fs = { path = "/workspace/providers/cloacina-provider-fs" }
# cloacina-provider-fs = { git = "https://github.com/...", tag = "v0.1.0" }
```

The workflow module wires a member exactly like a task:

```python
import cloaca

cloaca.constructor(
    id="reader",
    from_="cloacina-provider-fs@0.1.0",
    constructor="read_file",
    config={"path": "/etc/hostname"},
    grants={"fs": ["ro:/etc"]},
)

@cloaca.task(dependencies=["reader"])
def summarize(context):
    contents = context.get("contents") or ""
    context.set("bytes", len(contents))
    return context
```

The compiler synthesizes a scratch Cargo project from the declared specs,
builds each provider to wasm, and bundles it — the same hermetic result as the
Rust path. A `cloaca.constructor` reference to a provider **missing** from
`[metadata.providers]` fails at load with "no such provider" (it never resolves
against another package's bundle).

## Version pins

The optional `@version` suffix on `from` is **enforced**, at build time and at
load, with segment-prefix semantics:

| Pin | Matches | Does not match |
| --- | --- | --- |
| `@0.1.0` | exactly 0.1.0 | 0.1.1 |
| `@0.1` | 0.1.x | 0.10.x |
| `@1` | 1.x.y | 10.x.y |

A mismatch is a clear error naming both the pinned and the resolved version.
Full semver operators (`^`, `~`, ranges) are not supported; pin a segment
prefix instead.

## What happens at load and run

- The server's reconciler unpacks the package's bundled providers and resolves
  each declared node **before** the workflow assembles — a package that
  declares constructor nodes but carries no bundles refuses to load
  (fail-closed, hermetic).
- Execution agents do the same: they fetch the bundles from the server
  (content-addressed, alongside the artifact) and resolve nodes in their own
  load path — **fleet dispatch is transparent** to constructor workflows.
- The node executes inside a WASI sandbox scoped to the consumer's `grants` —
  see [Capability Grants]({{< ref "grants.md" >}}). With no grant, the sandbox
  reaches nothing.
- After a Cloacina upgrade that bumps the plugin ABI, previously-compiled
  packages are detected as stale at load and automatically recompiled from
  retained source — no manual rebuild sweep.

## Stream accumulators from a provider

A computation graph's `stream` accumulator can be **supplied by a provider**
instead of host-compiled backend code. In `package.toml`, add
`provider`/`constructor` routing keys to the accumulator config and declare the
provider under `[metadata.providers]`:

```toml
[[metadata.accumulators]]
name = "ticks"
accumulator_type = "stream"

[metadata.accumulators.config]
provider = "cloacina-provider-kafka"   # routing: which bundled provider
constructor = "kafka_source"           # routing: which member
broker = "{{ KAFKA_BROKER }}"          # member #[config], name-keyed + templated
topic = "tour.ticks"
group = "my-consumer-group"

[metadata.providers]
cloacina-provider-kafka = "0.1"        # crates.io / { git = … } / { path = … }
```

`[metadata.providers]` works for **both** languages: it's the only declaration
form for Python packages, and for Rust packages it's *additive* to source-scanned
`constructor!` refs — use it when the provider shouldn't join your crate's own
Cargo graph (a native rdkafka provider is the canonical case). The compiler
builds each declared provider per its own runtime marker (wasm component or
native cdylib) and bundles it; the reconciler stages the bundle and the
accumulator's source streams straight into the graph's boundary channel.

Config values bind **by name** against the member's declared `#[config]` schema
(unknown or missing keys fail the load with a key-named error), and `{{ VAR }}`
templates resolve via `CLOACINA_VAR_*` server env. Native providers are the
trusted tier — see the trust-tier table in
[Author a Constructor Provider]({{< ref "author-a-provider.md" >}}).

The runnable end-to-end example is
`examples/features/computation-graphs/cg-feature-tour`
(`angreal demos features cg-feature-tour`: real Kafka messages fire the graph
through the bundled native provider).

## Embedded (no server)

Embedded runners resolve `from` against a provider **search path** instead of a
bundle: the process-wide override (`set_provider_search_path`), else
`CLOACINA_PROVIDER_PATH`, else `./providers`. Stage provider packages there
(e.g. via `cloacinactl constructor package` + unpack). The runnable
`examples/constructor-contract/fs-grant-demo` shows the full embedded flow.
