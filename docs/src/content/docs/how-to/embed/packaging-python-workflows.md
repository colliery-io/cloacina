---
title: "Packaging Python Workflows"
description: "How to package Python workflows as .cloacina archives for deployment to the daemon or server"
sidebar:
  order: 40
---

This guide explains how to turn a Python workflow into a `.cloacina` package that
can be deployed to the Cloacina daemon or server without shipping your source
environment.

> **One canonical format.** A `.cloacina` package is described by a top-level
> **`package.toml`** and a Python module tree under a **`workflow/`** directory.
> This is the only format the server/daemon accept. (Older docs that used
> `[tool.cloaca]` in `pyproject.toml` + a hand-written `manifest.json` are
> obsolete. A `pyproject.toml` in the package now has one job: it declares the
> package's dependencies, see Step 3.)

## Prerequisites

- A working Python workflow (see [Python tutorials](../../../tutorials/embed/))
- `cloaca` installed (`pip install cloaca`)
- Understanding of the daemon or server deployment model

## Project Layout

The package root contains a `package.toml` and a `workflow/` directory holding
your module tree:

```
data-pipeline/
├── package.toml             # package + metadata (REQUIRED, at the root)
└── workflow/                # REQUIRED directory — your modules live here
    └── data_pipeline/       # your entry module package
        ├── __init__.py
        └── tasks.py         # @cloaca.task decorators here
```

Key requirements:
- The module tree **must** live under `workflow/`. A top-level module (e.g.
  `data-pipeline/data_pipeline/`) fails to load with
  `Missing workflow source directory`.
- `entry_module` in `package.toml` is the dotted path **relative to `workflow/`**
  (here, `data_pipeline.tasks`).
- Tasks/triggers must register at **import time** (module level) with **bare
  `@cloaca.task` / `@cloaca.trigger` decorators** — do **not** wrap them in a
  `WorkflowBuilder` context. The loader establishes the workflow context (from
  `workflow_name`) before importing your module; a `WorkflowBuilder` here would
  shadow it and the package would load with zero tasks. (`WorkflowBuilder` is only
  for the in-process test path in Step 4.)

## Step 1: Write `package.toml`

```toml
[package]
name = "data-pipeline"
version = "1.0.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "python"
workflow_name = "data_pipeline"
entry_module = "data_pipeline.tasks"
description = "ETL pipeline for analytics"
author = "you@example.com"
requires_python = ">=3.10"
```

The `[metadata]` table is a **closed schema** — unknown keys are rejected at
upload. The accepted keys are:

| Field | Required | Description |
|-------|----------|-------------|
| `language` | yes | `"python"` (or `"rust"`). |
| `workflow_name` | for workflows | The `WorkflowBuilder(name=...)` value. |
| `graph_name` | for computation graphs | The graph name (instead of `workflow_name`). |
| `entry_module` | yes | Dotted module path **relative to `workflow/`** that the loader imports. |
| `description` | no | Human-readable description. |
| `author` | no | Author/owner. |
| `requires_python` | no | A version specifier, e.g. `">=3.11"`. Checked against the Python that runs the package: at `package validate`, at upload and at load. |
| `vendored` | written by `package build` | The `[metadata.vendored]` table: the Python version and the platforms of `vendor/`. See Step 3. Do not write it by hand. |
| `reaction_mode` / `input_strategy` | computation graphs | `when_any`/`when_all`, `latest`/`sequential`. |
| `accumulators` | computation graphs | `[[metadata.accumulators]]` entries (stream/kafka sources). |

> **Do not** add a `package_type` key or `[[metadata.triggers]]` table — both are
> rejected by the parser. Triggers are declared in code via `@cloaca.trigger`,
> not in the manifest.

## Step 2: Write Your Workflow

In your entry module (`workflow/data_pipeline/tasks.py`), declare tasks with
**bare `@cloaca.task` decorators at module level** — no `WorkflowBuilder`:

```python
import cloaca

@cloaca.task(dependencies=[])
def extract(context):
    # cloaca.var() reads from CLOACINA_VAR_ env vars at runtime
    # See "External Configuration" section below
    source = cloaca.var("DATA_SOURCE")
    context.set("raw_data", fetch_from(source))  # Replace with your data function
    return context

@cloaca.task(dependencies=["extract"])
def transform(context):
    raw = context.get("raw_data")
    context.set("clean_data", clean(raw))  # Replace with your transform logic
    return context

@cloaca.task(dependencies=["transform"])
def load(context):
    dest = cloaca.var("WAREHOUSE_URL")
    write_to(dest, context.get("clean_data"))  # Replace with your load logic
    return context
```

The tasks are grouped into a workflow by the `workflow_name` you set in
`package.toml` — the loader establishes that workflow context before importing
`entry_module`, and your bare decorators register into it. Make
`workflow/data_pipeline/__init__.py` import the entry module (so importing the
package registers the tasks), or point `entry_module` directly at the file that
defines them (`data_pipeline.tasks`, as above).

:::caution[Bare decorators — not WorkflowBuilder]
In a **packaged** workflow, do **not** wrap tasks in
`with cloaca.WorkflowBuilder(...)`. That context manager is for running a
workflow in-process (it pushes its own workflow context); inside a package it
shadows the loader's context, so the loader finds no tasks under your
`workflow_name` and rejects the package with *"Empty package: registered no
tasks"*. Use bare `@cloaca.task` decorators and let `workflow_name` in
`package.toml` name the workflow.

All decorators **must** run at import time (module level) — if registration is
gated behind `if __name__ == "__main__"`, the loader won't find the tasks.
:::

## Step 3: Vendor the Dependencies

A package carries its third-party libraries with it. The agent that runs the
package installs nothing: it puts the package's `vendor/` folder on `sys.path`.
`cloacinactl package build` fills that folder.

### Declare the dependencies

Add a `pyproject.toml` at the package root and list the dependencies in
`[project].dependencies`:

```toml
[project]
name = "data-pipeline"
version = "1.0.0"
dependencies = [
    "requests>=2.31",
    "pyarrow>=18",
]
```

This list is the source of truth. A package with no `pyproject.toml`, or with no
`[project].dependencies`, has nothing to vendor, and the command says so.

### Build

```bash
cloacinactl package build .
```

The command needs [`uv`](https://docs.astral.sh/uv/) on `PATH` (or its path in
`CLOACINA_UV`). It runs on your machine; the server and the compiler download
no dependency. It does three things:

1. It resolves the dependencies to pinned versions with hashes and writes them
   to `requirements.lock` in the package root (`uv pip compile --universal`).
2. For each target platform it installs the wheels into `vendor/<platform>/`
   (`uv pip install --target`, with `--only-binary :all:` and
   `--require-hashes`).
3. It records the Python version and the platforms in `[metadata.vendored]` of
   `package.toml`.

```
data-pipeline/
├── package.toml             # now with [metadata.vendored]
├── pyproject.toml           # the dependencies
├── requirements.lock        # the pinned versions and hashes
├── workflow/
│   └── data_pipeline/
│       └── tasks.py
└── vendor/
    ├── aarch64-linux/       # wheels for 64-bit ARM Linux agents
    │   ├── requests/
    │   └── pyarrow/
    └── x86_64-linux/        # wheels for 64-bit x86 Linux agents
        ├── requests/
        └── pyarrow/
```

```toml
# Written by `cloacinactl package build`: what is in vendor/. Do not edit.
[metadata.vendored]
python_version = "3.11"
platforms = ["aarch64-linux", "x86_64-linux"]
native = true
lock_file = "requirements.lock"
```

`cloacinactl package publish` runs `package build` first, so a publish vendors.
Commit `pyproject.toml` and `requirements.lock`. `vendor/` can stay out of
version control: the build writes it again.

### Choose the target

The default target is the official agent image: Python 3.11 on Linux, for both
CPU architectures the image is built for.

| Flag | Default | Meaning |
|------|---------|---------|
| `--python-version <MAJOR.MINOR>` | `3.11` | The Python of the agents that run the package. |
| `--arch <x86_64\|aarch64\|both>` | `both` | The CPU architecture of the Linux agents. |
| `--platform <ARCH-OS>` | — | A platform to vendor for, repeatable: `aarch64-linux`, `x86_64-linux`, `aarch64-macos`, `x86_64-macos`, `x86_64-windows`. It replaces the platforms of `--arch`. |
| `--locked` | off | Install from the `requirements.lock` in the package as it is. Without it the build resolves the versions again. |
| `--no-vendor` | off | Do not touch `vendor/`. |

To run a package with native dependencies on a daemon on a Mac as well as on
Linux agents, name each platform:

```bash
cloacinactl package build . \
  --platform aarch64-linux --platform x86_64-linux --platform aarch64-macos
```

A platform name is `<arch>-<os>`, the same name an agent reports for itself.

### What the build refuses

- **A dependency with no wheel for the target.** A package holds built wheels
  only; the build compiles no source distribution. The error names the
  dependency, the platform and the Python version. Pin a version that has a
  wheel, or remove the dependency.
- **`uv` is not installed.** The error says how to install it.
- **A `vendor/` folder with files the command did not write.** The command
  writes only `vendor/<platform>/` folders. Move hand-copied files out and
  declare them in `pyproject.toml`, or pass `--no-vendor`.
- **A `requires_python` that excludes the target Python.**

### How a package is matched to an agent

- **Python version.** `requires_python` must allow the Python that runs the
  package, and vendored dependencies must be for the same `major.minor`.
  `cloacinactl package validate` and `package pack` check it (against
  `--python-version`, else the recorded version, else 3.11). The server checks
  it at upload against its own Python and refuses the upload. The loader checks
  it against the interpreter of the process that loads the package.
- **Platform.** The loader uses `vendor/<its own platform>/`. A package whose
  dependencies are all pure Python (`native = false`) loads on any platform,
  from one of its folders. A package with compiled dependencies and no folder
  for the loading platform is refused, with an error that names the platforms
  it has and the one it needs.

### Two packages, one library, different versions

One agent process has one Python interpreter. A module is imported one time,
and every package in the process gets that copy. So when a package loads, each
top-level module of its vendor folder is compared with the packages loaded
before it. If another package vendors the same module at a different version,
the load of the new package fails:

```
package `reports` (tenant `acme`) vendors the module `yaml` at version 6.0.2
(PyYAML 6.0.2), and package `billing` (tenant `acme`), loaded before it in this
process, vendors `yaml` at version 5.4.1 (PyYAML 5.4.1), which is already
imported. One Python interpreter holds one version of a module, so package
`reports` (tenant `acme`) is not loaded. Vendor the same version in both
packages, or run them on different agents.
```

The package loaded first keeps running. Two packages that vendor the same
version share one copy. To run both versions, give the packages to different
agents.

A process keeps the modules it has imported, and the vendor folders on its
`sys.path`, until it stops. To move two packages to a new version of a shared
library, publish both with the new version, then restart the processes that
load them (the agents, and the server when it runs tasks itself).

The comparison reads the versions from the `.dist-info` folders that the build
writes. It does not cover a namespace package (a top-level folder shared by
several distributions, such as `google`), a `vendor/` folder with no
`.dist-info` folders, a shared library that two different modules each bundle,
or a new version of the same package that changes a vendored version.

### Size

The server accepts a package up to its upload limit: 100 MiB by default, set
with `--max-package-bytes` or `CLOACINA_MAX_PACKAGE_BYTES` on `cloacina-server`.
An upload over the limit is refused with status 413 and an error that names the
size of the upload and the limit. Each platform adds its own copy of the
dependencies; `--arch` vendors for one architecture.

### A `vendor/` folder filled by hand

A package with a `vendor/` folder and no `[metadata.vendored]` table still
loads: the loader puts `vendor/` itself on `sys.path`. Publish it with
`--no-vendor`. The loader refuses it when it holds a wheel or a compiled file
built for another platform.

:::caution[Stdlib Shadowing]
The loader **rejects** any package that shadows Python standard library modules.
You cannot vendor modules named `os`, `sys`, `json`, `pathlib`, `subprocess`, or
other stdlib names. This is a security measure to prevent code injection.

Blocked modules include: `os`, `sys`, `subprocess`, `shutil`, `socket`, `http`,
`urllib`, `ctypes`, `importlib`, `pathlib`, `io`, `json`, `pickle`, `marshal`,
`code`, `codeop`, `compile`, `compileall`, `builtins`, `signal`,
`multiprocessing`, `threading`, `tempfile`, `glob`, `fnmatch`.
:::

## Step 4: Test Before Packaging

To run a bare-decorator module in-process, supply the workflow context the
packaged loader would normally provide by wrapping the **import** in a
`WorkflowBuilder` with the same name as your `workflow_name`:

```python
import cloaca

# WorkflowBuilder here stands in for the loader's context (in-process only —
# it is NOT part of the packaged module).
with cloaca.WorkflowBuilder("data_pipeline"):
    import data_pipeline.tasks   # bare @cloaca.task decorators register here

runner = cloaca.DefaultRunner(":memory:")
try:
    result = runner.execute("data_pipeline", cloaca.Context())
    assert result.status == "completed"
finally:
    runner.shutdown()
```

## Step 5: Build the Package

Optionally check the layout first — `cloacinactl package validate` runs the same
schema + `workflow/`/`entry_module` checks as the server, against a source
directory or a packed archive, without uploading:

```bash
cloacinactl package validate .
```

Then pack. Run `cloacinactl package build .` first when the package has
dependencies (Step 3). `cloacinactl package pack` reads `package.toml`, runs the
same validation, and emits the `.cloacina` archive:

```bash
cloacinactl package pack . --out data-pipeline-1.0.0.cloacina
```

A `.cloacina` package is a bzip2-compressed tar archive of `package.toml` + the
`workflow/` tree (+ `pyproject.toml`, `requirements.lock` and `vendor/` if
present), under a single `<name>-<version>/` top-level directory:

```
data-pipeline-1.0.0.cloacina
└── data-pipeline-1.0.0/
    ├── package.toml
    ├── pyproject.toml          # if present
    ├── requirements.lock       # if present
    ├── workflow/
    │   └── data_pipeline/
    │       └── tasks.py
    └── vendor/                 # if present
        ├── aarch64-linux/
        └── x86_64-linux/
```

:::note[Packing fails fast on a bad layout]
If the module tree isn't under `workflow/`, or `entry_module` doesn't resolve to
a module there, `pack` errors immediately — you don't have to wait for the server
to reject the upload. The same parse rejects `package_type` and
`[[metadata.triggers]]`.
:::

<details>
<summary>Building the archive by hand (no <code>cloacinactl</code>)</summary>

The archive is a plain bzip2 tar, so you can build it with standard tools:

```bash
name=data-pipeline
version=1.0.0
prefix="$name-$version"

stage="$(mktemp -d)/$prefix"
mkdir -p "$stage"
cp package.toml "$stage/"
cp -R workflow "$stage/"
[ -d vendor ] && cp -R vendor "$stage/"

tar -cjf "$name-$version.cloacina" -C "$(dirname "$stage")" "$prefix"
```

</details>

## Step 6: Deploy

### To the Daemon

Copy the `.cloacina` file into one of the daemon's watched directories:

```bash
cp data-pipeline-1.0.0.cloacina ~/.cloacina/packages/
```

The daemon's reconciler detects it, extracts the archive, imports your
`entry_module`, registers the tasks, and registers the workflow.

### To the Server

Upload via the HTTP API (or `cloacinactl package upload`):

```bash
curl -X POST \
  -H "Authorization: Bearer $API_KEY" \
  -F "file=@data-pipeline-1.0.0.cloacina" \
  https://cloacina.example.com/v1/tenants/my_tenant/workflows
```

## External Configuration with var/var_or

Use `cloaca.var()` and `cloaca.var_or()` to read configuration at runtime instead
of hardcoding values:

```python
# Resolved from CLOACINA_VAR_DATA_SOURCE environment variable
source = cloaca.var("DATA_SOURCE")

# With a default fallback
timeout = cloaca.var_or("FETCH_TIMEOUT", "30")
```

Set the variables on the host where the daemon or server runs:

```bash
export CLOACINA_VAR_DATA_SOURCE=postgres://analytics:pass@host/warehouse
export CLOACINA_VAR_FETCH_TIMEOUT=60
```

See [Variable Registry](../variable-registry/) for details.

## Troubleshooting

### "Missing workflow source directory"

Your module tree must live under a `workflow/` directory at the package root. A
top-level module (`<pkg>/<module>/`) is rejected — move it to
`<pkg>/workflow/<module>/`.

### "unknown field `package_type`" (or another `[metadata]` key)

`[metadata]` is a closed schema. Remove `package_type`, `[[metadata.triggers]]`,
or any other key not in the table in Step 1.

### "entry_module not found"

`entry_module` is a dotted path **relative to `workflow/`**. Verify the directory
matches (e.g. `workflow/data_pipeline/tasks.py` for
`entry_module = "data_pipeline.tasks"`) and that `__init__.py` files exist.

### Import times out after 60 seconds

The loader enforces a 60-second import timeout. Move expensive module-level
initialization into task functions; the package is rejected with
`"Python workflow import timed out after 60s"` otherwise.

### "requires Python …, and this host runs Python …"

`requires_python` in `package.toml` excludes the Python of the process that
loads the package. Change `requires_python`, or run the package on agents with a
Python it allows.

### "has dependencies vendored for Python …"

The dependencies in `vendor/` were built for another Python `major.minor` than
the one that loads the package. Build again with
`cloacinactl package build --python-version <MAJOR.MINOR>`.

### "has native dependencies vendored for …, and none for this platform"

The package has compiled dependencies and no `vendor/<platform>/` folder for
the platform that loads it. Build again and name every platform it runs on
(`--arch both`, or `--platform` for each).

### "vendors the module … at version …, and package …"

Two packages in one agent process vendor different versions of one library. See
"Two packages, one library, different versions" in Step 3.

### "package too large"

The archive is larger than the server's upload limit. See "Size" in Step 3.

### "stdlib shadowing detected"

Remove or rename any vendored module that conflicts with the Python standard
library (see the blocked list above).

### "no tasks registered" (or "Empty package: registered no tasks")

Tasks must register at **import time** with **bare `@cloaca.task` decorators at
module level** — the loader builds the workflow context from `workflow_name`, so
you do *not* wrap them in a `WorkflowBuilder` (wrapping shadows the loader's
context and the package loads with zero tasks). Tasks defined inside a function or
behind an `if __name__ == "__main__"` guard also won't be discovered on import.
The `WorkflowBuilder` in Step 4 is strictly for running the module in-process; it
is not part of the package.

## See Also

- [Creating Your First Package](../../service/creating-your-first-package/) — scaffold this layout with `cloacinactl package new`
- [Package Format](../../../explanation/engine/package-format/) — `.cloacina` archive structure
- [Running the Daemon](../running-the-daemon/) — deploying to the local scheduler
- [Deploying the API Server](../../service/deploying-the-api-server/) — deploying to the HTTP server
