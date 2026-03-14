---
id: code-example-validation
level: task
title: "Code Example Validation — Explanation & How-To Guide Code Blocks"
short_code: "CLOACI-T-0100"
created_at: 2026-03-13T14:30:08.895286+00:00
updated_at: 2026-03-14T01:28:03.614208+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# Code Example Validation — Explanation & How-To Guide Code Blocks

**Phase:** 3 — Code Example Validation (Pass 2)
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Validate all code examples in explanation docs and how-to guides (non-tutorial pages). These include architecture explanations, system descriptions, and procedural guides.

## Scope

- `docs/content/explanation/*.md` (15 files)
- `docs/content/how-to-guides/*.md`
- `docs/content/python-bindings/how-to-guides/*.md`

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Every Rust code block in explanation docs verified against current crate APIs
- [ ] Every Python code block in how-to guides verified against current cloaca API
- [ ] Simplified/pseudocode examples clearly marked as such (not misleading)
- [ ] Code snippets from source (e.g., "from `validator.rs`") match the actual current source
- [ ] Mermaid diagram syntax is valid and renders correctly
- [ ] All broken examples fixed in-place

## Implementation Notes

### Specific Pages to Scrutinize
- `explanation/macro-system.md` — generated code examples must match actual macro output
- `explanation/package-format.md` — manifest JSON examples must match ManifestV2 schema
- `explanation/task-handle-architecture.md` — internal code snippets must match source
- `explanation/packaged-workflow-architecture.md` — Python pipeline code must match cloaca internals
- `python-bindings/how-to-guides/packaging-for-multiple-platforms.md` — CLI examples must work
- `python-bindings/how-to-guides/testing-workflows.md` — test patterns must be current

### Approach
1. For each code block: identify if it's compilable code, pseudocode, or output
2. Compilable code: verify against source
3. Source snippets: diff against actual file at stated location
4. Output examples: verify they match current tool output format

## Status Updates

### Session 1 (2026-03-13)

**Validation approach:**
- Launched 3 parallel research agents covering: explanation docs (15 files), how-to guides (8 files), C4 architecture docs (10 files)
- Each agent cross-referenced code blocks against actual source files in crates/

**Fixes applied:**
1. **ffi-system.md**: Added 3 missing fields to `cloacina_ctl_package_tasks` struct (`package_description`, `package_author`, `workflow_fingerprint`) to match actual source in `cloacina-macros/src/packaged_workflow.rs`
2. **c4-data-access-layer.md**: Fixed 3 method name mismatches:
   - `mark_running()` removed (doesn't exist — only `mark_ready/completed/failed`)
   - `claim_task()` → `claim_ready_task()`
   - `find_orphaned_tasks()` → `get_orphaned_tasks()`
3. **security/local-development.md**: Fixed `cloacina sign` → `cloacinactl package sign` (correct CLI binary name)

**Not issues (verified correct):**
- `cloaca build` in Python how-to guides is correct (Python-side CLI, distinct from `cloacinactl`)
- `cloaca.DatabaseAdmin` IS exposed in Python bindings (PyO3 admin.rs)
- Python API is synchronous (no `await` needed) — PyO3 handles async internally
- All Mermaid diagrams: valid syntax
- All 50+ source file references in C4 docs: all paths verified to exist
- All trait/type signatures verified correct

**Verification:** Hugo docs build passes cleanly.
