---
id: consolidate-cloaca-test-harness
level: task
title: "Consolidate cloaca test harness into cloacina — cloaca is a Python interface, not a separate system"
short_code: "CLOACI-T-0486"
created_at: 2026-04-13T15:22:25.093649+00:00
updated_at: 2026-04-13T15:22:25.093649+00:00
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

# Consolidate cloaca test harness into cloacina — cloaca is a Python interface, not a separate system

## Objective

"Cloaca" is just the Python interface name for Cloacina — it's the same system, not a separate product. The test infrastructure (`angreal cloaca test`, `angreal cloaca smoke`, etc.) should be consolidated under the `cloacina` angreal namespace to reflect this. Tests for the Python bindings are cloacina tests, not tests for a separate system.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

### Technical Debt Impact
- **Current Problems**: The `angreal cloaca` namespace implies cloaca is a separate system with its own test harness. This is confusing — it's the Python interface to cloacina. The separation adds cognitive overhead ("do I run `angreal cloaca test` or `angreal cloacina test`?").
- **Benefits of Fixing**: Single `angreal cloacina` namespace for all testing (Rust unit, integration, macros, Python bindings). Clearer mental model.
- **Risk Assessment**: Low risk. Mostly renaming angreal tasks and moving test scripts. No code changes to the actual library.

## Acceptance Criteria

- [ ] Python binding tests runnable via `angreal cloacina python` or similar under the cloacina namespace
- [ ] `angreal cloaca` tasks either removed or aliased to the new names
- [ ] Python wheel build/package tasks consolidated (currently `angreal cloaca package/release`)
- [ ] All existing test scenarios continue to pass under the new task names

## Status Updates

*To be added during implementation*
