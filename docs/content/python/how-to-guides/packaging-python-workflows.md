---
title: "Packaging Python Workflows"
description: "How to package Python workflows as .cloacina archives for deployment to the daemon or server"
weight: 40
---

# Packaging Python Workflows

This guide explains how to turn a Python workflow into a `.cloacina` package that can be deployed to the Cloacina daemon or server without shipping your source environment.

## Prerequisites

- A working Python workflow (see [Python tutorials]({{< ref "/python/tutorials/" >}}))
- `cloaca` installed (`pip install cloaca`)
- Understanding of the daemon or server deployment model

## Project Layout

Organize your workflow as a standard Python package:

```
my-workflow/
├── pyproject.toml
├── data_pipeline/
│   ├── __init__.py      # WorkflowBuilder + @task decorators here
│   └── helpers.py       # Optional helper modules
└── vendor/              # Optional vendored dependencies
    └── my_lib/
        └── __init__.py
```

The key requirements:
- Tasks and triggers must be registered via decorators when the entry module is imported
- The `[tool.cloaca]` section in `pyproject.toml` tells the packager which module to import

## Step 1: Configure pyproject.toml

Add the `[tool.cloaca]` section to your `pyproject.toml`:

```toml
[project]
name = "data-pipeline"
version = "1.0.0"
description = "ETL pipeline for analytics"
requires-python = ">=3.10"
dependencies = []

[tool.cloaca]
entry_module = "data_pipeline"
```

| Field | Required | Description |
|-------|----------|-------------|
| `entry_module` | yes | Python module imported by the loader for task/trigger discovery |

The `entry_module` is the dotted module path that the loader will `import`. When imported, all `@cloaca.task` and `@cloaca.trigger` decorators in that module fire, registering tasks and triggers.

## Step 2: Write Your Workflow

In your entry module (`data_pipeline/__init__.py`):

```python
import cloaca

with cloaca.WorkflowBuilder("data_pipeline") as builder:
    builder.description("ETL pipeline for analytics data")

    @cloaca.task(id="extract")
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

{{< hint type="important" title="Module-Level Registration" >}}
All `@cloaca.task` and `@cloaca.trigger` decorators **must** execute at import time (module level, inside a `WorkflowBuilder` context). The loader discovers tasks by importing your module — if registration is gated behind `if __name__ == "__main__"`, the tasks won't be found.
{{< /hint >}}

## Step 3: Vendoring Dependencies

If your workflow uses third-party libraries not available on the target host, place them in a `vendor/` directory at the package root. The loader adds both the workflow directory and `vendor/` to `sys.path` before importing.

```
my-workflow/
├── data_pipeline/
│   └── __init__.py
└── vendor/
    └── requests/
        └── __init__.py
```

{{< hint type="warning" title="Stdlib Shadowing" >}}
The loader **rejects** any package that shadows Python standard library modules. You cannot vendor modules named `os`, `sys`, `json`, `pathlib`, `subprocess`, or other stdlib names. This is a security measure to prevent code injection.

Blocked modules include: `os`, `sys`, `subprocess`, `shutil`, `socket`, `http`, `urllib`, `ctypes`, `importlib`, `pathlib`, `io`, `json`, `pickle`, `marshal`, `code`, `codeop`, `compile`, `compileall`, `builtins`, `signal`, `multiprocessing`, `threading`, `tempfile`, `glob`, `fnmatch`.
{{< /hint >}}

## Step 4: Test Before Packaging

Always verify your workflow runs correctly before packaging:

```python
import cloaca

runner = cloaca.DefaultRunner(":memory:")
try:
    result = runner.execute("data_pipeline", cloaca.Context())
    assert result.status == "completed"
finally:
    runner.shutdown()
```

## Step 5: Build the Package

The `.cloacina` package is a bzip2-compressed tar archive containing your Python modules and a `manifest.json`. Create the archive from your project directory:

```bash
# Create the archive structure
mkdir -p build/
cp -r data_pipeline/ build/data_pipeline/
cp -r vendor/ build/vendor/ 2>/dev/null || true

# Generate manifest.json (see Package Manifest Reference for the full schema)
# You can write this by hand or generate it from pyproject.toml
cat > build/manifest.json << 'EOF'
{
    "format_version": "2",
    "package": {
        "name": "data-pipeline",
        "version": "1.0.0",
        "fingerprint": "sha256:placeholder",
        "targets": ["linux-x86_64", "linux-arm64", "macos-x86_64", "macos-arm64"]
    },
    "language": "python",
    "python": {
        "requires_python": ">=3.10",
        "entry_module": "data_pipeline"
    },
    "tasks": [
        {"id": "extract", "function": "data_pipeline:extract", "dependencies": []},
        {"id": "transform", "function": "data_pipeline:transform", "dependencies": ["extract"]},
        {"id": "load", "function": "data_pipeline:load", "dependencies": ["transform"]}
    ],
    "created_at": "2026-01-15T10:30:00Z"
}
EOF

# Package as a bzip2 tar archive
cd build && tar -cjf ../data-pipeline-1.0.0.cloacina . && cd ..
```

The resulting archive contains:

```
data-pipeline-1.0.0.cloacina
├── manifest.json
├── data_pipeline/
│   ├── __init__.py
│   └── helpers.py
└── vendor/                 # If present
    └── ...
```

See the [Package Manifest Reference]({{< ref "/platform/reference/package-manifest" >}}) for the full `manifest.json` schema.

## Step 6: Deploy

### To the Daemon

Copy the `.cloacina` file into one of the daemon's watched directories:

```bash
cp data-pipeline-1.0.0.cloacina ~/.cloacina/packages/
```

The daemon's reconciler will detect it, extract the archive, import your entry module, validate tasks against the manifest, and register the workflow.

### To the Server

Upload via the HTTP API:

```bash
curl -X POST \
  -H "Authorization: Bearer $API_KEY" \
  -F "package=@data-pipeline-1.0.0.cloacina" \
  https://cloacina.example.com/v1/tenants/my_tenant/workflows
```

## External Configuration with var/var_or

Use `cloaca.var()` and `cloaca.var_or()` to read configuration at runtime instead of hardcoding values:

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

### Import times out after 60 seconds

The loader enforces a 60-second timeout on module import. If your entry module takes longer to import (e.g., heavy native extensions, expensive module-level computation), the package will be rejected with `"Python workflow import timed out after 60s"`. Move expensive initialization out of module scope and into task functions.

### "entry_module not found"

The `entry_module` in `[tool.cloaca]` must be a valid Python import path relative to the package root. Verify:
- The directory matches the dotted path (e.g., `data_pipeline` for `entry_module = "data_pipeline"`)
- An `__init__.py` exists in the module directory

### "stdlib shadowing detected"

Remove or rename any vendored module that conflicts with the Python standard library. See the blocked list above.

### "no tasks registered"

Tasks must be defined inside a `WorkflowBuilder` context at module level. If you define tasks inside a function or behind a conditional, they won't be discovered on import.

## See Also

- [Package Manifest Reference]({{< ref "/platform/reference/package-manifest" >}}) — full manifest schema
- [Package Format]({{< ref "/platform/explanation/package-format" >}}) — `.cloacina` archive structure
- [Running the Daemon]({{< ref "/platform/how-to-guides/running-the-daemon" >}}) — deploying to the local scheduler
- [Deploying the API Server]({{< ref "/platform/how-to-guides/deploying-the-api-server" >}}) — deploying to the HTTP server
