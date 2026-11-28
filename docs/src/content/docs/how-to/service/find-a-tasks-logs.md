---
title: "Find a Task's Logs"
description: "Read the traceback of a failed Python task, find a task's lines in the agent's log by its ids, and link the task view to your log stack."
sidebar:
  order: 84
---

Cloacina stores the result of a task, not its output. What a task prints is in
the log of the process that ran it. This page shows what Cloacina keeps for a
failed task, how to find a task's lines in the process log, and how to link
the web UI to your log stack.

## What Cloacina keeps for a task

| Where | What |
|-------|------|
| `last_error` | The message of the failure of the last attempt. For a Python task: the exception type and text. |
| `error_details` | For a failed Python task: the traceback. For a skipped or abandoned task: the reason. |

Both are fields of each task in
`GET /v1/tenants/{tenant_id}/executions/{exec_id}/tasks`. The web UI shows
`last_error` in the task table of a run, and the details under **Details** in
the same cell.

The traceback is what Python's `traceback.format_exception` gives: the frames,
the exception, and the exceptions it was raised from. At most 16 KiB are
kept. A longer traceback keeps its end, and starts with a line that says how
many bytes were cut.

```text
Traceback (most recent call last):
  File "/cache/etl/tasks.py", line 12, in inner
    raise ValueError('bad row 7')
ValueError: bad row 7

The above exception was the direct cause of the following exception:

Traceback (most recent call last):
  File "/cache/etl/tasks.py", line 18, in load
    raise RuntimeError('load failed') from e
RuntimeError: load failed
```

Retry conditions (`TransientOnly`, error patterns) read the message only. A
word in the traceback does not make a failure transient.

The two fields hold the failure of the attempt that failed the task. An
attempt that failed and was retried leaves no error on the task: find it in
the process log by its `attempt`.

A Rust task has no traceback. Its failure is the message of the error it
returns.

## Where the output of a task goes

| Task runs in | Output goes to |
|--------------|----------------|
| An execution agent (`cloacina-agent`) | The agent's stdout and stderr: the pod or container log. |
| The server's in-process executor | The server's stderr, and for log lines also the JSON log file under `--home`. |
| The embedded runner, `cloacinactl daemon` | The host program's stdout and its own logger. |

An agent runs the tasks of one tenant. The server's in-process executor runs
the tasks of every tenant in one process.

## Ids on log lines

Each log line that the agent or the server writes while a task runs carries
the ids of that attempt. They are the fields of a span named `task`:

| Field | Value |
|-------|-------|
| `tenant` | The tenant (`public` when there is none). |
| `workflow` | The workflow name. |
| `execution_id` | The id of the run. |
| `task` | The full task name, as the task API gives it: `tenant::package::workflow::task`. |
| `attempt` | The attempt number, from 1. |
| `dispatch_id` | The id of this dispatch to an agent. Agent only. |

In the text format the fields come before the message:

```text
2026-11-26T10:15:02.118Z  WARN task{tenant=acme workflow=etl execution_id=6f1c2b1e-58a0-4d0e-9a53-0c8a2f1f4a10 task=acme::etl_pkg::etl::load attempt=2 dispatch_id=0d9c7a52-4f43-41f0-8a39-5b0a3f6f7f11}: cloacina::registry: ...
```

In the JSON format they are under `span`:

```json
{"timestamp":"2026-11-26T10:15:02.118Z","level":"WARN","fields":{"message":"..."},"target":"cloacina::registry","span":{"attempt":2,"dispatch_id":"0d9c7a52-4f43-41f0-8a39-5b0a3f6f7f11","execution_id":"6f1c2b1e-58a0-4d0e-9a53-0c8a2f1f4a10","name":"task","task":"acme::etl_pkg::etl::load","tenant":"acme","workflow":"etl"},"spans":[...]}
```

The ids are on a line at any log level: `RUST_LOG=warn` keeps them on the
lines it lets through.

### Which lines carry the ids

- Lines that the agent, the server or the engine write with their logger
  while the task call runs. This includes the Rust side of a Python task.
- Lines of an embedded Rust task (compiled into the host program) that uses
  the `tracing` macros.

These lines do not carry the ids:

- `print` in a Python task and `println!` / `eprintln!` in a Rust task. They
  go to the stdout or stderr of the process as the task wrote them.
- Python's `logging` module. Its handlers write on their own, to stderr
  unless the task sets them up.
- `tracing` events of a packaged Rust task. A packaged workflow is a loaded
  library with its own copy of `tracing` and no subscriber, so the agent does
  not write these events. Use `println!` in a packaged Rust task.

To find the unlabelled output of a task, take the time range of the attempt
from the labelled lines and read the same pod's log in that range. With
`--max-concurrency 1` an agent runs one task at a time, so the range holds one
task.

## JSON log lines

A log stack reads JSON lines without a parse rule.

| Process | Flag | Env var | Default |
|---------|------|---------|---------|
| `cloacina-agent` | `--log-format <text\|json>` | `CLOACINA_AGENT_LOG_FORMAT` | `text` |
| `cloacina-server` | `--log-format <text\|json>` | `CLOACINA_LOG_FORMAT` | `text` |

The agent writes its log to stdout. The server's flag sets the format of
stderr; its log file under `--home` is JSON in both.

In the Helm charts: `logFormat: json` in `cloacina-agent`, and
`server.logFormat: json` in `cloacina-server`.

Agents that the server's fleet actuator starts take their format from the
server. Set `CLOACINA_AGENT_LOG_FORMAT=json` in the server's environment
(`fleet.agentLogFormat: json` in the `cloacina-server` chart): the actuator
then sets the same variable on each agent it starts. It is one value for every
tenant and pool; the default is `text`. A value that is not `text` or `json`
stops the server at start. On Kubernetes a change rolls the agent pods on the
next reconcile. With the Docker actuator a running agent container keeps its
format; a container started after the change has the new one.

`cloacinactl daemon` and the embedded runner use the host program's logger.
Give it a JSON format there.

## A log link on the task view

Set a URL template on the server. The task API then gives a `log_url` for
each task, and the web UI shows it as **Logs ↗** in the task table of a run.
With no template there is no `log_url` and no link.

```bash
cloacina-server \
  --task-log-url-template 'https://logs.example.com/search?tenant={tenant}&run={execution_id}&task={task}&attempt={attempt}'
```

| Flag | Env var | Default | Purpose |
|------|---------|---------|---------|
| `--task-log-url-template <URL>` | `CLOACINA_TASK_LOG_URL_TEMPLATE` | — | URL of a task's logs, with placeholders. Not set, or empty: no link. |

| Placeholder | Filled in with |
|-------------|----------------|
| `{tenant}` | The tenant of the run. |
| `{execution_id}` | The id of the run. |
| `{task}` | The full task name. |
| `{attempt}` | The attempt number of the task. |

The values are the ones on the log lines, so a query built from them finds
the lines. Each value is percent-encoded. Any other text in braces stays as
it is. The template must start with `http://` or `https://`; the server does
not start with another scheme.

`cloacinactl server start` takes the same flag and env var and passes them
on. In the Helm chart `cloacina-server` the value is
`server.taskLogUrlTemplate`.

The link opens your log stack. It works for a person who can sign in there.
The server does not fetch logs and does not check the URL.

### Example: Grafana with Loki

With the agents' JSON lines in Loki under the label `app="cloacina-agent"`,
this query finds the lines of one attempt:

```text
{app="cloacina-agent"} | json | span_execution_id="<run id>" | span_attempt="<attempt>"
```

Build the query in Grafana Explore for a known run, copy the URL from the
browser, and put `{execution_id}` and `{attempt}` in the place of the run id
and the attempt. That URL is the template.

## See also

- [Alert on a Failed Run](../alert-on-a-failed-run/): a webhook with the failed tasks and their last error.
- [Observability](../../../explanation/service/observability/): metrics, logs and traces.
- [Environment Variables](../../../reference/environment-variables/) and the [CLI reference](../../../reference/cli/).
