---
title: "Package Format"
description: ".cloacina archive structure, package.toml schema, and how packages are loaded"
sidebar:
  order: 20
reviewer: "dstorey"
review_date: "2026-06-14"
---

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
  time; nothing is compiled). Its third-party dependencies come as a
  `pyproject.toml` and a `requirements.lock` that the compiler vendors from, or
  as a `vendor/` directory in the archive, one folder per platform.

The server reads `package.toml` to learn the package's identity and language,
then compiles (Rust) or imports (Python) the source to discover tasks.

:::note[package.toml is the manifest]
There is no `manifest.json`. `package.toml` is the only manifest the server
reads. It is parsed by `fidius_core::package::load_manifest` into the
`CloacinaMetadata` schema described below.
:::

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
    ├── pyproject.toml               # optional — declares the dependencies
    ├── requirements.lock            # optional — the pinned versions and hashes
    └── vendor/                      # optional — vendored dependencies
        ├── aarch64-linux/           # one folder per platform, <arch>-<os>
        └── x86_64-linux/
```

For Python, the module tree **must** live under `workflow/`; a top-level module
is rejected at load with `Missing workflow source directory`. `workflow/` and
one vendor folder are added to `sys.path` before import. See
[Packaging Python Workflows](../../../how-to/embed/packaging-python-workflows/)
for the full Python procedure.

#### Dependencies

`cloacinactl package build` reads `[project].dependencies` of `pyproject.toml`
and writes `requirements.lock`: each dependency pinned to a version, with
hashes. From there a package gets its dependencies in one of two ways.

**The compiler vendors.** A package with a `requirements.lock` and no `vendor/`
folder is vendored when it is built on the server: the compiler installs the
wheels the lock names, for each platform of its settings, and stores one set
per platform next to the package. A set is an archive in the package format: a
`package.toml` whose `[metadata]` is the `[metadata.vendored]` record of one
platform, and `vendor/<platform>/`. The uploaded archive is not changed. A
server or an agent that loads the package gets the set of its own platform and
unpacks it into `vendor/<platform>/`; it gets no set of another platform. The
compiler refuses a package that has dependencies and no lock file, and a lock
file that holds more than pinned versions with hashes. The server refuses such
an upload with the same checks, before it stores anything (code
`python_dependencies_not_locked`), and so does `cloacinactl package validate`.

**The package ships `vendor/`.** `cloacinactl package build --vendor` also
writes:

- `vendor/<platform>/`: the installed wheels for one platform. A platform is
  named `<arch>-<os>` (`x86_64-linux`, `aarch64-linux`, `aarch64-macos`,
  `x86_64-macos`, `x86_64-windows`), the name an agent reports for itself.
- `[metadata.vendored]` in `package.toml`: the Python version and the platforms.

The compiler uses such a package as it is.

The loader picks the vendor folder in this order:

1. `vendor/<platform of the loading process>/`, when it exists.
2. With a `[metadata.vendored]` table and no folder for the loading platform:
   another platform's folder when no folder holds compiled code. A package with
   compiled code is refused.
3. With no `[metadata.vendored]` table: `vendor/` itself (a folder filled by
   hand). It is refused when it holds a wheel or a compiled file built for
   another platform.

An archive unpacks to at most `--max-unpacked-bytes` /
`CLOACINA_MAX_UNPACKED_BYTES` (500 MiB by default) and 10,000 files and
folders, and to at most 10 times its own size. The server, the compiler, the
agent, the daemon and `cloacinactl package validate` and `pack` each read the
setting. An upload is at most the server's `--max-package-bytes` (100 MiB by
default).

One package holds every platform. The server keeps one active version per
package name, so a package per platform is not a supported layout.

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

# Written by `cloacinactl package build`: what is in vendor/. Do not edit.
[metadata.vendored]
python_version = "3.11"
platforms = ["aarch64-linux", "x86_64-linux"]
native = true
lock_file = "requirements.lock"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `language` | string | **yes** | `"rust"` or `"python"` |
| `workflow_name` | string | for workflows | The workflow name (Rust may source this from `#[workflow(name = …)]`) |
| `graph_name` | string | for computation graphs | Graph name — set instead of `workflow_name` for CG packages |
| `entry_module` | string | Python only | Dotted module path **relative to `workflow/`** the loader imports |
| `requires_python` | string | no | PEP 440 specifier, e.g. `">=3.10"` (Python only). Enforced against the Python that runs the package: at `package validate`, at upload (the server's Python) and at load (the loading interpreter) |
| `vendored` | table | no | `[metadata.vendored]`, written by `cloacinactl package build` (Python only). See below |
| `description` | string | no | Human-readable description |
| `author` | string | no | Author/owner |
| `reaction_mode` | string | computation graphs | `"when_any"` or `"when_all"` |
| `input_strategy` | string | computation graphs | `"latest"` or `"sequential"` |
| `accumulators` | array | computation graphs | `[[metadata.accumulators]]` source configs |

`[metadata.vendored]` (Python only):

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `python_version` | string | **yes** | The Python `major.minor` the dependencies were resolved and installed for, e.g. `"3.11"`. The package loads only on that `major.minor` |
| `platforms` | array of strings | **yes** | The platforms that have a `vendor/<platform>/` folder |
| `native` | bool | no (default `false`) | `true` when a folder holds compiled code. Such a package loads on the listed platforms only. `false`: every dependency is pure Python and the package runs on any platform |
| `lock_file` | string | no | The lock file, relative to the package root |

:::caution[Rejected keys]
`package_type` and `[[metadata.triggers]]` are **hard-rejected** by the closed
schema. Package classification (workflow vs. computation graph vs. reactor) flows
through FFI metadata for Rust and the presence of `graph_name` for Python.
**Triggers are declared in code** (`#[trigger]` in Rust, `@cloaca.trigger` in
Python), never in the manifest.
:::

## How a Package Is Loaded

1. The server/daemon receives the archive (upload, or the daemon's watch
   directory) and extracts it.
2. It reads `package.toml` and dispatches on `[metadata].language`.
3. **Rust** — the compiler runs `cargo build` on the unpacked source, then loads
   the resulting cdylib and calls its FFI entry points to enumerate tasks.
   **Python** — the compiler vendors the dependencies of the lock file when
   the package ships no `vendor/` folder. The loader unpacks the set of its
   platform, checks `requires_python` and the vendored Python version (of
   `[metadata.vendored]`, or of the set) against its interpreter, picks the
   vendor folder for its platform, and compares the vendored modules with the
   packages already loaded in the process (a module that another package
   vendors at a different version fails the load). Then it adds `workflow/`
   and the vendor folder to `sys.path` and imports `entry_module`; the
   `@cloaca.task` / `@cloaca.trigger` decorators register tasks and triggers
   as a side effect of import.
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

### Python — `cloacinactl package build` and `pack`

```bash
# Lock the dependencies of pyproject.toml into requirements.lock
# (add --vendor to also install them into vendor/<platform>/)
cloacinactl package build .

# Pack the source directory into a .cloacina archive
cloacinactl package pack . --out data-pipeline-1.0.0.cloacina
```

`build` reads `[metadata].language`; for `python` it compiles nothing and
locks the dependencies that `pyproject.toml` declares, with `uv`, for Python
3.11 by default. With `--vendor` it also vendors them, for `aarch64-linux` and
`x86_64-linux` by default. A package with no dependencies needs no `build`.

`pack` validates the `workflow/` layout (that the directory exists and
`entry_module` resolves under it), `requires_python`, the
`[metadata.vendored]` record and that dependencies have a lock file or a
`vendor/` folder, and archives the source. A mis-laid-out package
fails at pack time rather than at upload. The archive can also be built by
hand — see
[Packaging Python Workflows](../../../how-to/embed/packaging-python-workflows/).

## Inspecting a Package

```bash
# Validate + inspect a local archive (unpacks it and checks the manifest)
cloacinactl package validate analytics-workflow-1.0.0.cloacina

# Manual — it's just a bzip2 tar
tar -tjf analytics-workflow-1.0.0.cloacina            # list contents
tar -xjOf analytics-workflow-1.0.0.cloacina \
    analytics-workflow-1.0.0/package.toml             # read the manifest
```

> `cloacinactl package validate <file>` operates on a **local** `.cloacina`
> archive. `cloacinactl package inspect <id>` is different — it queries a
> **running server** for an already-registered workflow by id, not a local file.

## Related Resources

- [Creating Your First Package](../../../how-to/service/creating-your-first-package/) — scaffold/validate/pack/upload with `package new`
- [Packaging Python Workflows](../../../how-to/embed/packaging-python-workflows/) — the full Python procedure
- [Packaged Triggers](../../../tutorials/embed/14-packaged-triggers/) — triggers in a packaged Python workflow
- [FFI System](../ffi-system/) — how the host calls into compiled packages
- [Packaged Workflow Architecture](../packaged-workflow-architecture/) — load/registration internals
