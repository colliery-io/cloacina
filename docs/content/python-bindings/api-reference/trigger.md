---
title: "Trigger Decorator"
description: "Define event-driven workflow triggers with the @trigger decorator"
weight: 55
---

# Trigger Decorator

The `@trigger` decorator defines custom Python triggers that poll user-defined conditions and fire workflows when those conditions are met. Triggers are declared in package manifests and auto-registered when the package is loaded.

## Package-First Design

Triggers are part of `.cloacina` packages. The workflow to fire is declared in the package manifest (`manifest.json`), not in the decorator. The decorator registers the Python poll function.

### manifest.json

```json
{
  "triggers": [
    {
      "name": "check_inbox",
      "type": "python",
      "workflow": "process_files",
      "poll_interval": "30s",
      "allow_concurrent": false,
      "config": {}
    }
  ]
}
```

### Python source

```python
import cloaca

@cloaca.trigger(name="check_inbox", poll_interval="30s")
def check_inbox():
    if new_files_available("/inbox/"):
        return cloaca.TriggerResult(should_fire=True, context={"path": "/inbox/"})
    return cloaca.TriggerResult(should_fire=False)
```

## Decorator Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `name` | `str` | function name | Unique trigger identifier (must match manifest) |
| `poll_interval` | `str` | `"30s"` | How often to poll (`"5s"`, `"1m"`, `"500ms"`) |
| `allow_concurrent` | `bool` | `False` | Allow parallel executions |

## TriggerResult

The poll function must return a `TriggerResult` or a plain `bool`.

### TriggerResult(should_fire, context)

```python
# Skip — condition not met
return cloaca.TriggerResult(should_fire=False)

# Fire without context
return cloaca.TriggerResult(should_fire=True)

# Fire with context dict
return cloaca.TriggerResult(should_fire=True, context={"key": "value"})
```

### Bool shorthand

```python
# Equivalent to TriggerResult(should_fire=False)
return False

# Equivalent to TriggerResult(should_fire=True) with no context
return True
```

## Built-In Trigger Types

In addition to Python triggers, packages can declare built-in trigger types that don't require Python code:

### webhook

Receives HTTP POST payloads. The server creates a `/webhooks/{name}` endpoint.

```json
{
  "name": "on_upload",
  "type": "webhook",
  "workflow": "process_upload",
  "config": { "path": "/hooks/upload" }
}
```

### http_poll

Polls an HTTP endpoint and fires when the response matches expectations.

```json
{
  "name": "check_api",
  "type": "http_poll",
  "workflow": "sync_data",
  "poll_interval": "5m",
  "config": {
    "url": "https://api.example.com/status",
    "method": "GET",
    "expect_status": 200
  }
}
```

### file_watch

Scans a directory for new files matching a glob pattern.

```json
{
  "name": "watch_inbox",
  "type": "file_watch",
  "workflow": "process_files",
  "poll_interval": "10s",
  "config": {
    "directory": "/data/inbox",
    "glob": "*.csv"
  }
}
```

## REST API

Triggers are managed via the server API (read-only + enable/disable):

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/triggers` | GET | List all trigger schedules |
| `/triggers/{name}` | GET | Get trigger detail |
| `/triggers/{name}/enable` | POST | Enable a trigger |
| `/triggers/{name}/disable` | POST | Disable a trigger |

## CLI (Daemon)

```bash
# List all triggers
cloacinactl daemon trigger list

# Enable/disable
cloacinactl daemon trigger enable check_inbox
cloacinactl daemon trigger disable check_inbox
```

## Deduplication

When `allow_concurrent=False` (default), the trigger scheduler prevents duplicate executions:

1. Context is hashed when the trigger fires
2. Active executions are tracked by (trigger_name, context_hash)
3. If an execution with the same hash is running, the trigger skips

## Error Handling

Python exceptions in trigger poll functions are caught and logged — the trigger is not crashed. Polling continues on the next interval.

```python
@cloaca.trigger(name="resilient", poll_interval="1m")
def resilient_trigger():
    try:
        if check_for_updates():
            return cloaca.TriggerResult(should_fire=True)
    except Exception as e:
        logging.warning(f"Trigger check failed: {e}")
    return cloaca.TriggerResult(should_fire=False)
```

## See Also

- **[Context]({{< ref "/python-bindings/api-reference/context/" >}})** - Data passed from triggers to workflows
- **[Tutorial: Event Triggers]({{< ref "/python-bindings/tutorials/07-event-triggers/" >}})** - Step-by-step guide
