---
title: "Environment Variables"
description: "Complete reference for all environment variables used by Cloacina"
sidebar:
  order: 25
---

This page documents all environment variables recognized by Cloacina components. Variables are grouped by functional area.

---

## Database

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `DATABASE_URL` | Database connection URL (PostgreSQL or SQLite) for the server, compiler, and admin commands. Can also be set via `--database-url` CLI flag or `database_url` in `config.toml`. | None | `postgresql://cloacina:cloacina@localhost:5432/cloacina` | Server, Compiler, Admin CLI | Yes (for `server start`, `compiler start`, and `admin` commands) |

### Resolution Order

The database URL is resolved with the following precedence (highest wins):

1. `--database-url` CLI argument
2. `DATABASE_URL` environment variable
3. `database_url` key in `~/.cloacina/config.toml`

If none of these are set, the command exits with an error message listing all three options.

### Notes

- Scheme selects the backend: `postgresql://` / `postgres://` for PostgreSQL; `sqlite://`, a bare file path, or `:memory:` for SQLite.
- The daemon does **not** use `DATABASE_URL`; it uses an embedded SQLite database at `~/.cloacina/`.
- For multi-tenant deployments, the URL points to a shared PostgreSQL instance and schema isolation is configured separately via the API.

---

## Server

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_BOOTSTRAP_KEY` | Pre-defined API key used as the initial admin key on first server startup. If set and no API keys exist in the database, this value is registered as the admin key instead of auto-generating one. | None (auto-generated) | `sk-my-secret-bootstrap-key-abc123` | Server | No |
| `CLOACINA_REQUIRE_SIGNATURES` | When set (any value), the server enforces package signature verification at upload time. The signer must be a trusted key of the tenant, or of `CLOACINA_VERIFICATION_ORG_ID` if it is set. | `false` (off) | `true` | Server | No |
| `CLOACINA_VERIFICATION_ORG_ID` | Optional server-wide trusted organization UUID. A key trusted in it is accepted for every tenant, after the tenant's own keys. With none, each tenant trusts only its own keys. | None | `12345678-1234-1234-1234-123456789abc` | Server | No |
| `CLOACINA_SECRET_KEK` | Server key-encryption key (KEK) for the [tenant secrets](../../explanation/service/secrets/) store. Base64 or hex encoding of **exactly 32 bytes** (AES-256). It wraps each tenant's data key (envelope encryption at rest); keep it stable and backed up — losing it makes existing secrets unrecoverable, changing it invalidates them. When unset or malformed, secret routes return `503` and workflow secret resolution fails closed. Generate with `openssl rand -base64 32` or `openssl rand -hex 32`. | None (secrets disabled) | `openssl rand -base64 32` output | Server | No |
| `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | Warning limit on per-tenant `DefaultRunner` instances. Every tenant has a runner; none is stopped at the limit. Above it the server logs a warning. | `256` | `1024` | Server | No |
| `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | Max seconds to wait for in-flight workflows to drain during tenant teardown (step 2 of the 4-step orchestration). Past this, the runner is hard-evicted; tasks ignoring cooperative cancellation will error on next DB write. | `30` | `60` | Server | No |
| `CLOACINA_CORS_ALLOWED_ORIGINS` | Comma-separated CORS allowed origins. **CORS is disabled by default** — set this to opt in (REQ-009). Use `*` to allow any origin. Needed when a browser app (e.g. the web UI on a different origin) calls the API. | None (CORS off) | `https://ui.internal.example.com,https://app.example.com` | Server | No |
| `CLOACINA_CORS_ALLOWED_METHODS` | Comma-separated CORS allowed methods. Only applies once origins are set. | `GET,POST,DELETE,OPTIONS` | `GET,POST` | Server | No |
| `CLOACINA_CORS_ALLOWED_HEADERS` | Comma-separated CORS allowed request headers. Only applies once origins are set. | `authorization,content-type` | `authorization,content-type,x-tenant` | Server | No |

### Server CLI Flags (also accept env vars)

These are specified via `clap`'s `env = "..."` attribute and can be set as environment variables:

| Variable | CLI Flag | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | `--database-url` | None | Database connection URL |
| `CLOACINA_BOOTSTRAP_KEY` | `--bootstrap-key` | None | Bootstrap admin API key |
| `CLOACINA_METRICS_BIND` | `--metrics-bind` | None | Address of a separate listener for `/metrics`. When set, the main listener answers 404 for `/metrics` |
| `CLOACINA_REQUIRE_SIGNATURES` | `--require-signatures` | off | Signature enforcement toggle |
| `CLOACINA_ALERT_WEBHOOK_URL` | `--alert-webhook-url` | None | URL that gets a JSON `POST` when a run ends. Not set = off. Never logged. See [Alert on a Failed Run](../../how-to/service/alert-on-a-failed-run/) |
| `CLOACINA_ALERT_WEBHOOK_EVENTS` | `--alert-webhook-events` | `failed` | Outcomes that send an alert, comma-separated: `failed`, `completed`, `cancelled`, or `all` |
| `CLOACINA_ALERT_WEBHOOK_SECRET` | — (environment only) | None | Shared secret that signs each alert request (`X-Cloacina-Signature`, HMAC-SHA256). Never logged |
| `CLOACINA_ALERT_WEBHOOK_TIMEOUT_S` | `--alert-webhook-timeout-s` | `10` | Seconds allowed for one alert request |
| `CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS` | `--alert-webhook-max-attempts` | `8` | The most sends of one alert before it is dropped |
| `CLOACINA_PUBLIC_BASE_URL` | `--public-base-url` | None | Public URL of the server, for the absolute links in an alert |
| `CLOACINA_MAX_PACKAGE_BYTES` | `--max-package-bytes` | `104857600` | The largest package archive, in bytes, that an upload accepts (100 MiB). A larger upload is refused with status 413 and an error that names the size and the limit |
| `CLOACINA_MAX_UNPACKED_BYTES` | `--max-unpacked-bytes` | `524288000` | The largest size, in bytes, that a package archive may unpack to (500 MiB). A larger archive is refused at upload and at load. Also read by `cloacina-compiler`, `cloacina-agent`, `cloacinactl daemon` and `cloacinactl package validate`, `pack` and `publish`: give each the same value |
| `CLOACINA_LOG_FORMAT` | `--log-format` | `text` | Format of the server's log lines on stderr: `text` or `json` (one JSON object per line). The log file under `--home` is JSON in both |
| `CLOACINA_TASK_LOG_URL_TEMPLATE` | `--task-log-url-template` | None | URL of a task's logs in your log stack, with the placeholders `{tenant}`, `{execution_id}`, `{task}` and `{attempt}`. The task API gives it as `log_url` and the web UI shows a link. Not set = no link. Also taken by `cloacinactl server start`. See [Find a Task's Logs](../../how-to/service/find-a-tasks-logs/) |
| `CLOACINA_DATABASE_WAIT_TIMEOUT_S` | `--database-wait-timeout-s` | `60` | Seconds the server waits at start for the database to accept a connection. It tries again with a growing delay (0.5 s, doubling, at most 5 s), then exits with an error. `0` = one try, no wait |
| `CLOACINA_VERIFICATION_ORG_ID` | `--verification-org-id` | None | Server-wide trusted org UUID (optional) |
| `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | `--tenant-runner-cache-size` | `256` | Per-tenant runner warning limit |
| `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | `--tenant-deletion-drain-timeout-s` | `30` | Drain timeout during teardown |
| `CLOACINA_DEFAULT_EXECUTOR` | `--default-executor` | `default` | Executor every task is dispatched to. `default` runs all work on the in-process thread executor; `fleet` sends it to the [execution-agent fleet](../../explanation/service/execution-agent-fleet/). Hard-matched against registered executors at startup. Preferred surface is `[server].default_executor` in `config.toml`, which `cloacinactl server start` forwards. |
| `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `--agent-heartbeat-interval-s` | `15` | Heartbeat interval (seconds) the server advertises to fleet agents and uses as its liveness-sweep cadence. Lower = faster dead-agent detection + in-flight reclaim, at the cost of more heartbeat traffic. |
| `CLOACINA_AGENT_LIVENESS_MISSES` | `--agent-liveness-misses` | `3` | Consecutive missed heartbeats before the server marks a fleet agent dead and reclaims its in-flight work. Effective dead-after = interval × misses (default 15s × 3 = 45s). |
| `CLOACINA_RUN_HISTORY_RETENTION_DAYS` | `--run-history-retention-days` | `0` (off) | Days to keep a finished workflow run (Completed, Failed or Cancelled) before the server's retention job deletes it with its task executions, task metadata, contexts, events, recovery events and schedule executions. `0` turns it off. Runs that are not finished are never deleted. |
| `CLOACINA_OUTBOX_ACKED_RETENTION_S` | `--outbox-acked-retention-s` | `0` (off) | Seconds to keep a `delivery_outbox` row after its ack. `0` keeps acked rows for ever: `delivery_outbox` grows until a period is set. |
| `CLOACINA_RETENTION_INTERVAL_S` | `--retention-interval-s` | `300` | Seconds between passes of the retention job. |
| `CLOACINA_FLEET_TASK_TIMEOUT_S` | `--fleet-task-timeout-s` | `300` | Time limit (seconds) of a fleet task that sets none of its own (`#[task(timeout_seconds = N)]`, `@task(timeout_seconds=N)`). The agent enforces it; the server waits for the result that long plus 30 seconds. A graph firing sent to an agent gets this limit too. |
| `CLOACINA_FLEET_TASK_TIMEOUT_MAX_S` | `--fleet-task-timeout-max-s` | `86400` | Longest time limit (seconds) a fleet task can get. A task that asks for more gets this value, with a warning in the log. A default above the maximum stops the server at start. |
| `CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S` | `--fleet-artifact-fetch-wait-s` | `300` | How long (seconds) a fleet task waits, per attempt, while agents cannot fetch its package artifact. The time counts from the first refused fetch of the attempt. The task is dispatched again after a delay of 2 s that grows to 30 s, and uses no retry attempt. After the limit the attempt fails with an error that names the artifact fetch, and the task's retry policy decides what follows. `0`: the first failed fetch fails the attempt. |
| `CLOACINA_CONTEXT_WARN_BYTES` | `--context-warn-bytes` | `1048576` (1 MiB) | Size in bytes above which a task context (the one a task returns, or the one built for a task) is logged as a warning with the task, the size and the largest keys, and counted in `cloacina_context_size_warnings_total`. `0` turns the warning off. |
| `CLOACINA_CONTEXT_MAX_BYTES` | `--context-max-bytes` | `16777216` (16 MiB) | Largest size in bytes of a task context. A task whose result or input context is larger fails with the size, the limit and the largest keys in the error, and is not retried. The context is not saved. One setting for the whole server. `0` turns the limit off. See [Context size limits](../../explanation/engine/context-management/#context-size-limits). |
| `CLOACINA_CORS_ALLOWED_ORIGINS` | `--cors-allowed-origins` | None (CORS off) | Comma-separated allowed origins; CORS is off until set (REQ-009). `*` allows any. |
| `CLOACINA_CORS_ALLOWED_METHODS` | `--cors-allowed-methods` | `GET,POST,DELETE,OPTIONS` | Comma-separated allowed methods. |
| `CLOACINA_CORS_ALLOWED_HEADERS` | `--cors-allowed-headers` | `authorization,content-type` | Comma-separated allowed request headers. |

`CLOACINA_DEFAULT_EXECUTOR` / `--default-executor` is forwarded by the
`cloacinactl server start` wrapper (preferably set via
`[server].default_executor` in `config.toml`). The wrapper also takes
`CLOACINA_DATABASE_WAIT_TIMEOUT_S`, the
three retention settings, the two fleet time limits, the alert webhook
settings (but not the secret) and `CLOACINA_PUBLIC_BASE_URL`: as a flag of
the same name, as the env var above, or as a key of `[server]` in
`config.toml` (see [Configuration](../configuration/#server-section)).
`CLOACINA_ALERT_WEBHOOK_SECRET` is an env var only; the server that the
wrapper starts inherits it. The two `CLOACINA_AGENT_*` liveness flags are
read by the `cloacina-server` binary directly (and via the env vars above);
the wrapper does **not** forward them, so set them on `cloacina-server`
itself or through the environment.

The Helm chart `cloacina-server` sets these variables from its values:

| Variable | Chart value | Chart default |
|----------|-------------|---------------|
| `CLOACINA_DATABASE_WAIT_TIMEOUT_S` | `server.databaseWaitTimeoutSeconds` | `60` |
| `CLOACINA_RUN_HISTORY_RETENTION_DAYS` | `retention.runHistoryDays` | `0` |
| `CLOACINA_OUTBOX_ACKED_RETENTION_S` | `retention.outboxAckedSeconds` | `0` |
| `CLOACINA_RETENTION_INTERVAL_S` | `retention.intervalSeconds` | `300` |
| `CLOACINA_FLEET_TASK_TIMEOUT_S` | `fleetTaskTimeLimits.defaultSeconds` | `300` |
| `CLOACINA_FLEET_TASK_TIMEOUT_MAX_S` | `fleetTaskTimeLimits.maxSeconds` | `86400` |
| `CLOACINA_ALERT_WEBHOOK_URL` | `alertWebhook.url`, or `alertWebhook.urlSecretRef` (an existing Secret) | not set |
| `CLOACINA_ALERT_WEBHOOK_EVENTS` | `alertWebhook.events` | `[failed]` |
| `CLOACINA_ALERT_WEBHOOK_TIMEOUT_S` | `alertWebhook.timeoutSeconds` | `10` |
| `CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS` | `alertWebhook.maxAttempts` | `8` |
| `CLOACINA_PUBLIC_BASE_URL` | `alertWebhook.publicBaseUrl` | not set |
| `CLOACINA_ALERT_WEBHOOK_SECRET` | `alertWebhook.secretRef` (an existing Secret) | not set |

The alert variables are rendered only when a URL is set.

The bind address (`--bind`, default `127.0.0.1:8080`), `--reconcile-interval-s`, and `--log-retention-days` are CLI-only and do not have environment variable equivalents.

---

## Authentication (OIDC / SSO)

The server can federate login to an external OpenID Connect identity provider (single sign-on). OIDC is **opt-in**: if `CLOACINA_OIDC_ISSUER`, `CLOACINA_OIDC_CLIENT_ID`, and `CLOACINA_OIDC_REDIRECT_URI` are not all set, the OIDC login routes are simply not mounted and only local accounts / API keys are available. A validated OIDC identity is mapped to a cloacina principal by the god-owned allowlist in `CLOACINA_OIDC_MAP`; an identity matching no rule is **denied** (there is no implicit access). See [Configure OIDC / SSO login](../../how-to/service/configure-oidc-sso/) for the full setup.

| Variable | Purpose | Default | Component | Required |
|----------|---------|---------|-----------|----------|
| `CLOACINA_OIDC_ISSUER` | OIDC issuer URL used for discovery + JWKS. Presence of this (with client id + redirect) is what mounts the OIDC login routes. | None (OIDC off) | Server | Conditional (all three of issuer/client-id/redirect to enable) |
| `CLOACINA_OIDC_CLIENT_ID` | Relying-party client ID registered with the IdP. | None | Server | Conditional |
| `CLOACINA_OIDC_CLIENT_SECRET` | Relying-party client secret. Empty when the IdP treats the client as public (PKCE). | Empty | Server | No |
| `CLOACINA_OIDC_REDIRECT_URI` | Callback URL the IdP redirects to after login. Must match the server's `/v1/auth/callback` route as registered with the IdP. | None | Server | Conditional |
| `CLOACINA_OIDC_SCOPES` | Comma-separated scopes requested at login. | `openid,email,profile,groups` | Server | No |
| `CLOACINA_OIDC_MAP` | God-owned allowlist mapping IdP claims to `{tenant, role}`. `;`-separated rules `<match>=<tenant>:<role>`, where `<match>` is `group:NAME` / `domain:NAME` / `sub:NAME` and `<tenant>` may be `_` for a global principal. A `domain:` rule matches only an identity whose ID token carries `email_verified: true`. First matching rule wins; an unmatched identity is denied. Example: `group:acme-admins=acme:admin;domain:acme.com=acme:write`. | Empty (all identities denied) | Server | No (but empty = no OIDC access granted) |
| `CLOACINA_OIDC_SUCCESS_REDIRECT` | When set, the browser login flow redirects here on success, handing the minted membership set to the SPA via the URL fragment. Unset = the callback returns the memberships as JSON. | None (JSON response) | Server | No |

---

## Execution Agent

The [execution-agent fleet](../../explanation/service/execution-agent-fleet/) (`cloacina-agent`) is a DB-less worker that registers with a server, fetches compiled workflow artifacts, executes tasks, and reports results. It holds **no** database connection; all of its configuration is server + API-key oriented.

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_SERVER` | Base URL of the `cloacina-server` the agent registers with (used for both REST and the WebSocket ticket mint). Equivalent to `--server`. | None | `http://cloacina-server:8080` | Agent | Yes |
| `CLOACINA_API_KEY` | API key the agent authenticates with. Its tenant scope determines which tenants' work the agent may receive (REQ-008 tenant isolation). Equivalent to `--api-key`. The agent removes it from its environment at start. | None | `sk-...` | Agent | Yes, or `CLOACINA_API_KEY_FILE` |
| `CLOACINA_API_KEY_FILE` | Path of a file that holds the agent's API key (surrounding whitespace is ignored). Use it in place of `CLOACINA_API_KEY`; setting both is an error. The agent does not delete the file. Equivalent to `--api-key-file`. | None | `/run/secrets/cloacina/api-key` | Agent | No |
| `CLOACINA_AGENT_CACHE_DIR` | Directory used to cache fetched workflow cdylibs by digest, so a cache hit skips the REST fetch. Equivalent to `--cache-dir`. | `<TMPDIR>/cloacina-agent-cache` | `/var/lib/cloacina-agent/cache` | Agent | No |
| `CLOACINA_MAX_UNPACKED_BYTES` | The largest size, in bytes, that a package or provider archive may unpack to. The agent refuses to load a larger one. Equivalent to `--max-unpacked-bytes`. Use the value of the server. The fleet actuator sets it to the server's value when that is not the default. | `524288000` | `629145600` | Agent | No |
| `CLOACINA_AGENT_MAX_CONCURRENCY` | How many work packets the agent runs at the same time. Equivalent to `--max-concurrency`. The fleet actuator sets it from the pool's `max_concurrency` setting. | `4` | `1` | Agent | No |
| `CLOACINA_AGENT_POOL` | The [agent pool](../../explanation/service/execution-agent-fleet/#agent-pools) the agent is in, registered as the capability `pool:<name>`. Equivalent to `--pool`. The fleet actuator sets it for a pool other than `default`. Unset: the server counts the agent in `default`. | None | `big` | Agent | No |
| `CLOACINA_AGENT_CPU` | Total CPU the agent offers to tasks, a Kubernetes quantity above zero. Equivalent to `--cpu`. Reported at registration; the server places tasks by their CPU requests against it. The Kubernetes actuator sets it from the pool's `cpu_limit`, or its `cpu_request` when it sets no limit; the Docker actuator from the container's CPU limit. Unset: the agent reports none and is packed by its slots only. | None | `4` | Agent | No |
| `CLOACINA_AGENT_MEMORY` | Total memory the agent offers to tasks, a Kubernetes quantity above zero. Equivalent to `--memory`. The actuators set it as they set `CLOACINA_AGENT_CPU`, from the pool's memory limit or request. | None | `32Gi` | Agent | No |
| `CLOACINA_AGENT_LOG_FORMAT` | Format of the agent's log lines on stdout: `text` or `json` (one JSON object per line). Equivalent to `--log-format`. The fleet actuator does not set it. See [Find a Task's Logs](../../how-to/service/find-a-tasks-logs/). | `text` | `json` | Agent | No |

The remaining agent options — `--agent-id`, `--capabilities`, and `--target-triple-override` — are CLI-only; see the [CLI Reference](../cli/#agent).

---

## Fleet actuator & autoscaler

These server-side variables drive the **agent self-management control plane**: the server can hold a per-tenant agent-capacity limit, provision a per-tenant pool of `cloacina-agent` workloads on a pluggable substrate (the *actuator*), and autoscale that pool from observed utilization. The actuator and autoscaler only run when an actuator is selected — with `CLOACINA_FLEET_ACTUATOR=none` (the default) no pool is provisioned and the control loop does not start. A provisioned pool only does useful work when the server's [default executor](../../explanation/service/execution-agent-fleet/) is `fleet` (otherwise tasks run in-process and the agents sit idle).

> **Fail-closed substrate guard.** `CLOACINA_FLEET_ACTUATOR` is validated against the detected host at boot. `docker` **refuses to start** when Kubernetes is detected (service-account token mount or `KUBERNETES_SERVICE_HOST`) or when no Docker socket is reachable; `kubernetes` refuses to start when the server is not running in-cluster. A misconfigured actuator is a fatal boot error, never a silent wrong-scaling.

### Capacity limits & provisioning

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_DEFAULT_MAX_AGENTS` | Platform-wide default ceiling on a tenant's agent count. The per-tenant *effective limit* is this value unless an admin sets a per-tenant override via `POST /v1/tenants/{id}/limits`. Provisioning and the autoscaler both clamp to the effective limit. | `4` | `8` | Server | No |
| `CLOACINA_INITIAL_AGENTS` | Agent(s) auto-provisioned (as `desired_count`) when a tenant is created, clamped to `min(initial_agents, default_max_agents)`. `0` disables auto-provision. Best-effort: a failure logs a warning but still returns tenant-created success. | `1` | `2` | Server | No |

### Actuator selection

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_FLEET_ACTUATOR` | Substrate the fleet actuator reconciles each tenant's running agent count on: `none` (actuation off), `docker` (dev-only — spawns labelled `cloacina-agent` containers), or `kubernetes` (scales a per-tenant `cloacina-agent` Deployment in the tenant's own namespace). Validated fail-closed against the host at boot. | `none` | `kubernetes` | Server | No |
| `CLOACINA_AGENT_IMAGE` | Agent image the actuator runs for each tenant. | `cloacina-agent:latest` | `ghcr.io/colliery-io/cloacina-agent:latest` | Server (actuator) | No |
| `CLOACINA_AGENT_SERVER_URL` | Server URL injected into each spawned agent as `CLOACINA_SERVER` (the agent's registration target). | `http://server:8080` | `http://cloacina-server:8080` | Server (actuator) | No |
| `CLOACINA_AGENT_NETWORK` | **Docker actuator only.** Docker network attached to each spawned agent container so it can reach the server (e.g. the compose network). Unset = the daemon default. Ignored by the Kubernetes actuator (in-cluster pods reach the server by Service DNS). | None | `cloacina_net` | Server (docker actuator) | No |

#### Kubernetes agent-pod hardening

These are read by the **Kubernetes actuator** and applied to every `cloacina-agent` pod it creates. They are the server-wide defaults: an [agent pool](../../explanation/service/execution-agent-fleet/#agent-pools)'s settings override the image and the four resource values for that pool. The chart renders them from `fleet.agentResources` / `fleet.networkPolicy` when `fleet.actuator=kubernetes`; defaults are sane if unset. Agent pods are created non-root (uid/gid `10001`) with `readOnlyRootFilesystem`, dropped capabilities, and `seccompProfile: RuntimeDefault`, so they pass PodSecurity `restricted`. They carry **no httpGet probes** — the agent is a WebSocket client with no health endpoint; the server tracks liveness via heartbeat/eviction (`CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` × `CLOACINA_AGENT_LIVENESS_MISSES`).

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_AGENT_CPU_REQUEST` | CPU request on each agent pod. | `250m` | `500m` | Server (k8s actuator) | No |
| `CLOACINA_PRIORITY_AGING_INTERVAL_SECONDS` | The ageing interval of waiting work, in whole seconds. For each interval a task has waited, its [priority](../../explanation/priority/) in the order of waiting work improves by one, never past 0. `0` switches ageing off. | `120` | `300` | Server | No |
| `CLOACINA_AGENT_MEMORY_REQUEST` | Memory request on each agent pod. | `256Mi` | `512Mi` | Server (k8s actuator) | No |
| `CLOACINA_AGENT_CPU_LIMIT` | CPU limit on each agent pod. | `1` | `2` | Server (k8s actuator) | No |
| `CLOACINA_AGENT_MEMORY_LIMIT` | Memory limit on each agent pod. The agent embeds a CPython interpreter (PyO3) and unpacks a `workflow/`+`vendor/` tree, so the default is generous to avoid OOM-killing Python workflows. | `1Gi` | `2Gi` | Server (k8s actuator) | No |
| `CLOACINA_FLEET_NETWORK_POLICY` | Install a per-tenant agent `NetworkPolicy` (REQ-007 defense-in-depth): default-deny ingress, egress to DNS + the server only. Off-values (`false`/`0`/`no`/`off`) disable it. **Defense-in-depth only** — the server-side ABAC (NFR-004) remains the real tenant boundary. | `true` | `false` | Server (k8s actuator) | No |
| `CLOACINA_SERVER_NAMESPACE` | Namespace the `cloacina-server` runs in. The NetworkPolicy egress allow targets this namespace. Required for the policy; if unset, the actuator **skips** the policy (fail-open) so a missing knob never strands the fleet. | None | `cloacina-system` | Server (k8s actuator) | No |
| `CLOACINA_SERVER_POD_SELECTOR` | Server pod label selector (`k=v,k=v`) the NetworkPolicy egress allow targets. The chart renders the server's `selectorLabels`. Required for the policy (see above). | None | `app.kubernetes.io/name=cloacina-server,app.kubernetes.io/instance=rel` | Server (k8s actuator) | No |
| `CLOACINA_SERVER_PORT` | Server pod port the NetworkPolicy egress allow opens. | `8080` | `8080` | Server (k8s actuator) | No |
| `CLOACINA_FLEET_DNS_NAMESPACE` | Namespace the cluster DNS service runs in; the NetworkPolicy allows UDP+TCP 53 egress there so agents can resolve the server FQDN. | `kube-system` | `kube-system` | Server (k8s actuator) | No |

The actuator mints a tenant-scoped `read` API key and injects it as `CLOACINA_API_KEY`, so spawned agents self-register exactly like a hand-run agent (see [Execution Agent](#execution-agent)). The Docker actuator mints one key per container; the Kubernetes actuator mints one shared per-tenant key into a `Secret` and re-mints it on scale-up.

#### A compiler per tenant

Read by the server. With a fleet actuator and `CLOACINA_TENANT_COMPILERS=true`, the server runs one `cloacina-compiler` for each tenant whose build queue has work and stops it when the queue has been empty for the cooldown. See [Run cloacina-compiler in Production](../../how-to/service/running-the-compiler/#how-many-compilers-a-deployment-runs).

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_TENANT_COMPILERS` | Run a compiler per tenant. | `false` (the chart sets `true` with `fleet.actuator=kubernetes`) | `true` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_IMAGE` | The compiler image. | `cloacina-compiler:latest` | `ghcr.io/colliery-io/cloacina-compiler:0.12.0` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_DATABASE_URL` | The database URL the tenant compilers connect with. | The server's own URL | `postgres://builder:…@db.example:5432/cloacina` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_CARGO_FLAGS` | The whole cargo flag list of a build, separated by spaces (the compiler's `--cargo-flags-replace`). Empty keeps `build --release --lib --frozen --offline`. | empty | `build --lib` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_VENDOR_DIR` | `--vendor-dir`: a cargo home inside the compiler image. | None | `/opt/vendor` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_DEV_WORKSPACE` | `--dev-workspace`: a cloacina workspace inside the image. Development only. | None | `/workspace` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_TARGET_DIR` | `--cargo-target-dir`. An absolute path. | `/workspace/target` | `/var/cache/target` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_BUILD_TIMEOUT_S` | `--build-timeout-s`. | None (the compiler's `600`) | `1200` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_POLL_INTERVAL_MS` | `--poll-interval-ms`. | `1000` | `2000` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_COOLDOWN_S` | Seconds a tenant's build queue must be empty before its compiler is stopped. | `300` | `900` | Server (actuator) | No |
| `CLOACINA_TENANT_COMPILER_CPU_REQUEST` | CPU request of the compiler container. | `500m` | `1` | Server (k8s actuator) | No |
| `CLOACINA_TENANT_COMPILER_MEMORY_REQUEST` | Memory request of the compiler container. | `1Gi` | `2Gi` | Server (k8s actuator) | No |
| `CLOACINA_TENANT_COMPILER_CPU_LIMIT` | CPU limit of the compiler container. | `2` | `4` | Server (k8s actuator) | No |
| `CLOACINA_TENANT_COMPILER_MEMORY_LIMIT` | Memory limit of the compiler container. | `4Gi` | `8Gi` | Server (k8s actuator) | No |
| `CLOACINA_TENANT_COMPILER_CACHE_SIZE` | Size limit of each cache volume (cargo home, target directory). | `8Gi` | `20Gi` | Server (k8s actuator) | No |
| `CLOACINA_TENANT_COMPILER_HARDENED` | Pod hardening: uid `10001`, read-only root filesystem, no capabilities, writable paths on `emptyDir` volumes. `false` runs the image as it is built. | `true` | `false` | Server (k8s actuator) | No |
| `CLOACINA_TENANT_COMPILER_EGRESS` | Crate egress in the compiler pods' NetworkPolicy: `internet` (TCP 443 to public IPv4 addresses), `all`, or `none`. | `internet` | `none` | Server (k8s actuator) | No |
| `CLOACINA_TENANT_COMPILER_EGRESS_CIDRS` | More egress CIDR blocks, separated by commas, every port. | None | `10.9.0.0/24` | Server (k8s actuator) | No |
| `CLOACINA_TENANT_COMPILER_DATABASE_CIDRS` | CIDR blocks of the database, separated by commas. Empty allows the database port to every destination. | None | `10.1.2.3/32` | Server (k8s actuator) | No |

The compiler pods' NetworkPolicy is written when `CLOACINA_FLEET_NETWORK_POLICY` is on, and uses `CLOACINA_FLEET_DNS_NAMESPACE`. The Docker actuator attaches a compiler container to `CLOACINA_AGENT_NETWORK`.

### Autoscaler

The autoscaler is a single leader-gated control loop (one replica drives the fleet via a Postgres advisory lock). Each tick it computes per-tenant utilization (Σ `in_flight` / Σ `max_concurrency` over the tenant's live agents) and nudges `desired_count` by ±1 within `[floor, effective_limit]`, then reconciles actual → desired through the actuator.

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_AUTOSCALE` | Kill-switch for **only** the autoscale step. Off-values (`0`, `false`, `off`, `no`) freeze `desired_count` so operators drive it by hand (provision API); reconciliation keeps running. Defaults **on** whenever an actuator is active. | on (when actuator ≠ `none`) | `false` | Server | No |
| `CLOACINA_AUTOSCALE_UP_THRESHOLD` | Scale **up** by one when a tenant's utilization is strictly greater than this. | `0.8` | `0.75` | Server | No |
| `CLOACINA_AUTOSCALE_DOWN_THRESHOLD` | Scale **down** by one when utilization is strictly less than this. The gap to the up-threshold is the hysteresis band that prevents thrash. | `0.2` | `0.1` | Server | No |
| `CLOACINA_AUTOSCALE_COOLDOWN_S` | Minimum wall-clock seconds between consecutive scale changes for a tenant. | `60` | `120` | Server | No |
| `CLOACINA_AUTOSCALE_FLOOR` | Lower bound on a tenant's `desired_count`; scale-down never goes below it. | `0` | `1` | Server | No |
| `CLOACINA_AUTOSCALE_INTERVAL_S` | How often the control loop ticks (both the autoscale and reconcile steps). Values ≤ 0 fall back to the default. | `30` | `15` | Server | No |

> The unified control loop ticks at `CLOACINA_AUTOSCALE_INTERVAL_S`. (An earlier `CLOACINA_RECONCILE_INTERVAL_S` proposal was superseded — there is no separate reconcile-interval variable.)

See [Execution-Agent Fleet](../../explanation/service/execution-agent-fleet/) for the control-plane concepts and the [Tenant agent fleet](../http-api/#tenant-agent-fleet) API for the per-tenant limit + provision endpoints.

---

## Cloacina Variables (User-Defined Runtime Variables)

Cloacina provides a variable injection system for passing configuration to workflows at runtime, used for external configuration, secrets, and connection strings. All user-defined variables follow the naming convention:

```
CLOACINA_VAR_{NAME}
```

### Convention

```bash
export CLOACINA_VAR_KAFKA_BROKER=localhost:9092
export CLOACINA_VAR_ANALYTICS_DB=postgres://user:pass@host/db
export CLOACINA_VAR_API_KEY=abc123
export CLOACINA_VAR_MODEL_THRESHOLD=0.85
```

### Rust API

```rust
use cloacina::{var, var_or};

// Required variable — returns Err(VarNotFound) if missing
let broker = var("KAFKA_BROKER")?;

// Optional variable with default
let threshold: f64 = var_or("MODEL_THRESHOLD", "0.5").parse().unwrap();
```

### Python API

```python
import cloaca

# Required variable — raises KeyError if missing
broker = cloaca.var("KAFKA_BROKER")

# Optional variable with default
threshold = cloaca.var_or("MODEL_THRESHOLD", "0.5")
```

### Template Resolution

Package metadata can reference variables with `{{ VAR_NAME }}` syntax. The `resolve_template` function expands these references at runtime:

```toml
# In package metadata
[[metadata.accumulators]]
broker = "{{ KAFKA_BROKER }}"
topic = "{{ EVENTS_TOPIC }}"
```

Unresolved references (missing env vars) produce an error listing all missing variable names.

### Common Variable Names

| Variable | Purpose | Example |
|----------|---------|---------|
| `CLOACINA_VAR_KAFKA_BROKER` | Kafka bootstrap server address | `localhost:9092` |
| `CLOACINA_VAR_ANALYTICS_DB` | Analytics database connection string | `postgres://user:pass@host/db` |
| `CLOACINA_VAR_API_KEY` | External API key for workflow tasks | `abc123` |
| `CLOACINA_VAR_MODEL_THRESHOLD` | Numeric configuration for ML models | `0.85` |

These are examples only. You can define any `CLOACINA_VAR_*` name your workflows need.

---

## Constructor Providers

| Variable | Purpose | Default | Component | Required |
|----------|---------|---------|-----------|----------|
| `CLOACINA_PROVIDER_PATH` | Directory of unpacked provider packages that a `constructor!(from = "name[@version]")` reference resolves against. Resolution precedence is: a process-wide override set in code, then this variable, then the `providers` directory relative to the process CWD. Read by the Rust engine, the computation-graph loader, and the Python bindings alike. | `./providers` (relative to CWD) | Library, Computation Graph, Python | No |

See [Consume a provider](../../how-to/constructors/consume-a-provider/) for how providers are packaged and resolved.

---

## Observability

### Logging

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `RUST_LOG` | Controls the tracing/log filter level. Uses the `tracing-subscriber` `EnvFilter` syntax. When unset, defaults to `info`. The `--verbose` CLI flag overrides this to `debug`. The `trace` level writes task data to the log: on the server, the full outcome of each result an agent reports (the output context of a success, the message of a failure). | `info` | `debug`, `cloacina=trace,tower_http=debug` | Server, Daemon, Admin CLI, Python runner | No |

The `RUST_LOG` variable is checked at startup. If set, the Python runner thread also initializes a tracing subscriber using `EnvFilter::from_default_env()`.

### OpenTelemetry (requires `telemetry` feature)

OpenTelemetry tracing is available when `cloacinactl` is compiled with the `telemetry` Cargo feature. The OTLP layer is activated only when `OTEL_EXPORTER_OTLP_ENDPOINT` is set.

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP collector endpoint. When set, enables the OpenTelemetry tracing layer. Uses gRPC via tonic. HTTP/protobuf export is not currently supported. | None (OTEL disabled) | `http://localhost:4317` | Server | No |
| `OTEL_SERVICE_NAME` | Service name reported in traces. | `cloacina` | `cloacina-production` | Server | No |

When `OTEL_EXPORTER_OTLP_ENDPOINT` is **not** set, no OpenTelemetry overhead is incurred; the subscriber runs without the OTEL layer.

### Prometheus Metrics

The server (and `cloacina-compiler`) expose a `/metrics` endpoint in Prometheus exposition format. No environment variable is needed; metrics are always available when the binary is running. By default `/metrics` is on the main listener with no authentication; set `CLOACINA_METRICS_BIND` (server) or `CLOACINA_COMPILER_METRICS_BIND` (compiler) to serve it only on a separate address. The full catalog — every metric name, type, labels, meaning, and example PromQL — lives in [Metrics Catalog](../metrics-catalog/).

The top-of-mind metrics for operators:

- `cloacina_workflows_total` (counter, by `status`, `reason`, `workflow`, `tenant`) — workflow execution counter.
- `cloacina_tasks_total` (counter, by `status`, `reason`, `workflow`, `tenant`) — task attempt counter.
- `cloacina_api_requests_total` (counter, by `method` + `status`).
- `cloacina_workflow_duration_seconds` (histogram).
- `cloacina_task_duration_seconds` (histogram).
- `cloacina_active_workflows` (gauge, by `tenant`, SQL-derived re-seed).
- `cloacina_active_tasks` (gauge, by `tenant`, SQL-derived re-seed).
- `cloacina_waiting_tasks` and `cloacina_oldest_waiting_task_age_seconds` (gauges, by `tenant` and `pool`) — the tasks that wait for capacity and the age of the oldest.
- `cloacina_schedule_lag_seconds` (gauge, by `tenant`) — how late the most overdue cron schedule is.

The legacy `cloacina_pipeline*` names (used in pre-2026 docs) are gone — emission was renamed to `workflow*` to match the user-facing primitive. See the metrics catalog for the full namespace.

---

## Build and Development

### Cargo Features

Cargo features are controlled via `--features` flags at build time, not environment variables. They are documented here for completeness since they affect which environment variables are recognized at runtime.

| Feature | Crate | Description |
|---------|-------|-------------|
| `postgres` | `cloacina`, `cloacinactl` | Enables PostgreSQL backend (diesel, deadpool, tokio-postgres) |
| `sqlite` | `cloacina`, `cloacinactl` | Enables SQLite backend (bundled libsqlite3) |
| `kafka` | `cloacina`, `cloacinactl` | Enables Kafka integration (rdkafka) |
| `macros` | `cloacina` | Enables `#[workflow]` and `#[task]` proc macros |
| `auth` | `cloacina` | Enables API key authentication (requires `postgres`) |
| `telemetry` | `cloacinactl` | Enables OpenTelemetry OTLP tracing export |
| `extension-module` | `cloacina-python` | Enables the PyO3 extension module (the switch maturin flips on when building the `cloaca` wheel) |
| `constructor-packaging` | `cloacina` | Provider archive assemble/sign/pack path — wasmtime-free |
| `constructors-wasm` | `cloacina` | WASM constructor loader + executor bridge (pulls wasmtime/cranelift, ~55 crates); implies `constructor-packaging`; **off by default** |

Default features for `cloacina`: `macros`, `postgres`, `sqlite`

Default features for `cloacinactl`: `postgres`, `sqlite`

There is **no `kafka` feature** — event-source backends ship as
constructor provider crates (e.g. `cloacina-provider-kafka`), not as
core cargo features.

### Test Configuration

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `DATABASE_URL` | PostgreSQL URL for integration tests. | `postgres://cloacina:cloacina@localhost:15432/cloacina` | `postgres://user:pass@ci-host:5432/testdb` | Integration tests | No (uses default) |
| `CLOACINA_TEST_SCHEMA` | PostgreSQL schema for test isolation. When unset, a unique UUID-based schema is generated per test session. Useful in CI for parallelism. | Auto-generated (`test_{uuid}`) | `test_ci_job_42` | Integration tests | No |

---

## Python (PyO3 / Maturin)

The Python wheel (`cloaca`) is built using [maturin](https://github.com/PyO3/maturin) and [PyO3](https://pyo3.rs/). The following environment variables are relevant during the **build** process:

| Variable | Purpose | Default | Example | Context |
|----------|---------|---------|---------|---------|
| `CARGO_PKG_VERSION` | Embedded at compile time as `CLOACINA_VERSION` in the built binary. Set automatically by Cargo. | From `Cargo.toml` | `0.11.0` | Build-time (Cargo) |

### Build Commands

```bash
# Development build (in-place, for testing)
maturin develop --features "postgres,sqlite,macros,extension-module"

# Release wheel
maturin build --release --features "postgres,sqlite,macros,extension-module"
```

Maturin respects standard Python/Rust build environment variables (`CC`, `RUSTFLAGS`, `PYO3_CROSS_*`, etc.) but Cloacina does not define any custom build-time environment variables beyond those in `pyproject.toml`.

### Python Runtime

At runtime, the `cloaca` Python module uses only:

- `CLOACINA_VAR_*` variables (via `cloaca.var()` / `cloaca.var_or()`)
- `RUST_LOG` (to initialize tracing in the dedicated runner thread)

---

## Docker Compose (Development)

The development docker-compose (`.angreal/docker-compose.yaml`) provisions local services with these defaults:

### PostgreSQL

| Variable | Value | Description |
|----------|-------|-------------|
| `POSTGRES_USER` | `cloacina` | PostgreSQL superuser name |
| `POSTGRES_PASSWORD` | `cloacina` | PostgreSQL superuser password |
| `POSTGRES_DB` | `cloacina` | Default database name |

Exposed on host port **15432** (mapped to 5432 in-container) so it
never collides with a locally installed PostgreSQL. This matches the
default `DATABASE_URL` used by tests:

```
postgres://cloacina:cloacina@localhost:15432/cloacina
```

### Kafka

| Variable | Value | Description |
|----------|-------|-------------|
| `KAFKA_NODE_ID` | `1` | KRaft node identifier |
| `KAFKA_PROCESS_ROLES` | `broker,controller` | Combined mode (no ZooKeeper) |
| `KAFKA_LISTENERS` | `PLAINTEXT://:9092,CONTROLLER://:9093` | Listener addresses |
| `KAFKA_ADVERTISED_LISTENERS` | `PLAINTEXT://localhost:9092` | Advertised address for clients |
| `CLUSTER_ID` | `cloacina-dev-cluster-001` | KRaft cluster ID |

Exposed on `localhost:9092`.

---

## Compiler

`cloacina-compiler` reads its own envs from the underlying binary (see [Compiler + Server Deployment Runbook](../../how-to/service/compiler-deployment-runbook/) for the wrapper-flag set and [Running the Compiler](../../how-to/service/running-the-compiler/) for the threat model + hardening flags).

| Variable | Purpose | Default | Component | Notes |
|----------|---------|---------|-----------|-------|
| `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | Wall-clock cap on a single cargo build (seconds). | `600` | Compiler | Past timeout, the build is killed and the row is left for the stale-build sweeper to reclaim. |
| `CLOACINA_MAX_UNPACKED_BYTES` | The largest size, in bytes, that a package archive may unpack to. The compiler refuses to build a larger one. | `524288000` | Compiler | Also settable via `--max-unpacked-bytes`. Use the value of the server. |
| `CLOACINA_COMPILER_VENDOR_DIR` | `CARGO_HOME` for the cargo subprocess — point it at a curated pre-vendored source tree. | (cargo's usual `~/.cargo` when unset) | Compiler | Combined with `--frozen --offline` so package builds resolve only what the operator has allowed. |
| `CLOACINA_COMPILER_BUILD_RLIMIT_*` | Per-build resource caps via `setrlimit` — CPU, memory, FDs, processes. | (binary default) | Compiler | Linux only. The specific variable names mirror `RLIMIT_*` constants. |
| `CLOACINA_COMPILER_DEV_WORKSPACE` | **Dev escape hatch** (CLOACI-T-0887): injects `[patch.crates-io]` entries pointing at a local workspace so package builds resolve local dev crates instead of crates.io. Not for production. | (unset) | Compiler | Also settable via `--dev-workspace`. The demo compose uses it because cloacina crates aren't on crates.io yet. |
| `CLOACINA_COMPILER_PYTHON_PLATFORMS` | The platforms the compiler vendors the dependencies of a Python package for, as a comma-separated list of `<arch>-<os>`. | `aarch64-linux,x86_64-linux` | Compiler | Also settable via `--python-platform` (repeatable). A package the compiler vendored loads on these platforms only. |
| `CLOACINA_COMPILER_PYTHON_VERSION` | The Python version (`major.minor`) the compiler vendors for. | `3.11` | Compiler | Also settable via `--python-version`. Use the Python of the servers and agents. |
| `CLOACINA_COMPILER_PYTHON_INDEX_URL` | The package index that wheels are downloaded from. | (PyPI) | Compiler | Also settable via `--python-index-url`. Given to `uv` in the environment, not on the command line. Not used with `--python-offline` and a wheel folder. |
| `CLOACINA_COMPILER_PYTHON_WHEEL_DIR` | A folder of wheel files, for example one in the compiler image: one more place to find wheels, next to the index. With `CLOACINA_COMPILER_PYTHON_OFFLINE`, the only source. | (unset) | Compiler | Also settable via `--python-wheel-dir`. |
| `CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR` | The cache folder of `uv`. | (`uv`'s own, under the home folder) | Compiler | Also settable via `--python-uv-cache-dir`. The compiler's user must be able to write to it. |
| `CLOACINA_COMPILER_PYTHON_OFFLINE` | `true`: vendor with no network. Every wheel must be in the wheel folder, or else in the `uv` cache. | `false` | Compiler | Also settable via `--python-offline`. Needs a wheel folder or a cache folder. |
| `CLOACINA_UV` | The `uv` program. | `uv` on `PATH` | Compiler, `cloacinactl package build` | Also settable via `--uv` on the compiler. |

### Compiler tenant / target scoping (also accept env vars)

These `cloacina-compiler` CLI flags accept env-var equivalents (via `clap`'s `env = "..."`). They partition build work across dedicated compiler processes.

| Variable | CLI Flag | Default | Description |
|----------|----------|---------|-------------|
| `CLOACINA_TENANT_SCHEMA` | `--tenant-schema` | None (public schema) | Scope this compiler to one tenant's Postgres schema for build isolation. When set, it claims and builds **only** that tenant's pending packages (separate source, logs, and target dir per tenant). Run one compiler per tenant, mirroring the tenant-scoped agent fleet. |
| `CLOACINA_BUILD_TARGET` | `--build-target` | None (primary host compiler) | Run as a **per-target** compiler producing cdylibs for this triple (e.g. `x86_64-linux`). Scan-and-fills `package_artifacts` for success packages lacking this arch, building natively — run the container on that arch. Omit for the primary host compiler that claims pending rows. |
| `CLOACINA_BUILD_TARGET_PACKAGE` | `--build-target-package` | None | Restrict the per-target scan to a single package name (keeps an emulated build cheap). Only meaningful alongside `CLOACINA_BUILD_TARGET`. |

## Install script (`scripts/install.sh`)

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLOACINA_REPO` | Override the GitHub repo to install from (e.g. a fork). | `colliery-io/cloacina` |

Version pinning and install location are flags, not env vars: pass
`--version vX.Y.Z` and `--prefix DIR` (binary lands in `DIR/bin`;
default `$HOME/.cloacina`). The script installs `cloacinactl` only.

## Summary Table

Quick reference of all Cloacina-specific environment variables:

| Variable | Component | Purpose |
|----------|-----------|---------|
| `DATABASE_URL` | Server, Compiler, Admin, Tests | Database connection URL (PostgreSQL or SQLite) |
| `CLOACINA_BOOTSTRAP_KEY` | Server | First-run admin API key |
| `CLOACINA_REQUIRE_SIGNATURES` | Server | Toggle package signature enforcement |
| `CLOACINA_VERIFICATION_ORG_ID` | Server | Server-wide trusted org UUID for signature verification (optional) |
| `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | Server | Per-tenant runner warning limit |
| `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | Server | Drain timeout during tenant teardown |
| `CLOACINA_DATABASE_WAIT_TIMEOUT_S` | Server | Wait for the database at start |
| `CLOACINA_MAX_PACKAGE_BYTES` | Server | Largest package upload, in bytes (default `104857600`) |
| `CLOACINA_MAX_UNPACKED_BYTES` | Server, Compiler, Agent, Daemon, `cloacinactl` | Largest unpacked size of a package archive, in bytes (default `524288000`) |
| `CLOACINA_UV` | `cloacinactl` | Path of the `uv` program that `package build` runs (default: `uv` on `PATH`) |
| `CLOACINA_DEFAULT_EXECUTOR` | Server | Executor key every task is dispatched to (default `default`; set `fleet` to offload to the agent fleet) |
| `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | Server | Advertised fleet heartbeat interval + sweep cadence |
| `CLOACINA_AGENT_LIVENESS_MISSES` | Server | Missed heartbeats before an agent is declared dead |
| `CLOACINA_FLEET_TASK_TIMEOUT_S` | Server | Time limit of a fleet task that sets none of its own (default `300`) |
| `CLOACINA_FLEET_TASK_TIMEOUT_MAX_S` | Server | Longest time limit a fleet task can get (default `86400`) |
| `CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S` | Server | How long a fleet task waits, per attempt, while agents cannot fetch its package artifact (default `300`) |
| `CLOACINA_CONTEXT_WARN_BYTES` | Server | Size above which a task context is logged and counted (default `1048576`; `0` = off) |
| `CLOACINA_CONTEXT_MAX_BYTES` | Server | Largest size of a task context; a larger one fails its task (default `16777216`; `0` = off) |
| `CLOACINA_DEFAULT_MAX_AGENTS` | Server | Platform-wide default per-tenant agent limit (default `4`) |
| `CLOACINA_INITIAL_AGENTS` | Server | Agents auto-provisioned on tenant create (default `1`; `0` disables) |
| `CLOACINA_FLEET_ACTUATOR` | Server | Fleet actuator substrate: `none` / `docker` / `kubernetes` (default `none`) |
| `CLOACINA_AGENT_IMAGE` | Server (actuator) | Agent image the actuator runs per tenant |
| `CLOACINA_AGENT_SERVER_URL` | Server (actuator) | Server URL injected into spawned agents as `CLOACINA_SERVER` |
| `CLOACINA_AGENT_NETWORK` | Server (docker actuator) | Docker network attached to spawned agent containers |
| `CLOACINA_AGENT_CPU_REQUEST` / `_MEMORY_REQUEST` / `_CPU_LIMIT` / `_MEMORY_LIMIT` | Server (k8s actuator) | Agent pod resource requests/limits (defaults `250m`/`256Mi`/`1`/`1Gi`) |
| `CLOACINA_FLEET_NETWORK_POLICY` | Server (k8s actuator) | Install per-tenant agent NetworkPolicy (default `true`; defense-in-depth) |
| `CLOACINA_TENANT_COMPILERS` | Server (actuator) | Run one compiler per tenant, scaled with the tenant's build queue (default `false`); settings are `CLOACINA_TENANT_COMPILER_*` |
| `CLOACINA_SERVER_NAMESPACE` | Server (k8s actuator) | Server namespace targeted by the NetworkPolicy egress allow |
| `CLOACINA_SERVER_POD_SELECTOR` | Server (k8s actuator) | Server pod selector (`k=v,k=v`) targeted by the NetworkPolicy egress allow |
| `CLOACINA_SERVER_PORT` | Server (k8s actuator) | Server pod port opened by the NetworkPolicy egress (default `8080`) |
| `CLOACINA_FLEET_DNS_NAMESPACE` | Server (k8s actuator) | DNS namespace allowed for port-53 egress (default `kube-system`) |
| `CLOACINA_AUTOSCALE` | Server | Kill-switch for the autoscale step (default on when an actuator is active) |
| `CLOACINA_AUTOSCALE_UP_THRESHOLD` | Server | Utilization above which a tenant scales up (default `0.8`) |
| `CLOACINA_AUTOSCALE_DOWN_THRESHOLD` | Server | Utilization below which a tenant scales down (default `0.2`) |
| `CLOACINA_AUTOSCALE_COOLDOWN_S` | Server | Min seconds between scale changes per tenant (default `60`) |
| `CLOACINA_AUTOSCALE_FLOOR` | Server | Lower bound on `desired_count` (default `0`) |
| `CLOACINA_AUTOSCALE_INTERVAL_S` | Server | Control-loop tick interval (default `30`) |
| `CLOACINA_SERVER` | Agent | Server base URL the agent registers with |
| `CLOACINA_API_KEY` | Agent | Agent API key (tenant scope) |
| `CLOACINA_AGENT_CACHE_DIR` | Agent | Fetched-cdylib cache directory |
| `CLOACINA_AGENT_MAX_CONCURRENCY` | Agent | Work packets the agent runs at the same time (default `4`) |
| `CLOACINA_AGENT_POOL` | Agent | The agent pool the agent registers in (unset: `default`) |
| `CLOACINA_AGENT_CPU` | Agent | Total CPU the agent reports for task placement (unset: none) |
| `CLOACINA_AGENT_MEMORY` | Agent | Total memory the agent reports for task placement (unset: none) |
| `CLOACINA_AGENT_LOG_FORMAT` | Agent | Format of the agent's log lines: `text` (default) or `json` |
| `CLOACINA_LOG_FORMAT` | Server | Format of the server's log lines on stderr: `text` (default) or `json` |
| `CLOACINA_TASK_LOG_URL_TEMPLATE` | Server | URL template of a task's logs in the log stack; unset = no link in the UI |
| `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | Compiler | Per-build wall-clock cap (default `600`) |
| `CLOACINA_COMPILER_VENDOR_DIR` | Compiler | `CARGO_HOME` pointed at a curated vendored source tree |
| `CLOACINA_COMPILER_BUILD_RLIMIT_*` | Compiler | Per-build setrlimit caps |
| `CLOACINA_TENANT_SCHEMA` | Compiler | Scope the compiler to one tenant's schema for build isolation |
| `CLOACINA_BUILD_TARGET` | Compiler | Run as a per-target compiler for this triple (e.g. `x86_64-linux`) |
| `CLOACINA_BUILD_TARGET_PACKAGE` | Compiler | Restrict the per-target scan to one package name |
| `CLOACINA_OIDC_ISSUER` | Server | OIDC issuer URL (enables SSO login when set with client-id + redirect) |
| `CLOACINA_OIDC_CLIENT_ID` | Server | OIDC relying-party client ID |
| `CLOACINA_OIDC_CLIENT_SECRET` | Server | OIDC relying-party client secret (empty for public/PKCE clients) |
| `CLOACINA_OIDC_REDIRECT_URI` | Server | OIDC callback URL (matches `/v1/auth/callback`) |
| `CLOACINA_OIDC_SCOPES` | Server | Requested scopes (default `openid,email,profile,groups`) |
| `CLOACINA_OIDC_MAP` | Server | God-owned claim→`{tenant, role}` allowlist; unmatched identities denied |
| `CLOACINA_OIDC_SUCCESS_REDIRECT` | Server | Browser success-redirect URL; unset returns memberships as JSON |
| `CLOACINA_VAR_*` | Library, Python | User-defined runtime variables |
| `CLOACINA_PROVIDER_PATH` | Library, Computation Graph, Python | Provider search-path directory `constructor!(from = …)` resolves against (default `./providers`) |
| `RUST_LOG` | All | Log/trace filter level |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Server (telemetry) | OTLP collector endpoint |
| `OTEL_SERVICE_NAME` | Server (telemetry) | Service name in traces |
| `CLOACINA_TEST_SCHEMA` | Tests | Schema isolation for CI |
| `CLOACINA_COMPILER_DEV_WORKSPACE` | Compiler (dev) | `[patch.crates-io]` escape hatch to local crates — not for production |
| `CLOACINA_DEMO_TENANT_KEYS` | Server (demo) | Deterministic tenant-scoped key seeding, `tenant:key:role,…` — demo stacks only |
| `CLOACINA_EMBEDDED_UI_SKIP_NPM` | Server build | Skip the trunk UI rebuild when compiling with the `embedded-ui` feature and use the existing `ui/dist` (container builds prebuild it in a separate stage; the variable name predates the Leptos port) |
| `CLOACINACTL_VERSION` | Install script | Pin to a release tag |
| `INSTALL_DIR` | Install script | Override install root |

---

## See Also

- [Configuration Reference](../configuration/) -- `DefaultRunnerConfig` fields and `config.toml` schema
- [CLI Reference](../cli/) -- Command-line flags and their env var equivalents
- [Production Deployment](../../how-to/service/production-deployment/) -- Using environment variables in production
