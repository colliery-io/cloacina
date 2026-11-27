---
title: "Deploying to Kubernetes (Helm)"
description: "Install cloacina-server on Kubernetes using the official Helm chart"
sidebar:
  order: 57
---

The official Helm chart for `cloacina-server` is published as an OCI
artifact to `ghcr.io/colliery-io/charts/cloacina-server`. This
page covers install, configuration, and upgrade.

## Prerequisites

- Kubernetes 1.27+
- `kubectl` and `helm` 3.8+ (OCI support)
- A reachable Postgres instance (managed RDS / Cloud SQL / etc., or use the bundled subchart for demos)

## Quick install

### Bring-your-own Postgres (recommended)

```sh
helm install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --set database.url=postgres://cloacina:cloacina@postgres.svc:5432/cloacina \
  --set fleet.actuator=kubernetes
```

### With a `DATABASE_URL` Secret

```sh
kubectl create secret generic cloacina-db \
  --from-literal=DATABASE_URL=postgres://cloacina:s3cr3t@postgres.svc:5432/cloacina

helm install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --set databaseUrlSecretRef.name=cloacina-db \
  --set fleet.actuator=kubernetes
```

### Bundled Postgres (demo / dev)

```sh
helm install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --set postgresql.enabled=true \
  --set postgresql.auth.password=$(openssl rand -hex 16) \
  --set fleet.actuator=kubernetes
```

This enables the bundled **in-tree** `postgresql` subchart — a Service + PVC around the official `docker.io/library/postgres` image (it replaced the Bitnami chart). **Don't use this in
production** — the credentials are exposed via Helm values, and the
PVC isn't backed up by anything you don't run yourself.

The chart fails fast if you don't pick exactly one of these three paths.

## Where tenant tasks run

The server runs no tenant code in its own process (see the
[security model](../../../explanation/service/security-model/#tenant-code-and-the-server-process)):
it loads each package from the metadata document its compiler stored and
sends every task to an agent of the tenant. The chart fails at render unless
the install says where the agents come from:

| Value | Agents | What runs in the server |
|---|---|---|
| `fleet.actuator=kubernetes` | the server makes them, in each tenant's namespace ([below](#fleet-actuator-kubernetes--rbac)) | no tenant code |
| `fleet.externalAgents=true` | you run them, for example with the `cloacina-agent` chart ([Deploy an execution-agent fleet](../deploy-an-execution-agent-fleet/)) | no tenant code |
| `server.singleTrust=true` | none needed | every tenant's packages are loaded and their tasks run in the server process |

With `fleet.actuator=kubernetes` or `fleet.externalAgents=true` a task waits
until an agent of its tenant can take it. A package that uses a poll trigger,
a stream accumulator on a native provider, a Python polling or stream
accumulator, or a polling accumulator in a Rust package does not load: its
`build_status` is `failed` and `build_error` names what it uses. Python
reactors and graphs with other accumulators load, and their firings run on
agents. A native provider that only a constructor-node task uses loads: the
task runs on an agent.

`server.singleTrust=true` is for a server whose every tenant is trusted as
much as the server: one team, or one person. Each tenant's code then reaches
the database, the secrets key and the other tenants' data. The chart passes
`--single-trust` and the server logs a warning at start.

### Upgrading from a chart with no single-trust setting

Before this setting a chart install ran every task in the server process
(`fleet.actuator=none`, no executor set). After the upgrade:

- `helm upgrade` with the old values fails at render until one of the three
  values above is set.
- To keep running tasks in the server, set `server.singleTrust=true`. Nothing
  else changes.
- To move tasks to agents, set `fleet.actuator=kubernetes` or run agents and
  set `fleet.externalAgents=true`. Upgrade the compiler with the server: the
  server loads a package only once its compiler has stored the package's
  metadata document, and the compiler describes the packages built before on
  its own ([Running the Compiler](../running-the-compiler/)). Until then the
  workflow detail gives `not_loaded_reason`.
- Without single trust, a package with code that would run in the server (the
  list above) is refused at its first load and gets `build_status` `failed`.
  The refusal stays when the setting changes: to load it with single trust
  after a refusal, publish a new version of it. Set `server.singleTrust=true`
  before the upgrade to keep such packages in service.
- A package described by an earlier compiler keeps the stricter rule of that
  document until its compiler describes it again: any bundled native provider
  and any Python reactor, graph or accumulator refuse it. Upgrade the compiler
  first, so its pass describes the packages again before the server loads
  them.
- With `CLOACINA_DEFAULT_EXECUTOR=default` in `server.extraEnv`, the server
  does not start without `server.singleTrust=true`.

## Pinning the bootstrap API key

By default, `cloacina-server` generates a random admin API key on first
start and prints it to stdout. To pin one:

```sh
kubectl create secret generic cloacina-bootstrap \
  --from-literal=bootstrap-key=$(openssl rand -hex 32)

helm upgrade --install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --set apiKeySecretRef.name=cloacina-bootstrap \
  --reuse-values
```

The chart wires the secret into the container as
`CLOACINA_BOOTSTRAP_KEY`.

## Ingress + TLS

```yaml
# values-prod.yaml
ingress:
  enabled: true
  className: nginx
  hosts:
    - host: cloacina.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: cloacina-tls
      hosts:
        - cloacina.example.com
```

```sh
helm upgrade --install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --values values-prod.yaml
```

cert-manager users can wire issuance via the standard
`cert-manager.io/cluster-issuer` annotation under `ingress.annotations`.

## Probes + signatures + tenants

| Knob | Default | Effect |
|------|---------|--------|
| `livenessProbe`, `readinessProbe`, `startupProbe` | `/health` (liveness/startup) and `/ready` (readiness) HTTP | Override per-probe; set to `null` to disable |
| `server.requireSignatures` | `false` | Reject unsigned packages |
| `server.verificationOrgId` | `""` | Server-wide trusted org UUID (optional); a key in it is trusted for every tenant |
| `server.tenantRunnerCacheSize` | `256` | Warning limit on per-tenant `DefaultRunner` instances; none is stopped at the limit |
| `retention.runHistoryDays` | `0` | Days to keep a finished run; `0` keeps run history for ever. See [Run history retention](../production-deployment/#run-history-retention) |
| `fleetTaskTimeLimits.defaultSeconds` / `.maxSeconds` | `300` / `86400` | Time limits of fleet tasks |
| `alertWebhook.url`, `alertWebhook.secretRef` | `""` | Alert webhook and its signing secret from a Secret. See [Alert on a Failed Run](../alert-on-a-failed-run/) |
| `context.warnBytes` | `1048576` | Size in bytes above which a task context is logged as a warning and counted (`0` = off) |
| `context.maxBytes` | `16777216` | Largest size in bytes of a task context; a task with a larger one fails and is not retried (`0` = off) |

See `values.yaml` for the full reference.

## Metrics and Prometheus

`/metrics` has no authentication, and its labels hold tenant names,
workflow names and other names that tenants choose. The chart keeps it
off the API port by default:

- The server starts with `--metrics-bind 0.0.0.0:9090`. `/metrics` is
  served on port 9090 only, and `:8080/metrics` answers 404.
- The ClusterIP Service `<fullname>-metrics` exposes port 9090. The API
  Service and the Ingress do not.
- A NetworkPolicy for the server pods keeps the API port open to every
  source and opens port 9090 only to the peers in
  `metrics.networkPolicy.from`. The list is empty by default, so no pod
  can reach the metrics port until you name your Prometheus.

### Let Prometheus scrape

Name the pods that may scrape, and turn on the `ServiceMonitor` if you
run the Prometheus operator:

```yaml
metrics:
  networkPolicy:
    from:
      - namespaceSelector:
          matchLabels:
            kubernetes.io/metadata.name: monitoring
        podSelector:
          matchLabels:
            app.kubernetes.io/name: prometheus
serviceMonitor:
  enabled: true
```

The `ServiceMonitor` selects the Service `<fullname>-metrics`, port
`metrics`, path `/metrics`, every 30 seconds. Without the Prometheus
operator, point a scrape job at
`<fullname>-metrics.<namespace>.svc:9090/metrics`.

With `compiler.enabled=true` the compiler pods get the same treatment:
`--metrics-bind` on `metrics.port`, the Service
`<fullname>-compiler-metrics` (port `cmp-metrics`), a second
`ServiceMonitor` named `<fullname>-compiler`, and a NetworkPolicy that
keeps the compiler's bind port open and opens the metrics port only to
the peers in `from`.

| Knob | Default | Effect |
|------|---------|--------|
| `metrics.enabled` | `true` | Starts the server (and the compiler, when enabled) with `--metrics-bind 0.0.0.0:<port>`. Adds the container port and the ClusterIP metrics Service. `:8080/metrics` answers 404 |
| `metrics.port` | `9090` | Port of the metrics listener |
| `metrics.networkPolicy.enabled` | `true` | Adds the NetworkPolicy for the server pods (and the compiler pods). Rendered only with `metrics.enabled=true`. Takes effect on a CNI that enforces NetworkPolicy |
| `metrics.networkPolicy.from` | `[]` | NetworkPolicy peers that may reach the metrics port. Empty admits nobody |
| `serviceMonitor.enabled` | `false` | Emits a `ServiceMonitor` for the metrics Service |

### Who can reach the metrics port

- Agents that the fleet actuator provisions run in namespaces named
  `cloacina-tenant-<tenant>`, labelled `cloacina.managed: "true"`. Their
  own NetworkPolicy (`fleet.networkPolicy.enabled`, on by default)
  allows egress only to cluster DNS and to the server pods on the API
  port. Port 9090 is not in that list.
- The server NetworkPolicy closes port 9090 to every pod that is not in
  `from`. This covers agents installed with the `cloacina-agent` chart,
  agents in a cluster where `fleet.networkPolicy.enabled=false`, and
  every other workload.
- Both policies need a CNI that enforces NetworkPolicy. On a CNI that
  does not, the metrics port is reachable from every pod in the cluster;
  the API port still answers 404 for `/metrics`.

### `/metrics` on the API port

`metrics.enabled=false` puts `/metrics` back on port 8080, where every
client that can reach the API can read it, each tenant's agents
included. The chart then renders no metrics Service and no
NetworkPolicy, and the `ServiceMonitor` scrapes the API Service on port
`http`.

### Upgrading from a chart where `metrics.enabled` was `false`

- Scrape `<fullname>-metrics:9090/metrics`, not
  `<fullname>:8080/metrics`. A scrape of the API port gets 404.
- With `serviceMonitor.enabled=true` the chart moves the `ServiceMonitor`
  to the metrics Service on upgrade.
- Add the Prometheus pods to `metrics.networkPolicy.from`. Until then
  the NetworkPolicy refuses the scrape.
- The server image must have `--metrics-bind`: a release after 0.12.0.
  With an older image, set `metrics.enabled=false`.

## The compiler and Python dependencies

With `compiler.enabled=true` the chart's compiler vendors the dependencies of
a Python package that is published with a `requirements.lock` and no `vendor/`
folder. Its image must hold `uv`. By default it downloads the wheels from PyPI,
for `aarch64-linux` and `x86_64-linux` and Python 3.11.

| Knob | Default | Effect |
|------|---------|--------|
| `compiler.python.platforms` | `[]` | The platforms to vendor for (`--python-platform`). Empty: `aarch64-linux` and `x86_64-linux` |
| `compiler.python.version` | `""` | The Python the wheels are for (`--python-version`). Empty: `3.11` |
| `compiler.python.indexUrl` | `""` | The package index (`CLOACINA_COMPILER_PYTHON_INDEX_URL`). Empty: PyPI. For a URL with a password, set the variable from a Secret through `compiler.extraEnv` |
| `compiler.python.wheelDir` | `""` | A folder of wheel files in the compiler image (`--python-wheel-dir`): one more source next to the index, or the only source with `offline` |
| `compiler.python.offline` | `false` | No network for Python builds (`--python-offline`). Needs `wheelDir`, or a `uv` cache in the image |
| `compiler.python.uvCacheDir` | `/tmp/uv-cache` | The cache folder of `uv` (`--python-uv-cache-dir`), on the pod's `/tmp` volume |

The image recipe for a compiler that carries its wheels, and what the compiler
downloads, are in
[Running the Compiler](../running-the-compiler/#python-packages-vendoring-dependencies).

## Fleet actuator (Kubernetes) + RBAC

The server can provision and autoscale a per-tenant pool of `cloacina-agent`
workloads itself, instead of you running agents by hand (the agent
self-management control plane, CLOACI-I-0127). On Kubernetes this is the
**Kubernetes fleet actuator**. `fleet.actuator` is `none` by default: no
ServiceAccount, no RBAC and no actuator env are rendered. An install with
`none` sets `fleet.externalAgents=true` or `server.singleTrust=true`
([Where tenant tasks run](#where-tenant-tasks-run)). (The Docker actuator is
dev-only and is **not** offered through this chart.)

### Enable it

```sh
helm upgrade --install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --reuse-values \
  --set fleet.actuator=kubernetes
```

Setting `fleet.actuator=kubernetes` makes the chart:

1. Render the ServiceAccount + least-privilege RBAC below and run the server pod
   as that ServiceAccount.
2. Wire the actuator env into the container: `CLOACINA_FLEET_ACTUATOR=kubernetes`,
   `CLOACINA_AGENT_IMAGE` (from `fleet.agentImage`, default
   `ghcr.io/colliery-software/cloacina-agent:latest`), and
   `CLOACINA_AGENT_SERVER_URL` (from `fleet.agentServerUrl`, defaulting to this
   release's in-cluster Service DNS).
3. Render the agent-pod hardening + per-tenant NetworkPolicy knobs (see
   [Hardened agent pods + NetworkPolicy](#hardened-agent-pods--networkpolicy)).

| Value | Default | Effect |
|------|---------|--------|
| `fleet.actuator` | `none` | `none` (off) or `kubernetes`. |
| `fleet.externalAgents` | `false` | `true`: you run the agents; the chart renders with `fleet.actuator=none`. |
| `server.singleTrust` | `false` | `true`: tenant code runs in the server process (`--single-trust`). |
| `fleet.agentImage` | `ghcr.io/colliery-software/cloacina-agent:latest` | Agent image the actuator runs per tenant. |
| `fleet.agentServerUrl` | `""` (→ Service DNS) | URL injected into agents as `CLOACINA_SERVER`. |
| `fleet.agentLogFormat` | `text` | Format of the agents' log lines: `text` or `json`. Rendered as the server's `CLOACINA_AGENT_LOG_FORMAT` when it is not `text`; the actuator sets the same variable on each agent pod. See [Find a Task's Logs](../find-a-tasks-logs/#json-log-lines). |
| `fleet.serviceAccount.name` | `""` (→ `<fullname>-fleet`) | Override the fleet ServiceAccount name. |
| `fleet.serviceAccount.annotations` | `{}` | Annotations on that ServiceAccount (e.g. IRSA / Workload Identity). |
| `fleet.agentResources` | requests `250m`/`256Mi`, limits `1`/`1Gi` | Resource requests/limits applied to every agent pod (rendered as `CLOACINA_AGENT_*`). |
| `fleet.networkPolicy.enabled` | `true` | Install the per-tenant agent NetworkPolicy (deny ingress; egress to DNS + server only). |
| `fleet.networkPolicy.dnsNamespace` | `kube-system` | Namespace the policy allows port-53 egress to. |

### Per-tenant namespaces

The actuator gives each tenant its **own** namespace, `cloacina-tenant-<t>`
(the tenant id is sanitized to a DNS-1123 label). Inside it the actuator manages
one Deployment per [agent pool](../../../explanation/service/execution-agent-fleet/#agent-pools)
and one agent-key Secret. The pool `default` is the Deployment `cloacina-agent`,
as before pools; another pool is `cloacina-agent-<pool>`. Scaling a pool patches
its Deployment's `replicas`. Tenant isolation is the namespace boundary — the
actuator only ever touches the requesting tenant's namespace.

`fleet.agentImage` and `fleet.agentResources` are the server-wide defaults. A
pool's settings override them for that pool: resources, max concurrency, image,
a ServiceAccount with annotations, pod labels and annotations and the
`automountServiceAccountToken` switch (for a cloud workload identity; see
[Give a pool a cloud identity](../deploy-an-execution-agent-fleet/#give-a-pool-a-cloud-identity)),
and extra egress rules. A platform admin sets them through the
[pool API](../../../reference/http-api/#agent-pools) or
`cloacinactl tenant pool set`; see
[Size the agents: pools](../deploy-an-execution-agent-fleet/#size-the-agents-pools).
The actuator applies them on its next tick, and a changed setting rolls the
pool's Deployment. An upgrade to a version with pools does not roll the
existing Deployment: with no settings the manifest is unchanged.

### The exact RBAC the chart grants

`fleet.actuator=kubernetes` renders a `ServiceAccount`, a `ClusterRole`, and a
`ClusterRoleBinding` (a ClusterRole — not a namespaced Role — because creating
per-tenant namespaces is a cluster-scoped operation). There is **no**
cluster-admin and **no** wildcard verb or resource; every rule is enumerated:

| API group | Resource | Verbs | Why |
|---|---|---|---|
| (core) | `namespaces` | `create`, `patch` | Ensure each tenant's namespace via server-side apply (a single create-or-update call, authorized as `create`+`patch`). No `get`/`list`/`delete`. |
| `apps` | `deployments` | `create`, `delete`, `get`, `patch` | Ensure each pool's agent Deployment (server-side apply) and `patch` its replica count to scale; `get` reads the ready-replica count and the id of the agent key that the pod template names during reconcile. `delete` removes the Deployment of a pool that an admin removes, by name, after the server checked that the pool has no agents; a tenant's deprovision still scales to zero. No `list`. |
| (core) | `serviceaccounts` | `create`, `patch` | Ensure the ServiceAccount a pool names, with the pool's annotations (server-side apply). No `get`/`list`/`delete`: the ServiceAccount stays when its pool is removed. |
| (core) | `secrets` | `create`, `patch` | Ensure the per-tenant agent-key Secret (server-side apply) delivered to pods as `CLOACINA_API_KEY`. Addressed by its known name — no `get`/`list`/`update`/`delete`. |
| `networking.k8s.io` | `networkpolicies` | `create`, `delete`, `patch` | Ensure the per-tenant agent NetworkPolicy and each pool's own NetworkPolicy (server-side apply). `delete` removes the policy of a removed pool, by name. Granted whenever `fleet.actuator=kubernetes` so toggling `fleet.networkPolicy.enabled` later never needs an RBAC change. No `get`/`list`. |
| (core) | `persistentvolumeclaims` | `create`, `delete`, `get` | The cache claim `cloacina-compiler-cache` of a tenant's compiler: read to see whether it exists, made once, deleted with the tenant. No `patch`: a claim that exists is never changed. No `list`/`update`. |

The ClusterRole is cluster-wide, so the `serviceaccounts` rule lets the server
write a ServiceAccount's annotations in any namespace, as the `secrets` rule
lets it write Secrets. The actuator writes only in tenant namespaces. If you
upgrade the chart from a version before pools, the new verbs come with it; a
server that runs with the old ClusterRole fails the reconcile of a pool that
names a ServiceAccount, and cannot remove a pool.

### Hardened agent pods + NetworkPolicy

The agent pods the actuator creates are hardened to clear a PodSecurity
**`restricted`** cluster, and each tenant namespace is network-isolated by
default. This is **defense-in-depth** — the server-side ABAC (NFR-004) remains
the real tenant-isolation boundary; none of it weakens a server-side check.

- **`securityContext`.** Pods run non-root as uid/gid `10001` (the agent image's
  user) with `seccompProfile: RuntimeDefault`; containers drop all capabilities,
  forbid privilege escalation, and use a `readOnlyRootFilesystem`. The agent's
  writable paths (`$HOME` for the unpacked `workflow/`+`vendor/` tree and cdylib
  cache, plus `/tmp`) are backed by `emptyDir` volumes.
- **No service account token.** Pods set `automountServiceAccountToken: false`;
  the agent never calls the Kubernetes API.
- **Resources.** Requests/limits come from `fleet.agentResources`. The defaults
  account for the agent's embedded CPython interpreter (PyO3) — bump
  `fleet.agentResources.limits.memory` for heavy vendored dependencies.
- **No probes.** The agent is a WebSocket client with no health endpoint, so the
  pods carry no kubelet probe; the server tracks liveness via heartbeat/eviction.
- **NetworkPolicy** (`fleet.networkPolicy.enabled`, default on). Per tenant
  namespace: **deny all ingress**, and **allow egress only** to cluster DNS
  (UDP+TCP 53 in `fleet.networkPolicy.dnsNamespace`) and the `cloacina-server`
  pods on the server port — the single path agents need to register, heartbeat,
  and stream work. The actuator learns the server's coordinates from
  `CLOACINA_SERVER_NAMESPACE` (this release's namespace) and
  `CLOACINA_SERVER_POD_SELECTOR` (the server's pod labels), both rendered by the
  chart. Set `fleet.networkPolicy.enabled=false` to skip the policy (the RBAC
  verb stays granted, so you can re-enable without a redeploy of RBAC).
- **Extra egress per pool.** Each pool has its own NetworkPolicy,
  `cloacina-agent-pool-<pool>`, that selects the pool's pods and allows the
  CIDR blocks of the pool's `extra_egress` setting (for object storage, a
  catalog, a cloud API). With no rules it allows nothing more, so the default
  stays DNS and the server only. It is written only when the tenant policy is.

> **Restricted clusters.** Because the agent pods are already `restricted`-clean,
> you can safely label tenant namespaces `pod-security.kubernetes.io/enforce: restricted`.

### A compiler per tenant

With `fleet.actuator=kubernetes` the server runs one compiler for each
tenant, `public` included (`fleet.compiler.enabled`, default `true`): a
Deployment `cloacina-compiler` in the tenant's namespace
(`cloacina-tenant-public` for `public`) with `--tenant-schema <tenant>`. The
chart's own compiler (`compiler.enabled`) is then not rendered. It is rendered
only with the fleet compilers off, for `public`, with the chart's database URL:
the single-trust install.
The server creates it when the tenant's build queue first has a package, sets
it to 0 replicas when the queue has been empty for
`fleet.compiler.cooldownSeconds`, and deletes it when the tenant is removed. A
package published to a tenant created after install is built with no manual
step. A deployment with `T` tenants runs between 0 and `T` tenant compilers,
one for each tenant that has a package to build or built one in the last
cooldown.

```yaml
fleet:
  actuator: kubernetes
  compiler:
    enabled: true
    cooldownSeconds: 300
    # Where builds fetch crates from: internet | all | none
    egress: internet
```

| Value | Default | Purpose |
|---|---|---|
| `fleet.compiler.enabled` | `true` | Run a compiler per tenant. |
| `fleet.compiler.image.repository`, `.tag` | the image of `compiler.image` | The compiler image. |
| `fleet.compiler.cargoFlags` | `[build, --lib]` | The whole cargo flag list of a build. An empty list keeps the compiler's `build --release --lib --frozen --offline`. |
| `fleet.compiler.vendorDir` | `""` | A cargo home inside the image for offline builds (`--vendor-dir`). |
| `fleet.compiler.targetDir` | `/workspace/target` | The cargo target directory; an `emptyDir` is mounted there. |
| `fleet.compiler.buildTimeoutSeconds` | `0` (the compiler's 600) | Wall-clock limit of one build. |
| `fleet.compiler.cooldownSeconds` | `300` | How long a tenant's queue must be empty before its compiler is set to 0 replicas. |
| `fleet.compiler.cacheSize` | `8Gi` | Size limit of each cache volume (cargo home, target directory). |
| `fleet.compiler.hardened` | `true` | Not root, read-only root filesystem, no capabilities, writable paths on `emptyDir` volumes. |
| `fleet.compiler.resources` | `500m`/`1Gi` requests, `2`/`4Gi` limits | Requests and limits of the compiler container. |
| `fleet.compiler.databaseUrlSecretRef` | empty (the server's URL) | A Secret in the release namespace with a URL that says where the tenant compilers reach the database. Its user and password are not used: each compiler connects as its tenant's build role. |
| `fleet.compiler.egress` | `internet` | Crate egress of the compiler pods: `internet` (TCP 443 to public IPv4 addresses), `all`, or `none`. |
| `fleet.compiler.egressCidrs` | `[]` | More egress CIDR blocks, every port (a mirror, a git host, a proxy). |
| `fleet.compiler.databaseCidrs` | `[]` | CIDR blocks of the database; empty allows the database port to every destination. |
| `fleet.compiler.cacheClaim.enabled` | `true` | A PersistentVolumeClaim `cloacina-compiler-cache` per tenant compiler for its cargo home, target directory and `uv` cache: kept across scale to 0, deleted with the tenant. Hardened pods only. `false`: `emptyDir`s. |
| `fleet.compiler.cacheClaim.size` | `20Gi` | Requested size of the claim. |
| `fleet.compiler.cacheClaim.storageClass` | `""` (the cluster's default class) | Storage class of the claim. |
| `fleet.compiler.homeSize` | `4Gi` | Size limit of the home volume of a hardened pod, where the build folder is. |
| `fleet.compiler.python.platforms` | `[]` (the compiler's `aarch64-linux`, `x86_64-linux`) | Platforms to vendor Python dependencies for. |
| `fleet.compiler.python.version` | `""` (the compiler's 3.11) | The Python the wheels are for. |
| `fleet.compiler.python.indexUrl`, `.indexUrlSecretRef` | `""` (PyPI) | The package index; the server puts it in the Secret `cloacina-compiler-db`. |
| `fleet.compiler.python.wheelDir` | `""` | A folder of wheels inside the image. |
| `fleet.compiler.python.offline` | `false` | No network for Python builds; with `hardened` it needs `wheelDir`. |
| `fleet.compiler.python.uvCacheDir` | `""` (hardened: `/var/cloacina/uv-cache`) | The `uv` cache folder. |
| `fleet.compiler.python.cacheSize` | `4Gi` | Size limit of the `uv` cache volume of a hardened pod. |

See [Python in the compilers the server makes](../running-the-compiler/#python-in-the-compilers-the-server-makes)
for the image, the egress each mode needs, the space and the cost of a cold
start.

What the server writes in a tenant namespace for the compiler:

- the Deployment `cloacina-compiler`, whose pod template carries the
  annotation `cloacina.io/build-credential`;
- the Secret `cloacina-compiler-db`, with the URL of the tenant's build role
  (`<tenant>$build`) the pod reads as `DATABASE_URL`, and the Python index URL
  (key `python-index-url`) when one is set. The server sets a new password
  each time it starts the compiler, writes the Secret only then, and keeps no
  copy. The role reaches the four build tables of the tenant's schema and
  nothing else;
- the PersistentVolumeClaim `cloacina-compiler-cache` (with
  `fleet.compiler.cacheClaim.enabled` and `hardened`), made once with `create`
  after a `get`, and deleted with the tenant. This needs the RBAC rule on
  `persistentvolumeclaims` (`create`, `delete`, `get`) that the chart renders
  with the actuator;
- the NetworkPolicy `cloacina-compiler` (when `fleet.networkPolicy.enabled`):
  no ingress; egress to DNS, to the database port, and to where crates come
  from by `fleet.compiler.egress` and `fleet.compiler.egressCidrs`.

These need no RBAC verb beyond the table above. The server has no `delete` on
Secrets, so when a tenant is removed it writes an empty value over
`cloacina-compiler-db`. It does so with a patch, on each call of the tenant
delete: a Secret that exists is emptied whatever is left of the Deployment,
and no Secret is made for a tenant that has none.

The cache volumes are `emptyDir`s: each start of a tenant's compiler is a cold
start, and its first build downloads and compiles every dependency.

With the cache claim (the default) the cargo home, the target directory and
the `uv` cache are on the claim instead, and are kept when the compiler is
scaled to 0: only the first start of a tenant's compiler, or a start on a new
claim, is cold.

With `postgresql.enabled` the chart gives the tenant compilers the bundled
database by its full Service name. With `database.url` or
`databaseUrlSecretRef`, the host in the URL must resolve from the tenant
namespaces.

The details (the scale rules, the pod, the database credential, an install
with no route to a registry) are in
[Run cloacina-compiler in Production](../running-the-compiler/#how-many-compilers-a-deployment-runs).

### Fail-closed guard

The actuator validates its substrate at boot and **refuses to start** on a
mismatch — so a misconfiguration is a loud crash, never silent wrong-scaling. In
particular, `CLOACINA_FLEET_ACTUATOR=kubernetes` requires the server to be
running in-cluster (a service-account token mount); it errors out otherwise.
Because the chart only sets `kubernetes` when it also binds the ServiceAccount,
the in-cluster path is satisfied by construction. See
[Execution-Agent Fleet](../../../explanation/service/execution-agent-fleet/#pluggable-actuators--substrate-guard)
for the concept and
[Environment Variables](../../../reference/environment-variables/#fleet-actuator--autoscaler)
for the autoscaler tuning knobs.

## High availability (multi-replica)

The server is safe to run with `replicaCount > 1`, with **no session affinity
required**: the reconciler/autoscaler leader is gated by a Postgres advisory
lock (ADR CLOACI-A-0008); WebSocket auth tickets, the execution-agent fleet
roster, and OIDC login flows are all Postgres-backed, so any request or
heartbeat can land on any replica; and work packets ride the durable delivery
outbox to whichever replica holds the recipient's socket. Readiness (`/ready`)
is scoped to platform health (DB reachability) — a crashed computation graph
does not eject replicas from the load-balancer pool (it stays visible via
`GET /v1/health/graphs`). See
[Horizontal Scaling](../../../explanation/service/horizontal-scaling/)
for the two deliberate residuals (replica-local fleet secret key pools;
reactive-layer placement, CLOACI-T-0851). When you raise the
replica count the chart adds two HA primitives automatically:

| Value | Default | Effect |
|------|---------|--------|
| `podDisruptionBudget.enabled` | `true` | Render a PodDisruptionBudget — **only** when `replicaCount > 1` (a PDB over a single replica would wedge node drains). |
| `podDisruptionBudget.minAvailable` | `1` | Minimum server replicas kept available during voluntary disruptions (drains, rolling updates). |
| `affinity` | `{}` (→ default) | When unset, the chart applies a **soft** pod-anti-affinity that prefers spreading replicas across hostnames (`topologyKey: kubernetes.io/hostname`, preferred — never blocks scheduling). Set `affinity` to override. |

```sh
helm upgrade --install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 --reuse-values \
  --set replicaCount=3
```

> **No HorizontalPodAutoscaler.** The chart intentionally ships none. Server
> *fleet* scaling is driven by the control-plane back-pressure autoscaler, which
> sees per-tenant utilization an HPA's CPU/memory metrics cannot. Scale the
> server *replica* count by hand (or your own policy) for HTTP/HA headroom.

## Upgrades

```sh
helm upgrade cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.2.0 \
  --reuse-values
```

The deployment uses `RollingUpdate` with `maxSurge=1` /
`maxUnavailable=0`, so the existing pod stays serving until the new
one is healthy. Database migrations run on container start, for the
`public` schema and then for each tenant schema; the
startup probe gives the new pod up to 5 minutes (60 × 5s) to come up.
Pods that start together run the migrations once: they run under a
PostgreSQL advisory lock, and a pod that waited for the lock finds
nothing left to do. If a tenant schema cannot be migrated, the new pod
logs the tenant's name and exits, and the existing pod keeps serving.
A tenant created through the old pod while the new pod runs is migrated
by the new pod when it first opens that tenant.

## Uninstall

```sh
helm uninstall cloacina
```

If `postgresql.enabled=true`, the bundled Postgres PVC is preserved
by default. Delete it manually if you no longer need the data:

```sh
kubectl delete pvc -l app.kubernetes.io/instance=cloacina
```

## See also

- [Deploying the API Server](../deploying-the-api-server/) —
  CLI-level configuration reference (auth, signatures, multi-tenancy)
- [Running the cloacina-server Docker image](../running-the-server-image/) —
  the underlying image the chart deploys
- [Production Deployment](../production-deployment/) —
  scaling and operational hardening
