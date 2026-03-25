---
id: packaged-triggers-configurable
level: initiative
title: "Packaged Triggers — Configurable Event-Driven Workflow Activation via API"
short_code: "CLOACI-I-0044"
created_at: 2026-03-24T21:15:19.754297+00:00
updated_at: 2026-03-24T21:21:20.905769+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: packaged-triggers-configurable
---

# Packaged Triggers — Configurable Event-Driven Workflow Activation via API Initiative

Absorbs: T-0214 (backlog task). Establishes packaging patterns that I-0037 (Packaged Continuous Tasks) will reuse.

## Context

Triggers define **when** a workflow should run. Today, triggers can only be defined in Rust code or via the `@cloaca.trigger` Python decorator — both require the trigger to be compiled into the process. There's no way to package triggers with workflows or deploy them dynamically.

The natural model: **triggers are declared alongside the workflows they activate, packaged together in `.cloacina` archives, and auto-registered when the package loads.** This matches how the codebase already works — the `@cloaca.trigger` decorator binds a trigger to a workflow at definition time, and the Rust `Trigger` trait takes a `workflow_name` on registration.

The existing infrastructure is substantial:
- `Trigger` trait, `TriggerScheduler`, `TriggerConfig` — all working
- `trigger_schedules` + `trigger_executions` tables with full DAL — all working
- Python `@cloaca.trigger` decorator with `PythonTriggerWrapper` — working
- What's missing: manifest declaration, package loading, built-in types, operational API

## Goals & Non-Goals

**Goals:**
- Triggers declared in `.cloacina` package manifest (alongside tasks)
- Reconciler auto-registers triggers when package loads, unregisters on removal
- Built-in trigger types: webhook, http_poll, file_watch (configurable via manifest)
- Custom Rust triggers via `#[packaged_workflow]`-style FFI symbols
- Python triggers via `@cloaca.trigger` decorator packaged in Python `.cloacina` archives
- Operational API: list, enable/disable, view execution history (read + lifecycle, not create)
- Daemon support: triggers from directory-watched packages
- Full test coverage: unit, integration, soak, chaos
- Documentation: tutorial + API reference

**Non-Goals:**
- Standalone trigger creation via API (triggers come from packages, not API calls)
- Queue consumers (SQS, Kafka) — future plugin system
- Continuous scheduling triggers (separate system, I-0037)

## Detailed Design

### Package Manifest Extension

The ManifestV2 gains a `triggers` section:

```json
{
  "format_version": "2",
  "package": { "name": "my-workflow", "version": "1.0.0" },
  "language": "rust",
  "tasks": [ ... ],
  "triggers": [
    {
      "name": "on-webhook",
      "type": "webhook",
      "workflow": "data_processing",
      "config": { "secret": "optional-hmac-secret" }
    },
    {
      "name": "poll-api",
      "type": "http_poll",
      "workflow": "data_processing",
      "config": {
        "url": "https://api.example.com/status",
        "interval_secs": 60,
        "fire_when": { "status": 200 }
      }
    },
    {
      "name": "watch-files",
      "type": "file_watch",
      "workflow": "data_processing",
      "config": { "directory": "/data/inbox", "glob": "*.csv" }
    },
    {
      "name": "custom-check",
      "type": "custom",
      "workflow": "data_processing",
      "config": {}
    }
  ]
}
```

For Python packages, the `@cloaca.trigger` decorator emits trigger metadata that gets included in the manifest at build time.

For Rust packages, the `#[trigger]` macro emits FFI symbols for custom trigger implementations, and the manifest declares them.

### Built-in Trigger Types

Built-in types are instantiated from config — no user code needed:

**Webhook** — server exposes `POST /webhooks/{trigger_id}`. POST fires the workflow with request body as context. Optional HMAC secret validation.

**HTTP Poll** — polls external URL at configurable interval. Fires when response matches condition (status code, JSON path match).

**File Watch** (daemon mode only) — watches directory for files matching glob. Fires with file path in context.

**Custom** — loaded from FFI symbol in the package (Rust) or from `@cloaca.trigger` decorator (Python). The trigger's `poll()` implementation is user-authored.

### Reconciler Integration

When the reconciler loads a package:
1. Parse `triggers` from manifest
2. For each trigger:
   - Built-in type → instantiate from config
   - Custom type → load FFI symbol or Python function
3. Register with `TriggerScheduler` via existing `register_trigger()` API
4. Store in `trigger_schedules` table (already exists)

When a package is removed:
1. Find triggers registered from this package
2. Disable and delete from `trigger_schedules`
3. Unregister from `TriggerScheduler`

### Operational API (Read + Lifecycle)

Triggers are created by packages, not API. The API provides operational control:

```
GET    /triggers                      — list all trigger schedules
GET    /triggers/{id}                 — get trigger details
POST   /triggers/{id}/enable          — enable trigger
POST   /triggers/{id}/disable         — disable trigger
GET    /triggers/{id}/executions      — execution history
```

No POST/PUT for creation/update — that comes from uploading a new package version.

### Python Integration

```python
from cloaca import trigger, WorkflowBuilder

with WorkflowBuilder("my-pipeline") as wb:
    @trigger(workflow="my-pipeline", poll_interval="5s")
    def check_for_updates(context):
        # Custom poll logic
        if new_data_available():
            return TriggerResult.fire({"source": "api"})
        return TriggerResult.skip()

    @task(id="process")
    def process(ctx):
        ...
```

When `cloacinactl package build` runs on this Python project, the trigger decorator metadata is included in the manifest. The built package contains both the task and its trigger.

## Alternatives Considered

**Triggers as standalone API resources:** Create triggers via POST API independent of packages. Rejected — leads to orphaned triggers, unclear ownership, and doesn't match the "package is the unit of deployment" model. Operational enable/disable is still available.

**Triggers only in Rust (no built-in types):** Users must write Rust or Python for every trigger. Rejected — webhooks and HTTP polling are so common they should be configuration, not code.

**Separate trigger packages:** Triggers in their own `.cloacina` packages, independent of workflows. Rejected — a trigger without its workflow is meaningless. Ship them together.

## Implementation Plan

### Phase 1: Manifest + Built-in Types
- Extend ManifestV2 with `triggers` field (TriggerDefinitionV2)
- Implement WebhookTrigger, HttpPollTrigger, FileWatchTrigger structs
- Factory: `create_trigger_from_config(type, config) -> Box<dyn Trigger>`

### Phase 2: Reconciler + Package Loading
- Extend reconciler to detect triggers in loaded packages
- Register triggers with TriggerScheduler on package load
- Unregister on package removal
- Webhook endpoint registration (server creates `/webhooks/{id}` route dynamically)

### Phase 3: Operational API + Daemon
- GET/enable/disable/executions endpoints
- Daemon: triggers from directory-watched packages auto-registered

### Phase 4: Python Integration
- `@cloaca.trigger` decorator metadata included in manifest at build time
- Python builder extracts trigger declarations from source (AST or decorator metadata)
- `PythonTriggerWrapper` loaded from package at registration time

### Phase 5: Testing + Documentation
- Unit tests: each built-in type, manifest parsing, factory
- Integration tests: upload package with triggers → webhook fires → workflow executes → completes
- Soak test: triggers + cron + continuous all running under load
- Chaos test: kill with active triggers, restart, verify triggers resume from packages
- Tutorial: "Packaging Workflows with Triggers"
- API reference: trigger manifest schema, operational endpoints
