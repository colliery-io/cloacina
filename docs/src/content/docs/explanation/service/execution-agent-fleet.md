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

The fleet is the server's default executor (CLOACINA-T-1075): every task runs
on an agent of its tenant, and the server loads packages from their metadata
only, so no tenant code runs in the server process. The in-process `default`
executor is used only with `--single-trust` (`[server].single_trust`,
`CLOACINA_SINGLE_TRUST`), and then only when `--default-executor default` is
set or no executor is set. There is no per-task matching. See
[Tenant code and the server process](../security-model/#tenant-code-and-the-server-process).

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
   A task also waits when the agent refuses the packet because it **could not
   fetch the package artifact** (the server restarts, the artifact is not
   there yet). The task goes back to `Ready`, keeps its attempt count, and is
   dispatched again after a delay that starts at 2 s and grows to 30 s. Each
   try selects an agent again, so it can go to a different agent. This wait
   has a limit, `--fleet-artifact-fetch-wait-s`
   (`CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S`, default `300`), counted per
   attempt from the first refused fetch. A refusal at or past the limit fails
   the attempt with the error `agent could not fetch the package artifact`,
   and the task's retry policy decides what follows; a new attempt gets the
   full wait again. `cloacina_fleet_artifact_fetch_refusals_total` counts
   these refusals.
3. **Resolve the artifact.** For the first task of an execution it looks up
   the active (built, non-superseded) version of the task's package in that
   tenant and records it as the execution's **version pin**. For each later
   task of the same execution it reads the pin. So an execution runs all its
   tasks on the version it started with, also when a newer version becomes
   active or an earlier one is made active again while it runs. The artifact
   of a superseded version stays fetchable by digest. If the pinned version
   was deleted, the task fails; it does not move to another version.
   The per-architecture artifact is looked up for that same version.

   Limits of the rule: the pin covers the artifact an agent runs and the
   task's dependencies (step 4). The retry policy, the time limit and the
   placement of the task come from the task the server has loaded, which is
   the active version. The scheduler decides when a task is ready from the
   edges of the active version. A runner that executes tasks in its own
   process (no fleet) holds one loaded version of a package and is not pinned.
   Computation graph fires have no execution and always use the active
   version.
4. **Inline the context.** It builds the merged dependency context with the same
   `TaskContextBuilder` the in-process path uses, so a fleet-run task sees
   byte-for-byte the input context it would running locally. The dependencies
   are those of the task in the pinned version, from the task list stored with
   that version; a version stored with no task list uses the loaded task.
5. **Enqueue + register a rendezvous.** It registers a one-shot keyed by the
   `task_execution_id`, then enqueues a **work packet** (task name, context,
   artifact reference, time limit, tenant, and the task's pool, requests and
   `inline` mark) into `delivery_outbox` addressed to
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
8. **Fetch + cache.** It fetches the package by digest over REST, one time per
   digest: the cdylib of a Rust package, or the source archive and the vendored
   dependencies of a Python package, and the bundled providers.
9. **Execute + report.** It starts a [worker process](#worker-processes-and-limits)
   for the task. The worker loads the package (`dlopen` via
   [fidius](../../engine/ffi-system/), or a Python import), resolves the task
   and runs it with the inlined context. The agent holds the worker to the
   packet's time limit and to the task's memory limit, and POSTs the outcome
   (`Success` / `Failure` / `Refused`) back to the server. A task marked
   `inline` runs in the agent process, with no worker.
   A POST that fails for a reason that can pass (a network error, a time
   limit, a 5xx) is sent again with a backoff of 1 s, doubled up to 15 s, at
   most 12 times, and not past the point where the server stops waiting. A
   4xx that rejects the report itself (protocol mismatch, an agent id that is
   not registered, an agent that another key holds) is not sent again.
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

### A replica that stops: another one takes over the wait

A server restart or a rolling upgrade stops the replica that waits for a
report. The agent keeps running the task. So the dispatch is also recorded on
the task row: when the server hands the packet to the agent, the executor
writes the packet's `dispatch_id` and its `delivery_outbox` row id next to its
claim, and clears them with the claim.

Every 15 s each fleet executor looks for rows of its tenant with a recorded
dispatch and a claim heartbeat older than 60 s: their replica stopped. For
each one it reads the packet's outbox row, then the stored reports:

- **The packet is not acked yet, or the agent's report of this dispatch is
  stored.** The executor takes the claim (a compare and set on the old claim,
  so of two replicas one wins) and waits for the report in the place of the
  stopped replica. The wait ends when the first one would have: at the start
  time plus the task's time limit plus 30 s. A report that a replica stored
  while nobody waited is taken at once. The report then has the same effect
  as on the first wait: the task finishes once, on its first dispatch.
- **The packet is acked and no report is stored, or the row is gone.** No
  report comes. The executor clears the recorded dispatch and the stale-claim
  sweep makes the task `Ready`, with its recovery count. The task runs again,
  once.

An agent stores its report (through any replica) before it acks the packet,
so an acked packet with no stored report has no report to come. If the agent
dies too, the dead-agent reclaim moves the packet to a live agent with the
same `dispatch_id`, and the wait takes that agent's report.

While a server sends tasks to the fleet (`--default-executor fleet`), its
stale-claim sweep leaves rows with a recorded dispatch to the fleet executor.
A server that does not use the fleet sweeps them as any stale claim.

The executor records the runner of its claim with the dispatch, and a
takeover moves it with the claim. A replica takes over only a dispatch
recorded under the claim it finds stale. A row whose claim another runner
took without recording a dispatch, or a row with no runner recorded, names a
dispatch that is not the live one of its claim: the stale-claim sweep makes it
`Ready` as any stale claim.

Each takeover is counted in `cloacina_fleet_dispatches_taken_over_total`, by
`outcome`.

Limits:

- A takeover comes 60 to 75 s after the replica stops. Until then the task's
  slot on the agent stays taken, and a report waits in the inbox.
- The dispatch is recorded right after the packet is enqueued. A replica that
  stops in between leaves no record; the sweep makes that task `Ready` and it
  can run twice.
- During the roll that first brings this, the replicas of the older version
  record no dispatch, and their stale-claim sweep makes a stale fleet task
  `Ready` after 60 s as before. A task dispatched by a new replica can be swept
  by an old replica before a new one takes it over. An old replica that then
  sends the task again leaves the new replica's record on the row; the record
  is not under its claim, so no new replica waits for that older dispatch
  when the old replica stops: the sweep makes the task `Ready` and it is sent
  again. In that roll a fleet task can run twice, as before. From the next
  roll on, every replica takes over.
- A graph firing has no task row and no claim: a replica that stops ends its
  firings, as before.

### A packet the agent cannot decode

A push whose payload the agent cannot read as a work packet (a payload that
is not base64 or not JSON, or a packet with fields the agent does not know
how to read) gives the agent no task id, so it cannot send a result. This
happens when the server and the agent are different versions.

The agent sends `POST /v1/agent/undecodable` with the id of the push, which
is the id of the packet's `delivery_outbox` row, and its decode error. The
server reads the task or the firing from its own row and ends the wait of
step 5 with a failure. The error names the agent, both versions and the
decode error. For a task the retry policy decides the next run, as for any
task error; a graph firing ends with the error and is not run in-process.
The server counts each one in `cloacina_fleet_undecodable_packets_total`.

The agent retries the message as it retries a result, and acks the push
after it. The load of the agent makes no difference: a full agent and an
agent with a free slot do the same.

Mixed versions:

- A server from before this route answers 404. The agent acks the push, and
  the task waits out its time limit.
- An agent from before this message sends nothing. A full one acks the push;
  one with a free slot does not, and gets the push again. The task waits out
  its time limit.

### Who may act as an agent

An agent row has an owner: the tenant and the API key that registered it.

- **Register.** With no `agent_id` in the request the server assigns a new
  id. A caller-chosen id is checked: if another tenant or another key holds
  it, the register is refused with `409 agent_id_taken` and the holder's row
  and one-time keys stay as they are. The key that holds an id may register
  it again, which is what an agent does when it restarts or reconnects.
- **Heartbeat, result, key top-up.** The server accepts these only from the
  key that holds the agent. An agent id that is not registered gets
  `404 agent_not_registered`; an agent that another key holds gets
  `403 agent_access_denied`.
- **Result.** The wait of step 5 knows the tenant, the agent and the
  `dispatch_id` of its packet. A report resolves it only when it comes from
  that tenant and from the agent the packet is addressed to, and carries that
  `dispatch_id`. A report for a task dispatch with no `dispatch_id` resolves
  nothing. A report from another agent, for a wait this replica holds, is
  refused, logged, and counted in `cloacina_fleet_refused_results_total`;
  the agent still gets a 200, and no state changes. A graph firing is keyed
  by its firing id and has no `dispatch_id`; the tenant and agent check is
  the same.
- **Delivery socket.** `agent:<agent_id>` is open only to the key that holds
  the agent, and an `ack` applies only to a row addressed to that agent in
  its tenant.
- **Undecodable packet.** The key must hold the agent, as for a result, and
  the delivery row the message names must be addressed to that agent in its
  tenant, as for an `ack`. A message for any other row changes nothing and
  is counted in `cloacina_fleet_refused_undecodable_total`.

An agent keeps one id for the life of its process: the id of its first
register, which it registers again at each reconnect. A task that is still
running when the socket drops reports under that id.

Agents that share one API key are one owner and can act as each other. The
Kubernetes actuator gives every agent of a tenant the same key.

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

Because the work keeps its original `task_execution_id` and `dispatch_id`, the
executor invocation still awaiting that rendezvous receives the new agent's
result — the task completes on a survivor with no workflow-level failure. The
re-targeted `delivery_outbox` row is the record of where the packet went: the
wait reads it, on any replica, and accepts the result of the agent the row is
now addressed to. An evicted agent is no longer registered, so a result it
posts after the eviction is refused with `404`, and its `ack` of the
re-targeted row changes nothing. If no live agent is
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
- **The agent enforces the limit.** The work packet carries it. The clock
  starts when the task's worker process has loaded the package. When the task
  runs past the limit the agent kills the worker process, with every process
  the task started, and reports a timeout: the attempt fails as a timeout, the
  same as on the in-process executor. The task's slot is free when the worker
  has exited. A worker that needs more than 20 seconds to load its package
  loses that time from its limit: it ends no later than the limit plus 20
  seconds after its start. The server waits for the report for the limit plus
  30 seconds, then gives up on the dispatch and sends the agent a
  [cancel](#worker-processes-and-limits).
- **An inline task is not killed.** For a task marked `inline` the agent stops
  waiting at the limit, reports a timeout and frees the slot. A blocking body
  runs on inside the agent process.
- **A graph firing gets the server default.** A computation-graph firing sent
  to an agent carries `--fleet-task-timeout-s`, and the server waits for its
  result for that limit plus 30 seconds, then reports the firing as failed. A
  graph has no time limit of its own, and the maximum does not apply to it.
  By default an agent runs a firing inline and does not stop it at the limit.
  With `--fleet-graph-firings worker` the agent kills the firing's worker at
  the limit, as it does a task's, and the server sends a cancel when its wait
  ends.
- **The retry policy is the task's own.** `retry_attempts`, `retry_condition`
  and the backoff settings of the task apply on the fleet as they do in
  process. `retry_attempts = 0` or `retry_condition = "never"` gives one
  attempt.
- **Retry conditions read the task's error text.** A failure arrives at the
  server as `agent failure (<class>): <the task's error>`. An error pattern or
  the `transient` condition tests that text, so it sees what the task wrote.

Things worth knowing about the recovery characteristics:

- **It is failover, not checkpointing.** The survivor re-runs the task from the
  start; there is no mid-task resume. Tasks run on the fleet should be
  idempotent, like any retryable task.
- **The first copy is stopped in one case, and not in two.** When the server's
  wait for a result ends with no report, the server sends a cancel to the
  agent that has the packet, and the agent kills the task's worker process.
  When the server declares an agent dead, or the server process that
  dispatched the task stops, no cancel is sent: the agent, if it is alive,
  runs the task on until its time limit and kills it then. In those two cases
  two copies of one task can be alive at the same time. A task that writes
  outside its context, such as a table commit, must be safe to run twice, also
  at the same time. A task marked `inline` is never stopped.
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
tenant-scoped API key of role `agent` and injects it (with the server URL) so the
spawned agent self-registers down the same path a hand-run agent uses. A key of
role `agent` opens the agent routes and nothing else. The Docker actuator mints
one key per container and revokes it when the container is stopped. The
Kubernetes actuator keeps one key per tenant in a `Secret`; the pod template of
each agent Deployment names the key's id, so a new key rolls the pods. Each
reconcile revokes every other key the actuator minted for the tenant. The
actuator's keys are named `agent:<tenant>:<uuid>`: do not give that prefix to a
key you create. Every list/spawn/scale is
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
  API, and task code runs in the agent's pod with the agent's user id (in a
  worker process, or in the agent process for an inline task), where it could
  read a mounted token. The standalone `cloacina-agent` chart does the same
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
With the policy on, a server that cannot write it does not start
(CLOACINA-T-1069): the server's coordinates (`CLOACINA_SERVER_NAMESPACE`,
`CLOACINA_SERVER_POD_SELECTOR`) are unset, or its RBAC lacks `create`, `patch`
or `delete` on `networkpolicies`. The log names what is missing. It runs no
agents without their policy.

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

For memory, every agent keeps its own share (`--reserved-memory`, default
128Mi) and reports the rest: an agent whose pod has 32Gi offers 32Gi less
128Mi to tasks.

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

Example: a pool whose agents have 32Gi, of which each offers 32Gi less 128Mi
to tasks. One agent runs a 24Gi task and three 2Gi tasks together, which is
30Gi. A second 24Gi task does not fit there. It goes to another agent of the
pool with 24Gi free, or it waits.

**A task that fits nowhere waits.** It stays `Ready`, keeps its attempt count
and is tried again every 2 s. It does not fail and it uses no retry attempt.
`cloacina_waiting_tasks{pool="large"}` counts the waiting tasks of each pool.

**Two server replicas and the last free room.** Each replica writes its
placement on the task's row and then reads the agent's sum again. If the sum is
now more than the agent can hold, that replica takes its placement back and its
task waits. The replica that wrote second always sees the first one, so an
agent never keeps two tasks on the same room. When both wrote before either
read, both take their placement back and both tasks are tried again.

**A memory request is also the task's limit.** The agent stops a task that
uses more memory than it requested; see
[worker processes and limits](#worker-processes-and-limits). Set each memory
request to what the task needs at its peak. A CPU request decides where a task
is sent and caps nothing.

Graph firings (computation graphs dispatched to the fleet) name no pool and
have no request. They go to the pool `default` and are counted by the agent's
free slots only.

### Worker processes and limits

An agent runs each task in a worker process: the agent binary, started again
as a worker. The agent fetches the package and unwraps the task's secrets. The
worker loads the package, runs the task and hands the result back over a pipe.
A worker then stays, with the package loaded, for the next task of the same
package version and tenant: a [warm worker](#warm-workers). A worker runs one
task at a time.

**What crosses to the worker.** The work packet, the task's secrets and the
paths of the fetched package files go in over the worker's stdin, a pipe, one
line for each task. A secret is never in the worker's arguments or
environment. The worker has the agent's environment (the agent takes its own
key out of its environment at start), a temporary directory of its own
(`TMPDIR`), which the agent empties after each task and removes when the
worker ends, and the agent's stdout and stderr, so a task's log lines land in
the agent's log. The worker has no API key and makes no request to the
server.

**The memory limit of a task** is its memory request. A task with no request
gets a share of the agent: what the agent offers to tasks, divided by
`max_concurrency`. What the agent offers is `--memory` less
`--reserved-memory`; an agent with no `--memory` uses the memory limit of its
own cgroup. An agent with neither holds a task with no request to no limit.

So when this version is installed, every task of every package that is already
published runs in a worker process and has a memory limit, without a new
publish. A pool with the default 1Gi pods and four slots gives a task with no
request 224Mi.

**What is enforced, and how.**

| What | How | When it acts |
|---|---|---|
| Memory of one task | The agent reads the memory of each worker, with every process the task started, every 250 ms, and kills the worker that is over its limit. On Linux it counts anonymous and shared memory (`RssAnon` + `RssShmem`); pages of a mapped file are not counted. In a warm worker the count includes what the worker holds from its earlier tasks. | Within 250 ms of the task passing its limit. The attempt fails with `task exceeded its memory limit of <limit>`. The other tasks on the agent go on. |
| Memory of the agent as a whole | Above 80% of the agent's memory the agent ends its largest idle warm worker. Above 95% it ends an idle warm worker if one is left, else kills its largest worker. It reads its own cgroup (cgroup v2) where that has a limit, else its own memory plus that of its workers against `--memory`. Idle warm workers count. | Within 250 ms. A killed task's attempt fails with `the agent ran out of memory`. The agent stays registered. |
| The kernel's out-of-memory kill | Each worker sets its `oom_score_adj` to 1000, so that the kernel picks a worker and not the agent where it picks one process. A worker the kernel killed is reported as `the agent ran out of memory: the kernel killed the task's worker`. | When the pod reaches its memory limit between two reads of the agent. |
| Time | The agent kills the worker at the task's [time limit](#time-limits-and-retries). | At the limit. |
| A cancel from the server | When the server's wait for a result ends with no report, it sends the agent a cancel for that dispatch. The agent kills the worker and reports nothing. | When the server gives up; see [recovery](#liveness-and-dead-agent-reclaim). |
| Address space, open files, processes, CPU time | `setrlimit` on the worker (Linux): address space at 8 times the memory limit plus 64 GiB (`--task-address-space-factor`; `0` turns it off; the 64 GiB leave room for what a runtime reserves and never uses, such as the memory of a WASM constructor), 65536 open files, 32768 processes, CPU seconds at the time limit times the host's CPUs (a warm worker sets this soft limit again before each task, to what it used so far plus that allowance). Each is lowered to the agent's own limit where that is lower. | A backstop behind the rows above. A task that hits one fails with its own error or with `the task's worker process ended with no result`. |
| A crash | A task that crashes its process (a segmentation fault, an abort) ends its worker only. | The attempt fails with `the task's worker process ended with no result` and the signal. |

A kill is a `SIGKILL` to the worker's process group: the task gets no chance
to clean up. A kill ends a warm worker too; the next task of its package gets
a new worker. A slot is free when the task's worker has exited, or has handed
back its result and waits for a next task.

**What is not enforced.**

- **CPU.** There is no cap on the CPU of a task. Tasks on one agent share the
  CPU of its pod; a task that uses more than it requested slows its
  neighbours and does not fail them. Workers run at a lower scheduling
  priority (`nice` 5) than the agent, so a busy task does not hold back the
  agent's heartbeat.
- **Between two reads.** The agent reads memory every 250 ms. A task that
  takes memory faster than that can reach the pod's limit before the agent
  acts, and then the kernel acts. On Kubernetes 1.28 and later with cgroup v2
  the kernel kills every process of the container, the agent too, unless the
  kubelet runs with `singleProcessOOMKill: true`. The agent's pod then
  restarts and its tasks are run again.
- **Memory the count leaves out.** Pages of mapped files are not counted for a
  task. Memory that a child and its parent share after a `fork` is counted
  for each of them.
- **cgroups.** The agent does not put a worker in a cgroup of its own. A pod
  that runs as non-root with a read-only root filesystem and no capabilities
  has a read-only cgroup filesystem. The agent logs at start whether it could
  create one.
- **A process that leaves.** A process the task starts in a new session
  leaves the worker's process group. It is still counted for the memory limit
  while its parents live, and it is not killed with the worker.
- **Systems other than Linux.** macOS has the memory watchdog, the time limit
  and the cancel; it has no `setrlimit` limits and no out-of-memory score.

**An inline task** (`inline = true` on the task) runs in the agent process, as
every task did before worker processes. It saves the start of a process and,
for Python, of an interpreter and the import of the package: the agent loads a
package one time and keeps it. It gives up:

- the memory limit: nothing measures it, and its memory counts as the agent's;
- the kill: at its time limit the agent stops waiting and frees the slot, and
  a blocking body runs on; a cancel from the server does nothing;
- the separation: a crash in it takes the agent down with every task on it.

Its requests still count for placement. Mark a task inline when it is short,
uses little memory and is run often.

#### Warm workers

A new worker loads the package: a Rust worker opens the library and registers
its tasks, a Python worker starts an interpreter, unpacks the package and
imports it. That load is most of what a task in a worker costs. So after a
task the worker waits, with the package still loaded, and the agent gives it
the next task of the same **key**:

- the same kind of job (a task, a graph firing, or a
  [poll](#poll-triggers-on-agents)),
- the same package version (the package's digest),
- the same tenant,
- the same memory limit.

A warm worker never runs a task of another tenant, another package or another
version. A new version of a package has a new digest: its tasks start new
workers, and the workers of the old version end at their idle time.

**What two tasks of one warm worker share, and what not.**

| Shared | Not shared |
|---|---|
| The loaded package: a Rust package's statics, a Python package's modules and their module-level state (a cache, a client, a global), as in one process. | Files in `TMPDIR`: the agent empties the directory after each task. |
| Threads a task left running. | The environment: a task that changes an environment variable ends its worker after it; the next task gets a new one. |
| Files written outside `TMPDIR`, such as in the unpacked package. | Processes: a task that leaves a process running ends its worker after it, and the process is killed with the worker. |
| Memory a task left allocated: it counts for the next task's memory limit. | Secrets: each task gets only its own; see the [security model](../security-model/). |

**When a warm worker ends.**

- It was idle for `--warm-worker-idle-seconds` (default 60).
- It ran `--warm-worker-max-jobs` tasks (default 100).
- After a task it holds more than `--warm-worker-max-memory-percent` (default
  50) of the memory limit of its key. A Python process seldom gives memory
  back, so this keeps room for the next task.
- Its task changed the environment or left a process running.
- The agent keeps at most `--warm-workers` idle workers (default: the value of
  `--max-concurrency`); when one more comes, the oldest ends.
- The agent is above 80% of its memory (see the table above).
- Its task was killed: at its memory limit, its time limit, or by a cancel.

`--warm-workers 0` turns warm workers off: each task gets a new worker, which
ends with it. Idle warm workers hold no slot. They hold memory, which counts
for the agent as a whole but is not taken off what the agent offers to tasks.

The agent counts each task that started a new worker and each that ran in a
warm one, and each warm worker it ended, by reason. It sends the counts in its
heartbeat; the server exports them as `cloacina_fleet_worker_starts_total` and
`cloacina_fleet_warm_workers_retired_total` (see the [metrics
catalog](../../../reference/metrics-catalog/)).

**What a task costs in a worker.** Measured on a development machine (Apple
silicon, a debug build, the test binary as the worker), 20 runs each, from the
hand-over of the task to its result, median:

| Task | New worker for each task | Warm worker | Memory of an idle warm worker |
|---|---|---|---|
| Rust task (an empty task of a small workflow library) | 792 ms | 8.7 ms | 28Mi |
| Rust graph firing (a small routing graph) | 226 ms | 2.2 ms | 21Mi |
| Python task, one module | 95 ms | 1.7 ms | 27Mi |
| Python task, a module that imports 16 standard-library modules | 197 ms | 2.2 ms | 44Mi |
| Python graph firing, one node | 114 ms | 1.9 ms | 27Mi |

The warm column includes the first run of the 20, which loads the package. A
worker with no package starts, answers and exits in about 13 ms.

#### Graph firings

The server setting `--fleet-graph-firings`
(`CLOACINA_FLEET_GRAPH_FIRINGS`, chart value `fleet.graphFirings`) says where
an agent runs a computation-graph firing:

- **`inline` (the default).** The agent runs the firing in its own process, as
  an inline task: the package is loaded one time and kept. A firing has no
  memory limit and is not killed; a crash in it takes the agent down.
- **`worker`.** The agent runs each firing in a worker process, with the rows
  of the table above. The agent fetches the package; the worker loads it (a
  Rust graph: its library; a Python graph: the import, with the package's
  providers), runs the graph on the reactor's input snapshot, and hands back
  the outputs. Any other value stops the server at start.

Why inline is the default: a firing in a worker pays the start of a worker and
the load of its package each time, and a reactor waits for each firing before
the next. Measured on a development machine (Apple silicon, a debug build, the
test binary as the worker), a firing of a small Rust routing graph took about
210 ms in a worker, most of it the load of the library, and about 0.2 ms
inline once the library was loaded. With [warm workers](#warm-workers) only
the first firing of a graph version pays the load: the same firing took a
median of 2.2 ms over 20 runs in a warm worker. The agent that runs a firing belongs to
the firing's tenant, so an inline firing shares a process with that tenant's
own work only.

Either way, accumulators, the input cache and the reactor's state stay on the
server, and a firing carries no secrets. Its time limit is the server's
default (`--fleet-task-timeout-s`). With `worker`:

- its memory limit is the share of a task with no request: a graph has no
  memory request;
- a cancel names the firing, and the agent kills that worker;
- the errors name a firing: `graph firing exceeded its memory limit of
  <limit>`, `graph firing exceeded timeout of <n>s`. The reactor gets the
  error as a failed firing and does not run the firing again in-process.

The setting rides in the graph packet as `inline`. An agent from before this
field runs every firing in its own process and acks a cancel and does nothing
with it. A server from before it sends no `inline`, so the agent runs each
firing in a worker; it sends no cancel for a firing, and the agent still kills
the firing's worker at its time limit.

### Which waiting task gets the room that frees

The waiting tasks of a tenant are dispatched in the order of their
[priority](../../priority/): the priority of the task's workflow, then the
task's own, then the time waited, with ageing. In each pass of the scheduler
the tasks look for an agent one after another in that order, so the room
that is free goes to the first task in the order that fits.

A task that fits no agent is put off for two seconds and keeps its place.
While a task that was given a better priority is put off, the worse tasks of
its pool are not dispatched, until they have aged level with it. Tasks that
were given the same priorities do not hold each other back: a small task
still passes a large one of the same priority that waits for room.

A task that runs on an agent is never stopped for more important work.

### What a publish checks

When a package is published, the server checks each of its tasks against the
tenant's pools. The package is refused when a task:

- names a pool the tenant does not have, or
- requests more CPU or memory than one agent of its pool can hold. That size is
  what the pool's settings say: its limit, or its request when it sets no
  limit. For memory the agent's own share, 128Mi, comes off first. A pool that
  sets neither for a resource has no size to compare with.

The check runs when the tenant's runner loads the package, because that is when
its tasks are known: for a Rust package, after the compile. A refused package
is not loaded. Its build status becomes `failed` and its build error holds the
reason, with the task, the pool and the numbers:

```text
package orders v1.4.0 is refused: task 'load_history' requests memory 64Gi,
but one agent of the pool 'large' holds 32Gi less the agent's own 128Mi at
most. Lower the request, or raise the pool's memory limit, and publish again.
```

`cloacinactl package inspect <id>` and the workflow routes show it, the same
way they show a compile error. Fix the task or the pool, then publish the
package again.

A pool for agents you start by hand needs a row too, or tasks that name it are
refused: create it with `max_agents` 0, so the server starts no agents for it.

### What pools do not do

- **The tenant's agent limit is for `default`.** The tenant's agent limit and
  the provision API do not count or bound the other pools; each has its own
  `min_agents` and `max_agents`, set by a platform admin. See
  [How a pool scales](#how-a-pool-scales).
- **No cap on the CPU of a task.** A task's memory is limited and its CPU is
  not. See [worker processes and limits](#worker-processes-and-limits).

### How a change of settings reaches the agents

The pool's settings are stored in the server's database. The fleet loop reads
them on every tick (`CLOACINA_AUTOSCALE_INTERVAL_S`, default 30s) and the
Kubernetes actuator applies each pool's Deployment with server-side apply. A
changed image, resource, concurrency or service account changes the pod
template, so Kubernetes rolls the Deployment: new pods start with the new
settings and the old ones are stopped. An old agent drains first (see
[Stopping an agent](#stopping-an-agent)). A changed egress rule changes the
pool's NetworkPolicy only; no pod restarts.

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
  is `false`: the pod mounts no ServiceAccount token. The agent does not call
  the Kubernetes API and does not need that token. Set it to `true` for a pool
  whose identity system needs the token. The identity
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

Two numbers bound a tenant's pool `default`:

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

The autoscaler moves `desired_count` on its own, and the count of every other
pool: see [How a pool scales](#how-a-pool-scales). After it, the same loop
**reconciles** actual → desired through the actuator. `CLOACINA_AUTOSCALE=false`
freezes automatic scaling while leaving reconciliation running: operators then
drive `desired_count` by hand, and another pool runs its `min_agents`.

The autoscaler decision lives in Cloacina's control plane rather than a
Kubernetes HPA because the signal is *per tenant and per pool* (the tasks that
wait and what they request) and a tenant is the isolation boundary: an HPA
cannot see it.

### How a pool scales

Each tick (`CLOACINA_AUTOSCALE_INTERVAL_S`, default 30 s), for each pool of
each tenant:

1. **The waiting work.** The pool's tasks that are Ready and on no agent:
   tasks put off because no agent had room, and tasks not yet tried.
2. **What does not fit.** They are packed, largest first, onto the free room
   of the pool's live agents: free slots, and CPU and memory less what the
   tasks in flight request. A task with no request counts as one slot's share
   of an agent, as in [placement](#how-a-task-is-placed). What is left does
   not fit.
3. **Up.** What is left is packed, largest first, into empty agents of the
   pool's size (what a live agent of the pool reports, else the pool's
   settings). The pool's count becomes its live agents plus those new agents,
   at most its max, in one step. Agents that are still starting count: a
   tick while they start adds no more. A scale-up does not wait for a
   cooldown.
4. **Down.** A pool with no task in flight and none waiting is idle. When it
   has been idle for `CLOACINA_AUTOSCALE_IDLE_S` (default 300 s), and its last
   scale-down is `CLOACINA_AUTOSCALE_COOLDOWN_S` (default 60 s) ago, it goes
   back to its min. The work is read again just before the write; a task
   placed in between keeps the pool.
5. **Step down.** A pool with work in flight has a spare agent when one of
   its agents is idle (no task in flight, none placed) and nothing waits that
   needs an agent. When it has had a spare agent for
   `CLOACINA_AUTOSCALE_IDLE_S`, it loses one agent, and one more each
   `CLOACINA_AUTOSCALE_COOLDOWN_S` while it stays so, down to its min. The
   agent that goes is an idle one: of several, the one registered last. The
   work is read again just before the write; if that agent got work, or work
   waits, the pool keeps its count.

**Which agent goes.** On each tick the autoscaler lists each pool's idle
agents (the one registered last first) and its busy ones, and the actuator
that lowers the count removes in that order: idle agents first, then agents
not registered yet, busy agents last. A busy agent is never removed while an
idle one exists.

- **Kubernetes.** Each agent's id is its pod's name: the actuator starts the
  agent with `CLOACINA_AGENT_ID` from the pod's `metadata.name`. Before it
  lowers `replicas`, it sets `controller.kubernetes.io/pod-deletion-cost` on
  the pool's pods: below -1000 on the pods of idle agents (lowest on the one
  that goes first), 1000 on those of busy agents, none on a pod of no known
  agent. The ReplicaSet deletes the pods of lowest cost first. This needs
  `list` and `patch` on pods; if the costs cannot be written while an agent
  of the pool is busy, the pool keeps its pods that tick.
- **Docker.** Each container has an agent key of its own, so the agent that
  registered with that key is the container's agent. The actuator stops the
  idle agents' containers by id.

The time a pool has had a spare agent is kept in the memory of the server
replica that leads the tick. A replica that did not lead the last ticks starts
it again, so a step-down can come later than `CLOACINA_AUTOSCALE_IDLE_S`,
never sooner.

So ten tasks that request 2 Gi on a pool of 8 Gi agents with a max of 3 take
the pool from 0 to 3 agents in one tick: each agent offers 8 Gi less its own
128 Mi, so three tasks fit on one, and the tenth waits for a free slot.

| Pool | min | max |
|---|---|---|
| `default` | `CLOACINA_AUTOSCALE_FLOOR` (0), at least 1 for a tenant with an active poll trigger | the tenant's effective limit |
| another pool | its `min_agents`; with none, its `max_agents` (a fixed count, as before pools scaled) | its `max_agents` |

**Scale to zero.** A pool with min 0 has no agent until work comes. With the
autoscaler on, the dispatch of a task for it goes through to the agent
selection, finds no agent and puts the task off for capacity: the task waits
with its attempt kept, and its wait is counted from then. The next tick starts
an agent. The time to the first run is one tick plus the agent's start.

**A task larger than one agent of its pool** (the pool was made smaller after
the package was published) cannot be placed by more agents. It does not scale
the pool and does not keep it busy; it is counted in
`cloacina_fleet_pool_waiting_unplaceable_tasks`.

**What an operator sees.** The pool routes (`GET /v1/tenants/{t}/pools`) give
each pool's `desired_agents` and `live_agents`. The gauges, labelled `tenant`
and `pool`: `cloacina_fleet_pool_agents_desired`,
`cloacina_fleet_pool_agents_ready`, `cloacina_fleet_pool_cpu_requested_millis`,
`cloacina_fleet_pool_cpu_offered_millis`,
`cloacina_fleet_pool_memory_requested_bytes`,
`cloacina_fleet_pool_memory_offered_bytes`, `cloacina_fleet_pool_waiting_tasks`
and `cloacina_fleet_pool_waiting_unplaceable_tasks`; the age of the oldest
waiting task is `cloacina_oldest_waiting_task_age_seconds{tenant,pool}`. Each change of a
count is a log line (tenant, pool, from, to) and
`cloacina_fleet_pool_scaled_total{direction}`.

The state of each pool (its count, since when it is idle, its last
scale-down) is in the database, with the database clock, so the rules hold
whichever server replica leads a tick. The time a pool has had a spare agent
is the one part in a replica's memory (see step 5).

### Stopping an agent

Kubernetes stops an agent pod with SIGTERM: a scale-down, a roll (new
settings, a new agent key, a new image), a node drain. The agent then:

1. takes no new work: a packet that arrives is refused as "stopping", which
   the server reads as "not run" (the task waits for another agent, with no
   use of a retry attempt), and its heartbeat offers no free slot;
2. lets the work it runs finish and report, for up to its drain time
   (`CLOACINA_AGENT_DRAIN_SECONDS`, default 25 s);
3. stops the workers that still run and hands their work back: the task is
   Ready again with the same attempt, and runs again on another agent from
   the start. Then the agent exits.

The pod's grace period must be longer than the drain time. The Kubernetes
default is 30 s. With `CLOACINA_AGENT_DRAIN_S` set on the server, the actuator
gives each agent that drain time and sets the pods'
`terminationGracePeriodSeconds` to it plus 15 s; set it to the longest task
time limit of the tenant's pools so a roll lets every task finish. A task
marked `inline` and a poll run in the agent process and cannot be handed back:
they end with the process, and the server takes the task back by its stale
claim sweep. The Docker actuator stops a container with Docker's own timeout
(10 s), after which Docker kills it.

Because every replica runs the same loop, the fleet is driven through
**Postgres-advisory-lock leader election**: each tick, only the replica holding
the lock executes the loop body; the rest skip it. One replica drives the whole
fleet, so two replicas never scale or actuate the same tenant concurrently
(NFR-003). The knobs above are documented in
[Environment Variables](../../../reference/environment-variables/#fleet-actuator--autoscaler).

## Poll triggers on agents

A server that loads packages from their metadata (no `--single-trust`) runs
no tenant code, so a poll trigger's code runs on an agent of its tenant
(CLOACINA-T-1077, CLOACINA-A-0019). The interval, the target workflow and
`allow_concurrent` come from the package's stored metadata document.

1. **Due.** The scheduler of the tenant's runner reads the trigger's schedule
   row. A poll is due when `poll_interval_ms` has passed since
   `last_poll_at`. The scheduler takes the poll with a compare-and-set on
   `last_poll_at`, so of several server replicas one sends each poll.
2. **Off the loop.** The poll runs on its own task; the scheduler loop does
   not wait for the agent. A trigger whose last poll still runs at its next
   interval is not polled again: that interval is skipped and counted.
3. **Dispatch.** The server picks a live agent of the tenant, pool
   `default`, with a free slot, a runnable arch and the capability
   `job:poll`, and sends a poll packet (kind `agent_poll`) on the delivery
   outbox. A poll takes one slot of the agent while it runs.
4. **On the agent.** The agent fetches the package as for a task and runs
   the poll in a [worker process](#worker-processes-and-limits): the worker
   loads the package (a Python package is imported, a Rust build's library is
   opened), calls the trigger and hands back fire, with the trigger's
   context, or skip. The agent reports the answer.
5. **Fire.** On a fire the server starts the workflow with the context, as
   for a poll in the server: the dedup by context hash and the execution row
   are the same.

**Settings.** `--fleet-poll-timeout-s` / `CLOACINA_FLEET_POLL_TIMEOUT_S`
(default 30) is the time an agent lets one poll run. The server waits that
plus 10 s for the answer.

**Limits of a poll.** A poll is held as a task with no memory request:

- its memory limit is the agent's share for a task with no request; a poll
  over it is killed and fails with `poll exceeded its memory limit of
  <limit>`, and the other work on the agent goes on;
- at `--fleet-poll-timeout-s` the agent kills the poll's worker, with every
  process the poll started, and the poll fails with `poll exceeded timeout of
  <n>s`;
- a crash in a poll ends its worker only.

**Warm workers for polls.** After a poll the worker waits for the next poll
of the same build, tenant and memory limit, with the package loaded, under
the [rules of warm workers](#warm-workers). A poll never takes the warm
worker of a task or a graph firing, and a task never takes a poll's. A
trigger's state in memory (a Rust static, a Python module global) lasts as
long as its warm worker: up to `--warm-worker-max-jobs` polls, and no longer
than `--warm-worker-idle-seconds` between two polls. A trigger whose interval
is longer than the idle time gets a new worker, and a fresh state, at each
poll. Keep state that must last in a store outside the process.

**A poll that gets no answer** fires nothing, and the reason is in
`last_poll_error` of the trigger in the triggers routes and the UI:

- no live agent of the tenant, or none with the capability (an agent of a
  release before poll packets registers none and is never sent a poll);
- every such agent is full;
- no answer in time, or the packet is lost: the server stops waiting. A later
  answer finds no wait and changes nothing. A packet carries the time the
  server stops waiting; an agent that gets it later (for example after the
  packet was moved from an agent that died) does not run the poll;
- the agent refused it, or the trigger returned an error.

The next interval polls again. A poll that answers clears `last_poll_error`.

**One agent for a tenant with a poll trigger.** The fleet control loop reads,
on each tick, which tenants have an enabled, unpaused poll trigger. For those
the autoscaler's floor and the count the actuator reconciles to are at least
one, also for a tenant whose desired count is 0. A tenant with no poll
trigger keeps `CLOACINA_AUTOSCALE_FLOOR` (0 by default) and can scale to zero.
The loop runs only with an actuator; with agents that register themselves,
keep one agent of each such tenant running.

**Cost.** Each poll is one dispatch: a trigger with a 10 s interval sends 360
packets an hour. A poll in a warm worker adds under a millisecond; a poll in a
new worker loads the package. Measured on a development machine (Apple
silicon, a debug build, the test binary as the worker), 20 runs each, median:

| Poll | New worker | Warm worker | In the agent process | Memory of an idle warm worker |
|---|---|---|---|---|
| Python, one module | 90 ms | 0.46 ms | 0.03 ms | 27Mi |
| Rust, a small library | 17 ms | 0.65 ms | 0.03 ms | 18Mi |

There is no setting to run a poll in the agent process: unlike a graph
firing, no other work waits for a poll to end.

**Metrics.** `cloacina_trigger_polls_total{outcome}` (`fire`, `skip`,
`error`, `in_flight`), `cloacina_fleet_polls_total{answer}` (`fire`, `skip`,
`failed`, `missed`), `cloacina_fleet_polls_missed_total{reason}`
(`no_build`, `no_agent`, `enqueue`, `no_answer`, `refused`, `bad_answer`) and
the gauge `cloacina_fleet_poll_floor_tenants`.

With `--single-trust` the server polls in its own process, as the daemon
does; the scheduler's claim and the off-loop poll are the same.

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
