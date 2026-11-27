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
  --set database.url=postgres://user:pass@host:5432/cloacina \
  --set fleet.actuator=kubernetes
```

### From a local clone

```sh
helm install cloacina ./charts/cloacina-server \
  --set database.url=postgres://user:pass@host:5432/cloacina \
  --set fleet.actuator=kubernetes
```

### With bundled Postgres (demo / dev only)

```sh
helm dependency update ./charts/cloacina-server
helm install cloacina ./charts/cloacina-server \
  --set postgresql.enabled=true \
  --set postgresql.auth.password=$(openssl rand -hex 16) \
  --set fleet.actuator=kubernetes
```

The chart fails fast if you don't configure exactly one of:
- `database.url`
- `databaseUrlSecretRef.name`
- `postgresql.enabled=true`

It also fails unless the install says where tenant tasks run (CLOACINA-T-1075):
- `fleet.actuator=kubernetes`: the server makes each tenant's agents;
- `fleet.externalAgents=true`: you run the agents, for example with the
  `cloacina-agent` chart;
- `server.singleTrust=true`: tenant code runs in the server process.

## Where tenant tasks run

By default the server runs no tenant code in its own process. It loads each
package from the metadata document its compiler stored, opens no tenant
library and imports no tenant Python module, and sends every task to an agent
of the tenant. A task waits until such an agent can take it. A package that
uses a poll trigger, a native provider or Python reactive code does not load;
its `build_error` names what it uses.

`server.singleTrust=true` (`--single-trust`) is for a server whose every
tenant is trusted as much as the server. The server then loads packages and
runs tasks in its own process, where each tenant's code reaches the database,
the secrets key and the other tenants' data. The server logs a warning at
start.

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
| `server.singleTrust` | `false` | Tenant code runs in the server process: packages load in process and tasks may run there (`--single-trust`) |
| `fleet.actuator` | `none` | `kubernetes`: the server makes each tenant's agents |
| `fleet.externalAgents` | `false` | `true`: you run the agents; with `fleet.actuator=none` the server makes none |
| `server.tenantRunnerCacheSize` | `256` | Warning limit on per-tenant runners; none is stopped at the limit |
| `context.warnBytes` | `1048576` | Size in bytes above which a task context is logged as a warning and counted (`0` = off) |
| `context.maxBytes` | `16777216` | Largest size in bytes of a task context; a task with a larger one fails and is not retried (`0` = off) |
| `server.logFormat` | `text` | Format of the server's log lines on stderr: `text` or `json` (`--log-format`) |
| `server.taskLogUrlTemplate` | `""` | URL of a task's logs in your log stack, with `{tenant}`, `{execution_id}`, `{task}`, `{attempt}`; the UI shows it as a link on each task; empty = no link (`--task-log-url-template`) |
| `service.type` | `ClusterIP` | Service type |
| `ingress.enabled` | `false` | Provision an Ingress |
| `serviceMonitor.enabled` | `false` | Emit a Prometheus operator `ServiceMonitor` |
| `metrics.enabled` | `true` | Serve `/metrics` on its own port and Service; the API port answers 404 for it |
| `metrics.port` | `9090` | Port of the metrics listener |
| `metrics.networkPolicy.enabled` | `true` | NetworkPolicy for the server pods: API port open, metrics port open only to `from` |
| `metrics.networkPolicy.from` | `[]` | NetworkPolicy peers allowed to reach the metrics port; empty admits nobody |
| `server.databaseWaitTimeoutSeconds` | `60` | Seconds the server waits at start for the database, with retries; `0` = one try (`CLOACINA_DATABASE_WAIT_TIMEOUT_S`) |
| `retention.runHistoryDays` | `0` | Days to keep a finished run; `0` keeps run history for ever (`CLOACINA_RUN_HISTORY_RETENTION_DAYS`) |
| `retention.outboxAckedSeconds` | `0` | Seconds to keep an acked `delivery_outbox` row; `0` keeps the rows for ever (`CLOACINA_OUTBOX_ACKED_RETENTION_S`) |
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

## Retention, time limits and alerts

The chart sets these server settings as environment variables on the
server Deployment. Each default is the server's own, so an install with
no values behaves as the binary does.

```yaml
retention:
  runHistoryDays: 30        # 0 (the default) keeps run history for ever
  outboxAckedSeconds: 3600  # 0 (the default) keeps acked rows for ever
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

### Upgrade note: tenant tasks need agents, or single trust

Before `server.singleTrust` a default install ran every tenant's tasks in the
server process. After an upgrade with the old values:

- The chart fails at render until `fleet.actuator=kubernetes`,
  `fleet.externalAgents=true` or `server.singleTrust=true` is set.
- To keep running tasks in the server, set `server.singleTrust=true`.
- Without single trust, the server loads a package only once its compiler
  has stored the package's metadata document; upgrade the compiler with the
  server. A package with a poll trigger, a native provider or Python reactive
  code is refused at its first load (`build_status` `failed`), and stays
  refused if the setting is turned on later: set `server.singleTrust=true`
  before the upgrade to keep such packages in service.
- `CLOACINA_DEFAULT_EXECUTOR=default` in `server.extraEnv` stops the server
  at start without `server.singleTrust=true`.

## Uninstall

```sh
helm uninstall cloacina
```

If `postgresql.enabled=true`, the bundled Postgres PVC is preserved by
default — delete it manually if you no longer need the data.

## See also

- [Deploying to Kubernetes](https://cloacina.dev/platform/how-to-guides/deploying-to-kubernetes)
- [Production Deployment](https://cloacina.dev/platform/how-to-guides/production-deployment)
