---
id: clean-up-stale-sqlite-databases-in
level: task
title: "Clean up stale SQLite databases in examples and test harnesses before/after runs"
short_code: "CLOACI-T-0312"
created_at: 2026-03-30T12:05:29.120078+00:00
updated_at: 2026-04-03T01:33:27.316529+00:00
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

# Clean up stale SQLite databases in examples and test harnesses before/after runs

## Objective

Stale SQLite database files from previous runs cause misleading test failures and runtime errors. Examples and tutorials write `.db` files that persist between runs. When task namespaces change (e.g., from `public::embedded::` to `public::{pkg_name}::`), the old pipeline/task records in the DB reference namespaces that no longer exist, causing `Task not found` errors.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All tutorial examples delete their SQLite DB file at startup (before runner init)
- [ ] All feature examples that use SQLite do the same
- [ ] angreal demo tasks clean up DB files before running examples
- [ ] `.gitignore` entries for `*.db`, `*.db-shm`, `*.db-wal` in example directories
- [ ] Integration test harnesses use fresh in-memory or temp-dir databases (most already do)

## Implementation Notes

### Affected files
- `examples/tutorials/*/src/main.rs` — add `std::fs::remove_file` for the DB path before runner init
- `.angreal/demos/` — add cleanup step before each demo run
- `examples/**/.gitignore` — add DB file patterns

### Priority
P2 — causes confusion during development but doesn't affect production

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*
