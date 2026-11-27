---
title: "Deploy an Execution-Agent Fleet"
description: "Offload tasks to a pool of DB-less cloacina-agent workers: set the default executor to the fleet, run agents, verify, tune liveness, and operate the fleet"
sidebar:
  order: 58
---

This guide stands up an [execution-agent fleet](../../../explanation/service/execution-agent-fleet/):
a pool of DB-less `cloacina-agent` workers that the server offloads task
execution to. For *why* the fleet exists and how it works internally, read the
explanation; this is the operational path.

## Prerequisites

- A running `cloacina-server` (see [Deploying the API Server](../deploying-the-api-server/)).
- A running `cloacina-compiler`, or another way packages get built, so the
  server has `.cloacina` cdylibs for agents to fetch (see [Running the Compiler](../running-the-compiler/)).
- An API key for the tenant whose work the fleet will run. **The agent's API key
  tenant scope decides which tenants' tasks it may receive** — scope it
  deliberately.
- Network reachability: agents must reach the server over HTTP/WebSocket.
- On each agent host: the shared libraries the compiled workflow cdylibs link
  (`libpq`, `libpython`, `libssl`, `libsasl2`, …). The published agent image
  carries these; a hand-rolled host must install them or `dlopen` fails at load.

## 1. Send tasks to the fleet

Execution topology is a single server-level knob: the **default executor** key.
Every task is dispatched to that one executor — there is no per-task matching.
The default is `default` (the in-process thread executor); set it to `fleet` to
send all work to the agent fleet.

The preferred surface is a `[server]` section in `~/.cloacina/config.toml`, which
`cloacinactl server start` reads and forwards to the `cloacina-server` binary:

```toml
[server]
default_executor = "fleet"
```

For ad-hoc or direct runs you can override it on the binary or via the
environment (precedence: explicit CLI/env > `config.toml` > built-in `default`):

```bash
# All three forms are equivalent overrides:
cloacina-server --default-executor fleet --bind 0.0.0.0:8080
CLOACINA_DEFAULT_EXECUTOR=fleet cloacina-server --bind 0.0.0.0:8080
cloacinactl server start --default-executor fleet
```

> **Deploy the fleet before you select it.** The configured key is hard-matched
> against registered executors at server startup. `fleet` is only a registered
> executor when you've opted into the fleet; if you set `default_executor =
> "fleet"` without the fleet deployed, the server fails fast at boot with an
> error listing the valid keys (e.g. `default`). There is no silent fallback to
> `default`. Set `fleet` together with (or after) standing up the agents below.

## 2. Run the agents

Each agent is a standalone `cloacina-agent` process. The minimum is a server URL
and an API key of role `agent`, scoped to the tenant whose work the agent runs.
No other role may register an agent, and neither may the bootstrap key. Create
the key as a tenant admin or the platform admin:

```bash
curl -s -X POST http://cloacina-server:8080/v1/tenants/acme/keys \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "acme-agents", "role": "agent"}' | jq -r '.key'
```

Then start the agent with it:

```bash
cloacina-agent \
  --server http://cloacina-server:8080 \
  --api-key "$CLOACINA_API_KEY" \
  --max-concurrency 4
```

Or with the published image / Kubernetes — run N replicas, all pointed at the
server, with the key from a secret:

```yaml
env:
  - name: CLOACINA_SERVER
    value: "http://cloacina-server:8080"
  - name: CLOACINA_API_KEY
    valueFrom:
      secretKeyRef: { name: cloacina-agent, key: api-key }
args: ["--max-concurrency", "4"]
```

Scale by adding replicas; the server places each task on a live agent of its
pool with a free slot and room for its requests
([How a task is placed](../../../explanation/service/execution-agent-fleet/#how-a-task-is-placed)).
Useful options (full list in the [CLI reference](../../../reference/cli/#agent)):

- `--max-concurrency <N>` (default 4) — packets this agent runs at once; a
  saturated agent refuses further work.
- `--pool <NAME>` / `CLOACINA_AGENT_POOL` — the agent pool the agent is in
  (default: the pool `default`).
- `--cpu <QTY>` / `CLOACINA_AGENT_CPU` and `--memory <QTY>` /
  `CLOACINA_AGENT_MEMORY` — the total CPU and memory of the agent, as
  Kubernetes quantities (`4`, `32Gi`): the limits of its pod. The server places
  tasks by their requests against these. With neither, the server counts only
  the agent's slots.
- `--reserved-memory <QTY>` / `CLOACINA_AGENT_RESERVED_MEMORY` (default
  `128Mi`) — the agent's own share of `--memory`. The agent offers the rest to
  tasks. A task's memory request is its limit; a task with no request gets the
  offered memory divided by `--max-concurrency`. See
  [worker processes and limits](../../../explanation/service/execution-agent-fleet/#worker-processes-and-limits).
- `--task-address-space-factor <N>` /
  `CLOACINA_AGENT_TASK_ADDRESS_SPACE_FACTOR` (default 8) — the address-space
  ceiling of a task's worker process, as a multiple of its memory limit. `0`
  sets none.
- `--warm-workers <N>` / `CLOACINA_AGENT_WARM_WORKERS` (default: the value of
  `--max-concurrency`) — how many idle
  [warm workers](../../../explanation/service/execution-agent-fleet/#warm-workers)
  the agent keeps, so the next task of a package version and tenant skips the
  load of the package. `0` sets none. `--warm-worker-idle-seconds` (60),
  `--warm-worker-max-jobs` (100) and `--warm-worker-max-memory-percent` (50)
  say when a warm worker ends.
- `--cache-dir <PATH>` / `CLOACINA_AGENT_CACHE_DIR` — persist the fetched-cdylib
  cache across restarts to skip re-fetching artifacts.
- `--capabilities a,b` — advertise free-form tags at registration.
- `--log-format json` / `CLOACINA_AGENT_LOG_FORMAT` — write one JSON object per
  log line, with the task's ids on the lines written while a task runs. See
  [Find a Task's Logs](../find-a-tasks-logs/).
- `--api-key-file <PATH>` / `CLOACINA_API_KEY_FILE` — read the key from a file
  in place of `CLOACINA_API_KEY`.

**The key and task code.** A task runs in a worker process that the agent
starts, with the agent's user id and the agent's environment; a task marked
`inline` runs inside the agent process. At start the
agent reads `CLOACINA_API_KEY`, overwrites the value in its start-time
environment and removes the variable, so a task finds no key in `std::env`,
`os.environ` or `/proc/self/environ`, and a worker process starts with no key. A key given with `--api-key` on the command
line stays in the process's argument list, and a key file stays readable by the
agent's user: for an agent that runs untrusted tasks, give the key in
`CLOACINA_API_KEY`. See
[what a task can reach on its agent](../../../explanation/service/security-model/#what-a-task-can-reach-on-its-agent).

> **Architecture must match.** Agents `dlopen` the compiler's cdylibs — native
> code that only runs on the target triple it was built for. The fidius wire
> format is always bincode regardless of build profile, so debug/release
> mixing is not a compatibility concern; matching the CPU architecture is (see
> the [multi-architecture dispatch](../../../explanation/service/execution-agent-fleet/#multi-architecture-dispatch)
> story).

## 3. Verify it's running on the fleet

With `default_executor = "fleet"`, every task runs on the fleet. Upload and run
any workflow, then confirm it executed on an agent rather than in-process:

```bash
cloacinactl package upload my-workflow.cloacina
cloacinactl workflow run my_workflow            # prints an execution id
cloacinactl execution status <execution-id>     # -> Completed
```

Confirm the fleet path (not the `default` executor) handled it:

- **Server log:** `fleet: agent reported; reconciling via shared TaskResultHandler`.
- **Agent log:** the agent registering the package's cdylib as it loads it.
- **Metrics:** `/metrics` exposes the `cloacina_fleet_*` and
  `cloacina_delivery_outbox_open` series.

If the task instead runs on `default`, the server isn't configured for the fleet
— re-check that `default_executor` resolves to `fleet` (config.toml `[server]`,
`CLOACINA_DEFAULT_EXECUTOR`, or `--default-executor`).

## 4. Tune failover aggressiveness

Dead-agent detection and in-flight reclaim are governed by two server flags
(defaults reproduce the prior hard-coded 15s / 45s):

| Flag / env | Default | Effect |
|---|---|---|
| `--agent-heartbeat-interval-s` / `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `15` | Heartbeat cadence advertised to agents + the sweep tick. |
| `--agent-liveness-misses` / `CLOACINA_AGENT_LIVENESS_MISSES` | `3` | Missed beats before an agent is declared dead. |
| `--fleet-task-timeout-s` / `CLOACINA_FLEET_TASK_TIMEOUT_S` | `300` | Time limit (seconds) of a fleet task that sets none of its own. |
| `--fleet-task-timeout-max-s` / `CLOACINA_FLEET_TASK_TIMEOUT_MAX_S` | `86400` | Longest time limit a task can get. A task that asks for more gets this value. |
| `--fleet-artifact-fetch-wait-s` / `CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S` | `300` | How long a task waits, per attempt, while agents cannot fetch its package artifact. After that the attempt fails. `0`: the first failed fetch fails the attempt. |

Effective dead-after = interval × misses. For faster failover (at the cost of
more heartbeat traffic), e.g. ~10s detection:

```bash
CLOACINA_AGENT_HEARTBEAT_INTERVAL_S=5 CLOACINA_AGENT_LIVENESS_MISSES=2 cloacina-server ...
```

## 5. Operate

- **Losing an agent is safe.** When an agent dies, the sweeper reclaims its
  in-flight work onto a live agent in the same tenant and the task completes
  there — no workflow-level failure. The work re-runs from the start, so
  tasks run on the fleet should be idempotent.
- **Watch these signals:**
  - `cloacina_delivery_outbox_open` climbing → delivery is wedged (often no live
    agent for a tenant's work, or agents can't connect).
  - `cloacina_fleet_agents_evicted_total` rising → agents dying / flapping.
  - `cloacina_fleet_work_reassigned_total` → how much work crashed agents are
    shedding onto survivors.
- **Capacity:** if `delivery_outbox_open` grows under steady load, add agents or
  raise `--max-concurrency`.
- **Draining:** to retire an agent gracefully, stop sending it new work by
  scaling the pool and let it finish in-flight packets; a hard kill is recovered
  by the reclaim path within the detection window.

## Let the server provision the fleet (control plane)

Everything above runs agents by hand. The **control plane** (CLOACI-I-0127) can
instead provision and scale a per-tenant agent pool *for* you, on a pluggable
substrate (a *FleetActuator*). It is off by default
(`CLOACINA_FLEET_ACTUATOR=none`); opt in per substrate.

### Capacity limits & self-service provisioning

Each tenant has an **effective limit** — the platform default
(`CLOACINA_DEFAULT_MAX_AGENTS`, default 4) unless a platform admin grants a
per-tenant override — and a **`desired_count`** in `[0, effective_limit]`
that a tenant scales for itself via the
[tenant agent fleet API](../../../reference/http-api/#tenant-agent-fleet).
Provisioning only sets the target; an actuator (below) turns it into running
agents.

```bash
# Tenant-admin: view, then scale this tenant's fleet (+1 / -1).
curl -H "Authorization: Bearer $TENANT_ADMIN_KEY" \
  http://cloacina-server:8080/v1/tenants/tenant_acme/fleet
curl -X POST -H "Authorization: Bearer $TENANT_ADMIN_KEY" \
  http://cloacina-server:8080/v1/tenants/tenant_acme/fleet/provision   # → 409 at capacity

# Platform-admin (god key): raise the tenant's ceiling.
curl -X POST -H "Authorization: Bearer $ADMIN_KEY" \
  -H "content-type: application/json" -d '{"max_agents": 8}' \
  http://cloacina-server:8080/v1/tenants/tenant_acme/limits
```

A newly created tenant is auto-seeded with
`min(CLOACINA_INITIAL_AGENTS, CLOACINA_DEFAULT_MAX_AGENTS)` (default 1) on
`POST /v1/tenants`; set `CLOACINA_INITIAL_AGENTS=0` to disable.

### Size the agents: pools

A tenant's managed agents are grouped in pools, each with its own size. Every
tenant has the pool `default`. A platform admin sets a pool's settings; each
field is optional, and one that is absent means the server-wide default.

```bash
# Platform-admin (god key): make the tenant's default pool large, one task per
# agent, with a cloud identity and a path to object storage.
curl -X PUT -H "Authorization: Bearer $ADMIN_KEY" \
  -H "content-type: application/json" \
  http://cloacina-server:8080/v1/tenants/tenant_acme/pools/default -d '{
    "cpu_request": "2", "cpu_limit": "8",
    "memory_request": "16Gi", "memory_limit": "32Gi",
    "max_concurrency": 1,
    "service_account_name": "team-data",
    "service_account_annotations": {
      "eks.amazonaws.com/role-arn": "arn:aws:iam::000000000000:role/example-team"
    },
    "extra_egress": [
      { "cidr": "198.51.100.0/24", "ports": [{ "port": 443 }] }
    ]
  }'

# The same with cloacinactl.
cloacinactl tenant pool set tenant_acme default \
  --cpu-request 2 --cpu-limit 8 --memory-request 16Gi --memory-limit 32Gi \
  --max-concurrency 1 --service-account team-data \
  --service-account-annotation eks.amazonaws.com/role-arn=arn:aws:iam::000000000000:role/example-team \
  --egress 198.51.100.0/24=443

cloacinactl tenant pool list tenant_acme
```

`PUT` replaces the pool's settings as a whole: send every setting you want to
keep. The server checks the values (a malformed quantity such as `32GB`, a
request above its limit, a CIDR block with host bits set are a `400`). The
actuator applies the settings on its next tick (30s by default) and Kubernetes
rolls the pool's agents.

### Run large tasks in their own pool

A tenant whose tasks are mostly small, with some that need about 32 GB, runs
the large ones in a second pool.

1. Create the pool. Its limits are what one agent of the pool can hold.

   ```bash
   cloacinactl tenant pool set tenant_acme large --cpu-limit 4 \
     --memory-limit 32Gi --memory-request 16Gi --max-concurrency 8 --agents 2
   ```

   A pool other than `default` runs a fixed number of agents (`--agents`, the
   `max_agents` setting). It is not autoscaled, and the tenant's agent limit
   does not count it.

2. Name the pool on the tasks that need it, with what each one needs.

   ```rust
   #[task(pool = "large", cpu = "2", memory = "24Gi")]
   pub async fn load_history(context: &mut Context<Value>) -> Result<(), TaskError> { Ok(()) }
   ```

   ```python
   @cloaca.task(pool="large", cpu="2", memory="24Gi")
   def load_history(context):
       return context
   ```

   A task with no `pool` stays on `default`.

3. Publish the package. A task that names a pool the tenant does not have, or
   that requests more than one agent of its pool holds, makes the package fail:
   `cloacinactl package inspect <id>` shows build status `failed` and an error
   that names the task. Create the pool before you publish.

4. Watch the pool. `cloacina_waiting_tasks{tenant="tenant_acme", pool="large"}`
   is the number of tasks of the pool that wait for room, and
   `cloacina_oldest_waiting_task_age_seconds` with the same labels is how long
   the oldest has waited. A count that stays above zero means the pool needs
   more agents (`--agents`) or larger ones.

The server packs the tasks of a pool onto its agents by their requests. An
agent with 32Gi keeps 128Mi for itself; it runs one 24Gi task and three 2Gi
tasks together, and a second 24Gi task waits for room or goes to the pool's
other agent. The rule is in
[How a task is placed](../../../explanation/service/execution-agent-fleet/#how-a-task-is-placed).

A memory request is also the task's limit. The agent runs each task in a
worker process and stops a task that uses more memory than it requested: the
task fails with `task exceeded its memory limit of 24Gi`, and the other tasks
on the agent go on. Request what the task needs at its peak. A task with no
memory request gets a share of the agent as its limit. A CPU request is not a
cap. What is enforced and what is not is in
[worker processes and limits](../../../explanation/service/execution-agent-fleet/#worker-processes-and-limits).

A task that is short, uses little memory and runs often can skip the worker
process: mark it `inline = true` (`#[task(inline = true)]`,
`@cloaca.task(inline=True)`). It then runs inside the agent process, with no
memory limit and no kill.

To use agents you start by hand in a pool, create the pool with `--agents 0`
and give each agent its pool and its size:

```bash
cloacinactl tenant pool set tenant_acme large --cpu-limit 4 --memory-limit 32Gi --agents 0
cloacina-agent --server http://cloacina-server:8080 --api-key-file /run/secrets/agent-key \
  --pool large --cpu 4 --memory 32Gi --max-concurrency 8
```

An agent started with no `--cpu` and no `--memory` reports no size. The server
then counts only its slots (`--max-concurrency`).

To remove a pool, set it to 0 agents, wait until its agents have stopped, then
delete it:

```bash
cloacinactl tenant pool set tenant_acme large --agents 0
cloacinactl tenant pool delete tenant_acme large
```

See [Agent pools](../../../explanation/service/execution-agent-fleet/#agent-pools)
for how a change reaches the agents and what the identity and egress settings do.

### Give a pool a cloud identity

Cloacina has no cloud-specific code. A pool has four generic settings, and a
cloud's workload identity system is set up with them:

| Setting | What the actuator does |
|---|---|
| `service_account_name`, `service_account_annotations` | Creates the ServiceAccount in the tenant namespace with the annotations; the pool's pods run as it. |
| `pod_labels` | Adds the labels to the pool's pods. |
| `pod_annotations` | Adds the annotations to the pool's pods. |
| `automount_service_account_token` | Sets the pod's field of that name. Not set: `false`, no ServiceAccount token is mounted. |

The three recipes below are examples of those settings. The account ids, role
names and addresses are placeholders. Each recipe shows the identity settings
only. `set` replaces all of a pool's settings, so give the pool's resources in
the same command. The cloud side (the role, its trust in the cluster's identity
provider, its permissions) is yours to set up.

**Amazon EKS, IAM roles for service accounts.** An annotation on the
ServiceAccount names the role. A webhook in the cluster then adds a projected
token volume and environment variables to pods that use the ServiceAccount.

```bash
cloacinactl tenant pool set tenant_acme default \
  --service-account team-data \
  --service-account-annotation eks.amazonaws.com/role-arn=arn:aws:iam::000000000000:role/example-team \
  --automount-service-account-token false \
  --egress 198.51.100.0/24=443
```

**Google GKE, Workload Identity.** An annotation on the ServiceAccount names the
Google service account. Pods get credentials from the node's metadata server.

```bash
cloacinactl tenant pool set tenant_acme default \
  --service-account team-data \
  --service-account-annotation iam.gke.io/gcp-service-account=example-team@example-project.iam.gserviceaccount.com \
  --egress 198.51.100.0/24=443
```

**Azure AKS, Workload Identity.** An annotation on the ServiceAccount names the
client id, and a label on the pod turns the webhook on for it. The webhook adds
a projected token volume and environment variables.

```bash
cloacinactl tenant pool set tenant_acme default \
  --service-account team-data \
  --service-account-annotation azure.workload.identity/client-id=00000000-0000-0000-0000-000000000000 \
  --pod-label azure.workload.identity/use=true \
  --automount-service-account-token false \
  --egress 198.51.100.0/24=443
```

**Egress.** A managed agent reaches cluster DNS and the server only, so the
pool needs egress rules for the identity and for the service the task calls.
`198.51.100.0/24` above stands for your real ranges. The rules are CIDR blocks,
not host names: for an endpoint whose addresses change, use a private endpoint
in your network (its subnet is a fixed range) or the provider's published
ranges.

- **EKS:** the AWS STS endpoint on port 443, where the pod exchanges its token
  for credentials, and the endpoints of the services the task calls (object
  storage, the catalog) on 443.
- **GKE:** the node's metadata server, a link-local address that your GKE
  documentation gives with its port, and the Google APIs the task calls on 443.
- **Azure:** the Microsoft Entra login endpoint on port 443, where the pod
  exchanges its token, and the endpoints of the services the task calls on 443.

How an `ipBlock` rule applies to a node-local address such as a metadata server
depends on the cluster's network plugin: follow the plugin's documentation for
that address. A missing rule fails closed: the task gets a network timeout.

**The mounted Kubernetes token.** A pool's pods mount no ServiceAccount API
token unless the pool sets `--automount-service-account-token true`. The agent
does not use that token. The token a workload identity webhook injects is a
separate projected volume and still arrives.

A changed label, annotation, ServiceAccount or automount setting changes the
pod template, so the pool's agents are rolled. Setting a ServiceAccount's
annotations alone does not restart pods: a webhook acts when a pod is created,
so change something in the pod template too, or delete the pods.

### Docker actuator (local dev)

For a single-host dev loop, the Docker actuator spawns/stops `cloacina-agent`
**containers** to match each tenant's `desired_count`:

```bash
CLOACINA_FLEET_ACTUATOR=docker \
CLOACINA_AGENT_IMAGE=cloacina-agent:latest \
CLOACINA_AGENT_NETWORK=cloacina_net \
CLOACINA_AGENT_SERVER_URL=http://server:8080 \
cloacina-server --default-executor fleet --bind 0.0.0.0:8080
```

The actuator mints a tenant-scoped key of role `agent` per container, injects it
as `CLOACINA_API_KEY`, and revokes it when the container is stopped, so you don't
manage agent keys yourself. The substrate guard
is **fail-closed**: `docker` refuses to start if it detects Kubernetes or finds
no Docker socket. Of a pool's settings it applies the image, the memory and CPU
limits and the max concurrency, and replaces the containers of a pool whose
settings changed; resource requests, the service account and the egress rules
have no Docker counterpart and are ignored. It is **dev-only** — for production use the
[Kubernetes actuator](../deploying-to-kubernetes/#fleet-actuator-kubernetes--rbac),
which scales a per-tenant agent Deployment with least-privilege RBAC.

### Kubernetes actuator (production)

On Kubernetes the Helm chart drives the actuator for you (`fleet.actuator=kubernetes`).
Beyond least-privilege RBAC and per-tenant namespaces, it hardens the fleet for a
production cluster:

- **`restricted`-clean agent pods** — non-root (uid/gid `10001`), dropped
  capabilities, `readOnlyRootFilesystem` with `emptyDir`s for the agent's
  writable paths, `seccompProfile: RuntimeDefault`, and configurable
  resource requests/limits (`fleet.agentResources`). The pods carry **no probes**
  — the agent has no health endpoint; liveness is the server's heartbeat/eviction
  sweep, tuned by `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` / `CLOACINA_AGENT_LIVENESS_MISSES`.
- **No service account token** — the pods set
  `automountServiceAccountToken: false`. The agent never calls the Kubernetes API.
- **Per-tenant `NetworkPolicy`** (`fleet.networkPolicy.enabled`, default on) —
  deny all ingress; egress only to DNS + the `cloacina-server`, plus the extra
  egress rules of each pool for that pool's pods. Defense-in-depth; the
  server-side ABAC remains the real boundary.

See [Deploying to Kubernetes → Hardened agent pods + NetworkPolicy](../deploying-to-kubernetes/#hardened-agent-pods--networkpolicy)
for the full knob list.

### Autoscaler tuning

When an actuator is active, a leader-gated control loop autoscales
`desired_count` from per-tenant utilization (Σ `in_flight` / Σ `max_concurrency`)
and reconciles toward it. Tune it with the `CLOACINA_AUTOSCALE_*` knobs — up/down
thresholds, cooldown, floor, and tick interval — documented in
[Environment Variables](../../../reference/environment-variables/#autoscaler).
Set `CLOACINA_AUTOSCALE=false` to freeze automatic scaling and drive
`desired_count` by hand through the provision API while reconciliation keeps
running.

## See also

- [Execution-Agent Fleet](../../../explanation/service/execution-agent-fleet/) — how it works.
- [CLI Reference](../../../reference/cli/#agent) — `cloacina-agent` + server fleet flags.
- [Environment Variables](../../../reference/environment-variables/#execution-agent) — agent + fleet env vars.
- [Metrics Catalog](../../../reference/metrics-catalog/) — the `cloacina_fleet_*` series.
