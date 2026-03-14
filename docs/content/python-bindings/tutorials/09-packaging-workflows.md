---
title: "09 - Packaging Workflows"
description: "Build and distribute Python workflows as .cloacina packages"
weight: 19
reviewer: "dstorey"
review_date: "2025-03-13"
---

# Packaging Workflows

In this tutorial, you'll learn how to package Python workflows as `.cloacina` archives for distribution and deployment. The `cloaca build` command handles the entire process: task discovery, dependency vendoring, and archive creation.

## Learning Objectives

- Understand the expected project structure for packaging
- Configure `pyproject.toml` with `[tool.cloaca]` settings
- Build `.cloacina` packages for one or more target platforms
- Understand how AST-based task discovery works
- Inspect the manifest v2 format and archive contents
- Know what happens when the server loads a Python package

## Prerequisites

- Completion of [Tutorial 01]({{< ref "/python-bindings/tutorials/01-first-python-workflow/" >}})
- `uv` installed (for dependency resolution)
- A Python project with `pyproject.toml`

## Time Estimate

15-20 minutes

## Overview

Python workflows can be packaged as `.cloacina` archives for distribution and deployment. The `cloaca build` command (or `cloacinactl package build`) handles the entire process: task discovery, dependency vendoring, and archive creation.

The resulting archive is a self-contained tar.gz file that includes your workflow code, vendored dependencies, and a manifest describing every task in the package.

## Project Structure

A packagable workflow project follows this layout:

```
my-workflow/
├── pyproject.toml
├── workflow/
│   ├── __init__.py
│   └── tasks.py
```

The `pyproject.toml` must include a `[tool.cloaca]` section that tells the build tool where to find your tasks:

```toml
[project]
name = "my-workflow"
version = "1.0.0"
requires-python = ">=3.11"
dependencies = [
    "requests>=2.28",
]

[tool.cloaca]
entry_module = "workflow.tasks"
```

The `entry_module` field points to the Python module that contains your `@task`-decorated functions. The build tool uses this as the starting point for task discovery.

## Task Discovery

When you run `cloaca build`, the tool performs **AST-based static discovery** on your entry module. This is an important design decision: it does **not** import or execute your code.

How it works:

1. The entry module file is parsed into an abstract syntax tree (AST)
2. The AST is scanned for functions decorated with `@task` (or `@cloaca.task`)
3. Decorator keyword arguments are extracted: `id`, `dependencies`, `description`, `retries`, `timeout_seconds`
4. Each discovered task is recorded with its function path in the format `"module.path:function_name"`

{{< hint type="info" title="Static Discovery" >}}
Because discovery is AST-based, your code is never imported during the build. This means import-time side effects, missing runtime dependencies, or platform-specific code won't cause build failures. The build tool only needs to parse valid Python syntax.
{{< /hint >}}

## Dependency Vendoring

The build process vendors all third-party dependencies into the archive so that packages are self-contained. This uses `uv pip compile` under the hood.

The vendoring process:

1. Reads `dependencies` from `pyproject.toml`
2. Runs `uv pip compile` for platform-specific dependency resolution
3. Downloads pre-built wheels only (`--only-binary :all:`)
4. Extracts wheels into a `vendor/` directory inside the archive
5. Generates `requirements.lock` with pinned versions and SHA-256 hashes

Supported target platforms:

| Platform | Target string |
|----------|---------------|
| Linux x86_64 | `linux-x86_64` |
| Linux ARM64 | `linux-arm64` |
| macOS x86_64 | `macos-x86_64` |
| macOS ARM64 | `macos-arm64` |

## Building a Package

Use `cloaca build` to create a `.cloacina` archive:

```bash
# Build for current platform
cloaca build -o my-workflow.cloacina

# Build for specific target(s)
cloaca build -o my-workflow.cloacina --target linux-x86_64

# Build for multiple targets
cloaca build -o my-workflow.cloacina --target linux-x86_64 --target macos-arm64

# Dry run (show what would be built without creating the archive)
cloaca build -o my-workflow.cloacina --dry-run
```

The `cloacinactl` CLI provides the same functionality as a subcommand:

```bash
# Using cloacinactl (superset, same functionality)
cloacinactl package build -o my-workflow.cloacina --target linux-x86_64
```

## Package Contents

The resulting `.cloacina` file is a tar.gz archive with this structure:

```
my-workflow.cloacina (tar.gz)
├── manifest.json
├── workflow/
│   ├── __init__.py
│   └── tasks.py
└── vendor/
    ├── requests/
    ├── urllib3/
    └── VENDORED.txt
```

- **`manifest.json`** -- describes the package metadata, language, and every task
- **`workflow/`** -- your source code, mirroring the project layout
- **`vendor/`** -- extracted wheel contents for all dependencies

## Manifest V2 Format

The manifest uses format version 2, which supports both Rust and Python packages via a language discriminator field. Here is an example manifest for a Python package:

```json
{
  "format_version": "2",
  "package": {
    "name": "my-workflow",
    "version": "1.0.0",
    "description": "Example workflow",
    "fingerprint": "sha256:...",
    "targets": ["linux-x86_64", "macos-arm64"]
  },
  "language": "python",
  "python": {
    "requires_python": ">=3.11",
    "entry_module": "workflow.tasks"
  },
  "tasks": [
    {
      "id": "fetch-data",
      "function": "workflow.tasks:fetch_data",
      "dependencies": [],
      "description": "Fetch raw data",
      "retries": 3,
      "timeout_seconds": 300
    }
  ],
  "created_at": "2025-03-13T..."
}
```

Key fields:

- **`format_version`** -- always `"2"` for the current format
- **`language`** -- `"python"` or `"rust"`, determines how the server loads the package
- **`python.entry_module`** -- the module path used at runtime to import tasks
- **`tasks[].function`** -- the fully qualified function path (`module:function`)
- **`package.fingerprint`** -- SHA-256 hash of the archive contents for integrity verification
- **`package.targets`** -- list of platforms this package was built for

{{< hint type="info" title="Manifest V2 Language Support" >}}
The manifest v2 format supports both Rust and Python packages. The `language` field acts as a discriminator that tells the server which loader to use. See the [package format explanation]({{< ref "/explanation/package-format/" >}}) for the full specification.
{{< /hint >}}

## Server-Side Loading

When the Cloacina server loads a Python `.cloacina` package, the following steps occur:

1. **Archive extraction** -- the tar.gz is extracted to a staging directory
2. **Manifest validation** -- `manifest.json` is parsed and validated (`format_version` must be `"2"`, `language` must be `"python"`)
3. **Path setup** -- the `vendor/` and `workflow/` directories are added to Python's `sys.path`
4. **Task import** -- tasks are imported from the `entry_module` via PyO3
5. **Registration** -- tasks are registered in the global task registry with namespace isolation to prevent collisions between packages

{{< hint type="info" title="Namespace Isolation" >}}
Each loaded package gets its own namespace in the task registry. This means two packages can both define a task with `id="fetch-data"` without conflict. Tasks are addressed as `package_name::task_id` when disambiguation is needed.
{{< /hint >}}

## Complete Example

Let's walk through packaging a data pipeline workflow from start to finish.

### 1. Define the Tasks

```python
# data_pipeline/tasks.py
import cloaca

@cloaca.task(id="fetch-data", description="Fetch raw data from source")
def fetch_data(context):
    """Fetch raw data and store it in the workflow context."""
    context.set("raw_data", [{"id": 1, "value": 10.5}])
    return context

@cloaca.task(
    id="validate-data",
    dependencies=["fetch-data"],
    description="Validate and filter raw data"
)
def validate_data(context):
    """Validate raw data, keeping only records with numeric values."""
    raw = context.get("raw_data")
    context.set("validated", [
        r for r in raw if isinstance(r.get("value"), (int, float))
    ])
    return context
```

### 2. Configure the Project

```toml
# pyproject.toml
[project]
name = "data-pipeline"
version = "1.0.0"
requires-python = ">=3.11"
dependencies = []

[tool.cloaca]
entry_module = "data_pipeline.tasks"
```

### 3. Build the Package

```bash
cloaca build -o data-pipeline.cloacina
```

You should see output similar to:

```
Discovering tasks in data_pipeline.tasks...
  Found task: fetch-data (data_pipeline.tasks:fetch_data)
  Found task: validate-data (data_pipeline.tasks:validate_data)
Resolving dependencies...
  No third-party dependencies to vendor.
Writing archive: data-pipeline.cloacina
  manifest.json
  data_pipeline/__init__.py
  data_pipeline/tasks.py
Package built successfully: data-pipeline.cloacina (2.1 KB)
```

### 4. Verify the Package

Use the dry run flag to inspect what the build would produce without creating the archive:

```bash
cloaca build -o data-pipeline.cloacina --dry-run
```

## What You've Learned

In this tutorial, you learned:

- **Project structure** required for packaging (`pyproject.toml` with `[tool.cloaca]`)
- **AST-based task discovery** that scans for `@task` decorators without importing code
- **Dependency vendoring** with `uv` for platform-specific wheel resolution
- **Building packages** with `cloaca build` for single or multiple targets
- **Manifest v2 format** with language-level discrimination for Python and Rust
- **Server-side loading** steps from archive extraction to task registration

## Next Steps

{{< button relref="/explanation/package-format/" >}}Package Format Explanation{{< /button >}}
{{< button relref="/explanation/packaged-workflow-architecture/" >}}Packaged Workflow Architecture{{< /button >}}

## Related Resources

- [Explanation: Package Format]({{< ref "/explanation/package-format/" >}}) -- Full specification of the `.cloacina` archive format and manifest schema
- [Explanation: Packaged Workflow Architecture]({{< ref "/explanation/packaged-workflow-architecture/" >}}) -- How packaged workflows are loaded, isolated, and executed
- [Tutorial 01: First Python Workflow]({{< ref "/python-bindings/tutorials/01-first-python-workflow/" >}}) -- Getting started with Python workflows
