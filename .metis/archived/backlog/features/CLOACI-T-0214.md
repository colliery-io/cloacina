---
id: packaged-triggers-compilable
level: task
title: "Packaged Triggers — compilable, uploadable trigger distribution (mirrors workflow packaging)"
short_code: "CLOACI-T-0214"
created_at: 2026-03-18T02:54:57.045180+00:00
updated_at: 2026-03-18T02:54:57.045180+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Configurable trigger types for server — webhook, file watch, queue consumer

## Objective

Extend the packaging and distribution model to cover triggers — not just workflows. Users compile a `Trigger` trait implementation to a cdylib, package it, and upload it to the server (or drop it in the daemon's packages directory). The server loads it via FFI and registers it with the `TriggerScheduler`, just like workflow packages are loaded by the `RegistryReconciler`.

The core principle: **a workflow is the "what", a trigger is the "when/why"** — both are user-authored Rust code, both are distributable packages, both are loaded dynamically at runtime.

| Strategy | Status | Interface |
|----------|--------|-----------|
| On-demand | Done | `POST /executions` |
| Cron | Done (CLOACI-T-0213) | `POST /workflows/{name}/schedules` |
| Webhook | **This task** | `POST /workflows/{name}/triggers` |
| File watch | **This task** | `POST /workflows/{name}/triggers` |
| Queue consumer | **This task** | `POST /workflows/{name}/triggers` |
| Continuous/reactive | Done (I-0023/24/25) | Library-level only (separate system) |

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Event-driven workflows without writing Rust. Attach a webhook or file watcher to any uploaded workflow via a single API call.
- **Business Value**: Makes the server useful for real-world integration patterns (webhooks from SaaS, file drops from partners, queue-driven ETL). Without this, users must embed Cloacina as a library to get event triggers.
- **Effort Estimate**: L — needs built-in trigger implementations, persistence (trigger configs in DB), API endpoints, and TriggerScheduler integration.

## Background: Current Trigger Architecture

The `Trigger` trait (`crates/cloacina/src/trigger/mod.rs:253`) defines:
```rust
pub trait Trigger: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn poll_interval(&self) -> Duration;
    fn allow_concurrent(&self) -> bool;
    async fn poll(&self) -> Result<TriggerResult, TriggerError>;
}
```

The `TriggerScheduler` (`trigger_scheduler.rs`) polls registered triggers at their `poll_interval` and fires workflows when `poll()` returns `TriggerResult::Fire(context)`. Triggers are registered in code:

```rust
runner.trigger_scheduler().register_trigger(&my_trigger, "workflow_name").await;
```

There's no persistence, no API, no configuration — purely in-process Rust. The `Trigger` trait itself is sound; the gap is making it configurable without code.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `POST /workflows/{name}/triggers` creates a trigger attached to a workflow, with `type` and `config` fields
- [ ] `GET /workflows/{name}/triggers` lists triggers for a workflow
- [ ] `DELETE /workflows/triggers/{id}` removes a trigger
- [ ] Built-in trigger type: **webhook** — returns a unique URL; POSTing to it fires the workflow with the request body as context
- [ ] Built-in trigger type: **http_poll** — polls a URL at an interval, fires when response matches a condition (status code, body contains, JSON path)
- [ ] Built-in trigger type: **file_watch** — watches a directory for new files matching a glob pattern (daemon mode only — requires local filesystem)
- [ ] Trigger configs are persisted to the database and restored on server restart
- [ ] TriggerScheduler automatically loads persisted triggers on startup (similar to how RegistryReconciler loads packages)
- [ ] Daemon CLI: `cloacinactl daemon trigger add <workflow> --type webhook` (mirrors the HTTP API)

## Implementation Notes

### API shape

```
POST /workflows/{name}/triggers
{
  "type": "webhook",
  "config": { "secret": "optional-hmac-secret" }
}
→ 201 { "id": "...", "type": "webhook", "url": "/webhooks/{id}", ... }

POST /workflows/{name}/triggers
{
  "type": "http_poll",
  "config": {
    "url": "https://api.example.com/status",
    "interval_secs": 60,
    "fire_when": { "status": 200, "json_path": "$.ready", "equals": true }
  }
}
→ 201 { "id": "...", "type": "http_poll", ... }
```

### Built-in trigger types to ship

1. **Webhook** — server exposes `POST /webhooks/{trigger_id}`. Any POST to that URL fires the associated workflow with the request body as context. Optional HMAC secret validation. This is the simplest and most broadly useful.

2. **HTTP Poll** — polls an external URL at a configured interval. Fires when the response matches a condition. Useful for "run when external system is ready" patterns.

3. **File Watch** (daemon mode only) — watches a local directory for files matching a glob. Fires with file path in context. Useful for ETL file-drop patterns.

### Persistence

New table:
```sql
CREATE TABLE workflow_triggers (
    id UUID PRIMARY KEY,
    workflow_name TEXT NOT NULL,
    trigger_type TEXT NOT NULL,        -- 'webhook', 'http_poll', 'file_watch'
    config JSONB NOT NULL,             -- type-specific configuration
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
```

On server startup, load all enabled triggers from `workflow_triggers`, instantiate the appropriate `Trigger` impl, and register with `TriggerScheduler`.

### Architecture: built-in types implement `Trigger` trait

Each built-in type is a struct that implements `Trigger`:
- `WebhookTrigger` — doesn't actually poll; instead the webhook HTTP handler pushes into a channel that the trigger's `poll()` drains
- `HttpPollTrigger` — `poll()` makes an HTTP request and evaluates the condition
- `FileWatchTrigger` — `poll()` scans the directory for new files

This means the existing `TriggerScheduler` works unchanged — it just gets trigger instances from the database instead of from user code.

### Design decisions to make
- **Webhook routing**: dedicated `/webhooks/{id}` path (public, no auth) vs reuse existing auth? Webhooks typically use HMAC signatures rather than API keys.
- **Trigger hot-reload**: if a trigger config is updated via API, does the running TriggerScheduler pick it up? Simplest: require server restart. Better: reconciler pattern (poll DB for changes).
- **Queue consumers** (SQS, Kafka, etc.): ship as built-in types or leave for a future plugin system? These need SDK dependencies. Probably future work — webhook + http_poll cover most cases.

### Related
- CLOACI-T-0213 — Cron schedule API (same pattern, different trigger type)
- CLOACI-T-0212 — Daemon mode (file_watch trigger is daemon-only)
- `crates/cloacina/src/trigger/mod.rs` — Trigger trait definition
- `crates/cloacina/src/trigger_scheduler.rs` — TriggerScheduler that polls triggers
- Tutorial 09 — Event triggers (library-level, covers the Trigger trait)

## Status Updates

*To be added during implementation*
