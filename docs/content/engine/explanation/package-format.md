---
title: "Package Format"
description: ".cloacina archive structure, package.toml schema, and how packages are loaded"
weight: 20
reviewer: "dstorey"
review_date: "2026-06-14"
---

# Package Format

This page is the canonical reference for the `.cloacina` package format: the
archive layout, the `package.toml` manifest schema, and how the server/daemon
load a package. It applies to **both** Rust and Python packages.

## Overview

A `.cloacina` package is a **bzip2-compressed tar archive of source**, not a
compiled binary. Inside, everything lives under a single top-level directory
named `<name>-<version>/`. That directory always contains a **`package.toml`**
manifest plus the workflow source:

- **Rust** — `Cargo.toml`, `build.rs`, and `src/` (the cdylib is compiled by the
  server's compiler *at load time*, not shipped in the archive).
- **Python** — a `workflow/` directory holding your module tree (imported at load
  time; nothing is compiled), plus an optional `vendor/` directory for
  third-party dependencies.

The server reads `package.toml` to learn the package's identity and language,
then compiles (Rust) or imports (Python) the source to discover tasks.

{{< hint type="info" title="package.toml is the manifest" >}}
There is no `manifest.json`. `package.toml` is the only manifest the server
reads. It is parsed by `fidius_core::package::load_manifest` into the
`CloacinaMetadata` schema described below.
{{< /hint >}}

## Archive Structure

### Rust package

```
analytics-workflow-1.0.0.cloacina   (bzip2 tar)
└── analytics-workflow-1.0.0/
    ├── package.toml
    ├── Cargo.toml
    ├── build.rs                     # calls cloacina_build::configure()
    └── src/
        └── lib.rs                   # #[task] / cloacina::package!()
```

### Python package

```
data-pipeline-1.0.0.cloacina        (bzip2 tar)
└── data-pipeline-1.0.0/
    ├── package.toml
    ├── workflow/                    # REQUIRED — module tree lives here
    │   └── data_pipeline/
    │       ├── __init__.py
    │       └── tasks.py             # @cloaca.task / @cloaca.trigger
    └── vendor/                      # optional — vendored dependencies
```

For Python, the module tree **must** live under `workflow/`; a top-level module
is rejected at load with `Missing workflow source directory`. The `vendor/`
directory (if present) and `workflow/` are both added to `sys.path` before
import. See [Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}})
for the full Python procedure.

## The `package.toml` Manifest

Every package has a top-level `package.toml` with two tables: `[package]`
(identity) and `[metadata]` (workflow descriptor).

### `[package]` — identity

```toml
[package]
name = "data-pipeline"
version = "1.0.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Package name |
| `version` | string | yes | Semantic version |
| `interface` | string | yes | Plugin interface — `"cloacina-workflow-plugin"` |
| `interface_version` | integer | yes | Interface version — currently `1` |
| `extension` | string | yes | Archive extension — `"cloacina"` |

### `[metadata]` — workflow descriptor

`[metadata]` deserializes into `CloacinaMetadata`
(`crates/cloacina-workflow-plugin/src/types.rs`), which is a **closed schema**
(`#[serde(deny_unknown_fields)]`). Unknown keys are rejected at upload/load.

```toml
# Rust
[metadata]
language = "rust"
workflow_name = "analytics_workflow"
description = "ETL pipeline for analytics data"
author = "you@example.com"
```

```toml
# Python
[metadata]
language = "python"
workflow_name = "data_pipeline"
entry_module = "data_pipeline.tasks"
description = "ETL pipeline for analytics data"
author = "you@example.com"
requires_python = ">=3.10"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `language` | string | **yes** | `"rust"` or `"python"` |
| `workflow_name` | string | for workflows | The workflow name (Rust may source this from `#[workflow(name = …)]`) |
| `graph_name` | string | for computation graphs | Graph name — set instead of `workflow_name` for CG packages |
| `entry_module` | string | Python only | Dotted module path **relative to `workflow/`** the loader imports |
| `requires_python` | string | no | PEP 440 specifier, e.g. `">=3.10"` (Python only) |
| `description` | string | no | Human-readable description |
| `author` | string | no | Author/owner |
| `reaction_mode` | string | computation graphs | `"when_any"` or `"when_all"` |
| `input_strategy` | string | computation graphs | `"latest"` or `"sequential"` |
| `accumulators` | array | computation graphs | `[[metadata.accumulators]]` source configs |

{{< hint type="warning" title="Rejected keys" >}}
`package_type` and `[[metadata.triggers]]` are **hard-rejected** by the closed
schema. Package classification (workflow vs. computation graph vs. reactor) flows
through FFI metadata for Rust and the presence of `graph_name` for Python.
**Triggers are declared in code** (`#[trigger]` in Rust, `@cloaca.trigger` in
Python), never in the manifest.
{{< /hint >}}

## How a Package Is Loaded

1. The server/daemon receives the archive (upload, or the daemon's watch
   directory) and extracts it.
2. It reads `package.toml` and dispatches on `[metadata].language`.
3. **Rust** — the compiler runs `cargo build` on the unpacked source, then loads
   the resulting cdylib and calls its FFI entry points to enumerate tasks.
   **Python** — the loader adds `workflow/` (and `vendor/`) to `sys.path` and
   imports `entry_module`; the `@cloaca.task` / `@cloaca.trigger` decorators
   register tasks and triggers as a side effect of import.
4. Discovered tasks/triggers are registered and the workflow becomes available.

Because discovery happens by **building/importing source**, registration must
occur at import/macro-expansion time — not behind `if __name__ == "__main__"` or
inside a function body.

### Rust FFI entry points

Rust packages don't hand-write `#[no_mangle]` symbols. `cloacina::package!()`
(invoked at crate root) and the `#[plugin_impl]` machinery emit the fidius plugin
registry plus the entry points the host calls: `get_task_metadata`,
`execute_task`, and the computation-graph/reactor/trigger metadata accessors.
Fidius validates the interface hash before any call.

## Building a Package

### Rust — `cloacinactl package`

```bash
# Compile the workflow cdylib (wrapper around cargo build)
cloacinactl package build .

# Pack the source directory into a .cloacina archive
cloacinactl package pack . --out analytics-workflow-1.0.0.cloacina
```

`pack` requires a `package.toml` in the directory and archives the source; it
does not compile. (Compilation happens at load time on the server.)

### Python — `cloacinactl package pack`

```bash
cloacinactl package pack . --out data-pipeline-1.0.0.cloacina
```

`pack` reads `[metadata].language`; for `python` it skips `cargo`, validates the
`workflow/` layout (that the directory exists and `entry_module` resolves under
it), vendors declared dependencies if applicable, and archives the source. A
mis-laid-out package fails at pack time rather than at upload. `package build` is
a no-op for Python (there is nothing to compile). The archive can also be built
by hand — see
[Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}}).

## Inspecting a Package

```bash
# Built-in inspection
cloacinactl package inspect analytics-workflow-1.0.0.cloacina

# Manual — it's just a bzip2 tar
tar -tjf analytics-workflow-1.0.0.cloacina            # list contents
tar -xjOf analytics-workflow-1.0.0.cloacina \
    analytics-workflow-1.0.0/package.toml             # read the manifest
```

## Related Resources

- [Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}}) — scaffold/validate/pack/upload with `package new`
- [Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}}) — the full Python procedure
- [Packaged Triggers]({{< ref "/embed/tutorials/14-packaged-triggers" >}}) — triggers in a packaged Python workflow
- [FFI System]({{< ref "/engine/explanation/ffi-system/" >}}) — how the host calls into compiled packages
- [Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture/" >}}) — load/registration internals
