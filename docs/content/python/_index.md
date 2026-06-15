---
title: "Python (Cloaca)"
description: "Python bindings for Cloacina — define and execute workflows and computation graphs from Python"
weight: 15
---

# Python (Cloaca)

**Cloaca** is the Python package that provides full access to Cloacina's workflow and computation graph engines. Built with PyO3, it offers native performance with Pythonic ergonomics.

**Who this is for:** Python developers building task pipelines or event-driven
graphs. **Prerequisites:** Python 3.9+ and a database (SQLite needs nothing
extra; Postgres for multi-tenancy/scale).

**Relationship to Rust:** Cloaca is *not* a reimplementation — it's a PyO3 layer
over the same Rust engine the [`cloacina` crate]({{< ref "/workflows" >}}) uses,
with full surface parity. The same primitives, packaging format, and database
model apply; where the Python ergonomics differ (decorators + a builder vs.
macros, owned values vs. borrows, the GIL), the
[Python Runtime Architecture]({{< ref "/embed/explanation/python-runtime-architecture" >}})
explains why. Concepts and platform/operations docs in the Rust-side sections
apply to Python deployments too.

## Installation

```bash
pip install cloaca              # Default (SQLite + PostgreSQL)
pip install cloaca[sqlite]      # SQLite only
pip install cloaca[postgres]    # PostgreSQL only
```

## Features

- **Workflow orchestration** — Define tasks with `@cloaca.task`, build workflows with the `WorkflowBuilder` context manager, execute with `DefaultRunner`.
- **Computation graphs** — Author event-driven graphs with `ComputationGraphBuilder` + `@cloaca.reactor` + the accumulator decorators (`@cloaca.passthrough_accumulator`, `@cloaca.stream_accumulator`, `@cloaca.polling_accumulator`, `@cloaca.batch_accumulator`) + `@cloaca.node`.
- **Multi-tenancy** — PostgreSQL schema isolation via `DatabaseAdmin` (fail-closed `search_path`, decommission flow).
- **Cron scheduling** — Time-based workflow execution with skip / run-all catch-up policies.
- **Event triggers** — `@cloaca.trigger` for external or reactor-sourced workflow firing.
- **First-class parity** — Python tracks the Rust surface 1:1; the PyO3 layer is kept out of the core library so `cloacina` can ship without it.

## Two surfaces, Diataxis-aligned

The Python section mirrors the Rust split: workflows on one side, computation graphs on the other. Most users start with workflows.

### Workflows

- [Quick Start]({{< ref "/python/quick-start" >}}) — Get running in 5 minutes.
- [Workflows · Tutorials]({{< ref "/python/workflows/tutorials" >}}) — 00 — basic-workflow through 08 — packaged-triggers.
- [Workflows · How-to Guides]({{< ref "/python/workflows/how-to-guides" >}}) — Backend selection, testing, packaging, performance.
- [Workflows · Reference]({{< ref "/python/workflows/reference" >}}) — Python-specific reference material.
- [Workflows · Explanation]({{< ref "/python/workflows/explanation" >}}) — Python runtime architecture, PyO3 boundary, GIL trade-offs.

### Computation Graphs

- [Computation Graphs · Tutorials]({{< ref "/python/computation-graphs/tutorials" >}}) — Tutorials 09–11 mirroring the Rust library tutorials.
- [Computation Graphs · How-to Guides]({{< ref "/python/computation-graphs/how-to-guides" >}}) — Packaging, reactor-subscription filtering.
- [Computation Graphs · Reference]({{< ref "/python/computation-graphs/reference" >}}) — Topology dict schema and other Python-specific reference.
- [Computation Graphs · Explanation]({{< ref "/python/computation-graphs/explanation" >}}) — How the Python decorators map onto the Rust macro family.

### API

- [API Reference]({{< ref "/reference/python-api" >}}) — Class-by-class / method-by-method Python API surface.
