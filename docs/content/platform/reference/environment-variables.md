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

### Server CLI Flags (also accept env vars)

These are specified via `clap`'s `env = "..."` attribute and can be set as environment variables:

| Variable | CLI Flag | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | `--database-url` | None | Database connection URL |
| `CLOACINA_BOOTSTRAP_KEY` | `--bootstrap-key` | None | Bootstrap admin API key |

The bind address (`--bind`, default `0.0.0.0:8080`) is CLI-only and does not have an environment variable equivalent.

---

## Cloacina Variables (User-Defined Runtime Variables)

Cloacina provides an Airflow-style variable system for injecting external configuration, secrets, and connection strings into workflows at runtime. All user-defined variables follow the naming convention:

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
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP collector endpoint. When set, enables the OpenTelemetry tracing layer. Uses gRPC (tonic). | None (OTEL disabled) | `http://localhost:4317` | Server | No |
| `OTEL_SERVICE_NAME` | Service name reported in traces. | `cloacina` | `cloacina-production` | Server | No |

When `OTEL_EXPORTER_OTLP_ENDPOINT` is **not** set, no OpenTelemetry overhead is incurred; the subscriber runs without the OTEL layer.

### Prometheus Metrics

The server exposes a `/metrics` endpoint in Prometheus exposition format. No environment variable is needed; metrics are always available when the server is running. Metrics include:

- `cloacina_pipelines_total` (counter, by status)
- `cloacina_tasks_total` (counter, by status)
- `cloacina_api_requests_total` (counter, by method/path/status)
- `cloacina_pipeline_duration_seconds` (histogram)
- `cloacina_task_duration_seconds` (histogram)
- `cloacina_active_pipelines` (gauge)
- `cloacina_active_tasks` (gauge)

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
| `CARGO_PKG_VERSION` | Embedded at compile time as `CLOACINA_VERSION` in the built binary. Set automatically by Cargo. | From `Cargo.toml` | `0.5.0` | Build-time (Cargo) |

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

## Summary Table

Quick reference of all Cloacina-specific environment variables:

| Variable | Component | Purpose |
|----------|-----------|---------|
| `DATABASE_URL` | Server, Admin, Tests | PostgreSQL connection URL |
| `CLOACINA_BOOTSTRAP_KEY` | Server | First-run admin API key |
| `CLOACINA_VAR_*` | Library, Python | User-defined runtime variables |
| `RUST_LOG` | All | Log/trace filter level |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Server (telemetry) | OTLP collector endpoint |
| `OTEL_SERVICE_NAME` | Server (telemetry) | Service name in traces |
| `CLOACINA_TEST_SCHEMA` | Tests | Schema isolation for CI |

---

## See Also

- [Configuration Reference]({{< ref "configuration" >}}) -- `DefaultRunnerConfig` fields and `config.toml` schema
- [CLI Reference]({{< ref "cli" >}}) -- Command-line flags and their env var equivalents
- [Production Deployment]({{< ref "/platform/how-to-guides/production-deployment" >}}) -- Using environment variables in production
