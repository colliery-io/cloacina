---
title: "Environment Variables"
description: "Complete reference for all environment variables used by Cloacina"
weight: 25
aliases:
  - "/platform/reference/environment-variables/"

---

# Environment Variables

This page documents all environment variables recognized by Cloacina components. Variables are grouped by functional area.

---

## Database

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `DATABASE_URL` | PostgreSQL connection URL for the server and admin commands. Can also be set via `--database-url` CLI flag or `database_url` in `config.toml`. | None | `postgresql://cloacina:cloacina@localhost:5432/cloacina` | Server, Admin CLI | Yes (for `serve` and `admin` subcommands) |

### Resolution Order

The database URL is resolved with the following precedence (highest wins):

1. `--database-url` CLI argument
2. `DATABASE_URL` environment variable
3. `database_url` key in `~/.cloacina/config.toml`

If none of these are set, the command exits with an error message listing all three options.

### Notes

- The URL must begin with `postgresql://` or `postgres://`.
- The daemon does **not** use `DATABASE_URL`; it uses an embedded SQLite database at `~/.cloacina/`.
- For multi-tenant deployments, the URL points to a shared PostgreSQL instance and schema isolation is configured separately via the API.

---

## Server

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_BOOTSTRAP_KEY` | Pre-defined API key used as the initial admin key on first server startup. If set and no API keys exist in the database, this value is registered as the admin key instead of auto-generating one. | None (auto-generated) | `sk-my-secret-bootstrap-key-abc123` | Server | No |
| `CLOACINA_REQUIRE_SIGNATURES` | When set (any value), the server enforces package signature verification at upload time. Requires `CLOACINA_VERIFICATION_ORG_ID` to also be set; startup fails fast otherwise. | `false` (off) | `true` | Server | No |
| `CLOACINA_VERIFICATION_ORG_ID` | Trusted organization UUID used to verify package signatures. **Required when `CLOACINA_REQUIRE_SIGNATURES` is set**. | None | `12345678-1234-1234-1234-123456789abc` | Server | Conditional |
| `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | LRU cap on cached per-tenant `DefaultRunner` instances. Each cached runner has its own scheduler loop, executor pool, and DB pool. Bump for high-cardinality SaaS; drop for memory-tight deployments. | `256` | `1024` | Server | No |
| `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | Max seconds to wait for in-flight workflows to drain during tenant teardown (step 2 of the 4-step orchestration). Past this, the runner is hard-evicted; tasks ignoring cooperative cancellation will error on next DB write. | `30` | `60` | Server | No |
| `CLOACINA_CORS_ALLOWED_ORIGINS` | Comma-separated CORS allowed origins. **CORS is disabled by default** — set this to opt in (REQ-009). Use `*` to allow any origin. Needed when a browser app (e.g. the web UI on a different origin) calls the API. | None (CORS off) | `http://localhost:8082,https://app.example.com` | Server | No |
| `CLOACINA_CORS_ALLOWED_METHODS` | Comma-separated CORS allowed methods. Only applies once origins are set. | `GET,POST,DELETE,OPTIONS` | `GET,POST` | Server | No |
| `CLOACINA_CORS_ALLOWED_HEADERS` | Comma-separated CORS allowed request headers. Only applies once origins are set. | `authorization,content-type` | `authorization,content-type,x-tenant` | Server | No |

### Server CLI Flags (also accept env vars)

These are specified via `clap`'s `env = "..."` attribute and can be set as environment variables:

| Variable | CLI Flag | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | `--database-url` | None | Database connection URL |
| `CLOACINA_BOOTSTRAP_KEY` | `--bootstrap-key` | None | Bootstrap admin API key |
| `CLOACINA_REQUIRE_SIGNATURES` | `--require-signatures` | off | Signature enforcement toggle |
| `CLOACINA_VERIFICATION_ORG_ID` | `--verification-org-id` | None | Trusted org UUID |
| `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | `--tenant-runner-cache-size` | `256` | Per-tenant runner cache cap |
| `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | `--tenant-deletion-drain-timeout-s` | `30` | Drain timeout during teardown |
| `CLOACINA_DEFAULT_EXECUTOR` | `--default-executor` | `default` | Executor every task is dispatched to. `default` runs all work on the in-process thread executor; `fleet` sends it to the [execution-agent fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}). Hard-matched against registered executors at startup. Preferred surface is `[server].default_executor` in `config.toml`, which `cloacinactl server start` forwards. |
| `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `--agent-heartbeat-interval-s` | `15` | Heartbeat interval (seconds) the server advertises to fleet agents and uses as its liveness-sweep cadence. Lower = faster dead-agent detection + in-flight reclaim, at the cost of more heartbeat traffic. |
| `CLOACINA_AGENT_LIVENESS_MISSES` | `--agent-liveness-misses` | `3` | Consecutive missed heartbeats before the server marks a fleet agent dead and reclaims its in-flight work. Effective dead-after = interval × misses (default 15s × 3 = 45s). |
| `CLOACINA_CORS_ALLOWED_ORIGINS` | `--cors-allowed-origins` | None (CORS off) | Comma-separated allowed origins; CORS is off until set (REQ-009). `*` allows any. |
| `CLOACINA_CORS_ALLOWED_METHODS` | `--cors-allowed-methods` | `GET,POST,DELETE,OPTIONS` | Comma-separated allowed methods. |
| `CLOACINA_CORS_ALLOWED_HEADERS` | `--cors-allowed-headers` | `authorization,content-type` | Comma-separated allowed request headers. |

`CLOACINA_DEFAULT_EXECUTOR` / `--default-executor` is forwarded by the
`cloacinactl server start` wrapper (preferably set via
`[server].default_executor` in `config.toml`). The two
`CLOACINA_AGENT_*` liveness flags are read by the `cloacina-server` binary
directly (and via the env vars above); the wrapper does **not** forward them,
so set them on `cloacina-server` itself or through the environment.

The bind address (`--bind`, default `127.0.0.1:8080`), `--reconcile-interval-s`, and `--log-retention-days` are CLI-only and do not have environment variable equivalents.

---

## Authentication (OIDC / SSO)

The server can federate login to an external OpenID Connect identity provider (single sign-on). OIDC is **opt-in**: if `CLOACINA_OIDC_ISSUER`, `CLOACINA_OIDC_CLIENT_ID`, and `CLOACINA_OIDC_REDIRECT_URI` are not all set, the OIDC login routes are simply not mounted and only local accounts / API keys are available. A validated OIDC identity is mapped to a cloacina principal by the god-owned allowlist in `CLOACINA_OIDC_MAP`; an identity matching no rule is **denied** (there is no implicit access). See [Configure OIDC / SSO login]({{< ref "/service/how-to/configure-oidc-sso" >}}) for the full setup.

| Variable | Purpose | Default | Component | Required |
|----------|---------|---------|-----------|----------|
| `CLOACINA_OIDC_ISSUER` | OIDC issuer URL used for discovery + JWKS. Presence of this (with client id + redirect) is what mounts the OIDC login routes. | None (OIDC off) | Server | Conditional (all three of issuer/client-id/redirect to enable) |
| `CLOACINA_OIDC_CLIENT_ID` | Relying-party client ID registered with the IdP. | None | Server | Conditional |
| `CLOACINA_OIDC_CLIENT_SECRET` | Relying-party client secret. Empty when the IdP treats the client as public (PKCE). | Empty | Server | No |
| `CLOACINA_OIDC_REDIRECT_URI` | Callback URL the IdP redirects to after login. Must match the server's `/v1/auth/callback` route as registered with the IdP. | None | Server | Conditional |
| `CLOACINA_OIDC_SCOPES` | Comma-separated scopes requested at login. | `openid,email,profile,groups` | Server | No |
| `CLOACINA_OIDC_MAP` | God-owned allowlist mapping IdP claims to `{tenant, role}`. `;`-separated rules `<match>=<tenant>:<role>`, where `<match>` is `group:NAME` / `domain:NAME` / `sub:NAME` and `<tenant>` may be `_` for a global principal. First matching rule wins; an unmatched identity is denied. Example: `group:acme-admins=acme:admin;domain:acme.com=acme:write`. | Empty (all identities denied) | Server | No (but empty = no OIDC access granted) |
| `CLOACINA_OIDC_SUCCESS_REDIRECT` | When set, the browser login flow redirects here on success, handing the minted membership set to the SPA via the URL fragment. Unset = the callback returns the memberships as JSON. | None (JSON response) | Server | No |

---

## Execution Agent

The [execution-agent fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}) (`cloacina-agent`) is a DB-less worker that registers with a server, fetches compiled workflow artifacts, executes tasks, and reports results. It holds **no** database connection; all of its configuration is server + API-key oriented.

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_SERVER` | Base URL of the `cloacina-server` the agent registers with (used for both REST and the WebSocket ticket mint). Equivalent to `--server`. | None | `http://cloacina-server:8080` | Agent | Yes |
| `CLOACINA_API_KEY` | API key the agent authenticates with. Its tenant scope determines which tenants' work the agent may receive (REQ-008 tenant isolation). Equivalent to `--api-key`. | None | `sk-...` | Agent | Yes |
| `CLOACINA_AGENT_CACHE_DIR` | Directory used to cache fetched workflow cdylibs by digest, so a cache hit skips the REST fetch. Equivalent to `--cache-dir`. | `<TMPDIR>/cloacina-agent-cache` | `/var/lib/cloacina-agent/cache` | Agent | No |

The remaining agent options — `--agent-id`, `--max-concurrency` (default `4`), `--capabilities`, and `--target-triple-override` — are CLI-only; see the [CLI Reference]({{< ref "cli" >}}#agent).

---

## Fleet actuator & autoscaler

These server-side variables drive the **agent self-management control plane**: the server can hold a per-tenant agent-capacity limit, provision a per-tenant pool of `cloacina-agent` workloads on a pluggable substrate (the *actuator*), and autoscale that pool from observed utilization. The actuator and autoscaler only run when an actuator is selected — with `CLOACINA_FLEET_ACTUATOR=none` (the default) no pool is provisioned and the control loop does not start. A provisioned pool only does useful work when the server's [default executor]({{< ref "/service/explanation/execution-agent-fleet" >}}) is `fleet` (otherwise tasks run in-process and the agents sit idle).

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
| `CLOACINA_AGENT_IMAGE` | Agent image the actuator runs for each tenant. | `cloacina-agent:latest` | `ghcr.io/colliery-software/cloacina-agent:latest` | Server (actuator) | No |
| `CLOACINA_AGENT_SERVER_URL` | Server URL injected into each spawned agent as `CLOACINA_SERVER` (the agent's registration target). | `http://server:8080` | `http://cloacina-server:8080` | Server (actuator) | No |
| `CLOACINA_AGENT_NETWORK` | **Docker actuator only.** Docker network attached to each spawned agent container so it can reach the server (e.g. the compose network). Unset = the daemon default. Ignored by the Kubernetes actuator (in-cluster pods reach the server by Service DNS). | None | `cloacina_net` | Server (docker actuator) | No |

#### Kubernetes agent-pod hardening

These are read by the **Kubernetes actuator** and applied to every `cloacina-agent` pod it creates. The chart renders them from `fleet.agentResources` / `fleet.networkPolicy` when `fleet.actuator=kubernetes`; defaults are sane if unset. Agent pods are created non-root (uid/gid `10001`) with `readOnlyRootFilesystem`, dropped capabilities, and `seccompProfile: RuntimeDefault`, so they pass PodSecurity `restricted`. They carry **no httpGet probes** — the agent is a WebSocket client with no health endpoint; the server tracks liveness via heartbeat/eviction (`CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` × `CLOACINA_AGENT_LIVENESS_MISSES`).

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_AGENT_CPU_REQUEST` | CPU request on each agent pod. | `250m` | `500m` | Server (k8s actuator) | No |
| `CLOACINA_AGENT_MEMORY_REQUEST` | Memory request on each agent pod. | `256Mi` | `512Mi` | Server (k8s actuator) | No |
| `CLOACINA_AGENT_CPU_LIMIT` | CPU limit on each agent pod. | `1` | `2` | Server (k8s actuator) | No |
| `CLOACINA_AGENT_MEMORY_LIMIT` | Memory limit on each agent pod. The agent embeds a CPython interpreter (PyO3) and unpacks a `workflow/`+`vendor/` tree, so the default is generous to avoid OOM-killing Python workflows. | `1Gi` | `2Gi` | Server (k8s actuator) | No |
| `CLOACINA_FLEET_NETWORK_POLICY` | Install a per-tenant agent `NetworkPolicy` (REQ-007 defense-in-depth): default-deny ingress, egress to DNS + the server only. Off-values (`false`/`0`/`no`/`off`) disable it. **Defense-in-depth only** — the server-side ABAC (NFR-004) remains the real tenant boundary. | `true` | `false` | Server (k8s actuator) | No |
| `CLOACINA_SERVER_NAMESPACE` | Namespace the `cloacina-server` runs in. The NetworkPolicy egress allow targets this namespace. Required for the policy; if unset, the actuator **skips** the policy (fail-open) so a missing knob never strands the fleet. | None | `cloacina-system` | Server (k8s actuator) | No |
| `CLOACINA_SERVER_POD_SELECTOR` | Server pod label selector (`k=v,k=v`) the NetworkPolicy egress allow targets. The chart renders the server's `selectorLabels`. Required for the policy (see above). | None | `app.kubernetes.io/name=cloacina-server,app.kubernetes.io/instance=rel` | Server (k8s actuator) | No |
| `CLOACINA_SERVER_PORT` | Server pod port the NetworkPolicy egress allow opens. | `8080` | `8080` | Server (k8s actuator) | No |
| `CLOACINA_FLEET_DNS_NAMESPACE` | Namespace the cluster DNS service runs in; the NetworkPolicy allows UDP+TCP 53 egress there so agents can resolve the server FQDN. | `kube-system` | `kube-system` | Server (k8s actuator) | No |

The actuator mints a tenant-scoped `read` API key and injects it as `CLOACINA_API_KEY`, so spawned agents self-register exactly like a hand-run agent (see [Execution Agent](#execution-agent)). The Docker actuator mints one key per container; the Kubernetes actuator mints one shared per-tenant key into a `Secret` and re-mints it on scale-up.

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

See [Execution-Agent Fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}) for the control-plane concepts and the [Tenant agent fleet]({{< ref "/reference/http-api" >}}#tenant-agent-fleet) API for the per-tenant limit + provision endpoints.

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

See [Consume a provider]({{< ref "/engine/constructors/consume-a-provider" >}}) for how providers are packaged and resolved.

---

## Observability

### Logging

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `RUST_LOG` | Controls the tracing/log filter level. Uses the `tracing-subscriber` `EnvFilter` syntax. When unset, defaults to `info`. The `--verbose` CLI flag overrides this to `debug`. | `info` | `debug`, `cloacina=trace,tower_http=debug` | Server, Daemon, Admin CLI, Python runner | No |

The `RUST_LOG` variable is checked at startup. If set, the Python runner thread also initializes a tracing subscriber using `EnvFilter::from_default_env()`.

### OpenTelemetry (requires `telemetry` feature)

OpenTelemetry tracing is available when `cloacinactl` is compiled with the `telemetry` Cargo feature. The OTLP layer is activated only when `OTEL_EXPORTER_OTLP_ENDPOINT` is set.

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP collector endpoint. When set, enables the OpenTelemetry tracing layer. Uses gRPC via tonic. HTTP/protobuf export is not currently supported. | None (OTEL disabled) | `http://localhost:4317` | Server | No |
| `OTEL_SERVICE_NAME` | Service name reported in traces. | `cloacina` | `cloacina-production` | Server | No |

When `OTEL_EXPORTER_OTLP_ENDPOINT` is **not** set, no OpenTelemetry overhead is incurred; the subscriber runs without the OTEL layer.

### Prometheus Metrics

The server (and `cloacina-compiler`) expose a `/metrics` endpoint in Prometheus exposition format. No environment variable is needed; metrics are always available when the binary is running. The full catalog — every metric name, type, labels, meaning, and example PromQL — lives in [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}}).

The top-of-mind metrics for operators:

- `cloacina_workflows_total` (counter, by `status` + `reason`) — workflow execution counter.
- `cloacina_tasks_total` (counter, by `status` + `reason`) — task execution counter.
- `cloacina_api_requests_total` (counter, by `method` + `status`).
- `cloacina_workflow_duration_seconds` (histogram).
- `cloacina_task_duration_seconds` (histogram).
- `cloacina_active_workflows` (gauge, SQL-derived re-seed).
- `cloacina_active_tasks` (gauge, SQL-derived re-seed).

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
| `extension-module` | `cloacina` | Enables PyO3 extension module (used by maturin build) |

Default features for `cloacina`: `macros`, `postgres`, `sqlite`, `kafka`

Default features for `cloacinactl`: `postgres`, `sqlite`, `kafka`

### Test Configuration

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `DATABASE_URL` | PostgreSQL URL for integration tests. | `postgres://cloacina:cloacina@localhost:5432/cloacina` | `postgres://user:pass@ci-host:5432/testdb` | Integration tests | No (uses default) |
| `CLOACINA_TEST_SCHEMA` | PostgreSQL schema for test isolation. When unset, a unique UUID-based schema is generated per test session. Useful in CI for parallelism. | Auto-generated (`test_{uuid}`) | `test_ci_job_42` | Integration tests | No |

---

## Python (PyO3 / Maturin)

The Python wheel (`cloaca`) is built using [maturin](https://github.com/PyO3/maturin) and [PyO3](https://pyo3.rs/). The following environment variables are relevant during the **build** process:

| Variable | Purpose | Default | Example | Context |
|----------|---------|---------|---------|---------|
| `CARGO_PKG_VERSION` | Embedded at compile time as `CLOACINA_VERSION` in the built binary. Set automatically by Cargo. | From `Cargo.toml` | `0.7.0` | Build-time (Cargo) |

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

Exposed on `localhost:5432`. This matches the default `DATABASE_URL` used by tests:

```
postgres://cloacina:cloacina@localhost:5432/cloacina
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

`cloacina-compiler` reads its own envs from the underlying binary (see [Compiler + Server Deployment Runbook]({{< ref "/service/how-to/compiler-deployment-runbook" >}}) for the wrapper-flag set and [Running the Compiler]({{< ref "/service/how-to/running-the-compiler" >}}) for the threat model + hardening flags).

| Variable | Purpose | Default | Component | Notes |
|----------|---------|---------|-----------|-------|
| `CLOACINA_COMPILER_SANDBOX` | Process-isolation posture for the build sandbox. `required` = builds run under the bwrap namespace sandbox (level 1) or the compiler **refuses to start**; `preferred` = use the best available level, logging any downgrade loudly; `off` = no process sandbox (dev laptops only). Validated **fail-closed at boot** — an invalid value or a `required` selection with no bwrap available is a fatal boot error, never a silent downgrade. See [Compiler Build Sandbox]({{< ref "/service/compiler-sandbox" >}}). | `preferred` | Compiler | The achieved level (bwrap / landlock / none) is recorded on every build's audit row. |
| `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | Wall-clock cap on a single cargo build (seconds). | `600` | Compiler | Past timeout, the build is killed and the row is left for the stale-build sweeper to reclaim. |
| `CLOACINA_COMPILER_VENDOR_DIR` | `CARGO_HOME` for the cargo subprocess — point it at a curated pre-vendored source tree. | (cargo's usual `~/.cargo` when unset) | Compiler | Combined with `--frozen --offline` so package builds resolve only what the operator has allowed. |
| `CLOACINA_COMPILER_BUILD_RLIMIT_*` | Per-build resource caps via `setrlimit` — CPU, memory, FDs, processes. | (binary default) | Compiler | Linux only. The specific variable names mirror `RLIMIT_*` constants. |

### Compiler tenant / target scoping (also accept env vars)

These `cloacina-compiler` CLI flags accept env-var equivalents (via `clap`'s `env = "..."`). They partition build work across dedicated compiler processes.

| Variable | CLI Flag | Default | Description |
|----------|----------|---------|-------------|
| `CLOACINA_TENANT_SCHEMA` | `--tenant-schema` | None (public schema) | Scope this compiler to one tenant's Postgres schema for build isolation. When set, it claims and builds **only** that tenant's pending packages (separate source, logs, and target dir per tenant). Run one compiler per tenant, mirroring the tenant-scoped agent fleet. |
| `CLOACINA_BUILD_TARGET` | `--build-target` | None (primary host compiler) | Run as a **per-target** compiler producing cdylibs for this triple (e.g. `x86_64-linux`). Scan-and-fills `package_artifacts` for success packages lacking this arch, building natively — run the container on that arch. Omit for the primary host compiler that claims pending rows. |
| `CLOACINA_BUILD_TARGET_PACKAGE` | `--build-target-package` | None | Restrict the per-target scan to a single package name (keeps an emulated build cheap). Only meaningful alongside `CLOACINA_BUILD_TARGET`. |

## Install script (`install.sh`)

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLOACINACTL_VERSION` | Pin to a specific release tag. Equivalent to `--version` on the one-liner. | (latest) |
| `INSTALL_DIR` | Override install root. Equivalent to `--prefix`. | `$HOME/.cloacina` |
| `CLOACINA_REPO` | Install from a fork instead of `colliery-io/cloacina`. | `colliery-io/cloacina` |

## Summary Table

Quick reference of all Cloacina-specific environment variables:

| Variable | Component | Purpose |
|----------|-----------|---------|
| `DATABASE_URL` | Server, Admin, Tests | PostgreSQL connection URL |
| `CLOACINA_BOOTSTRAP_KEY` | Server | First-run admin API key |
| `CLOACINA_REQUIRE_SIGNATURES` | Server | Toggle package signature enforcement |
| `CLOACINA_VERIFICATION_ORG_ID` | Server | Trusted org UUID for signature verification |
| `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | Server | Per-tenant runner LRU cap |
| `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | Server | Drain timeout during tenant teardown |
| `CLOACINA_DEFAULT_EXECUTOR` | Server | Executor key every task is dispatched to (default `default`; set `fleet` to offload to the agent fleet) |
| `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | Server | Advertised fleet heartbeat interval + sweep cadence |
| `CLOACINA_AGENT_LIVENESS_MISSES` | Server | Missed heartbeats before an agent is declared dead |
| `CLOACINA_DEFAULT_MAX_AGENTS` | Server | Platform-wide default per-tenant agent limit (default `4`) |
| `CLOACINA_INITIAL_AGENTS` | Server | Agents auto-provisioned on tenant create (default `1`; `0` disables) |
| `CLOACINA_FLEET_ACTUATOR` | Server | Fleet actuator substrate: `none` / `docker` / `kubernetes` (default `none`) |
| `CLOACINA_AGENT_IMAGE` | Server (actuator) | Agent image the actuator runs per tenant |
| `CLOACINA_AGENT_SERVER_URL` | Server (actuator) | Server URL injected into spawned agents as `CLOACINA_SERVER` |
| `CLOACINA_AGENT_NETWORK` | Server (docker actuator) | Docker network attached to spawned agent containers |
| `CLOACINA_AGENT_CPU_REQUEST` / `_MEMORY_REQUEST` / `_CPU_LIMIT` / `_MEMORY_LIMIT` | Server (k8s actuator) | Agent pod resource requests/limits (defaults `250m`/`256Mi`/`1`/`1Gi`) |
| `CLOACINA_FLEET_NETWORK_POLICY` | Server (k8s actuator) | Install per-tenant agent NetworkPolicy (default `true`; defense-in-depth) |
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
| `CLOACINA_COMPILER_SANDBOX` | Compiler | Build sandbox posture `required` / `preferred` / `off` (default `preferred`; fail-closed at boot) |
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
| `CLOACINACTL_VERSION` | Install script | Pin to a release tag |
| `INSTALL_DIR` | Install script | Override install root |
| `CLOACINA_REPO` | Install script | Install from a fork |

---

## See Also

- [Configuration Reference]({{< ref "configuration" >}}) -- `DefaultRunnerConfig` fields and `config.toml` schema
- [CLI Reference]({{< ref "cli" >}}) -- Command-line flags and their env var equivalents
- [Production Deployment]({{< ref "/service/how-to/production-deployment" >}}) -- Using environment variables in production
