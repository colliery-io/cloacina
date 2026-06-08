---
title: "Execution-Agent Fleet"
description: "How Cloacina offloads task execution to a pool of DB-less remote agents — registration, routing, delivery, execution, and dead-agent reclaim"
weight: 45
---

## Introduction

[Horizontal Scaling]({{< ref "horizontal-scaling" >}}) explains how multiple
runner instances share one PostgreSQL database and avoid double-executing a task
through atomic claiming. That model scales the *control plane*: every runner is a
full Cloacina instance with a database connection.

The **execution-agent fleet** scales the *data plane* a different way. It moves
task execution onto a pool of **DB-less** worker processes (`cloacina-agent`)
that hold no database connection at all. The server stays the single point of
DB authority; agents are pure executors that fetch compiled workflow code, run a
task, and report the result. This is the right tool when you want to:

- run task code on machines you don't want to grant database access to,
- scale execution capacity independently of the server, or
- isolate heavy or untrusted task execution away from the control plane.

Routing is opt-in and per-task: tasks whose names match a configured glob run on
the fleet; everything else keeps running in-process on the server's `default`
executor. You can adopt the fleet for one workflow without changing anything
else.

## The pieces

| Component | Role |
|---|---|
| `cloacina-server` | DB authority. Routes matching tasks to the fleet, selects an agent, pushes work, reconciles results. |
| `cloacina-agent` | DB-less worker. Registers, fetches the compiled cdylib, executes the task, reports the result. |
| `cloacina-compiler` | Builds uploaded workflow packages into `.cloacina` cdylibs the agents load. (Unchanged by the fleet.) |
| `delivery_outbox` | Durable, ack-tracked push queue (the substrate, CLOACI-I-0115) that carries work packets to agents over a WebSocket. |

## How a task reaches an agent

When a fleet-routed task becomes `Ready`, the server's `FleetExecutor` runs the
following, end to end:

1. **Claim.** The executor atomically claims the task (the same `claim_for_runner`
   mechanism the in-process executor uses) so exactly one invocation owns it,
   then marks the workflow execution `Running`. This is what keeps the
   over-selecting scheduler from dispatching the same task twice.
2. **Select an agent.** From the in-memory agent roster it picks a *live* agent
   **in the task's tenant** with spare capacity, greedy on most-free-capacity so
   load spreads. Tenant scope is load-bearing: an agent only ever receives work
   for the tenant its API key is scoped to (REQ-008).
3. **Resolve the artifact.** It looks up the active (built, non-superseded)
   `.cloacina` cdylib digest for the task's package in that tenant.
4. **Inline the context.** It builds the merged dependency context with the same
   `TaskContextBuilder` the in-process path uses, so a fleet-run task sees
   byte-for-byte the input context it would running locally.
5. **Enqueue + register a rendezvous.** It registers a one-shot keyed by the
   `task_execution_id`, then enqueues a **work packet** (task name, context,
   artifact reference, timeout, tenant) into `delivery_outbox` addressed to the
   chosen agent and wakes the delivery relay.
6. **Push.** The relay pushes the work packet over the agent's delivery
   WebSocket. (A LISTEN/NOTIFY wake keeps same-replica delivery prompt; a
   safety-net sweeper re-pushes anything that slips through.)

On the agent:

7. **Triple check (fail-closed).** The agent refuses any packet whose artifact
   was built for a different target triple than its own (OQ-6). The server only
   selects agents whose triple matches, but the agent enforces it independently.
8. **Fetch + cache.** It fetches the cdylib by digest over REST (skipped on a
   cache hit) and `dlopen`s it via [fidius]({{< ref "ffi-system" >}}).
9. **Execute + report.** It resolves the task in the loaded library, runs it
   under the packet's timeout with the inlined context, and POSTs the outcome
   (`Success` / `Failure` / `Refused`) back to the server.
10. **Reconcile.** The server hands the outcome to the **shared**
    `TaskResultHandler` — the same code the in-process executor uses — so state
    writes, retries, and context persistence are identical by construction
    whether a task ran on the fleet or on the server.

The agent reporting wakes the rendezvous registered in step 5, so the original
executor invocation resumes and finalizes the task.

> **Wire format follows the build profile.** fidius serializes in JSON for debug
> builds and bincode for release builds, so an agent must load a cdylib built
> with the *same* profile it runs. Production images are release builds; build
> your workflow packages release too.

## Liveness and dead-agent reclaim

Agents send a heartbeat on an interval the server advertises at registration
(`--agent-heartbeat-interval-s`, default 15s). A background sweeper marks an
agent **dead** after `--agent-liveness-misses` consecutive missed beats
(default 3 → ~45s) and then **reclaims its in-flight work**: every non-acked
`delivery_outbox` row addressed to the dead agent is re-targeted to a live agent
in the same tenant and reset to `pending`, so the relay re-pushes it.

Because the work keeps its original `task_execution_id`, the executor invocation
still awaiting that rendezvous receives the new agent's result unchanged — the
task completes on a survivor with no workflow-level failure. If no live agent is
available, the rows stay put and the executor's own result-wait timeout drives a
retry (degraded, not lost).

Two things worth knowing about the recovery characteristics:

- **It is failover, not checkpointing.** The survivor re-runs the task from the
  start; there is no mid-task resume. Tasks routed to the fleet should be
  idempotent, like any retryable task.
- **Detection latency is tunable.** Total time to recover ≈ (dead-after
  detection) + (re-run). The detection floor is `interval × misses`; lower both
  for more aggressive failover at the cost of more heartbeat traffic.

## When to use the fleet

| Situation | Use |
|---|---|
| Single process, modest load | In-process `default` executor (no fleet). |
| Scale the control plane; runners may hold DB access | [Multiple runners on one DB]({{< ref "horizontal-scaling" >}}). |
| Scale execution on workers that must **not** touch the DB, or isolate heavy/untrusted task code | **Execution-agent fleet.** |

The models compose: a server can run some tasks in-process and route others to
the fleet via per-task globs, and you can run several servers against one DB
while each fans matching work out to agents.

## Observability

The fleet surfaces itself through the server's `/metrics`:

- `cloacina_fleet_agents_evicted_total` — agents the sweeper declared dead.
  Sustained non-zero means agents are crashing or losing connectivity.
- `cloacina_fleet_work_reassigned_total` — in-flight rows reclaimed from dead
  agents onto survivors.
- `cloacina_delivery_outbox_open` — depth of the push queue. Sustained growth
  means delivery is wedged (e.g. no live agent for a recipient).

Agents themselves expose no HTTP surface; observe them via their logs and the
server-side metrics above. See the [Metrics Catalog]({{< ref "metrics-catalog" >}}).

## See also

- [Deploy an execution-agent fleet]({{< ref "/platform/how-to-guides/deploy-an-execution-agent-fleet" >}}) — the operational how-to.
- [CLI Reference]({{< ref "/platform/reference/cli" >}}#agent) — `cloacina-agent` flags and the server's fleet/liveness flags.
- [Horizontal Scaling]({{< ref "horizontal-scaling" >}}) — the single-DB multi-runner model the fleet complements.
- [FFI System]({{< ref "ffi-system" >}}) — how compiled workflow cdylibs are loaded.
