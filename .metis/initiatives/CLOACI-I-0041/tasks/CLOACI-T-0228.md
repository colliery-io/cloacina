---
id: test-infrastructure-test-db-and
level: task
title: "Test infrastructure — test_db() and test_dal() helpers in cloacina-testing"
short_code: "CLOACI-T-0228"
created_at: 2026-03-22T13:05:15.236469+00:00
updated_at: 2026-03-22T22:07:30.852891+00:00
parent: CLOACI-I-0041
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0041
---

# Test infrastructure — test_db() and test_dal() helpers in cloacina-testing

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0041]]

## Objective

Create `test_db()` and `test_dal()` helpers in `cloacina-testing` that return a real in-memory SQLite database with migrations applied. This is the single blocker for all stub tests — once available, tests can use real SQL queries against a real schema.

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

- [ ] `test_db()` returns `Database` backed by in-memory SQLite with all migrations applied
- [ ] `test_dal()` returns `DAL` wrapping `test_db()` — ready for queries
- [ ] PRAGMAs set (foreign_keys=ON, WAL, busy_timeout) matching production
- [ ] Each call returns an isolated database (no cross-test contamination)
- [ ] Helpers are async-compatible (work inside `#[tokio::test]`)
- [ ] At least one integration test demonstrates: insert a cron schedule via DAL, read it back
- [ ] Exported from `cloacina-testing` so other crates can use them

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

### 2026-03-22 — Complete

**New file:** `crates/cloacina-testing/src/test_db.rs`
- `test_db()` — creates in-memory SQLite with all migrations, returns `Database`
- `test_dal()` — wraps `test_db()` in `DAL`, ready for queries
- Uses unique file URIs (`file:testdb_{uuid}?mode=memory&cache=shared`) for isolation
- PRAGMAs set by `run_migrations()` (foreign_keys=ON, WAL, busy_timeout)

**Updated:** `crates/cloacina-testing/Cargo.toml`
- Added `db` feature flag — brings in `cloacina` (sqlite), `tokio`, `uuid`
- Added `pyo3-build-config` build dep for Python rpath
- New `build.rs` for Python framework rpath on macOS

**Updated:** `crates/cloacina-testing/src/lib.rs`
- `test_db` module exported behind `#[cfg(feature = "db")]`
- Re-exports `test_db` and `test_dal` functions

**Tests (3 new, all pass):**
- `test_db_creates_isolated_databases` — verifies SQLite backend detection
- `test_dal_cron_schedule_roundtrip` — create schedule via DAL, read back, verify fields
- `test_dal_isolation_between_tests` — insert in dal1, verify dal2 is empty

14/14 cloacina-testing tests pass. 490 workspace tests pass.
