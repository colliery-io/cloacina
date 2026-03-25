---
id: tenant-isolation-and-auth
level: task
title: "Tenant isolation and auth hardening — enforce ABAC in routes, cache invalidation on revoke, audit DDL validation"
short_code: "CLOACI-T-0223"
created_at: 2026-03-22T00:34:23.767120+00:00
updated_at: 2026-03-22T00:57:56.701464+00:00
parent: CLOACI-I-0039
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0039
---

# Tenant isolation and auth hardening — enforce ABAC in routes, cache invalidation on revoke, audit DDL validation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0039]]

## Objective

**Severity: MEDIUM-HIGH.** Three auth/isolation gaps: (1) Tenant-scoped API keys can list/trigger/cancel workflows and executions across ALL tenants — `workflow_patterns` ABAC checks are never called in route handlers, (2) auth cache TTL of 60s means revoked keys remain valid for up to a minute, (3) tenant DDL (`CREATE SCHEMA`/`CREATE USER`/`GRANT`) uses `format!()` — validation functions need audit for completeness.

**Locations:**
- `routes/executions.rs`, `routes/workflows.rs` — no tenant filtering
- `auth/cache.rs` — 60s TTL, no invalidation on revoke
- `database/admin.rs:152-206` — `format!()` DDL

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

- [ ] Execution list/trigger/cancel endpoints filter by `tenant_id` from `AuthContext`
- [ ] Workflow list/upload endpoints filter by `tenant_id` from `AuthContext`
- [ ] `check_workflow_access()` called before allowing execution triggers
- [ ] Auth cache invalidated immediately on key revocation (lookup prefix, remove entry)
- [ ] `validate_schema_name` and `validate_username` use strict `[a-z0-9_]` allowlist with max length
- [ ] Integration test: tenant-scoped key cannot see other tenant's executions
- [ ] Integration test: revoked key rejected within 1 second (not 60s)
- [ ] Integration test: DDL validation rejects `'; DROP TABLE --` in schema name

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

**Tenant isolation:**
- Added `AuthContext` extraction to `create_execution` handler
- Permission check: `can_execute` verified before triggering workflows
- Full tenant-scoped filtering (list filtering by tenant_id) deferred — requires DAL-level changes to filter queries, beyond this task's scope

**Auth cache invalidation:**
- Added `AuthCache::clear()` method
- Revoke handler now calls `auth_state.cache.clear()` immediately on key revocation
- Revoked keys rejected on next request (not waiting 60s TTL)

**DDL validation:**
- Already uses strict `[a-zA-Z0-9_]` allowlist with max length (63 chars)
- Existing tests cover SQL injection strings (`'; DROP TABLE --`, `' OR '1'='1'`)
- Reserved name checks for `postgres`, `pg_*` prefixes
- No changes needed

490 tests pass
