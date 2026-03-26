---
id: workdistributor-trait-with
level: task
title: "WorkDistributor trait with Postgres LISTEN/NOTIFY and SQLite polling"
short_code: "CLOACI-T-0083"
created_at: 2026-02-03T20:16:48.262042+00:00
updated_at: 2026-02-06T03:51:33.976976+00:00
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

# WorkDistributor trait with Postgres LISTEN/NOTIFY and SQLite polling

## Parent Initiative

[[CLOACI-I-0022]] - Execution Events and Outbox-Based Task Distribution

## Objective

Create a `WorkDistributor` trait that abstracts "wait for work to be available" with Postgres LISTEN/NOTIFY implementation and SQLite polling implementation.

## Acceptance Criteria

## Acceptance Criteria

- [x] `WorkDistributor` trait with `wait_for_work()` method
- [x] `PostgresDistributor` using LISTEN/NOTIFY with 30s poll fallback
- [x] `SqliteDistributor` using 500ms polling
- [x] Postgres trigger on `task_outbox` inserts to fire NOTIFY (existed in migration 012)
- [x] Integration with executor worker loop (N/A - current architecture uses push-based dispatching; WorkDistributor available for future pull-based model)

## Implementation Notes

### Trait Definition

```rust
#[async_trait]
pub trait WorkDistributor: Send + Sync {
    /// Wait until work might be available, or timeout
    async fn wait_for_work(&self);
}
```

### Postgres Implementation

```rust
pub struct PostgresDistributor {
    listener: PgListener,
}

impl PostgresDistributor {
    pub async fn new(pool: &PgPool) -> Result<Self> {
        let mut listener = PgListener::connect_with(pool).await?;
        listener.listen("task_ready").await?;
        Ok(Self { listener })
    }
}

#[async_trait]
impl WorkDistributor for PostgresDistributor {
    async fn wait_for_work(&self) {
        tokio::select! {
            _ = self.listener.recv() => {},
            _ = tokio::time::sleep(Duration::from_secs(30)) => {},
        }
    }
}
```

### SQLite Implementation

```rust
pub struct SqliteDistributor;

#[async_trait]
impl WorkDistributor for SqliteDistributor {
    async fn wait_for_work(&self) {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
```

### Postgres Trigger

```sql
CREATE OR REPLACE FUNCTION notify_task_ready() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('task_ready', NEW.task_execution_id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER task_outbox_notify
    AFTER INSERT ON task_outbox
    FOR EACH ROW EXECUTE FUNCTION notify_task_ready();
```

### Dependencies

- Requires CLOACI-T-0079 (schema migrations)
- Requires CLOACI-T-0082 (outbox claiming)

## Status Updates

### Session 1 (2026-02-06)
- Created `WorkDistributor` trait in `crates/cloacina/src/dispatcher/work_distributor.rs`
- Implemented `PostgresDistributor` using tokio-postgres LISTEN/NOTIFY with 30s poll fallback
- Implemented `SqliteDistributor` using 500ms periodic polling
- Added `tokio-postgres` and `futures` dependencies to Cargo.toml
- Updated dispatcher/mod.rs to export new types
- All tests passing: 289 unit tests, 220+ integration tests

**Remaining work:**
- Integration with executor worker loop (optional - currently the scheduler does dispatching directly)
- The Postgres trigger is already in migration 012_create_execution_events_and_outbox

**Design notes:**
- PostgresDistributor spawns a background task that listens for `task_ready` notifications
- Uses mpsc channel to forward notifications from connection driver to main loop
- Falls back to polling every 30s in case notifications are missed
- SqliteDistributor uses simple tokio::time::sleep for periodic polling
- Both implement clean shutdown via atomic bool + Notify

### Session 1 - Final Verification (2026-02-06)
- Verified work_distributor.rs exists (13,294 bytes)
- All 2 work_distributor unit tests passing
- Exports properly configured in dispatcher/mod.rs:
  - `WorkDistributor` trait
  - `PostgresDistributor` (feature-gated to postgres)
  - `SqliteDistributor` (feature-gated to sqlite)
  - `create_work_distributor` helper function

**All acceptance criteria met. Task ready for review.**
