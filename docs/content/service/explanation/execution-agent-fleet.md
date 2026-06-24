---
title: "Execution-Agent Fleet"
description: "How Cloacina offloads task execution to a pool of DB-less remote agents — registration, dispatch, delivery, execution, and dead-agent reclaim"
weight: 45
aliases:
  - "/platform/explanation/execution-agent-fleet/"

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

Adopting the fleet is a single server-level switch: set the **default executor**
to `fleet` (via `[server].default_executor` in `config.toml`, or
`CLOACINA_DEFAULT_EXECUTOR=fleet` / `--default-executor fleet`). Every task then
runs on the fleet instead of the in-process `default` executor — there is no
per-task matching. The `fleet` key is only a registered executor when you've
opted in; if you select it without the fleet deployed, the server fails fast at
startup.

## The pieces

| Component | Role |
|---|---|
| `cloacina-server` | DB authority. When the default executor is `fleet`, dispatches tasks to the fleet, selects an agent, pushes work, reconciles results. |
| `cloacina-agent` | DB-less worker. Registers, fetches the compiled cdylib, executes the task, reports the result. |
| `cloacina-compiler` | Builds uploaded workflow packages into `.cloacina` cdylibs the agents load. (Unchanged by the fleet.) |
| `delivery_outbox` | Durable, ack-tracked push queue (the substrate, CLOACI-I-0115) that carries work packets to agents over a WebSocket. |

## How a task reaches an agent

When the default executor is `fleet` and a task becomes `Ready`, the server's
`FleetExecutor` runs the following, end to end:

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

## Multi-architecture dispatch

A compiled `.cloacina` cdylib is native code — it only runs on the architecture
it was built for. A fleet of mixed hardware (say aarch64 nodes alongside x86)
therefore needs more than one build of the same package. 0.9.0 (CLOACI-T-0780)
carries **per-target artifacts** so a single logical package can fan out across a
heterogeneous fleet.

Two tables hold the builds:

| Table | Holds |
|---|---|
| `workflow_packages` | The **primary** cdylib, built for the server's own host arch. |
| `package_artifacts` | **Extra** per-target cdylibs, one row per `(package_name, version, tenant_id, target_triple)`. (Migrations: Postgres `031_create_package_artifacts`, SQLite `027_create_package_artifacts`.) |

Each `package_artifacts` row carries its `target_triple`, a `content_hash`, and
the `compiled_data` blob; a unique index on
`(package_name, version, tenant_id, target_triple)` keeps it to one cdylib per
target. The primary build in `workflow_packages` is the host-arch fallback —
compiled packages with no per-target row for a given triple can only run on a
host-arch agent.

Dispatch (in `FleetExecutor`) then becomes arch-aware, between claiming the task
and pushing the work packet:

1. **Compute the runnable arches.** For a **compiled** package, that's the host
   primary triple ∪ the set of `target_triple`s with a `package_artifacts` row
   for the package. Agent selection filters the roster to agents whose
   `target_triple` is in that set (on top of the existing live-and-in-tenant,
   most-free-capacity selection), so a task is only ever handed to an agent that
   can actually load it.
2. **Resolve the cdylib for the chosen agent.** Dispatch looks up the
   `package_artifacts` digest matching the selected agent's `target_triple`; if a
   per-target build exists it ships that one, otherwise it falls back to the
   primary host-arch digest. The work packet stamps the triple the artifact was
   built for, and the agent's fail-closed triple check (step 7 above) enforces it
   independently.

**Interpreted (Python) packages are architecture-independent** — they run from
source through the agent's interpreter, so there is no native cdylib to match.
For these, dispatch **skips the arch filter entirely** (any live, in-tenant,
spare-capacity agent is eligible) and stamps the **selected agent's own**
`target_triple` on the work packet, so the fail-closed guard is a no-op rather
than a rejection.

This composes cleanly with tenant scoping: artifacts are keyed per tenant, and
agent selection still requires the agent's tenant to match the task's.

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
  start; there is no mid-task resume. Tasks run on the fleet should be
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

The models compose: a server runs all work either in-process or on the fleet
(per its `default_executor`), and you can run several servers against one DB,
each fanning its work out to agents when set to `fleet`.

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

- [Deploy an execution-agent fleet]({{< ref "/service/how-to/deploy-an-execution-agent-fleet" >}}) — the operational how-to.
- [CLI Reference]({{< ref "/reference/cli" >}}#agent) — `cloacina-agent` flags and the server's fleet/liveness flags.
- [Horizontal Scaling]({{< ref "horizontal-scaling" >}}) — the single-DB multi-runner model the fleet complements.
- [FFI System]({{< ref "ffi-system" >}}) — how compiled workflow cdylibs are loaded.
