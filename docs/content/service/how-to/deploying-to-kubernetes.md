---
title: "Deploying to Kubernetes (Helm)"
description: "Install cloacina-server on Kubernetes using the official Helm chart"
weight: 57
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

This pulls in Bitnami's `postgresql` subchart. **Don't use this in
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
| `livenessProbe`, `readinessProbe`, `startupProbe` | `/v1/health` HTTP | Override per-probe; set to `null` to disable |
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
