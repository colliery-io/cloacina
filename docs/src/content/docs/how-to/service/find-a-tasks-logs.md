---
title: "Find a Task's Logs"
description: "Read the end of a task's output and the traceback of a failed Python task, find a task's lines in the agent's log by its ids, and link the task view to your log stack."
sidebar:
  order: 84
---

Cloacina keeps the result of a task and, for a task that runs on an
execution agent, the end of what each attempt printed. The whole output is in
the log of the process that ran it. This page shows what Cloacina keeps for a
task, how to find a task's lines in the process log, and how to link the web
UI to your log stack.

## What Cloacina keeps for a task

| Where | What |
|-------|------|
| `last_error` | The message of the failure of the last attempt. For a Python task: the exception type and text. |
| `error_details` | For a failed Python task: the traceback. For a skipped or abandoned task: the reason. |
| `attempt_errors` | The error of each failed attempt, with its attempt number: the message and the traceback. A task that was retried keeps the errors of the attempts before. |

All three are fields of each task in
`GET /v1/tenants/{tenant_id}/executions/{exec_id}/tasks`. The web UI shows
`last_error` in the task table of a run, the details under **Details** in
the same cell, and the errors of the attempts before the current one under
**Earlier attempts**. They are deleted with the run by run-history
retention.

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

## The end of a task's output

An execution agent runs each task in a worker process. It reads the worker's
stdout and stderr as one stream, and keeps the last 256 KiB of each attempt.
The server stores it with the result, one row per attempt, for an attempt
that succeeded and for one that failed. A worker that the agent stops at the
task's memory or time limit leaves what it wrote before the stop.

Read it when the attempt has ended:

| Where | How |
|-------|-----|
| Web UI | **Output** in the task table of a run: each attempt, with its number and size. |
| `cloacinactl` | `cloacinactl execution output <execution id> <task id> [--attempt N]` |
| HTTP | `GET /v1/tenants/{tenant_id}/executions/{exec_id}/tasks/{task_id}/output[?attempt=N]` |
| Rust client | `Client::get_task_output(exec_id, task_id, attempt, tenant)` |
| Python client | `client.get_task_output(exec_id, task_id, attempt=None, tenant=None)` |
| TypeScript client | `client.getTaskOutput(execId, taskId, { attempt, tenant })` |

The task id is the `id` of the task in
`GET /v1/tenants/{tenant_id}/executions/{exec_id}/tasks`. A key that may read
the run may read its output (the `read` role of the tenant). The answer has,
for each attempt: `output` (the kept text), `total_bytes` (what the attempt
wrote in all), `cut` (`true` when the start was dropped) and `recorded_at`.
A task with no stored output gives an empty list.

Longer output is cut at its start: the end stays.
A task never fails because of its output. Bytes that are not UTF-8 are
replaced by U+FFFD.

| Setting | Env var | Default | Purpose |
|---------|---------|---------|---------|
| `cloacina-server --task-output-max-bytes` | `CLOACINA_TASK_OUTPUT_MAX_BYTES` | `262144` (256 KiB) | Bytes kept per attempt. The server sends it to the agent with each task, and cuts again to it. `0` keeps none. |
| `cloacina-server --task-output-retention-days` | `CLOACINA_TASK_OUTPUT_RETENTION_DAYS` | `30` | Days a stored output is kept. The retention job deletes it after that, whether run-history retention is on or not. `0` keeps it for ever. |
| `cloacina-agent --task-output-masking` | `CLOACINA_AGENT_TASK_OUTPUT_MASKING` | `true` | Replace the exact values of the task's own secrets by `***`. |

`cloacinactl server start` takes the two server settings as flags, env vars
and the `[server]` keys `task_output_max_bytes` and
`task_output_retention_days`. In the Helm chart `cloacina-server` they are
`taskOutput.maxBytes` and `retention.taskOutputDays`. Run-history retention
deletes a run's output with the run.

### What masking covers

The agent replaces, line by line, the exact value of each field of each
secret that it unwrapped for that attempt, before the line goes to its log or
into the kept output. It does not find:

- a value that the task encoded (base64, URL encoding, JSON escapes), cut,
  or joined into a longer string with no change;
- a value split over two lines;
- a secret that the task fetched itself, a value in the task's context, or
  personal data;
- the secrets of a graph firing or a poll (they have none).

Stored output is tenant data. Treat it like the task's own logs.

### What keeps no output

These run the task with no worker process, so their stdout and stderr belong
to the process, not to the task. No output is stored, and the output route
gives an empty list:

- a task marked `inline = true`, and any task on an agent on a system with no
  worker processes (not Unix);
- the server's in-process executor (`--default-executor default`);
- the embedded runner and `cloacinactl daemon`;
- graph firings and polls of poll triggers.

There is no view of the output while the attempt runs. An agent that dies
loses the tail of the attempts it ran; the pod log still has the lines.

## Where the output of a task goes

| Task runs in | Output goes to |
|--------------|----------------|
| An execution agent (`cloacina-agent`), in a worker process | The agent's log (stdout: the pod or container log), one line per line of output, with the task's ids. The end is stored with the result (above). |
| An execution agent, `inline = true` | The agent's stdout and stderr, as the task wrote them. |
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
- Lines of a packaged Rust task that uses the `tracing` macros or the `log`
  crate. The process that runs the task writes them with its own logger: the
  agent or its worker process, the server, or the embedded runner. Each line
  keeps its level and target, and the process's filter applies to it:
  `RUST_LOG=info` drops a `debug!` of the task.
- Lines of a packaged Rust poll trigger polled on an agent, with the
  `tracing` macros or the `log` crate, in the agent process or in its worker
  process. They carry the ids of the poll, the fields of a span named `poll`:
  `tenant`, `package`, `trigger` and `poll_id`. A worker process writes them
  to the agent's stdout, which it inherits.
- Lines of a native constructor provider (`runtime = "native"`) that uses the
  `tracing` macros or the `log` crate, from a task, trigger, accumulator or
  reactor member. The process that loaded the provider writes them, in the
  span of the call.
- Every line a task in an agent's worker process writes on stdout or stderr:
  `print`, Python's `logging` module, `println!` / `eprintln!`, and native
  libraries. The agent writes each line to its log at info, with the target
  `task_output`, in the span of the task.

These lines do not carry the ids:

- `print` in a Python task and `println!` / `eprintln!` in a Rust task that
  does not run in an agent's worker process (`inline = true`, the server's
  in-process executor, the embedded runner). They go to the stdout or stderr
  of the process as the task wrote them.
- Python's `logging` module outside an agent's worker process. Its handlers
  write on their own, to stderr unless the task sets them up.
- Lines a worker process writes while no task runs on it (its start, a graph
  firing, a poll). The agent writes them with the target `worker_output` and
  the worker's `pid`.
- `tracing` and `log` lines of a packaged Rust task that come from a thread
  or a tokio task that the task starts itself. They are written, with no ids.
- `tracing` and `log` lines of a package built against
  `cloacina-workflow-plugin` 0.12 or earlier. They are not written; the
  package runs as before. Build and publish it again to get them.
- `tracing` and `log` lines of a native provider's stream accumulator
  (`mode = stream`) from its `source` iterator. They are written, with no ids:
  the iterator runs on its own thread.
- `tracing` and `log` lines of a native provider built against
  `cloacina-constructor-contract` 0.12 or earlier, and of a wasm provider.
  They are not written; the provider runs as before.

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
