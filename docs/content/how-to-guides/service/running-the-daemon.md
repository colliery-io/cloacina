---
title: "Running the Daemon"
description: "How to run the Cloacina daemon to watch for workflow packages and execute cron schedules and triggers"
weight: 50
---

# Running the Daemon

This guide walks you through starting and configuring the Cloacina daemon, a lightweight local scheduler that watches directories for `.cloacina` packages and runs their cron schedules and triggers using a SQLite backend.

## Prerequisites

- `cloacinactl` binary installed and on your PATH
- One or more `.cloacina` workflow packages built (see [Tutorial 07 - Packaged Workflows]({{< ref "/tutorials/workflows/service/07-packaged-workflows" >}}) for building packages)

## Starting the Daemon

### Step 1: Start with Default Settings

The simplest way to start the daemon is with no arguments:

```bash
cloacinactl daemon
```

This creates the `~/.cloacina/` home directory (if it does not exist), opens a SQLite database at `~/.cloacina/cloacina.db`, and watches `~/.cloacina/packages/` for `.cloacina` files.

### Step 2: Watch Additional Directories

To watch directories beyond the default `~/.cloacina/packages/`, use the `--watch-dir` flag (repeatable):

```bash
cloacinactl daemon \
  --watch-dir /opt/cloacina/workflows \
  --watch-dir ~/my-project/packages
```

The default packages directory is always watched regardless of what you pass.

### Step 3: Deploy Packages

Copy `.cloacina` package files into any watched directory:

```bash
cp my-workflow.cloacina ~/.cloacina/packages/
```

The daemon detects the new file via filesystem events, loads the package through the registry reconciler, and registers any cron or trigger schedules defined in the package manifest. You will see log output confirming the load:

```
Reconciliation: 1 loaded, 0 unloaded
Registered cron schedule: 'nightly-cleanup' -> workflow 'cleanup' (cron: 0 0 * * *, id: ...)
```

To remove a workflow, delete its `.cloacina` file from the watched directory. The daemon will unload it on the next reconciliation cycle.

## Configuring via config.toml

The daemon reads `~/.cloacina/config.toml` on startup. You can edit it directly or use the `config` subcommand.

### Daemon Settings

```toml
[daemon]
# Reconciler poll interval in milliseconds (fallback if filesystem events are missed)
poll_interval_ms = 500

# Log level: trace, debug, info, warn, error
log_level = "info"

# Graceful shutdown timeout in seconds
shutdown_timeout_s = 30

# Filesystem watcher debounce interval in milliseconds
watcher_debounce_ms = 500

# Trigger scheduler base poll interval in milliseconds
trigger_poll_interval_ms = 1000

# Maximum cron catchup executions after downtime (omit for unlimited)
# cron_max_catchup = 10

# Cron recovery check interval in seconds
cron_recovery_interval_s = 300

# Cron lost task threshold in minutes
cron_lost_threshold_min = 10
```

### Watch Directories in Config

```toml
[watch]
directories = [
  "/opt/cloacina/workflows",
  "~/my-project/packages",
]
```

Paths starting with `~/` are expanded to the user's home directory. These directories are merged with any `--watch-dir` flags passed on the command line.

### Using the Config CLI

```bash
# View a single value
cloacinactl config get daemon.poll_interval_ms

# Change a value
cloacinactl config set daemon.poll_interval_ms 1000

# List all values
cloacinactl config list
```

### Overriding Poll Interval from the Command Line

The `--poll-interval` flag overrides the config file value for the cron reconciler:

```bash
cloacinactl daemon --poll-interval 2000
```

## Inspecting Logs

The daemon writes logs to two destinations simultaneously:

- **stderr**: human-readable format for interactive use
- **File**: JSON-structured logs at `~/.cloacina/logs/cloacina.log` (daily rotation)

### Controlling Log Verbosity

Use the `RUST_LOG` environment variable for fine-grained control:

```bash
# Debug logging for Cloacina, info for everything else
RUST_LOG=cloacina=debug,info cloacinactl daemon

# Or use the --verbose flag for global debug output
cloacinactl daemon --verbose
```

You can also set the default level in `config.toml`:

```toml
[daemon]
log_level = "debug"
```

### Reading the Log File

```bash
# Tail the current log file
tail -f ~/.cloacina/logs/cloacina.log

# Parse JSON logs with jq
tail -f ~/.cloacina/logs/cloacina.log | jq '.fields.message'
```

## Signal Handling

The daemon responds to the following Unix signals:

| Signal | Behavior |
|--------|----------|
| **SIGINT** (Ctrl+C) | Initiates graceful shutdown. In-flight pipelines are drained with a configurable timeout (default 30s). A second SIGINT forces immediate exit. |
| **SIGTERM** | Same as SIGINT -- graceful shutdown with drain. |
| **SIGHUP** | Reloads `~/.cloacina/config.toml` without restarting. New watch directories are added, removed directories are unwatched, and a reconciliation runs to pick up any packages in newly watched paths. |

### Example: Reloading Configuration

After editing `config.toml` to add a new watch directory:

```bash
kill -HUP $(pgrep -f 'cloacinactl daemon')
```

The daemon logs will confirm the reload:

```
Received SIGHUP -- reloading configuration...
Added watch directory: /opt/cloacina/new-workflows
Triggering reconciliation after config reload...
Configuration reload complete.
```

## Troubleshooting

### Package Not Loading

1. Verify the file has a `.cloacina` extension and is in a watched directory.
2. Check the daemon logs for reconciliation errors:
   ```bash
   grep -i "failed" ~/.cloacina/logs/cloacina.log | tail -20
   ```
3. If the package was built against a different platform or has a corrupted archive, the reconciler will report a failure with the package ID and error message.
4. Ensure the daemon has read permissions on the package file and the watched directory.

### Cron Schedule Not Firing

1. Confirm the schedule was registered by looking for `Registered cron schedule` in the logs.
2. Check that `cron_max_catchup` is not set too low if the daemon was down for a period. When omitted, the daemon runs all missed executions.
3. Verify the cron expression is valid (standard 5-field format: minute, hour, day-of-month, month, day-of-week).
4. If the daemon was recently restarted, recovery runs after `cron_recovery_interval_s` seconds (default 300).

### Trigger Not Polling

1. The trigger must have a registered `Trigger` implementation in the package. If only a `cron_expression` is defined in `package.toml`, it is treated as a cron schedule, not a poll trigger.
2. Look for the warning `Trigger '...' declared in package.toml but no Trigger impl found in registry` in the logs.
3. Adjust `trigger_poll_interval_ms` if the default 1000ms polling is too frequent or too slow.

### High CPU or Disk I/O

- Increase `watcher_debounce_ms` to reduce filesystem event processing (default 500ms).
- Increase `poll_interval_ms` to reduce periodic reconciliation frequency.
- If you have many packages, the periodic reconciliation can be costly. The filesystem watcher handles immediate detection; the periodic tick is a fallback.
