---
title: "Packaging Python Workflows"
description: "How to package Python workflows as .cloacina archives for deployment to the daemon or server"
weight: 40
---

# Packaging Python Workflows

This guide explains how to turn a Python workflow into a `.cloacina` package that
can be deployed to the Cloacina daemon or server without shipping your source
environment.

> **One canonical format.** A `.cloacina` package is described by a top-level
> **`package.toml`** and a Python module tree under a **`workflow/`** directory.
> This is the only format the server/daemon accept. (Older docs that used
> `pyproject.toml` + `[tool.cloaca]` + a hand-written `manifest.json` are
> obsolete.)

## Prerequisites

- A working Python workflow (see [Python tutorials]({{< ref "/python/workflows/tutorials/" >}}))
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
- Tasks/triggers must register at import time (module level, inside a
  `WorkflowBuilder` context).

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
| `requires_python` | no | e.g. `">=3.11"`. |
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

@cloaca.task(id="extract", dependencies=[])
def extract(context):
    # cloaca.var() reads from CLOACINA_VAR_ env vars at runtime
    # See "External Configuration" section below
    source = cloaca.var("DATA_SOURCE")
    context.set("raw_data", fetch_from(source))  # Replace with your data function
    return context

@cloaca.task(id="transform", dependencies=["extract"])
def transform(context):
    raw = context.get("raw_data")
    context.set("clean_data", clean(raw))  # Replace with your transform logic
    return context

@cloaca.task(id="load", dependencies=["transform"])
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

{{< hint type="important" title="Bare decorators — not WorkflowBuilder" >}}
In a **packaged** workflow, do **not** wrap tasks in
`with cloaca.WorkflowBuilder(...)`. That context manager is for running a
workflow in-process (it pushes its own workflow context); inside a package it
shadows the loader's context, so the loader finds no tasks under your
`workflow_name` and rejects the package with *"Empty package: registered no
tasks"*. Use bare `@cloaca.task` decorators and let `workflow_name` in
`package.toml` name the workflow.

All decorators **must** run at import time (module level) — if registration is
gated behind `if __name__ == "__main__"`, the loader won't find the tasks.
{{< /hint >}}

## Step 3: Vendoring Dependencies

If your workflow uses third-party libraries not available on the target host,
place them in a `vendor/` directory at the package root. The loader adds both the
`workflow/` directory and `vendor/` to `sys.path` before importing.

```
data-pipeline/
├── package.toml
├── workflow/
│   └── data_pipeline/
│       └── tasks.py
└── vendor/
    └── requests/
        └── __init__.py
```

{{< hint type="warning" title="Stdlib Shadowing" >}}
The loader **rejects** any package that shadows Python standard library modules.
You cannot vendor modules named `os`, `sys`, `json`, `pathlib`, `subprocess`, or
other stdlib names. This is a security measure to prevent code injection.

Blocked modules include: `os`, `sys`, `subprocess`, `shutil`, `socket`, `http`,
`urllib`, `ctypes`, `importlib`, `pathlib`, `io`, `json`, `pickle`, `marshal`,
`code`, `codeop`, `compile`, `compileall`, `builtins`, `signal`,
`multiprocessing`, `threading`, `tempfile`, `glob`, `fnmatch`.
{{< /hint >}}

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

Then pack. `cloacinactl package pack` reads `package.toml`, runs the same
validation, and emits the `.cloacina` archive:

```bash
cloacinactl package pack . --out data-pipeline-1.0.0.cloacina
```

A `.cloacina` package is a bzip2-compressed tar archive of `package.toml` + the
`workflow/` tree (+ `vendor/` if present), under a single `<name>-<version>/`
top-level directory:

```
data-pipeline-1.0.0.cloacina
└── data-pipeline-1.0.0/
    ├── package.toml
    ├── workflow/
    │   └── data_pipeline/
    │       └── tasks.py
    └── vendor/                 # if present
```

{{< hint type="info" title="Packing fails fast on a bad layout" >}}
If the module tree isn't under `workflow/`, or `entry_module` doesn't resolve to
a module there, `pack` errors immediately — you don't have to wait for the server
to reject the upload. The same parse rejects `package_type` and
`[[metadata.triggers]]`.
{{< /hint >}}

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

See [Variable Registry]({{< ref "/workflows/how-to-guides/variable-registry" >}}) for details.

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

### "stdlib shadowing detected"

Remove or rename any vendored module that conflicts with the Python standard
library (see the blocked list above).

### "no tasks registered"

Tasks must be defined inside a `WorkflowBuilder` context at module level. Tasks
defined inside a function or behind a conditional won't be discovered on import.

## See Also

- [Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}}) — scaffold this layout with `cloacinactl package new`
- [Package Format]({{< ref "/platform/explanation/package-format" >}}) — `.cloacina` archive structure
- [Running the Daemon]({{< ref "/embed/how-to/running-the-daemon" >}}) — deploying to the local scheduler
- [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}}) — deploying to the HTTP server
