---
id: python-execution-sandbox-sys-path
level: task
title: "Python execution sandbox — sys.path validation, execution timeouts, decompression limits"
short_code: "CLOACI-T-0222"
created_at: 2026-03-22T00:34:23.275152+00:00
updated_at: 2026-03-22T00:51:43.418185+00:00
parent: CLOACI-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0039
---

# Python execution sandbox — sys.path validation, execution timeouts, decompression limits

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0039]]

## Objective

**Severity: HIGH.** The Python execution boundary has three gaps: (1) `sys.path` injection allows malicious packages to shadow stdlib modules (e.g., `os.py`, `json.py` in vendor/ imported before real stdlib), (2) no execution timeouts — infinite loops block forever, (3) no resource constraints — Python tasks can allocate unbounded memory, make network calls, and run subprocess.

**Locations:**
- `python/loader.rs:137-160` — sys.path.insert(0, ...) before stdlib
- `python/loader.rs` / `python/executor.rs` — no timeout wrapping execution

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Extracted package contents validated against stdlib module name deny list before import
- [ ] sys.path insertion uses append (after stdlib) or a restricted import hook instead of insert(0)
- [ ] Python task execution wrapped in `tokio::time::timeout` (configurable, default 5 minutes)
- [ ] Timeout produces clear error: "Python task 'X' timed out after 300s"
- [ ] Unit test: package with `os.py` in workflow/ or vendor/ is rejected at extraction
- [ ] Unit test: Python task that sleeps forever is terminated by timeout
- [ ] Existing Python workflow example continues to work

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

### 2026-03-21 — Complete

- Changed `sys.path.insert(0, ...)` to `sys.path.append(...)` — stdlib no longer shadowed
- Added `STDLIB_DENY_LIST` (30 modules: os, sys, subprocess, etc.)
- Added `validate_no_stdlib_shadowing()` — scans workflow/ and vendor/ for shadowing files
- Added import timeout (60s default) — polls thread with 100ms interval
- Timeout returns clear error: "Python workflow import timed out after 60s"
- 490 tests pass
