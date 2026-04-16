---
id: dal-backend-consolidation-reduce
level: initiative
title: "DAL Backend Consolidation — Reduce Dual-Backend Code Duplication"
short_code: "CLOACI-I-0092"
created_at: 2026-04-09T16:57:43.004473+00:00
updated_at: 2026-04-09T16:57:43.004473+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: dal-backend-consolidation-reduce
---

# DAL Backend Consolidation — Reduce Dual-Backend Code Duplication Initiative

*Extracted from I-0091. Parked for future reconsideration.*

## Context

Every DAL method is structurally duplicated as `_postgres`/`_sqlite` pairs via the `dispatch_backend!` macro. This doubles code in the DAL layer and requires parallel modification for every schema change. The duplication is an inherent cost of Diesel's `MultiConnection` strategy — Diesel does not support generic connection abstractions (documented in ADR-0001).

## Goals & Non-Goals

**Goals:**
- Investigate if future Diesel versions add generic connection support
- Add CI equivalence check verifying both backends produce identical results
- Reduce maintenance burden of the dual-backend pattern where possible

**Non-Goals:**
- Replacing Diesel with another ORM
- Dropping SQLite or PostgreSQL support

## Current State

**Mitigations already applied (I-0089)**:
- Consolidated on single `dispatch_backend!` macro (removed `backend_dispatch!` and `connection_match!`)

**Known constraint**: Diesel's `MultiConnection` enum dispatch requires concrete connection types in each method. Generic `impl Connection` is not supported for the query builder.

## Implementation Plan

Parked. Revisit when:
1. Diesel releases a version with generic connection support
2. A third backend is needed (at which point triplication forces the issue)
3. Schema evolution velocity makes the duplication actively painful
