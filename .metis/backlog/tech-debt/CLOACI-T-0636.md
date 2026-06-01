---
id: restore-wire-cloacina-server-lib
level: task
title: "Restore + wire cloacina-server lib tests into angreal (orphaned since PR #72)"
short_code: "CLOACI-T-0636"
created_at: 2026-06-01T14:26:57.952112+00:00
updated_at: 2026-06-01T14:44:18.075886+00:00
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

# Restore + wire cloacina-server lib tests into angreal (orphaned since PR #72)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

cloacina-server's `[lib]` `#[cfg(test)] mod tests` (router/handler/metrics tests, added
in PR #72 `05f5d7ba`) was run by NO angreal suite and NO CI job, so it drifted: 18 of 90
tests failed when run raw. Restore them and wire them into `angreal test integration` so
they run going forward (the server is Postgres-only → postgres lane).

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

## Acceptance Criteria

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

### 2026-06-01 — DONE. `angreal test integration "tests::" --backend postgres` → cloacina-server lib: 90 passed, 0 failed.

18 orphaned/drifted failures → 0. Root causes + fixes:
1. **Metrics recorder (8 tests):** `install_recorder()` is global-once; each `test_state()`
   installed-or-fell-back to a disconnected handle → only the first test's metrics were
   scrapeable. Added a process-wide `OnceLock` (`shared_test_metrics_handle`) shared by all
   `test_state()` calls. (lib.rs tests module)
2. **Missing fixtures (2 upload tests):** `fixture_path` → `crates/cloacina-server/test-fixtures/`
   didn't exist. Copied `rust-workflow.cloacina` + `python-workflow.cloacina` from cloacinactl.
3. **Wrong tenant (tenant-scoped tests):** `default` → `search_path setup failed ... syntax
   error` (schema absent). Switched to `public` (always-present default schema the e2e uses).
4. **Stale response shape (4 list tests):** API returns `{"items":[...]}`; tests asserted
   `body["keys"]/["workflows"]/["executions"]/["schedules"]` → `body["items"]`.
5. **404 body drift:** `fallback_404` returns "no route matches this request"; test asserted
   "not found" → updated.
6. **Cardinality guard (i0099):** counted raw histogram series (buckets); the now-shared
   recorder's request-duration histogram blew the 64 ceiling. Rewrote to count distinct LABEL
   SETS (collapsing the `le` bucket label) — matches the test's own stated intent and still
   catches unbounded labels.
7. **angreal wiring:** added `cargo test -p cloacina-server --lib` to the postgres lane in
   `.angreal/test/integration.py`.

Files: `crates/cloacina-server/src/lib.rs` (tests module), `crates/cloacina-server/test-fixtures/*`
(new), `.angreal/test/integration.py`. Update [[project_server_lib_tests_orphaned]] memory —
no longer orphaned.
