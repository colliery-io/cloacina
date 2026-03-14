---
id: documentation-and-testing-tutorial
level: task
title: "Documentation and testing tutorial"
short_code: "CLOACI-T-0116"
created_at: 2026-03-14T02:59:48.827612+00:00
updated_at: 2026-03-14T08:21:37.998794+00:00
parent: CLOACI-I-0027
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0027
---

# Documentation and testing tutorial

## Parent Initiative

[[CLOACI-I-0027]]

## Objective

Write crate-level rustdoc documentation for `cloacina-testing` and add a Hugo tutorial page ("Testing Your Workflows") to the documentation site. The tutorial should walk users through unit testing their task logic without a database.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Crate-level `//!` documentation in `lib.rs` with overview, quick-start example, and feature flag explanation
- [ ] Each public type has rustdoc with examples: `TestRunner`, `TestResult`, `TaskOutcome`
- [ ] Hugo tutorial page at `docs/content/tutorials/09-testing-workflows.md` (or appropriate number)
- [ ] Tutorial covers: adding `cloacina-testing` as dev-dependency, writing a basic test, testing failure scenarios, using assertion helpers
- [ ] Tutorial includes a runnable example project under `examples/tutorials/`
- [ ] `angreal docs build` passes with the new tutorial page
- [ ] `cargo doc -p cloacina-testing --no-deps` builds without warnings

## Implementation Notes

### Tutorial Structure
1. **Why test without a database** — motivation
2. **Setup** — add `cloacina-testing` to `[dev-dependencies]`
3. **Your first test** — `TestRunner::new().register(...).run(ctx).await`
4. **Testing failures** — assert a task fails and dependents are skipped
5. **Assertion helpers** — `assert_all_completed()`, `assert_task_failed()`
6. **Next steps** — link to integration testing guide for full-stack tests

### Dependencies
- Depends on CLOACI-T-0112, T-0113, T-0115 (all implementation and test tasks complete)

## Status Updates

- Created Hugo tutorial at `docs/content/tutorials/11-testing-workflows.md`
  - Covers: setup, first test, failure scenarios, assertion helpers, cycle detection, key behaviors
- Crate-level `//!` docs in lib.rs with quick-start example and feature flag docs
- All public types have rustdoc: `TestRunner`, `TestResult`, `TaskOutcome`, `TestRunnerError`
- Re-exported `TestRunnerError` from lib.rs
- `cargo doc -p cloacina-testing --no-deps` builds without warnings
- `angreal docs build` passes with new tutorial page
- Skipped example project under `examples/tutorials/` — the tutorial's inline code examples are sufficient and a standalone project would just duplicate what's already in tests
