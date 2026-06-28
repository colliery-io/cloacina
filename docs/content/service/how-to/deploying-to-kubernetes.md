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

| Value | Default | Effect |
|------|---------|--------|
| `fleet.actuator` | `none` | `none` (off) or `kubernetes`. |
| `fleet.agentImage` | `ghcr.io/colliery-software/cloacina-agent:latest` | Agent image the actuator runs per tenant. |
| `fleet.agentServerUrl` | `""` (→ Service DNS) | URL injected into agents as `CLOACINA_SERVER`. |
| `fleet.serviceAccount.name` | `""` (→ `<fullname>-fleet`) | Override the fleet ServiceAccount name. |
| `fleet.serviceAccount.annotations` | `{}` | Annotations on that ServiceAccount (e.g. IRSA / Workload Identity). |

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
