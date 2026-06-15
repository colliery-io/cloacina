---
title: "CLI Reference"
description: "Complete reference for cloacinactl: all nouns, verbs, flags, environment variables, exit codes, and configuration."
weight: 5
---

# CLI Reference

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
| `--no-color` | `false` | Disable ANSI colors in table output. |

## API Key Schemes

`--api-key` and the `api_key` profile field accept several schemes:

| Scheme | Example | Behavior |
|---|---|---|
| Raw | `clk_a1b2c3...` | The literal API key. |
| `env:VAR` | `env:CLOACINA_API_KEY` | Read the key from the named environment variable. Errors if the variable is unset or empty. |
| `file:PATH` | `file:/etc/cloacina/key` | Read the first non-empty line of the file. Whitespace is trimmed. |
| `keyring:NAME` | `keyring:prod` | **Reserved for v1.1**; rejected today with a clear error message. |

## Output Formats

| Format | Behavior |
|---|---|
| `table` (default) | Human-readable aligned columns. Long strings are truncated with `…`. Respects `--no-color`. Auto-infers columns from the first object's keys; not ideal for deeply-nested resources. |
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
| `CLOACINA_BOOTSTRAP_KEY` | `server start` | Pre-supplied bootstrap admin key. If unset and no keys exist, one is generated. Overridden by `--bootstrap-key`. |
| `CLOACINA_REQUIRE_SIGNATURES` | `server start` | If `true`, the server enforces package signature verification at upload. Overridden by `--require-signatures`. |
| `RUST_LOG` | All commands | Log filter directive (e.g., `info`, `debug`, `cloacina=debug`). Overridden by `-v` / `--verbose`. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `server start` | If set, enables OpenTelemetry tracing to the named gRPC collector. |
| `OTEL_SERVICE_NAME` | `server start` | Service name in OpenTelemetry spans. Default: `cloacina`. |
| `CLOACINA_VAR_<NAME>` | Workflow context | Read by `cloaca.var("NAME")` in Python; the `CLOACINA_VAR_` prefix is mandatory. |

---

# Service Commands

These commands manage local services. They exec long-running binaries
in the foreground or stop them via PID file.

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
SQLite. Exposes the [HTTP API]({{< ref "/platform/reference/http-api" >}})
under the `/v1/` prefix and the unauth probes `/health`, `/ready`,
`/metrics`.

### `server start`

```text
cloacinactl server start [--bind <ADDR>] [--database-url <URL>]
                        [--bootstrap-key <KEY>]
                        [--require-signatures] [--verification-org-id <UUID>]
                        [--reconcile-interval-s <N>]
                        [--tenant-runner-cache-size <N>]
                        [--tenant-deletion-drain-timeout-s <N>]
                        [--log-retention-days <N>]
```

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--bind <ADDR>` | | `127.0.0.1:8080` | Listen address. |
| `--database-url <URL>` | `DATABASE_URL` | (required) | Database connection URL. Examples: `sqlite:///path/to/cloacina.db`, `postgres://user:pass@host/dbname`. |
| `--bootstrap-key <KEY>` | `CLOACINA_BOOTSTRAP_KEY` | (auto-generated) | Pre-supplied admin key for first startup. If a key is generated, the plaintext is written to `~/.cloacina/bootstrap-key` with `0600` perms exactly once. |
| `--require-signatures` | `CLOACINA_REQUIRE_SIGNATURES` | `false` | Enforce package signature verification at upload time. Requires `--verification-org-id`. CLOACI-I-0103. |
| `--verification-org-id <UUID>` | `CLOACINA_VERIFICATION_ORG_ID` | (none) | Trusted organization UUID used to verify package signatures. **Required when `--require-signatures` is set**; startup fails fast otherwise. CLOACI-I-0103 / T-0567. |
| `--reconcile-interval-s <N>` | | (runtime default) | Interval (seconds) between reconciler passes that sync the in-runner workflow registry with the DB. Increase for quiet prod; decrease for fast e2e tests. |
| `--tenant-runner-cache-size <N>` | `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | `256` | LRU cap on cached per-tenant `DefaultRunner` instances. Each cached runner has its own scheduler loop, executor pool, and DB connection pool. Bump for high-cardinality SaaS deployments; drop for memory-tight ones. CLOACI-T-0580. |
| `--tenant-deletion-drain-timeout-s <N>` | `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | `30` | Max seconds to wait for in-flight workflows to drain during tenant teardown (step 2 of the 4-step orchestration). Past this, the runner is hard-evicted; any task that ignored cooperative cancellation errors on its next DB write once the schema is dropped. CLOACI-T-0581. |
| `--log-retention-days <N>` | | `14` | Number of daily-rotated log files to retain. `0` disables pruning entirely. CLOACI-I-0109 / T-0592. |

This subcommand was renamed from `serve` in an earlier release; older
docs may still mention the old name. The reconciler poll interval, the
multi-tenant cache knobs, and the default executor are exposed via
`cloacinactl server start` (the wrapper forwards them to the underlying
`cloacina-server` binary). Other runtime-tuning knobs are not surfaced
through the wrapper — if you need to tune them, invoke `cloacina-server`
directly.

### Default executor (`cloacinactl server start` + `cloacina-server`)

Execution topology is a single deployment knob (CLOACI-T-0640). The
preferred way to set it is `[server].default_executor` in `config.toml`;
`cloacinactl server start` reads it and forwards `--default-executor` to
`cloacina-server`. The flag/env below override the config value for ad-hoc
runs. The key is hard-matched against the registered executors at startup —
an unknown key fails fast.

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--default-executor <KEY>` | `CLOACINA_DEFAULT_EXECUTOR` | `default` | Executor every task is dispatched to. `default` runs all work on the in-process thread executor; `fleet` sends it to the [execution-agent fleet]({{< ref "/platform/explanation/execution-agent-fleet" >}}). Forwarded by `cloacinactl server start`; also settable directly on `cloacina-server`. CLOACI-T-0640. |

### Fleet agent liveness (`cloacina-server`)

These flags tune the [execution-agent fleet]({{< ref "/platform/explanation/execution-agent-fleet" >}}). They live on the `cloacina-server` binary directly (and via the env vars below); the `cloacinactl server start` wrapper does **not** forward them, so set them on `cloacina-server` itself or through the environment.

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--agent-heartbeat-interval-s <N>` | `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `15` | Heartbeat interval (seconds) advertised to agents and used as the liveness-sweep cadence. Lower = faster dead-agent detection + in-flight reclaim, more heartbeat traffic. CLOACI-T-0639. |
| `--agent-liveness-misses <N>` | `CLOACINA_AGENT_LIVENESS_MISSES` | `3` | Consecutive missed heartbeats before an agent is declared dead and its in-flight work reclaimed. Effective dead-after = interval × misses (default 45s). CLOACI-T-0639. |

### `server stop` / `status` / `health`

Same shape as the daemon equivalents, but use the server's PID file
and HTTP-based status (`GET /v1/health/status`) and health (`GET
/health`) probes instead of a Unix socket.

## `compiler`

The compilation service (`cloacina-compiler`). Polls the database for
pending package builds and produces signed `.cloacina` archives.

### `compiler start` / `stop` / `status` / `health`

```text
cloacinactl compiler start [--bind <ADDR>] [--database-url <URL>]
                          [--poll-interval-ms <N>]
                          [--heartbeat-interval-s <N>]
                          [--stale-threshold-s <N>]
                          [--sweep-interval-s <N>]
```

| Flag | Default | Description |
|---|---|---|
| `--bind <ADDR>` | `127.0.0.1:9000` | Listen address. |
| `--database-url <URL>` | from `DATABASE_URL` | Connection URL (Postgres or SQLite). |
| `--poll-interval-ms` | (binary default) | How often to poll for pending build rows. |
| `--heartbeat-interval-s` | (binary default) | Worker heartbeat cadence. |
| `--stale-threshold-s` | (binary default) | Seconds before an in-flight build is considered stale and reclaimable. |
| `--sweep-interval-s` | (binary default) | How often to sweep for stale claims. |

`stop` / `status` / `health` follow the same pattern as `server`: PID
file for stop, HTTP `GET /v1/status` for status, HTTP `GET /health`
for health.

## `agent`

The execution-agent (`cloacina-agent`) is a **DB-less** worker that joins a
server's [fleet]({{< ref "/platform/explanation/execution-agent-fleet" >}}). It
registers over REST, opens the delivery WebSocket, fetches compiled workflow
cdylibs by digest, executes the task in-process, and reports the result back —
holding no database connection of its own. It is a standalone binary, not a
`cloacinactl` subcommand.

```text
cloacina-agent --server <URL> --api-key <KEY>
               [--agent-id <ID>] [--max-concurrency <N>]
               [--capabilities <TAG,TAG>] [--target-triple-override <TRIPLE>]
               [--cache-dir <PATH>]
```

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--server <URL>` | `CLOACINA_SERVER` | (required) | Base URL of the server to register with (REST + WS ticket mint). |
| `--api-key <KEY>` | `CLOACINA_API_KEY` | (required) | API key for REST + WS auth. Its tenant scope determines which tenants' work the agent may receive (REQ-008). |
| `--agent-id <ID>` | | (server-assigned) | Optional caller-chosen agent id. If omitted, the server assigns one. |
| `--max-concurrency <N>` | | `4` | Max work packets the agent runs concurrently. The server's capacity-aware selection won't exceed this; a saturated agent refuses further packets. |
| `--capabilities <TAG,TAG>` | | (none) | Free-form capability tags advertised at registration. |
| `--target-triple-override <TRIPLE>` | | (host triple) | Override the advertised host target triple. Rarely needed — the server only dispatches a cdylib built for the agent's triple (OQ-6 fail-closed), so this is mainly for testing that path. |
| `--cache-dir <PATH>` | `CLOACINA_AGENT_CACHE_DIR` | `<TMPDIR>/cloacina-agent-cache` | Where fetched cdylibs are cached by digest; a cache hit skips the REST fetch. |

The agent exposes no HTTP surface of its own — observe it via its logs and the
server's fleet metrics (`cloacina_fleet_*`). See the how-to guide
[Deploy an execution-agent fleet]({{< ref "/platform/how-to-guides/deploy-an-execution-agent-fleet" >}}).

---

# Client Commands

These commands talk to a running `cloacina-server` over HTTP. They
respect the global `--profile` / `--server` / `--api-key` /
`--tenant` flags. All require authentication except where noted.

## `package`

### `package build <DIR> [--release]`

Runs `cargo build` in `<DIR>` (must contain a `Cargo.toml` and
`package.toml`). With `--release`, builds the release profile.
Local-only; does not contact the server. Exits 1 on missing files or
build failure.

### `package pack <DIR> [--out <PATH>] [--sign <KEY>]`

Calls `fidius_core::package::pack_package()` to produce a `.cloacina`
archive. The `--sign <KEY>` flag is **accepted but currently ignored**
— detached signature side-car generation is not implemented in the CLI
yet (the side-car infrastructure exists in
`cloacina::security::package_signer`, the wiring is pending).

### `package publish <DIR> [--release] [--sign <KEY>]`

`build` → `pack` → `upload` in one shot.

### `package upload <FILE>`

POSTs a `.cloacina` archive to `/v1/tenants/<tenant>/workflows`.
Requires `--api-key` + `--server`. Server-side signature verification
is enforced if the server was started with `--require-signatures`.
Exit codes: 1 (user error), 2 (network), 4 (auth), 5 (server reject).

### `package list [--filter <SUBSTRING>]`

`GET /v1/packages`. The `--filter` is applied client-side as a
substring match on the package name.

### `package inspect <ID>`

`GET /v1/packages/<id>`. Prints a single object respecting
`--output`.

### `package delete <ID> [--force]`

`DELETE /v1/packages/<id>`. Interactive confirmation unless
`--force`.

## `workflow`

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `workflow list [--package <FILTER>]` | `GET /v1/tenants/<tenant>/workflows` | Client-side `--package` substring filter on the package name. |
| `workflow inspect <NAME>` | `GET /v1/tenants/<tenant>/workflows/<name>` | Full workflow metadata: tasks, dependencies, trigger rules, schedules. |
| `workflow run <NAME> [--context <SOURCE>]` | `POST /v1/tenants/<tenant>/workflows/<name>/execute` | `--context` accepts a path to a JSON file or `-` for stdin. Defaults to `{}`. JSON is validated before submission. Prints the execution ID. |

## `execution`

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `execution list [--workflow <F>] [--status <S>] [--limit <N>] [--offset <N>]` | `GET /v1/tenants/<tenant>/executions?status=…&workflow=…&limit=…&offset=…` | Default limit: 100, max 1000. `--status` and `--workflow` map to the server query params of the same names (CLOACI-T-0594 / API-02). |
| `execution status <ID>` | `GET /v1/tenants/<tenant>/executions/<id>` | Returns Pending / Running / Completed / Failed / Cancelled / Paused. |
| `execution events <ID> [--since <DURATION>] [--follow]` | `GET /v1/tenants/<tenant>/executions/<id>/events?since=<dur>` | `--follow` streams live events over the server's WebSocket delivery substrate (CLOACI-I-0115) until interrupted. `--since` cannot be combined with `--follow` (cursor support is future work); use `--since` on a non-follow call for the historical snapshot. |

## `graph`

Per CLOACI-S-0011, *graph* is the unit of scheduling; *reactor* is a
node inside the graph.

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `graph list` | `GET /v1/health/graphs` | Lists loaded computation graphs with health + reactor pause state. |
| `graph status <NAME>` | `GET /v1/health/graphs/<name>` | Single graph's health + accumulators + reactor pause state. |
| `graph accumulators` | `GET /v1/health/accumulators` | Lists all accumulators across all graphs. |

## `tenant` (admin)

Requires an admin-role key.

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `tenant create <NAME> [--description <STR>]` | `POST /v1/tenants` | Creates a new Postgres schema (per-tenant). The schema's password is **never returned** — it's set during provisioning and not surfaced. |
| `tenant list` | `GET /v1/tenants` | Lists tenant schema names. |
| `tenant delete <NAME> [--force]` | `DELETE /v1/tenants/<name>` | Triggers the 4-step teardown orchestration (revoke keys → evict runner cache → evict DB cache → drop schema). Each step emits an audit event. The drain step is bounded by `--tenant-deletion-drain-timeout-s` (server flag, default 30s); past that, the runner is hard-evicted. Interactive confirmation unless `--force`. CLOACI-T-0581. |

## `key`

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `key create <NAME> [--role <ROLE>]` | `POST /v1/auth/keys` (or `/v1/tenants/<tenant>/keys` if tenant-scoped) | Roles: `admin`, `write`, `read`. Default `read`. **The plaintext key is shown exactly once.** Save it; it cannot be retrieved later. |
| `key list` | `GET /v1/auth/keys` | Returns metadata only (ID, name, role, created_at, last_used_at). No hashes, no plaintext. |
| `key revoke <ID> [--force]` | `DELETE /v1/auth/keys/<id>` | Revokes the key. The server clears its **entire** auth cache on revoke (not just the revoked key) so revocation is immediate; subsequent requests re-validate against the database. |

## `trigger`

| Command | HTTP Endpoint | Notes |
|---|---|---|
| `trigger list [--limit <N>] [--offset <N>]` | `GET /v1/tenants/<tenant>/triggers?limit=…&offset=…` | Combined cron + custom-poll trigger schedules. Default limit: 100, max 1000 (CLOACI-T-0596 / API-10). |
| `trigger inspect <NAME>` | `GET /v1/tenants/<tenant>/triggers/<name>` | Single trigger metadata + recent executions. |

---

# Singleton Commands

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
**rejects unknown fields** to catch typos early.

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
log_level = "info"                  # trace / debug / info / warn / error
shutdown_timeout_s = 30             # Graceful drain timeout
watcher_debounce_ms = 500           # Filesystem watcher debounce
trigger_poll_interval_ms = 1000     # Custom-poll trigger base interval
# cron_max_catchup = 100            # Max cron catchup executions; omit for unlimited
cron_recovery_interval_s = 300      # Cron recovery sweeper cadence
cron_lost_threshold_min = 10        # Lost-task threshold before reclaim

# Compiler settings (used by `compiler status` / `compiler health` probes).
[compiler]
local_addr = "127.0.0.1:9000"

# Additional package directories for the daemon (~ expansion supported).
[watch]
directories = ["~/my-workflows", "/opt/cloacina/packages"]
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

- [HTTP API Reference]({{< ref "/platform/reference/http-api" >}}) — endpoints exposed by `cloacinactl server start`.
- [Configuration Reference]({{< ref "/platform/reference/configuration" >}}) — `DefaultRunnerConfig` builder details.
- [Environment Variables Reference]({{< ref "/platform/reference/environment-variables" >}}) — full env var list.
- [Cron Scheduling Architecture]({{< ref "/workflows/explanation/cron-scheduling" >}}) — how the daemon processes cron schedules.
- [Reconciler Pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}}) — what the daemon's reconciler does after detecting a new package.
