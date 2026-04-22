# Prometheus Metrics

`cloacina-server` exposes metrics in the standard Prometheus text exposition
format at `GET /metrics`. The endpoint is public (no auth) so Prometheus can
scrape it without credential management.

The exposition format is version-checked in CI via `angreal check
metrics-format`, which boots the server and pipes `/metrics` through
`promtool check metrics`. If that job fails, a recent metric change broke
the format — see T-0536 for how the check is wired.

## Metric reference

All metric names are prefixed `cloacina_`. All labels have bounded
cardinality — no task IDs, tenant IDs, workflow names, or package names are
used as labels. Adding a new metric should preserve this invariant; see
[Adding a metric](#adding-a-metric) at the bottom.

### Counters

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_workflows_total` | `status`, `reason` | Total workflow executions. `status` ∈ `completed`, `failed`. `reason` is `ok` on success, `dependency_failed` on failure (workflow failure is always downstream of task failure). |
| `cloacina_tasks_total` | `status`, `reason` | Total task executions. `status` ∈ `completed`, `failed`. `reason` is `ok` on success, or one of: `task_error`, `timeout`, `validation_failed`, `infrastructure`, `task_not_found`, `claim_lost` (reserved — not emitted yet; tracked for T-0487), `unknown`. |
| `cloacina_api_requests_total` | `method`, `status` | Total HTTP API requests. `method` is the HTTP verb; `status` is the numeric HTTP status code. |

### Histograms

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_api_request_duration_seconds` | `method`, `status` | Handler duration for HTTP API requests, measured inside the `api_request_metrics` middleware. |
| `cloacina_workflow_duration_seconds` | — | Wall-clock duration from workflow execution start to finalize (success or failure). |
| `cloacina_task_duration_seconds` | — | Wall-clock duration from task execution start to end, including timeouts. |

### Gauges

| Name | Labels | Description |
|------|--------|-------------|
| `cloacina_active_workflows` | — | Workflow executions in `Pending` or `Running` state. SQL-derived — re-seeded every scheduler tick from `workflow_executions` row count, so the value is correct by construction across crashes, claim loss, and finalize-path errors. Lags real DB state by at most one scheduler `poll_interval`. |
| `cloacina_active_tasks` | — | Tasks currently inside the executor's run body. Incremented at the top of `ThreadTaskExecutor::execute_task`, decremented at the bottom; a panic between the two leaks one. |

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

## Current gaps

The computation-graph subsystem — reactors, accumulators, the scheduler
loop's health signals, and the WebSocket layer — **does not emit any
metrics today**. Operators running CG workloads should rely on logs plus
`/v1/health/reactors` and `/v1/health/accumulators` for observability
until [CLOACI-I-0099 (Computation Graph Observability)](../../.metis/initiatives/CLOACI-I-0099/initiative.md)
lands. See that initiative for the planned metric set.

The audit that produced the current state is
[CLOACI-T-0498](../../.metis/CLOACI-T-0498.md).

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
4. Run `angreal check metrics-format` locally before pushing. The same
   check runs in CI (`metrics-format` job in `.github/workflows/cloacina.yml`).
