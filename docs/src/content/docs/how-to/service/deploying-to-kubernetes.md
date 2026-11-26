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
  --set database.url=postgres://cloacina:cloacina@postgres.svc:5432/cloacina
```

### With a `DATABASE_URL` Secret

```sh
kubectl create secret generic cloacina-db \
  --from-literal=DATABASE_URL=postgres://cloacina:s3cr3t@postgres.svc:5432/cloacina

helm install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --set databaseUrlSecretRef.name=cloacina-db
```

### Bundled Postgres (demo / dev)

```sh
helm install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --set postgresql.enabled=true \
  --set postgresql.auth.password=$(openssl rand -hex 16)
```

This enables the bundled **in-tree** `postgresql` subchart — a Service + PVC around the official `docker.io/library/postgres` image (it replaced the Bitnami chart). **Don't use this in
production** — the credentials are exposed via Helm values, and the
PVC isn't backed up by anything you don't run yourself.

The chart fails fast if you don't pick exactly one of these three paths.

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
| `server.verificationOrgId` | `""` | Trusted org UUID (required when signatures are required) |
| `server.tenantRunnerCacheSize` | `256` | Warning limit on per-tenant `DefaultRunner` instances; none is stopped at the limit |

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

## Fleet actuator (Kubernetes) + RBAC

The server can provision and autoscale a per-tenant pool of `cloacina-agent`
workloads itself, instead of you running agents by hand (the agent
self-management control plane, CLOACI-I-0127). On Kubernetes this is the
**Kubernetes fleet actuator**. It is **off by default** — existing installs are
unchanged: no ServiceAccount, no RBAC, and no actuator env are rendered unless
you opt in. (The Docker actuator is dev-only and is **not** offered through this
chart.)

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
| `fleet.agentImage` | `ghcr.io/colliery-software/cloacina-agent:latest` | Agent image the actuator runs per tenant. |
| `fleet.agentServerUrl` | `""` (→ Service DNS) | URL injected into agents as `CLOACINA_SERVER`. |
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
| `apps` | `deployments` | `create`, `delete`, `get`, `patch` | Ensure each pool's agent Deployment (server-side apply) and `patch` its replica count to scale; `get` reads the ready-replica count during reconcile. `delete` removes the Deployment of a pool that an admin removes, by name, after the server checked that the pool has no agents; a tenant's deprovision still scales to zero. No `list`. |
| (core) | `serviceaccounts` | `create`, `patch` | Ensure the ServiceAccount a pool names, with the pool's annotations (server-side apply). No `get`/`list`/`delete`: the ServiceAccount stays when its pool is removed. |
| (core) | `secrets` | `create`, `patch` | Ensure the per-tenant agent-key Secret (server-side apply) delivered to pods as `CLOACINA_API_KEY`. Addressed by its known name — no `get`/`list`/`update`/`delete`. |
| `networking.k8s.io` | `networkpolicies` | `create`, `delete`, `patch` | Ensure the per-tenant agent NetworkPolicy and each pool's own NetworkPolicy (server-side apply). `delete` removes the policy of a removed pool, by name. Granted whenever `fleet.actuator=kubernetes` so toggling `fleet.networkPolicy.enabled` later never needs an RBAC change. No `get`/`list`. |

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

The chart's own compiler (`compiler.enabled`) serves the `public` schema only.
With `fleet.actuator=kubernetes` the server also runs one compiler for each
other tenant (`fleet.compiler.enabled`, default `true`): a Deployment
`cloacina-compiler` in the tenant's namespace with `--tenant-schema <tenant>`.
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
| `fleet.compiler.databaseUrlSecretRef` | empty (the server's URL) | A Secret in the release namespace with the database URL of the tenant compilers. |
| `fleet.compiler.egress` | `internet` | Crate egress of the compiler pods: `internet` (TCP 443 to public IPv4 addresses), `all`, or `none`. |
| `fleet.compiler.egressCidrs` | `[]` | More egress CIDR blocks, every port (a mirror, a git host, a proxy). |
| `fleet.compiler.databaseCidrs` | `[]` | CIDR blocks of the database; empty allows the database port to every destination. |

What the server writes in a tenant namespace for the compiler:

- the Deployment `cloacina-compiler`;
- the Secret `cloacina-compiler-db`, with the database URL the pod reads as
  `DATABASE_URL`;
- the NetworkPolicy `cloacina-compiler` (when `fleet.networkPolicy.enabled`):
  no ingress; egress to DNS, to the database port, and to where crates come
  from by `fleet.compiler.egress` and `fleet.compiler.egressCidrs`.

These need no RBAC verb beyond the table above. The server has no `delete` on
Secrets, so when a tenant is removed it writes an empty value over
`cloacina-compiler-db`.

The cache volumes are `emptyDir`s: each start of a tenant's compiler is a cold
start, and its first build downloads and compiles every dependency.

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
one is healthy. Database migrations run on container start; the
startup probe gives the new pod up to 5 minutes (60 × 5s) to come up.

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
