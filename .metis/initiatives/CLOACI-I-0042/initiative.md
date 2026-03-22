---
id: codebase-refactoring-dal
level: initiative
title: "Codebase Refactoring — DAL Deduplication, Module Decomposition, Dead Code Cleanup"
short_code: "CLOACI-I-0042"
created_at: 2026-03-22T22:46:54.566957+00:00
updated_at: 2026-03-22T22:46:54.566957+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: codebase-refactoring-dal
---

# Codebase Refactoring — DAL Deduplication, Module Decomposition, Dead Code Cleanup Initiative

## Context

The codebase has grown rapidly since v0.3.0 (~82k lines). The dual postgres/sqlite backend support led to 162 duplicated function pairs in the DAL layer, 6 files exceed 1,000 lines, and 16 `#[allow(dead_code)]` annotations hide potentially abandoned code. This creates a maintenance burden: every DAL change must be made twice, large files are hard to navigate, and dead code obscures what's actually used.

## Goals & Non-Goals

**Goals:**
- Eliminate DAL duplication via macro or trait abstraction (target: 50%+ line count reduction)
- Split all files exceeding 1,000 lines into sub-modules
- Audit and resolve all `#[allow(dead_code)]` annotations
- Resolve OpenTelemetry stub (implement or remove)

**Non-Goals:**
- Changing database behavior or query semantics
- Adding new features
- Rewriting working code that's merely inelegant

## Current State

### DAL Duplication (biggest item)

Every DAL operation has a `_postgres` and `_sqlite` variant with identical business logic. The only difference is the connection type.

| DAL file | Function pairs | Lines |
|----------|---------------|-------|
| `pipeline_execution.rs` | 30 | 1,412 |
| `pending_boundary_dal.rs` | 18 | ~600 |
| `execution_event.rs` | 16 | ~500 |
| `task_execution_metadata.rs` | 14 | ~450 |
| `workflow_packages.rs` | 12 | ~400 |
| `api_key_dal.rs` | 12 | ~400 |
| + 7 more files | ~60 | ~2,000 |
| **Total** | **~162** | **~5,800** |

The existing `dispatch_backend!` macro handles the dispatch but doesn't eliminate the duplicate function bodies.

### Large Files

| File | Lines | Should become |
|------|-------|---------------|
| `runner.rs` (bindings) | 2,263 | Split: execution, cron, triggers |
| `continuous/scheduler.rs` | 1,487 | Split: core, state, policies |
| `dal/pipeline_execution.rs` | 1,412 | Split: crud, state, queries (+ dedup) |
| `workflow/mod.rs` | 1,377 | Split: builder, validation, graph |
| `cloacina-macros/packaged_workflow.rs` | 1,266 | Split: codegen, manifest, validation |
| `security/db_key_manager.rs` | 1,200 | Split: keys, trust, ACL |

### Dead Code

16 `#[allow(dead_code)]` annotations across: `admin.rs` (3), `dispatcher/default.rs` (2), `executor/pipeline_executor.rs` (1), `cloacina-macros/registry.rs` (3), `cloacina-testing/runner.rs` (1), + others.

## Alternatives Considered

**Diesel MultiConnection derive** — Could unify postgres/sqlite at the connection level. Rejected: already using MultiConnection for the schema, but individual queries still need type-specific code due to diesel's type system.

**Generic trait with associated types** — Define a `Backend` trait that abstracts connection + pool. Each DAL method would be generic over `B: Backend`. Promising but requires significant type gymnastics with diesel.

**Macro expansion** — Extend `dispatch_backend!` to generate both function bodies from a single template. Most pragmatic — keeps diesel's type safety while eliminating duplication. **Recommended approach.**

## Implementation Plan

### Phase 1: DAL dedup macro
- Design a `dal_method!` macro that takes a single function body and generates both `_postgres` and `_sqlite` variants
- Start with one DAL file (api_key_dal.rs — smallest, 12 pairs) as proof of concept
- Roll out to remaining 12 DAL files
- Target: 50%+ reduction in DAL line count

### Phase 2: Split large files
- Split each >1,000 line file into sub-modules
- Maintain public API via re-exports from `mod.rs`
- No behavioral changes — pure structural refactoring
- One file at a time, with tests after each

### Phase 3: Dead code + stubs
- Audit each `#[allow(dead_code)]` — remove code or justify with comment
- Resolve OpenTelemetry stub: implement basic OTLP export or remove the placeholder
- Remove any other stubs or TODO-only code
