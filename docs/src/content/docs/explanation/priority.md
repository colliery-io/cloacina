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

## Who may set which priority

A principal has a limit: the highest priority it may set. The limit comes
from its role.

| Role | May set |
|---|---|
| `admin` | 0 to 10 |
| `write` | 5 to 10 |
| `read`, `agent` | none |

A run that asks for a priority above the limit of its key is not refused.
It runs at the limit, and the server says so:

- The answer carries `priority` with both numbers and `priority_notice` with
  one sentence that says what was asked, what was given and why.
  `cloacinactl workflow run` prints the notice as a warning on stderr.
- The server logs a warning with the key and the tenant.
- The audit event of the run (`workflow.run`) records `priority_asked`,
  `priority_given` and `priority_lowered`.
- `cloacina_priority_held_total{tenant}` counts the run.

A package declares its priorities in its code. On a server, a priority that a
package declares is held to what a `write` principal may set when each run
starts: a package that declares priority 2 runs at 5. The execution shows
asked 2 and given 5, the counter counts each such run, and the server logs a
warning when the given priority of a workflow changes. A run that asks for a
priority itself is held to the limit of its key instead, so an `admin` key
can run any workflow at 0 to 10.

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
