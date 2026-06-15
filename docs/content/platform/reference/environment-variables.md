---
title: "Environment Variables"
description: "Complete reference for all environment variables used by Cloacina"
weight: 25
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
| `CLOACINA_REQUIRE_SIGNATURES` | When set (any value), the server enforces package signature verification at upload time. Requires `CLOACINA_VERIFICATION_ORG_ID` to also be set; startup fails fast otherwise. CLOACI-I-0103. | `false` (off) | `true` | Server | No |
| `CLOACINA_VERIFICATION_ORG_ID` | Trusted organization UUID used to verify package signatures. **Required when `CLOACINA_REQUIRE_SIGNATURES` is set**. CLOACI-I-0103 / T-0567. | None | `12345678-1234-1234-1234-123456789abc` | Server | Conditional |
| `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | LRU cap on cached per-tenant `DefaultRunner` instances. Each cached runner has its own scheduler loop, executor pool, and DB pool. Bump for high-cardinality SaaS; drop for memory-tight deployments. CLOACI-T-0580. | `256` | `1024` | Server | No |
| `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | Max seconds to wait for in-flight workflows to drain during tenant teardown (step 2 of the 4-step orchestration). Past this, the runner is hard-evicted; tasks ignoring cooperative cancellation will error on next DB write. CLOACI-T-0581. | `30` | `60` | Server | No |
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
| `CLOACINA_DEFAULT_EXECUTOR` | `--default-executor` | `default` | Executor every task is dispatched to (CLOACI-T-0640). `default` runs all work on the in-process thread executor; `fleet` sends it to the [execution-agent fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}). Hard-matched against registered executors at startup. Preferred surface is `[server].default_executor` in `config.toml`, which `cloacinactl server start` forwards. |
| `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `--agent-heartbeat-interval-s` | `15` | Heartbeat interval (seconds) the server advertises to fleet agents and uses as its liveness-sweep cadence. Lower = faster dead-agent detection + in-flight reclaim, at the cost of more heartbeat traffic. CLOACI-T-0639. |
| `CLOACINA_AGENT_LIVENESS_MISSES` | `--agent-liveness-misses` | `3` | Consecutive missed heartbeats before the server marks a fleet agent dead and reclaims its in-flight work. Effective dead-after = interval × misses (default 15s × 3 = 45s). CLOACI-T-0639. |
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

## Execution Agent

The [execution-agent fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}) (`cloacina-agent`) is a DB-less worker that registers with a server, fetches compiled workflow artifacts, executes tasks, and reports results. It holds **no** database connection; all of its configuration is server + API-key oriented.

| Variable | Purpose | Default | Example | Component | Required |
|----------|---------|---------|---------|-----------|----------|
| `CLOACINA_SERVER` | Base URL of the `cloacina-server` the agent registers with (used for both REST and the WebSocket ticket mint). Equivalent to `--server`. | None | `http://cloacina-server:8080` | Agent | Yes |
| `CLOACINA_API_KEY` | API key the agent authenticates with. Its tenant scope determines which tenants' work the agent may receive (REQ-008 tenant isolation). Equivalent to `--api-key`. | None | `sk-...` | Agent | Yes |
| `CLOACINA_AGENT_CACHE_DIR` | Directory used to cache fetched workflow cdylibs by digest, so a cache hit skips the REST fetch. Equivalent to `--cache-dir`. | `<TMPDIR>/cloacina-agent-cache` | `/var/lib/cloacina-agent/cache` | Agent | No |

The remaining agent options — `--agent-id`, `--max-concurrency` (default `4`), `--capabilities`, and `--target-triple-override` — are CLI-only; see the [CLI Reference]({{< ref "cli" >}}#agent).

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

The server (and `cloacina-compiler`, per CLOACI-I-0109) expose a `/metrics` endpoint in Prometheus exposition format. No environment variable is needed; metrics are always available when the binary is running. The full catalog — every metric name, type, labels, meaning, and example PromQL — lives in [Metrics Catalog]({{< ref "/platform/reference/metrics-catalog" >}}).

The top-of-mind metrics for operators:

- `cloacina_workflows_total` (counter, by `status` + `reason`) — workflow execution counter.
- `cloacina_tasks_total` (counter, by `status` + `reason`) — task execution counter.
- `cloacina_api_requests_total` (counter, by `method` + `status`).
- `cloacina_workflow_duration_seconds` (histogram).
- `cloacina_task_duration_seconds` (histogram).
- `cloacina_active_workflows` (gauge, SQL-derived re-seed per CLOACI-I-0108).
- `cloacina_active_tasks` (gauge, SQL-derived re-seed per CLOACI-I-0108).

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
| `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | Wall-clock cap on a single cargo build, per CLOACI-I-0104 OPS-10. | (binary default) | Compiler | Past timeout, the build row is marked `timed_out`; the stale-build sweeper reclaims it. |
| `CLOACINA_COMPILER_VENDOR_DIR` | Path to the curated pre-vendored cargo registry. Defaults to a directory under `CLOACINA_HOME`. | (binary default) | Compiler | Builds run with `--frozen --offline` against this directory. CLOACI-I-0104. |
| `CLOACINA_COMPILER_BUILD_RLIMIT_*` | Per-build resource caps via `setrlimit` — CPU, memory, FDs, processes. | (binary default) | Compiler | Linux only. The specific variable names mirror `RLIMIT_*` constants. CLOACI-I-0104. |

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
| `CLOACINA_SERVER` | Agent | Server base URL the agent registers with |
| `CLOACINA_API_KEY` | Agent | Agent API key (tenant scope) |
| `CLOACINA_AGENT_CACHE_DIR` | Agent | Fetched-cdylib cache directory |
| `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | Compiler | Per-build wall-clock cap |
| `CLOACINA_COMPILER_VENDOR_DIR` | Compiler | Curated vendored cargo registry path |
| `CLOACINA_COMPILER_BUILD_RLIMIT_*` | Compiler | Per-build setrlimit caps |
| `CLOACINA_VAR_*` | Library, Python | User-defined runtime variables |
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
