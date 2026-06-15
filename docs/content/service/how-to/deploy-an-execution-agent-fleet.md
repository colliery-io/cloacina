---
title: "Deploy an Execution-Agent Fleet"
description: "Offload tasks to a pool of DB-less cloacina-agent workers: set the default executor to the fleet, run agents, verify, tune liveness, and operate the fleet"
weight: 58
---

# Deploy an Execution-Agent Fleet

This guide stands up an [execution-agent fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}):
a pool of DB-less `cloacina-agent` workers that the server offloads task
execution to. For *why* the fleet exists and how it works internally, read the
explanation; this is the operational path.

## Prerequisites

- A running `cloacina-server` (see [Deploying the API Server]({{< ref "deploying-the-api-server" >}})).
- A running `cloacina-compiler`, or another way packages get built, so the
  server has `.cloacina` cdylibs for agents to fetch (see [Running the Compiler]({{< ref "running-the-compiler" >}})).
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
and an API key:

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

Scale by adding replicas; the server spreads work across live agents greedily by
free capacity. Useful options (full list in the [CLI reference]({{< ref "/reference/cli" >}}#agent)):

- `--max-concurrency <N>` (default 4) — packets this agent runs at once; a
  saturated agent refuses further work.
- `--cache-dir <PATH>` / `CLOACINA_AGENT_CACHE_DIR` — persist the fetched-cdylib
  cache across restarts to skip re-fetching artifacts.
- `--capabilities a,b` — advertise free-form tags at registration.

> **Build profile must match.** Agents `dlopen` the compiler's cdylibs, and the
> fidius wire format depends on the build profile (debug = JSON, release =
> bincode). Release agents need release-built packages. Production images are
> release; build your workflow packages release too.

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

## See also

- [Execution-Agent Fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}) — how it works.
- [CLI Reference]({{< ref "/reference/cli" >}}#agent) — `cloacina-agent` + server fleet flags.
- [Environment Variables]({{< ref "/reference/environment-variables" >}}#execution-agent) — agent + fleet env vars.
- [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}}) — the `cloacina_fleet_*` series.
