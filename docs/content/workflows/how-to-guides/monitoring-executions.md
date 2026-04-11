---
title: "Monitoring Executions"
description: "How to monitor and troubleshoot workflow executions using the API and Python bindings"
weight: 60
---

# Monitoring Executions

This guide shows how to monitor workflow executions, inspect event logs, check trigger status, and build a simple monitoring script using the Cloacina API and Python bindings.

## Prerequisites

- API server running (see [Deploying the API Server]({{< ref "/platform/how-to-guides/deploying-the-api-server" >}}))
- A valid API key stored in the `API_KEY` environment variable
- At least one tenant created with workflows deployed

## Listing Executions

To see all active pipeline executions for a tenant:

```bash
curl -s http://localhost:8080/tenants/tenant_a/executions \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response:

```json
{
  "tenant_id": "tenant_a",
  "executions": [
    {
      "id": "d4e5f6a7-b8c9-0123-4567-890abcdef012",
      "pipeline_name": "data-ingest",
      "status": "running",
      "started_at": "2026-04-02T14:30:00+00:00",
      "completed_at": null
    },
    {
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "pipeline_name": "nightly-cleanup",
      "status": "completed",
      "started_at": "2026-04-02T00:00:00+00:00",
      "completed_at": "2026-04-02T00:05:23+00:00"
    }
  ]
}
```

## Getting Execution Details

Fetch the status of a single execution by ID:

```bash
curl -s http://localhost:8080/tenants/tenant_a/executions/d4e5f6a7-b8c9-0123-4567-890abcdef012 \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response:

```json
{
  "tenant_id": "tenant_a",
  "execution_id": "d4e5f6a7-b8c9-0123-4567-890abcdef012",
  "status": "Running"
}
```

## Viewing Event Logs

Each execution records a sequence of events as tasks start, complete, or fail. Retrieve the full event log:

```bash
curl -s http://localhost:8080/tenants/tenant_a/executions/d4e5f6a7-b8c9-0123-4567-890abcdef012/events \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response:

```json
{
  "tenant_id": "tenant_a",
  "execution_id": "d4e5f6a7-b8c9-0123-4567-890abcdef012",
  "events": [
    {
      "id": "e1f2a3b4-c5d6-7890-1234-567890abcdef",
      "event_type": "task_started",
      "event_data": "{\"task_name\": \"extract\", \"attempt\": 1}",
      "created_at": "2026-04-02T14:30:01+00:00",
      "sequence_num": 1
    },
    {
      "id": "f2a3b4c5-d6e7-8901-2345-67890abcdef1",
      "event_type": "task_completed",
      "event_data": "{\"task_name\": \"extract\", \"duration_ms\": 4523}",
      "created_at": "2026-04-02T14:30:05+00:00",
      "sequence_num": 2
    },
    {
      "id": "a3b4c5d6-e7f8-9012-3456-7890abcdef12",
      "event_type": "task_started",
      "event_data": "{\"task_name\": \"transform\", \"attempt\": 1}",
      "created_at": "2026-04-02T14:30:06+00:00",
      "sequence_num": 3
    }
  ]
}
```

Events are ordered by `sequence_num`. Common event types include `task_started`, `task_completed`, `task_failed`, and `pipeline_completed`.

## Checking Trigger and Cron Status

### List All Schedules

View all cron and trigger schedules for a tenant:

```bash
curl -s http://localhost:8080/tenants/tenant_a/triggers \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response:

```json
{
  "tenant_id": "tenant_a",
  "schedules": [
    {
      "id": "b1c2d3e4-f5a6-7890-bcde-f12345678901",
      "schedule_type": "cron",
      "workflow_name": "nightly-cleanup",
      "enabled": true,
      "cron_expression": "0 0 * * *",
      "trigger_name": null,
      "poll_interval_ms": null,
      "next_run_at": "2026-04-03T00:00:00+00:00",
      "last_run_at": "2026-04-02T00:00:00+00:00",
      "created_at": "2026-03-15T10:00:00+00:00"
    },
    {
      "id": "c2d3e4f5-a6b7-8901-cdef-234567890123",
      "schedule_type": "trigger",
      "workflow_name": "s3-watcher",
      "enabled": true,
      "cron_expression": null,
      "trigger_name": "check_s3_bucket",
      "poll_interval_ms": 5000,
      "next_run_at": null,
      "last_run_at": "2026-04-02T14:29:55+00:00",
      "created_at": "2026-03-20T08:00:00+00:00"
    }
  ]
}
```

### Get Trigger Details with Recent Executions

Drill into a specific trigger or cron schedule by name to see its recent execution history:

```bash
curl -s http://localhost:8080/tenants/tenant_a/triggers/nightly-cleanup \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response:

```json
{
  "tenant_id": "tenant_a",
  "schedule": {
    "id": "b1c2d3e4-f5a6-7890-bcde-f12345678901",
    "schedule_type": "cron",
    "workflow_name": "nightly-cleanup",
    "enabled": true,
    "cron_expression": "0 0 * * *"
  },
  "recent_executions": [
    {
      "id": "d3e4f5a6-b7c8-9012-def0-345678901234",
      "scheduled_time": "2026-04-02T00:00:00+00:00",
      "started_at": "2026-04-02T00:00:01+00:00",
      "completed_at": "2026-04-02T00:05:23+00:00"
    },
    {
      "id": "e4f5a6b7-c8d9-0123-ef01-456789012345",
      "scheduled_time": "2026-04-01T00:00:00+00:00",
      "started_at": "2026-04-01T00:00:01+00:00",
      "completed_at": "2026-04-01T00:04:58+00:00"
    }
  ]
}
```

## Python API Alternative

The Python bindings provide methods for querying cron and trigger status directly from workflow code or monitoring scripts. Install the package first if you have not already:

```bash
pip install cloaca
```

### Cron Execution Stats

```python
from datetime import datetime, timedelta, timezone
from cloacina import Runner

runner = Runner("postgresql://cloacina:cloacina@localhost:5432/cloacina")

# Get aggregate stats for the last 24 hours
since = datetime.now(timezone.utc) - timedelta(hours=24)
stats = runner.get_cron_execution_stats(since)
print(f"Total: {stats.total}, Succeeded: {stats.succeeded}, Failed: {stats.failed}")
```

### List Cron Schedules

```python
schedules = runner.list_cron_schedules(enabled_only=True, limit=50, offset=0)
for s in schedules:
    print(f"{s.workflow_name}: {s.cron_expression} (next: {s.next_run_at})")
```

### Cron Execution History

```python
history = runner.get_cron_execution_history(schedule_id="b1c2d3e4-...", limit=10, offset=0)
for h in history:
    print(f"  {h.scheduled_time} -> started {h.started_at}, completed {h.completed_at}")
```

### Trigger Schedules

```python
triggers = runner.list_trigger_schedules(enabled_only=True, limit=50, offset=0)
for t in triggers:
    print(f"{t.trigger_name}: polling every {t.poll_interval_ms}ms")

# Get history for a specific trigger
history = runner.get_trigger_execution_history("check_s3_bucket", limit=10, offset=0)
```

## Building a Monitoring Script

The following Python script polls the API and reports execution status. Run it via cron or as a long-running process.

```python
#!/usr/bin/env python3
"""Simple Cloacina execution monitor."""

import json
import os
import sys
import time
import urllib.request

API_URL = "http://localhost:8080"
API_KEY = os.environ.get("API_KEY", "clk_your_api_key_here")
TENANT = "tenant_a"
POLL_INTERVAL = 60  # seconds


def api_get(path):
    """Make an authenticated GET request to the Cloacina API."""
    req = urllib.request.Request(
        f"{API_URL}{path}",
        headers={"Authorization": f"Bearer {API_KEY}"},
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read())


def check_executions():
    """Check for running and failed executions."""
    data = api_get(f"/tenants/{TENANT}/executions")
    running = [e for e in data["executions"] if e["status"] == "running"]
    failed = [e for e in data["executions"] if e["status"] == "failed"]

    if failed:
        print(f"[ALERT] {len(failed)} failed execution(s):")
        for e in failed:
            print(f"  - {e['pipeline_name']} ({e['id']})")

    if running:
        print(f"[INFO] {len(running)} execution(s) in progress:")
        for e in running:
            print(f"  - {e['pipeline_name']} started {e['started_at']}")

    return len(failed)


def check_schedules():
    """Check that all enabled schedules have fired recently."""
    data = api_get(f"/tenants/{TENANT}/triggers")
    for s in data["schedules"]:
        if s["enabled"] and s["last_run_at"] is None:
            print(f"[WARN] Schedule '{s['workflow_name']}' has never fired")


def main():
    print(f"Monitoring Cloacina tenant '{TENANT}' every {POLL_INTERVAL}s")
    while True:
        try:
            failures = check_executions()
            check_schedules()
            if failures > 0:
                # In production, send to PagerDuty, Slack, etc.
                pass
        except Exception as e:
            print(f"[ERROR] Monitoring check failed: {e}", file=sys.stderr)
        time.sleep(POLL_INTERVAL)


if __name__ == "__main__":
    main()
```

Save as `monitor.py` and run:

```bash
API_KEY="clk_..." python3 monitor.py
```

For production monitoring, integrate the API calls into your existing observability stack (Prometheus, Datadog, Grafana) using the `/metrics` endpoint or by polling the execution and trigger APIs.
