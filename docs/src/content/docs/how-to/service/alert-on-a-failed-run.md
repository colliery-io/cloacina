---
title: "Alert on a Failed Run"
description: "Send a signed webhook when a run fails, with no code in the workflow, and route it to Slack, PagerDuty or Alertmanager."
sidebar:
  order: 83
---

`cloacina-server` can send a JSON `POST` to one URL when a run ends. It needs
no code in a workflow. The hook is off until you set the URL.

There are two ways to get an alert:

- **The webhook** (this page): one request per run, with the workflow, the
  tenant, the failed tasks and their last error.
- **A Prometheus rule** on `cloacina_workflows_total`: works with no setting
  on the server, but it has counts only, no error text. See
  [Alertmanager](#alertmanager-no-webhook-needed).

## Turn it on

```bash
export CLOACINA_ALERT_WEBHOOK_SECRET='<a long random string>'
cloacina-server \
  --alert-webhook-url https://alerts.example.com/hook \
  --public-base-url https://cloacina.example.com
```

| Flag | Env var | Default | Purpose |
|------|---------|---------|---------|
| `--alert-webhook-url` | `CLOACINA_ALERT_WEBHOOK_URL` | — | The URL that gets the `POST`. Not set, or empty: the hook is off and the server does nothing new. |
| `--alert-webhook-events` | `CLOACINA_ALERT_WEBHOOK_EVENTS` | `failed` | Which outcomes send an alert, comma-separated: `failed`, `completed`, `cancelled`, or `all`. |
| — | `CLOACINA_ALERT_WEBHOOK_SECRET` | — | Shared secret that signs each request. Environment only, so it is not in the process arguments. Not set: requests are not signed. |
| `--alert-webhook-timeout-s` | `CLOACINA_ALERT_WEBHOOK_TIMEOUT_S` | `10` | Seconds allowed for one request. |
| `--alert-webhook-max-attempts` | `CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS` | `8` | The most sends of one alert. |
| `--public-base-url` | `CLOACINA_PUBLIC_BASE_URL` | — | The public URL of the server. With it the alert holds absolute links; without it, paths only. |

The server never logs the URL or the secret. Log lines name the host of the
URL only, because a webhook URL often holds a token.

The hook is one for the whole server. Every tenant's runs go to the same URL,
and the payload names the tenant. There is no per-tenant URL.

`cloacinactl server start` takes the same flags and environment variables, and
these keys of `[server]` in `config.toml`:

```toml
[server]
alert_webhook_url = "https://alerts.example.com/hook"
alert_webhook_events = ["failed"]
public_base_url = "https://cloacina.example.com"
```

The secret has no key in `config.toml`. Set `CLOACINA_ALERT_WEBHOOK_SECRET` in
the environment of `cloacinactl server start`; the server inherits it.

### With the Helm chart

The chart `cloacina-server` has the values `alertWebhook.*`. The signing secret
comes from a Kubernetes Secret that you create; the chart holds no plain value
for it.

```sh
kubectl create secret generic cloacina-alerts \
  --from-literal=secret="$(openssl rand -hex 32)"
```

```yaml
alertWebhook:
  url: https://alerts.example.com/hook
  events: [failed]                 # failed, completed, cancelled, or all
  timeoutSeconds: 10
  maxAttempts: 8
  publicBaseUrl: https://cloacina.example.com
  secretRef:
    name: cloacina-alerts
    key: secret
```

When the URL holds a token, keep it in a Secret too: put it under a key of a
Secret and set `alertWebhook.urlSecretRef.name` and `.key` in place of
`alertWebhook.url`. The chart renders the alert settings only when a URL is
set.

## What the server sends

A `POST` with `Content-Type: application/json` and these headers:

| Header | Value |
|--------|-------|
| `X-Cloacina-Event` | `workflow_execution.failed`, `workflow_execution.completed` or `workflow_execution.cancelled` |
| `X-Cloacina-Delivery` | The execution id. The same for each retry of one alert. |
| `X-Cloacina-Signature` | `t=<unix seconds>,v1=<hex HMAC-SHA256>`. Only when the secret is set. |

Example body:

```json
{
  "schema_version": 1,
  "event": "workflow_execution.failed",
  "tenant": "acme",
  "workflow": "nightly_etl",
  "workflow_version": "3",
  "package": "etl_pkg",
  "package_version": "1.4.2",
  "execution_id": "7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
  "status": "Failed",
  "trigger_origin": "schedule",
  "scheduled_time": "2026-11-25T02:00:00+00:00",
  "started_at": "2026-11-25T02:00:01.204+00:00",
  "finished_at": "2026-11-25T02:01:36.910+00:00",
  "error": "1 task(s) failed, 1 completed, 1 skipped",
  "failed_task_count": 1,
  "failed_tasks": [
    {
      "name": "load",
      "attempt": 3,
      "max_attempts": 3,
      "error": "connection refused"
    }
  ],
  "links": {
    "api_path": "/v1/tenants/acme/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
    "ui_path": "/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
    "api_url": "https://cloacina.example.com/v1/tenants/acme/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
    "ui_url": "https://cloacina.example.com/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11"
  }
}
```

| Field | Meaning |
|-------|---------|
| `schema_version` | `1`. It changes only when a field is removed or changes meaning. New fields can appear without a change: ignore fields you do not know. |
| `event` | `workflow_execution.failed`, `.completed` or `.cancelled`. |
| `tenant` | The tenant (schema) of the run. `public` for the default one. |
| `workflow`, `workflow_version` | The name and version stored on the run. |
| `package` | The package of the workflow, read from its task names. `null` for a workflow compiled into the binary, or a run with no task. |
| `package_version` | The package version the run is pinned to. `null` when the run has no pin: no package, or no task of the run was dispatched. |
| `execution_id` | The id of the run. Use it to drop a repeated alert. |
| `status` | The status of the run in the database when the alert was built. |
| `trigger_origin` | `schedule` (cron), `trigger`, `manual` (started through the API or the UI), or `null` when the server cannot tell. |
| `scheduled_time` | The time the cron schedule asked for. `null` for other runs. |
| `started_at`, `finished_at` | RFC 3339, UTC. |
| `error` | The reason stored on the run, cut to 500 characters. |
| `failed_task_count`, `failed_tasks` | The tasks with status `Failed`: name, attempt, max attempts and last error text (cut to 500 characters). The list holds 20 tasks at most; the count is the full number. |
| `links` | The run in the API and in the web UI. The `*_url` fields are `null` without `--public-base-url`. The UI link works when the server serves the [embedded UI](../embedded-ui/). |

The payload holds no context data and no secret. The error text of a task is
whatever the task returned: do not put credentials in error messages.

## Verify the signature

The signature is the HMAC-SHA256 of the timestamp, a dot, and the raw request
body, with the shared secret as the key, in lower-case hex. Check it against
the raw bytes, before any JSON parsing.

```python
import hashlib
import hmac
import time


def verify(secret: str, header: str, raw_body: bytes, tolerance_s: int = 300) -> bool:
    """`header` is the value of X-Cloacina-Signature."""
    parts = dict(item.split("=", 1) for item in header.split(","))
    timestamp = parts["t"]
    if abs(time.time() - int(timestamp)) > tolerance_s:
        return False  # too old: a replayed request
    expected = hmac.new(
        secret.encode(), timestamp.encode() + b"." + raw_body, hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(expected, parts["v1"])
```

Each retry is signed again with a new timestamp.

## Delivery

- The receiver must answer with a `2xx` status inside the timeout. Any other
  answer, a timeout or a connection error counts as a failed send. The server
  does not follow redirects.
- A failed send is tried again after 30 seconds, then 1, 2, 4, 8 and 15
  minutes (15 minutes from then on; never less than twice the timeout). With
  the default of 8 sends, the server gives up about 45 minutes after the run
  ended, logs `alert webhook: alert dropped` and counts it in
  `cloacina_alert_webhook_dropped_total`.
- An alert that is still not sent 24 hours after the run ended is dropped.
- The alert is stored in the database in the same transaction that ends the
  run, so a server restart does not lose it. With several server replicas, one
  replica sends each alert.
- Delivery is **at least once**. The receiver can get the same alert twice
  when it handled a request and its answer was lost, or when a server stopped
  between the send and the record of it. Use `execution_id` plus `event` to
  drop a repeat.
- A slow or down receiver does not slow the scheduler. The server looks for
  alerts to send every 5 seconds, so an alert leaves a few seconds after the
  run ends.

Runs that ended while the hook was off send nothing when you turn it on.

## Metrics

| Metric | Labels | Meaning |
|--------|--------|---------|
| `cloacina_alert_webhook_sent_total` | `event` | Requests the receiver answered with `2xx`. |
| `cloacina_alert_webhook_failed_total` | `reason` | Failed sends that will be tried again: `timeout`, `status`, `transport`, `load`. |
| `cloacina_alert_webhook_dropped_total` | `reason` | Alerts given up: `attempts`, `expired`, `invalid`. |

Alert on the hook itself:

```promql
increase(cloacina_alert_webhook_dropped_total[1h]) > 0
```

## Slack

A Slack incoming webhook accepts only Slack's own JSON shape (`{"text": …}` or
blocks). **Do not point `--alert-webhook-url` at a Slack webhook URL: Slack
rejects the Cloacina payload.** Put something between the two that rewrites
the body:

- **Slack Workflow Builder**: create a workflow that starts "from a webhook",
  declare the variables you want (they must be top-level text fields, so
  `workflow`, `tenant`, `execution_id`, `error` work; `failed_tasks` and
  `links` are nested and do not), and add a "send a message" step. Use the
  workflow's URL as `--alert-webhook-url`. Slack does not check the Cloacina
  signature.
- **A small relay** (a serverless function, or an automation tool such as n8n
  or a self-hosted webhook relay): verify the signature, then post to the
  Slack incoming webhook:

  ```json
  {
    "text": ":red_circle: *nightly_etl* failed for tenant *acme*\n1 task(s) failed, 1 completed, 1 skipped\n`load` (attempt 3 of 3): connection refused\n<https://cloacina.example.com/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11|Open the run>"
  }
  ```

## PagerDuty

The PagerDuty Events API v2 (`https://events.pagerduty.com/v2/enqueue`) also
expects its own JSON shape, with the routing key in the body. **The Cloacina
payload does not post to it as it is.** Two ways that need no code in
Cloacina:

- **A relay**, as for Slack, that sends:

  ```json
  {
    "routing_key": "<the integration key of the PagerDuty service>",
    "event_action": "trigger",
    "dedup_key": "cloacina:acme:7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
    "payload": {
      "summary": "nightly_etl failed for tenant acme: 1 task(s) failed, 1 completed, 1 skipped",
      "source": "cloacina.example.com",
      "severity": "error",
      "component": "nightly_etl",
      "group": "acme",
      "custom_details": { "failed_tasks": [{ "name": "load", "error": "connection refused" }] }
    },
    "links": [{ "href": "https://cloacina.example.com/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11", "text": "Open the run" }]
  }
  ```

  Build `dedup_key` from `tenant` and `execution_id`, so a repeated alert
  does not open a second incident.
- **Alertmanager**, which has a PagerDuty receiver built in: see the next
  section.

## Alertmanager (no webhook needed)

Each finished run increments `cloacina_workflows_total`, with the labels
`status`, `reason`, `workflow` and `tenant`. A Prometheus rule on it works
with no setting on the server:

```yaml
groups:
  - name: cloacina
    rules:
      - alert: CloacinaRunFailed
        expr: |
          sum by (tenant, workflow) (
            increase(cloacina_workflows_total{status="failed"}[10m])
          ) > 0
        labels:
          severity: page
        annotations:
          summary: "{{ $labels.workflow }} failed for tenant {{ $labels.tenant }}"
```

Route it in Alertmanager:

```yaml
route:
  receiver: default
  routes:
    - matchers: ['alertname="CloacinaRunFailed"']
      group_by: [tenant, workflow]
      receiver: oncall

receivers:
  - name: default
  - name: oncall
    pagerduty_configs:
      - routing_key_file: /etc/alertmanager/pagerduty-key
    slack_configs:
      - api_url_file: /etc/alertmanager/slack-webhook-url
        channel: "#data-alerts"
```

What this route does not give you: the execution id, the failed tasks and the
error text. The `workflow` label holds 100 names per tenant at most, then
`other` (see the [Metrics Catalog](../../reference/metrics-catalog/)). A
single failure between two scrapes right after a server start can be missed
by `increase()`, because the counter starts to exist with the first failure.

## Related

- [Metrics Catalog](../../reference/metrics-catalog/)
- [Environment Variables](../../reference/environment-variables/)
- [Configuration](../../reference/configuration/)
