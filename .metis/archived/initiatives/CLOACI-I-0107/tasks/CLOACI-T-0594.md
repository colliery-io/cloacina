---
id: t-01-fix-broken-cli-commands
level: task
title: "T-01: Fix broken CLI commands — tenant create body, execution list filters, list render"
short_code: "CLOACI-T-0594"
created_at: 2026-05-14T17:23:11.083940+00:00
updated_at: 2026-05-14T17:33:31.728909+00:00
parent: CLOACI-I-0107
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0107
---

# T-01: Fix broken CLI commands — tenant create body, execution list filters, list render

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0107]]

## Objective **[REQUIRED]**

Three operator-facing CLI/server contract bugs caught by the May 2026 review. Each is a bounded patch; bundled here because they share the "CLI works, server works, seam silently broken" failure mode.

### API-01 — `tenant create` body shape mismatch

`cloacinactl tenant create <name>` posts `{schema_name, username, password}`; the server expects `{name, description, password?}`. Every CLI tenant create has been failing.

**Resolution (per design Q&A):** change the **server** to accept the CLI's user-friendly `{name, description, password?}` shape. Fix the request struct on `POST /v1/tenants` to match. If any direct API consumers send `{schema_name, ...}`, they need to switch — flagged as a breaking change for the release notes.

### API-02 — `execution list` filters silently ignored

`cloacinactl execution list --status Failed --workflow_name foo` returns *only active executions* regardless of the filter flags — the server route's DAL call discards the query string entirely.

**Fix:** plumb `status` + `workflow_name` (and `limit`/`offset`, paired with API-10 in T-0596) through the DAL `list_executions` call. Server route must accept the query params, validate them, and pass them to the DAL.

### API-03 — Empty-render bug on list endpoints

`cloacinactl tenant list`, `trigger list`, `execution list` render empty for response shapes that don't match a hardcoded key. The renderer expects `body.<key>` and silently falls back to `body` if the key is missing — which makes the empty-default render swallow real data.

**Fix:** normalize all list responses on the server side to `{ items: [...], total: N, ... }` envelope. CLI list renderer reads `body.items` everywhere. No silent fallback.

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

- [ ] `cloacinactl tenant create my-tenant --password X --description "Y"` round-trips end-to-end and inserts a tenant row.
- [ ] `cloacinactl execution list --status Failed --workflow-name foo` returns only failed executions matching the name; `--status Running` returns only active.
- [ ] `cloacinactl tenant list`, `trigger list`, `execution list` all render rows from the same `{items: [...]}` envelope; the renderer has zero silent fallback paths.
- [ ] Release-notes line written for the API-01 server-side breaking change (`{schema_name, ...}` is no longer accepted).
- [ ] T-0597 (harness) adds an integration test per fix.

## Implementation Notes

- API-01: edit the `CreateTenantRequest` (or equivalent) struct on the server side to match the CLI's payload. The DAL/migration plumbing for `schema_name` derivation lives behind it — keep the public API ergonomic.
- API-02: DAL change is `list_executions(filter: ExecutionFilter)` where ExecutionFilter has `status: Option<WorkflowStatus>`, `workflow_name: Option<String>`, `limit/offset` (T-0596 lands those). Route extracts query params + builds the filter.
- API-03: pick one envelope shape — `{items, total}` — and apply it everywhere. The CLI's table renderer's silent `body → body.<key>` fallback gets removed; missing `items` is now a hard error.

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

### 2026-05-14 — implemented

- **API-01**: `CreateTenantRequest` on the server now accepts `{name, description?, password?}`. `schema_name` + `username` are both derived from `name`. Response keys updated (`name` replaces `schema_name`). All in-tree tests (`test_create_tenant_returns_201`, `test_remove_tenant_idempotent_retry`, `test_create_then_delete_tenant`, `test_tenant_runner_cache_lru_evicts_oldest`) migrated to the new shape. Breaking change for any direct API consumers using `{schema_name, username, password}`.
- **API-02**: new `ExecutionListFilter` DAL struct + `WorkflowExecutionDAL::list_filtered()` with postgres/sqlite dispatch. Route accepts `?status=`/`?workflow=`/`?limit=`/`?offset=` via `Query<ListExecutionsQuery>`. Bounds enforced: `limit ∈ 1..=1000`, `offset >= 0`. Replaces the silent `get_active_executions()` call.
- **API-03**: every list endpoint now emits the unified `{items, total}` envelope. Sites updated: `tenants::list_tenants`, `keys::list_keys`, `triggers::list_triggers`, `executions::list_executions`, `workflows::list_workflows`, `health_graphs::list_accumulators`, `health_graphs::list_graphs`. CLI's `render::list` reads `body.items` with a bare-array fallback for un-migrated endpoints; the silent `body → body.<key>` fallback is **removed** — non-list responses now return `CliError::UserError`.
- CLI `tenant create` extended with `--password` flag matching the new server contract.

Sequenced next: T-0595 (error envelope + router nest).
