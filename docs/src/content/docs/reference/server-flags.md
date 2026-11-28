---
title: "Server Flags"
description: "Every flag and environment variable of cloacina-server, generated from its command."
sidebar:
  order: 16
---

This page is generated from the `cloacina-server` command
(`crates/cloacina-server/src/main.rs`) by `python3 scripts/check_release_compat.py flags --write`.
Do not edit it by hand: the release gate fails when it differs from the
code, and compares the flags and environment variables with the last
release (see [the release gate](../../contributing/release-gate/)).

A default is the text of the code: a literal, or the name of a constant
of the code. For the meaning of each setting, see
[Configuration](../configuration/) and
[Environment variables](../environment-variables/).

## Flags

| Flag | Environment variable | Default |
|------|----------------------|---------|
| `--agent-heartbeat-interval-s` | `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `cloacina::fleet::DEFAULT_HEARTBEAT_INTERVAL_SECONDS` |
| `--agent-key-lifetime-days` | `CLOACINA_AGENT_KEY_LIFETIME_DAYS` | `cloacina_server::actuator::AgentKeyExpiry::DEFAULT_LIFETIME_DAYS` |
| `--agent-key-rotate-days` | `CLOACINA_AGENT_KEY_ROTATE_DAYS` | `cloacina_server::actuator::AgentKeyExpiry::DEFAULT_ROTATE_DAYS` |
| `--agent-liveness-misses` | `CLOACINA_AGENT_LIVENESS_MISSES` | `3` |
| `--alert-webhook-events` | `CLOACINA_ALERT_WEBHOOK_EVENTS` | — |
| `--alert-webhook-max-attempts` | `CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS` | `8` |
| `--alert-webhook-timeout-s` | `CLOACINA_ALERT_WEBHOOK_TIMEOUT_S` | `10` |
| `--alert-webhook-url` | `CLOACINA_ALERT_WEBHOOK_URL` | — |
| `--bind` | — | `"127.0.0.1:8080"` |
| `--bootstrap-key` | `CLOACINA_BOOTSTRAP_KEY` | — |
| `--context-max-bytes` | `CLOACINA_CONTEXT_MAX_BYTES` | `cloacina::executor::context_size::DEFAULT_CONTEXT_MAX_BYTES` |
| `--context-warn-bytes` | `CLOACINA_CONTEXT_WARN_BYTES` | `cloacina::executor::context_size::DEFAULT_CONTEXT_WARN_BYTES` |
| `--cors-allowed-headers` | `CLOACINA_CORS_ALLOWED_HEADERS` | — |
| `--cors-allowed-methods` | `CLOACINA_CORS_ALLOWED_METHODS` | — |
| `--cors-allowed-origins` | `CLOACINA_CORS_ALLOWED_ORIGINS` | — |
| `--database-url` | `DATABASE_URL` | — |
| `--database-wait-timeout-s` | `CLOACINA_DATABASE_WAIT_TIMEOUT_S` | `60` |
| `--default-executor` | `CLOACINA_DEFAULT_EXECUTOR` | — |
| `--fleet-artifact-fetch-wait-s` | `CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S` | `cloacina_server::fleet_executor::DEFAULT_ARTIFACT_FETCH_WAIT_SECONDS` |
| `--fleet-graph-firings` | `CLOACINA_FLEET_GRAPH_FIRINGS` | `cloacina_server::fleet_graph_executor::GraphFiringExecution::Inline` |
| `--fleet-poll-timeout-s` | `CLOACINA_FLEET_POLL_TIMEOUT_S` | `cloacina_server::fleet_poll_dispatcher::DEFAULT_POLL_TIMEOUT_SECONDS` |
| `--fleet-task-timeout-max-s` | `CLOACINA_FLEET_TASK_TIMEOUT_MAX_S` | `cloacina_server::fleet_executor::FleetTaskTimeLimits::DEFAULT_MAX_SECONDS` |
| `--fleet-task-timeout-s` | `CLOACINA_FLEET_TASK_TIMEOUT_S` | `cloacina_server::fleet_executor::FleetTaskTimeLimits::DEFAULT_SECONDS` |
| `--home` | — | `default_home()` |
| `--log-format` | `CLOACINA_LOG_FORMAT` | `Default::default()` |
| `--log-retention-days` | — | `14` |
| `--max-package-bytes` | `CLOACINA_MAX_PACKAGE_BYTES` | `cloacina_server::upload_limit::DEFAULT_MAX_PACKAGE_BYTES` |
| `--max-unpacked-bytes` | `CLOACINA_MAX_UNPACKED_BYTES` | `cloacina::cloacina_workflow_plugin::unpack_limit::DEFAULT_MAX_UNPACKED_BYTES` |
| `--metrics-bind` | `CLOACINA_METRICS_BIND` | — |
| `--outbox-acked-retention-s` | `CLOACINA_OUTBOX_ACKED_RETENTION_S` | `0` |
| `--priority-aging-interval-s` | `CLOACINA_PRIORITY_AGING_INTERVAL_SECONDS` | `cloacina_api_types::DEFAULT_PRIORITY_AGING_INTERVAL.as_secs()` |
| `--public-base-url` | `CLOACINA_PUBLIC_BASE_URL` | — |
| `--reconcile-interval-s` | — | — |
| `--require-signatures` | `CLOACINA_REQUIRE_SIGNATURES` | — |
| `--retention-interval-s` | `CLOACINA_RETENTION_INTERVAL_S` | `300` |
| `--run-history-retention-days` | `CLOACINA_RUN_HISTORY_RETENTION_DAYS` | `0` |
| `--single-trust` | `CLOACINA_SINGLE_TRUST` | — |
| `--task-log-url-template` | `CLOACINA_TASK_LOG_URL_TEMPLATE` | — |
| `--task-output-max-bytes` | `CLOACINA_TASK_OUTPUT_MAX_BYTES` | `cloacina::fleet::DEFAULT_TASK_OUTPUT_MAX_BYTES` |
| `--task-output-retention-days` | `CLOACINA_TASK_OUTPUT_RETENTION_DAYS` | `cloacina_server::retention::DEFAULT_TASK_OUTPUT_RETENTION_DAYS` |
| `--tenant-deletion-drain-timeout-s` | `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | `30` |
| `--tenant-runner-cache-size` | `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | `256` |
| `--verbose`, `-v` | — | — |
| `--verification-org-id` | `CLOACINA_VERIFICATION_ORG_ID` | — |

## Environment variables with no flag

The environment variable names in the server code (tests left out) that
no flag reads: a name read with `env::var`, or a `CLOACINA_*` string. Most
are settings the server reads; some are variables it sets on the agent and
compiler pods it starts.

- `CLOACINA_ADVERTISED_ADDRESS`
- `CLOACINA_AGENT_CPU`
- `CLOACINA_AGENT_CPU_LIMIT`
- `CLOACINA_AGENT_CPU_REQUEST`
- `CLOACINA_AGENT_DRAIN_S`
- `CLOACINA_AGENT_DRAIN_SECONDS`
- `CLOACINA_AGENT_ID`
- `CLOACINA_AGENT_IMAGE`
- `CLOACINA_AGENT_LOG_FORMAT`
- `CLOACINA_AGENT_MAX_CONCURRENCY`
- `CLOACINA_AGENT_MEMORY`
- `CLOACINA_AGENT_MEMORY_LIMIT`
- `CLOACINA_AGENT_MEMORY_REQUEST`
- `CLOACINA_AGENT_NETWORK`
- `CLOACINA_AGENT_POOL`
- `CLOACINA_AGENT_SERVER_URL`
- `CLOACINA_ALERT_WEBHOOK_SECRET`
- `CLOACINA_API_KEY`
- `CLOACINA_AUTOSCALE`
- `CLOACINA_AUTOSCALE_COOLDOWN_S`
- `CLOACINA_AUTOSCALE_FLOOR`
- `CLOACINA_AUTOSCALE_IDLE_S`
- `CLOACINA_AUTOSCALE_INTERVAL_S`
- `CLOACINA_COMPILER_DEV_WORKSPACE`
- `CLOACINA_COMPILER_PYTHON_INDEX_URL`
- `CLOACINA_COMPILER_PYTHON_OFFLINE`
- `CLOACINA_COMPILER_PYTHON_PLATFORMS`
- `CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR`
- `CLOACINA_COMPILER_PYTHON_VERSION`
- `CLOACINA_COMPILER_PYTHON_WHEEL_DIR`
- `CLOACINA_COMPILER_VENDOR_DIR`
- `CLOACINA_DEFAULT_MAX_AGENTS`
- `CLOACINA_DEMO_TENANT_KEYS`
- `CLOACINA_FLEET_ACTUATOR`
- `CLOACINA_FLEET_DNS_NAMESPACE`
- `CLOACINA_FLEET_NETWORK_POLICY`
- `CLOACINA_INITIAL_AGENTS`
- `CLOACINA_LOGIN_THROTTLE_BASE_LOCK_S`
- `CLOACINA_LOGIN_THROTTLE_IP_THRESHOLD`
- `CLOACINA_LOGIN_THROTTLE_MAX_LOCK_S`
- `CLOACINA_LOGIN_THROTTLE_USER_THRESHOLD`
- `CLOACINA_OIDC_CLIENT_ID`
- `CLOACINA_OIDC_CLIENT_SECRET`
- `CLOACINA_OIDC_GROUP_CHECK_INTERVAL_S`
- `CLOACINA_OIDC_ISSUER`
- `CLOACINA_OIDC_MAP`
- `CLOACINA_OIDC_REDIRECT_URI`
- `CLOACINA_OIDC_SCOPES`
- `CLOACINA_OIDC_SESSION_MAX_AGE_S`
- `CLOACINA_OIDC_SUCCESS_REDIRECT`
- `CLOACINA_SERVER`
- `CLOACINA_SERVER_NAMESPACE`
- `CLOACINA_SERVER_POD_SELECTOR`
- `CLOACINA_SERVER_PORT`
- `CLOACINA_TENANT_COMPILERS`
- `CLOACINA_TENANT_COMPILER_BUILD_TIMEOUT_S`
- `CLOACINA_TENANT_COMPILER_CACHE_CLAIM`
- `CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE`
- `CLOACINA_TENANT_COMPILER_CACHE_CLAIM_STORAGE_CLASS`
- `CLOACINA_TENANT_COMPILER_CACHE_SIZE`
- `CLOACINA_TENANT_COMPILER_CARGO_FLAGS`
- `CLOACINA_TENANT_COMPILER_COOLDOWN_S`
- `CLOACINA_TENANT_COMPILER_CPU_LIMIT`
- `CLOACINA_TENANT_COMPILER_CPU_REQUEST`
- `CLOACINA_TENANT_COMPILER_DATABASE_CIDRS`
- `CLOACINA_TENANT_COMPILER_DATABASE_URL`
- `CLOACINA_TENANT_COMPILER_DEV_WORKSPACE`
- `CLOACINA_TENANT_COMPILER_EGRESS`
- `CLOACINA_TENANT_COMPILER_EGRESS_CIDRS`
- `CLOACINA_TENANT_COMPILER_HARDENED`
- `CLOACINA_TENANT_COMPILER_HOME_SIZE`
- `CLOACINA_TENANT_COMPILER_IMAGE`
- `CLOACINA_TENANT_COMPILER_MEMORY_LIMIT`
- `CLOACINA_TENANT_COMPILER_MEMORY_REQUEST`
- `CLOACINA_TENANT_COMPILER_POLL_INTERVAL_MS`
- `CLOACINA_TENANT_COMPILER_PYTHON_CACHE_SIZE`
- `CLOACINA_TENANT_COMPILER_PYTHON_INDEX_URL`
- `CLOACINA_TENANT_COMPILER_PYTHON_OFFLINE`
- `CLOACINA_TENANT_COMPILER_PYTHON_PLATFORMS`
- `CLOACINA_TENANT_COMPILER_PYTHON_UV_CACHE_DIR`
- `CLOACINA_TENANT_COMPILER_PYTHON_VERSION`
- `CLOACINA_TENANT_COMPILER_PYTHON_WHEEL_DIR`
- `CLOACINA_TENANT_COMPILER_TARGET_DIR`
- `CLOACINA_TENANT_COMPILER_VENDOR_DIR`
- `CLOACINA_TRUST_PROXY_HEADERS`
- `DOCKER_HOST`
- `KUBERNETES_SERVICE_HOST`
- `OTEL_EXPORTER_OTLP_ENDPOINT`
- `OTEL_SERVICE_NAME`
