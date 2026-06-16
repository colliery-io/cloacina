---
title: "Package a Python Computation Graph"
description: "Build, validate, pack, and upload a .cloacina archive containing a Python computation graph."
weight: 10
aliases:
  - "/python/computation-graphs/how-to-guides/package-a-python-computation-graph/"

---

# Package a Python Computation Graph

A Python-authored computation graph packages and deploys exactly like a Python
workflow: a `package.toml` plus a module tree under `workflow/`. The server loads
it through the same path as a Rust-authored graph. This how-to packages the
`market_maker` graph end to end. (For the workflow analog, see
[Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}}).)

## Prerequisites

- A Python computation graph (see [Python CG Decorator Surface]({{< ref "/engine/explanation/python-cg-decorator-surface" >}})).
- `cloacinactl` installed, and a running server to upload to.

## Step 1: Lay out the package

```
market-maker/
├── package.toml
└── workflow/
    └── market_maker/
        ├── __init__.py
        └── graph.py        # @cloaca.reactor + ComputationGraphBuilder + @cloaca.node
```

The module tree **must** live under `workflow/`. `graph.py` holds the reactor,
the accumulators, the `ComputationGraphBuilder`, and the `@cloaca.node` functions
(see the [Topology Dict Schema]({{< ref "/reference/topology-dict-schema" >}})).

## Step 2: Write `package.toml`

A computation-graph package sets `graph_name` (this is what marks it a CG rather
than a workflow) and `entry_module` (the dotted path, relative to `workflow/`,
the loader imports):

```toml
[package]
name = "market-maker"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "python"
graph_name = "market_maker"
entry_module = "market_maker.graph"
reaction_mode = "when_any"     # or "when_all"
input_strategy = "latest"      # or "sequential"
requires_python = ">=3.10"
```

Both are optional (defaults: `reaction_mode = "when_any"`, `input_strategy =
"latest"`):

- `reaction_mode` should match the `mode=` on your `@cloaca.reactor`
  (`when_any` = fire when any accumulator has new data; `when_all` = wait for all).
- `input_strategy` controls how queued boundaries are consumed: `latest`
  collapses to the newest boundary (one execution on the most recent data);
  `sequential` runs the graph once per boundary in order.

Importing `entry_module` runs your decorators, which register the graph.

## Step 3: Validate and pack

```bash
cloacinactl package validate market-maker
cloacinactl package pack market-maker --out market-maker-0.1.0.cloacina
```

`validate` checks the layout and the closed `[metadata]` schema; `pack` writes
the `.cloacina` archive. (No `maturin`/compile step — a Python package ships
source.)

## Step 4: Upload and verify

```bash
cloacinactl --tenant my_tenant package upload market-maker-0.1.0.cloacina
cloacinactl --tenant my_tenant graph list
```

Once the server loads it, the graph appears in `graph list` and its health is
queryable via `cloacinactl graph status market_maker`.

## See also

- [Topology Dict Schema]({{< ref "/reference/topology-dict-schema" >}})
- [Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}}) — the same packaging model for workflows.
- [Package Format]({{< ref "/engine/explanation/package-format" >}}) — the `.cloacina` archive structure.
