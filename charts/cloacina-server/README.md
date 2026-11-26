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
| `server.tenantRunnerCacheSize` | `256` | LRU cap on per-tenant runners |
| `service.type` | `ClusterIP` | Service type |
| `ingress.enabled` | `false` | Provision an Ingress |
| `serviceMonitor.enabled` | `false` | Emit a Prometheus operator `ServiceMonitor` |
| `metrics.enabled` | `false` | Serve `/metrics` on its own port and Service; the API port answers 404 for it |
| `metrics.port` | `9090` | Port of the metrics listener |
| `metrics.networkPolicy.enabled` | `false` | NetworkPolicy for the server pods: API port open, metrics port open only to `from` |
| `metrics.networkPolicy.from` | `[]` | NetworkPolicy peers allowed to reach the metrics port |
| `retention.runHistoryDays` | `0` | Days to keep a finished run; `0` keeps run history for ever (`CLOACINA_RUN_HISTORY_RETENTION_DAYS`) |
| `retention.outboxAckedSeconds` | `3600` | Seconds to keep an acked `delivery_outbox` row; `0` keeps the rows for ever (`CLOACINA_OUTBOX_ACKED_RETENTION_S`) |
| `retention.intervalSeconds` | `300` | Seconds between passes of the retention job (`CLOACINA_RETENTION_INTERVAL_S`) |
| `fleetTaskTimeLimits.defaultSeconds` | `300` | Time limit of a fleet task that sets none, and of a graph firing on the fleet (`CLOACINA_FLEET_TASK_TIMEOUT_S`) |
| `fleetTaskTimeLimits.maxSeconds` | `86400` | Longest time limit a fleet task can get (`CLOACINA_FLEET_TASK_TIMEOUT_MAX_S`) |
| `alertWebhook.url` | `""` | URL that gets a JSON `POST` when a run ends; empty = off (`CLOACINA_ALERT_WEBHOOK_URL`) |
| `alertWebhook.urlSecretRef.name` / `.key` | `""` / `url` | Existing Secret that holds the URL; used in place of `alertWebhook.url` |
| `alertWebhook.events` | `[failed]` | Run outcomes that send an alert: `failed`, `completed`, `cancelled`, `all` (`CLOACINA_ALERT_WEBHOOK_EVENTS`) |
| `alertWebhook.timeoutSeconds` | `10` | Seconds allowed for one request (`CLOACINA_ALERT_WEBHOOK_TIMEOUT_S`) |
| `alertWebhook.maxAttempts` | `8` | The most sends of one alert (`CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS`) |
| `alertWebhook.publicBaseUrl` | `""` | Public URL of the server, for the links in an alert (`CLOACINA_PUBLIC_BASE_URL`) |
| `alertWebhook.secretRef.name` / `.key` | `""` / `secret` | Existing Secret that holds the signing secret (`CLOACINA_ALERT_WEBHOOK_SECRET`) |
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

Set `serviceMonitor.enabled=true` to register a `ServiceMonitor` for
the Prometheus operator. The chart scrapes `:8080/metrics` every 30s.

By default `/metrics` is on the API port with no authentication: every
client that can reach the API can read it, and the metric labels hold
names that tenants choose. To separate it:

```yaml
metrics:
  enabled: true          # /metrics moves to port 9090; :8080/metrics answers 404
  networkPolicy:
    enabled: true        # needs a CNI that enforces NetworkPolicy
    from:
      - namespaceSelector:
          matchLabels:
            kubernetes.io/metadata.name: monitoring
serviceMonitor:
  enabled: true          # now scrapes the Service <fullname>-metrics
```

With `metrics.enabled=true` the chart adds the container port `metrics`
and a ClusterIP Service `<fullname>-metrics`. The NetworkPolicy keeps the
API port open to every source, opens the metrics port only to the peers
in `from` (nobody when the list is empty), and refuses every other
ingress to the server pods.

## Retention, time limits and alerts

The chart sets these server settings as environment variables on the
server Deployment. Each default is the server's own, so an install with
no values behaves as the binary does.

```yaml
retention:
  runHistoryDays: 30        # 0 (the default) keeps run history for ever
  outboxAckedSeconds: 3600
  intervalSeconds: 300

fleetTaskTimeLimits:
  defaultSeconds: 300       # a task with no limit of its own; a graph firing
  maxSeconds: 86400

alertWebhook:
  url: https://alerts.example.com/hook
  events: [failed]
  publicBaseUrl: https://cloacina.example.com
  secretRef:                # the signing secret, from an existing Secret
    name: cloacina-alerts
    key: secret
```

The retention and time limit variables are always rendered. The alert
variables are rendered only when a URL is set, by `alertWebhook.url` or
by `alertWebhook.urlSecretRef`.

The signing secret has no plain value in the chart. Create the Secret and
name it in `alertWebhook.secretRef`:

```sh
kubectl create secret generic cloacina-alerts \
  --from-literal=secret="$(openssl rand -hex 32)"
```

A webhook URL that holds a token belongs in a Secret too. Put it under a
key of a Secret and set `alertWebhook.urlSecretRef.name` and `.key` in
place of `alertWebhook.url`.

`server.extraEnv` is rendered after these variables. An entry there with
the same name takes the place of the chart's value.

## Upgrades

```sh
helm upgrade cloacina oci://ghcr.io/colliery-io/charts/cloacina-server \
  --version 0.2.0 \
  --reuse-values
```

The deployment uses a `RollingUpdate` strategy with `maxSurge=1` /
`maxUnavailable=0`, so the old pod stays serving until the new one is
ready.

## Uninstall

```sh
helm uninstall cloacina
```

If `postgresql.enabled=true`, the bundled Postgres PVC is preserved by
default — delete it manually if you no longer need the data.

## See also

- [Deploying to Kubernetes](https://cloacina.dev/platform/how-to-guides/deploying-to-kubernetes)
- [Production Deployment](https://cloacina.dev/platform/how-to-guides/production-deployment)
