---
id: test-coverage-gaps-pre-1-0
level: initiative
title: "Test Coverage Gaps — Pre-1.0 Hardening"
short_code: "CLOACI-I-0067"
created_at: 2026-04-03T02:34:12.057836+00:00
updated_at: 2026-04-03T02:39:22.258760+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: test-coverage-gaps-pre-1-0
---

# Test Coverage Gaps — Pre-1.0 Hardening Initiative

## Context

A deep analysis of the test suite (April 2026) revealed significant coverage gaps. The library core is at ~3.5/5 confidence — happy paths tested against real databases, but scheduler internals and error paths are thin. The services (daemon/server) are at 2/5 — only validated by soak tests with zero handler-level or unit tests.

### Current test inventory
- ~320 unit tests across 56 files in cloacina core
- ~220 integration tests across 57 files (real Postgres/SQLite)
- ~92 Python test functions across 26 scenario files
- 2 soak tests (daemon + server)
- Zero tests for cloacinactl server handlers, daemon watcher, or config parsing

### Risk assessment (ordered by severity)
1. **Server HTTP API — zero handler tests** (HIGH). Auth, tenants, workflows, executions, triggers — only tested by one soak script. Malformed requests, auth edge cases, concurrent uploads untested.
2. **Daemon — zero tests** (HIGH). Watcher, hot-reload, config parsing, shutdown logic — only exercised by soak script.
3. **Task scheduler loop — zero unit tests** (HIGH). 1,200+ lines of scheduling logic (scheduler_loop.rs, state_manager.rs, context_manager.rs, recovery.rs) tested only indirectly.
4. **Trigger rule evaluation untested** (MEDIUM-HIGH). Serialization tested, but runtime evaluation of TaskSuccess/ContextValue/All/Any/Not conditions is not directly verified.
5. **Stale claim sweeper untested** (MEDIUM-HIGH). Grace period logic, sweep-and-reset, partial failure handling — none tested directly.
6. **Cron recovery thin** (MEDIUM). 2 unit tests. No integration test for missed execution catchup or recovery after downtime.
7. **Error paths underexercised** (MEDIUM). DB connection failures, pool exhaustion, corrupt context data, invalid task config — not tested.
8. **Sleep-based test assertions** (MEDIUM). 9 integration test files use fixed sleeps — flaky under CI load.
9. **Python package e2e gap** (MEDIUM). Full lifecycle only in soak tests, no deterministic integration test.
10. **Multi-tenancy under concurrency** (MEDIUM). No test for concurrent cross-tenant requests or schema migration races.

## Goals & Non-Goals
e
**Goals:**
- Move library confidence from 3.5/5 to 4.5/5
- Move service confidence from 2/5 to 3.5/5
- Establish coverage measurement in CI (cargo-tarpaulin or llvm-cov)
- Eliminate flaky sleep-based assertions
- Test all critical error paths

**Non-Goals:**
- 100% line coverage (diminishing returns)
- Load testing / performance benchmarks (covered by I-0054)
- Chaos testing (process crashes, network partitions — future work)

## Detailed Design

### Tier 1 — Highest impact (service confidence)

**T1: HTTP API handler tests** — Use axum::TestServer to test each endpoint with valid, invalid, and malicious inputs. Cover: auth middleware (no token, invalid token, expired cache), tenant CRUD (duplicate names, invalid chars), workflow upload (corrupt package, oversized, wrong content-type), execution (nonexistent workflow, concurrent), triggers (list empty, list with data).

**T2: Daemon watcher and lifecycle tests** — Mock filesystem events, test reconciliation logic in isolation. Cover: malformed package in watch dir, rapid add/remove, config reload via SIGHUP, graceful shutdown with in-flight work.

### Tier 2 — Core engine hardening

**T3: Task scheduler unit tests** — Extract pure logic from scheduler_loop.rs, state_manager.rs, context_manager.rs into testable functions. Test all state transitions, context merge conflicts, empty-queue behavior.

**T4: Trigger rule runtime evaluation tests** — Integration tests that schedule workflows with All/Any/Not/ContextValue rules and verify correct task execution/skipping.

**T5: Stale claim sweeper tests** — Test grace period, sweep-and-reset flow, edge cases (task completes between find and release).

**T6: Cron recovery integration tests** — Simulate scheduler downtime, verify catchup count, test deduplication of catchup runs.

### Tier 3 — Robustness

**T7: Error path tests** — DB connection loss, pool exhaustion, corrupt context JSON, invalid task configuration. Verify graceful degradation, not panics.

**T8: Replace sleep-based assertions** — Convert fixed sleeps to `tokio::time::timeout` + condition polling in all 9 affected test files.

**T9: Python package deterministic e2e test** — Single integration test: create .cloacina Python package → load via registry → execute → verify context output. Not a soak test.

**T10: Coverage measurement in CI** — Add cargo-tarpaulin to nightly workflow. Report coverage, set minimum thresholds for critical modules.

## Alternatives Considered

**Mock-heavy testing instead of real-database tests**: Rejected. The existing dual-backend real-DB approach is superior for catching actual integration bugs. Mocks would reduce confidence.

**Separate test repository**: Rejected. Co-located tests stay in sync with code changes.

## Implementation Plan

Tier 1 (T1-T2) first — biggest confidence improvement for services.
Tier 2 (T3-T6) next — core engine hardening.
Tier 3 (T7-T10) last — robustness and infrastructure.
