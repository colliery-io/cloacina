---
title: "CLI Reference"
description: "Complete reference for cloacinactl: all nouns, verbs, flags, environment variables, exit codes, and configuration."
sidebar:
  order: 5
---

`cloacinactl` is the command-line interface for Cloacina. It manages
local services (daemon, server, compiler), talks to the HTTP API as a
client, and edits the local config file at `~/.cloacina/config.toml`.

The command shape is a strict **noun-verb** pattern, with a handful of
top-level singletons (`status`, `config`, `admin`, `completions`).

## Synopsis

```text
cloacinactl [GLOBAL FLAGS] <noun> <verb> [ARGS]
cloacinactl [GLOBAL FLAGS] <singleton> [ARGS]
```

## Global Flags

These flags apply to every subcommand.

| Flag | Default | Description |
|---|---|---|
| `-v`, `--verbose` | `false` | Enable debug-level logging on stderr. |
| `--home <PATH>` | `~/.cloacina` | Override the Cloacina home directory. Affects all home-relative paths (config, logs, packages, sockets, PID files, bootstrap key). |
| `--profile <NAME>` | (default profile from config) | Select a named profile from `~/.cloacina/config.toml`. Resolves the server URL and API key. |
| `--server <URL>` | (from profile) | Override the profile's server URL. Highest precedence among server-targeting flags. |
| `--api-key <KEY>` | (from profile) | Override the profile's API key. Supports the [API key schemes](#api-key-schemes) below. |
| `--tenant <NAME>` | `public` | Tenant name for tenant-scoped commands. Defaults to the admin "public" schema if unset. |
| `--json` | `false` | Shorthand for `-o json`. |
| `-o`, `--output <FORMAT>` | `table` | Output format: `table`, `json`, `yaml`, or `id`. |
| `--no-color` | `false` | Accepted, but currently a no-op — no code path reads it yet. |

## API Key Schemes

`--api-key` and the `api_key` profile field accept several schemes:

| Scheme | Example | Behavior |
|---|---|---|
| Raw | `clk_a1b2c3...` | The literal API key. |
| `env:VAR` | `env:CLOACINA_API_KEY` | Read the key from the named environment variable. Errors if the variable is unset or empty. |
| `file:PATH` | `file:/etc/cloacina/key` | Read the first non-empty line of the file. Whitespace is trimmed. |
| `keyring:NAME` | `keyring:prod` | Deferred; rejected today with a clear error message. |

## Output Formats

| Format | Behavior |
|---|---|
| `table` (default) | Human-readable aligned columns. Long strings are truncated with `…`. Auto-infers columns from the first object's keys; not ideal for deeply-nested resources. |
| `json` | Pretty-printed JSON, one document per response. |
| `yaml` | YAML output, one document per response. |
| `id` | One ID per line. Extracts the `id` or `name` field from each object. Useful in shell pipelines: `cloacinactl execution list -o id \| xargs -n1 cloacinactl execution status`. |

## Exit Codes

`cloacinactl` uses a fixed mapping (per ADR-0003 §6) so scripts can
distinguish failure modes:

| Code | Meaning |
|---|---|
| `0` | Success. |
| `1` | User error: bad flags, missing files, validation failure, local I/O failure. |
| `2` | Network/transport failure: unreachable daemon socket, HTTP connection refused. |
| `3` | Resource not found (HTTP 404). |
| `4` | Authentication failure (HTTP 401/403). |
| `5` | Server-side rejection (HTTP 4xx/5xx other than 404/401/403). |

## Environment Variables

| Variable | Used By | Description |
|---|---|---|
| `DATABASE_URL` | `server start`, `compiler start`, `admin cleanup-events` | PostgreSQL or SQLite connection URL. Overridden by the corresponding `--database-url` flag where applicable. |
| `CLOACINA_BOOTSTRAP_KEY` | `server start` | Pre-supplied bootstrap admin key. If unset and no live platform admin key exists, one is generated. Overridden by `--bootstrap-key`. |
| `CLOACINA_METRICS_BIND` | `server start` | Address of a separate listener for `/metrics`. Overridden by `--metrics-bind`. |
| `CLOACINA_REQUIRE_SIGNATURES` | `server start` | If `true`, the server enforces package signature verification at upload. Overridden by `--require-signatures`. |
| `RUST_LOG` | All commands | Log filter directive (e.g., `info`, `debug`, `cloacina=debug`). Overridden by `-v` / `--verbose`. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `server start` | If set, enables OpenTelemetry tracing to the named gRPC collector. |
| `OTEL_SERVICE_NAME` | `server start` | Service name in OpenTelemetry spans. Default: `cloacina`. |
| `CLOACINA_VAR_<NAME>` | Workflow context | Read by `cloacina::var("NAME")` / `var_or` in Rust and `cloaca.var("NAME")` in Python; the `CLOACINA_VAR_` prefix is mandatory. |
| `CLOACINA_DEFAULT_EXECUTOR` | `server start` | Executor key for dispatch (`fleet`, or `default` with single trust). Overridden by `--default-executor`; overrides `[server].default_executor`. |
| `CLOACINA_SINGLE_TRUST` | `server start` | `true`: single trust, tenant code runs in the server process. Same as `--single-trust`. |

---

# Service Commands

These commands manage local services. `daemon start` runs the daemon
**in the foreground, in-process** (there is no built-in daemonize);
`server start` and `compiler start` **exec** the corresponding binary
from `PATH`. All write a PID file under `--home` first, and `stop` /
`status` / `health` resolve that same `--home` — mixing `--home`
values between start and stop silently misses the process.

## `daemon`

The daemon is a lightweight local scheduler. It watches directories for
`.cloacina` packages, runs the reconciler, and registers cron + custom-poll
trigger schedules. State lives in `~/.cloacina/cloacina.db` (SQLite, WAL
mode). The daemon does not expose an HTTP API; it exposes a Unix-domain
health socket at `~/.cloacina/daemon.sock`.

### `daemon start`

```text
cloacinactl daemon start [--watch-dir <DIR>]... [--poll-interval <MS>]
```

| Flag | Default | Description |
|---|---|---|
| `--watch-dir <DIR>` | (none) | Additional package directory to watch. Repeatable. The default watch dir (`~/.cloacina/packages/`) is always included; CLI dirs and config dirs are merged and deduplicated. |
| `--poll-interval <MS>` | `500` | Reconciler fallback poll interval in milliseconds. The filesystem watcher provides immediate detection; this is the safety net. |
| `--log-retention-days <N>` | `14` | Number of daily-rotated log files to retain in `~/.cloacina/logs/`. `0` disables pruning. CLOACI-I-0109 / T-0592. |

**Behavior:**

1. Creates `~/.cloacina/{packages,logs}/` if missing.
2. Sets up dual logging: stderr (human-readable) + `~/.cloacina/logs/cloacina.log` (JSON, daily rotation).
3. Opens or creates `~/.cloacina/cloacina.db` (SQLite, WAL mode).
4. Loads `~/.cloacina/config.toml` if present.
5. Builds the merged watch-directory set (default + CLI + config).
6. Initializes a `DefaultRunner` (cron + trigger schedulers, intervals from config).
7. Initializes a `FilesystemWorkflowRegistry` across all watch dirs.
8. Performs an initial reconciliation (loads existing packages).
9. Starts the health socket at `~/.cloacina/daemon.sock`.
10. Enters the event loop: filesystem changes (debounced) trigger reconciliation; `--poll-interval` is the periodic fallback.

**Signals:**

| Signal | Behavior |
|---|---|
| SIGINT (Ctrl-C) | Graceful shutdown. Drains in-flight workflows up to `daemon.shutdown_timeout_s`. Exit 0. |
| SIGTERM | Same as SIGINT. |
| SIGHUP | Reload `config.toml`, update watch directories, trigger reconciliation. No restart. |
| Second SIGINT during shutdown | Force immediate exit. Exit 1. |

### `daemon stop`

```text
cloacinactl daemon stop [--force]
```

Reads the daemon's PID file and sends SIGTERM (or SIGKILL with
`--force`). Exit 0 if stopped cleanly; exit 2 if the PID file is
missing or the process is unreachable.

### `daemon status`

```text
cloacinactl daemon status
```

Connects to `~/.cloacina/daemon.sock` and prints a status table:
health, uptime, packages loaded, last reconciliation. Respects
`--no-color`.

### `daemon health`

```text
cloacinactl daemon health
```

Terse health probe via the Unix socket at `~/.cloacina/daemon.sock`
(or `<home>/daemon.sock` if `--home` is set). No output. Exit 0 if
up, 2 otherwise. Use this in scripts and supervisor checks.

## `server`

The HTTP API server (`cloacina-server`). Backed by PostgreSQL or
SQLite. Exposes the [HTTP API](../http-api/)
under the `/v1/` prefix and the unauth probes `/health`, `/ready`,
`/metrics`.

### `server start`

```text
cloacinactl server start [--bind <ADDR>] [--metrics-bind <ADDR>]
                        [--task-log-url-template <URL>]
                        [--database-url <URL>]
                        [--bootstrap-key <KEY>]
                        [--require-signatures] [--verification-org-id <UUID>]
                        [--reconcile-interval-s <N>]
                        [--tenant-runner-cache-size <N>]
                        [--tenant-deletion-drain-timeout-s <N>]
                        [--log-retention-days <N>]
                        [--default-executor <KEY>] [--single-trust]
                        [--database-wait-timeout-s <N>]
                        [--run-history-retention-days <N>]
                        [--outbox-acked-retention-s <N>]
                        [--retention-interval-s <N>]
                        [--fleet-task-timeout-s <N>]
                        [--fleet-task-timeout-max-s <N>]
                        [--alert-webhook-url <URL>]
                        [--alert-webhook-events <LIST>]
                        [--alert-webhook-timeout-s <N>]
                        [--alert-webhook-max-attempts <N>]
                        [--public-base-url <URL>]
                        [--log-format <text|json>]
                        [--agent-heartbeat-interval-s <N>]
                        [--agent-liveness-misses <N>]
                        [--fleet-artifact-fetch-wait-s <N>]
                        [--context-warn-bytes <N>]
                        [--context-max-bytes <N>]
                        [--cors-allowed-origins <LIST>]
                        [--cors-allowed-methods <LIST>]
                        [--cors-allowed-headers <LIST>]
                        [--max-package-bytes <N>]
                        [--max-unpacked-bytes <N>]
                        [--agent-key-lifetime-days <N>]
                        [--agent-key-rotate-days <N>]
```

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--bind <ADDR>` | | `127.0.0.1:8080` | Listen address. |
| `--metrics-bind <ADDR>` | `CLOACINA_METRICS_BIND` | (none) | Address of a separate listener for `/metrics`. When set, `/metrics` is served only there and `--bind` answers 404 for it. When not set, `/metrics` is on `--bind`. |
| `--task-log-url-template <URL>` | `CLOACINA_TASK_LOG_URL_TEMPLATE` | (none) | URL of a task's logs in your log stack, with the placeholders `{tenant}`, `{execution_id}`, `{task}` and `{attempt}`. The web UI shows it as a link on each task. Not set: no link. See [Task logs](#task-logs-cloacina-server). |
| `--database-url <URL>` | `DATABASE_URL` | (required) | Database connection URL. Examples: `sqlite:///path/to/cloacina.db`, `postgres://user:pass@host/dbname`. |
| `--bootstrap-key <KEY>` | `CLOACINA_BOOTSTRAP_KEY` | (auto-generated) | Pre-supplied platform admin key, used at a start with no live platform admin key. The plaintext is written to `~/.cloacina/bootstrap-key` with `0600` perms at each bootstrap, over an older file. A value that was used before cannot be used again. |
| `--require-signatures` | `CLOACINA_REQUIRE_SIGNATURES` | `false` | Enforce package signature verification at upload time. The signer must be a trusted key of the tenant (see [`trusted-key`](#trusted-key)), or of `--verification-org-id` if one is set. |
| `--verification-org-id <UUID>` | `CLOACINA_VERIFICATION_ORG_ID` | (none) | Optional server-wide trusted organization UUID. A key trusted in it is accepted for every tenant, after the tenant's own keys. With none, each tenant trusts only its own keys. |
| `--reconcile-interval-s <N>` | | (runtime default) | Interval (seconds) between reconciler passes that sync the in-runner workflow registry with the DB. Increase for quiet prod; decrease for fast e2e tests. |
| `--tenant-runner-cache-size <N>` | `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | `256` | Warning limit on per-tenant `DefaultRunner` instances. Every tenant has a runner from server start until the tenant is deleted; none is stopped at the limit. Above it the server logs a warning. Each tenant uses up to 2 database connections on each replica. CLOACI-T-0580, CLOACINA-T-1031. |
| `--tenant-deletion-drain-timeout-s <N>` | `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | `30` | Max seconds to wait for in-flight workflows to drain during tenant teardown (step 2 of the 4-step orchestration). Past this, the runner is hard-evicted; any task that ignored cooperative cancellation errors on its next DB write once the schema is dropped. CLOACI-T-0581. |
| `--log-retention-days <N>` | | `14` | Number of daily-rotated log files to retain. `0` disables pruning entirely. Log files only: database rows are covered by the retention settings below. CLOACI-I-0109 / T-0592. |
| `--cors-allowed-origins <LIST>` | `CLOACINA_CORS_ALLOWED_ORIGINS` | (none: CORS off) | Comma-separated origins allowed by CORS; `*` allows any. CORS is off until set. |
| `--cors-allowed-methods <LIST>` | `CLOACINA_CORS_ALLOWED_METHODS` | `GET,POST,DELETE,OPTIONS` | Comma-separated methods allowed by CORS, once origins are set. |
| `--cors-allowed-headers <LIST>` | `CLOACINA_CORS_ALLOWED_HEADERS` | `authorization,content-type` | Comma-separated request headers allowed by CORS, once origins are set. |

This subcommand was renamed from `serve` in an earlier release; older
docs may still mention the old name.

Retention of run history in the database has three settings:
`--run-history-retention-days` / `CLOACINA_RUN_HISTORY_RETENTION_DAYS`
(default `0`, off), `--outbox-acked-retention-s` /
`CLOACINA_OUTBOX_ACKED_RETENTION_S` (default `0`, off: acked
`delivery_outbox` rows are kept and the table grows) and
`--retention-interval-s` / `CLOACINA_RETENTION_INTERVAL_S` (default `300`).
See
[Run history retention](../../how-to/service/production-deployment/#run-history-retention).

`server start` has every flag of `cloacina-server`. Each setting below
`--default-executor` in the tables of this section (from
`--verification-org-id` and `--reconcile-interval-s` to
`--agent-key-rotate-days`) is taken in three ways: a flag, the env var of the
setting, or a key of `[server]` in `config.toml` with the flag's name in
snake case (for example `fleet_task_timeout_s = 600`,
`cors_allowed_origins = ["https://ops.example.com"]`). The order is flag, env
var, `config.toml`. A setting that none of them gives is not forwarded, and
the server uses its own default. `--reconcile-interval-s` and
`--log-retention-days` have no env var. The alert webhook URL goes to the
server as the env var `CLOACINA_ALERT_WEBHOOK_URL`, not as an argument. The
signing secret `CLOACINA_ALERT_WEBHOOK_SECRET` is an env var only; the server
inherits it. A test of `cloacinactl` compares the flags of `server start`
with those of `cloacina-server`.

`server start` **execs the `cloacina-server` binary from `PATH`**
(after writing `<home>/server.pid`). The install script installs only
`cloacinactl` — if `cloacina-server` is not separately installed
(container image or cargo), `server start` fails with "failed to exec
cloacina-server". The wrapper resolves `database_url`, `default_executor`
and the other keys of `[server]` from `config.toml` and forwards them.
Env vars that are not flags of the server (for example
`CLOACINA_FLEET_ACTUATOR`) reach it because the server process inherits the
environment of `server start`.

### Default executor (`cloacinactl server start` + `cloacina-server`)

Execution topology is a single deployment knob (CLOACI-T-0640). The
preferred way to set it is `[server].default_executor` in `config.toml`;
`cloacinactl server start` reads it and forwards `--default-executor` to
`cloacina-server` when it is set. The flag/env below override the config value
for ad-hoc runs. The key is hard-matched against the registered executors at
startup — an unknown key fails fast.

Without single trust the server runs no tenant code in its process
(CLOACINA-T-1075): it loads each package from the metadata document its
compiler stored, and every task goes to the fleet. `--default-executor
default` then stops the start with an error that names `--single-trust`. See
[Tenant code and the server process](../../explanation/service/security-model/#tenant-code-and-the-server-process).

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--default-executor <KEY>` | `CLOACINA_DEFAULT_EXECUTOR` | `fleet`; `default` with `--single-trust` | Executor every task is dispatched to. `fleet` sends it to the [execution-agent fleet](../../explanation/service/execution-agent-fleet/); `default` runs all work on the in-process thread executor and needs `--single-trust`. Forwarded by `cloacinactl server start` when set; also settable directly on `cloacina-server`. CLOACI-T-0640. |
| `--single-trust` | `CLOACINA_SINGLE_TRUST` | off | Every tenant is trusted as much as the server: packages load in the server process (libraries opened, Python modules imported) and `--default-executor default` is allowed. The server logs a warning at start. `cloacinactl server start` forwards it, also from `[server].single_trust = true`. |

### Task logs (`cloacina-server`)

The server stores no task output. These flags set the format of its own log
lines and the link from a task to your log stack. See
[Find a Task's Logs](../../how-to/service/find-a-tasks-logs/).

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--log-format <text\|json>` | `CLOACINA_LOG_FORMAT` | `text` | Format of the server's log lines on stderr. `json` writes one JSON object per line. The log file under `--home` is JSON in both. Forwarded by `cloacinactl server start` (`[server].log_format`). |
| `--task-log-url-template <URL>` | `CLOACINA_TASK_LOG_URL_TEMPLATE` | (none) | URL of a task's logs in your log stack. The placeholders `{tenant}`, `{execution_id}`, `{task}` and `{attempt}` are filled in for each task, percent-encoded. The task API then gives `log_url` and the web UI shows a link. Must start with `http://` or `https://`. Not set, or empty: no link. Forwarded by `cloacinactl server start`. |

### Fleet agent liveness (`cloacina-server`)

These flags tune the [execution-agent fleet](../../explanation/service/execution-agent-fleet/) and the task context. They live on the `cloacina-server` binary (and via the env vars below). `cloacinactl server start` takes each of them, and the key of the same name in snake case of `[server]` in `config.toml` (for example `agent_liveness_misses = 4`, `fleet_artifact_fetch_wait_s = 120`, `fleet_graph_firings = "worker"`).

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--agent-heartbeat-interval-s <N>` | `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `15` | Heartbeat interval (seconds) advertised to agents and used as the liveness-sweep cadence. Lower = faster dead-agent detection + in-flight reclaim, more heartbeat traffic. CLOACI-T-0639. |
| `--agent-liveness-misses <N>` | `CLOACINA_AGENT_LIVENESS_MISSES` | `3` | Consecutive missed heartbeats before an agent is declared dead and its in-flight work reclaimed. Effective dead-after = interval × misses (default 45s). CLOACI-T-0639. |
| `--fleet-task-timeout-s <N>` | `CLOACINA_FLEET_TASK_TIMEOUT_S` | `300` | Time limit (seconds) of a fleet task that sets none of its own, and of a graph firing sent to an agent. CLOACINA-T-1014. |
| `--fleet-task-timeout-max-s <N>` | `CLOACINA_FLEET_TASK_TIMEOUT_MAX_S` | `86400` | Longest time limit (seconds) a fleet task can get. A task that asks for more gets this value. Must not be below `--fleet-task-timeout-s`. CLOACINA-T-1014. |
| `--fleet-graph-firings <inline\|worker>` | `CLOACINA_FLEET_GRAPH_FIRINGS` | `inline` | Where an agent runs a computation-graph firing: `inline` in the agent process, `worker` in a worker process with the limits, time limit and cancel of a task. Any other value stops the server at start. CLOACINA-T-1125. |
| `--fleet-artifact-fetch-wait-s <N>` | `CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S` | `300` | How long (seconds) a fleet task waits, per attempt, while agents cannot fetch its package artifact. The task is dispatched again after a delay (2 s, growing to 30 s) and uses no retry attempt. After the limit the attempt fails. `0`: the first failed fetch fails the attempt. CLOACINA-T-1064. |
| `--fleet-poll-timeout-s <N>` | `CLOACINA_FLEET_POLL_TIMEOUT_S` | `30` | Time limit (seconds) of one poll of a poll trigger on an agent. The server waits this plus 10 s for the answer; with no answer it records the poll as missed and fires nothing. `0` stops the start. CLOACINA-T-1077. |
| `--priority-aging-interval-s <N>` | `CLOACINA_PRIORITY_AGING_INTERVAL_SECONDS` | `120` | The ageing interval of waiting work, in whole seconds. For each interval a task has waited, its [priority](../../explanation/priority/) in the order of waiting work improves by one, never past 0. `0` switches ageing off. A value that is not a whole number stops the start. CLOACINA-T-1123. |
| `--context-warn-bytes <N>` | `CLOACINA_CONTEXT_WARN_BYTES` | `1048576` | Size in bytes above which a task context is logged as a warning and counted in `cloacina_context_size_warnings_total`. `0` turns the warning off. CLOACINA-T-1047. |
| `--context-max-bytes <N>` | `CLOACINA_CONTEXT_MAX_BYTES` | `16777216` | Largest size in bytes of a task context. A task whose result or input context is larger fails with the size, the limit and the largest keys in the error, and is not retried. `0` turns the limit off. CLOACINA-T-1047. |
| `--agent-key-lifetime-days <N>` | `CLOACINA_AGENT_KEY_LIFETIME_DAYS` | `30` | Days an agent key that the Kubernetes actuator mints stays valid. After it the key is refused with `401` `key_expired`. Docker actuator keys do not expire. CLOACINA-T-1083. |
| `--agent-key-rotate-days <N>` | `CLOACINA_AGENT_KEY_ROTATE_DAYS` | `10` | Days left on an agent key at which the Kubernetes actuator replaces it: a new key in the Secret, and every pool's pods roll onto it. The old key is revoked when every pool's rollout is complete. At least `1` and less than `--agent-key-lifetime-days`, or the server does not start. See [Key expiry](../../explanation/service/security-model/#key-expiry). CLOACINA-T-1083. |

### Alert webhook (`cloacina-server`)

These flags send a JSON `POST` when a run ends. They live on the `cloacina-server` binary (and via the env vars below). `cloacinactl server start` takes the same flags and env vars, and the keys `alert_webhook_url`, `alert_webhook_events`, `alert_webhook_timeout_s`, `alert_webhook_max_attempts` and `public_base_url` of `[server]` in `config.toml`. See [Alert on a Failed Run](../../how-to/service/alert-on-a-failed-run/).

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--alert-webhook-url <URL>` | `CLOACINA_ALERT_WEBHOOK_URL` | — | URL that gets the `POST`. Not set = off. |
| `--alert-webhook-events <LIST>` | `CLOACINA_ALERT_WEBHOOK_EVENTS` | `failed` | Outcomes that send an alert, comma-separated: `failed`, `completed`, `cancelled`, or `all`. |
| — | `CLOACINA_ALERT_WEBHOOK_SECRET` | — | Shared secret that signs each request (`X-Cloacina-Signature`). Environment only. |
| `--alert-webhook-timeout-s <N>` | `CLOACINA_ALERT_WEBHOOK_TIMEOUT_S` | `10` | Seconds allowed for one request. |
| `--alert-webhook-max-attempts <N>` | `CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS` | `8` | The most sends of one alert before it is dropped. |
| `--public-base-url <URL>` | `CLOACINA_PUBLIC_BASE_URL` | — | Public URL of the server, for the absolute links in an alert. |
| `--database-wait-timeout-s <N>` | `CLOACINA_DATABASE_WAIT_TIMEOUT_S` | `60` | Seconds `cloacina-server` waits at start for the database to accept a connection. It tries again with a growing delay (0.5 s, doubling, at most 5 s) and logs `Waiting for the database to accept connections` for each wait. After this time it exits with an error that holds the last connection error. `0` = one try, no wait. The HTTP listener opens after the database answers and the migrations ran, so `/health` and `/ready` do not answer during the wait. `cloacinactl server start` takes the same flag and env var, and the key `database_wait_timeout_s` of `[server]` in `config.toml`. |
| `--max-package-bytes <N>` | `CLOACINA_MAX_PACKAGE_BYTES` | `104857600` | The largest package archive, in bytes, that an upload accepts (100 MiB). A larger upload gets status 413 with the code `package_too_large` and an error that names the size of the upload and the limit. `cloacinactl server start` takes the same flag and env var, and the key `max_package_bytes` of `[server]`. CLOACINA-T-1048. |
| `--max-unpacked-bytes <N>` | `CLOACINA_MAX_UNPACKED_BYTES` | `524288000` | The largest size, in bytes, that a package archive may unpack to (500 MiB). The server refuses an upload that unpacks to more, and its package loader refuses to load one. The limits of 10,000 entries and of 10 times the archive size stay. `cloacina-compiler`, `cloacina-agent` and `cloacinactl package validate`, `pack` and `publish` take the same flag and variable; `cloacinactl daemon` reads the variable. Give each of them the same value. `cloacinactl server start` takes the same flag and env var, and the key `max_unpacked_bytes` of `[server]`; `cloacinactl compiler start` the key `max_unpacked_bytes` of `[compiler]`. CLOACINA-T-1048. |

### `server stop` / `status` / `health`

Same shape as the daemon equivalents, but use the server's PID file
for `stop` and HTTP probes for the rest: both `status` and `health`
hit `GET /health` on the resolved profile server (`status` prints
"server: unconfigured" when no profile/`--server` is set; `health`
exits 0 if up, 2 otherwise).

## `compiler`

The compilation service (`cloacina-compiler`). Polls the database for
uploaded packages with `build_status = pending` and compiles their
source into cdylib artifacts stored back in the database. Like
`server start`, `compiler start` **execs the `cloacina-compiler`
binary from `PATH`** (after writing `<home>/compiler.pid`).

### `compiler start` / `stop` / `status` / `health`

```text
cloacinactl compiler start [--bind <ADDR>] [--metrics-bind <ADDR>]
                          [--database-url <URL>]
                          [--poll-interval-ms <N>]
                          [--heartbeat-interval-s <N>]
                          [--stale-threshold-s <N>]
                          [--sweep-interval-s <N>]
                          [--max-unpacked-bytes <N>]
                          [--python-platform <ARCH-OS>]...
                          [--python-version <MAJOR.MINOR>]
                          [--python-index-url <URL>]
                          [--python-wheel-dir <DIR>]
                          [--python-uv-cache-dir <DIR>]
                          [--python-offline]
                          [--uv <PATH>]
                          [--tenant-schema <SCHEMA>]
                          [--build-target <TRIPLE>]
                          [--build-target-package <NAME>]
                          [--cargo-flag <FLAG>]... | [--cargo-flags-replace <FLAG>]...
                          [--cargo-target-dir <DIR>]
                          [--vendor-dir <DIR>]
                          [--dev-workspace <DIR>]
                          [--build-rlimit-cpu <SECONDS>]
                          [--build-rlimit-mem <SIZE>]
                          [--build-rlimit-files <N>]
                          [--build-rlimit-procs <N>]
                          [--log-retention-days <DAYS>]
```

| Flag | Default | Description |
|---|---|---|
| `--bind <ADDR>` | `127.0.0.1:9000` | Listen address. |
| `--metrics-bind <ADDR>` | (none) | Address of a separate listener for `/metrics` (env `CLOACINA_COMPILER_METRICS_BIND`). When set, `/metrics` is served only there and `--bind` answers 404 for it. Forwarded to `cloacina-compiler` only when set. |
| `--database-url <URL>` | from `DATABASE_URL` | Connection URL (Postgres or SQLite). |
| `--poll-interval-ms` | (binary default) | How often to poll for pending build rows. |
| `--heartbeat-interval-s` | (binary default) | Worker heartbeat cadence. |
| `--stale-threshold-s` | (binary default) | Seconds before an in-flight build is considered stale and reclaimable. |
| `--sweep-interval-s` | (binary default) | How often to sweep for stale claims. |
| `--max-unpacked-bytes <N>` | `524288000` | The largest size a package archive may unpack to (env `CLOACINA_MAX_UNPACKED_BYTES`). |
| `--python-platform <ARCH-OS>` | `aarch64-linux`, `x86_64-linux` | A platform to vendor Python dependencies for; repeat it or give a comma-separated list (env `CLOACINA_COMPILER_PYTHON_PLATFORMS`). |
| `--python-version <MAJOR.MINOR>` | `3.11` | The Python to vendor for (env `CLOACINA_COMPILER_PYTHON_VERSION`). |
| `--python-index-url <URL>` | PyPI | The package index of the wheels (env `CLOACINA_COMPILER_PYTHON_INDEX_URL`). Given to `cloacina-compiler` as that env var, not as an argument, because the URL can hold a password. |
| `--python-wheel-dir <DIR>` | (none) | A folder of wheel files (env `CLOACINA_COMPILER_PYTHON_WHEEL_DIR`). |
| `--python-uv-cache-dir <DIR>` | `uv`'s own | The cache folder of `uv` (env `CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR`). |
| `--python-offline` | off | Vendor with no network (env `CLOACINA_COMPILER_PYTHON_OFFLINE`). |
| `--uv <PATH>` | `uv` on `PATH` | The `uv` program (env `CLOACINA_UV`). |
| `--tenant-schema <SCHEMA>` | (none: `public`) | Build only this tenant's packages; run one compiler per tenant (env `CLOACINA_TENANT_SCHEMA`). |
| `--build-target <TRIPLE>` | (none: the primary compiler) | Run as a per-target compiler that builds for this triple (env `CLOACINA_BUILD_TARGET`). |
| `--build-target-package <NAME>` | (none) | With `--build-target`: build only this package (env `CLOACINA_BUILD_TARGET_PACKAGE`). |
| `--cargo-flag <FLAG>` | (none) | A cargo flag added to the default list; repeat it. Write a value that starts with `-` as `--cargo-flag=<value>`. Not with `--cargo-flags-replace`. |
| `--cargo-flags-replace <FLAG>` | (none) | The whole cargo flag list, defaults replaced, starting with the subcommand (usually `build`); repeat it. |
| `--cargo-target-dir <DIR>` | the package's own `target/` | A shared cargo target folder for package builds. |
| `--vendor-dir <DIR>` | `~/.cargo` | `CARGO_HOME` of the cargo build: a vendored cargo home (env `CLOACINA_COMPILER_VENDOR_DIR`). |
| `--dev-workspace <DIR>` | (none) | Dev only: a local cloacina workspace that builds patch to (env `CLOACINA_COMPILER_DEV_WORKSPACE`). |
| `--build-rlimit-cpu <SECONDS>` | (none) | `RLIMIT_CPU` of the cargo build, Linux (env `CLOACINA_COMPILER_BUILD_RLIMIT_CPU`). |
| `--build-rlimit-mem <SIZE>` | `4G` | `RLIMIT_AS` of the cargo build: bytes, or a size with `K`, `M` or `G`; Linux (env `CLOACINA_COMPILER_BUILD_RLIMIT_MEM`). |
| `--build-rlimit-files <N>` | `1024` | `RLIMIT_NOFILE` of the cargo build, Linux (env `CLOACINA_COMPILER_BUILD_RLIMIT_FILES`). |
| `--build-rlimit-procs <N>` | `256` | `RLIMIT_NPROC` of the cargo build, Linux (env `CLOACINA_COMPILER_BUILD_RLIMIT_PROCS`). |
| `--log-retention-days <DAYS>` | `14` | Daily log files to keep; `0` keeps all. |

Each of these settings, but `--bind`, `--metrics-bind`, `--database-url` and
the four intervals, is taken in three ways: a flag, the env var where the
compiler has one, or a key of `[compiler]` in `config.toml`
(`max_unpacked_bytes`, `python_platforms`, `python_version`,
`python_index_url`, `python_wheel_dir`, `python_uv_cache_dir`,
`python_offline`, `uv`, `tenant_schema`, `build_target`,
`build_target_package`, `cargo_flags`, `cargo_flags_replace`,
`cargo_target_dir`, `vendor_dir`, `dev_workspace`, `build_rlimit_cpu`,
`build_rlimit_mem`, `build_rlimit_files`, `build_rlimit_procs`,
`log_retention_days`). The order is flag, env var, `config.toml`; a setting
that none of them gives is not forwarded, and `cloacina-compiler` uses its
default. Cargo flag additions and a replacement cannot both be given, by
flags or by keys: `compiler start` stops with a message that names both. See
[Run cloacina-compiler in Production](../../how-to/service/running-the-compiler/)
for what each setting does.

Two flags of `cloacina-compiler` are not flags of `compiler start`: its time
limits `--build-timeout-s` and `--describe-timeout-s`. Set them through
their env vars (`CLOACINA_COMPILER_BUILD_TIMEOUT_S`,
`CLOACINA_COMPILER_DESCRIBE_TIMEOUT_S`), which `cloacina-compiler` inherits,
or run `cloacina-compiler` itself. A test of `cloacinactl` holds this list: a
new flag of `cloacina-compiler` must be added to `compiler start` or to it.

`cloacina-compiler` vendors the dependencies of a Python package that is
published with a `requirements.lock` and no `vendor/` folder. Its settings
(`cloacinactl compiler start` takes each one, as above):

| Flag | Env | Default | Description |
|---|---|---|---|
| `--python-platform <ARCH-OS>` | `CLOACINA_COMPILER_PYTHON_PLATFORMS` | `aarch64-linux`, `x86_64-linux` | A platform to vendor for. Repeatable, or a comma-separated list. |
| `--python-version <MAJOR.MINOR>` | `CLOACINA_COMPILER_PYTHON_VERSION` | `3.11` | The Python the wheels are for. |
| `--python-index-url <URL>` | `CLOACINA_COMPILER_PYTHON_INDEX_URL` | PyPI | The package index. |
| `--python-wheel-dir <DIR>` | `CLOACINA_COMPILER_PYTHON_WHEEL_DIR` | unset | A folder of wheel files: one more place to find wheels, next to the index. With `--python-offline`, the only source. |
| `--python-uv-cache-dir <DIR>` | `CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR` | `uv`'s own | The cache folder of `uv`. |
| `--python-offline` | `CLOACINA_COMPILER_PYTHON_OFFLINE` | off | No network: every wheel comes from the wheel folder, or else from the `uv` cache. |
| `--uv <PATH>` | `CLOACINA_UV` | `uv` on `PATH` | The `uv` program. |

See [Running the Compiler](../../how-to/service/running-the-compiler/#python-packages-vendoring-dependencies).

`stop` / `status` / `health` follow the same pattern as `server`: PID
file for stop, HTTP `GET /v1/status` for status, HTTP `GET /health`
for health. Note: `compiler status` / `health` probe the address in
`[compiler].local_addr` from `config.toml` (default `127.0.0.1:9000`),
**not** the `--bind` you passed to `compiler start` — if you started
the compiler on a non-default bind, set `compiler.local_addr` to match
or status will report unreachable.

## `agent`

The execution-agent (`cloacina-agent`) is a **DB-less** worker that joins a
server's [fleet](../../explanation/service/execution-agent-fleet/). It
registers over REST, opens the delivery WebSocket, fetches compiled workflow
cdylibs by digest, executes the task in-process, and reports the result back —
holding no database connection of its own. It is a standalone binary, not a
`cloacinactl` subcommand.

```text
cloacina-agent --server <URL> --api-key <KEY>
               [--agent-id <ID>] [--max-concurrency <N>] [--pool <NAME>]
               [--capabilities <TAG,TAG>] [--target-triple-override <TRIPLE>]
               [--cache-dir <PATH>] [--max-unpacked-bytes <N>]
               [--log-format <text|json>]
```

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--server <URL>` | `CLOACINA_SERVER` | (required) | Base URL of the server to register with (REST + WS ticket mint). |
| `--api-key <KEY>` | `CLOACINA_API_KEY` | (required, or `--api-key-file`) | API key for REST + WS auth. Its tenant scope determines which tenants' work the agent may receive (REQ-008). The agent removes the variable from its environment at start. |
| `--api-key-file <PATH>` | `CLOACINA_API_KEY_FILE` | | File that holds the API key, in place of `--api-key`. Giving both is an error. |
| `--ca-file <PATH>` | `CLOACINA_AGENT_CA_FILE` | | PEM file of CA certificates the agent trusts for an `https://` server, in addition to the system roots: for a mesh or gateway whose certificate a private CA signs. A file that cannot be read or holds no certificate stops the agent at start. See [Transport Security](../../how-to/service/transport-security/). |
| `--agent-id <ID>` | | (server-assigned) | Optional caller-chosen agent id. If omitted, the server assigns one. |
| `--max-concurrency <N>` | `CLOACINA_AGENT_MAX_CONCURRENCY` | `4` | Max work packets the agent runs concurrently. The server's capacity-aware selection won't exceed this; a saturated agent refuses further packets. |
| `--pool <NAME>` | `CLOACINA_AGENT_POOL` | (none) | The [agent pool](../../explanation/service/execution-agent-fleet/#agent-pools) the agent is in. Registered as the capability `pool:<name>`. Not set: the server counts the agent in `default`. A task goes only to agents of its own pool; a task with no pool goes to `default`. |
| `--cpu <QTY>` | `CLOACINA_AGENT_CPU` | (none) | Total CPU the agent offers to tasks, a Kubernetes quantity above zero (`4`, `500m`). Reported at registration; the server [places tasks](../../explanation/service/execution-agent-fleet/#how-a-task-is-placed) by their CPU requests against it. Not set: the agent reports none and is packed by `--max-concurrency` only. A value that is not a quantity stops the agent at start. The agent does not cap a task's CPU by it. |
| `--memory <QTY>` | `CLOACINA_AGENT_MEMORY` | (none) | Total memory of the agent, a Kubernetes quantity above zero (`32Gi`, `512Mi`): the memory limit of its pod. The agent reports it less `--reserved-memory` at registration, as what it offers to tasks. A task's memory request is its limit; a task with no request gets the offered memory divided by `--max-concurrency`. Not set: the agent reports none, and uses the memory limit of its own cgroup, when it has one, for the task limits. See [worker processes and limits](../../explanation/service/execution-agent-fleet/#worker-processes-and-limits). |
| `--reserved-memory <QTY>` | `CLOACINA_AGENT_RESERVED_MEMORY` | `128Mi` | The agent's own share of its memory, a Kubernetes quantity or `0`. Taken off `--memory` before the rest is offered to tasks. An agent whose memory is not above its share does not start. The server's publish check assumes the default. |
| `--task-address-space-factor <N>` | `CLOACINA_AGENT_TASK_ADDRESS_SPACE_FACTOR` | `8` | The address-space ceiling of a task's worker process (`RLIMIT_AS`, Linux): the task's memory limit times this, plus 64 GiB. A backstop behind the memory watchdog. `0`: no ceiling. |
| `--warm-workers <N>` | `CLOACINA_AGENT_WARM_WORKERS` | the value of `--max-concurrency` | The most idle [warm workers](../../explanation/service/execution-agent-fleet/#warm-workers) the agent keeps: worker processes that keep a package version loaded for the next task or graph firing of the same package version, tenant and memory limit. When one more comes, the oldest ends. `0`: none; each job gets a new worker. |
| `--warm-worker-idle-seconds <N>` | `CLOACINA_AGENT_WARM_WORKER_IDLE_SECONDS` | `60` | A warm worker idle for longer is ended. |
| `--warm-worker-max-jobs <N>` | `CLOACINA_AGENT_WARM_WORKER_MAX_JOBS` | `100` | The most jobs one worker process runs; it ends after the last. `1` turns warm workers off. |
| `--warm-worker-max-memory-percent <N>` | `CLOACINA_AGENT_WARM_WORKER_MAX_MEMORY_PERCENT` | `50` | A worker that holds more than this share of its jobs' memory limit after a job is ended and not used again. |
| `--capabilities <TAG,TAG>` | | (none) | Free-form capability tags advertised at registration. |
| `--target-triple-override <TRIPLE>` | | (host triple) | Override the advertised host target triple. Rarely needed — the server only dispatches a cdylib built for the agent's triple (OQ-6 fail-closed), so this is mainly for testing that path. |
| `--cache-dir <PATH>` | `CLOACINA_AGENT_CACHE_DIR` | `<TMPDIR>/cloacina-agent-cache` | Where fetched cdylibs are cached by digest; a cache hit skips the REST fetch. |
| `--max-unpacked-bytes <N>` | `CLOACINA_MAX_UNPACKED_BYTES` | `524288000` | The largest size, in bytes, that a package or provider archive may unpack to (500 MiB). The agent refuses to load a larger one. Use the value of the server. |
| `--log-format <text\|json>` | `CLOACINA_AGENT_LOG_FORMAT` | `text` | Format of the agent's log lines on stdout. `json` writes one JSON object per line; a line written while a task runs has the task's ids under `span`. See [Find a Task's Logs](../../how-to/service/find-a-tasks-logs/). |

The agent exposes no HTTP surface of its own — observe it via its logs and the
server's fleet metrics (`cloacina_fleet_*`). See the how-to guide
[Deploy an execution-agent fleet](../../how-to/service/deploy-an-execution-agent-fleet/).

---

# Client Commands

These commands talk to a running `cloacina-server` over HTTP. They
respect the global `--profile` / `--server` / `--api-key` /
`--tenant` flags. All require authentication except where noted.

## `package`

### `package new <NAME> [--lang <LANG>] [--kind <KIND>] [--path <DIR>]`

Scaffolds a canonical package source tree. Local-only.

| Flag | Default | Description |
|---|---|---|
| `--lang <python\|rust>` | `python` | Source language to scaffold. |
| `--kind <workflow\|graph\|cron>` | `workflow` | Package shape. All three kinds scaffold for both languages; `cron` emits a workflow bound to a cron trigger (`#[trigger(on, cron)]` in Rust, `@cloaca.trigger(on=..., cron=...)` in Python). |
| `--path <DIR>` | `./<name>` | Directory to create. |

### `package build <DIR> [--release] [--vendor] [--python-version <V>] [--arch <ARCH>] [--platform <ARCH-OS>]... [--locked] [--no-lock]`

Local-only; does not contact the server. Exits 1 on missing files or a
failed build. What it does depends on `[metadata].language`.

**Rust.** Runs `cargo build` in `<DIR>` (must contain a `Cargo.toml` and
`package.toml`). With `--release`, builds the release profile. Default
is debug. The other flags do nothing.

**Python.** Locks the dependencies of the package. It reads
`[project].dependencies` of `<DIR>/pyproject.toml` and writes the pinned
versions and hashes to `<DIR>/requirements.lock`. It does not fill
`vendor/`: the compiler vendors the dependencies of the lock file when
the package is published. With `--vendor` it also installs the wheels
of each target platform into `<DIR>/vendor/<platform>/`, and records
the Python version and the platforms in `[metadata.vendored]` of
`package.toml`; the package then ships its dependencies and the
compiler uses it as it is. With no `pyproject.toml` or no dependencies
it prints that there is nothing to lock and exits 0. It needs `uv` on
`PATH` (or its path in `CLOACINA_UV`), except with `--locked` and no
`--vendor`.

| Flag | Default | Description |
|---|---|---|
| `--vendor` | off | Also fill `vendor/<platform>/` and write `[metadata.vendored]`. Cannot be used with `--no-lock`. |
| `--python-version <MAJOR.MINOR>` | `3.11` | The Python the dependencies are locked for, and with `--vendor` vendored for. |
| `--arch <x86_64\|aarch64\|both>` | `both` | With `--vendor`: the CPU architecture of the Linux agents: `x86_64-linux`, `aarch64-linux`, or both. Cannot be used with `--platform`. |
| `--platform <ARCH-OS>` | — | A platform to vendor for. Needs `--vendor`. Repeatable. One of `aarch64-linux`, `x86_64-linux`, `aarch64-macos`, `x86_64-macos`, `x86_64-windows`. Replaces the platforms of `--arch`. |
| `--locked` | off | Use the existing `requirements.lock` as it is. Without it the versions are resolved again and the lock file is written again. |
| `--no-lock` | off | Do not read `pyproject.toml`, write no lock file and do not touch `vendor/`. |

The build fails when: `uv` is not found (the error says how to install
it); `requires_python` excludes `--python-version`; `--locked` is given
and there is no `requirements.lock`. Without `--vendor` it also fails
when the lock file is one the compiler would refuse (it holds an
option, a URL or a path, or does not pin a dependency of
`pyproject.toml`), and when `requirements.lock` changed while `vendor/`
holds the folders of an earlier `--vendor` build. With `--vendor` it
also fails when a dependency has no wheel for a target (the error names
it), and when `vendor/` holds an entry that is not a platform folder.

### `package validate <PATH> [--python-version <V>] [--max-unpacked-bytes <N>]`

Validates a package — a source directory or a `.cloacina` archive —
against the canonical format without uploading. Checks the resolved
manifest (closed schema), the language layout (Rust
`Cargo.toml`+`src/lib.rs` vs Python `workflow/` tree), and lints
known footguns (unrewritten `__WORKSPACE__` placeholders,
`#[computation_graph]` without `graph_name`, a cron trigger duplicated
in `#[workflow(triggers = ...)]`). Local-only.

For a Python package it also checks the Python environment:
`requires_python` must allow the Python that runs the package, the
vendored dependencies must be for the same `major.minor`, and each
platform of `[metadata.vendored]` must have its `vendor/<platform>/`
folder. The Python that runs the package is `--python-version` when
given, else `[metadata.vendored].python_version`, else `3.11`.

It also checks that the dependencies ship. A package with a `vendor/`
folder passes. Without one: dependencies in `pyproject.toml` need a
`requirements.lock`, the lock file must pin each of them, and it may
hold only pinned versions with hashes. These are the checks of the
compiler; `package pack` runs them too.

A package that unpacks to more than `--max-unpacked-bytes` /
`CLOACINA_MAX_UNPACKED_BYTES` (default `524288000`, 500 MiB) is refused:
a source directory with a `vendor/` folder by the size of its files, an
archive when it is unpacked. Give the value of the server and the agents.
`package pack` and `package publish` take the same flag.

### `package pack <DIR> [--out <PATH>] [--sign <KEY_FILE>] [--max-unpacked-bytes <N>]`

Packs the source directory into a `.cloacina` archive (tar + bzip2
**source** archive — no compiled artifacts). `--out` defaults to
`<dir>/<name>.cloacina`. Runs the same validation as
`package validate` first. The path of the archive is the one line on
stdout.

`--sign <KEY_FILE>` signs the archive and writes the signature beside
it as `<archive>.sig` (for `demo.cloacina`: `demo.cloacina.sig`). The
path of the signature file goes to stderr. The signature is Ed25519
over the SHA-256 of the archive; the file is JSON with the fields
`version`, `algorithm`, `package_hash`, `key_fingerprint`, `signature`
and `signed_at`, and holds nothing secret.

The key file is an unencrypted Ed25519 private key in PKCS#8 PEM, as
`openssl genpkey -algorithm ed25519` writes it. On Unix a key file that
the group or others can read is refused: give it mode `600`. The key is
read on this machine and is sent nowhere. A missing file, a file with
wider permissions and a file that is no such key each exit 1 and write
no signature.

### `package publish <DIR> [--release] [--sign <KEY_FILE>] [--vendor] [--python-version <V>] [--arch <ARCH>] [--platform <ARCH-OS>]... [--locked] [--no-lock] [--max-unpacked-bytes <N>]`

`build` → `pack` → `upload` in one shot. It takes the flags of
`package build`: a publish of a Python package writes its lock file,
and the compiler vendors the dependencies. Pass `--vendor` to ship the
dependencies in the package. Pass `--no-lock` for a package whose
`vendor/` is filled by hand.

`--sign <KEY_FILE>` signs the archive that was built and sends the
signature with the upload. The key file is the one of `package pack
--sign`; it is checked before the build starts. Without `--sign` the
upload has no signature, and a server that requires signatures refuses
it with `403 signature_required`.

### `package upload <FILE> [--signature <SIG_FILE>]`

Multipart POST of a `.cloacina` archive to
`/v1/tenants/<tenant>/workflows`. Requires a resolved server + API key
(profile or flags). Exit codes: 1 (user error), 2 (network), 4 (auth),
5 (server reject).

When `<FILE>.sig` is there (written by `package pack --sign`), it is
sent with the archive as the `signature` field. `--signature
<SIG_FILE>` sends another signature file in its place. A signature file
that was made for other bytes than `<FILE>` is refused before the
upload (exit 1). With no signature file the upload has no signature.

The server checks a signature that is sent, and refuses the upload when
the check fails: `untrusted_signer`, `signer_revoked`,
`invalid_signature`, `package_tampered`, `malformed_signature`. A
server started with `--require-signatures` also refuses an upload with
no signature (`signature_required`). See
[Require signed packages](../../how-to/service/require-signed-packages/#how-an-upload-is-checked).

The version that is in service stays active until the build of the
uploaded one succeeds. A failed build leaves it in service; see
`package versions`.

The server keeps who uploaded the package. Its runs are held to that
principal's [priority](../../explanation/priority/#the-limit-of-a-publisher-and-of-a-creator)
limit at each run start. When the limit is not 0 the answer has a
`priority_notice`, and `package upload` and `package publish` print it on
stderr as `warning: …`.

### `package list [--filter <SUBSTRING>]`

`GET /v1/tenants/<tenant>/workflows`. The `--filter` is applied
client-side as a substring match on the package name.

### `package inspect <ID>`

`GET /v1/tenants/<tenant>/workflows/<id>`. Prints a single object
respecting `--output`.

### `package versions <NAME>`

`GET /v1/tenants/<tenant>/workflows/<name>/versions`. Lists every stored
version of a package, newest upload first, with a `STATE` column:
`active`, `pending`, `building`, `failed`, `superseded` (was active
earlier) or `abandoned` (replaced before it was built). `--output json`
prints the raw rows.

### `package activate <NAME> <VERSION>`

`POST /v1/tenants/<tenant>/workflows/<name>/versions/<version>/activate`.
Makes an earlier built version the active one, with no rebuild: this is
the way back after a bad release. Executions that already run stay on
the version they started with. Exit code 5 when the version does not
exist or was never built.

```bash
cloacinactl package versions etl_pipeline
cloacinactl package activate etl_pipeline 1.2.0
```

### `package delete <ID> [--force]`

Resolves the package via `GET /v1/tenants/<tenant>/workflows/<id>`,
then issues
`DELETE /v1/tenants/<tenant>/workflows/<name>/<version>`. Interactive
confirmation unless `--force`.

## `constructor`

Packages a `#[constructor]` **provider** crate into a `.cloacina`
provider archive (CLOACI-T-0827). Providers are the plugin crates that
`constructor!(from = "provider@version", ...)` nodes resolve against.
Local-only (builds with cargo; no server contact).

### `constructor package <CRATE-DIR>`

```text
cloacinactl constructor package <CRATE-DIR> [--out <PATH>]
    [--sign-key <FILE>] [--manifest-bin <NAME>] [--debug] [--native]
```

| Flag | Default | Description |
|---|---|---|
| `--out <PATH>` | `<name>-<version>.cloacina` in CWD | Output archive path. |
| `--sign-key <FILE>` | (none) | Ed25519 secret key (32 raw bytes) used to sign the provider archive (`package.sig`). This provider-side signing scheme is distinct from the workflow-package `--sign`, which takes a PKCS#8 PEM key file and writes a `.sig` file. |
| `--manifest-bin <NAME>` | `emit_manifest` | Name of the provider crate's `[[bin]]` that prints the generated provider manifest JSON. |
| `--debug` | off | Build the component in debug profile (default is release). |
| `--native` | off | Package a **native host cdylib** provider instead of a sandboxed wasm32-wasip2 component (CLOACI-T-0903). Native providers run trusted and unsandboxed in-process — capability grants are advisory, not enforced. The crate must declare a `native` cargo feature plus fidius-core/fidius-macro host deps. The command prints the trust tier to stderr. |

## `workflow`

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `workflow list [--package <FILTER>]` | `GET /v1/tenants/<tenant>/workflows` | Client-side `--package` substring filter on the package name. |
| `workflow inspect <NAME>` | `GET /v1/tenants/<tenant>/workflows/<name>` | Full workflow metadata: tasks, dependencies, trigger rules, schedules. |
| `workflow run <NAME> [--context <SOURCE>] [--priority <0-10>]` | `POST /v1/tenants/<tenant>/workflows/<name>/execute` | `--context` accepts a path to a JSON file or `-` for stdin. Defaults to `{}`. JSON is validated before submission. `--priority` asks for a workflow [priority](../../explanation/priority/) for this run: 0 is the highest, 10 the lowest; with none the run gets what the workflow declares, or 5. A priority above what the key may set is lowered to the key's limit, and the command prints `warning: priority N was asked for and priority M was given …` on stderr. Prints the execution ID. |

## `execution`

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `execution list [--workflow <F>] [--status <S>] [--limit <N>] [--offset <N>]` | `GET /v1/tenants/<tenant>/executions?status=…&workflow=…&limit=…&offset=…` | Default limit: 50, server cap 1000; default offset 0. `--status` and `--workflow` map to the server query params of the same names (CLOACI-T-0594 / API-02). |
| `execution status <ID>` | `GET /v1/tenants/<tenant>/executions/<id>` | Returns Pending / Running / Completed / Failed / Cancelled / Paused. |
| `execution events <ID> [--since <DURATION>] [--follow]` | `GET /v1/tenants/<tenant>/executions/<id>/events?since=<dur>` | `--follow` streams live events over the server's WebSocket delivery substrate (CLOACI-I-0115) until interrupted. `--since` cannot be combined with `--follow` (cursor support is future work); use `--since` on a non-follow call for the historical snapshot. |

## `graph`

Per CLOACI-S-0011, `graph` is the current inspection noun for
computation graphs (*graph* is the unit of scheduling; a *reactor* is
a specialized trigger node inside it). The `reactor` noun below
survives **only** for operator manual fire — docs or scripts using
`reactor` as the inspection noun are stale.

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `graph list` | `GET /v1/health/graphs` | Lists loaded computation graphs with health + reactor pause state. |
| `graph status <NAME>` | `GET /v1/health/graphs/<name>` | Single graph's health + accumulators + reactor pause state. |
| `graph accumulators` | `GET /v1/health/accumulators` | Lists all accumulators across all graphs. |

## `reactor`

Operator manual-fire actions on reactors loaded in the server's graph
scheduler (CLOACI-T-0751). Typed JSON inputs are serialized to the
boundary encoding server-side, so you never hand-craft raw boundary
bytes.

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `reactor force-fire <NAME>` | `POST /v1/health/reactors/<name>/fire` | Fires the reactor with its **current** cache; no input injected (`mode: force_fire`). |
| `reactor fire <NAME> --input <SOURCE>=<JSON>` | `POST /v1/health/reactors/<name>/fire` | Replaces the reactor's cache with typed inputs, then fires (`mode: fire_with`). `--input` is repeatable and **required**; each is `source=<json>`. The JSON value is parsed as JSON, or treated as a JSON string if it isn't valid JSON (so `--input note=hello` works unquoted). Full-replace only — no partial merge. Example: `reactor fire pricing_reactor --input prices='{"sym":"ABC","px":12.5}'`. |

## `accumulator`

Operator manual-inject action on accumulators loaded in the server's
graph scheduler (CLOACI-T-0753) — the front-door analogue of `reactor
fire`. The typed JSON event is serialized to the boundary encoding
server-side.

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `accumulator inject <NAME> --event <JSON>` | `POST /v1/health/accumulators/<name>/inject` | Pushes a single typed event into a running accumulator. `--event` is parsed as JSON, or treated as a JSON string if it isn't valid JSON (so `--event hello` works unquoted). Example: `accumulator inject orderbook --event '{"sym":"ABC","px":12.5}'`. |

## `tenant` (admin)

Requires an admin-role key.

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `tenant create <NAME> [--description <STR>] [--password <PW>]` | `POST /v1/tenants` | Creates a new Postgres schema (per-tenant). `--password` sets the tenant DB password; omitted, the server auto-generates one. The password is **never returned** — it's set during provisioning and not surfaced. |
| `tenant list` | `GET /v1/tenants` | Lists tenant schema names. |
| `tenant pool list <TENANT>` | `GET /v1/tenants/<tenant>/pools` | Lists the tenant's [agent pools](../../explanation/service/execution-agent-fleet/#agent-pools). Every tenant has `default`. |
| `tenant pool get <TENANT> <POOL>` | `GET /v1/tenants/<tenant>/pools/<pool>` | One pool: its settings, the agent count the server keeps, and the live agents. |
| `tenant pool set <TENANT> <POOL> [--cpu-request <Q>] [--memory-request <Q>] [--cpu-limit <Q>] [--memory-limit <Q>] [--max-concurrency <N>] [--image <IMAGE>] [--service-account <NAME>] [--service-account-annotation <KEY=VALUE>]... [--pod-label <KEY=VALUE>]... [--pod-annotation <KEY=VALUE>]... [--automount-service-account-token <BOOL>] [--egress <CIDR[=PORTS]>]... [--agents <N>] [--min-agents <N>]` | `PUT /v1/tenants/<tenant>/pools/<pool>` | Creates the pool or replaces its settings with exactly what is given; a setting not given goes back to the server-wide default. `--egress` is `CIDR` or `CIDR=PORT[/PROTOCOL][,PORT...]` (`198.51.100.0/24=443,53/UDP`). `--agents` is the most agents of a pool other than `default` (`max_agents`; needed there, refused for `default`). `--min-agents` is its fewest (`min_agents`, at most `--agents`): the pool scales between the two on its waiting work, and `0` lets it scale to zero; not given, the pool keeps `--agents` agents. `--file <PATH>` (`-` for stdin) sends a JSON object instead of the flags. |
| `tenant pool delete <TENANT> <POOL> [--force]` | `DELETE /v1/tenants/<tenant>/pools/<pool>` | Removes the pool. The server refuses while the pool has agents: run `set --agents 0` first and wait. For `default` it clears the settings. Confirmation unless `--force`. |
| `tenant delete <NAME> [--force]` | `DELETE /v1/tenants/<name>` | Triggers the 4-step teardown orchestration (revoke keys → evict runner cache → evict DB cache → drop schema). Each step emits an audit event. The drain step is bounded by `--tenant-deletion-drain-timeout-s` (server flag, default 30s); past that, the runner is hard-evicted. Interactive confirmation unless `--force`. CLOACI-T-0581. |

## `audit` (admin)

Reads the [audit record](../../how-to/service/security/audit-record/): who did
what, when, in which tenant.

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `audit list [--action <ACTION>] [--actor <ACTOR>] [--since <TIME>] [--until <TIME>] [--limit <N>] [--cursor <ID>]` | `GET /v1/tenants/<tenant>/audit` | The tenant of the profile, newest first. Needs a tenant admin key. `--actor` matches a key id, a key name or a login identity. Times are RFC 3339. Default limit: 50, max 200. When more rows exist, the command prints the `--cursor` value for the next page on stderr. |
| `audit list --all-tenants […]` | `GET /v1/audit` | Every tenant and platform actions. Needs a platform admin key. |

## `key`

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `key create <NAME> [--role <ROLE> \| --platform-admin] [--expires-at <TIME>] [--max-priority <N>]` | `POST /v1/auth/keys` | Roles: `admin`, `write`, `read`, `agent`. Default `read`. `--platform-admin` makes a platform admin key (no tenant, role `admin`, `is_admin = true`) and cannot be combined with `--role`. `--expires-at` is an RFC 3339 time in the future after which the key is refused; not set, the key does not expire. `--max-priority` (0 to 10) is the highest [priority](../../explanation/priority/#a-limit-on-a-key) the key may set; not set, the default of its role; not on a `read` or `agent` key. **The plaintext key is shown exactly once** in table mode (JSON mode returns it in the body with no warning). Save it; it cannot be retrieved later. |
| `key set-priority <ID> (--max-priority <N> \| --clear)` | `PATCH /v1/tenants/<tenant>/keys/<id>` | Sets (0 to 10) or clears the highest priority a key of the tenant may set. Needs an `admin` key of the tenant or a platform admin key; the limit may not be better than the caller's own (`403`). Without `--tenant` the tenant is `public`. |
| `key list` | `GET /v1/auth/keys` | Returns metadata only (ID, name, role, created_at, last_used_at, max_priority). No hashes, no plaintext. |
| `key revoke <ID> [--force]` | `DELETE /v1/auth/keys/<id>` | Revokes the key. Confirmation unless `--force`. The last live platform admin key is refused (`409`). |

`key create --tenant <TENANT> <NAME> [--role <ROLE>]` calls
`POST /v1/tenants/<tenant>/keys` and makes a key of that tenant, with
the same body and roles. A platform admin key or an `admin` key of the
tenant can call it. Without `--tenant`, `key create` calls the
**global** `/v1/auth/keys` surface, which is restricted to
platform-admin (god-mode) keys and makes a key of `public`. `key list`
and `key revoke` always use the global surface.

## `trigger`

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `trigger list [--limit <N>] [--offset <N>]` | `GET /v1/tenants/<tenant>/triggers?limit=…&offset=…` | Combined cron + custom-poll trigger schedules. Default limit: 100, max 1000 (CLOACI-T-0596 / API-10). |
| `trigger inspect <NAME>` | `GET /v1/tenants/<tenant>/triggers/<name>` | Single trigger metadata + recent executions. |

## `backfill`

A backfill runs a workflow once for each fire time of a cron expression from a
start to an end, oldest first. See
[Load History with a Backfill](../../how-to/service/load-history-with-a-backfill/).

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `backfill create <WORKFLOW> --start <TIME> --end <TIME> [--cron <EXPR>] [--timezone <TZ>] [--max-active-runs <N>] [--param <KEY=VALUE>]… [--params <FILE>] [--priority <N>]` | `POST /v1/tenants/<tenant>/workflows/<workflow>/backfills` | Times are RFC 3339; both ends are in the range. Without `--cron`: the cron schedule of the workflow, when it has exactly one. `--max-active-runs` defaults to 1. `--param` values are parsed as JSON when they parse, else taken as strings; `--params` reads a JSON object file (`-` for stdin) and `--param` wins on a conflict. `--priority` (0 to 10) is the workflow [priority](../../explanation/priority/#give-a-schedule-a-priority) of each run, held to your limit when the run starts; a priority above your limit is printed as a warning. Prints the backfill with its id. |
| `backfill create <WORKFLOW> --interval <TIME> […]` | same | One interval: `--interval` is its fire time. The same as `--start` and `--end` with that value. |
| `backfill list <WORKFLOW> [--limit <N>] [--offset <N>]` | `GET /v1/tenants/<tenant>/workflows/<workflow>/backfills?limit=…&offset=…` | Newest first, each with its progress. Default limit: 100, max 1000. |
| `backfill inspect <WORKFLOW> <ID>` | `GET /v1/tenants/<tenant>/workflows/<workflow>/backfills/<id>` | Status, progress (`total`, `completed`, `failed`, `running`, `remaining`, position) and the failed intervals. |
| `backfill cancel <WORKFLOW> <ID>` | `POST /v1/tenants/<tenant>/workflows/<workflow>/backfills/<id>/cancel` | No new interval gets a run. Active runs end on their own. |
| `backfill retry <WORKFLOW> <ID> [--max-active-runs <N>]` | `POST /v1/tenants/<tenant>/workflows/<workflow>/backfills/<id>/retry` | Creates a new backfill from the failed intervals of `<ID>`. Without `--max-active-runs` it takes the limit of `<ID>`. Fails with `409` when `<ID>` has no failed interval. |

`create`, `cancel` and `retry` need a key with the `write` role; `list` and
`inspect` need `read`.

## `secret`

Tenant-scoped encrypted [secrets](../../explanation/service/secrets/).
Requires a tenant **admin** key, and the server must have `CLOACINA_SECRET_KEK`
set (routes return `503` otherwise). Field **values** are never passed on argv —
each `--field`/`-f` names a value source; a literal `NAME=value` is rejected.
Reads never show values. See [Manage Secrets](../../how-to/service/manage-secrets/).

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `secret create <NAME> -f <FIELD>…` | `POST /v1/tenants/<tenant>/secrets` | Each `-f`/`--field` (repeatable, required) reads its value from a file (`NAME=@path`), stdin (`NAME=-`, one field only), or an interactive prompt (`NAME` or `NAME=?`). Returns metadata only. |
| `secret rotate <NAME> -f <FIELD>…` | `PUT /v1/tenants/<tenant>/secrets/<name>` | Replaces the whole field map in place (same `-f` sources as create); the next fire sees the new value. |
| `secret list` | `GET /v1/tenants/<tenant>/secrets` | Metadata only: names, field names, timestamps. Never values. |
| `secret get <NAME>` | `GET /v1/tenants/<tenant>/secrets/<name>` | One secret's metadata. Never values. |
| `secret delete <NAME> [--force]` | `DELETE /v1/tenants/<tenant>/secrets/<name>` | Interactive confirmation unless `--force`. |

---

# Singleton Commands

## `trusted-key`

The keys a tenant trusts for [package signing](../../how-to/service/security/package-signing/).
With `--require-signatures` on the server, an upload to the tenant passes when
its signer is a live trusted key of the tenant. Requires a tenant **admin**
key; a platform admin key names the tenant with `--tenant`.

| Command | Maps to | Notes |
|---|---|---|
| `trusted-key add --pem-file <PATH> [--name <NAME>]` | `POST /v1/tenants/<tenant>/trusted-keys` | Adds the Ed25519 public key of a PEM file (`-----BEGIN PUBLIC KEY-----`). Prints the key's `id` and `fingerprint`. |
| `trusted-key add --public-key <HEX> [--name <NAME>]` | `POST /v1/tenants/<tenant>/trusted-keys` | Adds the raw 32-byte public key, given as 64 hexadecimal characters. Exactly one of `--pem-file` and `--public-key` is required. |
| `trusted-key list` | `GET /v1/tenants/<tenant>/trusted-keys` | The tenant's keys, oldest first. A revoked key stays in the list, with `revoked: true`. |
| `trusted-key revoke <ID> [--force]` | `DELETE /v1/tenants/<tenant>/trusted-keys/<id>` | Interactive confirmation unless `--force`. A revoked key cannot be added again. |

## `status`

```text
cloacinactl status
```

Composite probe: runs `daemon status`, `server status`, and
`compiler status` independently and prints a combined table. Each
probe failure is surfaced individually without affecting the others.

## `config`

Manage the local config file at `~/.cloacina/config.toml`.

### `config get <KEY>` / `config set <KEY> <VALUE>` / `config list`

Dotted-path key access. `set` preserves types: integers stay
integers, booleans stay booleans, arrays accept comma-separated
values. The config file is created on first `set` if missing.

### Profile management

```text
cloacinactl config profile set <NAME> --api-key <KEY> --server <URL> [--default]
cloacinactl config profile list
cloacinactl config profile use <NAME>
cloacinactl config profile delete <NAME>
```

Profiles live under `[profiles.<name>]`. The `default_profile` field
selects which profile is used when `--profile` is not supplied. See
[Profile resolution](#profile-resolution).

## `admin`

### `admin cleanup-events [--database-url <URL>] [--older-than <DUR>] [--dry-run]`

Deletes execution event records older than the threshold. Duration
format: combine `d` / `h` / `m` / `s` (e.g., `90d`, `7d12h`,
`1d2h30m45s`). Case-insensitive. Must be greater than zero. Defaults
to `90d`.

`--dry-run` reports what would be deleted without deleting.

## `completions`

```text
cloacinactl completions <SHELL>
```

Emits a shell-completion script to stdout. Supported shells: `bash`,
`zsh`, `fish`, `powershell`. Pipe to your shell's completion
directory or source it from your rcfile.

---

# Configuration File

Located at `~/.cloacina/config.toml` (or `<home>/config.toml` if
`--home` is set). Optional; all fields have defaults. The parser
**rejects unknown fields** (`deny_unknown_fields`) — but note the
load behavior: a file that fails to parse (including one with a single
unknown key) is logged as a warning and **ignored entirely**, falling
back to defaults. Because most client commands don't initialize
tracing, the warning is usually invisible — a typo'd config manifests
as "no server configured", not as a parse error.

```toml
# Top-level

# Database URL for commands that need it (admin, server start).
database_url = "sqlite:///path/to/cloacina.db"

# Named server-targeting profile selected when --profile is not supplied.
default_profile = "prod"

# Per-profile server-targeting blocks.
[profiles.prod]
server = "https://api.example.com"
api_key = "env:CLOACINA_API_KEY"   # or "raw-key", "file:/path"

[profiles.staging]
server = "https://staging.example.com"
api_key = "file:/etc/cloacina/staging-key"

# Daemon settings.
[daemon]
poll_interval_ms = 500              # Reconciler fallback interval
log_level = "info"                  # NOTE: not currently wired to runtime logging —
                                    # verbosity is controlled by RUST_LOG and `-v`/`--verbose`
shutdown_timeout_s = 30             # Graceful drain timeout
watcher_debounce_ms = 500           # Filesystem watcher debounce
trigger_poll_interval_ms = 1000     # Custom-poll trigger base interval
# cron_max_catchup = 100            # Max cron catchup executions; omit to use
                                    # the runtime default (100)
cron_recovery_interval_s = 300      # Cron recovery sweeper cadence
cron_lost_threshold_min = 10        # Lost-task threshold (present in the schema
                                    # but not currently forwarded to the runner
                                    # config; the runtime default of 10 applies)

# Compiler settings. `local_addr` is used by the `compiler status` /
# `compiler health` probes; the other keys are forwarded by `compiler start`.
[compiler]
local_addr = "127.0.0.1:9000"
# Optional. A key that is not set is not forwarded: the compiler default applies.
# max_unpacked_bytes = 524288000
# python_platforms = ["aarch64-linux", "x86_64-linux"]
# python_version = "3.11"
# python_index_url = "https://pypi.example.com/simple"   # sent as an env var
# python_wheel_dir = "/opt/wheels"
# python_uv_cache_dir = "/var/cache/uv"
# python_offline = false
# uv = "uv"
# tenant_schema = "team_a"
# build_target = "x86_64-linux"
# build_target_package = "etl"
# cargo_flags = ["--locked"]                # or cargo_flags_replace, not both
# cargo_target_dir = "/var/cache/cargo-target"
# vendor_dir = "/opt/cargo-home"
# build_rlimit_cpu = 600
# build_rlimit_mem = "4G"
# build_rlimit_files = 1024
# build_rlimit_procs = 256
# log_retention_days = 14

# Additional package directories for the daemon (~ expansion supported).
[watch]
directories = ["~/my-workflows", "/opt/cloacina/packages"]

# Server settings read by `cloacinactl server start`.
[server]
# Optional. A key that is not set is not forwarded: the server default applies.
# default_executor = "fleet"        # "fleet", or "default" (in-process) with single_trust
# single_trust = false              # true: tenant code runs in the server process
# database_wait_timeout_s = 60
# run_history_retention_days = 30   # 0 keeps run history for ever
# outbox_acked_retention_s = 3600
# retention_interval_s = 300
# fleet_task_timeout_s = 300
# fleet_task_timeout_max_s = 86400
# alert_webhook_url = "https://alerts.example.com/hook"
# alert_webhook_events = ["failed"]
# alert_webhook_timeout_s = 10
# alert_webhook_max_attempts = 8
# public_base_url = "https://cloacina.example.com"
# task_log_url_template = "https://logs.example.com/?run={execution_id}"
# verification_org_id = "6f1c2a7e-3b9d-4c55-9e0a-2d4b8f1e7a60"
# reconcile_interval_s = 30
# tenant_runner_cache_size = 256
# tenant_deletion_drain_timeout_s = 30
# log_retention_days = 14
# log_format = "json"
# agent_heartbeat_interval_s = 15
# agent_liveness_misses = 3
# fleet_artifact_fetch_wait_s = 300
# fleet_graph_firings = "inline"
# fleet_poll_timeout_s = 30
# priority_aging_interval_s = 120
# context_warn_bytes = 1048576
# context_max_bytes = 16777216
# cors_allowed_origins = ["https://ops.example.com"]
# cors_allowed_methods = ["GET", "POST", "DELETE", "OPTIONS"]
# cors_allowed_headers = ["authorization", "content-type"]
# max_package_bytes = 104857600
# max_unpacked_bytes = 524288000
# agent_key_lifetime_days = 30
# agent_key_rotate_days = 10
```

## Profile Resolution

Per ADR-0003 §3, server-targeting flags resolve in this order
(highest precedence first):

1. Explicit `--server` / `--api-key` flags on the command line.
2. The named profile from `--profile <name>`.
3. The default profile from `default_profile`.
4. Error: "no server/key configured."

Within a profile, the API key is decoded by scheme (`raw` / `env:` /
`file:` / `keyring:`). See [API Key Schemes](#api-key-schemes).

---

# File Locations

The `~/.cloacina/` directory (configurable via `--home`) holds:

```text
~/.cloacina/
├── config.toml              # Configuration file
├── cloacina.db              # SQLite database (daemon mode)
├── bootstrap-key            # Server bootstrap admin key, mode 0600 (server mode)
├── daemon.sock              # Daemon health probe (Unix domain socket)
├── daemon.pid               # Daemon PID file (used by `daemon stop` / `health`)
├── server.pid               # Server PID file
├── compiler.pid             # Compiler PID file
├── packages/                # Default daemon watch directory
│   └── *.cloacina           # Discovered packages
└── logs/
    ├── cloacina.log         # Daemon logs (JSON, daily rotation)
    └── cloacina-server.log  # Server logs (JSON, daily rotation)
```

# See Also

- [HTTP API Reference](../http-api/) — endpoints exposed by `cloacinactl server start`.
- [Configuration Reference](../configuration/) — `DefaultRunnerConfig` builder details.
- [Environment Variables Reference](../environment-variables/) — full env var list.
- [Cron Scheduling Architecture](../../explanation/engine/cron-scheduling/) — how the daemon processes cron schedules.
- [Reconciler Pipeline](../../explanation/service/reconciler-pipeline/) — what the daemon's reconciler does after detecting a new package.
