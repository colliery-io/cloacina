---
title: "Packaging CLI"
description: "Build and distribute Python workflow packages"
weight: 90
reviewer: "dstorey"
review_date: "2025-03-13"
---

# Packaging CLI

The `cloaca build` command (and its `cloacinactl package build` superset equivalent) packages Python workflows into distributable `.cloacina` archives.

## cloaca build

```bash
cloaca build [OPTIONS]
```

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-o`, `--output` | path | `.` | Output directory for the package |
| `--target` | string | current platform | Target platform (repeatable) |
| `--dry-run` | flag | | Show what would be built without building |
| `--verbose` | flag | | Enable verbose output |

### Supported Targets

- `linux-x86_64`
- `linux-arm64`
- `macos-x86_64`
- `macos-arm64`

### Examples

```bash
# Build for current platform
cloaca build -o dist/

# Build for specific target
cloaca build -o dist/ --target linux-x86_64

# Multi-target build
cloaca build -o dist/ --target linux-x86_64 --target macos-arm64

# Preview build
cloaca build -o dist/ --dry-run --verbose
```

## cloacinactl package build

The `cloacinactl` superset provides the same build functionality via an embedded Python interpreter (PyO3):

```bash
cloacinactl package build [OPTIONS]
```

Options are identical to `cloaca build`. The advantage of `cloacinactl` is that it includes build capabilities without requiring a separate `cloaca` Python package installation.

## Project Configuration

### pyproject.toml

The `[tool.cloaca]` section configures the build:

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

| Field | Required | Description |
|-------|----------|-------------|
| `entry_module` | Yes | Dotted module path containing `@task` decorated functions |

## Task Discovery

The build process uses AST-based static analysis to discover tasks without importing user code. Functions decorated with `@task` are scanned for these keyword arguments:

| Keyword | Type | Default | Description |
|---------|------|---------|-------------|
| `id` | str | function name | Unique task identifier |
| `dependencies` | list[str] | `[]` | Task IDs that must complete first |
| `description` | str | None | Human-readable description |
| `retries` | int | `0` | Automatic retry count |
| `timeout_seconds` | int | None | Maximum execution time |

## Manifest Format (V2)

The generated `manifest.json` follows the V2 schema:

```python
from cloaca.manifest import Manifest, PackageInfo, PythonRuntime, TaskDefinition

manifest = Manifest(
    format_version="2",
    package=PackageInfo(
        name="my-workflow",
        version="1.0.0",
        targets=["linux-x86_64"],
    ),
    language="python",
    python=PythonRuntime(
        requires_python=">=3.11",
        entry_module="workflow.tasks",
    ),
    tasks=[
        TaskDefinition(
            id="fetch-data",
            function="workflow.tasks:fetch_data",
            dependencies=[],
            retries=3,
        ),
    ],
)
```

### Manifest Python API

#### Manifest

| Method | Description |
|--------|-------------|
| `validate_all()` | Run all validation checks |
| `validate_targets()` | Validate target platforms |
| `validate_tasks()` | Validate task IDs, dependencies, function paths |
| `to_json()` | Serialize to JSON string |
| `write_to_file(path)` | Write manifest to file |
| `Manifest.read_from_file(path)` | Read manifest from file |

#### TaskDefinition

| Field | Type | Description |
|-------|------|-------------|
| `id` | str | Task identifier |
| `function` | str | `"module.path:function_name"` format |
| `dependencies` | list[str] | Upstream task IDs |
| `description` | str or None | Human-readable description |
| `retries` | int | Retry count (default 0) |
| `timeout_seconds` | int or None | Timeout in seconds |

#### PythonRuntime

| Field | Type | Description |
|-------|------|-------------|
| `requires_python` | str | PEP 440 version specifier |
| `entry_module` | str | Dotted module path for task discovery |

## Dependency Vendoring

Dependencies listed in `pyproject.toml` are vendored into the package using `uv`:

1. **Resolution**: `uv pip compile` resolves pinned versions with SHA-256 hashes
2. **Download**: `uv pip download` fetches platform-specific wheels
3. **Extraction**: Wheels extracted into `vendor/` directory
4. **Lock file**: `requirements.lock` written with pinned versions

### VendorResult

| Field | Type | Description |
|-------|------|-------------|
| `vendor_dir` | Path | Directory containing vendored packages |
| `dependencies` | list[ResolvedDependency] | Resolved dependency metadata |
| `lock_file` | Path or None | Path to requirements.lock |

### ResolvedDependency

| Field | Type | Description |
|-------|------|-------------|
| `name` | str | Package name |
| `version` | str | Pinned version |
| `hash_sha256` | str | SHA-256 hash of the wheel |

## Package Archive Structure

```
my-workflow.cloacina (tar.gz)
├── manifest.json
├── workflow/
│   ├── __init__.py
│   └── tasks.py
├── vendor/
│   ├── <vendored packages>/
│   ├── VENDORED.txt
│   └── requirements.lock
```

## See Also

- **[Packaging Workflows Tutorial]({{< ref "/python-bindings/tutorials/09-packaging-workflows/" >}})** - Step-by-step guide
- **[Packaging for Multiple Platforms]({{< ref "/python-bindings/how-to-guides/packaging-for-multiple-platforms/" >}})** - Multi-target builds
- **[Package Format]({{< ref "/explanation/package-format/" >}})** - Archive format specification
- **[Packaged Workflow Architecture]({{< ref "/explanation/packaged-workflow-architecture/" >}})** - System design
