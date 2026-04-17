---
id: hardening-security-resilience
level: initiative
title: "Hardening — Security, Resilience, Recovery"
short_code: "CLOACI-I-0051"
created_at: 2026-03-26T05:35:16.350475+00:00
updated_at: 2026-03-26T05:35:16.350475+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: hardening-security-resilience
---

# Hardening — Security, Resilience, Recovery Initiative

## Context

Three hardening efforts previously completed on the `archive/cloacina-server-week1` branch:

1. **Security Hardening** (I-0039) — Fixed auth bypass vulnerabilities, path traversal in package loading, added API rate limits, sandboxed Python execution.

2. **Operational Resilience** (I-0040) — Eliminated panics in production paths, ensured clean shutdown, fixed resource leaks, hardened database integrity under concurrent access.

3. **Recovery Redesign** (I-0043) — Replaced the old `RecoveryManager` with a heartbeat-based `RecoverySweepService`. Tasks claim with heartbeat, sweeper detects orphans via stale heartbeats. Added `claimed_by` and `heartbeat_at` columns to `task_executions`. The old `RecoveryManager` startup recovery was removed — `RecoverySweepService` subsumes it entirely.

**Key learnings:**
- Postgres schema must include `claimed_by` and `heartbeat_at` (migration added them but Diesel schema wasn't updated — caused test failures)
- Don't need both `RecoveryManager` and `RecoverySweepService` — sweeper subsumes the old recovery
- Recovery tests should test the sweeper directly, not through `TaskScheduler::new()`

## Goals & Non-Goals

**Goals:**
- Security: auth bypass fixes, path traversal protection, API rate limits, Python sandbox
- Resilience: no panics in production, clean shutdown, no resource leaks
- Recovery: heartbeat-based orphan detection via `RecoverySweepService`
- Remove old `RecoveryManager` code entirely

**Non-Goals:**
- New features unrelated to hardening
- Performance optimization work

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- No auth bypass possible on protected endpoints
- Package loading validates paths (no directory traversal)
- All production code paths are panic-free
- Graceful shutdown completes within timeout
- Orphaned tasks recovered within sweeper interval
- `RecoverySweepService` replaces `RecoveryManager` entirely
- All tests pass including recovery integration tests

## Prior Art

Reference implementation on `archive/cloacina-server-week1`:
- Security hardening: commit `eeebd80` (feat: security hardening — auth bypass, path traversal, API limits, Python sandbox)
- Operational resilience: commit `ffb8cb6` (feat: operational resilience — panic elimination, shutdown safety, resource leaks, DB integrity)
- Recovery redesign: commits `3299c5f`, `c3c12d3`, `6d5b02a` (heartbeat foundation, sweeper service, full redesign)

Key learnings:
- Diesel schema MUST include `claimed_by` and `heartbeat_at` after migration (was missing, caused test failures)
- RecoverySweepService subsumes old RecoveryManager entirely — don't keep both
- Recovery tests should test sweeper directly, not via TaskScheduler::new()

## Alternatives Considered

- **Keep both RecoveryManager and RecoverySweepService**: Rejected — the sweeper subsumes all RecoveryManager functionality, and maintaining both adds complexity with no benefit.
- **Time-based recovery without heartbeats**: Rejected — heartbeats give accurate liveness detection; pure time-based approaches either recover too aggressively or too slowly.

## Implementation Plan

Work is structured in three parallel tracks mirroring the original initiatives:
1. **Security** — Auth, path validation, rate limits, sandbox
2. **Resilience** — Panic elimination, shutdown, resource leaks, DB integrity
3. **Recovery** — Heartbeat migration, RecoverySweepService, remove RecoveryManager
