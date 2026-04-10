---
id: rewrite-python-runner-as-thin
level: task
title: "Rewrite Python runner as thin wrapper around Rust API instead of parallel reimplementation"
short_code: "CLOACI-T-0464"
created_at: 2026-04-09T15:54:27.311608+00:00
updated_at: 2026-04-09T15:54:27.311608+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


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

- [ ] `PyDefaultRunner` wraps `DefaultRunner` directly (not via message-passing thread)
- [ ] Runner creation, execution, shutdown all delegate to the Rust API
- [ ] Cron/trigger schedule management delegates to Rust API
- [ ] `runner.rs` reduced from ~2,888 lines to ~400 lines
- [ ] All 31 Python integration test scenarios pass
- [ ] All Python tutorials work unchanged

## Status Updates

*To be added during implementation*
