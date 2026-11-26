# cloacina-server Helm chart

Official Helm chart for the [Cloacina](https://cloacina.dev) HTTP API
server. Pairs with the `ghcr.io/colliery-io/cloacina-server` image.

CLOACI-I-0111 / T-0605.

## Install

### From the OCI registry

```sh
helm install cloacina \
  oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.1.0 \
  --set database.url=postgres://user:pass@host:5432/cloacina
```

### From a local clone

```sh
helm install cloacina ./charts/cloacina-server \
  --set database.url=postgres://user:pass@host:5432/cloacina
```

### With bundled Postgres (demo / dev only)

```sh
helm dependency update ./charts/cloacina-server
helm install cloacina ./charts/cloacina-server \
  --set postgresql.enabled=true \
  --set postgresql.auth.password=$(openssl rand -hex 16)
```

The chart fails fast if you don't configure exactly one of:
- `database.url`
- `databaseUrlSecretRef.name`
- `postgresql.enabled=true`

## Common values

| Key | Default | Description |
|-----|---------|-------------|
| `image.repository` | `ghcr.io/colliery-io/cloacina-server` | Image to pull |
| `image.tag` | `Chart.AppVersion` | Image tag (override to pin a server release) |
| `replicaCount` | `1` | Pod replicas |
| `database.url` | `""` | Plaintext Postgres URL |
| `databaseUrlSecretRef.name` | `""` | Secret holding `DATABASE_URL` |
| `postgresql.enabled` | `false` | Install Bitnami's Postgres in-cluster |
| `apiKeySecretRef.name` | `""` | Secret for `CLOACINA_BOOTSTRAP_KEY` |
| `server.requireSignatures` | `false` | Reject unsigned packages |
| `server.tenantRunnerCacheSize` | `256` | Warning limit on per-tenant runners; none is stopped at the limit |
| `service.type` | `ClusterIP` | Service type |
| `ingress.enabled` | `false` | Provision an Ingress |
| `serviceMonitor.enabled` | `false` | Emit a Prometheus operator `ServiceMonitor` |
| `metrics.enabled` | `true` | Serve `/metrics` on its own port and Service; the API port answers 404 for it |
| `metrics.port` | `9090` | Port of the metrics listener |
| `metrics.networkPolicy.enabled` | `true` | NetworkPolicy for the server pods: API port open, metrics port open only to `from` |
| `metrics.networkPolicy.from` | `[]` | NetworkPolicy peers allowed to reach the metrics port; empty admits nobody |
| `livenessProbe`/`readinessProbe`/`startupProbe` | `/v1/health` HTTP probes | Override or disable per probe |

See [`values.yaml`](./values.yaml) for the full reference.

## Probes

All three Kubernetes probes hit `/v1/health` over HTTP. `startupProbe`
gives the server up to 5 minutes (60 × 5s) to come up — useful when the
DB migration takes a while on first install.

Disable any probe with `--set <probe>=null`.

## Security

The image runs as uid `10001` with `readOnlyRootFilesystem: true`.
Writable paths (`/home/cloacina`, `/tmp`) are backed by `emptyDir`
volumes. All Linux capabilities are dropped.

If you run with PodSecurity admission `restricted`, the defaults pass
without further changes.

## Observability

`/metrics` has no authentication, and its labels hold tenant names,
workflow names and other names that tenants choose. By default the chart
keeps it off the API port:

- the server starts with `--metrics-bind 0.0.0.0:9090`; `:8080/metrics`
  answers 404;
- the ClusterIP Service `<fullname>-metrics` exposes port 9090;
- a NetworkPolicy keeps the API port open to every source and opens the
  metrics port only to the peers in `metrics.networkPolicy.from`. The
  list is empty by default: nobody reaches the metrics port until you
  name your Prometheus. It refuses every other ingress to the server
  pods. It needs a CNI that enforces NetworkPolicy.

To scrape:

```yaml
metrics:
  networkPolicy:
    from:
      - namespaceSelector:
          matchLabels:
            kubernetes.io/metadata.name: monitoring
serviceMonitor:
  enabled: true          # scrapes the Service <fullname>-metrics every 30s
```

With `compiler.enabled=true` the compiler pods get the same:
`--metrics-bind`, the Service `<fullname>-compiler-metrics`, a
`ServiceMonitor` `<fullname>-compiler` and a NetworkPolicy.

`metrics.enabled=false` puts `/metrics` back on the API port, where
every client that can reach the API can read it, tenant agents included.
No metrics Service and no NetworkPolicy is rendered then.

## Upgrades

```sh
helm upgrade cloacina oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.2.0 \
  --reuse-values
```

The deployment uses a `RollingUpdate` strategy with `maxSurge=1` /
`maxUnavailable=0`, so the old pod stays serving until the new one is
ready.

### Upgrade note: `/metrics` moves off the API port

`metrics.enabled` and `metrics.networkPolicy.enabled` are now `true` by
default (CLOACINA-T-1032). After an upgrade with default values:

- `:8080/metrics` answers 404. Scrape `<fullname>-metrics:9090/metrics`.
- With `serviceMonitor.enabled=true` the `ServiceMonitor` moves to the
  metrics Service by itself. A scrape config written by hand must change
  its target.
- The NetworkPolicy admits nobody to port 9090 until
  `metrics.networkPolicy.from` lists the Prometheus pods.
- The NetworkPolicy also refuses ingress to the server pods on every
  port other than 8080 and the metrics port.
- The server image must have `--metrics-bind`: a release after 0.12.0.
  With an older image, set `metrics.enabled=false`.
- To keep the old behaviour, set `metrics.enabled=false`.

## Uninstall

```sh
helm uninstall cloacina
```

If `postgresql.enabled=true`, the bundled Postgres PVC is preserved by
default — delete it manually if you no longer need the data.

## See also

- [Deploying to Kubernetes](https://cloacina.dev/platform/how-to-guides/deploying-to-kubernetes)
- [Production Deployment](https://cloacina.dev/platform/how-to-guides/production-deployment)
