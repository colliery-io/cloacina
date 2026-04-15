---
id: coverage-driven-test-sprint-target
level: initiative
title: "Coverage-Driven Test Sprint — Target 75% Line Coverage"
short_code: "CLOACI-I-0068"
created_at: 2026-04-03T13:03:22.968573+00:00
updated_at: 2026-04-03T13:19:02.261046+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: coverage-driven-test-sprint-target
---

# Coverage-Driven Test Sprint — Target 75% Line Coverage Initiative

## Context

Follow-up to I-0067 (Pre-1.0 Test Coverage Hardening). After completing 10 tasks and adding ~142 tests, `cargo llvm-cov` reports **57.1% line coverage** (30,668 lines, 13,483 missed). This initiative targets the remaining gaps identified by the coverage report to reach 75%.

### Current state (April 2026, post-I-0067)
- **Overall**: 57.1% line coverage
- **Well covered (>80%)**: crypto, api_keys, context, cron_evaluator, manifest_schema, trigger_rules, stale_claim_sweeper, watcher, serve handlers
- **Zero coverage**: Python bindings (~2,400 lines), packaging/debug, executor/types, reconciler/extraction
- **Critical low (<25%)**: security/db_key_manager (5%), schedule DAL (21%), task_outbox (14%), daemon run() (16%), reconciler/loading (18%)

### Biggest gaps by missed lines
1. Python bindings/runner.rs — 1,626 missed lines (0%)
2. Schedule CRUD — 820 missed lines (21%)
3. Security db_key_manager — 748 missed lines (5%)
4. Schedule execution CRUD — 440 missed lines (24%)
5. Reconciler loading — 337 missed lines (18%)
6. Macros packaged_workflow — 331 missed lines (25%)
7. Task execution metadata — 325 missed lines (45%)
8. Daemon run() — 259 missed lines (16%)
9. Workflow registry database — 251 missed lines (53%)
10. Task outbox — 247 missed lines (14%)

## Goals & Non-Goals

**Goals:**
- Move overall line coverage from 57% to 75%
- Eliminate all 0%-coverage production files
- Bring all critical-path modules to >60% coverage
- Use `angreal coverage --html` to measure progress

**Non-Goals:**
- 100% coverage (diminishing returns)
- Branch coverage (llvm-cov crashes on macOS merge — tracked upstream)
- Testing generated macro code (proc-macro output is tested by users of macros)
- Testing main.rs (CLI entry point — tested by soak/integration tests)

## Detailed Design

### T1: Python bindings test suite (PyO3 runtime tests)
Test the PyO3 binding layer: `bindings/runner.rs` (1,626 lines), `bindings/context.rs` (242), `bindings/admin.rs` (106), `bindings/retry.rs` (161), `python/workflow.rs` (219), `python/context.rs` (141), `python/namespace.rs` (70). These need `pyo3::prepare_freethreaded_python()` + Python::with_gil tests. ~2,565 missed lines.

### T2: Schedule and cron DAL tests
Integration tests for `dal/unified/schedule/crud.rs` (820 missed) and `dal/unified/schedule_execution/crud.rs` (440 missed). CRUD operations for cron schedules, schedule executions, and the `cron_api.rs` (190 missed) runner methods. ~1,450 missed lines.

### T3: Security module tests (db_key_manager + package_signer)
`security/db_key_manager.rs` (748 missed, 5% coverage) — key trust chain, DB-backed key management. `security/package_signer.rs` (258 missed, 41%) — package signing flows. `security/verification.rs` (114 missed, 68%) — signature verification. ~1,120 missed lines.

### T4: Task outbox and execution metadata DAL tests
`dal/unified/task_outbox.rs` (247 missed, 14%) — outbox pattern for task dispatch. `dal/unified/task_execution_metadata.rs` (325 missed, 45%) — task metadata storage. `dal/unified/task_execution/recovery.rs` (175 missed, 20%) — task recovery paths. ~747 missed lines.

### T5: Reconciler and package loading tests
`registry/reconciler/loading.rs` (337 missed, 18%) — package loading pipeline. `registry/reconciler/extraction.rs` (148 missed, 0%) — archive extraction. Need compiled test packages or Python packages. ~485 missed lines.

### T6: Executor and scheduler gap coverage
`executor/thread_task_executor.rs` (169 missed, 52%) — thread-based task execution. `scheduler.rs` (279 missed, 52%) — unified scheduler. `executor/types.rs` (62 missed, 0%) — executor type definitions. ~510 missed lines.

### T7: Workflow and registry database coverage
`workflow/mod.rs` (146 missed, 74%) — workflow operations. `registry/workflow_registry/database.rs` (251 missed, 53%) — DB-backed registry. `dal/unified/workflow_packages.rs` (290 missed, 36%). ~687 missed lines.

## Alternatives Considered

**Test only critical paths**: Rejected — the 0%-coverage files represent real production code (Python bindings used by every Python workflow user). Systematic coverage is more reliable than cherry-picking.

**Use mocks instead of integration tests**: Rejected for DAL tests (same reasoning as I-0067 — real DB tests catch actual bugs). Appropriate for Python binding tests where PyO3 is the boundary.

## Implementation Plan

Tier 1 (biggest impact): T1 (Python bindings), T2 (schedule DAL), T3 (security)
Tier 2 (core engine): T4 (outbox/metadata), T5 (reconciler), T6 (executor/scheduler)
Tier 3 (polish): T7 (workflow/registry DB)
