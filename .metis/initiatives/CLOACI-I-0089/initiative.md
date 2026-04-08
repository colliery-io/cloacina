---
id: code-quality-sweep-config
level: initiative
title: "Code Quality Sweep — Config Validation, Error Cleanup, CI Auditing"
short_code: "CLOACI-I-0089"
created_at: 2026-04-08T10:46:52.131631+00:00
updated_at: 2026-04-08T10:46:52.131631+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
initiative_id: code-quality-sweep-config
---

# Code Quality Sweep — Config Validation, Error Cleanup, CI Auditing Initiative

*Source: Architecture Review (review/10-recommendations.md) — Phase 5: Code Quality*

## Context

A collection of smaller code quality improvements identified across legibility, API design, operability, and security lenses. Each is individually low-effort but together they reduce paper cuts and improve the contributor experience. Can run in parallel with Phases 3-4.

## Goals & Non-Goals

**Goals:**
- Add configuration validation at startup (OPS-07, API-04)
- Clean up duplicate error variants and unused dispatch macros (LEG-04, LEG-03)
- Fix WorkflowBuilder context manager metadata loss (API-08)
- Add `cargo audit` to CI (SEC-14)
- Add production Dockerfile and docker-compose (OPS-06)

**Non-Goals:**
- Error framework redesign (covered by Reliability initiative I-0086)
- Full config system overhaul

## Detailed Design

### REC-14: Configuration Validation (OPS-07, API-04) — 3-4 hours
Add `validate()` to `DefaultRunnerConfig::build()`. Check: `max_concurrent_tasks > 0`, `stale_claim_threshold > heartbeat_interval`, `db_pool_size > 0`. Replace freeform strings with enums: `StorageBackend`, `BackoffStrategy`, `KeyRole`.

### REC-15: Error/Macro Cleanup (LEG-04, LEG-03) — 2-3 hours
Remove `MissingDependencyOld`. Consolidate `CyclicDependency`/`CircularDependency`. Remove `backend_dispatch!` and `connection_match!` macros (each used once), consolidate on `dispatch_backend!`.

### REC-19: WorkflowBuilder Context Manager Fix (API-08) — 1-2 hours
In `python/workflow.rs` `__exit__`, carry `description` and `tags` to the new `Workflow` object.

### REC-20: Dependency Auditing in CI (SEC-14) — 1-2 hours
Add `cargo audit` to `nightly.yml`. Consider `cargo deny` for license compliance. Start as non-blocking.

### REC-18: Production Dockerfile (OPS-06) — 1-2 days
Multi-stage Dockerfile for `cloacinactl serve`. Docker-compose with PostgreSQL. Document production deployment path.

## Implementation Plan

All items independent. Can be picked up opportunistically alongside other work. Target: ongoing, parallel with Phases 3-4.
