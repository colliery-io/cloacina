---
title: "Metrics Catalog"
description: "Complete reference for every cloacina_* and cloacina_compiler_* Prometheus metric: name, type, labels, meaning, and example PromQL."
sidebar:
  order: 15
---

Both `cloacina-server` and `cloacina-compiler` expose metrics in the
standard Prometheus text exposition format at `GET /metrics`. Both
endpoints are public (no auth) so Prometheus can scrape them without
credential management. By default `/metrics` is on each binary's main
listener; `--metrics-bind` on either binary serves it only on a separate
address, and the main listener then answers 404 for it.

`cloacinactl daemon` does **not** expose `/metrics` — it's a
hobbyist-tier local process per ADR-0005 (Deployment-Mode Trust
Model); observe it via logs. If a "daemon-as-service" deployment mode
emerges, revisit.

The exposition format of both binaries is version-checked in CI via
`angreal test metrics-format`, which boots each one and pipes `/metrics`
through `promtool check metrics`. If that job fails, a recent metric
change broke the format — see T-0536 (server) + T-0591 (compiler) for
how the check is wired.

## Metric reference

All metric names are prefixed `cloacina_`. All labels have bounded
cardinality — no task IDs or package names are used as labels. Adding a new
metric should preserve this invariant; see
[Adding a metric](#adding-a-metric) at the bottom.

Two labels come from names that a tenant or an operator chooses
(CLOACINA-T-1024):

- `tenant` is the tenant schema of the runner that emits the metric
  (`public` for the default schema and for an embedded runner). One value
  per tenant: the count is the number of tenants an admin has created.
- `workflow` is the workflow name. A tenant chooses it in the package it
  uploads, so it is capped: the first 100 names seen for a tenant in a
  server process keep their name, each later one is counted under
  `workflow="other"`. The set starts empty after a restart. `unknown` is
  used when a task name has no workflow part.

Series per counter are at most tenants × 101 workflows × the `status` and
`reason` values. If that is too many for your Prometheus, drop the `workflow`
label at scrape time with a `metric_relabel_configs` `labeldrop` rule.

Every metric with the `tenant` label grows with the number of tenants; no
setting caps it. The server writes a histogram as a Prometheus summary: each
label set is 9 series (7 `quantile` values, `_sum` and `_count`). So
`cloacina_context_size_bytes` (`kind`, `tenant`) is 18 series for each tenant
that has run a task: 1,000 tenants give 18,000. A gauge with only the `tenant`
label is one series for each tenant.

The server's cardinality test (`test_i0099_cardinality_within_ceiling`) counts
the label sets of one tenant: it leaves the `tenant` label and the `quantile`
label of a summary out of a label set, and fails a metric that then has more
than 64 label sets (256 for the two API request metrics). It is a limit of the
test, on the labels other than `tenant`; a running server has no such limit.

### Counters

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_workflows_total` | `status`, `reason`, `workflow`, `tenant` | Total workflow executions. `status` ∈ `completed`, `failed`. `reason` is `ok` on success, `dependency_failed` on failure (workflow failure is always downstream of task failure). `workflow` is the workflow name (capped, see above); `tenant` is the tenant schema. |
| `cloacina_tasks_total` | `status`, `reason`, `workflow`, `tenant` | Total task attempts, counted where the result is written, so a task run in-process and a task run by a fleet agent are counted the same way. `status` ∈ `completed`, `failed`, `retried` (the attempt failed and a retry is scheduled). `reason` is `ok` on success, or one of: `task_error`, `timeout`, `validation_failed`, `infrastructure`, `context_load_failed`, `context_too_large` (a context over the hard size limit), `task_not_found`, `claim_lost`, `unknown`. `workflow` and `tenant` as above. A dispatch that is put off for capacity is not an attempt and is not counted. |
| `cloacina_api_requests_total` | `method`, `status` | Total HTTP API requests. `method` is the HTTP verb; `status` is the numeric HTTP status code. |
| `cloacina_scheduler_claim_attempts_total` | `outcome` | Total task claim attempts. `outcome` ∈ `claimed` (claim succeeded), `contended` (another runner already held the claim), `empty` (scheduler tick found no ready tasks to dispatch). |
| `cloacina_scheduler_heartbeat_writes_total` | — | Total successful heartbeat writes by the per-task heartbeat loop. Failed heartbeats are recorded only in logs. |
| `cloacina_scheduler_stale_claims_swept_total` | — | Total stale claims released by the stale-claim sweeper. Each increment corresponds to one task whose runner heartbeat had expired and was reset to Ready. |
| `cloacina_supervisor_restarts_total` | `graph`, `component`, `reason` | Total computation-graph supervisor restarts. `component` ∈ `reactor` or an accumulator name. `reason` is `panic` (JoinError::is_panic), `error` (any other terminated handle), or `shutdown_timeout` (graceful-shutdown path). |
| `cloacina_accumulator_events_total` | `graph`, `accumulator`, `kind` | Total events processed by computation-graph accumulators. `kind` ∈ `passthrough`, `stream`, `polling`, `batch`. `graph` is the deployed graph name (or `embedded` for runtimes without a DAL). |
| `cloacina_accumulator_checkpoint_writes_total` | `graph`, `accumulator` | Total successful checkpoint writes via `CheckpointHandle::save` or `persist_boundary`. Failed writes appear only in logs. |
| `cloacina_reactor_fires_total` | `graph`, `reactor`, `strategy` | Total reactor fires (graph executions). `strategy` ∈ `when_any`, `when_all`, `sequential` — projects the (criteria × input_strategy) axes onto a single bounded label. |
| `cloacina_reactor_deduped_events_total` | `graph`, `reactor`, `source` | Boundary events the reactor rejected as duplicates of an already-seen emission sequence. **Reserved** — the reactor-side dedup path lands as a follow-up to T-0413; the metric is registered today so dashboards and alert rules can be authored against the eventual name. |
| `cloacina_ws_messages_total` | `endpoint`, `direction` | WebSocket framed messages by `endpoint` (`accumulator` \| `reactor`) and `direction` (`in` \| `out`). Ping/pong handled by axum are excluded. |
| `cloacina_ws_auth_failures_total` | `reason` | Rejected WebSocket upgrade requests. `reason` ∈ `ticket_expired`, `invalid_signature`, `tenant_mismatch`, `not_authorized`. |
| `cloacina_reactor_persist_failures_total` | `graph`, `reactor`, `kind` | Reactor state-persistence failures. `kind` ∈ `cache_serialize`, `dirty_serialize`, `seq_serialize`, `save`. The reactor downgrades to `Degraded` after 5 consecutive failures and recovers on the next success. |
| `cloacina_accumulator_persist_failures_total` | `graph`, `accumulator`, `kind` | Accumulator persist failures. `kind` ∈ `checkpoint` (polling save), `boundary` (persist_boundary), `batch_buffer` (batch buffer save). |
| `cloacina_context_merge_failures_total` | `kind` | Failures merging dependency contexts. `kind` ∈ `parse` (JSON deserialize failed — fails the task as `ContextLoadFailed`), `merge` (Context API rejected an insert/update; counted but does not fail the task). Closes COR-11. |
| `cloacina_context_size_warnings_total` | `tenant` | Task contexts over the warning size (`--context-warn-bytes`, default 1 MiB) and not over the hard limit. Counted for the context a task returns and for the input context built for a task. Each one also writes a warn log with the task, the size and the largest keys. Growth means a workflow passes data in its context that belongs in storage. See [Context size limits](../../explanation/engine/context-management/#context-size-limits). CLOACINA-T-1047. |
| `cloacina_audit_write_errors_total` | — | Audit rows that could not be written to `audit_events`. The action itself still ran; its audit row is missing. Alert on any increase. See [Read the Audit Record](../../how-to/service/security/audit-record/). |
| `cloacina_fleet_agents_evicted_total` | — | Execution-agent fleet: agents removed by the heartbeat sweeper after their heartbeat went stale (older than `--agent-liveness-misses` × the advertised interval). Sustained non-zero means agents are dying or losing connectivity. CLOACI-I-0114 / T-0634. |
| `cloacina_fleet_work_reassigned_total` | — | Execution-agent fleet: in-flight `delivery_outbox` rows re-targeted from an evicted (dead) agent to a live agent by the sweeper's reclaim path. Tracks how much work crashed agents shed onto the rest of the fleet. CLOACI-T-0634. |
| `cloacina_fleet_dispatches_taken_over_total` | `outcome` | Execution-agent fleet: dispatches whose server replica stopped (claim heartbeat older than 60 s), taken up by another replica. `outcome` ∈ `waiting` (the packet is open; this replica waits for the report), `stored_report` (the report was stored while no replica waited; this replica applies it), `lost` (no report comes; the stale-claim sweep sends the task again). Rises at each server restart or rolling upgrade with fleet tasks in flight. A series appears after its first takeover. CLOACINA-T-1084. |
| `cloacina_fleet_orphan_results_total` | — | Execution-agent fleet: agent reports that no dispatch on any replica waited for (the report of an older dispatch of a task, a second report, or a report for a dispatch that no replica took over). Counted when the stored report is deleted by age, about 15 minutes after it came. The report changes no state. Each one is work that ran and was not used. CLOACINA-T-1013, CLOACINA-T-1015. |
| `cloacina_fleet_cancels_sent_total` | — | Execution-agent fleet: cancel messages the server sent to an agent for a dispatch whose result wait ended with no report. The agent kills the task's worker process. Each one is an attempt that an agent did not report in the task's time limit plus 30 seconds. CLOACINA-T-1028. |
| `cloacina_fleet_worker_starts_total` | `start` | Execution-agent fleet: jobs (tasks and graph firings) that agents ran in a worker process, from the agents' heartbeats. `start` ∈ `cold` (a new worker process, which loads the package), `warm` (a [warm worker](../../explanation/service/execution-agent-fleet/#warm-workers) that kept the package loaded). Inline jobs are not counted. A low share of `warm` means jobs pay the package load. A series appears after its first count. CLOACINA-T-1126. |
| `cloacina_fleet_warm_workers_retired_total` | `reason` | Execution-agent fleet: warm workers that agents ended, from the agents' heartbeats. `reason` ∈ `idle`, `max_jobs`, `memory` (it held more memory after a job than its agent allows), `dirty` (a job changed the environment or left a process running), `pool_full`, `pressure` (the agent was above 80% of its memory), `dead`, `other`. A worker killed with its job (memory limit, time limit, cancel) is not counted. A series appears after its first count. CLOACINA-T-1126. |
| `cloacina_fleet_refused_results_total` | `reason` | Execution-agent fleet: agent reports for a dispatch this replica waits for that were refused because of their sender. `reason` ∈ `other_tenant` (the agent is in another tenant than the dispatch), `not_the_target` (the work packet is not addressed to that agent). The report changes no state and is not stored. An honest agent does not cause one. A series appears after its first refusal. CLOACINA-T-1050. |
| `cloacina_trigger_polls_total` | `outcome` | Polls of poll triggers by the scheduler. `outcome` ∈ `fire`, `skip`, `error` (no answer, or the trigger's error; the reason is in the trigger's `last_poll_error`), `in_flight` (an interval skipped because the last poll of the trigger still runs). CLOACINA-T-1077. |
| `cloacina_fleet_polls_total` | `answer` | Execution-agent fleet: polls of poll triggers sent to agents. `answer` ∈ `fire`, `skip`, `failed` (the trigger returned an error on the agent), `missed` (no answer: see `cloacina_fleet_polls_missed_total`). Each poll is one dispatch. CLOACINA-T-1077. |
| `cloacina_fleet_polls_missed_total` | `reason` | Execution-agent fleet: polls that got no answer, so nothing fired. `reason` ∈ `no_build` (no active build of the package), `no_agent` (no live agent of the tenant that runs polls has a free slot), `enqueue`, `no_answer` (no answer within `--fleet-poll-timeout-s` plus 10 s), `refused` (the agent did not run it), `bad_answer`. Sustained `no_agent` means the tenant has no agent of this release running. CLOACINA-T-1077. |
| `cloacina_fleet_poll_floor_tenants` | — | Gauge. Tenants held at one agent or more because they have a poll trigger, as read on the last tick of the fleet loop. Set only by the replica that led the tick. CLOACINA-T-1077. |
| `cloacina_fleet_undecodable_packets_total` | `kind` | Execution-agent fleet: work packets that their agent could not decode and reported with `POST /v1/agent/undecodable`. `kind` ∈ `agent_work` (a task), `agent_graph` (a graph firing), `agent_poll` (a poll of a poll trigger). The dispatch fails at once with an error that names the two versions. Any value above zero means a server and an agent of different versions: run the same version on both. A series appears after its first packet. CLOACINA-T-1058. |
| `cloacina_fleet_refused_undecodable_total` | — | Execution-agent fleet: `POST /v1/agent/undecodable` calls that named a delivery row not addressed to the calling agent in its tenant. Nothing changes. An honest agent does not cause one. CLOACINA-T-1058. |
| `cloacina_priority_held_total` | `tenant` | Runs that started at a lower [priority](../../explanation/priority/) than the one asked for, because who asked may not set that priority: a run request above the limit of its key, or a package that declares more than a package may. One for each such run. CLOACINA-T-1045. |
| `cloacina_fleet_dispatch_deferred_total` | — | Execution-agent fleet: dispatch tries put back in the queue because no agent could take the task (no live agent of its tenant had a free slot, or the chosen agent was full). The task keeps its attempt count and is tried again after 2 s. Sustained growth means a tenant needs more agents. CLOACINA-T-1012. |
| `cloacina_fleet_artifact_fetch_refusals_total` | — | Execution-agent fleet: dispatches that an agent refused because it could not fetch the task's package artifact or source. The task waits and is dispatched again, with no use of a retry attempt, for up to `--fleet-artifact-fetch-wait-s` per attempt; the refusal at or past that limit fails the attempt and is counted too. A short burst is a server restart or an artifact still on its way; sustained growth means agents cannot reach the server's artifact route, or a package artifact is gone. CLOACINA-T-1064. |
| `cloacina_tenant_compiler_starts_total` | — | Per-tenant compilers started by the fleet loop: one for each time a tenant's build queue got work while its compiler was not running. Each start is a cold start: the compiler's cargo caches are empty. A high rate against the build rate means the cooldown (`CLOACINA_TENANT_COMPILER_COOLDOWN_S`) is shorter than the gap between a tenant's publishes. CLOACINA-T-1034. |
| `cloacina_delivery_outbox_sweep_runs_total` | — | Delivery-outbox sweeper: total sweep passes executed. Complements the `cloacina_delivery_outbox_open` gauge. |
| `cloacina_delivery_outbox_sweep_redeliveries_total` | — | Delivery-outbox sweeper: total outbox rows re-delivered by a sweep pass (e.g. after a missed acknowledgement). |
| `cloacina_delivery_refused_acks_total` | — | Delivery WebSocket: acks that named an outbox row not addressed to the connection's own recipient and tenant. The row is left as it is. An evicted agent that acks a packet re-targeted to another agent counts one; more means a client acks rows it was not sent. CLOACINA-T-1052. |
| `cloacina_delivery_refused_subscriptions_total` | `reason` | Delivery WebSocket: upgrades refused because the key may not open the recipient. `reason` ∈ `unknown_agent`, `other_tenant`, `other_key`, `no_owner`, `no_tenant`, `unknown_recipient`. A series appears after its first refusal. CLOACINA-T-1051. |
| `cloacina_alert_webhook_sent_total` | `event` | Alert webhook: requests the receiver answered with a 2xx status. `event` ∈ `failed`, `completed`, `cancelled`. Zero while no `--alert-webhook-url` is set. See [Alert on a Failed Run](../../how-to/service/alert-on-a-failed-run/). CLOACINA-T-1024. |
| `cloacina_alert_webhook_failed_total` | `reason` | Alert webhook: sends that failed and will be tried again. `reason` ∈ `timeout`, `status` (a status outside 2xx), `transport` (no connection, TLS, DNS), `load` (the run could not be read). Sustained growth means the receiver is down or slow. |
| `cloacina_alert_webhook_dropped_total` | `reason` | Alert webhook: alerts given up without a 2xx answer. `reason` ∈ `attempts` (`--alert-webhook-max-attempts` sends failed), `expired` (not sent 24 hours after the run ended), `invalid` (the stored notice could not be read). Each one is an alert that nobody got. |
| `cloacina_reactor_firings_total` | `graph`, `reactor` | `reactor_firings` rows written by a reactor, one for each fire. The subscription poller reads these rows. Counts successful writes only; a failed write is in the logs. |
| `cloacina_reactor_firings_pruned_total` | — | Reactor firing-history rows pruned during retention cleanup. Each increment counts one pruned firing record. |
| `cloacina_reactor_predicate_errors_total` | `reactor` | Reactor subscription predicates that failed to evaluate. The firing is not dropped: the subscription holds its watermark and tries the firing again on the next tick. |
| `cloacina_reactor_predicate_dead_letters_total` | `reactor` | Reactor firings given up after their subscription's predicate failed 5 times in a row. The subscription is marked degraded and moves past the firing. Each one is a firing that started no workflow. |
| `cloacina_scheduler_tasks_abandoned_total` | — | Tasks the stale-claim sweeper marked `Failed` with an `ABANDONED:` error, because their claim went stale after the last recovery the cap allows. Each one also writes a `TaskAbandoned` event. |
| `cloacina_auth_login_attempts_total` | `outcome` | Local-login attempts. `outcome` ∈ `ok` (password verified, key minted), `denied` (unknown user, wrong password or disabled account; the three are not told apart), `throttled` (refused before the password check, because a throttle key was locked). A sustained `denied` rate is a credential-stuffing signal. |
| `cloacina_auth_login_lockouts_total` | `scope` | Local-login lockouts, counted once when the lock starts, not once per refused attempt. `scope` ∈ `username`, `ip`. Pairs with the `auth.login.lockout` audit event. |
| `cloacina_python_import_interrupted_total` | — | Python package imports that hung and were interrupted: the load failed and the interpreter stayed usable. |
| `cloacina_retention_rows_deleted_total` | `table` | Rows removed by the server's background retention job, summed over the admin schema and all tenant schemas. `table` ∈ `workflow_executions`, `task_executions`, `task_execution_metadata`, `contexts`, `execution_events`, `recovery_events`, `schedule_executions`, `delivery_outbox`, `workflow_execution_package_pins` (CLOACINA-T-1130), `task_attempt_errors` (CLOACINA-T-1100). Only the replica that holds the retention lock for a tick counts. A series appears after its first removed row. CLOACINA-T-1023. |
| `cloacina_retention_errors_total` | — | Retention passes over one schema that failed (also: the tenant schema list could not be read). The pass is retried on the next tick. Sustained growth means run history is not being removed. CLOACINA-T-1023. |

### Histograms

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_api_request_duration_seconds` | `method`, `status` | Handler duration for HTTP API requests, measured inside the `api_request_metrics` middleware. |
| `cloacina_workflow_duration_seconds` | — | Wall-clock duration from workflow execution start to finalize (success or failure). |
| `cloacina_task_duration_seconds` | — | Wall-clock duration of a task attempt, from the start of its dispatch to the result, including timeouts. Recorded where the result is written, for the in-process and the fleet executor. For a fleet task it includes the hand-off to the agent and the wait for its report; a dispatch that fails before it reaches an agent is recorded with its short duration. |
| `cloacina_context_size_bytes` | `kind`, `tenant` | Serialized size in bytes of a task context. `kind` ∈ `result` (the context a task returned, measured where the result is written, for the in-process and the fleet executor), `input` (the context built for a task from its dependencies, measured before the task runs or is sent to an agent). A context over the hard limit is recorded too. The size of each saved result is also on the task (`context_size_bytes` in `GET /v1/tenants/{tenant_id}/executions/{exec_id}/tasks`). CLOACINA-T-1047. |
| `cloacina_accumulator_emit_duration_seconds` | `graph`, `accumulator` | End-to-end emit latency per accumulator event: time from the event arriving on the merge channel through `process()`, boundary send, and checkpoint persistence. |
| `cloacina_reactor_fire_duration_seconds` | `graph`, `reactor` | Wall-clock duration of the user's compiled graph body (time inside `(graph)(snapshot).await`). Excludes cache lookup + persistence. |

### Gauges

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_active_workflows` | `tenant` | Workflow executions in `Pending` or `Running` state, per tenant. SQL-derived — re-seeded every scheduler tick from `workflow_executions` row count, so the value is correct by construction across crashes, claim loss, and finalize-path errors. Lags real DB state by at most one scheduler `poll_interval`. A server runs one scheduler per tenant, so each tenant has its own series; sum them for the server total. |
| `cloacina_active_tasks` | `tenant` | Task executions in the `Running` state, per tenant. SQL-derived — re-seeded every scheduler tick from a `task_executions WHERE status = 'Running'` count, so the value is correct by construction across crashes, claim loss, and panic-between-inc-and-dec paths. Lags real DB state by at most one scheduler `poll_interval`. One series per tenant; sum them for the server total. |
| `cloacina_waiting_tasks` | `tenant`, `pool`, `priority` | Tasks that wait for capacity, per tenant, [agent pool](../../explanation/service/execution-agent-fleet/#agent-pools) and workflow [priority](../../explanation/priority/): status `Ready` and past their `waiting_since`. `pool` is the pool the task names; a task with no pool is in `default`. `priority` is the workflow priority stored on the task, `0` to `10` (at most 11 values per pool; ageing does not change it). A tenant with no pools and no priorities has the one series `pool="default", priority="5"`. A series goes back to `0` when its last waiting task leaves. Sum over `priority` for the figure of a pool. `waiting_since` is a column of `task_executions`: it is set when the task becomes `Ready` and cleared when the task starts or ends. SQL-derived, read every 5 seconds. In fleet mode a task that found no free agent is put off for 2 s at a time; it is counted the whole time, also while the server holds it for a dispatch try. In process, a task that waits for a free executor slot is counted. A task in a retry backoff is not counted until the retry is due: its `waiting_since` is the retry's due time. `cloacina_fleet_dispatch_deferred_total` counts the dispatch tries that were put off. A fleet task that waits because agents cannot fetch its package artifact is counted here too. |
| `cloacina_oldest_waiting_task_age_seconds` | `tenant`, `pool`, `priority` | Seconds since the oldest waiting task of the pool at that workflow priority began to wait: now minus the earliest `waiting_since` (the time the task became `Ready`, or the due time of a retry); `0` when no task waits. A deferral for capacity does not restart it. Read with `cloacina_waiting_tasks`. Grows when tasks are ready and nothing runs them. |
| `cloacina_schedule_lag_seconds` | `tenant` | Seconds since the oldest `next_run_at` of the enabled, not paused cron schedules; `0` when no schedule is due. Set every second by the cron scheduler from the next-due time it holds, which it reads again after each cron check and at least once per cron poll interval. A due schedule that is not picked up keeps its `next_run_at`, so the value grows until it runs. An enabled schedule past its `end_date` also keeps a past `next_run_at` and shows as lag: disable it. |
| `cloacina_component_health` | `graph`, `component`, `state` | One-of indicator for a computation-graph component's current health. For each `(graph, component)` tuple the gauge is `1` on the current state and `0` on every other state. `state` is bounded: `healthy`, `degraded`, `starting`, `stopped`, `crashed`. Re-emitted every supervisor tick. |
| `cloacina_accumulator_buffer_depth` | `graph`, `accumulator` | Current internal buffer size for buffered accumulators. Meaningful for `batch` and stateful `stream` kinds; `passthrough` and `polling` emit `0` from runtime startup so dashboards see a stable series per (graph, accumulator). |
| `cloacina_reactor_cache_age_seconds` | `graph`, `reactor`, `source` | Age in seconds of the most-recent emission per source held in the reactor's input cache. Refreshed on every boundary arrival (all known sources re-emitted, so silent sources show increasing staleness). |
| `cloacina_ws_connections_active` | `endpoint` | Currently open WebSocket connections. `endpoint` ∈ `accumulator`, `reactor`. RAII-guarded so panics inside the handler still decrement on Drop. |
| `cloacina_tenant_compiler_tenants_pending` | — | Tenants whose build queue has a `pending` or `building` package, read on each tick of the fleet loop. The `public` schema is not counted. Set only by the replica that led the tick, and only when the server runs a compiler per tenant. A value that stays above 0 while `cloacina_tenant_compiler_starts_total` does not move means compilers do not start or do not finish. CLOACINA-T-1034. |
| `cloacina_python_runtime_wedged` | — | `1` when the embedded Python interpreter of the process is wedged (an import that could not be interrupted holds it), else `0`. Set to `0` when a Python runtime starts. While it is `1`, Python package loads in the process hang or fail and `/ready` answers 503: restart the process. |
| `cloacina_delivery_outbox_open` | — | Current count of non-`acked` rows in `delivery_outbox` (`pending` + `delivered`) — the durable push queue that carries fleet work packets to agents. Sustained growth means delivery is wedged (no live agent for the recipient, or the relay isn't draining). CLOACI-I-0115. |

## Example PromQL queries

### Task throughput (per second, last 5m)

```promql
sum(rate(cloacina_tasks_total[5m]))
```

### Workflow failure rate, by reason

```promql
sum by (reason) (
  rate(cloacina_workflows_total{status="failed"}[5m])
)
```

### Task p95 duration

```promql
histogram_quantile(
  0.95,
  sum by (le) (rate(cloacina_task_duration_seconds_bucket[5m]))
)
```

### API request p95 latency by method

```promql
histogram_quantile(
  0.95,
  sum by (le, method) (rate(cloacina_api_request_duration_seconds_bucket[5m]))
)
```

### Currently in-flight work

```promql
sum(cloacina_active_workflows)
sum(cloacina_active_tasks)
```

Per tenant:

```promql
sum by (tenant) (cloacina_active_workflows)
sum by (tenant) (cloacina_active_tasks)
```

### Failed runs, by workflow and tenant

```promql
sum by (tenant, workflow) (
  increase(cloacina_workflows_total{status="failed"}[15m])
) > 0
```

### Backlog: tasks wait and nothing takes them

Alert when a task has waited for capacity for more than 5 minutes.

```promql
cloacina_oldest_waiting_task_age_seconds > 300
```

```promql
sum by (tenant) (cloacina_waiting_tasks)
```

The waiting tasks of one pool, which is what to size the pool by:

```promql
sum by (tenant) (cloacina_waiting_tasks{pool="large"})
```

The waiting tasks of each workflow priority, to see whether low priority
work (a history load) holds the queue:

```promql
sum by (tenant, priority) (cloacina_waiting_tasks)
```

### Schedules run late

Alert when a cron schedule is more than 2 minutes past its due time.

```promql
cloacina_schedule_lag_seconds > 120
```

### Task failure breakdown (which reason is dominant?)

```promql
sum by (reason) (
  increase(cloacina_tasks_total{status="failed"}[1h])
)
```

### Scheduler claim contention rate

High `contended` rates indicate two or more runners are racing for the
same tasks; persistent non-zero `empty` rates are healthy — they just
mean there is no pending work in the outbox.

```promql
sum by (outcome) (
  rate(cloacina_scheduler_claim_attempts_total[5m])
)
```

### Stale-claim sweep activity

A non-zero rate of stale claims released means a runner is crashing or
losing its heartbeat — investigate executor logs for the affected
window.

```promql
rate(cloacina_scheduler_stale_claims_swept_total[5m])
```

### Graphs currently in Degraded state

The supervisor downgrades a reactor to `Degraded` after 5 consecutive
persist failures (see I-0108 / T-0590) and recovers on the next
success. This query lists every graph currently reporting Degraded
health — operators triage by checking the matching `*_persist_failures_total`
counters by `kind`.

```promql
cloacina_component_health{component="reactor",state="degraded"} == 1
```

## Compiler metrics

`cloacina-compiler` exposes the following `cloacina_compiler_*` family
on its own `/metrics` endpoint (default `127.0.0.1:9000`, shared with
`/health` and `/v1/status`).

### Counters

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_compiler_builds_total` | `status` | Total cargo builds executed. `status` ∈ `ok`, `failed`, `timed_out`. Timed-out rows are left for the stale-build sweeper. |
| `cloacina_compiler_sweep_resets_total` | — | Stale builds reset to `pending` by the sweeper. Each increment corresponds to one row reclaimed. Sustained non-zero rate indicates worker crashes or hung builds. |
| `cloacina_compiler_describes_total` | `status` | Describes of packages built before the package metadata document existed, read from their stored build. `status` ∈ `ok`, `failed`. A failed describe is recorded on the package (`describe_error`). |
| `cloacina_compiler_heartbeat_failures_total` | — | Heartbeat-update failures from the builder. Repeated failures starve `build_claimed_at` and the sweeper will eventually reclaim the row. |

### Histograms

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_compiler_build_duration_seconds` | — | Wall-clock duration of `execute_build` — covers the cargo subprocess from spawn through artifact persistence. Independent of result status. |

### Gauges

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_compiler_queue_depth` | `state` | Build queue size. `state` ∈ `queued`, `building`. SQL-derived — re-seeded every sweep tick from `compiled_data` row counts (REC-06 pattern), so it cannot drift on crash. |

### Example PromQL — compiler build rate by outcome

```promql
sum by (status) (rate(cloacina_compiler_builds_total[5m]))
```

### Example PromQL — compiler build p95 duration

```promql
histogram_quantile(
  0.95,
  sum by (le) (rate(cloacina_compiler_build_duration_seconds_bucket[5m]))
)
```

## Current gaps

Computation-graph observability for the full event-driven stack ships
in CLOACI-I-0099. The remaining gaps are reactor-side dedup wiring
(the `cloacina_reactor_deduped_events_total` metric is registered but
its emit path lands as a follow-up to T-0413) and any operator-defined
SLO/alert rules (out of scope for the metrics surface itself).
Operators running CG workloads should rely on logs plus
`/v1/health/graphs` and `/v1/health/accumulators` for observability
until CLOACI-I-0099 (Computation Graph Observability) is fully
realized. Scheduler-loop signals (claim attempts, heartbeat writes,
stale claim sweeps) are now covered by the `cloacina_scheduler_*`
family above. See I-0099 for the remaining planned metric set.

The audit that produced the current state is CLOACI-T-0498.

## Adding a metric

When introducing a new metric:

1. Register it with `metrics::describe_counter!` / `describe_histogram!` /
   `describe_gauge!` in the appropriate startup path (server metrics go in
   `crates/cloacina-server/src/lib.rs`; core-engine metrics go wherever the
   emission lives and are picked up by the global recorder).
2. Keep label cardinality bounded. Labels must be enums or derived from
   package/tenant metadata — **never** event keys, raw paths, or free-form
   user input. For a workflow name use
   `cloacina::metric_labels::workflow_label`, which caps the values per
   tenant; for the tenant use `DAL::tenant_label`.
3. Update this document with a row, an operator-facing description, and at
   least one example PromQL query if the metric is meant to drive an alert
   or dashboard.
4. Run `angreal test metrics-format` locally before pushing. The same
   check runs in CI (`metrics-format` job in `.github/workflows/cloacina.yml`).
5. The [release gate](../../contributing/release-gate/) reads this page: a
   metric name or label in the code that has no row here fails it, and so
   does a row that no code registers or records. A name or label removed from
   this page since the last release needs a `CHANGELOG.md` entry.

## Related

- [Compiler + Server Deployment Runbook](../../how-to/service/compiler-deployment-runbook/) — how to deploy and operate the metric-emitting binaries.
- [Performance Tuning](../../how-to/service/performance-tuning/) — uses these metrics to drive tuning decisions.
- [Workflows: Observability](../../how-to/embed/observe-execution-state/) — workflow-author perspective on which metrics matter when.
- ADR-0005 — Deployment-mode trust model (`/metrics` posture per deployment mode).
