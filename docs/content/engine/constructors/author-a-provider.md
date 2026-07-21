---
title: "Author a Constructor Provider"
description: "How to write a provider crate that ships N constructor members in one WASM component, and consume them by name with capability grants."
weight: 10
---

# Author a Constructor Provider

This guide shows how to author a **provider** — a suite of constructor members
compiled into one WASM component — and how a workflow consumes a member by name.

A single provider crate can expose **many** constructors. They share one component
and one `provider.json` index; a consumer picks one with `constructor = "<name>"`.
A single-constructor provider is just a suite of one.

## The authoring model

- **A provider is a suite.** You author N `#[constructor]` members in one crate and
  aggregate them with one `constructor_provider!(...)` declaration.
- **`from` names the provider crate; `constructor` names the member.** At the call
  site, `from = "<provider crate>"` resolves the provider package and
  `constructor = "<name>"` selects the member. The provider name **is** the provider
  crate's Cargo package name — the same string resolves the Cargo dependency at build
  time and the bundled package at load time.
- **Naming convention.** Name the crate `cloacina-provider-<name>` (e.g.
  `cloacina-provider-fs`); the provider name defaults to it. This is a discovery
  convention, not an enforced rule.
- **The mechanics are free.** Every member shares one per-kind fidius interface;
  the chosen member's name travels in the `configure` payload, so adding members
  costs nothing at the interface/loader layer.

## 1. Author the members

Each member is a struct plus the one body method for its kind. Fields are either
`#[config]` (bound once per instance at load) or `#[param(required|optional)]`
(pulled from the runtime context — `task` kind only).

```rust,ignore
use cloacina_macros::{constructor, constructor_provider};
use cloacina_constructor_contract::ConstructorError;

#[constructor(kind = task, name = "read_file", version = "0.1.0")]
pub struct ReadFile {
    #[config]
    path: String,
}
impl ReadFile {
    fn execute(&self) -> Result<(), ConstructorError> {
        let contents = std::fs::read_to_string(&self.path)
            .map_err(|e| ConstructorError::msg(format!("read {}: {e}", self.path)))?;
        self.set("contents", contents);
        Ok(())
    }
}

#[constructor(kind = task, name = "write_file", version = "0.1.0")]
pub struct WriteFile {
    #[config]
    path: String,
    #[param(required)]
    contents: String,
}
impl WriteFile {
    fn execute(&self) -> Result<(), ConstructorError> {
        std::fs::write(&self.path, &self.contents)
            .map_err(|e| ConstructorError::msg(format!("write {}: {e}", self.path)))?;
        self.set("written_bytes", self.contents.len() as i64);
        Ok(())
    }
}
```

## 2. Declare the provider suite

One `constructor_provider!` per crate aggregates the members into the component,
grouped by kind, and emits the `provider.json` index. Omit `name` and it defaults to
the crate's Cargo package name (`cloacina-provider-fs`) — the string a consumer's
`from` resolves:

```rust,ignore
constructor_provider!(
    version = "0.1.0",
    task = [ReadFile, WriteFile],
    // trigger = [...], accumulator = [...], reactor = [...] for other kinds
);
```

The crate is a standalone `crate-type = ["cdylib", "rlib"]` package with a small
`emit_manifest` host bin that prints `__provider_manifest()` — the packaging step
reads it to write `provider.json`.

## 3. Distribute the provider

**You usually don't package the provider by hand.** A provider is distributed
as an ordinary Cargo crate (crates.io, git, or a path). When a packaged
workflow depends on it and references a member, the **compiler** resolves it
from the dependency graph, builds it to `wasm32-wasip2`, and bundles it into
the workflow package automatically — see
[Consume a Constructor Provider]({{< ref "consume-a-provider.md" >}}).

(To be clear: the **workflow** package itself is still packed and uploaded
exactly as always — that remains the front door for distributing workflows.
What the compiler removes is the *second* channel: hand-packaging providers
and staging them on the server. The provider rides inside the workflow
package.)

For the **embedded** path (no server/compiler), package it explicitly:

```bash
cloacinactl constructor package ./cloacina-provider-fs --sign-key key.secret
```

This builds the crate to `wasm32-wasip2`, emits `provider.json` (both members),
assembles a `runtime = "wasm"` package, optionally Ed25519-signs it, and packs a
`cloacina-provider-fs-0.1.0.cloacina` archive you can stage on a provider
search path.

## 4. Consume a member with grants

Inside a `#[workflow]`, a `constructor!(...)` node instantiates ONE member. The
consumer supplies `config` (bound by name) and default-closed `grants` — the
constructor code is identical regardless of who instantiates it; only the grants
decide what the sandbox may reach.

```rust,ignore
#[workflow(name = "granted")]
pub mod granted {
    use super::*;

    constructor!(
        id = "reader",
        from = "cloacina-provider-fs@0.1.0",
        constructor = "read_file",
        config = { path = "/data/secret.txt" },
        grants = { fs = ["ro:/data"] },   // omit → default-closed, the read is denied
    );
}
```

Selecting the other member is just a different `constructor = "..."`:

```rust,ignore
constructor!(
    id = "writer",
    from = "cloacina-provider-fs@0.1.0",
    constructor = "write_file",
    config = { path = "/data/out.txt" },
    grants = { fs = ["rw:/data"] },
    dependencies = ["seed"],             // an upstream task supplies the `contents` param
);
```

Both `read_file` and `write_file` resolve from the **same** provider package and
component — one download, two members, coexisting independently.

The `@0.1.0` suffix is an enforced version pin (exact or segment-prefix — `@0.1`
matches 0.1.x but not 0.10.x); a mismatch fails at build and at load naming both
versions.

## Native providers — the trusted tier

Some providers cannot be WASM: anything wrapping a native C library
(librdkafka, a database driver, a GPU runtime) has no `wasm32-wasip2` target.
For these, a provider can be **native**: a host cdylib the engine loads
in-process.

The two tiers differ in exactly one thing — trust:

| | **wasm** (default) | **native** |
|---|---|---|
| Runs | sandboxed component | in-process cdylib (`dlopen`) |
| Capability grants | **enforced** (default-closed) | **advisory only** |
| Trust required | none — the sandbox contains it | same as any packaged Rust workflow cdylib |
| Can wrap C deps | no | yes |

`cloacinactl constructor package` prints the tier, and the loader logs
`loaded NATIVE constructor provider (trusted, unsandboxed in-process)` at load —
deploy native providers only from sources you'd trust with host code.

A native provider makes three declarations the WASM form doesn't need:

```toml
# Cargo.toml
[package.metadata.cloacina]
runtime = "native"        # tells the compiler's bundler to build a host cdylib

[features]
default = ["native"]      # gates the native glue the macros emit
native = []

[dependencies]
# fidius HOST SDK (instead of the target-gated guest SDK)
fidius-core = "0.5"
fidius-macro = "0.5"
rdkafka = "0.39"          # the whole point: the C dep ships IN the provider
```

Package it explicitly with `--native`:

```bash
cloacinactl constructor package ./cloacina-provider-kafka --native --sign-key key.secret
```

## Stream accumulators — loop-owning sources

A per-event accumulator (`kind = accumulator`) transforms one event per
`ingest` call. An event **source** — a Kafka consumer, a socket listener — owns
its own loop instead. Declare it with `mode = stream` and write a single
`source` method returning an iterator of boundary-JSON strings:

```rust,ignore
#[constructor(kind = accumulator, mode = stream, name = "kafka_source", version = "0.1.0")]
pub struct KafkaSource {
    #[config] broker: String,
    #[config] topic: String,
    #[config] group: String,
}

impl KafkaSource {
    fn source(&self) -> impl Iterator<Item = String> {
        // Build the consumer from the bound config; the iterator OWNS it
        // (an `-> impl Iterator` return can't borrow `&self`), so dropping
        // the stream tears the consumer down.
        let consumer = connect(&self.broker, &self.topic, &self.group);
        std::iter::from_fn(move || match consumer.poll(POLL_TIMEOUT) {
            Some(Ok(msg)) => Some(payload_string(msg)), // one boundary event
            Some(Err(e))  => { log(e); Some(String::new()) } // keepalive
            None          => Some(String::new()),            // keepalive
        })
    }
}

constructor_provider!(
    version = "0.1.0",
    stream_accumulator = [KafkaSource],
);
```

Two rules of the shape:

- **Blocking is fine.** The engine pumps a native stream on a dedicated OS
  thread — a `poll()` that blocks never touches the async runtime.
- **Yield `""` on poll timeouts.** An empty string is a *keepalive tick* the
  engine skips: it lets an idle stream notice shutdown within one poll window
  instead of parking its thread forever.

Stream accumulators are native-only today (WASM streaming parity is planned).
The complete implementation is
`examples/constructor-contract/cloacina-provider-kafka`.

## Worked example

A complete, runnable example lives at
`examples/constructor-contract/cloacina-provider-fs` (the `cloacina-provider-fs`
suite) and `examples/constructor-contract/fs-grant-demo` (three workflows: a
granted read, a denied read, and a granted write via the second member). Run it
with `cargo run` in the demo crate.

For the NATIVE + stream tier, the worked example is
`examples/constructor-contract/cloacina-provider-kafka` consumed by
`examples/features/computation-graphs/cg-feature-tour` (run it end to end with
`angreal demos features cg-feature-tour`).

The [Seed Providers]({{< ref "seed-providers.md" >}}) — one canonical provider
per primitive kind — are the reference implementations for authoring each kind
(trigger: `cloacina-provider-sensor`, accumulator: `cloacina-provider-extract`,
reactor: `cloacina-provider-quorum`).
