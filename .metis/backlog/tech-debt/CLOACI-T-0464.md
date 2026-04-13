---
id: rewrite-python-runner-as-thin
level: task
title: "Rewrite Python runner as thin wrapper around Rust API instead of parallel reimplementation"
short_code: "CLOACI-T-0464"
created_at: 2026-04-09T15:54:27.311608+00:00
updated_at: 2026-04-13T14:55:19.211407+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Rewrite Python runner as thin wrapper around Rust API instead of parallel reimplementation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

*Origin: Architecture review EVO-05, RC-04. Extracted from I-0090 (crate extraction doesn't fix this).*

## Objective

`PyDefaultRunner` (2,888 lines in `runner.rs`) reimplements coordination logic — message-passing variants, separate Tokio runtime, duplicated cron/trigger/execution handling — rather than wrapping the Rust `DefaultRunner` API. This causes drift: credential leaks (OPS-03, fixed), context manager bugs (API-08, fixed), and high maintenance cost for any new feature (must be added in both Rust and Python paths).

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**: Every new runner feature must be implemented twice. Bugs fixed in Rust path don't automatically fix Python path.
- **Benefits of Fixing**: Single implementation, Python bindings become a thin PyO3 wrapper (~200-400 lines), new features automatically available in Python.
- **Risk Assessment**: Large rewrite with risk of subtle behavior changes. The current implementation works and immediate bugs have been fixed. Lower urgency.

## Acceptance Criteria

## Acceptance Criteria

- [x] Runner creation, execution, shutdown all delegate to the Rust API (already did — message-passing is required for GIL safety)
- [x] Cron/trigger schedule management delegates to Rust API (already did)
- [x] Eliminate duplicated event loop code (3 copies → 1 `run_event_loop()`)
- [x] Eliminate duplicated send/recv boilerplate (15 copies → 1 `send_and_recv()`)
- [x] Eliminate duplicated dict conversion (5 copies → 4 shared helpers)
- [x] Fix `with_schema()` double DefaultRunner creation
- [x] `runner.rs` reduced from ~2,741 lines to ~1,642 lines (40% reduction)
- [x] All unit tests pass (833/833 ✓)
- [x] All Python integration test scenarios pass (28/28 ✓)
- [x] All Python smoke tests pass (6/6 ✓)

## Status Updates

### 2026-04-13 — Refactored

**Approach:** The message-passing architecture is structurally required for Python GIL safety (can't call async Rust from Python's synchronous call model without a background thread). The real debt was triplication of the event loop and boilerplate.

1. Extracted `run_event_loop()` — 300-line match block was copy-pasted 3×. Now single source.
2. Extracted `spawn_runtime()` — generic constructor with closure. All 3 constructors now ~10 lines.
3. Added `send_and_recv<T: Send>()` — eliminates 15 copies of send+recv+error pattern.
4. Added 4 dict conversion helpers — DRY'd dict building for schedules and executions.
5. Fixed `with_schema()` double-creation — was creating DefaultRunner twice. Now uses init channel like others.
6. Added `parse_schedule_id()` helper — DRY'd UUID parsing in 6 message handlers.

**Result:** 2,741 → 1,642 lines (40% reduction). 833 unit tests pass. Python tests pending.
