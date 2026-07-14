# Python Packaged Workflow

**The canonical Python packaged example** — the Python peer of
[`simple-packaged`](../simple-packaged). You author tasks in Python, pack the
source into a `.cloacina` archive, and hand it to a running server, which loads
and executes it. Same primary interface as Rust:

```
pack  →  upload  →  compile  →  reconcile  →  execute  →  observe
```

This example is a three-task pipeline:

```
collect_data → process_data → generate_report
```

## Layout

| File | Role |
|---|---|
| `package.toml` | Package manifest — name, version, and the workflow it exposes (`data_pipeline`) |
| `workflow/data_pipeline/tasks.py` | The tasks: bare `@cloaca.task` decorators |
| `workflow/data_pipeline/__init__.py` | Marks the module a package (empty) |

No `Cargo.toml`, no build script — a Python package carries source only; the
server's compiler skips cargo for `language = "python"` and the reconciler
imports the module via the embedded Python runtime. The manifest is minimal:
the loader infers `language = "python"` and the entry module from the
`workflow/<name>/` layout. This is exactly what `cloacinactl package new
--language python` scaffolds.

## How tasks are authored

```python
import cloaca

@cloaca.task(id="collect_data", dependencies=[])
def collect_data(context):
    context.set("raw_records", 1000)
    return context

@cloaca.task(id="process_data", dependencies=["collect_data"])
def process_data(context):
    context.set("processed_records", context.get("raw_records"))
    return context
```

Tasks take the execution `context`, read/write it with `context.get(...)` /
`context.set(...)`, and return it. Dependencies are declared per-task; the
engine derives the execution order. **Do not** wrap these in a
`WorkflowBuilder` — that's the in-process (embedded) form; a packaged workflow
registers its tasks on import and the loader builds the workflow from
`workflow_name`.

## Run it

Automated as `angreal demos features python-packaged` (the CI examples lane
runs exactly that).

### 1. Stack + CLI

```bash
angreal ui up
cloacinactl config profile set demo http://localhost:8080 \
    --api-key clk_demo_public_key_0003 --tenant public --default
```

### 2. Pack + upload

```bash
cloacinactl package pack . --out data-pipeline.cloacina
cloacinactl package upload data-pipeline.cloacina
cloacinactl package list   # wait for build_status: success
```

### 3. Execute

```bash
cloacinactl workflow run data_pipeline
```

### 4. Observe

```bash
cloacinactl execution list --workflow data_pipeline
cloacinactl execution status <execution-id>
```

Or watch it in the web UI at <http://localhost:8080> — executions, task states,
and the workflow DAG.
