---
title: "Python CG Decorator Surface"
description: "How the Python computation graph decorators map onto the Rust macro family — `@cloaca.reactor` / `ComputationGraphBuilder` / `@cloaca.accumulator` and friends."
weight: 10
---

# Python CG Decorator Surface

<!-- TODO(DOC-G Phase 5): full content deferred. Sources to read: -->
<!--   - crates/cloacina-python/src/computation_graph.rs (decorator implementations) -->
<!--   - crates/cloacina-python/src/reactor.rs (reactor surface) -->
<!--   - crates/cloacina-macros/src/{reactor_attr.rs,computation_graph/parser.rs} (Rust side) -->
<!--   - .metis archived I-0101 / I-0102 -->

This page sketches the 1:1 mapping between the Python CG decorator surface and the Rust macro family. Pending full verification, the headline correspondences are:

| Rust macro | Python equivalent | Notes |
|---|---|---|
| `#[reactor(name = ..., accumulators = [...], criteria = ...)]` | `@cloaca.reactor(name=..., accumulators=[...], criteria=...)` | Per CLOACI-I-0101, the reactor is a standalone publisher; same in Python. |
| `#[computation_graph(graph = { ... })]` | `cloaca.ComputationGraphBuilder("name", graph={...})` as a context manager | Per CLOACI-I-0101, the trigger is optional in both surfaces. The embedded form (`invokes = computation_graph(...)`) translates to `@cloaca.task(invokes="...")`. |
| `#[stream_accumulator]` / `#[batch_accumulator]` / `#[state_accumulator]` / `#[passthrough_accumulator]` | `@cloaca.stream_accumulator` / `@cloaca.batch_accumulator` / `@cloaca.state_accumulator` / `@cloaca.passthrough_accumulator` | Same family, same semantics. |

## Where it differs

Where the Python surface departs from the Rust surface, it's usually for ergonomic reasons:

- **Topology shape**: Python uses a dict (`graph={...}`); Rust uses a token-tree literal (`graph = { ... }`). The Python dict is parsed at build-time inside the context manager rather than at compile-time inside the macro.
- **Borrow semantics**: Rust node functions take `Option<&T>` (references into the cache); Python node functions get owned values (the GIL boundary forces a copy).

## Single FFI surface (CLOACI-I-0102)

Whichever language authors the CG, the resulting package implements the same 9-method FFI vtable per CLOACI-I-0102. The server / runner does not know whether a loaded package is Rust- or Python-authored.

## See also

- [Rust · CG · Reference]({{< ref "/computation-graphs/reference/computation-graphs" >}}).
- [Rust · CG · Architecture]({{< ref "/computation-graphs/explanation/architecture" >}}).
- **CLOACI-I-0101** — CG / reactor decouple + embedded CG form.
- **CLOACI-I-0102** — Unified `cloacina::package!()` shell + 9-method FFI vtable.
