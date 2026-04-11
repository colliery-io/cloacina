---
title: "Package Manifest"
description: "Complete reference for the manifest.json schema in .cloacina packages"
weight: 45
---

# Package Manifest Reference

Every `.cloacina` package contains a `manifest.json` file that declares the package's contents, tasks, triggers, and runtime requirements. The reconciler reads this manifest to register workflows and triggers.

## Format Version

The current schema version is `"2"`. Older packages with version `"1"` are not supported.

```json
{
    "format_version": "2"
}
```

---

## Top-Level Structure

```json
{
    "format_version": "2",
    "package": { ... },
    "language": "python" | "rust",
    "python": { ... },
    "rust": { ... },
    "tasks": [ ... ],
    "triggers": [ ... ],
    "created_at": "2026-01-15T10:30:00Z",
    "signature": "base64-encoded-signature"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `format_version` | string | yes | Always `"2"` |
| `package` | object | yes | Package metadata (name, version, fingerprint, targets) |
| `language` | string | yes | `"python"` or `"rust"` |
| `python` | object | if language=python | Python runtime configuration |
| `rust` | object | if language=rust | Rust runtime configuration |
| `tasks` | array | yes | Task definitions (at least one required) |
| `triggers` | array | no | Trigger definitions (default: empty) |
| `created_at` | string | yes | RFC 3339 timestamp (e.g., `"2026-01-15T10:30:00Z"`) |
| `signature` | string | no | Package signature for verified deployments |

---

## Package Metadata

```json
{
    "package": {
        "name": "my-workflow",
        "version": "1.2.0",
        "description": "A data processing pipeline",
        "fingerprint": "sha256:a1b2c3d4e5f6...",
        "targets": ["linux-x86_64", "macos-arm64"]
    }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Package name (used as the workflow identifier) |
| `version` | string | yes | Semantic version |
| `description` | string | no | Human-readable description |
| `fingerprint` | string | yes | SHA-256 content hash for integrity verification |
| `targets` | array of string | yes | Supported platforms |

### Supported Targets

| Target | Platform |
|--------|----------|
| `linux-x86_64` | Linux on x86-64 |
| `linux-arm64` | Linux on ARM64 (aarch64) |
| `macos-x86_64` | macOS on Intel |
| `macos-arm64` | macOS on Apple Silicon |

Python packages are platform-independent and should list all targets. Rust packages must list only the platforms for which the shared library was compiled.

---

## Language-Specific Configuration

### Python Runtime

Required when `language` is `"python"`.

```json
{
    "python": {
        "requires_python": ">=3.10",
        "entry_module": "workflow.tasks"
    }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `requires_python` | string | yes | PEP 440 version specifier (e.g., `">=3.10"`) |
| `entry_module` | string | yes | Python module to import for task/trigger discovery |

Imported by the loader at package load time for task and trigger discovery.

### Rust Runtime

Required when `language` is `"rust"`.

```json
{
    "rust": {
        "library_path": "lib/libworkflow.so"
    }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `library_path` | string | yes | Relative path to the compiled `cdylib` within the archive |

---

## Task Definitions

Every package must define at least one task. Task IDs must be unique within the package.

```json
{
    "tasks": [
        {
            "id": "extract",
            "function": "workflow.tasks:extract_data",
            "dependencies": [],
            "description": "Extract data from source",
            "retries": 3,
            "timeout_seconds": 300
        },
        {
            "id": "transform",
            "function": "workflow.tasks:transform_data",
            "dependencies": ["extract"],
            "description": "Transform extracted data",
            "retries": 0,
            "timeout_seconds": null
        }
    ]
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | yes | — | Unique task identifier within the package |
| `function` | string | yes | — | Callable path (format depends on language) |
| `dependencies` | array of string | no | `[]` | IDs of tasks that must complete first |
| `description` | string | no | — | Human-readable description |
| `retries` | integer | no | `0` | Automatic retry count on failure |
| `timeout_seconds` | integer | no | — | Maximum execution time (null = no limit) |

### Function Path Formats

**Python**: `"module.path:function_name"` — the module path and function name separated by a colon. The loader resolves this relative to the package root.

**Rust**: `"symbol_name"` — the FFI symbol name in the compiled shared library. No colon separator.

### Dependency Rules

- Dependencies must reference task IDs within the same package
- Circular dependencies are rejected at validation time
- Tasks with no dependencies run first (root tasks)

---

## Trigger Definitions

Triggers are optional. When present, they declare polling triggers that the daemon's TriggerScheduler manages.

```json
{
    "triggers": [
        {
            "name": "inbox_watcher",
            "trigger_type": "python",
            "workflow": "data_ingest",
            "poll_interval": "5s",
            "allow_concurrent": false,
            "config": {
                "path": "/data/inbox/",
                "pattern": "*.parquet"
            }
        }
    ]
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | yes | — | Unique trigger name (must match decorator name) |
| `trigger_type` | string | yes | — | Trigger implementation type |
| `workflow` | string | yes | — | Workflow to fire (package name or task ID) |
| `poll_interval` | string | yes | — | Duration between polls |
| `allow_concurrent` | boolean | no | `false` | Allow parallel executions |
| `config` | object | no | — | Arbitrary JSON for trigger-specific settings |

### Trigger Types

The `trigger_type` field is a string discriminator. Common values:

| Type | Description |
|------|-------------|
| `"python"` | Implemented via `@cloaca.trigger` decorator |
| `"rust"` | Implemented via `#[trigger]` attribute macro |
| `"file_watch"` | File system monitoring |
| `"http_poll"` | HTTP endpoint polling |
| `"webhook"` | Inbound webhook listener |

Arbitrary strings are accepted. Resolution is by the `name` field, not the `trigger_type` value.

### Poll Interval Format

Duration strings follow the pattern `{number}{unit}`:

| Unit | Suffix | Example |
|------|--------|---------|
| Milliseconds | `ms` | `"100ms"` |
| Seconds | `s` | `"5s"` |
| Minutes | `m` | `"2m"` |
| Hours | `h` | `"1h"` |

### Workflow Reference

The `workflow` field can reference:
- The **package name** (most common — fires the full workflow)
- A **task ID** within the package (fires just that task)

---

## Validation Rules

The manifest is validated when the package is loaded. The following rules are enforced:

| Rule | Error |
|------|-------|
| `format_version` must be `"2"` | `InvalidFormatVersion` |
| Language-specific config must be present | `MissingRuntime` |
| All targets must be in the supported set | `UnsupportedTarget` |
| At least one task required | `NoTasks` |
| No duplicate task IDs | `DuplicateTaskId` |
| All dependencies reference valid task IDs | `InvalidDependency` |
| Python functions must contain `:` separator | `InvalidFunctionPath` |
| No duplicate trigger names | `DuplicateTriggerName` |
| Trigger `workflow` must reference package name or task ID | `InvalidTriggerWorkflow` |
| Trigger `poll_interval` must be a valid duration | `InvalidTriggerPollInterval` |

---

## Complete Examples

### Python Workflow Package

```json
{
    "format_version": "2",
    "package": {
        "name": "data-pipeline",
        "version": "2.1.0",
        "description": "ETL pipeline for analytics data",
        "fingerprint": "sha256:e3b0c44298fc1c149afbf4c8996fb924...",
        "targets": ["linux-x86_64", "linux-arm64", "macos-x86_64", "macos-arm64"]
    },
    "language": "python",
    "python": {
        "requires_python": ">=3.10",
        "entry_module": "data_pipeline.tasks"
    },
    "tasks": [
        {
            "id": "extract",
            "function": "data_pipeline.tasks:extract_data",
            "dependencies": [],
            "description": "Extract data from sources",
            "retries": 3,
            "timeout_seconds": 300
        },
        {
            "id": "transform",
            "function": "data_pipeline.tasks:transform_data",
            "dependencies": ["extract"],
            "retries": 0
        },
        {
            "id": "load",
            "function": "data_pipeline.tasks:load_data",
            "dependencies": ["transform"],
            "retries": 2,
            "timeout_seconds": 600
        }
    ],
    "triggers": [
        {
            "name": "new_data_trigger",
            "trigger_type": "python",
            "workflow": "data-pipeline",
            "poll_interval": "30s",
            "allow_concurrent": false
        }
    ],
    "created_at": "2026-01-15T10:30:00Z"
}
```

### Rust Workflow Package

In Rust packages, the FFI entry point is typically a single symbol (`cloacina_execute_task`); the runtime dispatches to the correct task by ID.

```json
{
    "format_version": "2",
    "package": {
        "name": "analytics-workflow",
        "version": "0.3.0",
        "fingerprint": "sha256:abc123def456...",
        "targets": ["linux-x86_64"]
    },
    "language": "rust",
    "rust": {
        "library_path": "lib/libanalytics_workflow.so"
    },
    "tasks": [
        {
            "id": "extract_data",
            "function": "cloacina_execute_task",
            "dependencies": []
        },
        {
            "id": "validate_data",
            "function": "cloacina_execute_task",
            "dependencies": ["extract_data"]
        }
    ],
    "triggers": [
        {
            "name": "file_watcher",
            "trigger_type": "file_watch",
            "workflow": "analytics-workflow",
            "poll_interval": "5s",
            "config": { "path": "/inbox/" }
        },
        {
            "name": "api_poller",
            "trigger_type": "http_poll",
            "workflow": "analytics-workflow",
            "poll_interval": "1m",
            "allow_concurrent": true,
            "config": { "url": "https://api.example.com/status" }
        }
    ],
    "created_at": "2026-03-20T14:00:00Z",
    "signature": "base64-ed25519-signature..."
}
```

## See Also

- [Package Format]({{< ref "/platform/explanation/package-format" >}}) — how `.cloacina` archives are structured
- [Package Signing]({{< ref "/platform/how-to-guides/security/package-signing" >}}) — signing and verifying packages
- [Packaged Workflows Tutorial]({{< ref "/workflows/tutorials/service/07-packaged-workflows" >}}) — building your first package
