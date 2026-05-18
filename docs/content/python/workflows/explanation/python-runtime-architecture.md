---
title: "Python Runtime Architecture"
description: "How the Python surface relates to the underlying Rust runtime — PyO3 boundary, the `cloacina-python` crate split, GIL trade-offs, and the FFI it sits on."
weight: 10
---

# Python Runtime Architecture

<!-- TODO(DOC-G Phase 5): full content deferred. Code sources to read before filling: -->
<!--   - `crates/cloacina-python/Cargo.toml` (deps + features) -->
<!--   - `crates/cloacina-python/src/lib.rs` (pymodule registration) -->
<!--   - `crates/cloacina-python/src/bindings/{context,runner,admin,trigger}.rs` -->
<!--   - .metis/initiatives archives for T-0529 / T-0532 (the crate split rationale) -->

This page sketches how the Python `cloaca` module fits onto the underlying Rust runtime. Three points worth knowing:

## The `cloacina-python` crate split (CLOACI-T-0529 / CLOACI-T-0532)

The Python bindings live in a dedicated `cloacina-python` crate, distinct from the core `cloacina` crate. This split lets `cloacina` ship without PyO3 / maturin in its dependency closure — Python support is a build-time opt-in (`pip install cloaca`) rather than a feature flag on the base library.

The wheel (`cloaca`) wraps `cloacina-python`, which depends on `cloacina` at the Rust source level. The Python surface you call (`cloaca.DefaultRunner`, `@cloaca.task`, `@cloaca.workflow`, etc.) is a PyO3 layer over the same Rust types the Rust crate exposes.

## The PyO3 boundary

Every Python call crosses into Rust. Some implications:

- **Tasks run in the Rust async executor**, not the Python event loop. Python `async def` tasks are still polled by the Rust runtime — the PyO3 binding wraps the coroutine and drives it on the tokio executor.
- **Type marshalling is explicit.** Context values cross via JSON (or `serde_json::Value` equivalents in PyO3). Large/binary state should not round-trip through context for every task — use the DAL surface or pass references.
- **The GIL is held while Python code runs.** Cloacina releases the GIL on async boundaries inside Rust (`Python::allow_threads`), but a single Python task body holds the GIL for its duration. Pure-CPU Python tasks will not parallelize across threads in the same way Rust tasks do.

## What "feature parity" means

Python is a first-class surface (see the user feedback memory: Python support is a core capability, not a feature flag). The Rust and Python surfaces aim to track each other 1:1 on every macro / decorator / runtime API:

| Rust | Python equivalent |
|---|---|
| `#[task(...)]` | `@cloaca.task(...)` |
| `#[workflow(...)]` | `@cloaca.workflow(...)` or `WorkflowBuilder` context manager |
| `#[trigger(...)]` | `@cloaca.trigger(...)` |
| `#[reactor(...)]` | `@cloaca.reactor(...)` |
| `#[computation_graph(...)]` | `ComputationGraphBuilder` context manager |
| `DefaultRunner` | `cloaca.DefaultRunner` |

Drift between the two surfaces is treated as a bug. If you find a Rust capability missing in Python, file it as a parity gap.

## See also

- [Rust · Workflow Architecture Overview]({{< ref "/workflows/explanation/architecture-overview" >}}).
- [Rust · Macro System]({{< ref "/workflows/explanation/macro-system" >}}).
- [Python · API Reference]({{< ref "/python/api-reference" >}}).
- **CLOACI-T-0529** — Python crate split (carve PyO3 leakage out of `cloacina`).
- **CLOACI-T-0532** — Python wheel / packaging cleanup.
