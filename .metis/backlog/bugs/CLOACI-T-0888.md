---
id: bug-databaseadmin-build-connection
level: task
title: "BUG: DatabaseAdmin build_connection_string hardcodes localhost:5432 — tenant credentials point at the wrong host on any non-default deployment"
short_code: "CLOACI-T-0888"
created_at: 2026-07-11T21:01:45.516203+00:00
updated_at: 2026-08-08T13:21:11.620897+00:00
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

# BUG: DatabaseAdmin build_connection_string hardcodes localhost:5432 — tenant credentials point at the wrong host on any non-default deployment

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

`crates/cloacina/src/database/admin.rs` `build_connection_string()` (~line 324) returns a hardcoded template — `postgresql://{user}:{pass}@localhost:5432/cloacina` — regardless of the admin connection it was created from (the code even says "For now, return a template"). The `TenantCredentials.connection_string` handed back by `create_tenant` therefore points at the wrong host/port/database for ANY deployment that isn't exactly localhost:5432/cloacina (the dev stack on 15432, any remote postgres, any non-`cloacina` db name).

**Found** during the 2026-07-11 dev-stack port move (T-0887 verification): python tutorial 06 *prints* the credential connection string (wrong on 15432) but connects via `with_schema(admin_url, schema)`, so nothing in-repo currently breaks — it's latent until a consumer actually dials `credentials.connection_string` (which is the documented purpose of the field).

**Fix:** derive host/port/dbname from the admin connection URL (parse the admin URL, swap in the tenant username/password, keep host/port/db/params). Unit test: a non-default admin URL round-trips into the tenant credential string. Verify the python binding surface (`cloaca.TenantCredentials.connection_string`) reflects the fix (tutorial 06 prints it).

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

- [x] `build_connection_string` derives host/port/dbname from the admin connection instead of a hardcoded template
- [x] A non-default admin URL round-trips into the tenant credential string (unit test)
- [x] Query parameters carried by the admin URL (notably T-0910's `gssencmode=disable`) reach the tenant string
- [x] The admin credentials are not retained on the struct that carries the endpoint
- [x] The python binding surface (`cloaca.TenantCredentials.connection_string`) reflects the fix

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

- 2026-08-08: COMPLETED — PR #246 merged (squash).

  ROOT CAUSE WAS ONE LAYER DEEPER than this ticket described. The hardcoded
  string was not laziness: `Database` retained only pool/backend/schema, so
  `build_connection_string` had NO access to the admin URL and literally could
  not do better. That is why the original author wrote "For now, return a
  template" — the information wasn't there to use.

  Fixed at that root. `Database` now carries an `endpoint`: the resolved
  postgres URL with any userinfo stripped. Credentials are deliberately NOT
  retained — the pool already holds the admin secret, and this field exists
  purely to carry the non-secret coordinates; a test asserts the admin user and
  password are absent. `None` for sqlite, which has no per-user credential
  model to hand out. New `Database::tenant_connection_string(user, pass)`
  substitutes only the credentials.

  PRESERVING QUERY PARAMS turned out to matter more than the host fix itself:
  T-0910 defaults `gssencmode=disable` on the admin URL, so a tenant credential
  that dropped it would dial with DIFFERENT behavior than the admin connection
  — re-opening the glibc getenv/setenv crash class T-0910 closed. Covered by a
  test.

  The `localhost:5432` form survives only as an unreachable fallback (the
  module is postgres-gated, so the endpoint is always present). Kept rather
  than panicking because a best-effort string beats failing tenant creation
  AFTER the schema and role are already committed.

  String handling was split into two free functions (`endpoint_of`,
  `inject_credentials`) so the behavior is unit-testable without standing up a
  connection pool — constructing a `Database` in a test requires a live pool.

  Tests, 6: non-default deployment round-trips; admin credentials stripped and
  no dangling `@` from emptied userinfo; query params survive; empty password
  does not serialize a bare `user:@host`; a password containing `@` and `/` is
  escaped rather than corrupting the authority; an unparseable URL yields None.
  803 cloacina lib tests pass; cargo check clean for postgres-only,
  sqlite-only, and both (endpoint_of is cfg-gated to postgres so sqlite-only
  gains no dead-code warning), plus cloacina-server and cloacina-python.

  RESIDUAL: the python binding (`bindings/admin.rs:149`) rebuilds the admin URL
  as `scheme://user:pass@host:port` before constructing `Database`, so query
  parameters on the caller's original URL are dropped before the endpoint is
  derived. Pre-existing, unchanged here; host and port — this ticket's subject
  — are preserved. Worth a follow-up if a deployment needs sslmode to reach
  tenant credentials through the python path.
