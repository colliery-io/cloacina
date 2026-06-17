---
id: server-ignores-database-name-in
level: task
title: "Server ignores database name in --database-url — pools hardcode the cloacina dbname via build_postgres_url"
short_code: "CLOACI-T-0649"
created_at: 2026-06-10T03:24:08.288047+00:00
updated_at: 2026-06-17T11:46:45.498288+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Server ignores database name in --database-url — pools hardcode dbname "cloacina" via build_postgres_url

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

`Database::build_postgres_url` (crates/cloacina/src/database/connection/mod.rs:443) does `url.set_path(database_name)`, unconditionally replacing the dbname from the connection URL — and cloacina-server passes a hardcoded `"cloacina"` (e.g. `TenantDatabaseCache::resolve`, crates/cloacina-server/src/lib.rs:162-167; the main pool exhibits the same behavior). Net effect: `--database-url postgres://…/any_db_name` silently connects to database `cloacina`.

Found by the T-0645 TS contract suite: a dedicated `sdk_contract` database stayed completely empty (no migrations, no tables) while tenant schemas and api_keys rows appeared in `cloacina`, despite startup logging `Database: postgres://…/sdk_contract`.

Fix direction: respect the URL's path when present; only fall back to the `database_name` parameter when the URL lacks one. Audit every `Database::new*` call site for hardcoded `"cloacina"`.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [x] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [x] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: any operator pointing cloacina-server (or per-tenant pools) at a non-default-named Postgres database
- **Reproduction Steps**:
  1. `createdb mydb` and run `cloacina-server --database-url postgres://user:pass@host/mydb`
  2. Create an API key / tenant via the REST API
  3. Inspect both databases
- **Expected vs Actual**: data should land in `mydb`; it lands in `cloacina` while `mydb` stays empty, and startup logs claim `mydb` is in use

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

*To be added during implementation*

## Status Updates
- 2026-06-17: **Fixed + verified (unit).** `Database::build_postgres_url`
  (crates/cloacina/src/database/connection/mod.rs:443) no longer unconditionally
  `set_path(database_name)`; it now respects an explicit dbname in the URL and
  only falls back to the parameter when the URL has none (empty path or `/`).
  This fixes `--database-url postgres://…/mydb` silently connecting to the
  hardcoded `"cloacina"`. Added unit guards
  `build_postgres_url_respects_explicit_dbname` +
  `build_postgres_url_falls_back_when_no_dbname` (both pass); 53 database-module
  tests green. Audited all `Database::new*` call sites: server (lib.rs:163) and
  runner (mod.rs:108) pass `"cloacina"`, and the postgres test harness
  (tests/fixtures.rs:89) passes a base URL with no dbname — all use the
  unchanged fallback path, so no caller regresses. The only behavior change is
  the bug fix (explicit URL dbname now honored).