---
title: "CLI Reference"
description: "Complete command reference for the cloacinactl command-line interface"
weight: 5
---

# CLI Reference

`cloacinactl` is the command-line interface for the Cloacina task orchestration engine. It provides commands for running the daemon, serving the HTTP API, managing configuration, and performing administrative tasks.

## Global Flags

| Flag | Short | Description | Default |
|---|---|---|---|
| `--verbose` | `-v` | Enable verbose (debug-level) logging | `false` |
| `--home` | | Cloacina home directory | `~/.cloacina` |

Both flags are global and apply to all subcommands.

## Commands

### daemon

Run the daemon -- a lightweight local scheduler that watches directories for `.cloacina` packages and runs their cron schedules and triggers. Uses SQLite for state storage.

```bash
cloacinactl daemon [--watch-dir <DIR>]... [--poll-interval <MS>]
```

| Flag | Description | Default |
|---|---|---|
| `--watch-dir <DIR>` | Additional directories to watch for `.cloacina` packages. Repeatable. The default packages directory (`~/.cloacina/packages/`) is always watched. | (none) |
| `--poll-interval <MS>` | Reconciler poll interval in milliseconds. The filesystem watcher handles immediate detection; this is a fallback. | `500` |

**Behavior:**

1. Creates `~/.cloacina/` home directory structure (packages, logs, database)
2. Opens or creates the SQLite database at `~/.cloacina/cloacina.db` (WAL mode)
3. Initializes `DefaultRunner` with configured poll intervals
4. Creates a `FilesystemWorkflowRegistry` across all watch directories
5. Performs initial reconciliation (loads existing packages)
6. Starts filesystem watcher with debounce
7. Enters event loop: filesystem changes trigger reconciliation; periodic fallback reconciliation runs at `--poll-interval`
8. On SIGHUP: reloads `config.toml`, adds/removes watch directories, triggers reconciliation
9. On SIGINT/SIGTERM: drains in-flight pipelines with configurable timeout, then exits

**Logging:**

The daemon writes to two destinations:
- **stderr**: Human-readable log lines
- **File**: JSON-structured logs to `~/.cloacina/logs/cloacina.log` (daily rotation)

Log level is controlled by `--verbose` (sets `debug`) or the `RUST_LOG` environment variable (default: `info`).

### serve

Run the API server -- an HTTP service backed by PostgreSQL with authentication and multi-tenancy.

```bash
cloacinactl serve [--bind <ADDR>] [--database-url <URL>] [--bootstrap-key <KEY>]
```

| Flag | Env Var | Description | Default |
|---|---|---|---|
| `--bind <ADDR>` | | Address to bind the HTTP server to | `0.0.0.0:8080` |
| `--database-url <URL>` | `DATABASE_URL` | PostgreSQL connection URL. Overrides config file. | (required) |
| `--bootstrap-key <KEY>` | `CLOACINA_BOOTSTRAP_KEY` | Bootstrap API key for first startup. If provided and no keys exist, this key is registered as the admin key. | (auto-generated) |

**Bootstrap key behavior:**

On first startup (when no API keys exist in the database):

1. If `--bootstrap-key` is provided, that key is registered as the admin key
2. Otherwise, a cryptographically random key is generated (with `clk_` prefix)
3. The plaintext key is written to `~/.cloacina/bootstrap-key` with mode `0600`
4. The key is never logged

On subsequent startups, bootstrap is skipped if any API keys exist.

**Logging:**

Same dual-destination logging as the daemon:
- **stderr**: Human-readable
- **File**: JSON to `~/.cloacina/logs/cloacina-server.log` (daily rotation)

### config

Manage configuration values stored in `~/.cloacina/config.toml`.

#### config get

```bash
cloacinactl config get <KEY>
```

Prints the value for a dotted key path. Exits with error if the key is not found.

#### config set

```bash
cloacinactl config set <KEY> <VALUE>
```

Sets a configuration value. The type is preserved: integers stay integers, booleans stay booleans. Arrays accept comma-separated values.

#### config list

```bash
cloacinactl config list
```

Prints all configuration key-value pairs with dotted paths.

**Examples:**

```bash
# Set database URL
cloacinactl config set database_url "postgresql://user:pass@localhost/cloacina"

# Set daemon poll interval
cloacinactl config set daemon.poll_interval_ms 1000

# Add watch directories
cloacinactl config set watch.directories "/opt/workflows,~/my-workflows"

# View all config
cloacinactl config list

# Get a single value
cloacinactl config get daemon.shutdown_timeout_s
```

### admin cleanup-events

Clean up old execution events from the database.

```bash
cloacinactl admin cleanup-events [--database-url <URL>] [--older-than <DURATION>] [--dry-run]
```

| Flag | Env Var | Description | Default |
|---|---|---|---|
| `--database-url <URL>` | `DATABASE_URL` | Database connection URL | (required) |
| `--older-than <DURATION>` | | Delete events older than this duration | `90d` |
| `--dry-run` | | Preview what would be deleted without actually deleting | `false` |

**Duration format:**

Combine one or more units:

| Unit | Meaning |
|---|---|
| `d` | Days |
| `h` | Hours |
| `m` | Minutes |
| `s` | Seconds |

Examples: `90d`, `24h`, `7d12h`, `1d2h30m45s`. Case-insensitive. Must be greater than zero.

## Environment Variables

| Variable | Used By | Description |
|---|---|---|
| `DATABASE_URL` | `serve`, `admin cleanup-events` | PostgreSQL connection URL. Overridden by `--database-url` flag. |
| `CLOACINA_BOOTSTRAP_KEY` | `serve` | Bootstrap API key for first startup. Overridden by `--bootstrap-key` flag. |
| `RUST_LOG` | All commands | Log filter directive (e.g., `info`, `debug`, `cloacina=debug`). Overridden by `--verbose`. |

## Configuration File

The configuration file is located at `~/.cloacina/config.toml`. It is optional; all values have defaults.

### Schema

```toml
# Database URL for commands that need it (serve, admin)
database_url = "postgresql://user:pass@localhost/cloacina"

[daemon]
# Cron scheduler poll interval in milliseconds
poll_interval_ms = 500

# Log level: trace, debug, info, warn, error
log_level = "info"

# Graceful shutdown timeout in seconds
shutdown_timeout_s = 30

# Filesystem watcher debounce interval in milliseconds
watcher_debounce_ms = 500

# Trigger scheduler base poll interval in milliseconds
trigger_poll_interval_ms = 1000

# Maximum cron catchup executions (omit for unlimited)
# cron_max_catchup = 100

# Cron recovery check interval in seconds
cron_recovery_interval_s = 300

# Cron lost task threshold in minutes
cron_lost_threshold_min = 10

[watch]
# Additional directories to watch for packages (supports ~ expansion)
directories = [
    "~/my-workflows",
    "/opt/cloacina/packages",
]
```

### Key Paths

All keys can be used with `cloacinactl config get/set`:

| Key | Type | Default | Description |
|---|---|---|---|
| `database_url` | string | (none) | PostgreSQL connection URL |
| `daemon.poll_interval_ms` | integer | `500` | Cron scheduler poll interval (ms) |
| `daemon.log_level` | string | `"info"` | Log level |
| `daemon.shutdown_timeout_s` | integer | `30` | Graceful shutdown timeout (s) |
| `daemon.watcher_debounce_ms` | integer | `500` | Filesystem watcher debounce (ms) |
| `daemon.trigger_poll_interval_ms` | integer | `1000` | Trigger base poll interval (ms) |
| `daemon.cron_max_catchup` | integer | (unlimited) | Max cron catchup executions |
| `daemon.cron_recovery_interval_s` | integer | `300` | Cron recovery check interval (s) |
| `daemon.cron_lost_threshold_min` | integer | `10` | Minutes before task is considered lost |
| `watch.directories` | array | `[]` | Additional watch directories |

## File Locations

The `~/.cloacina/` directory (configurable via `--home`) contains:

```
~/.cloacina/
  config.toml          # Configuration file
  cloacina.db          # SQLite database (daemon mode)
  bootstrap-key        # Bootstrap admin API key (mode 0600, serve mode)
  packages/            # Default package directory (daemon mode)
  logs/
    cloacina.log       # Daemon log files (daily rotation)
    cloacina-server.log  # API server log files (daily rotation)
```

## See Also

- [HTTP API Reference]({{< ref "http-api" >}}) -- endpoints exposed by `cloacinactl serve`
- [Configuration Reference]({{< ref "configuration" >}}) -- `DefaultRunnerConfig` details
- [Cron Scheduling Architecture]({{< ref "/explanation/workflows/cron-scheduling" >}}) -- how the daemon processes cron schedules
