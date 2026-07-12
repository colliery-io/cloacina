---
id: bug-python-param-secret-boundary
level: task
title: "BUG: Python param/secret/boundary source scanners aren't comment-aware — an inline comment breaks the declaration"
short_code: "CLOACI-T-0899"
created_at: 2026-07-12T13:16:01.070778+00:00
updated_at: 2026-07-12T13:16:01.070778+00:00
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

# BUG: Python param/secret/boundary source scanners aren't comment-aware — an inline comment breaks the declaration

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

**Finding from T-0885 Python params example (2026-07-12) — FIXED same-session.** `cloacina-compiler`'s Python source scanners (`param_parse.rs`) parse `@cloaca.workflow_params(...)`, `@cloaca.workflow_secrets(...)`, and `@cloaca.boundary_schema(...)` by naive comma/`=` splitting with no Python lexer — **not comment-aware**. A perfectly normal inline comment breaks the declaration:

```python
@cloaca.workflow_params(
    source=str,   # required
    dst=str,      # required
)
```

parsed as a param literally named `"# required\n    dst"` → the server then rejected every run with `missing required param '# required`. Observed live: the packaged workflow built "successfully" but was **unrunnable** — validation demanded a param that doesn't exist. A user commenting their params (the natural thing) silently breaks their workflow.

**Fix (landed):** `strip_py_comments(src)` in `param_parse.rs` — a comment-aware stripper (`#`→EOL when NOT inside `'…'`/`"…"`/triple-quoted strings, backslash-escape aware, newlines preserved) applied at all three read sites (`walk_py`, `walk_py_secrets`, `walk_py_surfaces`) before parsing. One fix covers params, secrets, AND boundary schemas. Verified: the `python-parameterized` example (which naturally comments its params) builds + validates + runs to Completed after the fix (was broken before).

**Remaining hardening (this task's tail):** the scanners are still string-split heuristics — line continuations, unusual whitespace, or a `#` in a param DEFAULT string are edge cases the stripper mostly but not provably handles. Add unit tests for `strip_py_comments` (comment in string preserved, triple-quote, escaped quote) and consider whether a tiny real tokenizer is warranted. Related: [[CLOACI-T-0885]].

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

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

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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
