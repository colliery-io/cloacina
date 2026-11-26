---
title: "Execution-Agent Fleet"
description: "How Cloacina offloads task execution to a pool of DB-less remote agents — registration, dispatch, delivery, execution, and dead-agent reclaim"
sidebar:
  order: 45
---

## Introduction

[Horizontal Scaling](../horizontal-scaling/) explains how multiple
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
2. **Select an agent.** From the agent roster it picks a *live* agent
   **in the task's tenant** and **in the task's pool**, with a free slot and
   with room for the task's CPU and memory requests
   (see [How a task is placed](#how-a-task-is-placed)). Tenant scope is
   load-bearing: an agent only ever receives work
   for the tenant its API key is scoped to (REQ-008).
   When no agent of the tenant and pool can take the task now, the task **waits**: it
   stays `Ready`, keeps its attempt count, and is tried again after 2 s. The
   same holds when the chosen agent is full by the time the packet arrives and
   refuses it. Waiting for capacity never fails a task; watch
   `cloacina_fleet_dispatch_deferred_total` to see it. The task is in
   `cloacina_waiting_tasks` for as long as it waits, and
   `cloacina_oldest_waiting_task_age_seconds` grows. Both carry the task's pool
   as a label.
3. **Resolve the artifact.** For the first task of an execution it looks up
   the active (built, non-superseded) version of the task's package in that
   tenant and records it as the execution's **version pin**. For each later
   task of the same execution it reads the pin. So an execution runs all its
   tasks on the version it started with, also when a newer version becomes
   active or an earlier one is made active again while it runs. The artifact
   of a superseded version stays fetchable by digest. If the pinned version
   was deleted, the task fails; it does not move to another version.
   The per-architecture artifact is looked up for that same version.

   Limits of the rule: the pin covers the artifact an agent runs. The server
   still builds the task's dependency context from the workflow it has loaded,
   which is the active version, so a new version that removes a task or changes
   the edges between tasks can still affect an execution that is running. A
   runner that executes tasks in its own process (no fleet) holds one loaded
   version of a package and is not pinned. Computation graph fires have no
   execution and always use the active version.
4. **Inline the context.** It builds the merged dependency context with the same
   `TaskContextBuilder` the in-process path uses, so a fleet-run task sees
   byte-for-byte the input context it would running locally.
5. **Enqueue + register a rendezvous.** It registers a one-shot keyed by the
   `task_execution_id`, then enqueues a **work packet** (task name, context,
   artifact reference, time limit, tenant) into `delivery_outbox` addressed to
   the chosen agent and wakes the delivery relay. The time limit is the
   task's own (see [time limits and retries](#time-limits-and-retries)) or the
   server default.
6. **Push.** The relay pushes the work packet over the agent's delivery
   WebSocket. (A LISTEN/NOTIFY wake keeps same-replica delivery prompt; a
   safety-net sweeper re-pushes anything that slips through.)

On the agent:

7. **Triple check (fail-closed).** The agent refuses any packet whose artifact
   was built for a different target triple than its own (OQ-6). The server only
   selects agents whose triple matches, but the agent enforces it independently.
8. **Fetch + cache.** It fetches the cdylib by digest over REST (skipped on a
   cache hit) and `dlopen`s it via [fidius](../../engine/ffi-system/).
9. **Execute + report.** It resolves the task in the loaded library, runs it
   under the packet's time limit with the inlined context, and POSTs the outcome
   (`Success` / `Failure` / `Refused`) back to the server.
   A POST that fails for a reason that can pass (a network error, a time
   limit, a 5xx) is sent again with a backoff of 1 s, doubled up to 15 s, at
   most 12 times, and not past the point where the server stops waiting. A
   4xx that rejects the report itself (protocol mismatch, a key of another
   tenant) is not sent again.
   It acks the work packet only after that, when the work has ended and the
   report is delivered or given up: until then the open `delivery_outbox` row
   marks the work as in flight (see
   [dead-agent reclaim](#liveness-and-dead-agent-reclaim)). When the retries
   are used up the agent still acks; the server's result wait ends and its
   retry policy starts the task again.
10. **Reconcile.** The server hands the outcome to the **shared**
    `TaskResultHandler` — the same code the in-process executor uses — so state
    writes, retries, and context persistence are identical by construction
    whether a task ran on the fleet or on the server.

The agent reporting wakes the rendezvous registered in step 5, so the original
executor invocation resumes and finalizes the task.

### A report can reach any server replica

The rendezvous of step 5 lives in the memory of the replica that dispatched
the task. The agent posts to the server's Service address, so with more than
one replica the report can reach a different one. That replica has no wait for
it: it stores the report in the `fleet_result_inbox` table and answers 200.
The insert sends a Postgres `NOTIFY`; the replica that waits takes the row
(a delete, so one replica gets it) and resolves its wait. Each replica also
reads the inbox for its own waits every 5 s, which covers a missed
notification. If the report can be neither forwarded nor stored (the database
is down), the server answers with a 5xx and the agent sends it again.

The rules below do not change: a stored report resolves only its own dispatch,
the terminal write is still made by the executor that holds the claim, and a
report that no replica waits for changes no state. Such a row is deleted after
about 15 minutes and counted in `cloacina_fleet_orphan_results_total`.

What this does not cover: a replica that stops takes its waits with it. The
task is then picked up by the stale-claim sweep and runs again, as before.

> **Wire format is always bincode.** fidius serializes with bincode in every
> build profile — there is no debug/JSON split, so agents and packages don't
> need matching build profiles. What must match is the CPU architecture: a
> cdylib only runs on the target triple it was built for (see below).

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

A live agent keeps its packets. Each heartbeat names the packets the agent has
in work, and the server renews the lease of those rows, so the delivery sweeper
does not push a packet again while its agent still works on it. A packet that
arrives a second time anyway (after a WebSocket reconnect, for example) starts
no second run: the agent ignores a push it has in work, and acks again a push
it has finished.

Each dispatch of a task has its own `dispatch_id`, which the agent returns with
the result. The server accepts a result only for the dispatch it waits on, so a
report for an older dispatch of the same task cannot end a newer wait.

Because the work keeps its original `task_execution_id`, the executor invocation
still awaiting that rendezvous receives the new agent's result unchanged — the
task completes on a survivor with no workflow-level failure. If no live agent is
available, the rows stay put and the executor's own result-wait timeout drives a
retry (degraded, not lost).

## Time limits and retries

A fleet task runs under a time limit, and the task's own retry policy decides
what happens when an attempt fails.

- **The time limit is per task.** A task sets it with
  `#[task(timeout_seconds = N)]` (Rust) or `@task(timeout_seconds=N)`
  (Python). A task that sets none gets the server default,
  `--fleet-task-timeout-s` (`CLOACINA_FLEET_TASK_TIMEOUT_S`, default `300`).
- **The server has a maximum.** No task gets more than
  `--fleet-task-timeout-max-s` (`CLOACINA_FLEET_TASK_TIMEOUT_MAX_S`, default
  `86400`, 24 hours). A task that asks for more gets the maximum, and the
  server logs a warning on each dispatch. A default above the maximum stops
  the server at start.
- **The agent enforces the limit.** The work packet carries it. When the task
  runs past it the agent reports a timeout and the attempt fails as a timeout,
  the same as on the in-process executor. The server waits for the report for
  the limit plus 30 seconds, then gives up on the dispatch.
- **The retry policy is the task's own.** `retry_attempts`, `retry_condition`
  and the backoff settings of the task apply on the fleet as they do in
  process. `retry_attempts = 0` or `retry_condition = "never"` gives one
  attempt.
- **Retry conditions read the task's error text.** A failure arrives at the
  server as `agent failure (<class>): <the task's error>`. An error pattern or
  the `transient` condition tests that text, so it sees what the task wrote.

A task that ran past its limit is not stopped (see the second point below):
the agent stops waiting for it and frees the slot.

Two things worth knowing about the recovery characteristics:

- **It is failover, not checkpointing.** The survivor re-runs the task from the
  start; there is no mid-task resume. Tasks run on the fleet should be
  idempotent, like any retryable task.
- **The first copy is not stopped.** When the server gives up on a dispatch (a
  dead agent, a result wait that timed out, a server that stopped), the agent
  may still be running the task, and there is no cancel message. Two copies of
  one task can be alive at the same time. A task that writes outside its
  context, such as a table commit, must be safe to run twice, also at the same
  time.
- **A late report changes nothing.** A result counts only for the dispatch it
  belongs to, and only while the dispatching server holds the claim on the
  task. A report that no replica waits for is logged, deleted from the result
  inbox after about 15 minutes, and counted then in
  `cloacina_fleet_orphan_results_total`.
- **Detection latency is tunable.** Total time to recover ≈ (dead-after
  detection) + (re-run). The detection floor is `interval × misses`; lower both
  for more aggressive failover at the cost of more heartbeat traffic.

## Pluggable actuators & substrate guard

Everything above describes agents you start yourself. The
**control plane** (CLOACI-I-0127) lets the server provision and scale that pool
*for* a tenant, instead of you running `cloacina-agent` by hand. It is split in
two so the *decision* and the *mechanism* stay independent:

- The **control plane** decides how many agents each tenant should have — a
  `desired_count` per tenant, set by tenant self-service provisioning, by an
  admin, or by the autoscaler.
- A **`FleetActuator`** is the mechanism that makes reality match that number on
  a particular substrate. Three implementations ship:

  | Actuator | Substrate | What it reconciles |
  |---|---|---|
  | Noop | — (`CLOACINA_FLEET_ACTUATOR=none`, default) | Nothing. Actuation is off; you run agents yourself. |
  | Docker | local Docker daemon (`docker`) | Spawns/stops labelled `cloacina-agent` containers (`cloacina.tenant=<t>`, `cloacina.managed=true`). **Dev-only.** |
  | Kubernetes | in-cluster API (`kubernetes`) | Drives the `replicas` of one `cloacina-agent` Deployment in the tenant's **own** namespace (`cloacina-tenant-<t>`). |

The actuator is chosen explicitly at boot by `CLOACINA_FLEET_ACTUATOR` and
validated **fail-closed** by a *substrate guard*: a misconfigured actuator must
produce a loud boot error, never silent wrong-scaling. The `docker` actuator
**refuses to start** when it detects Kubernetes (a service-account token mount or
`KUBERNETES_SERVICE_HOST`) — so it can never scale throwaway containers on a host
whose real substrate is a cluster — and refuses when no Docker socket is
reachable. The `kubernetes` actuator refuses when the server is not running
in-cluster (it needs in-cluster credentials). Whichever actuator runs, it mints a
tenant-scoped `read` API key and injects it (with the server URL) so the spawned
agent self-registers down the same path a hand-run agent uses — the Docker
actuator mints one key per container, the Kubernetes actuator one shared
per-tenant key (in a `Secret`, re-minted on scale-up). Every list/spawn/scale is
scoped to the one tenant (a Docker label filter, or the tenant's Kubernetes
namespace) so the actuator never touches another tenant's workloads (REQ-008 /
NFR-004).

### Kubernetes agent-pod hardening (CLOACI-T-0819)

On the `kubernetes` substrate the actuator-created agent pods are hardened to
clear a PodSecurity **`restricted`** cluster, which would otherwise reject them:

- **Non-root, locked-down `securityContext`.** Pods run as the agent image's uid/gid
  `10001` (`runAsNonRoot`, `runAsUser`/`runAsGroup`/`fsGroup`) with
  `seccompProfile: RuntimeDefault`; containers drop **all** capabilities, set
  `allowPrivilegeEscalation: false`, and run with a `readOnlyRootFilesystem`. The
  agent's writable paths — `$HOME` (where it unpacks each Python package's
  `workflow/`+`vendor/` tree and caches cdylibs) and `/tmp` — are backed by
  `emptyDir` volumes so the read-only root still works.
- **No service account token.** The pod sets
  `automountServiceAccountToken: false`. The agent never calls the Kubernetes
  API, and task code runs in the agent process, where it could read a mounted
  token. The standalone `cloacina-agent` chart does the same
  (`automountServiceAccountToken`, default `false`).
- **The agent's key leaves the environment.** The key arrives as
  `CLOACINA_API_KEY` from the tenant namespace's Secret. The agent reads it at
  start, overwrites the value in its start-time environment and removes the
  variable. No volume holds the key.
- **Resource requests/limits.** Each pod gets requests + limits configurable via
  the chart's `fleet.agentResources` (rendered as `CLOACINA_AGENT_*`). Defaults
  are tuned to the agent's real footprint — it embeds a CPython interpreter
  (PyO3), so the memory limit is generous to avoid OOM-killing Python workflows.
- **No httpGet probes.** The agent is a WebSocket client with **no** health
  endpoint, so adding a kubelet probe would be meaningless. Liveness is tracked
  server-side instead, by the heartbeat/eviction sweep described above
  (`CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` × `CLOACINA_AGENT_LIVENESS_MISSES`).

The actuator also installs a per-tenant **`NetworkPolicy`** in each
`cloacina-tenant-<t>` namespace (toggle: `fleet.networkPolicy.enabled`, default
on): **deny all ingress** (agents serve no traffic) and **allow egress only** to
cluster DNS (UDP+TCP 53) and the `cloacina-server` pods (on the server port).
This is **defense-in-depth** — a tenant namespace that is network-isolated by
default — and it deliberately does **not** replace any server-side check: the
server-side ABAC (NFR-004) remains the real tenant-isolation boundary, and a
tenant is still isolated primarily by *being its own tenant*, not by the policy.
If the chart cannot supply the server's coordinates the actuator skips the policy
(fail-open) rather than strand a fleet that can no longer reach the server.

## Agent pools

A tenant's managed agents are grouped in **pools**. A pool is a set of agents of
one size: its own CPU and memory requests and limits, max concurrency, image,
service account and extra egress rules. On Kubernetes each pool is its own
Deployment in the tenant's namespace.

Every tenant has a pool named `default`. With no settings stored it is what a
tenant's fleet was before pools: the server-wide image and resources
(`CLOACINA_AGENT_*`), the tenant's agent limit as its ceiling, and the
autoscaler moving its count. A platform admin can add other pools and size them
through the [pool API](../../../reference/http-api/#agent-pools).

An agent tells the server its pool when it registers, as the capability
`pool:<name>` (`--pool` / `CLOACINA_AGENT_POOL`; the actuator sets it). An agent
with no pool capability, such as one you start by hand, is in `default`.

A task names its pool with `pool = "large"` on
[`#[task]`](../../../reference/macros/#pool-and-requests) or on the Python
[`@task`](../../../reference/python-api/task/#pool-and-requests). A task with a
pool runs only on an agent of that pool. A task with no pool runs only on
`default`. Work never moves to another pool because its own is full or empty:
it waits.

### How a task is placed

A task may also request CPU and memory (`cpu = "4"`, `memory = "32Gi"`). The
server packs tasks onto the agents of a pool by these requests.

**What an agent can hold.** An agent reports its total CPU and memory when it
registers, next to its slot count (`max_concurrency`):

- A managed agent on Kubernetes reports what its pool's settings give its pod:
  the pool's limit, or the pool's request when it sets no limit, for CPU and
  for memory. A pool that sets neither for a resource reports no total for it.
- A managed agent on Docker reports the container's CPU and memory limits.
- An agent you start by hand reports what you give it with `--cpu` and
  `--memory` (`CLOACINA_AGENT_CPU`, `CLOACINA_AGENT_MEMORY`). With neither it
  reports no totals.

**What is in flight.** The server keeps, for each agent, the tasks it has
placed there and not yet had a result for. It keeps them on the task rows in
the database, so every server replica counts the same tasks. The sum of their
requests is what is used on the agent.

**What a task counts for.**

| The task | The agent reports a total | The agent reports no total |
|---|---|---|
| has a request | the request | nothing: the agent is packed by slots only |
| has no request | one slot's share: the total divided by `max_concurrency` | nothing |

So a package built before requests existed behaves as before: an agent is full
at its slot count.

**The rule.** A task can go to an agent when all of these hold:

1. the agent is live, in the task's tenant and in the task's pool, and can run
   the package (its architecture);
2. it has a free slot. The slot count is an upper cap whatever the requests
   say: an agent never has more tasks than its `max_concurrency`;
3. what the task counts for in CPU fits in the agent's free CPU, and the same
   for memory.

Among the agents where the task fits, the server picks the one with the
**least free memory**, so that the agents with the most room stay free for
large tasks. Between agents with the same free memory it picks the one with the
most free slots. An agent that reports no memory total comes after every agent
that does.

Example: a pool whose agents hold 32Gi. One agent runs a 24Gi task and four
2Gi tasks together, which is 32Gi. A second 24Gi task does not fit there. It
goes to another agent of the pool with 24Gi free, or it waits.

**A task that fits nowhere waits.** It stays `Ready`, keeps its attempt count
and is tried again every 2 s. It does not fail and it uses no retry attempt.
`cloacina_waiting_tasks{pool="large"}` counts the waiting tasks of each pool.

**Two server replicas and the last free room.** Each replica writes its
placement on the task's row and then reads the agent's sum again. If the sum is
now more than the agent can hold, that replica takes its placement back and its
task waits. The replica that wrote second always sees the first one, so an
agent never keeps two tasks on the same room. When both wrote before either
read, both take their placement back and both tasks are tried again.

**Requests are not limits.** A request decides where a task is sent. Nothing
measures or limits what the task then uses. A task that uses more memory than
it requested is not stopped, and when the agent's pod runs out of memory every
task on that agent fails with it. Set each request to what the task needs at
its peak, and keep `max_concurrency` low on a pool for large tasks.

Graph firings (computation graphs dispatched to the fleet) name no pool and
have no request. They go to the pool `default` and are counted by the agent's
free slots only.

### What a publish checks

When a package is published, the server checks each of its tasks against the
tenant's pools. The package is refused when a task:

- names a pool the tenant does not have, or
- requests more CPU or memory than one agent of its pool can hold. That size is
  what the pool's settings say: its limit, or its request when it sets no
  limit. A pool that sets neither for a resource has no size to compare with.

The check runs when the tenant's runner loads the package, because that is when
its tasks are known: for a Rust package, after the compile. A refused package
is not loaded. Its build status becomes `failed` and its build error holds the
reason, with the task, the pool and the numbers:

```text
package orders v1.4.0 is refused: task 'load_history' requests memory 64Gi,
but one agent of the pool 'large' holds 32Gi at most. Lower the request, or
raise the pool's memory limit, and publish again.
```

`cloacinactl package inspect <id>` and the workflow routes show it, the same
way they show a compile error. Fix the task or the pool, then publish the
package again.

A pool for agents you start by hand needs a row too, or tasks that name it are
refused: create it with `max_agents` 0, so the server starts no agents for it.

### What pools do not do

- **Only `default` is autoscaled.** Another pool runs a fixed number of agents,
  its `max_agents`. The tenant's agent limit and the provision API are for
  `default`; they do not count or bound the other pools.
- **No limit on what a task uses.** A task runs inside the agent process, next
  to the other tasks of that agent. See "Requests are not limits" above.

### How a change of settings reaches the agents

The pool's settings are stored in the server's database. The fleet loop reads
them on every tick (`CLOACINA_AUTOSCALE_INTERVAL_S`, default 30s) and the
Kubernetes actuator applies each pool's Deployment with server-side apply. A
changed image, resource, concurrency or service account changes the pod
template, so Kubernetes rolls the Deployment: new pods start with the new
settings and the old ones are stopped. A task that was running on a stopped
agent is reclaimed and run again, as for any agent that goes away. A changed
egress rule changes the pool's NetworkPolicy only; no pod restarts.

If the settings cannot be read, the actuator changes nothing for that tenant on
that tick. It does not fall back to the defaults, which would resize the agents.

### Cloud identity and network reach

The pool settings for identity are generic: Cloacina has no code for any one
cloud. They are the same knobs a Helm chart gives a workload, and they let a
task reach object storage, a table catalog or a cloud API with the pool's own
identity:

- `service_account_name` and `service_account_annotations`: the actuator creates
  the ServiceAccount in the tenant namespace with the annotations, and the
  pool's pods run as it. Workload identity systems read an annotation on the
  ServiceAccount.
- `pod_labels` and `pod_annotations`: added to the pod template of the pool's
  Deployment. Some identity systems act only on pods that carry a label. The
  labels the actuator sets itself (`app.kubernetes.io/name`, `cloacina.managed`,
  `cloacina.tenant`, `cloacina.pool`) cannot be set: the first is the
  Deployment's selector, which never changes.
- `automount_service_account_token`: the pod's field of that name. Not set, it
  is left out and the cluster default applies (the token is mounted). The agent
  does not call the Kubernetes API and does not need that token. The identity
  token that a workload identity system injects is a separate projected volume,
  so `false` is the tighter setting where the identity system allows it.

An injected token reaches the agent as a mounted volume and environment
variables that an admission webhook adds to the pod, not to the Deployment, so
the actuator's apply does not undo them. The pod's hardening does not block a
mounted token: the read-only root filesystem and the dropped capabilities do
not apply to a volume, and the pod sets `fsGroup` to the agent's group
(`10001`), which is how a non-root container reads a projected token. See
[Give a pool a cloud identity](../../../how-to/service/deploy-an-execution-agent-fleet/#give-a-pool-a-cloud-identity)
for examples.

The network side is one more setting:

- `extra_egress`: CIDR blocks, each with optional ports, that the pool's pods
  may reach in addition to cluster DNS and the server. The rules are CIDR
  blocks only, not host names: for a service whose addresses change, use a
  private endpoint with a fixed range or the provider's published ranges.
  Include the address your cloud's identity mechanism calls for its token. A
  block that covers the cluster's pod or service range opens in-cluster
  traffic too.

With no `extra_egress` a managed agent reaches cluster DNS and the server and
nothing else.

## Capacity limits & autoscaling

These numbers are for the pool `default` (see [Agent pools](#agent-pools)).

Two numbers bound a tenant's fleet:

- **Effective limit** — the hard ceiling. It is the platform default
  (`CLOACINA_DEFAULT_MAX_AGENTS`, default 4) unless a platform admin sets a
  per-tenant override. A tenant cannot raise its own ceiling; only provision
  within it.
- **`desired_count`** — the operational target. A tenant self-services it in
  `[0, effective_limit]` through the
  [fleet API](../../../reference/http-api/#tenant-agent-fleet) (provision
  `+1`, deprovision `−1` down to 0), and a new tenant is seeded with
  `min(CLOACINA_INITIAL_AGENTS, CLOACINA_DEFAULT_MAX_AGENTS)` on create. The
  autoscaler moves it within `[CLOACINA_AUTOSCALE_FLOOR, effective_limit]` — the
  floor bounds the autoscaler, not the manual deprovision API.

A **back-pressure autoscaler** can move `desired_count` on its own. It runs as a
single control loop that, each tick (`CLOACINA_AUTOSCALE_INTERVAL_S`, default
30s), computes each tenant's **utilization** — Σ `in_flight` / Σ
`max_concurrency` over that tenant's live agents of the pool `default` — and
decides (capacity is what each agent registered, so a pool with another
`max_concurrency` is measured against its own capacity):

- **up** (+1) when utilization exceeds the up-threshold (default 0.8) and there
  is room under the effective limit;
- **down** (−1) when it drops below the down-threshold (default 0.2) and there is
  room above the floor;
- **hold** otherwise (the band between the thresholds is hysteresis that prevents
  thrash, and a per-tenant cooldown — default 60s — rate-limits changes).

After adjusting `desired_count`, the same loop **reconciles** actual → desired
through the actuator. The autoscale step is a separate decision from the signal
plumbing, so `CLOACINA_AUTOSCALE=false` freezes automatic scaling while leaving
reconciliation running — operators can then drive `desired_count` by hand.

Utilization is the v1 signal deliberately: it is reactive (it only rises once the
fleet is already saturated). The autoscaler decision lives in Cloacina's control
plane rather than a Kubernetes HPA precisely because the relevant signal is
*per-tenant* and a tenant is the isolation boundary — an HPA cannot see it.

Because every replica runs the same loop, the fleet is driven through
**Postgres-advisory-lock leader election**: each tick, only the replica holding
the lock executes the loop body; the rest skip it. One replica drives the whole
fleet, so two replicas never scale or actuate the same tenant concurrently
(NFR-003). The knobs above are documented in
[Environment Variables](../../../reference/environment-variables/#fleet-actuator--autoscaler).

## When to use the fleet

| Situation | Use |
|---|---|
| Single process, modest load | In-process `default` executor (no fleet). |
| Scale the control plane; runners may hold DB access | [Multiple runners on one DB](../horizontal-scaling/). |
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
server-side metrics above. See the [Metrics Catalog](../../../reference/metrics-catalog/).

## See also

- [Deploy an execution-agent fleet](../../../how-to/service/deploy-an-execution-agent-fleet/) — the operational how-to.
- [CLI Reference](../../../reference/cli/#agent) — `cloacina-agent` flags and the server's fleet/liveness flags.
- [Horizontal Scaling](../horizontal-scaling/) — the single-DB multi-runner model the fleet complements.
- [FFI System](../../engine/ffi-system/) — how compiled workflow cdylibs are loaded.
