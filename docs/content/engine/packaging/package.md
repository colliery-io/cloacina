---
title: "Package"
description: "The .cloacina distributable unit — package.toml plus source — portable across embedded, daemon, and server."
weight: 41
---

# Package

A **`.cloacina` package** is the distributable unit: a `package.toml` manifest
plus source. Once built, the **same package** runs under the embedded library, the
daemon, or the server unchanged — it is the seam that lets you move a
[Workflow]({{< ref "/engine/workflows/workflow" >}}) or
[Computation Graph]({{< ref "/engine/computation-graphs/computation-graph" >}})
between the two doors.

## Mental model

- A package wraps your code with a `package.toml` and a conventional source
  layout.
- The loop is **new → validate → pack → upload**, via `cloacinactl package`.
- The server loads it (the reconciler registers its workflows, triggers, graphs).

## Interfaces

The two languages package differently:

{{< tabs "package-layout" >}}
{{< tab "Python" >}}
A Python package is a module tree under `workflow/`, declaring tasks with **bare
`@cloaca.task` decorators** (no `WorkflowBuilder` — the loader supplies the
workflow context):

```
my-pipeline/
├── package.toml
└── workflow/
    └── my_pipeline/
        ├── __init__.py
        └── tasks.py        # bare @cloaca.task decorators
```
{{< /tab >}}
{{< tab "Rust" >}}
A Rust package ships source with a `Cargo.toml` + `src/lib.rs`; the server
**compiles it on load**. Build/check locally with `cloacinactl package build`.
{{< /tab >}}
{{< /tabs >}}

```bash
cloacinactl package new my-pipeline --lang python   # scaffold
cloacinactl package validate my-pipeline            # check format
cloacinactl package pack my-pipeline                # build the .cloacina archive
cloacinactl --tenant acme package upload my-pipeline-0.1.0.cloacina
```

## Key facts

- **Portable:** one artifact runs embedded / daemon / server.
- **Python:** bare decorators only; a `WorkflowBuilder` inside a package fails to
  load.
- **Rust:** compiled on the server at load time.

## See also

- [Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}}) · [Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}})
- [Run the Service]({{< ref "/service" >}}) — where packages are deployed.
