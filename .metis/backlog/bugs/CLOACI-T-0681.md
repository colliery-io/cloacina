---
id: cloacinactl-list-commands-read-pre
level: task
title: "cloacinactl list commands read pre-T-0594 envelope keys (package list broken; workflow --package filter dropped)"
short_code: "CLOACI-T-0681"
created_at: 2026-06-14T23:49:58.449798+00:00
updated_at: 2026-06-14T23:49:58.449798+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# cloacinactl list commands read pre-T-0594 envelope keys (package list broken; workflow --package filter dropped)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective **[REQUIRED]**

The T-0594 "unified list envelope" changed every server list endpoint to emit
`{items, total}` (or `{tenant_id, items, total}`), but several `cloacinactl`
list commands were never migrated — they still read the old resource-named key
(`workflows`/`keys`/`graphs`/`accumulators`) instead of `items`. Migrate every
list command onto the canonical `render::list` (`items`-aware) path.

### Impact (what actually broke)
- **`package list`** — hand-rolled its own renderer reading `body["workflows"]`;
  returned **empty** even when workflows existed. (Already fixed in #124 by
  reading `items`.)
- **`workflow list --package <pat>`** — read `body["workflows"]` → got the whole
  envelope object (not an array) → the `--package` **filter was silently
  dropped** (returned all workflows).
- **`key list` / `graph list` / `graph accumulators`** — used
  `body.get("<key>").unwrap_or(body)`; these *happened* to still render correctly
  because the whole envelope was then passed to `render::list`, which re-extracts
  `items` — but the pattern is fragile and the `unwrap_or(body)` "silent swallow"
  is exactly what T-0594 removed from `render::list`.

Commands already correct (use `render::list`): `tenant list`, `execution list`,
`trigger list`.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (`package list` already fixed in #124; the remaining
  user-visible bug is the dropped `workflow list --package` filter; the rest is
  defensive cleanup of the same root cause)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [x] `workflow list` reads `items` and applies `--package` to the actual array
      (filter no longer silently dropped).
- [x] `key list`, `graph list`, `graph accumulators` route through `render::list`
      (the `items`-aware helper) — no more `body.get("<key>").unwrap_or(body)`.
- [x] `package list` reads `items` (landed in #124).
- [x] e2e regression guard: `key list -o json` returns a non-empty `items` array
      of key objects.

## Status Updates

**2026-06-14 — Fixed.** `workflow/mod.rs` (extract `items`, then filter),
`key/mod.rs` + `graph/mod.rs` (pass body straight to `render::list`). Added a
`key list` non-empty assertion to the `angreal test e2e cli` authoring lane.
Verified: `cargo fmt --check` clean, `cargo test -p cloacinactl --bins` → 68
passed. (Local clippy flagged a pre-existing `useless_conversion` in
`cloacina-client` under rust-1.93.0 — unrelated to this change and not seen by
CI's pinned toolchain.) On branch `fix/cli-list-envelope-t0594`.

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*