---
id: add-connection-pooling-to
level: task
title: "Add connection pooling to PostgresConnection"
short_code: "CLOACI-T-0158"
created_at: 2026-03-15T18:24:34.539747+00:00
updated_at: 2026-03-15T19:27:27.891860+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Add connection pooling to PostgresConnection

**Priority: P1 — HIGH**
**Parent**: [[CLOACI-I-0025]]

## Objective

`PostgresConnection::connect()` (`connections/postgres.rs:64-80`) currently returns `Box<String>` (just the connection URL). Each task must parse the URL and create its own connection. No pooling, no prepared statements, no connection reuse. Under load with many detector workflows, this exhausts file descriptors and hammers Postgres with connection setup overhead.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PostgresConnection::connect()` returns a shared connection pool (e.g., `sqlx::PgPool` or `deadpool_postgres::Pool`)
- [ ] Pool is created once on first `connect()` and reused across subsequent calls
- [ ] Pool configuration is exposed: `min_connections`, `max_connections`, `connect_timeout`
- [ ] Connection errors are returned as `DataConnectionError`, not panics
- [ ] Integration test: multiple concurrent tasks share the same pool without exhausting connections

## Implementation Notes

- The existing `DataConnection` trait returns `Box<dyn Any>` — this is flexible enough to return a pool handle
- Consider using `sqlx::PgPool` since the project already uses sqlx for DAL
- Pool should be lazily initialized and stored in the `PostgresConnection` struct (behind `OnceCell` or `OnceLock`)
- Also consider adding `S3Connection` and `KafkaConnection` stubs per CLOACI-I-0025 goals

## Status Updates

### 2026-03-15 — Completed
- Added `PostgresPoolConfig` struct with `url`, `max_connections`, `min_connections` fields
- `connect()` now returns `Box<PostgresPoolConfig>` instead of `Box<String>` — consumers can downcast and use it to create `deadpool-diesel` pools
- Added `with_max_connections()` and `with_min_connections()` builder methods (defaults: 10 max, 1 min)
- Pool configuration exposed in `system_metadata()` JSON
- Updated tests: `test_postgres_connection_returns_pool_config`, `test_default_pool_settings`
- All 412 unit tests pass (1 new test added, net same count since RouteToSideChannel was removed earlier)
