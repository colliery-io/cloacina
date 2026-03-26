---
id: cli-cleanup-events-command-and
level: task
title: "CLI cleanup-events command and retention configuration"
short_code: "CLOACI-T-0084"
created_at: 2026-02-03T20:16:49.156865+00:00
updated_at: 2026-02-06T13:29:46.567133+00:00
parent: CLOACI-I-0022
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0022
---

# CLI cleanup-events command and retention configuration

## Parent Initiative

[[CLOACI-I-0022]] - Execution Events and Outbox-Based Task Distribution

## Objective

Add CLI command for cleaning up old execution events and configuration for retention policy.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `cloacina admin cleanup-events` CLI command
- [x] `--older-than` flag accepting duration (e.g., `90d`, `30d`, `7d`)
- [x] `--dry-run` flag to preview deletion count
- [ ] Configuration option `execution_events.retention_days` (deferred - not needed for CLI use case)
- [x] Reports count of deleted events

## Implementation Notes

### CLI Command

```bash
# Delete events older than 90 days
cloacina admin cleanup-events --older-than 90d

# Preview only
cloacina admin cleanup-events --older-than 90d --dry-run

# Use configured retention
cloacina admin cleanup-events
```

### Configuration

```toml
[execution_events]
retention_days = 90
```

### DAL Method

```rust
impl ExecutionEventDAL {
    pub async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query("DELETE FROM execution_events WHERE created_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn count_older_than(&self, cutoff: DateTime<Utc>) -> Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM execution_events WHERE created_at < $1")
            .bind(cutoff)
            .fetch_one(&self.pool)
            .await
    }
}
```

### Scheduling via Cloacina

Can be scheduled as a Cloacina workflow itself for self-maintenance.

### Dependencies

- Requires CLOACI-T-0079 (schema migrations)
- Requires CLOACI-T-0080 (event DAL)

## Status Updates

### 2026-02-05: Implementation Complete

Created the `cloacina-cli` crate with the admin cleanup-events command:

**Files Created:**
- `crates/cloacina-cli/Cargo.toml` - CLI crate configuration
- `crates/cloacina-cli/src/main.rs` - CLI entry point with clap
- `crates/cloacina-cli/src/commands/mod.rs` - Commands module
- `crates/cloacina-cli/src/commands/cleanup_events.rs` - Cleanup command implementation

**Features Implemented:**
- `cloacina admin cleanup-events --older-than <duration>` command
- Duration parsing supporting `d` (days), `h` (hours), `m` (minutes), `s` (seconds)
- Combined durations like `7d12h` work
- `--dry-run` flag for previewing what would be deleted
- Default retention of 90 days
- Database URL via `--database-url` flag or `DATABASE_URL` env var
- Verbose mode with `-v` flag

**Also Added:**
- `count_older_than` method to `ExecutionEventDAL` for dry-run support (in previous session)
- Added `cloacina-cli` to workspace members
- Added `env` feature to clap workspace dependency

**Tests:**
- 11 unit tests for duration parsing pass
- CLI builds and help output is correct

**Note:** Configuration option `execution_events.retention_days` was not implemented as the CLI command fully satisfies the use case. This could be added later if a background cleanup service is needed.
