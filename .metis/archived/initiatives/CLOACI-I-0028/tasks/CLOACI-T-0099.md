---
id: code-example-validation-python
level: task
title: "Code Example Validation — Python Tutorials (01–09)"
short_code: "CLOACI-T-0099"
created_at: 2026-03-13T14:30:07.504385+00:00
updated_at: 2026-03-13T23:22:27.484710+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# Code Example Validation — Python Tutorials (01–09)

**Phase:** 3 — Code Example Validation (Pass 2)
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Extract and validate every code example from Python tutorials 01–09. Each code block must execute correctly with current `cloaca` bindings.

## Scope

Files: `docs/content/python-bindings/tutorials/01-*.md` through `docs/content/python-bindings/tutorials/09-*.md`

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Every Python code block extracted and cataloged
- [ ] Each example verified against current `cloaca` package API (`import cloaca`, `cloaca.task`, `cloaca.WorkflowBuilder`, etc.)
- [ ] `pyproject.toml` examples have correct dependency specifications
- [ ] `@task` decorator parameters match current implementation (id, dependencies, retry_*, on_success, on_failure)
- [ ] Context API usage matches current implementation (`.get()`, `.set()`, `.insert()`)
- [ ] TaskHandle usage matches current implementation (`.defer_until()`, `.is_slot_held()`)
- [ ] All broken examples fixed in-place
- [ ] Run `angreal demos python-tutorial-01` through `python-tutorial-07` to verify working examples

## Implementation Notes

### Validation Approach
1. Extract code blocks with triple-backtick python/toml/bash tags
2. For Python blocks: verify imports, class/function names, method signatures against `cloaca` runtime
3. For pyproject.toml blocks: verify `[tool.cloaca]` section format
4. Cross-reference with `examples/tutorials/python/` directory
5. Use `angreal demos python-tutorial-*` to run actual examples

### Known Risk Areas
- Tutorial 08 (task handles) and 09 (packaging) are newly written — may have subtle API mismatches
- `cloaca build` CLI examples may not match current argument parsing
- Async task examples need verification against current async support

## Status Updates

### Session 1 (2026-03-13)

**Validation approach:**
- Cataloged all code blocks across 9 Python tutorial docs via research agent
- Mapped actual `cloaca` Python API surface from PyO3 bindings (context.rs, runner.rs, etc.)
- Cross-referenced tutorial code against actual API methods
- Ran all 7 Python tutorial demos (`angreal demos python-tutorial-01` through `07`) — all pass

**Fixes applied:**
- **Tutorial 03** (Complex Workflows): Fixed `context.delete()` → `context.remove()` at line 880. The Context class has no `delete()` method; the correct method is `remove()`.

**Verification:**
- All Python tutorial demos (01-07) pass cleanly
- Tutorials 08-09 have no corresponding example files but code blocks verified against PyO3 bindings
- Context API verified: `get()`, `set()`, `insert()`, `update()`, `remove()`, `to_dict()`, `update_from_dict()`, `__contains__`
- TaskHandle API verified: `defer_until(condition, poll_interval_ms)`, `is_slot_held()`
- WorkflowBuilder, DefaultRunner, @task decorator all match current implementation
- Hugo docs build passes after fix
