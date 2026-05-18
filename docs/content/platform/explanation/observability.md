---
title: "Observability"
description: "Why the cloacina_* metric namespace looks the way it does — bounded labels, SQL-derived gauges, Degraded health states, and how metrics, logs, and traces compose."
weight: 40
---

# Observability

Cloacina exposes three observability surfaces — Prometheus metrics, structured logs, and OpenTelemetry traces — built on a few non-obvious design choices that this page explains. The *what* lives in [Metrics Catalog]({{< ref "/platform/reference/metrics-catalog" >}}); this page is the *why*.

## The `cloacina_*` namespace

Every metric Cloacina emits is prefixed `cloacina_` (or `cloacina_compiler_` for the compiler binary). Two reasons:

- **Operator dashboards aggregate cleanly.** A single `{__name__=~"cloacina_.*"}` selector picks up the entire footprint. Mixed-namespace setups (one team's Prometheus instance scraping Cloacina alongside another service's metrics) get clean separation without per-metric allowlists.
- **Future-proofs the API surface.** The namespace is part of the contract — adding a new metric inside it is non-breaking; renaming a metric out of it is breaking. The same invariant holds for label keys (see below).

The namespace was retroactively normalized as part of CLOACI-I-0088; the post-rename emissions match what's documented in the catalog. Historical references to `cloacina_pipelines_*` (the pre-rename names) are gone — emissions are `cloacina_workflows_*` to match the user-facing primitive.

## Bounded label cardinality

Every label on every `cloacina_*` metric is bounded by code, not by user input. The discipline:

- **No task IDs, tenant IDs, package names, workflow names, or arbitrary user strings as label values.** Each of those has unbounded cardinality and would explode the per-series count in Prometheus over time.
- **Labels are enums** (`status` ∈ `completed | failed`; `kind` ∈ `passthrough | stream | polling | batch`; `outcome` ∈ `claimed | contended | empty`) or **derived from package/tenant metadata at a bounded position** (`graph` is the deployed graph name, where the count of distinct graphs is operator-controlled and bounded by deployment shape).
- **Per-route HTTP labels use `method` + numeric `status`** rather than the full path — labels stay finite even as the route surface grows.

Why this matters: a single misplaced unbounded label (say, putting `tenant_id` on every task counter) silently turns a 10-series counter into a 10 × N-series time-series-DB nightmare. Prometheus scrape times balloon, dashboards time out, and the only fix is a metric rename. The bounded-label invariant is enforced by code review, not by tooling — adding a new metric requires explicitly justifying every label.

The "Adding a metric" checklist in the catalog formalizes this.

## SQL-derived vs delta-counted gauges

Gauges in Cloacina come in two flavors. The choice between them is deliberate and is the most non-obvious correctness decision in the metric surface.

### Delta-counted gauges (problem)

The conventional approach: an `inc`/`dec` pair brackets every state transition.

```rust
ACTIVE_TASKS.inc();
run_task().await;
ACTIVE_TASKS.dec();
```

This works for steady-state but leaks on every failure path. Panics between `inc` and `dec`. Process crashes. Claim loss that bypasses the cleanup branch. Errors in the cleanup branch itself. Each leak is invisible — the gauge silently drifts upward forever, and a restart "fixes" it without anyone noticing it was wrong.

### SQL-derived gauges (Cloacina's choice)

`cloacina_active_tasks` (and its sibling `cloacina_active_workflows`) are **re-seeded every scheduler tick from a `SELECT count(*)` against the DAL**:

```rust
let active = dal.task_executions().count_where(status = 'Running').await?;
ACTIVE_TASKS.set(active);
```

The gauge value is correct by construction — it equals the real DB state, modulo the `poll_interval` lag. Crashes, claim loss, panics between `inc` and `dec`, errors in finalize paths: none of them drift the gauge. The next scheduler tick repairs it.

This is the CLOACI-I-0108 pattern. It's not the dominant Prometheus convention because most services don't have a system-of-record they can query cheaply for "true" gauge state. Cloacina does: the DAL row is the truth, and the scheduler is already polling it. The marginal cost of the gauge `SELECT` is one query per tick (~100ms intervals); the benefit is gauges that survive crashes.

Same pattern in the compiler: `cloacina_compiler_queue_depth{state}` re-seeds from `compiled_data` row counts per sweep tick.

When to use which:

- **Use SQL-derived gauges** when there is a single, cheaply-queryable system of record (Cloacina's `*_executions` and `compiled_data` tables fit).
- **Use delta-counted gauges** when the gauged quantity is purely in-process and has no persistent record (`cloacina_ws_connections_active` is delta-counted because connections are in-memory; it's RAII-guarded so panics in the handler still decrement on `Drop`).

The catalog flags which is which on each gauge row.

## Component health: the `Degraded` state and persist-failure threshold

Computation graphs publish a one-of indicator gauge — `cloacina_component_health{graph, component, state}` — set to `1` on the current health state and `0` on every other state, re-emitted every supervisor tick.

The interesting state is **`degraded`**. The reactor downgrades to `Degraded` after **5 consecutive `persist_reactor_state` failures** (CLOACI-I-0108 / T-0590) and recovers to `healthy` on the next successful persist. The threshold is deliberate:

- **`1` would flap** on every transient DB hiccup (a single failed write should not page on-call).
- **`100` would mask a real outage** (by the time the threshold tripped, the reactor would have been silently dropping state for tens of seconds).
- **`5`** matches the `MAX_RECOVERY_ATTEMPTS` constant used elsewhere for supervisor restart limits, so operators don't have to remember two different thresholds.

The query operators wire into alerts is:

```promql
cloacina_component_health{component="reactor",state="degraded"} == 1
```

This fires on the *gauge value*, not on a derivative, so it works the moment the supervisor re-emits — no rate-window dependency.

The matching counter is `cloacina_reactor_persist_failures_total{graph, reactor, kind}`. Operators triaging a `Degraded` alert look at the counter to see *which* persist kind (cache, dirty-flag, sequence number, save) is failing — typically all four if the DAL is unreachable; only `save` if there's a write-contention issue.

## Histogram bucket choices

Cloacina histograms use Prometheus's default buckets (`0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10` seconds) on duration metrics. The choice was deliberate:

- **Sub-second tail matters** for graph-fire latency (10ms bucket below the typical 5-15ms fire duration); the default coverage gives operators useful p50/p95/p99 detail without per-metric tuning.
- **Coarse buckets above 1s** are fine because Cloacina workloads that take >1s per task are typically blocked on I/O (DB queries, network) and the p99 vs p95 distinction at minute-scale isn't actionable — alert on the median instead.

Custom buckets are easy to add per metric if a future workload shape requires them; defer that change until the workload's distribution actually justifies it.

## The three legs: metrics, logs, traces

Cloacina deliberately uses all three observability surfaces because each is good at exactly one thing:

- **Metrics** answer "how many / how fast / is it healthy" at low cost over long windows. They aggregate cheaply, fan out into dashboards, and drive alerts. Cloacina's `/metrics` endpoint is the always-on, no-config observability floor.
- **Structured logs** answer "what happened to *this specific* request / task / firing" with full context. Cloacina emits logs with `request_id` spans so the operator can pivot from a metric anomaly to the request that drove it. JSON-formatted, daily-rotated, retention controlled by `--log-retention-days` (CLOACI-I-0109).
- **Traces** answer "what was the critical path" across the request → DAL → scheduler → executor chain. Cloacina ships an OpenTelemetry layer that activates only when `OTEL_EXPORTER_OTLP_ENDPOINT` is set; no overhead when disabled. Spans cover the same boundaries that the `request_id_middleware` log spans do, so logs and traces share IDs.

When to reach for which:

- **A metric is firing**: start in dashboards, narrow to the affected dimension via metric labels.
- **You know the request / task / firing ID**: go straight to logs (filter by `request_id` or `task_id`).
- **The slow path is somewhere in a multi-component flow** and you can't tell which: turn on traces and look at the longest span.

The three legs share enough context (IDs, namespaces, label conventions) that pivoting between them is a label-filter change, not a context switch.

## Trade-offs

A few choices Cloacina explicitly made the opposite way from common defaults:

- **`/metrics` is unauthenticated.** Per ADR-0005, the server's `/metrics` and `/health` probes are public — Prometheus scrapes them without credentials. The trade-off: metric values leak as observable side channels (you can tell from `cloacina_workflows_total` whether a tenant is active). The benefit: no credential management on the scraper path. For deployments where this is unacceptable, terminate `/metrics` at the reverse proxy and require client-cert auth there — Cloacina doesn't enforce it.
- **No built-in alert rules.** Cloacina ships metrics with bounded labels and stable names but does not ship a Prometheus alert-rule file. The rationale: alert thresholds depend on operator SLOs (a CG-fire p99 of 100ms is fine for batch use cases and a disaster for tick-by-tick trading); the metrics are uniform but the alerts can't be.
- **No metric for "current backlog of pending workflows".** The `*_active_*` gauges count Running/Pending state; for true queue depth, operators wire their own gauge against `workflow_executions` directly. The avoidance is deliberate — a separate "pending only" gauge would double-count with `cloacina_active_workflows` and create a measurement-coupling bug.

## See also

- [Metrics Catalog]({{< ref "/platform/reference/metrics-catalog" >}}) — every metric, type, labels, meaning, PromQL examples.
- [Performance Characteristics]({{< ref "performance-characteristics" >}}) — what numbers to expect from the gauges and histograms.
- [Performance Tuning]({{< ref "/platform/how-to-guides/performance-tuning" >}}) — using these metrics to drive tuning decisions.
- [Workflows: Observability]({{< ref "/workflows/how-to-guides/observe-execution-state" >}}) — workflow-author perspective on which metric matters when.
- CLOACI-A-0005 — deployment-mode trust model (`/metrics` posture).
- CLOACI-I-0099 — the initiative that defined the `cloacina_*` namespace.
- CLOACI-I-0108 — SQL-derived gauges + persist-failure `Degraded` threshold.
- CLOACI-I-0109 — compiler `/metrics` + `--log-retention-days`.
