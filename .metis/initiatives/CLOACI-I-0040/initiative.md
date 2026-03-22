---
id: operational-resilience-panic
level: initiative
title: "Operational Resilience — Panic Elimination, Shutdown Safety, Resource Leaks, DB Integrity"
short_code: "CLOACI-I-0040"
created_at: 2026-03-21T18:39:28.373118+00:00
updated_at: 2026-03-22T01:17:18.138734+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: operational-resilience-panic
---

# Operational Resilience — Panic Elimination, Shutdown Safety, Resource Leaks, DB Integrity Initiative

## Context

Audits of error handling, operational risks, and database integrity identified multiple paths where the system can crash, hang, leak resources, or silently corrupt data. The project has moved from a library to running long-lived server/daemon processes — these issues become critical in production.

## Goals & Non-Goals

**Goals:**
- Eliminate all `panic!`/`expect()` in production code paths — every error returns `Result`
- Ensure graceful shutdown with timeouts on all background services
- Fix resource leaks (staging dirs, DB connections, temp files)
- Fix database integrity issues (SQLite FK enforcement, atomic operations, per-connection PRAGMAs)
- Add health monitoring for background services

**Non-Goals:**
- Performance optimization (separate concern)
- New features
- Refactoring DAL duplication (covered by I-0041)

## Findings (from audit)

### Panic elimination

| Finding | Location | Impact |
|---------|----------|--------|
| `BackendType::from_url` panics on bad URL | `database/connection/backend.rs:83-104` | Config typo crashes process |
| `expect_postgres()`/`expect_sqlite()` panic on wrong backend | `backend.rs:186,194` | Runtime mismatch crashes |
| `get_connection_with_schema` panics on wrong backend | `connection/mod.rs:538,593` | Wrong backend crashes |
| `Database::new_with_schema` uses `expect()` | `connection/mod.rs:178` | DB unreachable crashes |
| `expect()` in API key hashing | `security/api_keys.rs:65` | Hash failure crashes |
| `expect()` in DAL init | `dal/unified/mod.rs:285` | Startup crash |
| `storage_type.parse().unwrap()` in model conversion | `dal/unified/models.rs:725` | Bad DB data crashes |
| `PyContext::clone()` unwraps insert | `python/context.rs:232` | Task execution crash |
| Mutex `lock().unwrap()` on workflow context stack | `python/task.rs:94,100,105` | Poisoned mutex cascades |

### Shutdown & lifecycle

| Finding | Location | Impact |
|---------|----------|--------|
| No shutdown timeout — hangs forever | `default_runner/mod.rs:323-375` | Process can't stop |
| Background service crash silently ignored | `services.rs:88-89,173-174,...` | System runs degraded silently |
| Shutdown swallows `JoinError` (panic in bg task invisible) | `mod.rs:332-368` | Hidden panics |
| `Drop` for `DefaultRunner` doesn't shutdown | `mod.rs:398-404` | Resource leak |
| Auth DB pool never closed on shutdown | `serve.rs:289-301` | Connection leak |

### Resource leaks

| Finding | Location | Impact |
|---------|----------|--------|
| `std::mem::forget(staging_dir)` leaks temp dirs permanently | `workflow_registry/mod.rs:327` | Disk fills up |
| Blocking I/O on async runtime in daemon scanner | `daemon.rs:313-337` | Starves tokio workers |

### Database integrity

| Finding | Location | Impact |
|---------|----------|--------|
| SQLite foreign keys not enforced | `connection/mod.rs` (PRAGMA missing) | Orphaned rows |
| SQLite PRAGMAs only set during migration, not per-connection | `connection/mod.rs:393-408` | `busy_timeout` reverts |
| `reset_task_for_recovery` missing transaction + outbox entry | `recovery.rs:91-149` | Stranded tasks |
| Cron execution create non-atomic | `cron_execution/crud.rs:31-68` | Orphaned audit records |
| `claim_and_update_postgres` SERIALIZABLE outside transaction | `cron_schedule/state.rs:281` | Isolation not applied |
| PostgreSQL `NOW()` vs application `UniversalTimestamp::now()` | `claiming.rs:257-278` | Clock drift inconsistency |
| `execution_exists` check not atomic with insert | `cron_execution/tracking.rs:148-204` | Duplicate executions |

## Implementation Plan

### Phase 1: Panic elimination
- Replace all `panic!`/`expect()`/`unwrap()` in non-test code with `Result` propagation
- Switch `Mutex` to `parking_lot::Mutex` (no poisoning) or use `unwrap_or_else`
- Audit `unsafe impl Send/Sync` on PythonTaskWrapper

### Phase 2: Shutdown & lifecycle
- Add `tokio::time::timeout(30s)` around each `handle.await` in `shutdown()`
- Log `JoinError` (distinguish panic vs cancellation)
- Add health flag per background service — set on crash, exposed via `/health`
- Close auth DB pool during shutdown
- Use `tokio::task::spawn_blocking` for daemon filesystem scans

### Phase 3: Resource management
- Replace `std::mem::forget(staging_dir)` with managed storage (track in registry, cleanup on unregister/shutdown)
- Remove `eprintln!` debug output from production FFI code

### Phase 4: Database integrity
- Add `PRAGMA foreign_keys=ON` via connection customizer (every connection)
- Set `busy_timeout` via customizer, not just migration
- Wrap `reset_task_for_recovery` in transaction with outbox entry
- Wrap cron execution create in single `interact` + transaction
- Add unique index on `cron_executions(schedule_id, scheduled_time)`
- Wrap PostgreSQL SERIALIZABLE + query in explicit transaction
