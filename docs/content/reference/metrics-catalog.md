---
title: "Metrics Catalog"
description: "Complete reference for every cloacina_* and cloacina_compiler_* Prometheus metric: name, type, labels, meaning, and example PromQL."
weight: 15
aliases:
  - "/platform/reference/metrics-catalog/"

---

# Metrics Catalog

Both `cloacina-server` and `cloacina-compiler` expose metrics in the
standard Prometheus text exposition format at `GET /metrics`. Both
endpoints are public (no auth) so Prometheus can scrape them without
credential management.

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
cardinality — no task IDs, tenant IDs, workflow names, or package names are
used as labels. Adding a new metric should preserve this invariant; see
[Adding a metric](#adding-a-metric) at the bottom.

### Counters

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_workflows_total` | `status`, `reason` | Total workflow executions. `status` ∈ `completed`, `failed`. `reason` is `ok` on success, `dependency_failed` on failure (workflow failure is always downstream of task failure). |
| `cloacina_tasks_total` | `status`, `reason` | Total task executions. `status` ∈ `completed`, `failed`. `reason` is `ok` on success, or one of: `task_error`, `timeout`, `validation_failed`, `infrastructure`, `context_load_failed`, `task_not_found`, `claim_lost`, `unknown`. |
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
| `cloacina_fleet_agents_evicted_total` | — | Execution-agent fleet: agents removed by the heartbeat sweeper after their heartbeat went stale (older than `--agent-liveness-misses` × the advertised interval). Sustained non-zero means agents are dying or losing connectivity. CLOACI-I-0114 / T-0634. |
| `cloacina_fleet_work_reassigned_total` | — | Execution-agent fleet: in-flight `delivery_outbox` rows re-targeted from an evicted (dead) agent to a live agent by the sweeper's reclaim path. Tracks how much work crashed agents shed onto the rest of the fleet. CLOACI-T-0634. |

### Histograms

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_api_request_duration_seconds` | `method`, `status` | Handler duration for HTTP API requests, measured inside the `api_request_metrics` middleware. |
| `cloacina_workflow_duration_seconds` | — | Wall-clock duration from workflow execution start to finalize (success or failure). |
| `cloacina_task_duration_seconds` | — | Wall-clock duration from task execution start to end, including timeouts. |
| `cloacina_accumulator_emit_duration_seconds` | `graph`, `accumulator` | End-to-end emit latency per accumulator event: time from the event arriving on the merge channel through `process()`, boundary send, and checkpoint persistence. |
| `cloacina_reactor_fire_duration_seconds` | `graph`, `reactor` | Wall-clock duration of the user's compiled graph body (time inside `(graph)(snapshot).await`). Excludes cache lookup + persistence. |

### Gauges

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_active_workflows` | — | Workflow executions in `Pending` or `Running` state. SQL-derived — re-seeded every scheduler tick from `workflow_executions` row count, so the value is correct by construction across crashes, claim loss, and finalize-path errors. Lags real DB state by at most one scheduler `poll_interval`. |
| `cloacina_active_tasks` | — | Task executions in the `Running` state. SQL-derived — re-seeded every scheduler tick from a `task_executions WHERE status = 'Running'` count, so the value is correct by construction across crashes, claim loss, and panic-between-inc-and-dec paths. Lags real DB state by at most one scheduler `poll_interval`. |
| `cloacina_component_health` | `graph`, `component`, `state` | One-of indicator for a computation-graph component's current health. For each `(graph, component)` tuple the gauge is `1` on the current state and `0` on every other state. `state` is bounded: `healthy`, `degraded`, `starting`, `stopped`, `crashed`. Re-emitted every supervisor tick. |
| `cloacina_accumulator_buffer_depth` | `graph`, `accumulator` | Current internal buffer size for buffered accumulators. Meaningful for `batch` and stateful `stream` kinds; `passthrough` and `polling` emit `0` from runtime startup so dashboards see a stable series per (graph, accumulator). |
| `cloacina_reactor_cache_age_seconds` | `graph`, `reactor`, `source` | Age in seconds of the most-recent emission per source held in the reactor's input cache. Refreshed on every boundary arrival (all known sources re-emitted, so silent sources show increasing staleness). |
| `cloacina_ws_connections_active` | `endpoint` | Currently open WebSocket connections. `endpoint` ∈ `accumulator`, `reactor`. RAII-guarded so panics inside the handler still decrement on Drop. |
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
cloacina_active_workflows
cloacina_active_tasks
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
   package/tenant metadata — **never** event keys, tenant IDs, raw paths,
   or free-form user input.
3. Update this document with a row, an operator-facing description, and at
   least one example PromQL query if the metric is meant to drive an alert
   or dashboard.
4. Run `angreal test metrics-format` locally before pushing. The same
   check runs in CI (`metrics-format` job in `.github/workflows/cloacina.yml`).

## Related

- [Compiler + Server Deployment Runbook]({{< ref "/service/how-to/compiler-deployment-runbook" >}}) — how to deploy and operate the metric-emitting binaries.
- [Performance Tuning]({{< ref "/service/how-to/performance-tuning" >}}) — uses these metrics to drive tuning decisions.
- [Workflows: Observability]({{< ref "/embed/how-to/observe-execution-state" >}}) — workflow-author perspective on which metrics matter when.
- ADR-0005 — Deployment-mode trust model (`/metrics` posture per deployment mode).
