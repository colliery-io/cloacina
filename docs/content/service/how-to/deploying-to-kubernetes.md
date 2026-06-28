---
title: "Deploying to Kubernetes (Helm)"
description: "Install cloacina-server on Kubernetes using the official Helm chart"
weight: 57
aliases:
  - "/platform/how-to-guides/deploying-to-kubernetes/"

---

# Deploying to Kubernetes (Helm)

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
| `server.tenantRunnerCacheSize` | `256` | LRU cap on per-tenant `DefaultRunner` instances |

See `values.yaml` for the full reference.

## Observability (Prometheus operator)

```sh
helm upgrade --install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --reuse-values \
  --set serviceMonitor.enabled=true
```

The chart emits a `ServiceMonitor` that scrapes `:8080/metrics` every
30 seconds — picked up automatically by a Prometheus operator that
watches your namespace.

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
(the tenant id is sanitized to a DNS-1123 label), and manages exactly one
`cloacina-agent` Deployment plus one agent-key Secret inside it. Scaling a
tenant's fleet patches that Deployment's `replicas`. Tenant isolation is the
namespace boundary — the actuator only ever touches the requesting tenant's
namespace.

### The exact RBAC the chart grants

`fleet.actuator=kubernetes` renders a `ServiceAccount`, a `ClusterRole`, and a
`ClusterRoleBinding` (a ClusterRole — not a namespaced Role — because creating
per-tenant namespaces is a cluster-scoped operation). There is **no**
cluster-admin and **no** wildcard verb or resource; every rule is enumerated:

| API group | Resource | Verbs | Why |
|---|---|---|---|
| (core) | `namespaces` | `create`, `patch` | Ensure each tenant's namespace via server-side apply (a single create-or-update call, authorized as `create`+`patch`). No `get`/`list`/`delete`. |
| `apps` | `deployments` | `create`, `get`, `patch` | Ensure the per-tenant agent Deployment (server-side apply) and `patch` its replica count to scale; `get` reads the ready-replica count during reconcile. No `list`/`delete` — scale-to-zero replaces deletion. |
| (core) | `secrets` | `create`, `patch` | Ensure the per-tenant agent-key Secret (server-side apply) delivered to pods as `CLOACINA_API_KEY`. Addressed by its known name — no `get`/`list`/`update`/`delete`. |
| `networking.k8s.io` | `networkpolicies` | `create`, `patch` | Ensure the per-tenant agent NetworkPolicy (server-side apply). Granted whenever `fleet.actuator=kubernetes` so toggling `fleet.networkPolicy.enabled` later never needs an RBAC change. No `get`/`list`/`delete`. |

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

> **Restricted clusters.** Because the agent pods are already `restricted`-clean,
> you can safely label tenant namespaces `pod-security.kubernetes.io/enforce: restricted`.

### Fail-closed guard

The actuator validates its substrate at boot and **refuses to start** on a
mismatch — so a misconfiguration is a loud crash, never silent wrong-scaling. In
particular, `CLOACINA_FLEET_ACTUATOR=kubernetes` requires the server to be
running in-cluster (a service-account token mount); it errors out otherwise.
Because the chart only sets `kubernetes` when it also binds the ServiceAccount,
the in-cluster path is satisfied by construction. See
[Execution-Agent Fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}#pluggable-actuators--substrate-guard)
for the concept and
[Environment Variables]({{< ref "/reference/environment-variables" >}}#fleet-actuator--autoscaler)
for the autoscaler tuning knobs.

## High availability (multi-replica)

The server is safe to run with `replicaCount > 1`: the reconciler/autoscaler
leader is gated by a Postgres advisory lock (ADR CLOACI-A-0008), so extra
replicas serve HTTP without double-driving the control loop. When you raise the
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

- [Deploying the API Server]({{< ref "deploying-the-api-server" >}}) —
  CLI-level configuration reference (auth, signatures, multi-tenancy)
- [Running the cloacina-server Docker image]({{< ref "running-the-server-image" >}}) —
  the underlying image the chart deploys
- [Production Deployment]({{< ref "production-deployment" >}}) —
  scaling and operational hardening
