---
title: "Cleaning Up Events"
description: "How to manage execution event table growth by purging old records"
weight: 65
---

# Cleaning Up Events

This guide covers using the `cloacinactl admin cleanup-events` command to manage the execution event table, which grows with every task execution and can impact query performance over time.

## Prerequisites

- `cloacinactl` binary installed
- Access to the Cloacina database (PostgreSQL or SQLite)
- Database URL available via `--database-url`, `DATABASE_URL` env var, or `~/.cloacina/config.toml`

## Why Cleanup Matters

The execution events table grows with every task execution and can reach millions of rows in busy systems. Regular cleanup keeps the database responsive without losing data about recent executions.

## Running Cleanup

### Step 1: Preview with Dry Run

Always start with `--dry-run` to see how many events would be deleted:

```bash
cloacinactl admin cleanup-events \
  --database-url postgresql://cloacina:cloacina@localhost:5432/cloacina \
  --older-than 90d \
  --dry-run
```

Output:

```
[DRY RUN] Would delete 284,391 execution event(s) older than 2026-01-02T14:30:00Z
```

### Step 2: Execute the Cleanup

Once satisfied with the preview, remove the `--dry-run` flag:

```bash
cloacinactl admin cleanup-events \
  --database-url postgresql://cloacina:cloacina@localhost:5432/cloacina \
  --older-than 90d
```

Output:

```
Deleted 284,391 execution event(s) older than 2026-01-02T14:30:00Z
```

If no events match the threshold:

```
No execution events found older than 2026-01-02T14:30:00Z
```

## Duration Format

The `--older-than` flag accepts a human-readable duration string with the following units:

| Unit | Meaning | Example |
|------|---------|---------|
| `d` | Days | `90d` = 90 days |
| `h` | Hours | `24h` = 24 hours |
| `m` | Minutes | `30m` = 30 minutes |
| `s` | Seconds | `60s` = 60 seconds |

Units can be combined:

```bash
# 7 days and 12 hours
cloacinactl admin cleanup-events --older-than 7d12h --dry-run

# 1 day, 2 hours, 30 minutes
cloacinactl admin cleanup-events --older-than 1d2h30m --dry-run
```

The duration is case-insensitive (`90D` works the same as `90d`). The default is `90d` if `--older-than` is not specified.

## Using the Config File for Database URL

To avoid passing `--database-url` every time, set it in the config file:

```bash
cloacinactl config set database_url "postgresql://cloacina:cloacina@localhost:5432/cloacina"
```

Then run cleanup without the flag:

```bash
cloacinactl admin cleanup-events --older-than 30d --dry-run
```

The command resolves the database URL from (in order): `--database-url` flag, `DATABASE_URL` environment variable, `~/.cloacina/config.toml`.

## Automating Cleanup

### System Cron (Weekly)

Add a cron entry to run cleanup every Sunday at 3:00 AM:

```bash
crontab -e
```

```cron
0 3 * * 0 /usr/local/bin/cloacinactl admin cleanup-events --older-than 90d >> /var/log/cloacina-cleanup.log 2>&1
```

### System Cron (Daily, Shorter Retention)

For high-volume systems where events are only needed for a week:

```cron
0 2 * * * /usr/local/bin/cloacinactl admin cleanup-events --older-than 7d >> /var/log/cloacina-cleanup.log 2>&1
```

### Using Environment Variables

If the database URL is stored in an environment file:

```cron
0 3 * * 0 . /etc/cloacina/env && /usr/local/bin/cloacinactl admin cleanup-events --older-than 90d >> /var/log/cloacina-cleanup.log 2>&1
```

Where `/etc/cloacina/env` contains:

```bash
export DATABASE_URL="postgresql://cloacina:secret@db.example.com:5432/cloacina"
```

## Considerations

### Retention Policies

Choose a retention period that balances operational needs with storage costs:

| Use Case | Suggested Retention | Rationale |
|----------|-------------------|-----------|
| Development | `7d` | Only recent runs matter for debugging |
| Staging | `30d` | Keep enough history for release validation |
| Production | `90d` | Quarterly retention for incident investigation |
| Compliance-heavy | `365d` or longer | Regulatory requirements may mandate longer retention |

### Before Cleanup

- **Back up first**: If you might need historical events for compliance or auditing, back up the database before running cleanup.
- **Off-peak hours**: Schedule cleanup during low-traffic periods to minimize lock contention on the events table.
- **Dry run in production**: Always use `--dry-run` first in production to verify the scope of deletion.

### What Gets Deleted

The cleanup command deletes rows from the execution events table where `created_at` is older than the specified threshold. It does not delete:

- Pipeline execution records (the parent execution metadata)
- Task execution records
- Workflow packages or schedules
- API keys or tenant data

Execution records remain intact so you can still see that a pipeline ran, but the detailed per-task event log for old executions will be gone.
