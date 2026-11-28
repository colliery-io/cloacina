---
title: "Priority"
description: "How a workflow and a task carry a priority, the order in which waiting work is dispatched, ageing, and who may set which priority"
sidebar:
  order: 46
---

## The scale

A priority is an integer from 0 to 10. 0 is the highest priority, 10 the
lowest. Work that sets no priority has priority 5.

A workflow has a priority. A task can have its own.

## The order of waiting work

A task waits when it is ready to run and no slot is free for it: no free
slot in the in-process executor, or no agent of its pool with room on an
[execution-agent fleet](../service/execution-agent-fleet/).

Waiting tasks are dispatched in this order:

1. The priority of the task's workflow. Work of a more important workflow
   goes first.
2. The task's own priority. Inside one workflow priority, more important
   tasks go first. A task that sets none has the priority of its workflow.
3. The time the task has waited. The task that has waited longest goes first.

Work that sets no priority is all at 5, so it is dispatched oldest first.

Priority orders the queue only. A task that runs is never stopped to make
room for more important work.

Each tenant has its own queue. A priority in one tenant has no meaning in
another.

## Ageing

Waiting work ages, so work at a low priority runs in the end. For each
ageing interval a task has waited, both of its priorities improve by one in
the order, never past 0. The interval is two minutes by default. A task at
priority 10 is level with a new task at priority 5 after ten minutes, and at
the top of the order after twenty.

Ageing changes the order only. The priority stored with the execution and
its tasks does not change, and the API shows the stored one.

On a server the interval is the flag `--priority-aging-interval-s`, the
`[server]` key `priority_aging_interval_s` of `cloacinactl server start`, or
the environment variable `CLOACINA_PRIORITY_AGING_INTERVAL_SECONDS`
(default `120`; `0` switches ageing off). In an embedded program it is
`DefaultRunnerConfig::builder().priority_aging_interval(..)`.

## Declare a priority

In Rust:

```rust
#[workflow(name = "nightly_history_load", priority = 8)]
pub mod nightly_history_load {
    use super::*;

    #[task(id = "extract")]
    pub async fn extract(context: &mut Context<Value>) -> Result<(), TaskError> {
        Ok(())
    }

    // More important than the other tasks of workflows at priority 8.
    #[task(id = "publish", dependencies = ["extract"], priority = 2)]
    pub async fn publish(context: &mut Context<Value>) -> Result<(), TaskError> {
        Ok(())
    }
}
```

In Python:

```python
with cloaca.WorkflowBuilder("nightly_history_load") as builder:
    builder.priority(8)

    @cloaca.task(id="extract")
    def extract(context):
        return context

    @cloaca.task(id="publish", dependencies=["extract"], priority=2)
    def publish(context):
        return context
```

A value outside 0 to 10 is a compile error in Rust and a `ValueError` in
Python.

## Ask for a priority on a run

A manual run can ask for a workflow priority. It replaces the priority the
workflow declares, for that run:

```bash
cloacinactl workflow run orders --priority 2
```

```http
POST /v1/tenants/acme/workflows/orders/execute
{"context": {}, "priority": 2}
```

The answer has the priority that was asked for and the one that was given:

```json
{
  "execution_id": "…",
  "status": "scheduled",
  "priority": {"asked": 2, "given": 2},
  "priority_notice": null
}
```

The executions list and the execution detail show the same `priority`
object for each execution.

## Give a schedule a priority

A named instance and a backfill can be created with a workflow priority.
Each of their runs gets it, held to the limit of whoever created the
schedule, as that limit is when the run starts:

```bash
cloacinactl instance create orders nightly --cron "0 2 * * *" --priority 2
cloacinactl backfill create history_load --start 2026-01-01T00:00:00Z \
  --end 2026-06-30T23:00:00Z --priority 9
```

`priority` in the body of `POST .../instances` and `POST .../backfills` does
the same. The answer, the instance and backfill lists and their detail show
`priority` (asked and given now) and a `priority_notice` when the two
differ. `cloacinactl` prints the notice as a warning. A schedule created with
no priority gives its runs the priority the workflow declares. A backfill
retry keeps the priority of the backfill it retries.

## Who may set which priority

A principal has a limit: the highest priority it may set. The limit comes
from its role, unless its key or its OIDC mapping rule carries its own.

| Role | May set by default |
|---|---|
| `admin` | 0 to 10 |
| `write` | 5 to 10 |
| `read`, `agent` | none |

### A limit on a key

A tenant admin sets a key's own limit when it makes the key, or later:

```bash
cloacinactl --tenant acme key create ops-bot --role write --max-priority 2
cloacinactl --tenant acme key set-priority <key-id> --max-priority 0
cloacinactl --tenant acme key set-priority <key-id> --clear
```

The routes are `POST /v1/tenants/{tenant_id}/keys` with `max_priority`, and
`PATCH /v1/tenants/{tenant_id}/keys/{key_id}` with `{"max_priority": N}` or
`{"max_priority": null}`. The limit is 0 to 10 and no better than the
caller's own limit (else 403 `priority_above_limit`). A `read` or `agent`
key sets no priority, so it cannot carry a limit (400). A change applies to
the key's next request on every server, and to the next run of each package
and schedule the key set a priority for. The key list shows `max_priority`.

### A limit on an OIDC mapping rule

A rule of `CLOACINA_OIDC_MAP` takes a third field after the role: the highest
priority a login that the rule matches may set.

```bash
CLOACINA_OIDC_MAP="group:acme-ops=acme:write:2;domain:acme.com=acme:write"
```

A member of `acme-ops` may set 2 to 10; any other person of the domain has
the default of `write`. A third field that is not a whole number from 0 to
10 stops the server at start. A local account has the default of its role.

### Held, not refused

A run that asks for a priority above the limit of its key is not refused.
It runs at the limit, and the server says so:

- The answer carries `priority` with both numbers and `priority_notice` with
  one sentence that says what was asked, what was given and why.
  `cloacinactl workflow run` prints the notice as a warning on stderr.
- The server logs a warning with the key and the tenant.
- The audit event of the run (`workflow.run`) records `priority_asked`,
  `priority_given` and `priority_lowered`.
- `cloacina_priority_held_total{tenant}` counts the run.

### The limit of a publisher and of a creator

A package declares its priorities in its code, and a schedule is created
with one. Their runs start later, with no principal in the request. So the
server keeps who published the package and who created the schedule, with
their role, and at each run start holds the declared or asked priority to
that principal's limit as it is then:

- A key: its own limit, else its role's. A platform admin key may set 0. A
  key that is revoked or expired has the default of the role it had.
- A person who signs in with OIDC: the current mapping rules, run on the
  groups and email of their newest login session in the tenant. With no
  session, or when no rule maps them to the tenant any more, the default of
  the role they had.
- A local account: the default of its role now; a disabled or deleted
  account, the default of the role it had.

So a package published by a key limited to 5 that declares priority 2 runs at
5. When the key's limit is raised to 0, its next run is at 2, with no new
publish; when it is lowered again, the next run is held to it. A package an
admin published runs at what it declares. A package published before the
server kept its publisher is held to the default of `write`.

The upload answer gives `priority_limit`, the limit its runs are held to now,
and a `priority_notice` when that is not 0; `cloacinactl package upload` and
`package publish` print the notice as a warning. The package's priority as
its code declares it is known once it is built: the workflow detail
(`GET .../workflows/{name}`, and the package view of the UI) then shows
`priority` with `asked` (what the code declares) and `given` (what its runs
get now), and a `priority_notice` when they differ.

A run that was held writes a warning to the log and the audit event
`priority.held` (target kind `package` or `schedule`; detail `workflow`,
`set_by`, `set_by_role`, `priority_asked`, `priority_given`,
`priority_lowered`) when its given priority changes, not at each run. The
last given priority is kept in the database, so a restart or a second server
writes no repeat. `cloacina_priority_held_total{tenant}` counts each held run.

A run that asks for a priority itself is held to the limit of its key
instead, so an `admin` key can run any workflow at 0 to 10.

An embedded program has no principals. A priority it declares or asks for is
given as asked.

## In process and on a fleet

In process, a task that finds no free executor slot stays in the queue. The
next pass of the scheduler orders all waiting tasks again, so a more
important task that became ready later goes ahead of a backlog.

On a fleet, a task that fits no agent now is put off for two seconds and
keeps its place in the order. While a task that was given a better priority
is put off, the worse tasks of its [pool](../service/execution-agent-fleet/#agent-pools)
are not dispatched: the room that frees next is for the better task. A worse
task is held back only until it has aged level with the better one. Tasks
that were given the same priorities never hold each other back, so work that
sets no priority packs onto agents as it did before priorities.

## Watch the queue

The waiting-task gauges `cloacina_waiting_tasks` and
`cloacina_oldest_waiting_task_age_seconds` carry a `priority` label: the
workflow priority stored on the task, `0` to `10`. Ageing does not change
it. So `sum by (tenant, priority) (cloacina_waiting_tasks)` shows how much
work of each priority waits.

A task row keeps two task priorities: the one its task declares
(`task_priority_asked`, empty when it declares none) and the one it was
given (`task_priority`).

## History loads

A load of history can put thousands of tasks in a tenant's queue. Give the
workflow that loads history a low priority, for example
`#[workflow(priority = 9)]`, or run the load with
`cloacinactl workflow run <name> --priority 9`. Each slot that frees then
goes to the daily work first, and the load uses what is left. Ageing still
moves the load forward: after eight minutes of waiting a task of the load is
level with new work at the default priority.

## See also

- [Macro reference](../../reference/macros/): `#[workflow(priority = N)]` and
  `#[task(priority = N)]`.
- [Python `@task`](../../reference/python-api/task/).
- [Security model](../service/security-model/): roles and limits.
- [Metrics catalog](../../reference/metrics-catalog/):
  `cloacina_priority_held_total`.
